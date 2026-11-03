use super::models::GameCandidate;
use super::provider::DiscoveryProvider;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub struct EpicProvider {
    override_path: Option<PathBuf>,
}

impl EpicProvider {
    pub fn new() -> Self {
        Self {
            override_path: None,
        }
    }

    #[cfg(test)]
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            override_path: Some(path),
        }
    }

    fn get_manifests_path(&self) -> PathBuf {
        if let Some(p) = &self.override_path {
            p.clone()
        } else {
            // Default Windows Epic Games manifest path
            PathBuf::from("C:\\ProgramData\\Epic\\EpicGamesLauncher\\Data\\Manifests")
        }
    }
}

impl DiscoveryProvider for EpicProvider {
    fn name(&self) -> &'static str {
        "Epic Games"
    }

    fn discover_games(&self) -> Result<Vec<GameCandidate>, String> {
        let manifests_dir = self.get_manifests_path();
        if !manifests_dir.exists() {
            // Not an error, just means Epic isn't installed or has no games
            return Ok(Vec::new());
        }

        let mut candidates = Vec::new();

        if let Ok(entries) = fs::read_dir(manifests_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("item") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(json) = serde_json::from_str::<Value>(&content) {
                            // Extract needed fields
                            let display_name = json.get("DisplayName").and_then(|v| v.as_str());
                            let install_location =
                                json.get("InstallLocation").and_then(|v| v.as_str());
                            let app_name = json.get("AppName").and_then(|v| v.as_str());
                            let launch_executable =
                                json.get("LaunchExecutable").and_then(|v| v.as_str());

                            if let (Some(name), Some(loc), Some(app_id), Some(exe)) =
                                (display_name, install_location, app_name, launch_executable)
                            {
                                // Epic games are sometimes engine components like "Unreal Engine"
                                if name.contains("Unreal Engine") {
                                    continue;
                                }

                                let exe_path = PathBuf::from(loc).join(exe);

                                // Convert to absolute string correctly
                                let mut executables = vec![];
                                if exe_path.exists() {
                                    executables.push(exe_path.to_string_lossy().to_string());
                                } else if !exe.is_empty() {
                                    // Sometimes LaunchExecutable is just a relative path, let's include it
                                    executables.push(exe_path.to_string_lossy().to_string());
                                }

                                candidates.push(GameCandidate {
                                    name: name.to_string(),
                                    install_path: loc.to_string(),
                                    executables,
                                    source: "Epic Games".to_string(),
                                    confidence: 100, // Epic explicitly gives us the launch executable
                                    platform_app_id: Some(app_id.to_string()),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_epic_discovery() {
        let dir = tempdir().unwrap();
        let item_path = dir.path().join("test_game.item");

        let json_content = r#"{
            "InstallLocation": "C:\\Games\\EpicGame",
            "Namespace": "epic",
            "ItemId": "epic_id_123",
            "AppName": "EpicTestAppID",
            "CatalogNamespace": "catalog",
            "CatalogItemId": "catalog_id",
            "DisplayName": "Epic Test Game",
            "LaunchExecutable": "Binaries\\Win64\\TestGame.exe"
        }"#;

        fs::write(&item_path, json_content).unwrap();

        let provider = EpicProvider::with_path(dir.path().to_path_buf());
        let candidates = provider.discover_games().unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "Epic Test Game");
        assert_eq!(
            candidates[0].platform_app_id.as_deref(),
            Some("EpicTestAppID")
        );
        assert_eq!(candidates[0].executables.len(), 1);
        assert_eq!(
            candidates[0].executables[0],
            "C:\\Games\\EpicGame\\Binaries\\Win64\\TestGame.exe"
        );
        assert_eq!(candidates[0].source, "Epic Games");
    }
}
