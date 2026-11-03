pub mod auth;
pub mod commands;
pub mod covers;
pub mod data;
pub mod database;
pub mod detection;
pub mod discovery;
pub mod events;
pub mod models;
pub mod settings;
pub mod stats;
pub mod tracking;
pub mod tray;
pub mod updates;

use commands::AppState;
use detection::detector::GameDetector;
use detection::monitor;
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

fn handle_process_event(state: &AppState, event: monitor::ProcessEvent) {
    let changed = match event {
        monitor::ProcessEvent::Started { pid, name, path } => {
            if !state.tracking_enabled.load(Ordering::SeqCst) {
                return;
            }
            let Some(game_id) = lock(&state.detector).identify(&name, path.as_deref()) else {
                return;
            };

            let db = lock(&state.db);
            let Ok(Some(game)) = database::get_game_by_id(&db, game_id) else {
                return;
            };
            if game.is_ignored || game.is_removed {
                return;
            }
            let started = lock(&state.session_tracker).on_game_started(
                pid,
                game_id,
                chrono::Utc::now(),
                &db,
            );
            if let Ok(true) = started {
                log::info!("Started tracking {} (process {})", game.name, name);
            }
            started
        }
        monitor::ProcessEvent::Stopped { pid, .. } => {
            let db = lock(&state.db);
            lock(&state.session_tracker).on_process_stopped(pid, chrono::Utc::now(), &db)
        }
    };
    match changed {
        Ok(true) => sessions_changed(state),
        Ok(false) => {}
        Err(e) => log::error!("Failed to record a session: {}", e),
    }
}

/// Tells the UI and the tray icon that the set of running sessions changed.
fn sessions_changed(state: &AppState) {
    events::sessions_changed(&state.app);
    let game_ids: Vec<i64> = lock(&state.session_tracker)
        .active_sessions
        .keys()
        .copied()
        .collect();
    let db = lock(&state.db);
    let names: Vec<String> = game_ids
        .iter()
        .filter_map(|id| database::get_game_by_id(&db, *id).ok().flatten())
        .map(|game| game.name)
        .collect();
    tray::set_now_playing(&state.app, &names);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Opening the app while it's already running (e.g. from the Start menu while it
        // sits in the tray) brings up the existing window instead of a second copy.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        // Logs go to %LOCALAPPDATA%\com.naseeh.gametracker\logs; Settings links to it.
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("gametracker".to_string()),
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                ])
                .level(log::LevelFilter::Info)
                .max_file_size(2_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(3))
                .build(),
        )
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![tray::MINIMIZED_ARG]),
        ))
        .on_window_event(|window, event| {
            // Closing the window hides it; the app keeps tracking from the tray.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            log::info!("GameTracker {} starting", app.package_info().version);
            tray::create_tray(app)?;
            // The window starts hidden (see tauri.conf.json). Show it unless the app was
            // launched at sign-in, in which case it stays in the tray.
            if !std::env::args().any(|arg| arg == tray::MINIMIZED_ARG) {
                tray::show_main_window(app.handle());
            }

            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let _ = app.deep_link().register_all();
            }

            // Initialize Database
            let conn =
                database::initialize_database(app.handle()).expect("Failed to initialize database");

            // Close sessions left open by a crash or power loss at their last heartbeat.
            if let Err(e) = SessionTracker::recover_sessions(&conn) {
                log::error!("Failed to recover unfinished sessions: {}", e);
            }

            let initial_detector = database::load_detector(&conn)
                .unwrap_or_else(|_| GameDetector::new(Vec::new(), Vec::new()));

            let state = AppState {
                app: app.handle().clone(),
                db: Arc::new(Mutex::new(conn)),
                session_tracker: Arc::new(Mutex::new(SessionTracker::new())),
                detector: Arc::new(Mutex::new(initial_detector)),
                tracking_enabled: Arc::new(AtomicBool::new(true)),
                polling_interval_secs: Arc::new(AtomicU64::new(2)),
                auto_discovery_enabled: Arc::new(AtomicBool::new(true)),
                discovery_interval_secs: Arc::new(AtomicU64::new(1800)),
                monitor_resync: Arc::new(AtomicBool::new(false)),
                cover_lookup_running: AtomicBool::new(false),
            };
            commands::load_runtime_settings(&state, &lock(&state.db));

            // Start Process Detection Thread
            let (tx, rx) = mpsc::channel();
            monitor::spawn_monitor_thread(
                tx,
                state.polling_interval_secs.clone(),
                state.monitor_resync.clone(),
            );

            app.manage(state);
            covers::spawn_cover_lookup(app.handle().clone());
            updates::check_in_background(app.handle().clone());

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
                        if let Err(e) = commands::run_discovery(&state) {
                            log::error!("Game discovery failed: {}", e);
                        }
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
                        let split = {
                            let db = lock(&state.db);
                            let result = lock(&state.session_tracker).heartbeat(chrono::Utc::now(), &db);
                            result
                        };
                        match split {
                            Ok(true) => sessions_changed(&state),
                            Ok(false) => {}
                            Err(e) => log::error!("Failed to record a heartbeat: {}", e),
                        }
                        last_heartbeat = Instant::now();
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_active_sessions,
            commands::get_recent_sessions,
            commands::get_library,
            commands::get_dashboard_stats,
            commands::get_game_details,
            commands::get_game_stats,
            commands::get_game_history,
            commands::get_game_session_list,
            commands::pick_and_scan_folder,
            commands::pick_folder,
            commands::pick_executable,
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
            commands::choose_game_cover,
            commands::trigger_discovery,
            commands::remove_game_from_library,
            commands::export_data,
            commands::import_data,
            commands::delete_all_history,
            commands::reset_application,
            commands::rename_game,
            commands::update_session,
            commands::delete_session,
            commands::get_game_detection,
            commands::add_game_executable,
            commands::remove_game_executable,
            commands::add_game_install_folder,
            commands::remove_game_install_folder,
            commands::get_removed_games,
            commands::restore_game,
            commands::has_steamgriddb_key,
            commands::set_steamgriddb_key,
            commands::open_log_folder,
            commands::get_update_status,
            commands::install_update,
            commands::get_autostart_enabled,
            commands::set_autostart_enabled
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app_handle.state::<AppState>();
                let db = lock(&state.db);
                if let Err(e) = lock(&state.session_tracker).close_all(chrono::Utc::now(), &db) {
                    log::error!("Failed to close sessions on exit: {}", e);
                }
                log::info!("GameTracker exiting");
            }
        });
}
