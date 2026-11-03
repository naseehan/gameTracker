//! Corrections the user makes to their library: renaming games, fixing or deleting
//! sessions, choosing which executables and folders identify a game, and restoring
//! removed games. Errors are messages meant for the UI.

use super::normalize_name;
use crate::models::{DetectionEntry, GameDetection, RemovedGame};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

fn sql(e: rusqlite::Error) -> String {
    e.to_string()
}

pub fn rename_game(conn: &Connection, game_id: i64, name: &str) -> Result<(), String> {
    let name = name.trim();
    let normalized = normalize_name(name);
    if normalized.is_empty() {
        return Err("Enter a name with at least one letter or number.".to_string());
    }
    let clash: Option<String> = conn
        .query_row(
            "SELECT name FROM games WHERE normalized_name = ?1 AND id != ?2 AND is_removed = 0",
            rusqlite::params![normalized, game_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(sql)?;
    if let Some(other) = clash {
        return Err(format!("\"{}\" is already in your library.", other));
    }
    conn.execute(
        "UPDATE games SET name = ?1, normalized_name = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
        rusqlite::params![name, normalized, game_id],
    )
    .map_err(sql)?;
    Ok(())
}

/// Changes a finished session's start and end time.
pub fn update_session(
    conn: &Connection,
    session_id: i64,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
) -> Result<(), String> {
    if ended_at <= started_at {
        return Err("The session must end after it starts.".to_string());
    }
    if ended_at > Utc::now() {
        return Err("A session can't end in the future.".to_string());
    }
    let changed = conn
        .execute(
            "UPDATE game_sessions SET started_at = ?1, ended_at = ?2, last_heartbeat_at = ?2, duration_seconds = ?3
             WHERE id = ?4 AND ended_at IS NOT NULL",
            rusqlite::params![started_at, ended_at, (ended_at - started_at).num_seconds(), session_id],
        )
        .map_err(sql)?;
    if changed == 0 {
        return Err("That session is still running or no longer exists.".to_string());
    }
    Ok(())
}

pub fn delete_session(conn: &Connection, session_id: i64) -> Result<(), String> {
    let changed = conn
        .execute(
            "DELETE FROM game_sessions WHERE id = ?1 AND ended_at IS NOT NULL",
            [session_id],
        )
        .map_err(sql)?;
    if changed == 0 {
        return Err("That session is still running or no longer exists.".to_string());
    }
    Ok(())
}

pub fn get_game_detection(conn: &Connection, game_id: i64) -> Result<GameDetection, String> {
    let entries = |query: &str| -> Result<Vec<DetectionEntry>, String> {
        let mut stmt = conn.prepare(query).map_err(sql)?;
        let rows = stmt
            .query_map([game_id], |r| Ok(DetectionEntry { id: r.get(0)?, path: r.get(1)? }))
            .map_err(sql)?;
        rows.collect::<rusqlite::Result<_>>().map_err(sql)
    };
    Ok(GameDetection {
        executables: entries(
            "SELECT id, COALESCE(executable_path, executable_name) FROM game_executables WHERE game_id = ?1 ORDER BY id",
        )?,
        install_folders: entries(
            "SELECT id, installation_path FROM game_installations
             WHERE game_id = ?1 AND installation_path IS NOT NULL ORDER BY id",
        )?,
    })
}

pub fn add_executable(conn: &Connection, game_id: i64, path: &Path) -> Result<(), String> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or("That isn't a file.")?;
    let path = path.to_string_lossy();
    conn.execute(
        "INSERT INTO game_executables (game_id, executable_name, executable_path, detection_confidence)
         SELECT ?1, ?2, ?3, 100
         WHERE NOT EXISTS (SELECT 1 FROM game_executables WHERE game_id = ?1 AND executable_path = ?3)",
        rusqlite::params![game_id, name, path],
    )
    .map_err(sql)?;
    Ok(())
}

pub fn remove_executable(conn: &Connection, executable_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM game_executables WHERE id = ?1", [executable_id])
        .map_err(sql)?;
    Ok(())
}

pub fn add_install_folder(conn: &Connection, game_id: i64, path: &Path) -> Result<(), String> {
    if path.parent().is_none() {
        return Err("Choose the game's own folder, not a whole drive.".to_string());
    }
    let path = path.to_string_lossy();
    conn.execute(
        "INSERT INTO game_installations (game_id, installation_path, detection_source)
         SELECT ?1, ?2, 'Manual'
         WHERE NOT EXISTS (SELECT 1 FROM game_installations WHERE game_id = ?1 AND installation_path = ?2)",
        rusqlite::params![game_id, path],
    )
    .map_err(sql)?;
    Ok(())
}

pub fn remove_install_folder(conn: &Connection, install_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM game_installations WHERE id = ?1", [install_id])
        .map_err(sql)?;
    Ok(())
}

pub fn get_removed_games(conn: &Connection) -> Result<Vec<RemovedGame>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT g.id, g.name, COALESCE(SUM(s.duration_seconds), 0)
             FROM games g LEFT JOIN game_sessions s ON s.game_id = g.id
             WHERE g.is_removed = 1
             GROUP BY g.id ORDER BY g.name COLLATE NOCASE",
        )
        .map_err(sql)?;
    let rows = stmt
        .query_map([], |r| {
            Ok(RemovedGame {
                id: r.get(0)?,
                name: r.get(1)?,
                total_playtime_seconds: r.get(2)?,
            })
        })
        .map_err(sql)?;
    rows.collect::<rusqlite::Result<_>>().map_err(sql)
}

