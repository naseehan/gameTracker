use crate::models::Game;
use crate::stats::dto::DailyPlaytime;
use crate::stats::game_queries::{
    get_game_playtime_history, get_game_sessions, get_game_statistics, GameSessionDto,
    GameStatistics,
};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};

use std::sync::atomic::{AtomicBool, AtomicU64};

pub struct AppState {
    pub db: Arc<Mutex<Option<Connection>>>,
    pub active_processes: Arc<Mutex<HashMap<u32, String>>>,
    pub active_games: Arc<Mutex<HashMap<u32, Game>>>,
    pub session_tracker: Arc<Mutex<crate::tracking::tracker::SessionTracker>>,
    pub detector: Arc<Mutex<crate::detection::detector::GameDetector>>,
    pub tracking_enabled: Arc<AtomicBool>,
    pub polling_interval_secs: Arc<AtomicU64>,
    pub auto_discovery_enabled: Arc<AtomicBool>,
    pub discovery_interval_secs: Arc<AtomicU64>,
}

#[tauri::command]
pub fn get_games(state: State<'_, AppState>) -> Result<Vec<Game>, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock database".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let mut stmt = db
        .prepare("SELECT id, name, normalized_name, cover_path, created_at, updated_at, first_seen_at, last_seen_at, first_played_at, last_played_at, current_install_status, is_ignored, is_removed FROM games WHERE is_removed = 0 ORDER BY last_played_at DESC")
        .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;

    let game_iter = stmt
        .query_map([], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                normalized_name: row.get(2)?,
                cover_path: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                first_seen_at: row.get(6)?,
                last_seen_at: row.get(7)?,
                first_played_at: row.get(8)?,
                last_played_at: row.get(9)?,
                current_install_status: row.get(10)?,
                is_ignored: row.get(11)?,
                is_removed: row.get(12)?,
            })
        })
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("FOREIGN KEY") {
                "Action failed due to a constraint. Your data is safe.".to_string()
            } else if msg.contains("locked") {
                "Database is currently busy, please try again.".to_string()
            } else {
                msg
            }
        })?;

    let mut games = Vec::new();
    for game in game_iter {
        games.push(game.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("FOREIGN KEY") {
                "Action failed due to a constraint. Your data is safe.".to_string()
            } else if msg.contains("locked") {
                "Database is currently busy, please try again.".to_string()
            } else {
                msg
            }
        })?);
    }

    Ok(games)
}

#[tauri::command]
pub fn get_active_processes(state: State<'_, AppState>) -> Result<HashMap<u32, String>, String> {
    let proc_guard = state
        .active_processes
        .lock()
        .map_err(|_| "Failed to lock process state".to_string())?;
    Ok(proc_guard.clone())
}

#[tauri::command]
pub fn get_detected_games(state: State<'_, AppState>) -> Result<Vec<Game>, String> {
    let games_guard = state
        .active_games
        .lock()
        .map_err(|_| "Failed to lock games state".to_string())?;
    Ok(games_guard.values().cloned().collect())
}

use crate::models::GameSession;
use crate::tracking::tracker::ActiveSession;

#[tauri::command]
pub fn get_active_sessions(state: State<'_, AppState>) -> Result<Vec<ActiveSession>, String> {
    let tracker_guard = state
        .session_tracker
        .lock()
        .map_err(|_| "Failed to lock tracker".to_string())?;
    Ok(tracker_guard.active_sessions.values().cloned().collect())
}

#[tauri::command]
pub fn get_recent_sessions(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<Vec<GameSession>, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::database::get_recent_sessions(db, limit).map_err(|e| e.to_string())
}

use crate::models::GameLibraryItem;

#[tauri::command]
pub fn get_library(state: State<'_, AppState>) -> Result<Vec<GameLibraryItem>, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let mut games = crate::database::get_library_games(db).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;

    if let Ok(tracker) = state.session_tracker.lock() {
        for game in games.iter_mut() {
            if tracker.active_sessions.contains_key(&game.id) {
                game.currently_playing = true;
            }
        }
    }

    Ok(games)
}

