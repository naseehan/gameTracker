use crate::database;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// How often active sessions record that their game is still running.
pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// A heartbeat gap longer than this means the machine was asleep or hibernating,
/// so the gap is not counted as playtime.
const MAX_HEARTBEAT_GAP_SECS: i64 = 5 * 60;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveSession {
    pub session_id: i64,
    pub game_id: i64,
    pub pids: HashSet<u32>,
    pub started_at: DateTime<Utc>,
    pub last_heartbeat_at: DateTime<Utc>,
}

impl ActiveSession {
    /// True if no heartbeat was recorded for so long that the machine must have been asleep.
    fn missed_heartbeats(&self, now: DateTime<Utc>) -> bool {
        (now - self.last_heartbeat_at).num_seconds() > MAX_HEARTBEAT_GAP_SECS
    }

    /// When the session ends if it stops at `now`: the last heartbeat if the machine
    /// was asleep in between, otherwise `now`.
    fn end_time(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        if self.missed_heartbeats(now) {
            self.last_heartbeat_at
        } else {
            now
        }
    }

    fn close(&self, ended_at: DateTime<Utc>, db: &Connection) -> Result<(), String> {
        let duration = (ended_at - self.started_at).num_seconds().max(0);
        database::close_session(db, self.session_id, ended_at, duration).map_err(|e| e.to_string())
    }
}

