use super::models::GameCandidate;
use super::provider::DiscoveryProvider;
use super::reg_query;
use super::scanner::evaluate_game_folder;
use std::fs;
use std::path::{Path, PathBuf};

/// Where Steam is installed, if it is.
pub fn steam_install_path() -> Option<PathBuf> {
    SteamProvider::new().get_steam_path()
}

pub struct SteamProvider {
    override_path: Option<PathBuf>,
}

impl SteamProvider {
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

    fn get_steam_path(&self) -> Option<PathBuf> {
        if let Some(p) = &self.override_path {
            return Some(p.clone());
        }
        // 1. Try registry
        if let Ok(output) = reg_query(&["HKCU\\Software\\Valve\\Steam", "/v", "SteamPath"]) {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("SteamPath") && line.contains("REG_SZ") {
                        let parts: Vec<&str> = line.split("REG_SZ").collect();
                        if parts.len() == 2 {
                            return Some(PathBuf::from(parts[1].trim()));
                        }
                    }
                }
            }
        }

        // 2. Fallbacks
        let fallbacks = ["C:\\Program Files (x86)\\Steam", "C:\\Program Files\\Steam"];
        for f in &fallbacks {
            let p = PathBuf::from(f);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }

    fn parse_library_folders(steam_path: &Path) -> Vec<PathBuf> {
        let mut libraries = vec![steam_path.to_path_buf()];
        let vdf_path = steam_path.join("steamapps").join("libraryfolders.vdf");

        if let Ok(content) = fs::read_to_string(&vdf_path) {
            // Basic parsing for "path" "D:\\SteamLibrary"
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("\"path\"") {
                    let parts: Vec<&str> = line.split('\"').collect();
                    if parts.len() >= 4 {
                        let path = parts[3].replace("\\\\", "\\");
                        let p = PathBuf::from(path);
                        if !libraries.contains(&p) {
                            libraries.push(p);
                        }
                    }
                }
            }
        }

        libraries
    }

    fn parse_appmanifest(path: &Path) -> Option<(String, String, String)> {
        // Returns (AppID, Name, InstallDir)
        let content = fs::read_to_string(path).ok()?;

        let mut appid = String::new();
        let mut name = String::new();
        let mut installdir = String::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("\"appid\"") {
                let parts: Vec<&str> = line.split('\"').collect();
                if parts.len() >= 4 {
                    appid = parts[3].to_string();
                }
            } else if line.starts_with("\"name\"") {
                let parts: Vec<&str> = line.split('\"').collect();
                if parts.len() >= 4 {
                    name = parts[3].to_string();
                }
            } else if line.starts_with("\"installdir\"") {
                let parts: Vec<&str> = line.split('\"').collect();
                if parts.len() >= 4 {
                    installdir = parts[3].to_string();
                }
            }
        }

        if !appid.is_empty() && !name.is_empty() && !installdir.is_empty() {
            Some((appid, name, installdir))
        } else {
            None
        }
    }
}

impl DiscoveryProvider for SteamProvider {
    fn name(&self) -> &'static str {
        "Steam"
    }

    fn discover_games(&self) -> Result<Vec<GameCandidate>, String> {
        let steam_path = self
            .get_steam_path()
            .ok_or("Could not locate Steam installation")?;
        let libraries = Self::parse_library_folders(&steam_path);

        let mut candidates = Vec::new();

        for lib in libraries {
            let steamapps = lib.join("steamapps");
            if !steamapps.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&steamapps) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("acf") {
                        if let Some((app_id, name, install_dir)) = Self::parse_appmanifest(&path) {
                            // Don't register Steamworks Common Redistributables etc.
                            if name.contains("Steamworks Common")
                                || name.contains("Proton")
                                || name.contains("Steam Linux")
                            {
                                continue;
                            }

                            let game_folder = steamapps.join("common").join(&install_dir);
                            if !game_folder.exists() {
                                continue;
                            }

                            // Use existing scanner to find executables
                            if let Some(mut candidate) = evaluate_game_folder(&game_folder) {
                                candidate.name = name.clone();
                                candidate.source = "Steam".to_string();
                                candidate.platform_app_id = Some(app_id.clone());
                                candidates.push(candidate);
                            } else {
                                // We found the manifest and folder, but no obvious exes.
                                // Still add it with low confidence so user can pick!
                                candidates.push(GameCandidate {
                                    name: name.clone(),
                                    install_path: game_folder.to_string_lossy().to_string(),
                                    executables: vec![],
                                    source: "Steam".to_string(),
                                    confidence: 0,
                                    platform_app_id: Some(app_id.clone()),
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
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_steam_discovery() {
        let dir = tempdir().unwrap();
        let steam_dir = dir.path().join("Steam");

        let steamapps = steam_dir.join("steamapps");
        fs::create_dir_all(&steamapps).unwrap();

        let lib_file = steamapps.join("libraryfolders.vdf");
        fs::write(
            lib_file,
            "\"libraryfolders\" {\n \"0\" { \"path\" \"".to_string()
                + &steam_dir.to_string_lossy().replace("\\", "\\\\")
                + "\" } \n}",
        )
        .unwrap();

        let manifest = steamapps.join("appmanifest_123.acf");
        fs::write(manifest, "\"AppState\"\n{\n\t\"appid\"\t\t\"123\"\n\t\"name\"\t\t\"Test Game\"\n\t\"installdir\"\t\t\"TestGame\"\n}").unwrap();

        let game_dir = steamapps.join("common").join("TestGame");
        fs::create_dir_all(&game_dir).unwrap();
        fs::write(game_dir.join("TestGame.exe"), "fake exe content").unwrap();

        let provider = SteamProvider::with_path(steam_dir);
        let candidates = provider.discover_games().unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "Test Game");
        assert_eq!(candidates[0].platform_app_id.as_deref(), Some("123"));
        assert_eq!(candidates[0].executables.len(), 1);
    }
}
