use rusqlite::{Connection, OptionalExtension, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub mod editing;

const DB_FILE: &str = "gametracker.db";
/// The app id before the app was named GameTracker. Its data folder was named after it.
const LEGACY_APP_ID: &str = "com.naseeh.tauri-app";

pub fn initialize_database(app: &AppHandle) -> Result<Connection> {
    let app_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data dir");

    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).expect("Failed to create app data dir");
    }
    let migrated_from = copy_legacy_data_dir(&app_dir);

    let db_path = app_dir.join(DB_FILE);
    let mut conn = Connection::open(db_path)?;

    // Configure SQLite
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;",
    )?;

    run_migrations(&conn)?;
    if let Some(legacy_dir) = migrated_from {
        if let Err(e) = move_cover_paths(&conn, &legacy_dir.join("covers"), &app_dir.join("covers")) {
            log::error!("Failed to update cover paths after moving data: {}", e);
        }
    }

    // One-time cleanup of games that were added twice under slightly different names.
    // New duplicates are prevented when games are added, so this doesn't need to rerun.
    if crate::settings::get_setting(&conn, "library_names_v2")?.is_none() {
        match merge_duplicate_games(&mut conn) {
            Ok(merged) => {
                log::info!("Merged {} duplicate games", merged);
                crate::settings::set_setting(&conn, "library_names_v2", "true")?;
            }
            // A failed merge leaves the library as it was; it's retried on the next start.
            Err(e) => log::error!("Failed to merge duplicate games: {}", e),
        }
    }

    Ok(conn)
}

/// Copies the database and covers from the folder of the app's old id the first time the
/// renamed app starts. The old folder is kept as a backup. Returns the old folder if
/// anything was copied.
fn copy_legacy_data_dir(app_dir: &Path) -> Option<PathBuf> {
    let legacy_dir = app_dir.parent()?.join(LEGACY_APP_ID);
    if legacy_dir == app_dir || app_dir.join(DB_FILE).exists() || !legacy_dir.join(DB_FILE).exists() {
        return None;
    }

    for file in [DB_FILE, "gametracker.db-wal", "gametracker.db-shm"] {
        let source = legacy_dir.join(file);
        if source.exists() {
            if let Err(e) = fs::copy(&source, app_dir.join(file)) {
                log::error!("Failed to copy {} from the old data folder: {}", file, e);
                let _ = fs::remove_file(app_dir.join(DB_FILE));
                return None;
            }
        }
    }

    let covers = legacy_dir.join("covers");
    if let Ok(entries) = fs::read_dir(&covers) {
        let _ = fs::create_dir_all(app_dir.join("covers"));
        for entry in entries.flatten() {
            let _ = fs::copy(entry.path(), app_dir.join("covers").join(entry.file_name()));
        }
    }

    log::info!("Copied data from {} to {}", legacy_dir.display(), app_dir.display());
    Some(legacy_dir)
}

/// Points covers stored in the old data folder at their copies in the new one.
fn move_cover_paths(conn: &Connection, old_covers: &Path, new_covers: &Path) -> Result<()> {
    let covers: Vec<(i64, String)> = {
        let mut stmt = conn.prepare("SELECT id, cover_path FROM games WHERE cover_path IS NOT NULL")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<Result<_>>()?
    };
    for (id, cover) in covers {
        if let Ok(file) = Path::new(&cover).strip_prefix(old_covers) {
            conn.execute(
                "UPDATE games SET cover_path = ?1 WHERE id = ?2",
                rusqlite::params![new_covers.join(file).to_string_lossy(), id],
            )?;
        }
    }
    Ok(())
}

