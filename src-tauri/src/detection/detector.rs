use crate::detection::filters::{is_non_game_dir, is_non_game_exe};
use crate::models::GameExecutable;
use std::collections::HashMap;

/// A folder a game is installed in. Any process started from inside it belongs to
/// that game, which covers launchers, Unreal Engine `*-Shipping.exe` binaries and
/// games that run several executables.
#[derive(Debug, Clone, PartialEq)]
pub struct GameInstallDir {
    pub game_id: i64,
    pub path: String,
}

pub struct GameDetector {
    /// Maps lowercased exact executable path to a game
    paths_cache: HashMap<String, i64>,
    /// Maps lowercased executable name (without .exe) to games with an executable of that name
    names_cache: HashMap<String, Vec<i64>>,
    /// Normalized install folders ending in '/', longest first, so the most specific
    /// folder wins when one install is inside another.
    install_dirs: Vec<(String, i64)>,
}

impl GameDetector {
    pub fn new(executables: Vec<GameExecutable>, installs: Vec<GameInstallDir>) -> Self {
        let mut paths_cache = HashMap::new();
        let mut names_cache: HashMap<String, Vec<i64>> = HashMap::new();

        for exec in executables {
            if let Some(ref path) = exec.executable_path {
                paths_cache.insert(Self::normalize_path(path), exec.game_id);
            }

            let name = exec.executable_name.to_lowercase();
            let name_no_ext = name.strip_suffix(".exe").unwrap_or(&name).to_string();
            names_cache.entry(name_no_ext).or_default().push(exec.game_id);
        }

        Self {
            paths_cache,
            names_cache,
            install_dirs: Self::usable_install_dirs(installs),
        }
    }

    /// Returns the id of the game a process belongs to, if any.
    pub fn identify(&self, name: &str, path: Option<&str>) -> Option<i64> {
        if let Some(p) = path {
            let normalized_p = Self::normalize_path(p);
            // Exact normalized path match (strongest identifier)
            if let Some(&game_id) = self.paths_cache.get(&normalized_p) {
                return Some(game_id);
            }
            // A known path that isn't a registered executable only counts if it's inside
            // a game's install folder. Matching it by file name would attribute any
            // unrelated program called e.g. "Launcher.exe" to a game.
            return self.game_for_folder(&normalized_p);
        }

        // The path couldn't be read (e.g. a protected process), so fall back to the name.
        let name_lower = name.to_lowercase();
        let name_no_ext = name_lower.strip_suffix(".exe").unwrap_or(&name_lower);
        self.names_cache
            .get(name_no_ext)
            .and_then(|games| games.first().copied())
    }

    fn game_for_folder(&self, normalized_exe_path: &str) -> Option<i64> {
        let (_, file_name) = normalized_exe_path.rsplit_once('/')?;
        if is_non_game_exe(file_name) {
            return None;
        }

        let (install_dir, game_id) = self
            .install_dirs
            .iter()
            .find(|(dir, _)| normalized_exe_path.starts_with(dir.as_str()))?;

        // Skip redistributable installers and engine tools inside the game folder.
        let inner = &normalized_exe_path[install_dir.len()..];
        let mut folders = inner.split('/').collect::<Vec<_>>();
        folders.pop(); // the file name
        if folders.into_iter().any(is_non_game_dir) {
            return None;
        }
        Some(*game_id)
    }

    /// Normalizes install folders and drops ones that would match far too much: drive
    /// roots, system folders, and folders that contain other games (a library folder
    /// that was added as if it were a game).
    fn usable_install_dirs(installs: Vec<GameInstallDir>) -> Vec<(String, i64)> {
        let dirs: Vec<(String, i64)> = installs
            .into_iter()
            .map(|i| (Self::normalize_dir(&i.path), i.game_id))
            .filter(|(dir, _)| !Self::is_too_broad(dir))
            .collect();

        let mut usable: Vec<(String, i64)> = dirs
            .iter()
            .filter(|(dir, game_id)| {
                !dirs.iter().any(|(other, other_id)| {
                    other_id != game_id && other.len() > dir.len() && other.starts_with(dir.as_str())
                })
            })
            .cloned()
            .collect();
        usable.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        usable
    }

    fn is_too_broad(normalized_dir: &str) -> bool {
        let parts: Vec<&str> = normalized_dir.split('/').filter(|p| !p.is_empty()).collect();
        match parts.as_slice() {
            [] | [_] => true,
            [_, top] => matches!(
                *top,
                "windows" | "program files" | "program files (x86)" | "programdata" | "users"
            ),
            [_, "users", _] => true,
            _ => false,
        }
    }

    fn normalize_path(path: &str) -> String {
        // Lowercase and replace backward slashes with forward slashes for unified comparison
        path.to_lowercase().replace('\\', "/")
    }

