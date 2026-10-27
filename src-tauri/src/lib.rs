pub mod auth;
pub mod commands;
pub mod data;
pub mod database;
pub mod detection;
pub mod discovery;
pub mod models;
pub mod settings;
pub mod stats;
pub mod tracking;

use commands::AppState;
use detection::detector::GameDetector;
use detection::monitor;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::Manager;
use tracking::tracker::{SessionTracker, HEARTBEAT_INTERVAL_SECS};

/// Locks a mutex even if another thread panicked while holding it. The data behind
/// these locks stays valid after a panic (an unfinished SQLite transaction rolls back
/// when dropped), so one failure shouldn't make every later command fail too.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn handle_process_event(state: &AppState, event: monitor::ProcessEvent) {
    match event {
        monitor::ProcessEvent::Started { pid, name, path } => {
            if !state.tracking_enabled.load(Ordering::SeqCst) {
                return;
            }
            let Some(game_exec) = lock(&state.detector).identify(&name, path.as_deref()) else {
                return;
            };

            let db = lock(&state.db);
            let Ok(Some(game)) = database::get_game_by_id(&db, game_exec.game_id) else {
                return;
            };
            if game.is_ignored || game.is_removed {
                return;
            }
            lock(&state.active_games).insert(pid, game);
            let _ = lock(&state.session_tracker).on_game_started(
                pid,
                game_exec.game_id,
                chrono::Utc::now(),
                &db,
            );
        }
        monitor::ProcessEvent::Stopped { pid, .. } => {
            lock(&state.active_games).remove(&pid);
            let db = lock(&state.db);
            let _ = lock(&state.session_tracker).on_process_stopped(pid, chrono::Utc::now(), &db);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::Builder::default().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let _ = app.deep_link().register_all();
            }

            // Initialize Database
            let conn =
                database::initialize_database(app.handle()).expect("Failed to initialize database");

            // Close sessions left open by a crash or power loss at their last heartbeat.
            let _ = SessionTracker::recover_sessions(&conn);

            let initial_execs = database::get_all_executables(&conn).unwrap_or_default();

            let state = AppState {
                db: Arc::new(Mutex::new(conn)),
                active_processes: Arc::new(Mutex::new(HashMap::new())),
                active_games: Arc::new(Mutex::new(HashMap::new())),
                session_tracker: Arc::new(Mutex::new(SessionTracker::new())),
                detector: Arc::new(Mutex::new(GameDetector::new(initial_execs))),
                tracking_enabled: Arc::new(AtomicBool::new(true)),
                polling_interval_secs: Arc::new(AtomicU64::new(2)),
                auto_discovery_enabled: Arc::new(AtomicBool::new(true)),
                discovery_interval_secs: Arc::new(AtomicU64::new(1800)),
                monitor_resync: Arc::new(AtomicBool::new(false)),
            };
            commands::load_runtime_settings(&state, &lock(&state.db));

            // Start Process Detection Thread
            let (tx, rx) = mpsc::channel();
            monitor::spawn_monitor_thread(
                tx,
                state.active_processes.clone(),
                state.polling_interval_secs.clone(),
                state.monitor_resync.clone(),
            );

            app.manage(state);

            // Start Discovery Thread. It wakes up every 30 seconds so changes to the
            // discovery settings apply without restarting the app.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let state = handle.state::<AppState>();
                let mut last_run: Option<Instant> = None;
                loop {
                    let interval = Duration::from_secs(
                        state.discovery_interval_secs.load(Ordering::SeqCst).max(60),
                    );
                    let due = last_run.map_or(true, |t| t.elapsed() >= interval);
                    if due && state.auto_discovery_enabled.load(Ordering::SeqCst) {
                        let _ = commands::run_discovery(&state);
                        last_run = Some(Instant::now());
                    }
                    std::thread::sleep(Duration::from_secs(30));
                }
            });

            // Handle Process Events, and record a heartbeat for running games so a
            // crash or power loss doesn't lose (or invent) playtime.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let state = handle.state::<AppState>();
                let heartbeat_interval = Duration::from_secs(HEARTBEAT_INTERVAL_SECS);
                let mut last_heartbeat = Instant::now();
                loop {
                    match rx.recv_timeout(heartbeat_interval) {
                        Ok(event) => handle_process_event(&state, event),
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                    if last_heartbeat.elapsed() >= heartbeat_interval {
                        let db = lock(&state.db);
                        let _ = lock(&state.session_tracker).heartbeat(chrono::Utc::now(), &db);
                        last_heartbeat = Instant::now();
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::get_games,
            commands::get_active_processes,
            commands::get_detected_games,
            commands::get_active_sessions,
            commands::get_recent_sessions,
            commands::get_library,
            commands::get_dashboard_stats,
            commands::get_game_details,
            commands::get_game_stats,
            commands::get_game_history,
            commands::get_game_session_list,
            commands::scan_folder,
            commands::add_game,
            commands::get_onboarding_status,
            commands::complete_onboarding,
            commands::get_setting,
            auth::store_auth_token,
            auth::get_auth_token,
            auth::delete_auth_token,
            auth::start_oauth_listener,
            commands::update_setting,
            commands::set_game_ignored,
            commands::set_game_cover,
            commands::trigger_discovery,
            commands::remove_game_from_library,
            commands::export_data,
            commands::import_data,
            commands::delete_all_history,
            commands::reset_application
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app_handle.state::<AppState>();
                let db = lock(&state.db);
                let _ = lock(&state.session_tracker).close_all(chrono::Utc::now(), &db);
            }
        });
}
