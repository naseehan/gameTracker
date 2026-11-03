use crate::database::editing;
use crate::discovery::manager::DiscoveryManager;
use crate::discovery::models::GameCandidate;
use crate::events;
use crate::lock;
use crate::models::{GameDetection, GameLibraryItem, RecentSession, RemovedGame};
use crate::settings;
use crate::stats::dto::{DailyPlaytime, DashboardStats};
use crate::stats::game_queries::{
    get_game_playtime_history, get_game_sessions, get_game_statistics, GameSessionDto,
    GameStatistics,
};
use crate::tracking::tracker::ActiveSession;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

// Every command is async so that Tauri runs it off the main thread; a slow command
// never freezes the window. Commands that scan folders, show file dialogs or read and
// write files also move to a blocking thread with `run_blocking`.
//
// The UI never passes file paths to read or write. Commands that work with files open
// the file dialog themselves, so a script injected into the page couldn't use them to
// reach arbitrary files.

pub struct AppState {
    pub app: AppHandle,
    pub db: Arc<Mutex<Connection>>,
    pub session_tracker: Arc<Mutex<crate::tracking::tracker::SessionTracker>>,
    pub detector: Arc<Mutex<crate::detection::detector::GameDetector>>,
    pub tracking_enabled: Arc<AtomicBool>,
    pub polling_interval_secs: Arc<AtomicU64>,
    pub auto_discovery_enabled: Arc<AtomicBool>,
    pub discovery_interval_secs: Arc<AtomicU64>,
    /// Makes the process monitor report every running process again, so games that
    /// are already open get new sessions after tracking state was reset.
    pub monitor_resync: Arc<AtomicBool>,
    pub cover_lookup_running: AtomicBool,
}

/// Turns a database error into a message that is safe to show in the UI.
fn db_error(e: rusqlite::Error) -> String {
    log::error!("Database error: {}", e);
    let msg = e.to_string();
    if msg.contains("FOREIGN KEY") {
        "Action failed due to a constraint. Your data is safe.".to_string()
    } else if msg.contains("locked") {
        "Database is currently busy, please try again.".to_string()
    } else {
        msg
    }
}

/// Runs slow work (folder scans, file dialogs, file I/O) on a blocking thread.
async fn run_blocking<T, F>(app: AppHandle, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || work(app.state::<AppState>().inner()))
        .await
        .map_err(|e| e.to_string())?
}

/// Reloads the executables and install folders used for process detection. Call after
/// any change to them, or new games aren't detected and removed ones still are.
fn refresh_detector(state: &AppState, db: &Connection) -> Result<(), String> {
    let detector = crate::database::load_detector(db).map_err(db_error)?;
    *lock(&state.detector) = detector;
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
    if let Err(e) = lock(&state.session_tracker).end_game_session(game_id, Utc::now(), db) {
        log::error!("Failed to end the session of game {}: {}", game_id, e);
    }
}

/// Ends every running session.
fn stop_all_tracking(state: &AppState, db: &Connection) {
    if let Err(e) = lock(&state.session_tracker).close_all(Utc::now(), db) {
        log::error!("Failed to end running sessions: {}", e);
    }
}

fn active_session(state: &AppState, game_id: i64) -> Option<ActiveSession> {
    lock(&state.session_tracker)
        .active_sessions
        .get(&game_id)
        .cloned()
}

fn picked_path(path: Option<tauri_plugin_dialog::FilePath>) -> Result<Option<PathBuf>, String> {
    path.map(|p| p.into_path().map_err(|e| e.to_string())).transpose()
}

/// Scans the game launchers and records what they report. Only the final write holds
/// the database lock, so the UI and process tracking keep working during the scan.
pub fn run_discovery(state: &AppState) -> Result<Vec<GameCandidate>, String> {
    let manager = DiscoveryManager::new();
    let enabled = manager.enabled_providers(&lock(&state.db));
    let scan = manager.scan(&enabled);

    {
        let mut db = lock(&state.db);
        manager.apply(&mut db, &scan)?;
        refresh_detector(state, &db)?;
    }
    events::library_changed(&state.app);
    crate::covers::spawn_cover_lookup(state.app.clone());
    Ok(scan.candidates)
}

/// A running session with its game's name and cover, for "now playing" displays.
#[derive(Serialize)]
pub struct ActiveSessionInfo {
    pub session_id: i64,
    pub game_id: i64,
    pub game_name: String,
    pub cover_path: Option<String>,
    pub started_at: DateTime<Utc>,
}

