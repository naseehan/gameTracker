use chrono::{DateTime, Duration, Local, TimeZone, Utc};
use rusqlite::Connection;
use std::collections::HashMap;

use super::dto::{DailyPlaytime, DashboardStats, DayOfWeekActivity, TopGame};
use crate::tracking::tracker::ActiveSession;

pub fn get_dashboard_stats(
    db: &Connection,
    active_sessions: &[ActiveSession],
    time_range: &str, // "7d", "30d", "all", "today"
) -> rusqlite::Result<DashboardStats> {
    let now_utc = Utc::now();
    let now_local = Local::now();

    // Calculate start of today (local time midnight, converted to UTC)
    let start_of_today_local = now_local
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap();
    let start_of_today_utc = start_of_today_local.with_timezone(&Utc);

    // Start of 7 days ago (including today)
    let start_of_week_utc = (start_of_today_local - Duration::days(6)).with_timezone(&Utc);

    // Global aggregations from SQL
    let total_playtime_sql: i64 = db.query_row(
        "SELECT COALESCE(SUM(duration_seconds), 0) FROM game_sessions",
        [],
        |r| r.get(0),
    )?;

    let mut total_sessions_sql: i64 =
        db.query_row("SELECT COUNT(*) FROM game_sessions", [], |r| r.get(0))?;

    // Aggregate active sessions globally
    let mut active_playtime_total = 0;
    let mut active_game_additions: HashMap<i64, i64> = HashMap::new(); // game_id -> duration

    for session in active_sessions {
        let elapsed = (now_utc - session.started_at).num_seconds().max(0);
        active_playtime_total += elapsed;
        *active_game_additions.entry(session.game_id).or_insert(0) += elapsed;
        // active sessions count as sessions too
        total_sessions_sql += 1;
    }

    let total_playtime_seconds = total_playtime_sql + active_playtime_total;

    // Fetch all games' playtime for Top Games and Games Played
    let mut stmt = db.prepare(
        "
        SELECT s.game_id, g.name, COALESCE(SUM(s.duration_seconds), 0)
        FROM game_sessions s
        JOIN games g ON s.game_id = g.id
        GROUP BY s.game_id
    ",
    )?;

    let mut game_playtimes: HashMap<i64, (String, i64)> = HashMap::new();
    let rows = stmt.query_map([], |row| {
        let game_id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        let duration: i64 = row.get(2)?;
        Ok((game_id, name, duration))
    })?;

    for row in rows {
        if let Ok((game_id, name, duration)) = row {
            game_playtimes.insert(game_id, (name, duration));
        }
    }

    // Add active playtime to the game playtimes map
    for (game_id, active_duration) in &active_game_additions {
        if let Some(entry) = game_playtimes.get_mut(game_id) {
            entry.1 += active_duration;
        } else {
            // If the game has no completed sessions, we need its name
            if let Ok(Some(name)) =
                db.query_row("SELECT name FROM games WHERE id = ?1", [game_id], |r| {
                    r.get(0)
                })
            {
                game_playtimes.insert(*game_id, (name, *active_duration));
            }
        }
    }

    let games_played = game_playtimes.len() as i64;

    // Sort games by playtime to find Most Played and Top Games
    let mut sorted_games: Vec<TopGame> = game_playtimes
        .into_iter()
        .map(|(game_id, (name, playtime_seconds))| TopGame {
            game_id,
            name,
            playtime_seconds,
        })
        .collect();

    sorted_games.sort_by(|a, b| b.playtime_seconds.cmp(&a.playtime_seconds));

    let most_played_game = sorted_games.first().map(|g| g.name.clone());
    let most_played_game_playtime = sorted_games
        .first()
        .map(|g| g.playtime_seconds)
        .unwrap_or(0);

    let top_games = sorted_games.into_iter().take(5).collect();

    // Fetch sessions for the time range (Today, Week, or selected range)
    // We'll fetch from the maximum range we need (start of week OR the selected range)
    let range_start_utc = match time_range {
        "today" => start_of_today_utc,
        "7d" => start_of_week_utc,
        "30d" => (start_of_today_local - Duration::days(29)).with_timezone(&Utc),
        _ => Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(), // "all"
    };

    // Calculate Today and Week playtimes specifically
    let mut today_playtime_seconds = 0;
    let mut week_playtime_seconds = 0;

    // We'll also bucket for the chart
    let mut daily_buckets: HashMap<String, i64> = HashMap::new();
    let mut dow_buckets: HashMap<String, i64> = HashMap::new();

    // Fill days with 0 for the selected range to prevent misleading zeros
    let days_to_fill = match time_range {
        "today" => 1,
        "7d" => 7,
        "30d" => 30,
        _ => 0, // "all" will just show days that have data, or we could calculate diff
    };

    if days_to_fill > 0 {
        for i in 0..days_to_fill {
            let d = start_of_today_local - Duration::days(i as i64);
            let label = d.format("%Y-%m-%d").to_string();
            daily_buckets.insert(label, 0);
        }
    }

    // Initialize DOW buckets
    let dow_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    for name in &dow_names {
        dow_buckets.insert(name.to_string(), 0);
    }

    // Process completed sessions
    let mut session_stmt = db
        .prepare("SELECT started_at, duration_seconds FROM game_sessions WHERE started_at >= ?1")?;

    // For calculating global today/week metrics, we use the furthest lookback needed
    let metrics_start = if range_start_utc < start_of_week_utc {
        range_start_utc
    } else {
        start_of_week_utc
    };

    let session_rows = session_stmt.query_map(rusqlite::params![metrics_start], |row| {
        let started_utc: DateTime<Utc> = row.get(0)?;
        let duration: i64 = row.get(1)?;
        Ok((started_utc, duration))
    })?;

    for row in session_rows {
        if let Ok((started_utc, duration)) = row {
            if started_utc >= start_of_today_utc {
                today_playtime_seconds += duration;
            }
            if started_utc >= start_of_week_utc {
                week_playtime_seconds += duration;
            }

            if started_utc >= range_start_utc || time_range == "all" {
                let local_start = started_utc.with_timezone(&Local);
                let day_label = local_start.format("%Y-%m-%d").to_string();
                let dow_label = local_start.format("%a").to_string();

                *daily_buckets.entry(day_label).or_insert(0) += duration;
                *dow_buckets.entry(dow_label).or_insert(0) += duration;
            }
        }
    }

    // Add active sessions to time buckets
    for session in active_sessions {
        let elapsed = (now_utc - session.started_at).num_seconds().max(0);

        if session.started_at >= start_of_today_utc {
            today_playtime_seconds += elapsed;
        }
        if session.started_at >= start_of_week_utc {
            week_playtime_seconds += elapsed;
        }

        if session.started_at >= range_start_utc || time_range == "all" {
            let local_start = session.started_at.with_timezone(&Local);
            let day_label = local_start.format("%Y-%m-%d").to_string();
            let dow_label = local_start.format("%a").to_string();

            *daily_buckets.entry(day_label).or_insert(0) += elapsed;
            *dow_buckets.entry(dow_label).or_insert(0) += elapsed;
        }
    }

    // Format chart data
    let mut playtime_over_time: Vec<DailyPlaytime> = daily_buckets
        .into_iter()
        .map(|(date_label, playtime_seconds)| DailyPlaytime {
            date_label,
            playtime_seconds,
        })
        .collect();
    playtime_over_time.sort_by(|a, b| a.date_label.cmp(&b.date_label));

    let mut activity_by_day_of_week: Vec<DayOfWeekActivity> = Vec::new();
    for name in &dow_names {
        activity_by_day_of_week.push(DayOfWeekActivity {
            day: name.to_string(),
            playtime_seconds: *dow_buckets.get(*name).unwrap_or(&0),
        });
    }

    Ok(DashboardStats {
        total_playtime_seconds,
        games_played,
        total_sessions: total_sessions_sql,
        today_playtime_seconds,
        week_playtime_seconds,
        most_played_game,
        most_played_game_playtime,
        playtime_over_time,
        top_games,
        activity_by_day_of_week,
    })
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
    fn test_get_dashboard_stats_empty() {
        let conn = setup_test_db();
        let stats = get_dashboard_stats(&conn, &[], "all").unwrap();
        assert_eq!(stats.total_playtime_seconds, 0);
        assert_eq!(stats.games_played, 0);
        assert_eq!(stats.total_sessions, 0);
    }
}