use crate::stats::dto::DashboardStats;

#[tauri::command]
pub fn get_game_details(
    state: State<'_, AppState>,
    game_id: i64,
) -> Result<GameLibraryItem, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let mut games = crate::database::get_library_games(db).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;

    let mut game = games
        .into_iter()
        .find(|g| g.id == game_id)
        .ok_or("Game not found".to_string())?;

    if let Ok(tracker) = state.session_tracker.lock() {
        if tracker.active_sessions.contains_key(&game.id) {
            game.currently_playing = true;
        }
    }

    Ok(game)
}

#[tauri::command]
pub fn get_game_stats(state: State<'_, AppState>, game_id: i64) -> Result<GameStatistics, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let mut active_elapsed = 0;
    if let Ok(tracker) = state.session_tracker.lock() {
        if let Some(session) = tracker.active_sessions.get(&game_id) {
            active_elapsed = (chrono::Utc::now() - session.started_at)
                .num_seconds()
                .max(0);
        }
    }

    get_game_statistics(db, game_id, active_elapsed)
}

#[tauri::command]
pub fn get_game_history(
    state: State<'_, AppState>,
    game_id: i64,
    days: i64,
) -> Result<Vec<DailyPlaytime>, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let mut active_elapsed = 0;
    if let Ok(tracker) = state.session_tracker.lock() {
        if let Some(session) = tracker.active_sessions.get(&game_id) {
            active_elapsed = (chrono::Utc::now() - session.started_at)
                .num_seconds()
                .max(0);
        }
    }

    get_game_playtime_history(db, game_id, days, active_elapsed)
}

#[tauri::command]
pub fn get_game_session_list(
    state: State<'_, AppState>,
    game_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<GameSessionDto>, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let mut active_start = None;
    if let Ok(tracker) = state.session_tracker.lock() {
        if let Some(session) = tracker.active_sessions.get(&game_id) {
            active_start = Some(session.started_at);
        }
    }

    get_game_sessions(db, game_id, limit, offset, active_start)
}

#[tauri::command]
pub fn get_dashboard_stats(
    state: State<'_, AppState>,
    time_range: String,
) -> Result<DashboardStats, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let active_sessions: Vec<_> = {
        if let Ok(tracker) = state.session_tracker.lock() {
            tracker.active_sessions.values().cloned().collect()
        } else {
            Vec::new()
        }
    };

    crate::stats::queries::get_dashboard_stats(db, &active_sessions, &time_range)
        .map_err(|e| e.to_string())
}

use crate::discovery::models::GameCandidate;

#[tauri::command]
pub fn scan_folder(path: String) -> Result<Vec<GameCandidate>, String> {
    Ok(crate::discovery::scanner::scan_folder(&path))
}

#[tauri::command]
pub fn add_game(state: State<'_, AppState>, candidate: GameCandidate) -> Result<i64, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let game_id = crate::database::add_discovered_game(db, &candidate).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;

    // Refresh the executables cache in GameDetector
    let execs = crate::database::get_all_executables(db).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;
    if let Ok(mut detector) = state.detector.lock() {
        detector.refresh_cache(execs);
    }

    Ok(game_id)
}

#[tauri::command]
pub fn get_onboarding_status(state: State<'_, AppState>) -> Result<bool, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let val: Option<String> = db
        .query_row(
            "SELECT value FROM settings WHERE key = 'onboarding_completed'",
            [],
            |r| r.get(0),
        )
        .ok();

    Ok(val.as_deref() == Some("true"))
}

#[tauri::command]
pub fn complete_onboarding(state: State<'_, AppState>) -> Result<(), String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    db.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('onboarding_completed', 'true')",
        [],
    )
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;

    Ok(())
}