pub struct SessionTracker {
    pub active_sessions: HashMap<i64, ActiveSession>,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self {
            active_sessions: HashMap::new(),
        }
    }

    /// Called when a known game process starts. Returns true if a new session started.
    pub fn on_game_started(
        &mut self,
        pid: u32,
        game_id: i64,
        started_at: DateTime<Utc>,
        db: &Connection,
    ) -> Result<bool, String> {
        // Prevent duplicate sessions for the same game, but track new pids
        if let Some(session) = self.active_sessions.get_mut(&game_id) {
            session.pids.insert(pid);
            return Ok(false);
        }

        let session_id =
            database::create_session(db, game_id, started_at).map_err(|e| e.to_string())?;

        let mut pids = HashSet::new();
        pids.insert(pid);

        self.active_sessions.insert(
            game_id,
            ActiveSession {
                session_id,
                game_id,
                pids,
                started_at,
                last_heartbeat_at: started_at,
            },
        );
        Ok(true)
    }

    /// Called when a process stops. Returns true if that ended a session.
    pub fn on_process_stopped(
        &mut self,
        pid: u32,
        ended_at: DateTime<Utc>,
        db: &Connection,
    ) -> Result<bool, String> {
        // Find if this pid is tracking an active session
        let mut game_to_remove = None;
        for (game_id, session) in &mut self.active_sessions {
            if session.pids.remove(&pid) {
                if session.pids.is_empty() {
                    game_to_remove = Some(*game_id);
                }
                break;
            }
        }

        if let Some(game_id) = game_to_remove {
            if let Some(session) = self.active_sessions.remove(&game_id) {
                session.close(session.end_time(ended_at), db)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Ends a game's active session, if it has one (e.g. when the game is ignored or removed).
    pub fn end_game_session(
        &mut self,
        game_id: i64,
        ended_at: DateTime<Utc>,
        db: &Connection,
    ) -> Result<(), String> {
        if let Some(session) = self.active_sessions.remove(&game_id) {
            session.close(session.end_time(ended_at), db)?;
        }
        Ok(())
    }

    /// Records that every active game is still running, so a crash loses at most one
    /// heartbeat interval. After a long gap (sleep or hibernation) the session is split:
    /// the old one ends at its last heartbeat and a new one starts now. Returns true if
    /// any session was split.
    pub fn heartbeat(&mut self, now: DateTime<Utc>, db: &Connection) -> Result<bool, String> {
        let mut split = false;
        for session in self.active_sessions.values_mut() {
            if session.missed_heartbeats(now) {
                split = true;
                session.close(session.last_heartbeat_at, db)?;
                session.session_id = database::create_session(db, session.game_id, now)
                    .map_err(|e| e.to_string())?;
                session.started_at = now;
            } else {
                database::update_session_heartbeat(db, session.session_id, now)
                    .map_err(|e| e.to_string())?;
            }
            session.last_heartbeat_at = now;
        }
        Ok(split)
    }

    /// Recovers unfinished sessions from a previous crash/shutdown.
    pub fn recover_sessions(db: &Connection) -> Result<(), String> {
        database::recover_sessions(db).map_err(|e| e.to_string())
    }

    /// Closes all currently active sessions (e.g. on clean shutdown).
    pub fn close_all(&mut self, ended_at: DateTime<Utc>, db: &Connection) -> Result<(), String> {
        let active = std::mem::take(&mut self.active_sessions);
        for (_, session) in active {
            session.close(session.end_time(ended_at), db)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        database::run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO games (id, name, normalized_name) VALUES (101, 'Test Game', 'testgame')",
            [],
        )
        .unwrap();
        conn
    }

    fn start_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap()
    }

    /// Sends a heartbeat every 30 seconds from `from` up to and including `to`.
    fn keep_running(tracker: &mut SessionTracker, db: &Connection, from: DateTime<Utc>, to: DateTime<Utc>) {
        let mut t = from;
        while t < to {
            t = (t + Duration::seconds(HEARTBEAT_INTERVAL_SECS as i64)).min(to);
            tracker.heartbeat(t, db).unwrap();
        }
    }

    fn session_rows(db: &Connection) -> Vec<(DateTime<Utc>, Option<DateTime<Utc>>, Option<i64>)> {
        let mut stmt = db
            .prepare("SELECT started_at, ended_at, duration_seconds FROM game_sessions ORDER BY id")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn test_start_session() {
        let db = setup_test_db();
        let mut tracker = SessionTracker::new();

        tracker.on_game_started(1234, 101, start_time(), &db).unwrap();

        assert_eq!(tracker.active_sessions.len(), 1);
        let session = tracker.active_sessions.get(&101).unwrap();
        assert!(session.pids.contains(&1234));
        assert_eq!(session.game_id, 101);
        assert_eq!(session_rows(&db).len(), 1);
    }

    #[test]
    fn test_duplicate_process_same_game() {
        let db = setup_test_db();
        let mut tracker = SessionTracker::new();

        tracker.on_game_started(1234, 101, start_time(), &db).unwrap();
        // second process for same game
        tracker.on_game_started(5678, 101, start_time(), &db).unwrap();

        assert_eq!(tracker.active_sessions.len(), 1);
        // It should track both PIDs
        let session = tracker.active_sessions.get(&101).unwrap();
        assert!(session.pids.contains(&1234));
        assert!(session.pids.contains(&5678));
    }

    #[test]
    fn test_stop_session() {
        let db = setup_test_db();
        let mut tracker = SessionTracker::new();
        let start = start_time();
        let end = start + Duration::minutes(30);
        tracker.on_game_started(1234, 101, start, &db).unwrap();
        keep_running(&mut tracker, &db, start, end);

        tracker.on_process_stopped(1234, end, &db).unwrap();

        assert_eq!(tracker.active_sessions.len(), 0);
        assert_eq!(session_rows(&db)[0].2, Some(1800));
    }

    #[test]
    fn test_unknown_process_stop() {
        let db = setup_test_db();
        let mut tracker = SessionTracker::new();
        // Should not panic or err
        assert!(tracker.on_process_stopped(9999, Utc::now(), &db).is_ok());
    }

    #[test]
    fn test_crash_recovery_ends_at_last_heartbeat() {
        let db = setup_test_db();
        let start = start_time();
        {
            let mut tracker = SessionTracker::new();
            tracker.on_game_started(1234, 101, start, &db).unwrap();
            keep_running(&mut tracker, &db, start, start + Duration::minutes(10));
            // The app crashes here: the tracker is dropped without closing the session.
        }

        // The app is started again the next morning.
        SessionTracker::recover_sessions(&db).unwrap();

        let rows = session_rows(&db);
        assert_eq!(rows[0].1, Some(start + Duration::minutes(10)));
        assert_eq!(rows[0].2, Some(600));
    }

    #[test]
    fn test_sleep_gap_splits_session() {
        let db = setup_test_db();
        let mut tracker = SessionTracker::new();
        let start = start_time();
        let wake = start + Duration::hours(10);
        tracker.on_game_started(1234, 101, start, &db).unwrap();
        keep_running(&mut tracker, &db, start, start + Duration::minutes(1));

        // The machine sleeps for ten hours with the game open, then wakes up.
        tracker.heartbeat(wake, &db).unwrap();

        let rows = session_rows(&db);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1, Some(start + Duration::minutes(1)));
        assert_eq!(rows[0].2, Some(60));
        assert_eq!(rows[1].0, wake);
        assert_eq!(rows[1].1, None);
        assert_eq!(tracker.active_sessions[&101].started_at, wake);
    }

    #[test]
    fn test_stop_right_after_sleep_ends_at_last_heartbeat() {
        let db = setup_test_db();
        let mut tracker = SessionTracker::new();
        let start = start_time();
        tracker.on_game_started(1234, 101, start, &db).unwrap();
        keep_running(&mut tracker, &db, start, start + Duration::minutes(2));

        // The game is closed right after waking up, before the next heartbeat.
        tracker
            .on_process_stopped(1234, start + Duration::hours(8), &db)
            .unwrap();

        assert_eq!(session_rows(&db)[0].2, Some(120));
    }

    #[test]
    fn test_same_pid_multiple_starts() {
        let db = setup_test_db();
        let mut tracker = SessionTracker::new();

        // Initial start (e.g., path was None, fallback matched)
        tracker.on_game_started(1234, 101, start_time(), &db).unwrap();

        // Second start (e.g., path resolved)
        tracker.on_game_started(1234, 101, start_time(), &db).unwrap();

        assert_eq!(tracker.active_sessions.len(), 1);
        let session = tracker.active_sessions.get(&101).unwrap();
        assert_eq!(session.pids.len(), 1);
        assert!(session.pids.contains(&1234));
    }
}
