use super::dto::DailyPlaytime;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use rusqlite::Connection;
use serde::Serialize;

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

// Ensure the time range logic uses UTC equivalent for 'today', 'week', 'month' bounds
fn get_time_bounds() -> (DateTime<Utc>, DateTime<Utc>, DateTime<Utc>) {
    let now = Utc::now();
    let today_start = Utc
        .with_ymd_and_hms(
            now.date_naive().year(),
            now.date_naive().month(),
            now.date_naive().day(),
            0,
            0,
            0,
        )
        .unwrap();
    let week_start = today_start - chrono::Duration::days(6); // Last 7 days including today
    let month_start = today_start - chrono::Duration::days(29); // Last 30 days including today

    (today_start, week_start, month_start)
}

pub fn get_game_statistics(
    db: &Connection,
    game_id: i64,
    active_session_elapsed: i64,
) -> Result<GameStatistics, String> {
    let (today_start, week_start, month_start) = get_time_bounds();

    let mut stmt = db
        .prepare(
            "
        SELECT 
            COUNT(id) as total_sessions,
            COALESCE(SUM(duration_seconds), 0) as total_playtime,
            COALESCE(MAX(duration_seconds), 0) as longest_session,
            MIN(started_at) as first_played,
            MAX(started_at) as last_played
        FROM game_sessions
        WHERE game_id = ?1
    ",
        )
        .map_err(|e| e.to_string())?;

    let mut stats = stmt
        .query_row(rusqlite::params![game_id], |row| {
            let total_sessions: i64 = row.get(0)?;
            let mut total_playtime: i64 = row.get(1)?;
            let mut longest_session: i64 = row.get(2)?;
            let mut first_played: Option<DateTime<Utc>> = row.get(3)?;
            let mut last_played: Option<DateTime<Utc>> = row.get(4)?;

            Ok((
                total_sessions,
                total_playtime,
                longest_session,
                first_played,
                last_played,
            ))
        })
        .map_err(|e| e.to_string())?;

    // Incorporate active session time
    let total_sessions = stats.0;
    let mut total_playtime = stats.1 + active_session_elapsed;
    let longest_session = stats.2.max(active_session_elapsed);

    // Fallback active session times if it's the very first session
    let first_played = stats.3.or_else(|| {
        if active_session_elapsed > 0 {
            Some(Utc::now())
        } else {
            None
        }
    });
    let last_played = stats.4.or_else(|| {
        if active_session_elapsed > 0 {
            Some(Utc::now())
        } else {
            None
        }
    });
    // Or if there is an active session, last_played is definitely "now" (or when it started).
    let last_played = if active_session_elapsed > 0 {
        Some(Utc::now())
    } else {
        last_played
    };

    let average_session = if total_sessions > 0 {
        total_playtime / total_sessions
    } else {
        if active_session_elapsed > 0 {
            active_session_elapsed
        } else {
            0
        }
    };

    // Calculate Today, Week, Month
    let mut played_today = active_session_elapsed;
    let mut played_week = active_session_elapsed;
    let mut played_month = active_session_elapsed;

    let mut range_stmt = db
        .prepare(
            "
        SELECT started_at, duration_seconds 
        FROM game_sessions 
        WHERE game_id = ?1 AND started_at >= ?2
    ",
        )
        .map_err(|e| e.to_string())?;

    let iter = range_stmt
        .query_map(rusqlite::params![game_id, month_start], |row| {
            Ok((row.get::<_, DateTime<Utc>>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| e.to_string())?;

    for item in iter.flatten() {
        let (started_at, duration) = item;
        played_month += duration;

        if started_at >= week_start {
            played_week += duration;
        }

        if started_at >= today_start {
            played_today += duration;
        }
    }

    Ok(GameStatistics {
        total_playtime_seconds: total_playtime,
        total_sessions,
        average_session_seconds: average_session,
        longest_session_seconds: longest_session,
        first_played_at: first_played,
        last_played_at: last_played,
        played_today_seconds: played_today,
        played_week_seconds: played_week,
        played_month_seconds: played_month,
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
        WHERE game_id = ?1
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
                duration_seconds: row.get(3)?,
                is_active: false,
            })
        })
        .map_err(|e| e.to_string())?;

    for item in iter.flatten() {
        sessions.push(item);
    }

    Ok(sessions)
}

pub fn get_game_playtime_history(
    db: &Connection,
    game_id: i64,
    days: i64,
    active_session_elapsed: i64,
) -> Result<Vec<DailyPlaytime>, String> {
    let now = Utc::now();
    let start_date = now - chrono::Duration::days(days - 1);
    let start_date_normalized = Utc
        .with_ymd_and_hms(
            start_date.date_naive().year(),
            start_date.date_naive().month(),
            start_date.date_naive().day(),
            0,
            0,
            0,
        )
        .unwrap();

    let mut stmt = db
        .prepare(
            "
        SELECT started_at, duration_seconds
        FROM game_sessions
        WHERE game_id = ?1 AND started_at >= ?2
    ",
        )
        .map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map(rusqlite::params![game_id, start_date_normalized], |row| {
            Ok((row.get::<_, DateTime<Utc>>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| e.to_string())?;

    // Create a zero-filled vector for the last X days
    let mut daily_totals: std::collections::BTreeMap<String, i64> =
        std::collections::BTreeMap::new();
    for i in 0..days {
        let d = now - chrono::Duration::days(days - 1 - i);
        let date_str = d.format("%Y-%m-%d").to_string();
        daily_totals.insert(date_str, 0);
    }

    // Populate with historical sessions
    for item in iter.flatten() {
        let (started_at, duration) = item;
        let date_str = started_at.format("%Y-%m-%d").to_string();
        if let Some(total) = daily_totals.get_mut(&date_str) {
            *total += duration;
        }
    }

    // Add active session elapsed time to today
    if active_session_elapsed > 0 {
        let date_str = now.format("%Y-%m-%d").to_string();
        if let Some(total) = daily_totals.get_mut(&date_str) {
            *total += active_session_elapsed;
        }
    }

    let mut result = Vec::new();
    for (date, seconds) in daily_totals {
        result.push(DailyPlaytime {
            date_label: date,
            playtime_seconds: seconds,
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::run_migrations;
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_get_game_statistics_empty() {
        let db = setup_test_db();
        let stats = get_game_statistics(&db, 1, 0).unwrap();
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.total_playtime_seconds, 0);
        assert_eq!(stats.average_session_seconds, 0);
        assert!(stats.first_played_at.is_none());
    }

    #[test]
    fn test_get_game_statistics_with_sessions() {
        let db = setup_test_db();
        db.execute(
            "INSERT INTO games (id, name, normalized_name) VALUES (1, 'Test', 'test')",
            [],
        )
        .unwrap();
        // Insert dummy sessions
        let now = Utc::now();
        db.execute(
            "INSERT INTO game_sessions (game_id, started_at, duration_seconds) VALUES (1, ?1, 100)",
            rusqlite::params![now],
        )
        .unwrap();
        db.execute(
            "INSERT INTO game_sessions (game_id, started_at, duration_seconds) VALUES (1, ?1, 300)",
            rusqlite::params![now - chrono::Duration::hours(2)],
        )
        .unwrap();

        let stats = get_game_statistics(&db, 1, 0).unwrap();
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.total_playtime_seconds, 400);
        assert_eq!(stats.average_session_seconds, 200);
        assert_eq!(stats.longest_session_seconds, 300);
    }

    #[test]
    fn test_get_game_sessions() {
        let db = setup_test_db();
        let now = Utc::now();
        db.execute("INSERT OR IGNORE INTO games (id, name, normalized_name) VALUES (1, \'Test\', \'test\')", []).unwrap();
        db.execute(
            "INSERT INTO game_sessions (game_id, started_at, duration_seconds) VALUES (1, ?1, 100)",
            rusqlite::params![now - chrono::Duration::hours(1)],
        )
        .unwrap();
        db.execute(
            "INSERT INTO game_sessions (game_id, started_at, duration_seconds) VALUES (1, ?1, 200)",
            rusqlite::params![now - chrono::Duration::hours(2)],
        )
        .unwrap();

        let sessions = get_game_sessions(&db, 1, 10, 0, Some(now)).unwrap();
        // Active session + 2 historical = 3
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[0].is_active, true);
        assert_eq!(sessions[1].duration_seconds, 100);
        assert_eq!(sessions[2].duration_seconds, 200);
    }
}
