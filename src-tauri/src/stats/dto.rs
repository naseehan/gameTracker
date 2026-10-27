use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_playtime_seconds: i64,
    pub games_played: i64,
    pub total_sessions: i64,
    pub today_playtime_seconds: i64,
    pub week_playtime_seconds: i64,
    pub most_played_game: Option<String>,
    pub most_played_game_playtime: i64,

    pub playtime_over_time: Vec<DailyPlaytime>,
    pub top_games: Vec<TopGame>,
    pub activity_by_day_of_week: Vec<DayOfWeekActivity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyPlaytime {
    pub date_label: String,
    pub playtime_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopGame {
    pub game_id: i64,
    pub name: String,
    pub playtime_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayOfWeekActivity {
    pub day: String,
    pub playtime_seconds: i64,
}
