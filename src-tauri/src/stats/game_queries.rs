use super::dto::DailyPlaytime;
use super::time::{bucket_label, bucket_labels, days_ago_start, overlap_seconds, split_by_bucket, today_start, Bucket};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct GameStatistics {
    pub total_playtime_seconds: i64,
    pub total_sessions: i64,
    pub average_session_seconds: i64,
    pub longest_session_seconds: i64,
    pub first_played_at: Option<DateTime<Utc>>,
    pub last_played_at: Option<DateTime<Utc>>,
    pub played_today_seconds: i64,
    pub played_week_seconds: i64,
    pub played_month_seconds: i64,
}

#[derive(Serialize)]
pub struct GameSessionDto {
    pub session_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: i64,
    pub is_active: bool,
}

/// A game's finished sessions that end at or after `since`, plus its running session.
fn play_intervals(
    db: &Connection,
    game_id: i64,
    since: DateTime<Utc>,
    active_started_at: Option<DateTime<Utc>>,
) -> Result<Vec<(DateTime<Utc>, DateTime<Utc>)>, String> {
    let now = Utc::now();
    let mut stmt = db
        .prepare(
            "SELECT started_at, ended_at FROM game_sessions
             WHERE game_id = ?1 AND ended_at IS NOT NULL AND ended_at >= ?2",
        )
        .map_err(|e| e.to_string())?;
    let mut intervals: Vec<(DateTime<Utc>, DateTime<Utc>)> = stmt
        .query_map(rusqlite::params![game_id, since], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())?;
    if let Some(started_at) = active_started_at {
        intervals.push((started_at, now));
    }
    Ok(intervals)
}

