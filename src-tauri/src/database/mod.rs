use rusqlite::{Connection, OptionalExtension, Result};
use std::fs;
use tauri::{AppHandle, Manager};

pub fn initialize_database(app: &AppHandle) -> Result<Connection> {
    let app_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir");

    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).expect("Failed to create app data dir");
    }

    let db_path = app_dir.join("gametracker.db");
    let conn = Connection::open(db_path)?;

    // Configure SQLite
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;",
    )?;

    run_migrations(&conn)?;

    Ok(conn)
}

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS games (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            normalized_name TEXT NOT NULL,
            cover_path TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            first_seen_at DATETIME,
            last_seen_at DATETIME,
            first_played_at DATETIME,
            last_played_at DATETIME,
            current_install_status TEXT DEFAULT 'Unknown',
            is_ignored BOOLEAN NOT NULL DEFAULT 0,
            is_removed BOOLEAN NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS game_platforms (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            game_id INTEGER NOT NULL,
            platform TEXT NOT NULL,
            external_id TEXT,
            FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS game_executables (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            game_id INTEGER NOT NULL,
            executable_name TEXT NOT NULL,
            executable_path TEXT,
            detection_confidence INTEGER DEFAULT 0,
            FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS game_installations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            game_id INTEGER NOT NULL,
            installed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            uninstalled_at DATETIME,
            installation_path TEXT,
            detection_source TEXT,
            FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS game_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            game_id INTEGER NOT NULL,
            started_at DATETIME NOT NULL,
            ended_at DATETIME,
            duration_seconds INTEGER,
            detection_source TEXT,
            last_heartbeat_at DATETIME,
            FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
        );
        ",
    )?;

    // Add is_ignored column if it doesn't exist (migration for existing DBs)
    let _ = conn.execute(
        "ALTER TABLE games ADD COLUMN is_ignored BOOLEAN NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE games ADD COLUMN is_removed BOOLEAN NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE game_sessions ADD COLUMN last_heartbeat_at DATETIME",
        [],
    );

    // Set default settings if not exists
    let _ = conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('tracking_enabled', 'true')",
        [],
    );
    let _ = conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('polling_interval_seconds', '2')",
        [],
    );

    Ok(())
}

use crate::models::{Game, GameExecutable};

pub fn get_all_executables(conn: &Connection) -> Result<Vec<GameExecutable>> {
    let mut stmt = conn.prepare("SELECT id, game_id, executable_name, executable_path, detection_confidence FROM game_executables")?;
    let exec_iter = stmt.query_map([], |row| {
        Ok(GameExecutable {
            id: row.get(0)?,
            game_id: row.get(1)?,
            executable_name: row.get(2)?,
            executable_path: row.get(3)?,
            detection_confidence: row.get(4)?,
        })
    })?;
    let mut execs = Vec::new();
    for exec in exec_iter {
        execs.push(exec?);
    }
    Ok(execs)
}

pub fn get_game_by_id(conn: &Connection, game_id: i64) -> Result<Option<Game>> {
    let mut stmt = conn.prepare("SELECT id, name, normalized_name, cover_path, created_at, updated_at, first_seen_at, last_seen_at, first_played_at, last_played_at, current_install_status, is_ignored, is_removed FROM games WHERE id = ?1")?;
    let mut iter = stmt.query_map([game_id], |row| {
        Ok(Game {
            id: row.get(0)?,
            name: row.get(1)?,
            normalized_name: row.get(2)?,
            cover_path: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            first_seen_at: row.get(6)?,
            last_seen_at: row.get(7)?,
            first_played_at: row.get(8)?,
            last_played_at: row.get(9)?,
            current_install_status: row.get(10)?,
            is_ignored: row.get(11)?,
            is_removed: row.get(12)?,
        })
    })?;
    if let Some(res) = iter.next() {
        Ok(Some(res?))
    } else {
        Ok(None)
    }
}

use crate::models::GameSession;
use chrono::{DateTime, Utc};

pub fn create_session(conn: &Connection, game_id: i64, started_at: DateTime<Utc>) -> Result<i64> {
    conn.execute("INSERT INTO game_sessions (game_id, started_at, last_heartbeat_at, detection_source) VALUES (?1, ?2, ?2, 'ProcessMonitor')", rusqlite::params![game_id, started_at])?;
    Ok(conn.last_insert_rowid())
}

