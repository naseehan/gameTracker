use crate::models::GameExecutable;
use std::collections::HashMap;

pub struct GameDetector {
    /// Maps lowercased exact executable path to a known game executable
    paths_cache: HashMap<String, GameExecutable>,
    /// Maps lowercased executable name (without .exe) to a list of known game executables
    names_cache: HashMap<String, Vec<GameExecutable>>,
}

impl GameDetector {
    pub fn new(executables: Vec<GameExecutable>) -> Self {
        let mut paths_cache = HashMap::new();
        let mut names_cache: HashMap<String, Vec<GameExecutable>> = HashMap::new();

        for exec in executables {
            if let Some(ref path) = exec.executable_path {
                paths_cache.insert(Self::normalize_path(path), exec.clone());
            }

            let name = exec.executable_name.to_lowercase();
            let name_no_ext = name.strip_suffix(".exe").unwrap_or(&name).to_string();
            names_cache.entry(name_no_ext).or_default().push(exec);
        }

        Self {
            paths_cache,
            names_cache,
        }
    }

    pub fn refresh_cache(&mut self, executables: Vec<GameExecutable>) {
        *self = Self::new(executables);
    }

    pub fn identify(&self, name: &str, path: Option<&str>) -> Option<GameExecutable> {
        // First pass: Exact normalized path match (strongest identifier)
        if let Some(p) = path {
            let normalized_p = Self::normalize_path(p);
            if let Some(exec) = self.paths_cache.get(&normalized_p) {
                return Some(exec.clone());
            }
        }

        // Second pass: Fallback to name match without extension
        let name_lower = name.to_lowercase();
        let name_no_ext = name_lower.strip_suffix(".exe").unwrap_or(&name_lower);

        if let Some(candidates) = self.names_cache.get(name_no_ext) {
            if let Some(first) = candidates.first() {
                return Some(first.clone());
            }
        }

        None
    }

    fn normalize_path(path: &str) -> String {
        // Lowercase and replace backward slashes with forward slashes for unified comparison
        path.to_lowercase().replace('\\', "/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_executables() -> Vec<GameExecutable> {
        vec![
            GameExecutable {
                id: 1,
                game_id: 101,
                executable_name: "Cyberpunk2077.exe".to_string(),
                executable_path: Some(
                    "C:\\Games\\Cyberpunk 2077\\bin\\x64\\Cyberpunk2077.exe".to_string(),
                ),
                detection_confidence: 100,
            },
            GameExecutable {
                id: 2,
                game_id: 102,
                executable_name: "GTA5.exe".to_string(),
                executable_path: None,
                detection_confidence: 100,
            },
            GameExecutable {
                id: 3,
                game_id: 103,
                executable_name: "Launcher.exe".to_string(),
                executable_path: Some("C:\\Games\\GameA\\Launcher.exe".to_string()),
                detection_confidence: 100,
            },
            GameExecutable {
                id: 4,
                game_id: 104,
                executable_name: "launcher.exe".to_string(),
                executable_path: Some(
                    "D:\\SteamLibrary\\steamapps\\common\\GameB\\Launcher.exe".to_string(),
                ),
                detection_confidence: 100,
            },
        ]
    }

    #[test]
    fn test_exact_path_matching() {
        let detector = GameDetector::new(sample_executables());
        let result = detector.identify(
            "Cyberpunk2077.exe",
            Some("C:\\Games\\Cyberpunk 2077\\bin\\x64\\Cyberpunk2077.exe"),
        );
        assert_eq!(result.unwrap().game_id, 101);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let detector = GameDetector::new(sample_executables());
        let result = detector.identify(
            "cyberpunk2077.EXE",
            Some("c:\\GAMES\\cyberpunk 2077\\BIN\\x64\\cyberpunk2077.exe"),
        );
        assert_eq!(result.unwrap().game_id, 101);
    }

    #[test]
    fn test_path_normalization() {
        let detector = GameDetector::new(sample_executables());
        // Forward slashes vs backward slashes
        let result = detector.identify(
            "Cyberpunk2077.exe",
            Some("C:/Games/Cyberpunk 2077/bin/x64/Cyberpunk2077.exe"),
        );
        assert_eq!(result.unwrap().game_id, 101);
    }

    #[test]
    fn test_unknown_executables_ignored() {
        let detector = GameDetector::new(sample_executables());
        let result = detector.identify("UnknownGame.exe", None);
        assert!(result.is_none());
    }

    #[test]
    fn test_multiple_executables_same_name_different_paths() {
        let detector = GameDetector::new(sample_executables());
        // Should match GameA
        let result_a = detector.identify("launcher.exe", Some("C:\\Games\\GameA\\Launcher.exe"));
        assert_eq!(result_a.unwrap().game_id, 103);

        // Should match GameB
        let result_b = detector.identify(
            "Launcher.exe",
            Some("D:\\SteamLibrary\\steamapps\\common\\GameB\\Launcher.exe"),
        );
        assert_eq!(result_b.unwrap().game_id, 104);
    }

    #[test]
    fn test_name_fallback_matching() {
        let detector = GameDetector::new(sample_executables());
        // No path provided, should still match GTA5.exe
        let result = detector.identify("gta5.exe", None);
        assert_eq!(result.unwrap().game_id, 102);
    }
}
