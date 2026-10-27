use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameCandidate {
    pub name: String,
    pub install_path: String,
    pub executables: Vec<String>, // Just filenames or relative paths
    pub source: String,           // "Steam", "Folder", "Manual"
    pub confidence: u32,
    pub platform_app_id: Option<String>,
}
