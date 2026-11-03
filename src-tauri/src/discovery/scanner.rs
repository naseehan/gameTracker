use crate::detection::filters::{is_non_game_dir, is_non_game_exe};
use crate::discovery::models::GameCandidate;
use std::path::Path;
use walkdir::WalkDir;

/// How deep to look for executables inside a game folder. Unreal Engine games keep the
/// real binary at `<Game>/<Project>/Binaries/Win64/<Project>-Win64-Shipping.exe`.
const MAX_EXE_DEPTH: usize = 5;

/// Sub-folders games keep their own binaries in. A folder that has one is a single
/// game, not a library of games.
const GAME_BINARY_DIRS: [&str; 8] = [
    "bin", "bin64", "bin32", "binaries", "x64", "x86", "win64", "win32",
];

/// Folders that are never games when scanning a library folder.
const SYSTEM_DIRS: [&str; 5] = [
    "windows",
    "program files",
    "program files (x86)",
    "$recycle.bin",
    "system volume information",
];

/// Scans a folder the user picked. It can be a single game (`D:\Games\Hades`) or a
/// library of games (`D:\Games`), in which case every sub-folder is a candidate.
pub fn scan_folder(folder_path: &str) -> Vec<GameCandidate> {
    let root = Path::new(folder_path);
    if !root.is_dir() {
        return Vec::new();
    }

    if looks_like_single_game(root) {
        return evaluate_game_folder(root).into_iter().collect();
    }

    let mut candidates: Vec<GameCandidate> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|path| path.is_dir() && !is_hidden_or_system(path))
        .filter_map(|path| evaluate_game_folder(&path))
        .collect();
    candidates.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    if candidates.is_empty() {
        // No sub-folder holds a game, so the folder itself might be one.
        return evaluate_game_folder(root).into_iter().collect();
    }
    candidates
}

/// Treats one folder as a game and collects its executables.
pub fn evaluate_game_folder(dir: &Path) -> Option<GameCandidate> {
    let dir_name = dir.file_name()?.to_string_lossy().to_string();

    let mut executables: Vec<String> = WalkDir::new(dir)
        .max_depth(MAX_EXE_DEPTH)
        .into_iter()
        // Don't descend into redistributable, engine-tool or save folders.
        .filter_entry(|e| {
            e.depth() == 0
                || !(e.file_type().is_dir() && is_non_game_dir(&e.file_name().to_string_lossy()))
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && is_game_exe(e.path()))
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();

    // Launchers are often just updaters, so keep them only if there's nothing else.
    if executables.iter().any(|exe| !is_launcher(exe)) {
        executables.retain(|exe| !is_launcher(exe));
    }
    if executables.is_empty() {
        return None;
    }

    Some(GameCandidate {
        name: dir_name,
        install_path: dir.to_string_lossy().to_string(),
        executables,
        source: "Folder".to_string(),
        confidence: 80,
        platform_app_id: None,
    })
}

/// True if the folder holds a game directly: an executable at its top level, or a
/// binaries sub-folder such as `bin\x64` that contains one.
fn looks_like_single_game(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .any(|entry| {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                GAME_BINARY_DIRS.contains(&name.as_str())
                    && WalkDir::new(&path)
                        .max_depth(3)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .any(|e| e.file_type().is_file() && is_game_exe(e.path()))
            } else {
                is_game_exe(&path)
            }
        })
}

fn is_game_exe(path: &Path) -> bool {
    let is_exe = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"));
    is_exe
        && !path
            .file_name()
            .is_some_and(|name| is_non_game_exe(&name.to_string_lossy()))
}

fn is_launcher(exe_path: &str) -> bool {
    Path::new(exe_path)
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("launcher.exe"))
}

fn is_hidden_or_system(dir: &Path) -> bool {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    name.starts_with('.') || SYSTEM_DIRS.contains(&name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn touch(path: PathBuf) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "dummy").unwrap();
    }

    fn names(candidates: &[GameCandidate]) -> Vec<&str> {
        candidates.iter().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn test_scanner_filters_non_games() {
        let dir = tempdir().unwrap();
        let game_dir = dir.path().join("MyGame");
        touch(game_dir.join("MyGame.exe"));
        touch(game_dir.join("uninstall.exe"));
        touch(game_dir.join("_CommonRedist").join("vcredist_x64.exe"));

        let candidates = scan_folder(dir.path().to_str().unwrap());

        assert_eq!(names(&candidates), ["MyGame"]);
        // Should only contain MyGame.exe, not uninstall.exe or vcredist
        assert_eq!(candidates[0].executables.len(), 1);
        assert!(candidates[0].executables[0].ends_with("MyGame.exe"));
    }

    #[test]
    fn test_library_folder_is_not_a_game_itself() {
        let dir = tempdir().unwrap();
        let library = dir.path().join("Games");
        touch(library.join("Hollow Knight").join("hollow_knight.exe"));
        touch(library.join("Celeste").join("Celeste.exe"));

        let candidates = scan_folder(library.to_str().unwrap());

        assert_eq!(names(&candidates), ["Celeste", "Hollow Knight"]);
    }

    #[test]
    fn test_picking_a_game_folder_with_a_bin_folder() {
        let dir = tempdir().unwrap();
        let game = dir.path().join("The Witcher 3");
        touch(game.join("bin").join("x64").join("witcher3.exe"));
        fs::create_dir_all(game.join("content")).unwrap();

        let candidates = scan_folder(game.to_str().unwrap());

        assert_eq!(names(&candidates), ["The Witcher 3"]);
    }

    #[test]
    fn test_engine_in_a_game_name_is_not_filtered() {
        let dir = tempdir().unwrap();
        touch(dir.path().join("Space Engineers").join("Bin64").join("SpaceEngineers.exe"));

        let candidates = scan_folder(dir.path().to_str().unwrap());

        assert_eq!(names(&candidates), ["Space Engineers"]);
    }

    #[test]
    fn test_finds_unreal_shipping_binary() {
        let dir = tempdir().unwrap();
        let game = dir.path().join("Stray");
        touch(game.join("Stray.exe"));
        touch(game.join("Hk_project").join("Binaries").join("Win64").join("Stray-Win64-Shipping.exe"));
        touch(game.join("Engine").join("Binaries").join("Win64").join("CrashReportClient.exe"));

        let candidates = scan_folder(dir.path().to_str().unwrap());

        let exes = &candidates[0].executables;
        assert_eq!(exes.len(), 2, "{:?}", exes);
        assert!(exes.iter().any(|e| e.ends_with("Stray-Win64-Shipping.exe")));
    }

    #[test]
    fn test_uppercase_extension_and_launcher_only_games() {
        let dir = tempdir().unwrap();
        touch(dir.path().join("OldGame").join("GAME.EXE"));
        touch(dir.path().join("OldGame").join("Launcher.exe"));
        touch(dir.path().join("LauncherOnly").join("Launcher.exe"));

        let candidates = scan_folder(dir.path().to_str().unwrap());

        assert_eq!(names(&candidates), ["LauncherOnly", "OldGame"]);
        assert!(candidates[1].executables[0].ends_with("GAME.EXE"));
        assert_eq!(candidates[1].executables.len(), 1);
        assert!(candidates[0].executables[0].ends_with("Launcher.exe"));
    }
}