use std::sync::atomic::Ordering;

#[tauri::command]
pub fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::settings::get_setting(db, &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let mut db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_mut().ok_or("Database not initialized")?;
    crate::settings::set_setting(db, &key, &value).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;

    // Update atomics if necessary
    if key == "tracking_enabled" {
        let enabled = value == "true";
        state.tracking_enabled.store(enabled, Ordering::SeqCst);
        if !enabled {
            if let Ok(mut tracker) = state.session_tracker.lock() {
                let _ = tracker.close_all(chrono::Utc::now(), db);
            }
            if let Ok(mut games) = state.active_games.lock() {
                games.clear();
            }
        }
    } else if key == "polling_interval_seconds" {
        if let Ok(val) = value.parse::<u64>() {
            state.polling_interval_secs.store(val, Ordering::SeqCst);
        }
    }

    Ok(())
}

#[tauri::command]
pub fn set_game_ignored(
    state: State<'_, AppState>,
    game_id: i64,
    is_ignored: bool,
) -> Result<(), String> {
    let mut db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_mut().ok_or("Database not initialized")?;
    crate::settings::set_game_ignored(db, game_id, is_ignored).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;

    if is_ignored {
        if let Ok(mut tracker) = state.session_tracker.lock() {
            if let Some(session) = tracker.active_sessions.remove(&game_id) {
                let ended_at = chrono::Utc::now();
                let duration = (ended_at.signed_duration_since(session.started_at))
                    .num_seconds()
                    .max(0);
                let _ = crate::database::close_session(db, session.session_id, ended_at, duration);
            }
        }
        if let Ok(mut games) = state.active_games.lock() {
            let mut to_remove = None;
            for (pid, game) in games.iter() {
                if game.id == game_id {
                    to_remove = Some(*pid);
                    break;
                }
            }
            if let Some(pid) = to_remove {
                games.remove(&pid);
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn export_data(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    let data = crate::data::export_all_data(db).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;
    let json = serde_json::to_string_pretty(&data).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;
    std::fs::write(&path, json).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;
    Ok(())
}

#[tauri::command]
pub fn import_data(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let json = std::fs::read_to_string(&path).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;
    let data: crate::data::ExportData = serde_json::from_str(&json).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;

    let mut db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_mut().ok_or("Database not initialized")?;

    crate::data::import_all_data(db, data)?;

    // Refresh memory caches
    let execs = crate::database::get_all_executables(db).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;
    if let Ok(mut detector) = state.detector.lock() {
        detector.refresh_cache(execs);
    }

    // Sync atomics
    let tracking = crate::settings::get_tracking_enabled(db).unwrap_or(true);
    state.tracking_enabled.store(tracking, Ordering::SeqCst);
    let polling = crate::settings::get_polling_interval(db).unwrap_or(2);
    state.polling_interval_secs.store(polling, Ordering::SeqCst);

    Ok(())
}

#[tauri::command]
pub fn delete_all_history(state: State<'_, AppState>) -> Result<(), String> {
    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;
    crate::data::delete_all_history(db).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;

    // Clear active sessions in memory
    if let Ok(mut tracker) = state.session_tracker.lock() {
        tracker.active_sessions.clear();
    }
    if let Ok(mut games) = state.active_games.lock() {
        games.clear();
    }

    Ok(())
}

#[tauri::command]
pub fn reset_application(state: State<'_, AppState>) -> Result<(), String> {
    let mut db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_mut().ok_or("Database not initialized")?;
    crate::data::reset_application(db).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;

    // Clear in memory states
    if let Ok(mut tracker) = state.session_tracker.lock() {
        tracker.active_sessions.clear();
    }
    if let Ok(mut games) = state.active_games.lock() {
        games.clear();
    }
    if let Ok(mut detector) = state.detector.lock() {
        detector.refresh_cache(Vec::new());
    }

    Ok(())
}

#[tauri::command]
pub fn set_game_cover(
    app: AppHandle,
    state: State<'_, AppState>,
    game_id: i64,
    source_path: String,
) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;
    let covers_dir = app_data_dir.join("covers");
    std::fs::create_dir_all(&covers_dir).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;

    let source_path_buf = PathBuf::from(&source_path);
    let extension = source_path_buf
        .extension()
        .unwrap_or_default()
        .to_string_lossy();
    let dest_filename = format!("{}.{}", game_id, extension);
    let dest_path = covers_dir.join(&dest_filename);

    std::fs::copy(&source_path, &dest_path).map_err(|e| format!("Failed to copy image: {}", e))?;

    let db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let dest_path_str = dest_path.to_string_lossy().to_string();
    db.execute(
        "UPDATE games SET cover_path = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        rusqlite::params![&dest_path_str, game_id],
    )
    .map_err(|e| format!("Failed to update database: {}", e))?;

    Ok(dest_path_str)
}
#[tauri::command]
pub fn trigger_discovery(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<crate::discovery::models::GameCandidate>, String> {
    let mut db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_mut().ok_or("Database not initialized")?;
    let manager = crate::discovery::manager::DiscoveryManager::new();
    manager.run_discovery(db)
}

#[tauri::command]
pub fn remove_game_from_library(
    state: tauri::State<'_, crate::AppState>,
    game_id: i64,
    delete_history: bool,
) -> Result<(), String> {
    let mut db_guard = state
        .db
        .lock()
        .map_err(|_| "Failed to lock DB".to_string())?;
    let db = db_guard.as_mut().ok_or("Database not initialized")?;

    // First, close active session if this game is currently running
    {
        let mut tracker = state.session_tracker.lock().unwrap();
        let active = std::mem::take(&mut tracker.active_sessions);
        for (active_game_id, session) in active {
            if active_game_id == game_id {
                let ended_at = chrono::Utc::now();
                let duration = (ended_at.signed_duration_since(session.started_at))
                    .num_seconds()
                    .max(0);
                let _ = crate::database::close_session(db, session.session_id, ended_at, duration);
            } else {
                tracker.active_sessions.insert(active_game_id, session);
            }
        }
    }

    if delete_history {
        // Hard delete
        db.execute(
            "DELETE FROM games WHERE id = ?1",
            rusqlite::params![game_id],
        )
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("FOREIGN KEY") {
                "Action failed due to a constraint. Your data is safe.".to_string()
            } else if msg.contains("locked") {
                "Database is currently busy, please try again.".to_string()
            } else {
                msg
            }
        })?;
    } else {
        // Soft delete (remove from detection and UI, keep history)
        db.execute("UPDATE games SET is_removed = 1, current_install_status = 'Not Installed' WHERE id = ?1", rusqlite::params![game_id])
            .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("FOREIGN KEY") {
            "Action failed due to a constraint. Your data is safe.".to_string()
        } else if msg.contains("locked") {
            "Database is currently busy, please try again.".to_string()
        } else {
            msg
        }
    })?;
        // Remove tracking mappings so it's not accidentally tracked
        db.execute(
            "DELETE FROM game_executables WHERE game_id = ?1",
            rusqlite::params![game_id],
        )
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("FOREIGN KEY") {
                "Action failed due to a constraint. Your data is safe.".to_string()
            } else if msg.contains("locked") {
                "Database is currently busy, please try again.".to_string()
            } else {
                msg
            }
        })?;
        db.execute(
            "DELETE FROM game_installations WHERE game_id = ?1",
            rusqlite::params![game_id],
        )
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("FOREIGN KEY") {
                "Action failed due to a constraint. Your data is safe.".to_string()
            } else if msg.contains("locked") {
                "Database is currently busy, please try again.".to_string()
            } else {
                msg
            }
        })?;
    }

    Ok(())
}