pub fn get_game_statistics(
    db: &Connection,
    game_id: i64,
    active_started_at: Option<DateTime<Utc>>,
) -> Result<GameStatistics, String> {
    let now = Utc::now();
    let active_elapsed = active_started_at
        .map(|started| (now - started).num_seconds().max(0))
        .unwrap_or(0);

    // Running sessions have an open row; it counts as a session but its time comes
    // from `active_elapsed`.
    let (total_sessions, finished_playtime, longest_finished, first_played, last_played): (
        i64,
        i64,
        i64,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
    ) = db
        .query_row(
            "SELECT COUNT(id), COALESCE(SUM(duration_seconds), 0), COALESCE(MAX(duration_seconds), 0),
                    MIN(started_at), MAX(started_at)
             FROM game_sessions WHERE game_id = ?1",
            rusqlite::params![game_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .map_err(|e| e.to_string())?;

    let total_playtime = finished_playtime + active_elapsed;
    let average_session = if total_sessions > 0 {
        total_playtime / total_sessions
    } else {
        0
    };
    let last_played = if active_started_at.is_some() {
        Some(now)
    } else {
        last_played
    };

    let (today, week, month) = (today_start(), days_ago_start(6), days_ago_start(29));
    let mut played = (0, 0, 0);
    for (start, end) in play_intervals(db, game_id, month, active_started_at)? {
        played.0 += overlap_seconds(start, end, today, now);
        played.1 += overlap_seconds(start, end, week, now);
        played.2 += overlap_seconds(start, end, month, now);
    }

    Ok(GameStatistics {
        total_playtime_seconds: total_playtime,
        total_sessions,
        average_session_seconds: average_session,
        longest_session_seconds: longest_finished.max(active_elapsed),
        first_played_at: first_played,
        last_played_at: last_played,
        played_today_seconds: played.0,
        played_week_seconds: played.1,
        played_month_seconds: played.2,
    })
}

pub fn get_game_sessions(
    db: &Connection,
    game_id: i64,
    limit: i64,
    offset: i64,
    active_session_started_at: Option<DateTime<Utc>>,
) -> Result<Vec<GameSessionDto>, String> {
    let mut stmt = db
        .prepare(
            "
        SELECT id, started_at, ended_at, duration_seconds
        FROM game_sessions
        WHERE game_id = ?1 AND ended_at IS NOT NULL
        ORDER BY started_at DESC
        LIMIT ?2 OFFSET ?3
    ",
        )
        .map_err(|e| e.to_string())?;

    let mut sessions = Vec::new();

    // Pre-insert the active session at the top if it exists and we're on page 0
    if offset == 0 {
        if let Some(started_at) = active_session_started_at {
            let elapsed = (Utc::now() - started_at).num_seconds().max(0);
            sessions.push(GameSessionDto {
                session_id: -1, // Virtual ID
                started_at,
                ended_at: None,
                duration_seconds: elapsed,
                is_active: true,
            });
        }
    }

    let iter = stmt
        .query_map(rusqlite::params![game_id, limit, offset], |row| {
            Ok(GameSessionDto {
                session_id: row.get(0)?,
                started_at: row.get(1)?,
                ended_at: row.get(2)?,
                duration_seconds: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                is_active: false,
            })
        })
        .map_err(|e| e.to_string())?;

    for item in iter {
        sessions.push(item.map_err(|e| e.to_string())?);
    }

    Ok(sessions)
}

/// Playtime per local day for the last `days` days, today included.
pub fn get_game_playtime_history(
    db: &Connection,
    game_id: i64,
    days: i64,
    active_started_at: Option<DateTime<Utc>>,
) -> Result<Vec<DailyPlaytime>, String> {
    let now = Utc::now();
    let from = days_ago_start(days.max(1) - 1);

    let mut daily_totals: BTreeMap<String, i64> = bucket_labels(from, now, Bucket::Day)
        .into_iter()
        .map(|label| (label, 0))
        .collect();
    for (start, end) in play_intervals(db, game_id, from, active_started_at)? {
        split_by_bucket(start.max(from), end.min(now), Bucket::Day, |piece, seconds| {
            *daily_totals.entry(bucket_label(piece, Bucket::Day)).or_insert(0) += seconds;
        });
    }

    Ok(daily_totals
        .into_iter()
        .map(|(date_label, playtime_seconds)| DailyPlaytime {
            date_label,
            playtime_seconds,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::run_migrations;
    use chrono::Duration;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO games (id, name, normalized_name) VALUES (1, 'Test', 'test')",
            [],
        )
        .unwrap();
        conn
    }

    fn insert_finished(db: &Connection, start: DateTime<Utc>, seconds: i64) {
        db.execute(
            "INSERT INTO game_sessions (game_id, started_at, ended_at, duration_seconds) VALUES (1, ?1, ?2, ?3)",
            rusqlite::params![start, start + Duration::seconds(seconds), seconds],
        )
        .unwrap();
    }

    #[test]
    fn test_get_game_statistics_empty() {
        let db = setup_test_db();
        let stats = get_game_statistics(&db, 1, None).unwrap();
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.total_playtime_seconds, 0);
        assert_eq!(stats.average_session_seconds, 0);
        assert!(stats.first_played_at.is_none());
    }

    #[test]
    fn test_get_game_statistics_with_sessions() {
        let db = setup_test_db();
        let now = Utc::now();
        insert_finished(&db, now - Duration::hours(1), 100);
        insert_finished(&db, now - Duration::hours(3), 300);

        let stats = get_game_statistics(&db, 1, None).unwrap();
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.total_playtime_seconds, 400);
        assert_eq!(stats.average_session_seconds, 200);
        assert_eq!(stats.longest_session_seconds, 300);
        assert_eq!(stats.played_month_seconds, 400);
    }

    #[test]
    fn test_get_game_sessions() {
        let db = setup_test_db();
        let now = Utc::now();
        insert_finished(&db, now - Duration::hours(1), 100);
        insert_finished(&db, now - Duration::hours(2), 200);
        // The running session's open row is represented by the virtual entry instead.
        crate::database::create_session(&db, 1, now).unwrap();

        let sessions = get_game_sessions(&db, 1, 10, 0, Some(now)).unwrap();
        // Active session + 2 historical = 3
        assert_eq!(sessions.len(), 3);
        assert!(sessions[0].is_active);
        assert_eq!(sessions[1].duration_seconds, 100);
        assert_eq!(sessions[2].duration_seconds, 200);
    }

    #[test]
    fn test_history_has_one_entry_per_local_day() {
        let db = setup_test_db();
        insert_finished(&db, Utc::now() - Duration::days(2), 600);
        let history = get_game_playtime_history(&db, 1, 7, None).unwrap();
        assert_eq!(history.len(), 7);
        assert_eq!(history.iter().map(|d| d.playtime_seconds).sum::<i64>(), 600);
    }
}
