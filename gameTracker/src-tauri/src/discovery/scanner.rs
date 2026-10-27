use crate::discovery::models::GameCandidate;
use std::path::Path;
use walkdir::WalkDir;

pub fn scan_folder(folder_path: &str) -> Vec<GameCandidate> {
    let mut candidates = Vec::new();
    let root = Path::new(folder_path);

    if !root.exists() || !root.is_dir() {
        return candidates;
    }

    // Heuristics for skipping known non-game subdirectories globally
    let ignored_dirs = [
        "_CommonRedist",
        "Redist",
        "DirectX",
        "vcredist",
        "DotNet",
        "Engine",
        "Binaries",
        "Saved",
        "CrashReporter",
    ];

    // Heuristics for skipping non-game executables
    let ignored_exes = [
        "uninstall.exe",
        "unins000.exe",
        "crashreporter.exe",
        "unitycrashhandler64.exe",
        "unitycrashhandler32.exe",
        "setup.exe",
        "dxsetup.exe",
        "vcredist_x64.exe",
        "vcredist_x86.exe",
        "launcher.exe", // We might want launcher, but often it's just an updater. We'll include it if it's the only one, but filter by default if there are others.
    ];

    // If the folder itself contains many .exe files, it might be a single game directory.
    // If the folder has subdirectories with .exe files, it might be a library directory (like D:\Games).
    // To handle both, we will just iterate immediate subdirectories.

    // Let's first check if the root itself is a game directory
    let root_candidate = evaluate_directory_as_game(root, &ignored_dirs, &ignored_exes);
    if let Some(candidate) = root_candidate {
        candidates.push(candidate);
    }

    // Now check immediate subdirectories
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                // If it's a directory, evaluate it
                if let Some(candidate) =
                    evaluate_directory_as_game(&path, &ignored_dirs, &ignored_exes)
                {
                    // Prevent duplicates if we accidentally scanned it twice
                    if !candidates
                        .iter()
                        .any(|c| c.install_path == candidate.install_path)
                    {
                        candidates.push(candidate);
                    }
                }
            }
        }
    }

    candidates
}

fn evaluate_directory_as_game(
    dir: &Path,
    ignored_dirs: &[&str],
    ignored_exes: &[&str],
) -> Option<GameCandidate> {
    let dir_name = dir.file_name()?.to_string_lossy().to_string();

    // Skip hidden or system-like directories
    if dir_name.starts_with('.')
        || dir_name == "Windows"
        || dir_name == "Program Files"
        || dir_name == "Program Files (x86)"
    {
        return None;
    }

    let mut executables = Vec::new();

    // Use WalkDir to find all .exe files up to depth 3
    for entry in WalkDir::new(dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip ignored directories
        if path.is_dir() {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if ignored_dirs.iter().any(|&d| name.eq_ignore_ascii_case(d)) {
                // We don't have a way to skip descending directly in this simple loop, but we can filter the files
                continue;
            }
        }

        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("exe") {
            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let file_name_lower = file_name.to_lowercase();

            // Filter obvious non-game executables
            if ignored_exes.contains(&file_name_lower.as_str()) {
                continue;
            }

            // Check if path contains any ignored directory
            let path_str = path.to_string_lossy().to_string();
            let contains_ignored_dir = ignored_dirs
                .iter()
                .any(|&d| path_str.to_lowercase().contains(&d.to_lowercase()));

            if !contains_ignored_dir {
                // Store the relative or absolute path?
                // Let's store the absolute path so we can register it accurately
                executables.push(path_str);
            }
        }
    }

    if executables.is_empty() {
        return None;
    }

    // Clean up launcher.exe vs main game exe logic:
    // If we have multiple exes, prefer the one matching the folder name.
    // For now, return all candidates and let user/system pick.

    Some(GameCandidate {
        name: dir_name,
        install_path: dir.to_string_lossy().to_string(),
        executables,
        source: "Folder".to_string(),
        confidence: 80,
        platform_app_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_scanner_filters_non_games() {
        let dir = tempdir().unwrap();
        let game_dir = dir.path().join("MyGame");
        fs::create_dir(&game_dir).unwrap();
        fs::write(game_dir.join("MyGame.exe"), "dummy").unwrap();
        fs::write(game_dir.join("uninstall.exe"), "dummy").unwrap();

        let redist_dir = game_dir.join("_CommonRedist");
        fs::create_dir(&redist_dir).unwrap();
        fs::write(redist_dir.join("vcredist_x64.exe"), "dummy").unwrap();

        let candidates = scan_folder(dir.path().to_str().unwrap());

        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.name, "MyGame");

        // Should only contain MyGame.exe, not uninstall.exe or vcredist
        assert_eq!(candidate.executables.len(), 1);
        assert!(candidate.executables[0].ends_with("MyGame.exe"));
    }
}
