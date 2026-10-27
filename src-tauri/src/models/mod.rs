use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub normalized_name: String,
    pub cover_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub first_seen_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub first_played_at: Option<DateTime<Utc>>,
    pub last_played_at: Option<DateTime<Utc>>,
    pub current_install_status: String,
    pub is_ignored: bool,
    // Older export files don't have this field.
    #[serde(default)]
    pub is_removed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameSession {
    pub id: i64,
    pub game_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
    pub detection_source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GameExecutable {
    pub id: i64,
    pub game_id: i64,
    pub executable_name: String,
    pub executable_path: Option<String>,
    pub detection_confidence: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GameLibraryItem {
    pub id: i64,
    pub name: String,
    pub cover_path: Option<String>,
    pub total_playtime_seconds: i64,
    pub last_played_at: Option<chrono::DateTime<chrono::Utc>>,
    pub currently_playing: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_ignored: bool,
    pub is_removed: bool,
    pub current_install_status: String,
    pub platforms: Vec<String>,
    pub installation_paths: Vec<String>,
}
