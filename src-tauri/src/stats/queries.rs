use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Utc};
use rusqlite::Connection;
use std::collections::{BTreeMap, HashMap};

use super::dto::{DailyPlaytime, DashboardStats, DayOfWeekActivity, TopGame};
use super::time::{
    bucket_labels, bucket_label, days_ago_start, overlap_seconds, split_by_bucket, today_start,
    Bucket,
};
use crate::tracking::tracker::ActiveSession;

/// A stretch of play: a finished session from the database or one that's running now.
struct PlayInterval {
    game_id: i64,
    name: String,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

/// Finished sessions that end at or after `since`. Running sessions also have an open
/// row in the database; they're left out here and taken from the tracker instead, so
/// each one is counted exactly once.
fn finished_sessions(db: &Connection, since: DateTime<Utc>) -> rusqlite::Result<Vec<PlayInterval>> {
    let mut stmt = db.prepare(
        "SELECT s.game_id, g.name, s.started_at, s.ended_at
         FROM game_sessions s JOIN games g ON g.id = s.game_id
         WHERE s.ended_at IS NOT NULL AND s.ended_at >= ?1",
    )?;
    let rows = stmt.query_map(rusqlite::params![since], |row| {
        Ok(PlayInterval {
            game_id: row.get(0)?,
            name: row.get(1)?,
            start: row.get(2)?,
            end: row.get(3)?,
        })
    })?;
    rows.collect()
}

fn running_sessions(
    db: &Connection,
    active_sessions: &[ActiveSession],
    now: DateTime<Utc>,
) -> Vec<PlayInterval> {
    active_sessions
        .iter()
        .map(|session| PlayInterval {
            game_id: session.game_id,
            name: db
                .query_row(
                    "SELECT name FROM games WHERE id = ?1",
                    [session.game_id],
                    |r| r.get(0),
                )
                .unwrap_or_default(),
            start: session.started_at,
            end: now,
        })
        .collect()
}

pub fn get_dashboard_stats(
    db: &Connection,
    active_sessions: &[ActiveSession],
    time_range: &str, // "7d", "30d", "all", "today"
) -> rusqlite::Result<DashboardStats> {
    let now = Utc::now();
    let today = today_start();
    let week_start = days_ago_start(6);

    let since = match time_range {
        "today" => today.min(week_start),
        "7d" => week_start,
        "30d" => days_ago_start(29),
        _ => Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(),
    };
    let mut sessions = finished_sessions(db, since)?;
    sessions.extend(running_sessions(db, active_sessions, now));
    for session in &mut sessions {
        session.end = session.end.min(now);
    }

    let (range_start, bucket) = match time_range {
        "today" => (today, Bucket::Hour),
        "7d" => (week_start, Bucket::Day),
        "30d" => (days_ago_start(29), Bucket::Day),
        _ => {
            let first = sessions.iter().map(|s| s.start).min().unwrap_or(today);
            let first_day = super::time::local_day_start(first.with_timezone(&Local).date_naive())
                .with_timezone(&Utc);
            // Long histories are shown per month so the chart stays readable.
            let bucket = if now - first_day > Duration::days(90) {
                Bucket::Month
            } else {
                Bucket::Day
            };
            (first_day, bucket)
        }
    };

    let mut buckets: BTreeMap<String, i64> = bucket_labels(range_start, now, bucket)
        .into_iter()
        .map(|label| (label, 0))
        .collect();
    let dow_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut dow_seconds = [0i64; 7];
    let mut game_playtimes: HashMap<i64, (String, i64)> = HashMap::new();
    let mut total_sessions = 0;
    let mut today_playtime_seconds = 0;
    let mut week_playtime_seconds = 0;

    for session in &sessions {
        today_playtime_seconds += overlap_seconds(session.start, session.end, today, now);
        week_playtime_seconds += overlap_seconds(session.start, session.end, week_start, now);

        let in_range = overlap_seconds(session.start, session.end, range_start, now);
        if in_range == 0 {
            continue;
        }
        total_sessions += 1;
        game_playtimes
            .entry(session.game_id)
            .or_insert_with(|| (session.name.clone(), 0))
            .1 += in_range;

        let clipped_start = session.start.max(range_start);
        split_by_bucket(clipped_start, session.end, bucket, |piece, seconds| {
            *buckets.entry(bucket_label(piece, bucket)).or_insert(0) += seconds;
        });
        split_by_bucket(clipped_start, session.end, Bucket::Day, |piece, seconds| {
            dow_seconds[piece.weekday().num_days_from_monday() as usize] += seconds;
        });
    }

    let mut sorted_games: Vec<TopGame> = game_playtimes
        .into_iter()
        .map(|(game_id, (name, playtime_seconds))| TopGame {
            game_id,
            name,
            playtime_seconds,
        })
        .collect();
    sorted_games.sort_by(|a, b| b.playtime_seconds.cmp(&a.playtime_seconds));

    Ok(DashboardStats {
        total_playtime_seconds: sorted_games.iter().map(|g| g.playtime_seconds).sum(),
        games_played: sorted_games.len() as i64,
        total_sessions,
        today_playtime_seconds,
        week_playtime_seconds,
        most_played_game: sorted_games.first().map(|g| g.name.clone()),
        most_played_game_playtime: sorted_games.first().map_or(0, |g| g.playtime_seconds),
        playtime_over_time: buckets
            .into_iter()
            .map(|(date_label, playtime_seconds)| DailyPlaytime {
                date_label,
                playtime_seconds,
            })
            .collect(),
        top_games: sorted_games.into_iter().take(5).collect(),
        activity_by_day_of_week: dow_names
            .iter()
            .zip(dow_seconds)
            .map(|(day, playtime_seconds)| DayOfWeekActivity {
                day: day.to_string(),
                playtime_seconds,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{create_session, run_migrations};
    use crate::stats::time::local_day_start;
    use std::collections::HashSet;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO games (id, name, normalized_name) VALUES (1, 'Hades', 'hades')",
            [],
        )
        .unwrap();
        conn
    }

    fn insert_finished(db: &Connection, start: DateTime<Utc>, end: DateTime<Utc>) {
        db.execute(
            "INSERT INTO game_sessions (game_id, started_at, ended_at, duration_seconds) VALUES (1, ?1, ?2, ?3)",
            rusqlite::params![start, end, (end - start).num_seconds()],
        )
        .unwrap();
    }

    fn local_time(days_ago: i64, hour: i64) -> DateTime<Utc> {
        (local_day_start(Local::now().date_naive() - Duration::days(days_ago)) + Duration::hours(hour))
            .with_timezone(&Utc)
    }

    #[test]
    fn test_get_dashboard_stats_empty() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let stats = get_dashboard_stats(&conn, &[], "all").unwrap();
        assert_eq!(stats.total_playtime_seconds, 0);
        assert_eq!(stats.games_played, 0);
        assert_eq!(stats.total_sessions, 0);
    }

    #[test]
    fn test_running_session_counted_once() {
        let db = setup_test_db();
        let started = Utc::now() - Duration::minutes(30);
        let session_id = create_session(&db, 1, started).unwrap();
        let active = ActiveSession {
            session_id,
            game_id: 1,
            pids: HashSet::from([42]),
            started_at: started,
            last_heartbeat_at: started,
        };

        let stats = get_dashboard_stats(&db, &[active], "7d").unwrap();

        assert_eq!(stats.total_sessions, 1);
        assert!((1790..=1810).contains(&stats.total_playtime_seconds));
    }

    #[test]
    fn test_totals_follow_the_selected_range() {
        let db = setup_test_db();
        insert_finished(&db, local_time(10, 12), local_time(10, 14));
        insert_finished(&db, local_time(2, 12), local_time(2, 13));

        let week = get_dashboard_stats(&db, &[], "7d").unwrap();
        assert_eq!(week.total_playtime_seconds, 3600);
        assert_eq!(week.total_sessions, 1);
        assert_eq!(week.top_games[0].playtime_seconds, 3600);

        let all = get_dashboard_stats(&db, &[], "all").unwrap();
        assert_eq!(all.total_playtime_seconds, 3 * 3600);
        assert_eq!(all.total_sessions, 2);
        // Days without play are still on the chart, as zero.
        assert_eq!(all.playtime_over_time.len(), 11);
    }

    #[test]
    fn test_session_across_midnight_counts_on_both_days() {
        let db = setup_test_db();
        insert_finished(&db, local_time(3, 23), local_time(3, 25));

        let stats = get_dashboard_stats(&db, &[], "7d").unwrap();

        let nonzero: Vec<i64> = stats
            .playtime_over_time
            .iter()
            .map(|d| d.playtime_seconds)
            .filter(|s| *s > 0)
            .collect();
        assert_eq!(nonzero, [3600, 3600]);
    }
}