    fn normalize_dir(path: &str) -> String {
        format!("{}/", Self::normalize_path(path).trim_end_matches('/'))
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

    fn install(game_id: i64, path: &str) -> GameInstallDir {
        GameInstallDir {
            game_id,
            path: path.to_string(),
        }
    }

    fn detector_with_installs(installs: Vec<GameInstallDir>) -> GameDetector {
        GameDetector::new(sample_executables(), installs)
    }

    #[test]
    fn test_exact_path_matching() {
        let detector = detector_with_installs(vec![]);
        let result = detector.identify(
            "Cyberpunk2077.exe",
            Some("C:\\Games\\Cyberpunk 2077\\bin\\x64\\Cyberpunk2077.exe"),
        );
        assert_eq!(result, Some(101));
    }

    #[test]
    fn test_case_insensitive_matching() {
        let detector = detector_with_installs(vec![]);
        let result = detector.identify(
            "cyberpunk2077.EXE",
            Some("c:\\GAMES\\cyberpunk 2077\\BIN\\x64\\cyberpunk2077.exe"),
        );
        assert_eq!(result, Some(101));
    }

    #[test]
    fn test_path_normalization() {
        let detector = detector_with_installs(vec![]);
        // Forward slashes vs backward slashes
        let result = detector.identify(
            "Cyberpunk2077.exe",
            Some("C:/Games/Cyberpunk 2077/bin/x64/Cyberpunk2077.exe"),
        );
        assert_eq!(result, Some(101));
    }

    #[test]
    fn test_unknown_executables_ignored() {
        let detector = detector_with_installs(vec![]);
        assert_eq!(detector.identify("UnknownGame.exe", None), None);
    }

    #[test]
    fn test_multiple_executables_same_name_different_paths() {
        let detector = detector_with_installs(vec![]);
        // Should match GameA
        let result_a = detector.identify("launcher.exe", Some("C:\\Games\\GameA\\Launcher.exe"));
        assert_eq!(result_a, Some(103));

        // Should match GameB
        let result_b = detector.identify(
            "Launcher.exe",
            Some("D:\\SteamLibrary\\steamapps\\common\\GameB\\Launcher.exe"),
        );
        assert_eq!(result_b, Some(104));
    }

    #[test]
    fn test_name_fallback_only_without_path() {
        let detector = detector_with_installs(vec![]);
        // No path provided, should still match GTA5.exe
        assert_eq!(detector.identify("gta5.exe", None), Some(102));
        // A known path elsewhere on the PC must not match by name.
        assert_eq!(
            detector.identify("Launcher.exe", Some("C:\\Program Files\\SomeTool\\Launcher.exe")),
            None
        );
    }

    #[test]
    fn test_any_process_in_install_folder_matches() {
        let detector = detector_with_installs(vec![install(201, "D:\\SteamLibrary\\steamapps\\common\\Stray\\")]);
        // The Unreal Engine binary the launcher starts, which was never registered.
        let result = detector.identify(
            "Stray-Win64-Shipping.exe",
            Some("D:\\SteamLibrary\\steamapps\\common\\Stray\\Hk_project\\Binaries\\Win64\\Stray-Win64-Shipping.exe"),
        );
        assert_eq!(result, Some(201));
        // A different folder that merely starts with the same text doesn't match.
        assert_eq!(
            detector.identify("x.exe", Some("D:\\SteamLibrary\\steamapps\\common\\Stray 2\\x.exe")),
            None
        );
    }

    #[test]
    fn test_helpers_in_install_folder_are_ignored() {
        let detector = detector_with_installs(vec![install(201, "D:\\Games\\Stray")]);
        let crash = "D:\\Games\\Stray\\Engine\\Binaries\\Win64\\CrashReportClient.exe";
        assert_eq!(detector.identify("CrashReportClient.exe", Some(crash)), None);
        let redist = "D:\\Games\\Stray\\_CommonRedist\\vcredist\\2019\\VC_redist.x64.exe";
        assert_eq!(detector.identify("VC_redist.x64.exe", Some(redist)), None);
        let unity = "D:\\Games\\Stray\\UnityCrashHandler64.exe";
        assert_eq!(detector.identify("UnityCrashHandler64.exe", Some(unity)), None);
    }

    #[test]
    fn test_nested_and_overly_broad_install_folders() {
        let detector = detector_with_installs(vec![
            // A library folder added as if it were a game, containing a real game.
            install(300, "D:\\Games"),
            install(301, "D:\\Games\\Celeste"),
            install(302, "C:\\"),
            install(303, "C:\\Program Files"),
        ]);
        assert_eq!(
            detector.identify("Celeste.exe", Some("D:\\Games\\Celeste\\Celeste.exe")),
            Some(301)
        );
        // Other programs in the library folder, on the drive or in Program Files don't count.
        assert_eq!(detector.identify("tool.exe", Some("D:\\Games\\tool.exe")), None);
        assert_eq!(detector.identify("app.exe", Some("C:\\Tools\\app.exe")), None);
        assert_eq!(
            detector.identify("app.exe", Some("C:\\Program Files\\App\\app.exe")),
            None
        );
    }
}
