use super::epic::EpicProvider;
use super::gog::GogProvider;
use super::models::GameCandidate;
use super::provider::DiscoveryProvider;
use super::steam::SteamProvider;
use crate::database::normalize_name;
use rusqlite::{Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};

/// The result of scanning the installed launchers.
pub struct DiscoveryScan {
    pub candidates: Vec<GameCandidate>,
    /// For each provider that ran successfully, the app IDs it found. A provider that
    /// failed is absent, so its games aren't marked as uninstalled.
    found_app_ids: HashMap<String, HashSet<String>>,
}

pub struct DiscoveryManager {
    providers: Vec<Box<dyn DiscoveryProvider + Send + Sync>>,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            providers: vec![
                Box::new(SteamProvider::new()),
                Box::new(EpicProvider::new()),
                Box::new(GogProvider::new()),
            ],
        }
    }

    /// Names of the providers the user has enabled in settings.
    pub fn enabled_providers(&self, db: &Connection) -> HashSet<&'static str> {
        self.providers
            .iter()
            .map(|p| p.name())
            .filter(|name| {
                let setting_key = format!(
                    "discovery_provider_{}",
                    name.replace(" ", "_").to_lowercase()
                );
                crate::settings::get_setting(db, &setting_key)
                    .ok()
                    .flatten()
                    .map_or(true, |v| v == "true")
            })
            .collect()
    }

    /// Runs the enabled providers. This reads the registry and walks game folders, which
    /// can take a while, so it must not be called while holding the database lock.
    pub fn scan(&self, enabled: &HashSet<&'static str>) -> DiscoveryScan {
        let mut candidates = Vec::new();
        let mut found_app_ids = HashMap::new();

        for provider in &self.providers {
            if !enabled.contains(provider.name()) {
                continue;
            }

            // If one provider fails, we just ignore it and move on
            if let Ok(found) = provider.discover_games() {
                let app_ids: HashSet<String> = found
                    .iter()
                    .filter_map(|c| c.platform_app_id.clone())
                    .collect();
                found_app_ids.insert(provider.name().to_string(), app_ids);
                candidates.extend(found);
            }
        }

        DiscoveryScan {
            candidates,
            found_app_ids,
        }
    }

    /// Records a scan in the database in a single short transaction.
    pub fn apply(&self, db: &mut Connection, scan: &DiscoveryScan) -> Result<(), String> {
        apply_scan(db, scan).map_err(|e| e.to_string())
    }
}

