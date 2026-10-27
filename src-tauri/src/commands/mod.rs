use crate::discovery::manager::DiscoveryManager;
use crate::discovery::models::GameCandidate;
use crate::lock;
use crate::models::{Game, GameLibraryItem, GameSession};
use crate::settings;
use crate::stats::dto::{DailyPlaytime, DashboardStats};
use crate::stats::game_queries::{
    get_game_playtime_history, get_game_sessions, get_game_statistics, GameSessionDto,
    GameStatistics,
};
use crate::tracking::tracker::ActiveSession;
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};

// Every command is async so that Tauri runs it off the main thread; a slow command
// never freezes the window. Commands that scan folders or read and write files also
// move to a blocking thread with `run_blocking`.

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub active_processes: Arc<Mutex<HashMap<u32, String>>>,
    pub active_games: Arc<Mutex<HashMap<u32, Game>>>,
    pub session_tracker: Arc<Mutex<crate::tracking::tracker::SessionTracker>>,
    pub detector: Arc<Mutex<crate::detection::detector::GameDetector>>,
    pub tracking_enabled: Arc<AtomicBool>,
    pub polling_interval_secs: Arc<AtomicU64>,
    pub auto_discovery_enabled: Arc<AtomicBool>,
    pub discovery_interval_secs: Arc<AtomicU64>,
    /// Makes the process monitor report every running process again, so games that
    /// are already open get new sessions after tracking state was reset.
    pub monitor_resync: Arc<AtomicBool>,
}

/// Turns a database error into a message that is safe to show in the UI.
fn db_error(e: rusqlite::Error) -> String {
    let msg = e.to_string();
    if msg.contains("FOREIGN KEY") {
        "Action failed due to a constraint. Your data is safe.".to_string()
    } else if msg.contains("locked") {
        "Database is currently busy, please try again.".to_string()
    } else {
        msg
    }
}

/// Runs slow work (folder scans, file I/O) on a blocking thread.
async fn run_blocking<T, F>(app: AppHandle, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || work(app.state::<AppState>().inner()))
        .await
        .map_err(|e| e.to_string())?
}

/// Reloads the executables used for process detection. Call after any change to
/// game_executables, or new games aren't detected and removed ones still are.
fn refresh_detector(state: &AppState, db: &Connection) -> Result<(), String> {
    let execs = crate::database::get_all_executables(db).map_err(db_error)?;
    lock(&state.detector).refresh_cache(execs);
    Ok(())
}

/// Copies the settings that background threads read into their atomics.
pub fn load_runtime_settings(state: &AppState, db: &Connection) {
    state.tracking_enabled.store(
        settings::get_tracking_enabled(db).unwrap_or(true),
        Ordering::SeqCst,
    );
    state.polling_interval_secs.store(
        settings::get_polling_interval(db).unwrap_or(2),
        Ordering::SeqCst,
    );
    state.auto_discovery_enabled.store(
        settings::get_auto_discovery_enabled(db).unwrap_or(true),
        Ordering::SeqCst,
    );
    state.discovery_interval_secs.store(
        settings::get_discovery_interval(db).unwrap_or(1800),
        Ordering::SeqCst,
    );
}

/// Ends a running game's session (when the game is ignored or removed).
fn stop_tracking_game(state: &AppState, db: &Connection, game_id: i64) {
    let _ = lock(&state.session_tracker).end_game_session(game_id, chrono::Utc::now(), db);
    lock(&state.active_games).retain(|_, game| game.id != game_id);
}

/// Ends every running session.
fn stop_all_tracking(state: &AppState, db: &Connection) {
    let _ = lock(&state.session_tracker).close_all(chrono::Utc::now(), db);
    lock(&state.active_games).clear();
}

fn active_session(state: &AppState, game_id: i64) -> Option<ActiveSession> {
    lock(&state.session_tracker)
        .active_sessions
        .get(&game_id)
        .cloned()
}

fn active_elapsed_seconds(state: &AppState, game_id: i64) -> i64 {
    active_session(state, game_id)
        .map(|s| (chrono::Utc::now() - s.started_at).num_seconds().max(0))
        .unwrap_or(0)
}

/// Scans the game launchers and records what they report. Only the final write holds
/// the database lock, so the UI and process tracking keep working during the scan.
pub fn run_discovery(state: &AppState) -> Result<Vec<GameCandidate>, String> {
    let manager = DiscoveryManager::new();
    let enabled = manager.enabled_providers(&lock(&state.db));
    let scan = manager.scan(&enabled);

    let mut db = lock(&state.db);
    manager.apply(&mut db, &scan)?;
    refresh_detector(state, &db)?;
    Ok(scan.candidates)
}

