use crate::database;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveSession {
    pub session_id: i64,
    pub game_id: i64,
    pub pids: HashSet<u32>,
    pub started_at: DateTime<Utc>,
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

    /// Called when a known game process starts.
    pub fn on_game_started(
        &mut self,
        pid: u32,
        game_id: i64,
        started_at: DateTime<Utc>,
        db: &Connection,
    ) -> Result<(), String> {
        // Prevent duplicate sessions for the same game, but track new pids
        if let Some(session) = self.active_sessions.get_mut(&game_id) {
            session.pids.insert(pid);
            return Ok(());
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
            },
        );
        Ok(())
    }

    /// Called when a process stops.
    pub fn on_process_stopped(
        &mut self,
        pid: u32,
        ended_at: DateTime<Utc>,
        db: &Connection,
    ) -> Result<(), String> {
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
                let duration = (ended_at - session.started_at).num_seconds().max(0);
                database::close_session(db, session.session_id, ended_at, duration)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Recovers unfinished sessions from a previous crash/shutdown.
    pub fn recover_sessions(db: &Connection, current_time: DateTime<Utc>) -> Result<(), String> {
        database::recover_sessions(db, current_time).map_err(|e| e.to_string())
    }

    /// Closes all currently active sessions (e.g. on clean shutdown).
    pub fn close_all(&mut self, ended_at: DateTime<Utc>, db: &Connection) -> Result<(), String> {
        let active = std::mem::take(&mut self.active_sessions);
        for (_, session) in active {
            let duration = (ended_at - session.started_at).num_seconds().max(0);
            database::close_session(db, session.session_id, ended_at, duration)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE game_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id INTEGER NOT NULL,
                started_at DATETIME NOT NULL,
                ended_at DATETIME,
                duration_seconds INTEGER,
                detection_source TEXT
            );
            ",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_start_session() {
        let db = setup_test_db();
        let mut tracker = SessionTracker::new();
        let start_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();

        tracker.on_game_started(1234, 101, start_time, &db).unwrap();

        assert_eq!(tracker.active_sessions.len(), 1);
        let session = tracker.active_sessions.get(&101).unwrap();
        assert!(session.pids.contains(&1234));
        assert_eq!(session.game_id, 101);

        let count: i64 = db
            .query_row("SELECT count(*) FROM game_sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_duplicate_process_same_game() {
        let db = setup_test_db();
        let mut tracker = SessionTracker::new();
        let start_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();

        tracker.on_game_started(1234, 101, start_time, &db).unwrap();
        // second process for same game
        tracker.on_game_started(5678, 101, start_time, &db).unwrap();

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
        let start_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        tracker.on_game_started(1234, 101, start_time, &db).unwrap();

        let end_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 30, 0).unwrap(); // 30 mins later
        tracker.on_process_stopped(1234, end_time, &db).unwrap();

        assert_eq!(tracker.active_sessions.len(), 0);
        let duration: i64 = db
            .query_row(
                "SELECT duration_seconds FROM game_sessions WHERE game_id = 101",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(duration, 1800); // 30 * 60
    }

    #[test]
    fn test_unknown_process_stop() {
        let db = setup_test_db();
        let mut tracker = SessionTracker::new();
        let end_time = Utc::now();
        // Should not panic or err
        assert!(tracker.on_process_stopped(9999, end_time, &db).is_ok());
    }

    #[test]
    fn test_crash_recovery() {
        let db = setup_test_db();
        let start_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        database::create_session(&db, 101, start_time).unwrap();

        let current_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 10, 0).unwrap(); // 10 mins later
        SessionTracker::recover_sessions(&db, current_time).unwrap();

        let duration: i64 = db
            .query_row(
                "SELECT duration_seconds FROM game_sessions WHERE game_id = 101",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(duration, 600);
    }

    #[test]
    fn test_same_pid_multiple_starts() {
        let db = setup_test_db();
        let mut tracker = SessionTracker::new();
        let start_time = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();

        // Initial start (e.g., path was None, fallback matched)
        tracker.on_game_started(1234, 101, start_time, &db).unwrap();

        // Second start (e.g., path resolved)
        tracker.on_game_started(1234, 101, start_time, &db).unwrap();

        assert_eq!(tracker.active_sessions.len(), 1);
        let session = tracker.active_sessions.get(&101).unwrap();
        assert_eq!(session.pids.len(), 1);
        assert!(session.pids.contains(&1234));
    }
}
