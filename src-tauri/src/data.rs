use crate::models::{Game, GameExecutable, GameSession};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GamePlatform {
    pub id: i64,
    pub game_id: i64,
    pub platform: String,
    pub external_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GameInstallation {
    pub id: i64,
    pub game_id: i64,
    pub installed_at: Option<DateTime<Utc>>,
    pub uninstalled_at: Option<DateTime<Utc>>,
    pub installation_path: Option<String>,
    pub detection_source: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct ExportData {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub games: Vec<Game>,
    pub platforms: Vec<GamePlatform>,
    pub executables: Vec<GameExecutable>,
    pub installations: Vec<GameInstallation>,
    pub sessions: Vec<GameSession>,
    pub settings: Vec<SettingEntry>,
}

pub fn export_all_data(conn: &Connection) -> Result<ExportData> {
    let mut games = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT id, name, normalized_name, cover_path, created_at, updated_at, first_seen_at, last_seen_at, first_played_at, last_played_at, current_install_status, is_ignored, is_removed FROM games")?;
        let iter = stmt.query_map([], |row| {
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
        for item in iter {
            games.push(item?);
        }
    }

    let mut platforms = Vec::new();
    {
        let mut stmt =
            conn.prepare("SELECT id, game_id, platform, external_id FROM game_platforms")?;
        let iter = stmt.query_map([], |row| {
            Ok(GamePlatform {
                id: row.get(0)?,
                game_id: row.get(1)?,
                platform: row.get(2)?,
                external_id: row.get(3)?,
            })
        })?;
        for item in iter {
            platforms.push(item?);
        }
    }

    let mut executables = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT id, game_id, executable_name, executable_path, detection_confidence FROM game_executables")?;
        let iter = stmt.query_map([], |row| {
            Ok(GameExecutable {
                id: row.get(0)?,
                game_id: row.get(1)?,
                executable_name: row.get(2)?,
                executable_path: row.get(3)?,
                detection_confidence: row.get(4)?,
            })
        })?;
        for item in iter {
            executables.push(item?);
        }
    }

    let mut installations = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT id, game_id, installed_at, uninstalled_at, installation_path, detection_source FROM game_installations")?;
        let iter = stmt.query_map([], |row| {
            Ok(GameInstallation {
                id: row.get(0)?,
                game_id: row.get(1)?,
                installed_at: row.get(2)?,
                uninstalled_at: row.get(3)?,
                installation_path: row.get(4)?,
                detection_source: row.get(5)?,
            })
        })?;
        for item in iter {
            installations.push(item?);
        }
    }

    let mut sessions = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT id, game_id, started_at, ended_at, duration_seconds, detection_source FROM game_sessions")?;
        let iter = stmt.query_map([], |row| {
            Ok(GameSession {
                id: row.get(0)?,
                game_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                duration_seconds: row.get(4)?,
                detection_source: row.get(5)?,
            })
        })?;
        for item in iter {
            sessions.push(item?);
        }
    }

    let mut settings = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let iter = stmt.query_map([], |row| {
            Ok(SettingEntry {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })?;
        for item in iter {
            settings.push(item?);
        }
    }

    Ok(ExportData {
        version: 1,
        exported_at: Utc::now(),
        games,
        platforms,
        executables,
        installations,
        sessions,
        settings,
    })
}

pub fn import_all_data(conn: &mut Connection, data: ExportData) -> Result<(), String> {
    if data.version != 1 {
        return Err("Unsupported export version".to_string());
    }

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Clear existing data
    tx.execute("DELETE FROM game_sessions", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM game_installations", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM game_executables", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM game_platforms", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM games", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM settings", [])
        .map_err(|e| e.to_string())?;

    for g in data.games {
        tx.execute(
            "INSERT INTO games (id, name, normalized_name, cover_path, created_at, updated_at, first_seen_at, last_seen_at, first_played_at, last_played_at, current_install_status, is_ignored, is_removed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![g.id, g.name, g.normalized_name, g.cover_path, g.created_at, g.updated_at, g.first_seen_at, g.last_seen_at, g.first_played_at, g.last_played_at, g.current_install_status, g.is_ignored, g.is_removed]
        ).map_err(|e| e.to_string())?;
    }

    for p in data.platforms {
        tx.execute(
            "INSERT INTO game_platforms (id, game_id, platform, external_id) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![p.id, p.game_id, p.platform, p.external_id]
        ).map_err(|e| e.to_string())?;
    }

    for e in data.executables {
        tx.execute(
            "INSERT INTO game_executables (id, game_id, executable_name, executable_path, detection_confidence) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![e.id, e.game_id, e.executable_name, e.executable_path, e.detection_confidence]
        ).map_err(|e| e.to_string())?;
    }

    for i in data.installations {
        tx.execute(
            "INSERT INTO game_installations (id, game_id, installed_at, uninstalled_at, installation_path, detection_source) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![i.id, i.game_id, i.installed_at, i.uninstalled_at, i.installation_path, i.detection_source]
        ).map_err(|e| e.to_string())?;
    }

    for s in data.sessions {
        tx.execute(
            "INSERT INTO game_sessions (id, game_id, started_at, ended_at, duration_seconds, detection_source) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![s.id, s.game_id, s.started_at, s.ended_at, s.duration_seconds, s.detection_source]
        ).map_err(|e| e.to_string())?;
    }

    for s in data.settings {
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![s.key, s.value],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_all_history(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM game_sessions", [])?;
    Ok(())
}

pub fn reset_application(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM game_sessions", [])?;
    tx.execute("DELETE FROM game_installations", [])?;
    tx.execute("DELETE FROM game_executables", [])?;
    tx.execute("DELETE FROM game_platforms", [])?;
    tx.execute("DELETE FROM games", [])?;
    tx.execute("DELETE FROM settings", [])?;
    tx.commit()?;
    Ok(())
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
    fn test_export_import_round_trip() {
        let source = setup_test_db();
        source
            .execute_batch(
                "INSERT INTO games (id, name, normalized_name, is_removed) VALUES (1, 'Hades', 'hades', 0);
                 INSERT INTO games (id, name, normalized_name, is_removed) VALUES (2, 'Celeste', 'celeste', 1);
                 INSERT INTO game_platforms (game_id, platform, external_id) VALUES (1, 'Steam', '1145360');
                 INSERT INTO game_executables (game_id, executable_name, executable_path, detection_confidence)
                     VALUES (1, 'Hades.exe', 'D:/Games/Hades/Hades.exe', 80);
                 INSERT INTO game_installations (game_id, installation_path, detection_source)
                     VALUES (1, 'D:/Games/Hades', 'Steam');
                 INSERT INTO game_sessions (game_id, started_at, ended_at, duration_seconds)
                     VALUES (1, '2026-01-01 12:00:00+00:00', '2026-01-01 13:00:00+00:00', 3600);
                 INSERT INTO settings (key, value) VALUES ('onboarding_completed', 'true');",
            )
            .unwrap();

        let exported = export_all_data(&source).unwrap();
        let json = serde_json::to_string(&exported).unwrap();
        let parsed: ExportData = serde_json::from_str(&json).unwrap();

        let mut target = setup_test_db();
        import_all_data(&mut target, parsed).unwrap();

        let count = |table: &str| -> i64 {
            target
                .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(count("games"), 2);
        assert_eq!(count("game_platforms"), 1);
        assert_eq!(count("game_executables"), 1);
        assert_eq!(count("game_installations"), 1);
        assert_eq!(count("game_sessions"), 1);

        let removed: bool = target
            .query_row("SELECT is_removed FROM games WHERE id = 2", [], |r| r.get(0))
            .unwrap();
        assert!(removed, "removed games must stay removed after import");

        let duration: i64 = target
            .query_row("SELECT duration_seconds FROM game_sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(duration, 3600);
    }
}