#[tauri::command]
pub async fn get_games(state: State<'_, AppState>) -> Result<Vec<Game>, String> {
    let db = lock(&state.db);
    let mut stmt = db
        .prepare("SELECT id, name, normalized_name, cover_path, created_at, updated_at, first_seen_at, last_seen_at, first_played_at, last_played_at, current_install_status, is_ignored, is_removed FROM games WHERE is_removed = 0 ORDER BY last_played_at DESC")
        .map_err(db_error)?;

    let games = stmt
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
        .map_err(db_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(db_error)?;

    Ok(games)
}

#[tauri::command]
pub async fn get_active_processes(
    state: State<'_, AppState>,
) -> Result<HashMap<u32, String>, String> {
    Ok(lock(&state.active_processes).clone())
}

#[tauri::command]
pub async fn get_detected_games(state: State<'_, AppState>) -> Result<Vec<Game>, String> {
    Ok(lock(&state.active_games).values().cloned().collect())
}

#[tauri::command]
pub async fn get_active_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<ActiveSession>, String> {
    Ok(lock(&state.session_tracker)
        .active_sessions
        .values()
        .cloned()
        .collect())
}

#[tauri::command]
pub async fn get_recent_sessions(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<Vec<GameSession>, String> {
    crate::database::get_recent_sessions(&lock(&state.db), limit).map_err(db_error)
}

#[tauri::command]
pub async fn get_library(state: State<'_, AppState>) -> Result<Vec<GameLibraryItem>, String> {
    let mut games = crate::database::get_library_games(&lock(&state.db)).map_err(db_error)?;

    let tracker = lock(&state.session_tracker);
    for game in games.iter_mut() {
        game.currently_playing = tracker.active_sessions.contains_key(&game.id);
    }

    Ok(games)
}

#[tauri::command]
pub async fn get_game_details(
    state: State<'_, AppState>,
    game_id: i64,
) -> Result<GameLibraryItem, String> {
    let games = crate::database::get_library_games(&lock(&state.db)).map_err(db_error)?;

    let mut game = games
        .into_iter()
        .find(|g| g.id == game_id)
        .ok_or("Game not found".to_string())?;
    game.currently_playing = active_session(&state, game_id).is_some();

    Ok(game)
}

#[tauri::command]
pub async fn get_game_stats(
    state: State<'_, AppState>,
    game_id: i64,
) -> Result<GameStatistics, String> {
    let active_elapsed = active_elapsed_seconds(&state, game_id);
    get_game_statistics(&lock(&state.db), game_id, active_elapsed)
}

#[tauri::command]
pub async fn get_game_history(
    state: State<'_, AppState>,
    game_id: i64,
    days: i64,
) -> Result<Vec<DailyPlaytime>, String> {
    let active_elapsed = active_elapsed_seconds(&state, game_id);
    get_game_playtime_history(&lock(&state.db), game_id, days, active_elapsed)
}

#[tauri::command]
pub async fn get_game_session_list(
    state: State<'_, AppState>,
    game_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<GameSessionDto>, String> {
    let active_start = active_session(&state, game_id).map(|s| s.started_at);
    get_game_sessions(&lock(&state.db), game_id, limit, offset, active_start)
}

#[tauri::command]
pub async fn get_dashboard_stats(
    state: State<'_, AppState>,
    time_range: String,
) -> Result<DashboardStats, String> {
    let active_sessions: Vec<_> = lock(&state.session_tracker)
        .active_sessions
        .values()
        .cloned()
        .collect();

    crate::stats::queries::get_dashboard_stats(&lock(&state.db), &active_sessions, &time_range)
        .map_err(db_error)
}

#[tauri::command]
pub async fn scan_folder(path: String) -> Result<Vec<GameCandidate>, String> {
    tauri::async_runtime::spawn_blocking(move || crate::discovery::scanner::scan_folder(&path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_game(state: State<'_, AppState>, candidate: GameCandidate) -> Result<i64, String> {
    let db = lock(&state.db);
    let game_id = crate::database::add_discovered_game(&db, &candidate).map_err(db_error)?;
    refresh_detector(&state, &db)?;
    Ok(game_id)
}

#[tauri::command]
pub async fn get_onboarding_status(state: State<'_, AppState>) -> Result<bool, String> {
    let val = settings::get_setting(&lock(&state.db), "onboarding_completed").map_err(db_error)?;
    Ok(val.as_deref() == Some("true"))
}

#[tauri::command]
pub async fn complete_onboarding(state: State<'_, AppState>) -> Result<(), String> {
    settings::set_setting(&lock(&state.db), "onboarding_completed", "true").map_err(db_error)
}

#[tauri::command]
pub async fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    settings::get_setting(&lock(&state.db), &key).map_err(db_error)
}

#[tauri::command]
pub async fn update_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let db = lock(&state.db);
    settings::set_setting(&db, &key, &value).map_err(db_error)?;
    load_runtime_settings(&state, &db);

    if key == "tracking_enabled" {
        if value == "true" {
            // Pick up games that are already running.
            state.monitor_resync.store(true, Ordering::SeqCst);
        } else {
            stop_all_tracking(&state, &db);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn set_game_ignored(
    state: State<'_, AppState>,
    game_id: i64,
    is_ignored: bool,
) -> Result<(), String> {
    let db = lock(&state.db);
    settings::set_game_ignored(&db, game_id, is_ignored).map_err(db_error)?;

    if is_ignored {
        stop_tracking_game(&state, &db, game_id);
    } else {
        // Start tracking the game if it's running right now.
        state.monitor_resync.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[tauri::command]
pub async fn export_data(app: AppHandle, path: String) -> Result<(), String> {
    run_blocking(app, move |state| {
        let data = crate::data::export_all_data(&lock(&state.db)).map_err(db_error)?;
        let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn import_data(app: AppHandle, path: String) -> Result<(), String> {
    run_blocking(app, move |state| {
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let data: crate::data::ExportData =
            serde_json::from_str(&json).map_err(|e| e.to_string())?;

        let mut db = lock(&state.db);
        // Close running sessions first: their rows are about to be replaced, and a
        // session id kept in memory could otherwise overwrite an imported session.
        stop_all_tracking(state, &db);
        let result = crate::data::import_all_data(&mut db, data)
            .and_then(|_| refresh_detector(state, &db));
        load_runtime_settings(state, &db);
        drop(db);

        // Games that are still running get new sessions.
        state.monitor_resync.store(true, Ordering::SeqCst);
        result
    })
    .await
}

#[tauri::command]
pub async fn delete_all_history(state: State<'_, AppState>) -> Result<(), String> {
    let db = lock(&state.db);
    stop_all_tracking(&state, &db);
    crate::data::delete_all_history(&db).map_err(db_error)?;
    // Games that are still running get new sessions.
    state.monitor_resync.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn reset_application(state: State<'_, AppState>) -> Result<(), String> {
    let mut db = lock(&state.db);
    stop_all_tracking(&state, &db);
    crate::data::reset_application(&mut db).map_err(db_error)?;
    refresh_detector(&state, &db)?;
    load_runtime_settings(&state, &db);
    Ok(())
}

const COVER_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

#[tauri::command]
pub async fn set_game_cover(
    app: AppHandle,
    state: State<'_, AppState>,
    game_id: i64,
    source_path: String,
) -> Result<String, String> {
    let source = PathBuf::from(&source_path);
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .filter(|e| COVER_EXTENSIONS.contains(&e.as_str()))
        .ok_or("Covers must be PNG, JPG, WebP or GIF images.")?;

    let covers_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    std::fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    // A new file name for every change, so the webview can't show a cached old cover.
    let dest_path = covers_dir.join(format!(
        "{}-{}.{}",
        game_id,
        chrono::Utc::now().timestamp_millis(),
        extension
    ));
    std::fs::copy(&source, &dest_path).map_err(|e| format!("Failed to copy image: {}", e))?;
    let dest_path_str = dest_path.to_string_lossy().to_string();

    let previous_cover: Option<String> = {
        let db = lock(&state.db);
        let previous = db
            .query_row(
                "SELECT cover_path FROM games WHERE id = ?1",
                rusqlite::params![game_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(db_error)?
            .flatten();
        db.execute(
            "UPDATE games SET cover_path = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            rusqlite::params![&dest_path_str, game_id],
        )
        .map_err(|e| format!("Failed to update database: {}", e))?;
        previous
    };

    // Delete the replaced cover, but only if this app created it.
    if let Some(previous) = previous_cover.map(PathBuf::from) {
        if previous.starts_with(&covers_dir) && previous != dest_path {
            let _ = std::fs::remove_file(previous);
        }
    }

    Ok(dest_path_str)
}

#[tauri::command]
pub async fn trigger_discovery(app: AppHandle) -> Result<Vec<GameCandidate>, String> {
    run_blocking(app, run_discovery).await
}

#[tauri::command]
pub async fn remove_game_from_library(
    state: State<'_, AppState>,
    game_id: i64,
    delete_history: bool,
) -> Result<(), String> {
    let db = lock(&state.db);

    // First, close active session if this game is currently running
    stop_tracking_game(&state, &db, game_id);

    if delete_history {
        // Hard delete
        db.execute("DELETE FROM games WHERE id = ?1", rusqlite::params![game_id])
            .map_err(db_error)?;
    } else {
        // Soft delete (remove from detection and UI, keep history)
        db.execute("UPDATE games SET is_removed = 1, current_install_status = 'Not Installed' WHERE id = ?1", rusqlite::params![game_id])
            .map_err(db_error)?;
        // Remove tracking mappings so it's not accidentally tracked
        db.execute(
            "DELETE FROM game_executables WHERE game_id = ?1",
            rusqlite::params![game_id],
        )
        .map_err(db_error)?;
        db.execute(
            "DELETE FROM game_installations WHERE game_id = ?1",
            rusqlite::params![game_id],
        )
        .map_err(db_error)?;
    }

    refresh_detector(&state, &db)
}