/// Puts a removed game back in the library. Its play history was kept when it was removed.
pub fn restore_game(conn: &Connection, game_id: i64) -> Result<(), String> {
    let name: String = conn
        .query_row("SELECT normalized_name FROM games WHERE id = ?1", [game_id], |r| r.get(0))
        .map_err(sql)?;
    let clash: Option<String> = conn
        .query_row(
            "SELECT name FROM games WHERE normalized_name = ?1 AND id != ?2 AND is_removed = 0",
            rusqlite::params![name, game_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(sql)?;
    if let Some(other) = clash {
        return Err(format!("\"{}\" is already in your library.", other));
    }
    conn.execute(
        "UPDATE games SET is_removed = 0, current_install_status = 'Unknown', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
        [game_id],
    )
    .map_err(sql)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::run_migrations;
    use chrono::Duration;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO games (id, name, normalized_name) VALUES (1, 'Hades', 'hades');
             INSERT INTO games (id, name, normalized_name, is_removed) VALUES (2, 'Celeste', 'celeste', 1);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_rename_rejects_names_already_in_library() {
        let db = setup_test_db();
        db.execute("INSERT INTO games (id, name, normalized_name) VALUES (3, 'Hollow Knight', 'hollowknight')", [])
            .unwrap();
        assert!(rename_game(&db, 3, "HADES").is_err());
        assert!(rename_game(&db, 3, "  ").is_err());
        // A removed game's name is free to use.
        rename_game(&db, 3, "Celeste").unwrap();
    }

    #[test]
    fn test_update_and_delete_session() {
        let db = setup_test_db();
        let now = Utc::now();
        db.execute(
            "INSERT INTO game_sessions (id, game_id, started_at, ended_at, duration_seconds) VALUES (10, 1, ?1, ?2, 39600)",
            rusqlite::params![now - Duration::hours(12), now - Duration::hours(1)],
        )
        .unwrap();

        assert!(update_session(&db, 10, now - Duration::hours(2), now - Duration::hours(3)).is_err());
        assert!(update_session(&db, 10, now - Duration::hours(2), now + Duration::hours(1)).is_err());
        update_session(&db, 10, now - Duration::hours(3), now - Duration::hours(1)).unwrap();
        let duration: i64 = db
            .query_row("SELECT duration_seconds FROM game_sessions WHERE id = 10", [], |r| r.get(0))
            .unwrap();
        assert_eq!(duration, 7200);

        delete_session(&db, 10).unwrap();
        assert!(delete_session(&db, 10).is_err());
    }

    #[test]
    fn test_running_session_cannot_be_edited() {
        let db = setup_test_db();
        let id = crate::database::create_session(&db, 1, Utc::now()).unwrap();
        assert!(delete_session(&db, id).is_err());
    }

    #[test]
    fn test_detection_entries() {
        let db = setup_test_db();
        add_executable(&db, 1, Path::new("D:/Games/Hades/Hades.exe")).unwrap();
        add_executable(&db, 1, Path::new("D:/Games/Hades/Hades.exe")).unwrap();
        add_install_folder(&db, 1, Path::new("D:/Games/Hades")).unwrap();
        assert!(add_install_folder(&db, 1, Path::new("D:/")).is_err());

        let detection = get_game_detection(&db, 1).unwrap();
        assert_eq!(detection.executables.len(), 1);
        assert_eq!(detection.install_folders.len(), 1);

        remove_executable(&db, detection.executables[0].id).unwrap();
        remove_install_folder(&db, detection.install_folders[0].id).unwrap();
        let detection = get_game_detection(&db, 1).unwrap();
        assert!(detection.executables.is_empty() && detection.install_folders.is_empty());
    }

    #[test]
    fn test_restore_removed_game() {
        let db = setup_test_db();
        assert_eq!(get_removed_games(&db).unwrap().len(), 1);
        restore_game(&db, 2).unwrap();
        assert!(get_removed_games(&db).unwrap().is_empty());
    }
}