/// Records that a session's game was still running at `seen_at`.
pub fn update_session_heartbeat(
    conn: &Connection,
    session_id: i64,
    seen_at: DateTime<Utc>,
) -> Result<()> {
    conn.execute(
        "UPDATE game_sessions SET last_heartbeat_at = ?1 WHERE id = ?2",
        rusqlite::params![seen_at, session_id],
    )?;
    Ok(())
}

pub fn close_session(
    conn: &Connection,
    session_id: i64,
    ended_at: DateTime<Utc>,
    duration_seconds: i64,
) -> Result<()> {
    conn.execute(
        "UPDATE game_sessions SET ended_at = ?1, duration_seconds = ?2 WHERE id = ?3",
        rusqlite::params![ended_at, duration_seconds, session_id],
    )?;
    Ok(())
}

/// Closes sessions left open by a crash or power loss. Each one ends at its last
/// heartbeat, the last moment the game was known to be running, rather than now,
/// which could be hours or days later.
pub fn recover_sessions(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT id, started_at, last_heartbeat_at FROM game_sessions WHERE ended_at IS NULL",
    )?;
    let rows: Vec<(i64, DateTime<Utc>, Option<DateTime<Utc>>)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .filter_map(Result::ok)
        .collect();

    for (id, started_at, last_heartbeat_at) in rows {
        // Sessions from before heartbeats existed have no known end, so they get no time.
        let ended_at = last_heartbeat_at.unwrap_or(started_at).max(started_at);
        let duration = (ended_at - started_at).num_seconds();
        close_session(conn, id, ended_at, duration)?;
    }
    Ok(())
}

pub fn get_recent_sessions(conn: &Connection, limit: u32) -> Result<Vec<GameSession>> {
    let mut stmt = conn.prepare("SELECT id, game_id, started_at, ended_at, duration_seconds, detection_source FROM game_sessions ORDER BY started_at DESC LIMIT ?1")?;
    let iter = stmt.query_map([limit], |row| {
        Ok(GameSession {
            id: row.get(0)?,
            game_id: row.get(1)?,
            started_at: row.get(2)?,
            ended_at: row.get(3)?,
            duration_seconds: row.get(4)?,
            detection_source: row.get(5)?,
        })
    })?;
    let mut res = Vec::new();
    for s in iter {
        res.push(s?);
    }
    Ok(res)
}

use crate::models::GameLibraryItem;