/// The key used to recognise the same game under slightly different names, e.g.
/// "Cyberpunk 2077" from Steam and "CYBERPUNK 2077™" added by hand.
pub fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Brings every game's normalized name up to date, then merges games that are in the
/// library twice (e.g. added by hand and later found on Steam). The oldest entry keeps
/// everything from the newer one: sessions, platforms, installs and executables.
/// Removed games are left alone. Returns how many duplicates were merged.
pub fn merge_duplicate_games(conn: &mut Connection) -> Result<usize> {
    let tx = conn.transaction()?;

    let games: Vec<(i64, String, bool)> = {
        let mut stmt = tx.prepare("SELECT id, name, is_removed FROM games ORDER BY id")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        rows.collect::<Result<_>>()?
    };

    let mut first_by_name: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut merged = 0;
    for (id, name, is_removed) in games {
        let key = normalize_name(&name);
        tx.execute(
            "UPDATE games SET normalized_name = ?1 WHERE id = ?2 AND normalized_name != ?1",
            rusqlite::params![key, id],
        )?;
        if is_removed || key.is_empty() {
            continue;
        }
        match first_by_name.get(&key) {
            Some(&keep_id) => {
                merge_game_into(&tx, id, keep_id)?;
                merged += 1;
            }
            None => {
                first_by_name.insert(key, id);
            }
        }
    }

    tx.commit()?;
    Ok(merged)
}