fn apply_scan(db: &mut Connection, scan: &DiscoveryScan) -> rusqlite::Result<()> {
    let tx = db.transaction()?;

    // 1. Get all currently tracked games by platform
    // (Platform, External_ID) -> Game ID
    let mut existing_platform_apps: HashMap<(String, String), i64> = HashMap::new();
    {
        let mut stmt = tx.prepare("SELECT g.id, gp.platform, gp.external_id FROM games g JOIN game_platforms gp ON g.id = gp.game_id WHERE g.is_removed = 0")?;
        let iter = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        for item in iter.flatten() {
            existing_platform_apps.insert((item.1, item.2), item.0);
        }
    }

    // 2. Get all known installation paths to detect duplicates
    // Installation_Path -> Game ID
    let mut existing_install_paths: HashMap<String, i64> = HashMap::new();
    {
        let mut stmt = tx.prepare("SELECT g.id, gi.installation_path FROM games g JOIN game_installations gi ON g.id = gi.game_id WHERE g.is_removed = 0")?;
        let iter = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        for item in iter.flatten() {
            existing_install_paths.insert(item.1.to_lowercase(), item.0);
        }
    }

    for candidate in &scan.candidates {
        let Some(app_id) = &candidate.platform_app_id else {
            continue;
        };
        let platform_key = (candidate.source.clone(), app_id.clone());

        // Priority 1: Match by platform & external ID
        if let Some(game_id) = existing_platform_apps.get(&platform_key).copied() {
            tx.execute("UPDATE games SET current_install_status = 'Installed', updated_at = CURRENT_TIMESTAMP WHERE id = ?1", rusqlite::params![game_id])?;

            let count: i64 = tx.query_row("SELECT count(*) FROM game_installations WHERE game_id = ?1 AND installation_path = ?2", rusqlite::params![game_id, candidate.install_path], |r| r.get(0))?;
            if count == 0 {
                tx.execute("INSERT INTO game_installations (game_id, installation_path, detection_source) VALUES (?1, ?2, ?3)", rusqlite::params![game_id, candidate.install_path, candidate.source])?;
            }
        }
        // Priority 2: Match by installation path
        else if let Some(game_id) = existing_install_paths
            .get(&candidate.install_path.to_lowercase())
            .copied()
        {
            // Update as installed
            tx.execute("UPDATE games SET current_install_status = 'Installed', updated_at = CURRENT_TIMESTAMP WHERE id = ?1", rusqlite::params![game_id])?;

            // Add this platform mapping to the existing game
            let count: i64 = tx.query_row("SELECT count(*) FROM game_platforms WHERE game_id = ?1 AND platform = ?2 AND external_id = ?3", rusqlite::params![game_id, candidate.source, app_id], |r| r.get(0))?;
            if count == 0 {
                tx.execute("INSERT INTO game_platforms (game_id, platform, external_id) VALUES (?1, ?2, ?3)", rusqlite::params![game_id, candidate.source, app_id])?;
            }
        }
        // Priority 3: Create entirely new game (but first check if it was removed)
        else {
            let removed_check: i64 = tx.query_row("SELECT count(*) FROM games g JOIN game_platforms gp ON g.id = gp.game_id WHERE gp.platform = ?1 AND gp.external_id = ?2", rusqlite::params![candidate.source, app_id], |r| r.get(0))?;

            if removed_check == 0 {
                // The same game may already be in the library, added by hand or found on
                // another launcher. Attach this platform to it instead of adding it twice.
                let normalized_name = normalize_name(&candidate.name);
                let existing_id: Option<i64> = tx
                    .query_row(
                        "SELECT id FROM games WHERE normalized_name = ?1 AND is_removed = 0 ORDER BY id LIMIT 1",
                        rusqlite::params![normalized_name],
                        |r| r.get(0),
                    )
                    .optional()?;

                let game_id = match existing_id {
                    Some(id) => {
                        tx.execute("UPDATE games SET current_install_status = 'Installed', updated_at = CURRENT_TIMESTAMP WHERE id = ?1", rusqlite::params![id])?;
                        id
                    }
                    None => {
                        tx.execute("INSERT INTO games (name, normalized_name, current_install_status) VALUES (?1, ?2, 'Installed')", rusqlite::params![candidate.name, normalized_name])?;
                        tx.last_insert_rowid()
                    }
                };

                tx.execute("INSERT INTO game_platforms (game_id, platform, external_id) VALUES (?1, ?2, ?3)", rusqlite::params![game_id, candidate.source, app_id])?;
                tx.execute("INSERT INTO game_installations (game_id, installation_path, detection_source) SELECT ?1, ?2, ?3 WHERE NOT EXISTS (SELECT 1 FROM game_installations WHERE game_id = ?1 AND installation_path = ?2)", rusqlite::params![game_id, candidate.install_path, candidate.source])?;

                for exe in &candidate.executables {
                    let exe_path = std::path::Path::new(exe);
                    let exe_name = exe_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    tx.execute("INSERT INTO game_executables (game_id, executable_name, executable_path, detection_confidence) SELECT ?1, ?2, ?3, ?4 WHERE NOT EXISTS (SELECT 1 FROM game_executables WHERE game_id = ?1 AND executable_path = ?3)", rusqlite::params![game_id, exe_name, exe, candidate.confidence])?;
                }
            }
        }
    }

    // 3. Mark missing games as 'Not Installed' for each platform
    for ((platform, app_id), game_id) in existing_platform_apps {
        if let Some(discovered_for_platform) = scan.found_app_ids.get(&platform) {
            // If the provider successfully ran (it's in the map), but this app wasn't found:
            if !discovered_for_platform.contains(&app_id) {
                tx.execute("UPDATE games SET current_install_status = 'Not Installed', updated_at = CURRENT_TIMESTAMP WHERE id = ?1", rusqlite::params![game_id])?;
            }
        }
    }

    tx.commit()
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

    fn steam_game(app_id: &str) -> GameCandidate {
        GameCandidate {
            name: "Hades".to_string(),
            install_path: "D:/SteamLibrary/steamapps/common/Hades".to_string(),
            executables: vec!["D:/SteamLibrary/steamapps/common/Hades/Hades.exe".to_string()],
            source: "Steam".to_string(),
            confidence: 80,
            platform_app_id: Some(app_id.to_string()),
        }
    }

    fn install_status(db: &Connection) -> String {
        db.query_row("SELECT current_install_status FROM games", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn test_apply_adds_new_game() {
        let mut db = setup_test_db();
        let scan = DiscoveryScan {
            candidates: vec![steam_game("1145360")],
            found_app_ids: HashMap::from([("Steam".to_string(), HashSet::from(["1145360".to_string()]))]),
        };

        DiscoveryManager::new().apply(&mut db, &scan).unwrap();

        assert_eq!(install_status(&db), "Installed");
        let exes: i64 = db
            .query_row("SELECT COUNT(*) FROM game_executables", [], |r| r.get(0))
            .unwrap();
        assert_eq!(exes, 1);
    }

    #[test]
    fn test_failed_provider_does_not_mark_games_uninstalled() {
        let mut db = setup_test_db();
        let manager = DiscoveryManager::new();
        let found = DiscoveryScan {
            candidates: vec![steam_game("1145360")],
            found_app_ids: HashMap::from([("Steam".to_string(), HashSet::from(["1145360".to_string()]))]),
        };
        manager.apply(&mut db, &found).unwrap();

        // Steam couldn't be read this time (e.g. registry lookup failed).
        let failed = DiscoveryScan {
            candidates: vec![],
            found_app_ids: HashMap::new(),
        };
        manager.apply(&mut db, &failed).unwrap();
        assert_eq!(install_status(&db), "Installed");

        // Steam was read and the game is gone.
        let uninstalled = DiscoveryScan {
            candidates: vec![],
            found_app_ids: HashMap::from([("Steam".to_string(), HashSet::new())]),
        };
        manager.apply(&mut db, &uninstalled).unwrap();
        assert_eq!(install_status(&db), "Not Installed");
    }

    #[test]
    fn test_discovered_game_joins_game_added_by_hand() {
        let mut db = setup_test_db();
        db.execute(
            "INSERT INTO games (id, name, normalized_name) VALUES (7, 'HADES', 'hades')",
            [],
        )
        .unwrap();
        let scan = DiscoveryScan {
            candidates: vec![steam_game("1145360")],
            found_app_ids: HashMap::from([("Steam".to_string(), HashSet::from(["1145360".to_string()]))]),
        };

        DiscoveryManager::new().apply(&mut db, &scan).unwrap();

        let games: i64 = db.query_row("SELECT COUNT(*) FROM games", [], |r| r.get(0)).unwrap();
        assert_eq!(games, 1);
        let platform_game: i64 = db
            .query_row("SELECT game_id FROM game_platforms", [], |r| r.get(0))
            .unwrap();
        assert_eq!(platform_game, 7);
    }
}