#[tauri::command]
pub async fn get_active_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<ActiveSessionInfo>, String> {
    let sessions: Vec<ActiveSession> = lock(&state.session_tracker)
        .active_sessions
        .values()
        .cloned()
        .collect();
    let db = lock(&state.db);
    let mut infos = Vec::new();
    for session in sessions {
        let game: Option<(String, Option<String>)> = db
            .query_row(
                "SELECT name, cover_path FROM games WHERE id = ?1",
                [session.game_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(db_error)?;
        if let Some((game_name, cover_path)) = game {
            infos.push(ActiveSessionInfo {
                session_id: session.session_id,
                game_id: session.game_id,
                game_name,
                cover_path,
                started_at: session.started_at,
            });
        }
    }
    infos.sort_by_key(|s| s.started_at);
    Ok(infos)
}

#[tauri::command]
pub async fn get_recent_sessions(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<Vec<RecentSession>, String> {
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
        .ok_or("This game isn't in your library anymore.".to_string())?;
    game.currently_playing = active_session(&state, game_id).is_some();

    Ok(game)
}

#[tauri::command]
pub async fn get_game_stats(
    state: State<'_, AppState>,
    game_id: i64,
) -> Result<GameStatistics, String> {
    let active_start = active_session(&state, game_id).map(|s| s.started_at);
    get_game_statistics(&lock(&state.db), game_id, active_start)
}

#[tauri::command]
pub async fn get_game_history(
    state: State<'_, AppState>,
    game_id: i64,
    days: i64,
) -> Result<Vec<DailyPlaytime>, String> {
    let active_start = active_session(&state, game_id).map(|s| s.started_at);
    get_game_playtime_history(&lock(&state.db), game_id, days, active_start)
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

/// Lets the user pick a folder and scans it for games. Returns None if they cancel.
#[tauri::command]
pub async fn pick_and_scan_folder(app: AppHandle) -> Result<Option<Vec<GameCandidate>>, String> {
    run_blocking(app, |state| {
        let folder = state
            .app
            .dialog()
            .file()
            .set_title("Choose a folder with games")
            .blocking_pick_folder();
        Ok(picked_path(folder)?
            .map(|folder| crate::discovery::scanner::scan_folder(&folder.to_string_lossy())))
    })
    .await
}

/// Lets the user pick a folder, for typing into a form. Returns None if they cancel.
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<String>, String> {
    run_blocking(app, |state| {
        let folder = state.app.dialog().file().blocking_pick_folder();
        Ok(picked_path(folder)?.map(|p| p.to_string_lossy().to_string()))
    })
    .await
}

/// Lets the user pick an executable, for typing into a form. Returns None if they cancel.
#[tauri::command]
pub async fn pick_executable(app: AppHandle) -> Result<Option<String>, String> {
    run_blocking(app, |state| {
        let file = state
            .app
            .dialog()
            .file()
            .add_filter("Programs", &["exe"])
            .blocking_pick_file();
        Ok(picked_path(file)?.map(|p| p.to_string_lossy().to_string()))
    })
    .await
}

#[tauri::command]
pub async fn add_game(state: State<'_, AppState>, candidate: GameCandidate) -> Result<i64, String> {
    let game_id = {
        let db = lock(&state.db);
        let game_id = crate::database::add_discovered_game(&db, &candidate).map_err(db_error)?;
        refresh_detector(&state, &db)?;
        game_id
    };
    events::library_changed(&state.app);
    crate::covers::spawn_cover_lookup(state.app.clone());
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
            events::sessions_changed(&state.app);
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
    {
        let db = lock(&state.db);
        settings::set_game_ignored(&db, game_id, is_ignored).map_err(db_error)?;
        if is_ignored {
            stop_tracking_game(&state, &db, game_id);
        } else {
            // Start tracking the game if it's running right now.
            state.monitor_resync.store(true, Ordering::SeqCst);
        }
    }
    events::library_changed(&state.app);
    events::sessions_changed(&state.app);
    Ok(())
}

/// Saves all data to a JSON file the user picks. Returns false if they cancel.
#[tauri::command]
pub async fn export_data(app: AppHandle) -> Result<bool, String> {
    run_blocking(app, |state| {
        let file = state
            .app
            .dialog()
            .file()
            .add_filter("JSON", &["json"])
            .set_file_name("gametracker_export.json")
            .blocking_save_file();
        let Some(path) = picked_path(file)? else {
            return Ok(false);
        };
        let data = crate::data::export_all_data(&lock(&state.db)).map_err(db_error)?;
        let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| format!("Couldn't save the file: {}", e))?;
        log::info!("Exported data to {}", path.display());
        Ok(true)
    })
    .await
}

/// Replaces all data with a JSON export the user picks. Returns false if they cancel.
#[tauri::command]
pub async fn import_data(app: AppHandle) -> Result<bool, String> {
    let imported = run_blocking(app.clone(), |state| {
        let file = state
            .app
            .dialog()
            .file()
            .add_filter("JSON", &["json"])
            .blocking_pick_file();
        let Some(path) = picked_path(file)? else {
            return Ok(false);
        };
        let json = std::fs::read_to_string(&path).map_err(|e| format!("Couldn't read the file: {}", e))?;
        let data: crate::data::ExportData = serde_json::from_str(&json)
            .map_err(|e| format!("That file isn't a GameTracker export: {}", e))?;

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
        result.map(|_| true)
    })
    .await?;
    if imported {
        events::library_changed(&app);
        events::sessions_changed(&app);
    }
    Ok(imported)
}

#[tauri::command]
pub async fn delete_all_history(state: State<'_, AppState>) -> Result<(), String> {
    {
        let db = lock(&state.db);
        stop_all_tracking(&state, &db);
        crate::data::delete_all_history(&db).map_err(db_error)?;
    }
    // Games that are still running get new sessions.
    state.monitor_resync.store(true, Ordering::SeqCst);
    events::library_changed(&state.app);
    events::sessions_changed(&state.app);
    Ok(())
}

#[tauri::command]
pub async fn reset_application(state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut db = lock(&state.db);
        stop_all_tracking(&state, &db);
        crate::data::reset_application(&mut db).map_err(db_error)?;
        refresh_detector(&state, &db)?;
        load_runtime_settings(&state, &db);
    }
    if let Ok(dir) = state.app.path().app_data_dir() {
        let _ = std::fs::remove_dir_all(dir.join("covers"));
    }
    events::library_changed(&state.app);
    events::sessions_changed(&state.app);
    Ok(())
}

const COVER_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

/// Lets the user pick an image as a game's cover. Returns the new cover's path, or
/// None if they cancel.
#[tauri::command]
pub async fn choose_game_cover(app: AppHandle, game_id: i64) -> Result<Option<String>, String> {
    let cover = run_blocking(app.clone(), move |state| {
        let file = state
            .app
            .dialog()
            .file()
            .add_filter("Images", &COVER_EXTENSIONS)
            .blocking_pick_file();
        let Some(source) = picked_path(file)? else {
            return Ok(None);
        };
        set_game_cover(state, game_id, &source).map(Some)
    })
    .await?;
    if cover.is_some() {
        events::library_changed(&app);
    }
    Ok(cover)
}

fn set_game_cover(state: &AppState, game_id: i64, source: &std::path::Path) -> Result<String, String> {
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .filter(|e| COVER_EXTENSIONS.contains(&e.as_str()))
        .ok_or("Covers must be PNG, JPG, WebP or GIF images.")?;

    let covers_dir = state
        .app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    std::fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    // A new file name for every change, so the webview can't show a cached old cover.
    let dest_path = covers_dir.join(format!(
        "{}-{}.{}",
        game_id,
        Utc::now().timestamp_millis(),
        extension
    ));
    std::fs::copy(source, &dest_path).map_err(|e| format!("Failed to copy image: {}", e))?;
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
    {
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

        refresh_detector(&state, &db)?;
    }
    events::library_changed(&state.app);
    events::sessions_changed(&state.app);
    Ok(())
}

#[tauri::command]
pub async fn rename_game(state: State<'_, AppState>, game_id: i64, name: String) -> Result<(), String> {
    editing::rename_game(&lock(&state.db), game_id, &name)?;
    events::library_changed(&state.app);
    Ok(())
}

#[tauri::command]
pub async fn update_session(
    state: State<'_, AppState>,
    session_id: i64,
    started_at: DateTime<Utc>,
    ended_at: DateTime<Utc>,
) -> Result<(), String> {
    editing::update_session(&lock(&state.db), session_id, started_at, ended_at)?;
    events::sessions_changed(&state.app);
    Ok(())
}

#[tauri::command]
pub async fn delete_session(state: State<'_, AppState>, session_id: i64) -> Result<(), String> {
    editing::delete_session(&lock(&state.db), session_id)?;
    events::sessions_changed(&state.app);
    Ok(())
}

#[tauri::command]
pub async fn get_game_detection(state: State<'_, AppState>, game_id: i64) -> Result<GameDetection, String> {
    editing::get_game_detection(&lock(&state.db), game_id)
}

/// Lets the user pick an executable that belongs to a game. Returns false if they cancel.
#[tauri::command]
pub async fn add_game_executable(app: AppHandle, game_id: i64) -> Result<bool, String> {
    let added = run_blocking(app.clone(), move |state| {
        let file = state
            .app
            .dialog()
            .file()
            .add_filter("Programs", &["exe"])
            .blocking_pick_file();
        let Some(path) = picked_path(file)? else {
            return Ok(false);
        };
        let db = lock(&state.db);
        editing::add_executable(&db, game_id, &path)?;
        refresh_detector(state, &db)?;
        Ok(true)
    })
    .await?;
    if added {
        events::library_changed(&app);
    }
    Ok(added)
}

#[tauri::command]
pub async fn remove_game_executable(state: State<'_, AppState>, executable_id: i64) -> Result<(), String> {
    {
        let db = lock(&state.db);
        editing::remove_executable(&db, executable_id)?;
        refresh_detector(&state, &db)?;
    }
    events::library_changed(&state.app);
    Ok(())
}

/// Lets the user pick a folder the game is installed in. Returns false if they cancel.
#[tauri::command]
pub async fn add_game_install_folder(app: AppHandle, game_id: i64) -> Result<bool, String> {
    let added = run_blocking(app.clone(), move |state| {
        let folder = state.app.dialog().file().blocking_pick_folder();
        let Some(path) = picked_path(folder)? else {
            return Ok(false);
        };
        let db = lock(&state.db);
        editing::add_install_folder(&db, game_id, &path)?;
        refresh_detector(state, &db)?;
        Ok(true)
    })
    .await?;
    if added {
        events::library_changed(&app);
    }
    Ok(added)
}

#[tauri::command]
pub async fn remove_game_install_folder(state: State<'_, AppState>, install_id: i64) -> Result<(), String> {
    {
        let db = lock(&state.db);
        editing::remove_install_folder(&db, install_id)?;
        refresh_detector(&state, &db)?;
    }
    events::library_changed(&state.app);
    Ok(())
}

#[tauri::command]
pub async fn get_removed_games(state: State<'_, AppState>) -> Result<Vec<RemovedGame>, String> {
    editing::get_removed_games(&lock(&state.db))
}

/// Puts a removed game back, then runs discovery so launcher games get their install
/// folder back and are tracked again.
#[tauri::command]
pub async fn restore_game(app: AppHandle, game_id: i64) -> Result<(), String> {
    editing::restore_game(&lock(&app.state::<AppState>().db), game_id)?;
    events::library_changed(&app);
    let discovery_app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = run_discovery(&discovery_app.state::<AppState>()) {
            log::error!("Discovery after restoring a game failed: {}", e);
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn has_steamgriddb_key() -> Result<bool, String> {
    Ok(crate::covers::steamgriddb_key().is_some())
}

/// Saves the SteamGridDB API key (an empty key removes it) and looks covers up again.
#[tauri::command]
pub async fn set_steamgriddb_key(state: State<'_, AppState>, key: String) -> Result<(), String> {
    let entry = crate::auth::get_keyring_entry(crate::covers::STEAMGRIDDB_KEY_ENTRY)?;
    let key = key.trim();
    if key.is_empty() {
        match entry.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(format!("Couldn't remove the key: {}", e)),
        }
    } else {
        entry
            .set_password(key)
            .map_err(|e| format!("Couldn't save the key: {}", e))?;
        crate::covers::reset_lookups(&lock(&state.db)).map_err(db_error)?;
        crate::covers::spawn_cover_lookup(state.app.clone());
    }
    Ok(())
}

#[tauri::command]
pub async fn open_log_folder(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_update_status(app: AppHandle) -> Result<crate::updates::UpdateStatus, String> {
    crate::updates::check(&app).await
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    crate::updates::install(&app).await
}

/// Whether the app is registered to start (in the tray) when the user signs in to Windows.
#[tauri::command]
pub async fn get_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    let result = if enabled {
        autostart.enable()
    } else {
        autostart.disable()
    };
    result.map_err(|e| e.to_string())
}
