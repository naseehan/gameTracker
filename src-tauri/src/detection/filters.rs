//! Executables and folders inside a game's install folder that aren't the game itself.

/// Helper programs that ship with games: crash reporters, installers, uninstallers.
const NON_GAME_EXES: [&str; 16] = [
    "uninstall.exe",
    "unins000.exe",
    "unins001.exe",
    "crashreporter.exe",
    "crashreportclient.exe",
    "crashpad_handler.exe",
    "unitycrashhandler64.exe",
    "unitycrashhandler32.exe",
    "setup.exe",
    "dxsetup.exe",
    "vcredist_x64.exe",
    "vcredist_x86.exe",
    "vc_redist.x64.exe",
    "vc_redist.x86.exe",
    "easyanticheat_setup.exe",
    "easyanticheat_eos_setup.exe",
];

/// Folders holding redistributable installers, engine tools or save data.
/// Matched against whole folder names, never as a substring of the path.
const NON_GAME_DIRS: [&str; 9] = [
    "_commonredist",
    "redist",
    "redistributables",
    "directx",
    "vcredist",
    "dotnet",
    "engine",
    "saved",
    "crashreporter",
];

/// True for a helper executable name (case-insensitive) such as a crash reporter.
pub fn is_non_game_exe(file_name: &str) -> bool {
    NON_GAME_EXES.contains(&file_name.to_lowercase().as_str())
}

/// True for a folder name (case-insensitive) that never contains the game itself.
pub fn is_non_game_dir(dir_name: &str) -> bool {
    NON_GAME_DIRS.contains(&dir_name.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_whole_names_only() {
        assert!(is_non_game_dir("Engine"));
        assert!(!is_non_game_dir("Space Engineers"));
        assert!(is_non_game_exe("UnityCrashHandler64.exe"));
        assert!(!is_non_game_exe("Hades.exe"));
    }
}