fn merge_game_into(tx: &rusqlite::Transaction, from_id: i64, into_id: i64) -> Result<()> {
    for table in [
        "game_sessions",
        "game_platforms",
        "game_installations",
        "game_executables",
    ] {
        tx.execute(
            &format!("UPDATE {} SET game_id = ?1 WHERE game_id = ?2", table),
            rusqlite::params![into_id, from_id],
        )?;
    }
    // Keep the duplicate's cover if the kept game has none, and treat the game as
    // installed or ignored if either entry was.
    tx.execute(
        "UPDATE games SET
            cover_path = COALESCE(cover_path, (SELECT cover_path FROM games WHERE id = ?2)),
            current_install_status = CASE
                WHEN (SELECT current_install_status FROM games WHERE id = ?2) = 'Installed' THEN 'Installed'
                ELSE current_install_status END,
            is_ignored = is_ignored OR (SELECT is_ignored FROM games WHERE id = ?2),
            updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1",
        rusqlite::params![into_id, from_id],
    )?;
    tx.execute("DELETE FROM games WHERE id = ?1", rusqlite::params![from_id])?;
    Ok(())
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
    // When cover art was last looked up for a game without a cover.
    let _ = conn.execute("ALTER TABLE games ADD COLUMN cover_lookup_at DATETIME", []);

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

use crate::detection::detector::{GameDetector, GameInstallDir};
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

/// Install folders of the games in the library, used to recognise their processes.
pub fn get_install_dirs(conn: &Connection) -> Result<Vec<GameInstallDir>> {
    let mut stmt = conn.prepare(
        "SELECT gi.game_id, gi.installation_path
         FROM game_installations gi JOIN games g ON g.id = gi.game_id
         WHERE g.is_removed = 0 AND gi.installation_path IS NOT NULL AND gi.uninstalled_at IS NULL",
    )?;
    let dirs = stmt
        .query_map([], |row| {
            Ok(GameInstallDir {
                game_id: row.get(0)?,
                path: row.get(1)?,
            })
        })?
        .collect();
    dirs
}

/// Builds a detector for every executable and install folder in the database.
pub fn load_detector(conn: &Connection) -> Result<GameDetector> {
    Ok(GameDetector::new(get_all_executables(conn)?, get_install_dirs(conn)?))
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

use crate::models::RecentSession;
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

/// The most recent finished sessions, with their game's name and cover. Running
/// sessions are left out; they come from the tracker.
pub fn get_recent_sessions(conn: &Connection, limit: u32) -> Result<Vec<RecentSession>> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.game_id, g.name, g.cover_path, s.started_at, s.ended_at, COALESCE(s.duration_seconds, 0)
         FROM game_sessions s JOIN games g ON g.id = s.game_id
         WHERE s.ended_at IS NOT NULL
         ORDER BY s.started_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit], |row| {
        Ok(RecentSession {
            id: row.get(0)?,
            game_id: row.get(1)?,
            game_name: row.get(2)?,
            cover_path: row.get(3)?,
            started_at: row.get(4)?,
            ended_at: row.get(5)?,
            duration_seconds: row.get(6)?,
        })
    })?;
    rows.collect()
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
    let normalized_name = normalize_name(&candidate.name);

    // Prefer a game still in the library over one that was removed.
    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM games WHERE normalized_name = ?1 ORDER BY is_removed, id LIMIT 1",
            rusqlite::params![normalized_name],
            |r| r.get(0),
        )
        .optional()?;

    let game_id = if let Some(id) = existing_id {
        // Adding a game by hand brings it back if it had been removed.
        conn.execute(
            "UPDATE games SET is_removed = 0, current_install_status = 'Installed', updated_at = ?2 WHERE id = ?1",
            rusqlite::params![id, chrono::Utc::now()],
        )?;
        id
    } else {
        conn.execute(
            "INSERT INTO games (name, normalized_name, current_install_status, created_at, updated_at) VALUES (?1, ?2, 'Installed', ?3, ?4)",
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

    fn manual_game(name: &str) -> GameCandidate {
        GameCandidate {
            name: name.to_string(),
            install_path: format!("D:/Games/{}", name),
            executables: vec![],
            source: "Manual".to_string(),
            confidence: 100,
            platform_app_id: None,
        }
    }

    #[test]
    fn test_normalize_name() {
        assert_eq!(normalize_name("Cyberpunk 2077"), normalize_name("CYBERPUNK 2077™"));
        assert_eq!(normalize_name("Baldur's Gate 3"), "baldursgate3");
        assert_ne!(normalize_name("Hades"), normalize_name("Hades II"));
    }

    #[test]
    fn test_adding_a_removed_game_brings_it_back() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO games (id, name, normalized_name, is_removed) VALUES (5, 'Celeste', 'celeste', 1)",
            [],
        )
        .unwrap();

        let id = add_discovered_game(&conn, &manual_game("Celeste")).unwrap();

        assert_eq!(id, 5);
        let removed: bool = conn
            .query_row("SELECT is_removed FROM games WHERE id = 5", [], |r| r.get(0))
            .unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_merge_duplicate_games() {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO games (id, name, normalized_name, cover_path) VALUES (1, 'Cyberpunk 2077', 'cyberpunk2077', NULL);
             INSERT INTO games (id, name, normalized_name, cover_path) VALUES (2, 'Cyberpunk 2077', 'cyberpunk 2077', 'cover.png');
             INSERT INTO games (id, name, normalized_name, is_removed) VALUES (3, 'Cyberpunk 2077', 'cyberpunk 2077', 1);
             INSERT INTO game_platforms (game_id, platform, external_id) VALUES (2, 'GOG', '1423049311');
             INSERT INTO game_sessions (game_id, started_at, duration_seconds) VALUES (1, '2026-01-01 10:00:00+00:00', 60);
             INSERT INTO game_sessions (game_id, started_at, duration_seconds) VALUES (2, '2026-01-02 10:00:00+00:00', 120);",
        )
        .unwrap();

        assert_eq!(merge_duplicate_games(&mut conn).unwrap(), 1);

        let ids: Vec<i64> = conn
            .prepare("SELECT id FROM games ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(ids, [1, 3], "the removed copy stays separate");
        let playtime: i64 = conn
            .query_row("SELECT SUM(duration_seconds) FROM game_sessions WHERE game_id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(playtime, 180);
        let (platform_game, cover): (i64, Option<String>) = conn
            .query_row(
                "SELECT gp.game_id, g.cover_path FROM game_platforms gp JOIN games g ON g.id = gp.game_id",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(platform_game, 1);
        assert_eq!(cover.as_deref(), Some("cover.png"));
    }
}
