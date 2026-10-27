use super::epic::EpicProvider;
use super::gog::GogProvider;
use super::models::GameCandidate;
use super::provider::DiscoveryProvider;
use super::steam::SteamProvider;
use rusqlite::Connection;

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

    pub fn run_discovery(&self, db: &mut Connection) -> Result<Vec<GameCandidate>, String> {
        let mut all_candidates = Vec::new();

        // Map from source (e.g. "Steam") to AppIDs that we discovered in this run
        let mut discovered_app_ids_by_source: std::collections::HashMap<
            String,
            std::collections::HashSet<String>,
        > = std::collections::HashMap::new();

        for provider in &self.providers {
            let source_name = provider.name().to_string();

            // Check if this provider is enabled in settings
            let setting_key = format!(
                "discovery_provider_{}",
                source_name.replace(" ", "_").to_lowercase()
            );
            let enabled = crate::settings::get_setting(db, &setting_key)
                .unwrap_or(Some("true".to_string()))
                .unwrap_or("true".to_string())
                == "true";

            if !enabled {
                continue;
            }

            discovered_app_ids_by_source
                .insert(source_name.clone(), std::collections::HashSet::new());

            // If one provider fails, we just ignore it and move on
            if let Ok(candidates) = provider.discover_games() {
                for c in candidates {
                    if let Some(app_id) = &c.platform_app_id {
                        discovered_app_ids_by_source
                            .get_mut(&source_name)
                            .unwrap()
                            .insert(app_id.clone());
                    }
                    all_candidates.push(c);
                }
            }
        }

        let tx = db.transaction().map_err(|e| e.to_string())?;

        // 1. Get all currently tracked games by platform
        // (Platform, External_ID) -> Game ID
        let mut existing_platform_apps: std::collections::HashMap<(String, String), i64> =
            std::collections::HashMap::new();
        {
            let mut stmt = tx.prepare("SELECT g.id, gp.platform, gp.external_id FROM games g JOIN game_platforms gp ON g.id = gp.game_id WHERE g.is_removed = 0").unwrap();
            let iter = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .unwrap();
            for item in iter.flatten() {
                existing_platform_apps.insert((item.1, item.2), item.0);
            }
        }

        // 2. Get all known installation paths to detect duplicates
        // Installation_Path -> Game ID
        let mut existing_install_paths: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        {
            let mut stmt = tx.prepare("SELECT g.id, gi.installation_path FROM games g JOIN game_installations gi ON g.id = gi.game_id WHERE g.is_removed = 0").unwrap();
            let iter = stmt
                .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
                .unwrap();
            for item in iter.flatten() {
                existing_install_paths.insert(item.1.to_lowercase(), item.0);
            }
        }

        for candidate in &all_candidates {
            if let Some(app_id) = &candidate.platform_app_id {
                let platform_key = (candidate.source.clone(), app_id.clone());

                // Priority 1: Match by platform & external ID
                if let Some(game_id) = existing_platform_apps.get(&platform_key).copied() {
                    tx.execute("UPDATE games SET current_install_status = 'Installed', updated_at = CURRENT_TIMESTAMP WHERE id = ?1", rusqlite::params![game_id]).unwrap();

                    let count: i64 = tx.query_row("SELECT count(*) FROM game_installations WHERE game_id = ?1 AND installation_path = ?2", rusqlite::params![game_id, candidate.install_path], |r| r.get(0)).unwrap();
                    if count == 0 {
                        tx.execute("INSERT INTO game_installations (game_id, installation_path, detection_source) VALUES (?1, ?2, ?3)", rusqlite::params![game_id, candidate.install_path, candidate.source]).unwrap();
                    }
                }
                // Priority 2: Match by installation path
                else if let Some(game_id) = existing_install_paths
                    .get(&candidate.install_path.to_lowercase())
                    .copied()
                {
                    // Update as installed
                    tx.execute("UPDATE games SET current_install_status = 'Installed', updated_at = CURRENT_TIMESTAMP WHERE id = ?1", rusqlite::params![game_id]).unwrap();

                    // Add this platform mapping to the existing game
                    let count: i64 = tx.query_row("SELECT count(*) FROM game_platforms WHERE game_id = ?1 AND platform = ?2 AND external_id = ?3", rusqlite::params![game_id, candidate.source, app_id], |r| r.get(0)).unwrap();
                    if count == 0 {
                        tx.execute("INSERT INTO game_platforms (game_id, platform, external_id) VALUES (?1, ?2, ?3)", rusqlite::params![game_id, candidate.source, app_id]).unwrap();
                    }
                }
                // Priority 3: Create entirely new game (but first check if it was removed)
                else {
                    let removed_check: i64 = tx.query_row("SELECT count(*) FROM games g JOIN game_platforms gp ON g.id = gp.game_id WHERE gp.platform = ?1 AND gp.external_id = ?2", rusqlite::params![candidate.source, app_id], |r| r.get(0)).unwrap_or(0);

                    if removed_check == 0 {
                        tx.execute("INSERT INTO games (name, normalized_name, current_install_status) VALUES (?1, ?2, 'Installed')", rusqlite::params![candidate.name, candidate.name.to_lowercase()]).unwrap();
                        let game_id = tx.last_insert_rowid();

                        tx.execute("INSERT INTO game_platforms (game_id, platform, external_id) VALUES (?1, ?2, ?3)", rusqlite::params![game_id, candidate.source, app_id]).unwrap();
                        tx.execute("INSERT INTO game_installations (game_id, installation_path, detection_source) VALUES (?1, ?2, ?3)", rusqlite::params![game_id, candidate.install_path, candidate.source]).unwrap();

                        for exe in &candidate.executables {
                            let exe_path = std::path::Path::new(exe);
                            let exe_name = exe_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            tx.execute("INSERT INTO game_executables (game_id, executable_name, executable_path, detection_confidence) VALUES (?1, ?2, ?3, ?4)", rusqlite::params![game_id, exe_name, exe, candidate.confidence]).unwrap();
                        }
                    }
                }
            }
        }

        // 3. Mark missing games as 'Not Installed' for each platform
        for ((platform, app_id), game_id) in existing_platform_apps {
            if let Some(discovered_for_platform) = discovered_app_ids_by_source.get(&platform) {
                // If the provider successfully ran (it's in the map), but this app wasn't found:
                if !discovered_for_platform.contains(&app_id) {
                    tx.execute("UPDATE games SET current_install_status = 'Not Installed', updated_at = CURRENT_TIMESTAMP WHERE id = ?1", rusqlite::params![game_id]).unwrap();
                }
            }
        }

        tx.commit().map_err(|e| e.to_string())?;

        Ok(all_candidates)
    }
}
