use super::models::GameCandidate;
use super::provider::DiscoveryProvider;
use super::reg_query;
use std::path::PathBuf;

pub struct GogProvider {
    mock_reg_output: Option<String>,
}

impl GogProvider {
    pub fn new() -> Self {
        Self {
            mock_reg_output: None,
        }
    }

    #[cfg(test)]
    pub fn with_mock_output(output: String) -> Self {
        Self {
            mock_reg_output: Some(output),
        }
    }

    fn get_registry_output(&self) -> Result<String, String> {
        if let Some(mock) = &self.mock_reg_output {
            return Ok(mock.clone());
        }

        let output = reg_query(&["HKLM\\SOFTWARE\\WOW6432Node\\GOG.com\\Games", "/s"]);

        match output {
            Ok(output) if output.status.success() => {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
            _ => {
                // Also try the non-WOW6432Node path just in case
                let output = reg_query(&["HKLM\\SOFTWARE\\GOG.com\\Games", "/s"]);

                match output {
                    Ok(output) if output.status.success() => {
                        Ok(String::from_utf8_lossy(&output.stdout).to_string())
                    }
                    _ => Err("Could not query GOG registry or GOG is not installed".to_string()),
                }
            }
        }
    }
}

impl DiscoveryProvider for GogProvider {
    fn name(&self) -> &'static str {
        "GOG"
    }

    fn discover_games(&self) -> Result<Vec<GameCandidate>, String> {
        let stdout = match self.get_registry_output() {
            Ok(out) => out,
            Err(_) => return Ok(Vec::new()), // Not an error if GOG simply isn't there
        };

        let mut candidates = Vec::new();

        let mut current_id = String::new();
        let mut current_name = String::new();
        let mut current_path = String::new();
        let mut current_exe = String::new();

        // Helper to push a candidate and reset
        let push_candidate = |id: &mut String,
                                  name: &mut String,
                                  path: &mut String,
                                  exe: &mut String,
                                  cands: &mut Vec<GameCandidate>| {
            if !id.is_empty() && !name.is_empty() && !path.is_empty() && !exe.is_empty() {
                let mut executables = vec![];

                // The exe in registry might be an absolute path or just a filename
                let exe_path = PathBuf::from(&*exe);
                if exe_path.is_absolute() {
                    executables.push(exe_path.to_string_lossy().to_string());
                } else {
                    executables.push(
                        PathBuf::from(&*path)
                            .join(&*exe)
                            .to_string_lossy()
                            .to_string(),
                    );
                }

                cands.push(GameCandidate {
                    name: name.clone(),
                    install_path: path.clone(),
                    executables,
                    source: "GOG".to_string(),
                    confidence: 100, // Explicit from GOG registry
                    platform_app_id: Some(id.clone()),
                });
            }
            id.clear();
            name.clear();
            path.clear();
            exe.clear();
        };

        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("HKEY_LOCAL_MACHINE") {
                // New game block
                push_candidate(
                    &mut current_id,
                    &mut current_name,
                    &mut current_path,
                    &mut current_exe,
                    &mut candidates,
                );

                if let Some(id) = line.split('\\').last() {
                    current_id = id.to_string();
                }
            } else if let Some((key, value)) = line.split_once("REG_SZ") {
                // Value lines look like `name    REG_SZ    data`. Compare whole names:
                // `exeFile` (just the file name) must not overwrite `exe` (the full path).
                let value = value.trim().to_string();
                match key.trim() {
                    "gameName" => current_name = value,
                    "path" => current_path = value,
                    "exe" => current_exe = value,
                    _ => {}
                }
            }
        }

        // Push the last one
        push_candidate(
            &mut current_id,
            &mut current_name,
            &mut current_path,
            &mut current_exe,
            &mut candidates,
        );

        Ok(candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gog_discovery() {
        let mock_output = r#"
HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\GOG.com\Games\1207658924
    gameName    REG_SZ    Cyberpunk 2077
    path    REG_SZ    C:\GOG Games\Cyberpunk 2077
    exe    REG_SZ    C:\GOG Games\Cyberpunk 2077\bin\x64\Cyberpunk2077.exe
    gameID    REG_SZ    1207658924

HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\GOG.com\Games\12345
    gameName    REG_SZ    The Witcher 3
    path    REG_SZ    D:\Games\Witcher 3
    exe    REG_SZ    bin\x64\witcher3.exe
        "#;

        let provider = GogProvider::with_mock_output(mock_output.to_string());
        let candidates = provider.discover_games().unwrap();

        assert_eq!(candidates.len(), 2);

        assert_eq!(candidates[0].name, "Cyberpunk 2077");
        assert_eq!(candidates[0].platform_app_id.as_deref(), Some("1207658924"));
        assert_eq!(
            candidates[0].executables[0],
            "C:\\GOG Games\\Cyberpunk 2077\\bin\\x64\\Cyberpunk2077.exe"
        );

        assert_eq!(candidates[1].name, "The Witcher 3");
        assert_eq!(candidates[1].platform_app_id.as_deref(), Some("12345"));
        assert_eq!(
            candidates[1].executables[0],
            "D:\\Games\\Witcher 3\\bin\\x64\\witcher3.exe"
        );
    }

    #[test]
    fn test_gog_exe_file_does_not_override_exe() {
        let mock_output = r#"
HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\GOG.com\Games\1207664643
    exe    REG_SZ    D:\GOG Games\The Witcher 3\bin\x64\witcher3.exe
    exeFile    REG_SZ    witcher3.exe
    gameName    REG_SZ    The Witcher 3
    path    REG_SZ    D:\GOG Games\The Witcher 3
    pathname    REG_SZ    unused
"#;

        let provider = GogProvider::with_mock_output(mock_output.to_string());
        let candidates = provider.discover_games().unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].executables[0],
            r"D:\GOG Games\The Witcher 3\bin\x64\witcher3.exe"
        );
        assert_eq!(candidates[0].install_path, r"D:\GOG Games\The Witcher 3");
    }
}