pub fn get_library_games(conn: &Connection) -> Result<Vec<GameLibraryItem>> {
    let mut stmt = conn.prepare("
        SELECT 
            g.id,
            g.name,
            g.cover_path,
            COALESCE(SUM(s.duration_seconds), 0) as total_playtime_seconds,
            MAX(s.started_at) as last_played_at,
            g.created_at,
            g.is_ignored,
            g.is_removed,
            g.current_install_status,
            (SELECT GROUP_CONCAT(DISTINCT platform) FROM game_platforms WHERE game_id = g.id) as platforms,
            (SELECT GROUP_CONCAT(DISTINCT installation_path) FROM game_installations WHERE game_id = g.id) as installation_paths
        FROM games g
        LEFT JOIN game_sessions s ON g.id = s.game_id
        WHERE g.is_removed = 0
        GROUP BY g.id
    ")?;

    let iter = stmt.query_map([], |row| {
        Ok(GameLibraryItem {
            id: row.get(0)?,
            name: row.get(1)?,
            cover_path: row.get(2)?,
            total_playtime_seconds: row.get(3)?,
            last_played_at: row.get(4)?,
            currently_playing: false,
            created_at: row.get(5)?,
            is_ignored: row.get(6)?,
            is_removed: row.get(7)?,
            current_install_status: row.get(8)?,
            platforms: row
                .get::<_, Option<String>>(9)?
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect(),
            installation_paths: row
                .get::<_, Option<String>>(10)?
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect(),
        })
    })?;

    let mut items = Vec::new();
    for item in iter {
        items.push(item?);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_get_library_games() {
        let conn = setup_test_db();
        // Insert game
        conn.execute(
            "INSERT INTO games (id, name, normalized_name) VALUES (1, 'Test Game', 'testgame')",
            [],
        )
        .unwrap();

        // Insert sessions
        let started_1 = chrono::Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
        conn.execute(
            "INSERT INTO game_sessions (game_id, started_at, duration_seconds) VALUES (1, ?1, 100)",
            rusqlite::params![started_1],
        )
        .unwrap();
        let started_2 = chrono::Utc.with_ymd_and_hms(2023, 1, 2, 12, 0, 0).unwrap();
        conn.execute(
            "INSERT INTO game_sessions (game_id, started_at, duration_seconds) VALUES (1, ?1, 200)",
            rusqlite::params![started_2],
        )
        .unwrap();

        // Insert game with no sessions
        conn.execute(
            "INSERT INTO games (id, name, normalized_name) VALUES (2, 'Empty Game', 'emptygame')",
            [],
        )
        .unwrap();

        let library = get_library_games(&conn).unwrap();
        assert_eq!(library.len(), 2);

        let game1 = library.iter().find(|g| g.id == 1).unwrap();
        assert_eq!(game1.total_playtime_seconds, 300);
        assert_eq!(game1.last_played_at.unwrap(), started_2);

        let game2 = library.iter().find(|g| g.id == 2).unwrap();
        assert_eq!(game2.total_playtime_seconds, 0);
        assert!(game2.last_played_at.is_none());
    }
}

use crate::discovery::models::GameCandidate;

pub fn add_discovered_game(conn: &Connection, candidate: &GameCandidate) -> Result<i64> {
    let normalized_name = candidate.name.to_lowercase().replace(' ', "");

    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM games WHERE normalized_name = ?1",
            rusqlite::params![normalized_name],
            |r| r.get(0),
        )
        .optional()?;

    let game_id = if let Some(id) = existing_id {
        id
    } else {
        conn.execute(
            "INSERT INTO games (name, normalized_name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![candidate.name, normalized_name, chrono::Utc::now(), chrono::Utc::now()]
        )?;
        conn.last_insert_rowid()
    };

    let inst_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM game_installations WHERE game_id = ?1 AND installation_path = ?2",
            rusqlite::params![game_id, candidate.install_path],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if inst_exists == 0 {
        conn.execute(
            "INSERT INTO game_installations (game_id, installation_path, detection_source) VALUES (?1, ?2, ?3)",
            rusqlite::params![game_id, candidate.install_path, candidate.source]
        )?;
    }

    for exe in &candidate.executables {
        let path_obj = std::path::Path::new(exe);
        let exe_name = path_obj
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let exe_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM game_executables WHERE game_id = ?1 AND executable_path = ?2",
                rusqlite::params![game_id, exe],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if exe_exists == 0 {
            conn.execute(
                "INSERT INTO game_executables (game_id, executable_name, executable_path, detection_confidence) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![game_id, exe_name, exe, candidate.confidence]
            )?;
        }
    }

    Ok(game_id)
}

#[cfg(test)]
mod discovery_tests {
    use super::*;
    use crate::discovery::models::GameCandidate;

    #[test]
    fn test_add_discovered_game_no_duplicates() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        let candidate = GameCandidate {
            name: "Cyberpunk 2077".to_string(),
            install_path: "C:\\Games\\Cyberpunk".to_string(),
            executables: vec!["C:\\Games\\Cyberpunk\\bin\\x64\\Cyberpunk2077.exe".to_string()],
            source: "Folder".to_string(),
            confidence: 90,
            platform_app_id: None,
        };

        let id1 = add_discovered_game(&conn, &candidate).unwrap();
        let mut candidate2 = candidate.clone();
        candidate2
            .executables
            .push("C:\\Games\\Cyberpunk\\bin\\x64\\RedEngine.exe".to_string());
        let id2 = add_discovered_game(&conn, &candidate2).unwrap();

        assert_eq!(id1, id2);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM games", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let exe_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM game_executables", [], |r| r.get(0))
            .unwrap();
        assert_eq!(exe_count, 2);
    }
}
