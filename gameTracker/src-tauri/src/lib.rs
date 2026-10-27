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
use detection::monitor;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            // Fallback manual emit just in case deep-link integration fails
            use tauri::Manager;
            use tauri::Emitter;
            for arg in argv {
                if arg.starts_with("gametracker://") {
                    let _ = app.emit("scheme-request-received", arg);
                }
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize Database
            let conn =
                database::initialize_database(app.handle()).expect("Failed to initialize database");

            let active_processes = Arc::new(Mutex::new(HashMap::new()));
            let active_games = Arc::new(Mutex::new(HashMap::new()));
            let session_tracker = Arc::new(Mutex::new(tracking::tracker::SessionTracker::new()));
            let db_conn = Arc::new(Mutex::new(Some(conn)));

            // Recover any crashed sessions
            if let Ok(guard) = db_conn.lock() {
                if let Some(db) = guard.as_ref() {
                    let _ =
                        tracking::tracker::SessionTracker::recover_sessions(db, chrono::Utc::now());
                }
            }

            // Load initial executables for detector
            let initial_execs = {
                let guard = db_conn.lock().unwrap();
                let db = guard.as_ref().unwrap();
                database::get_all_executables(db).unwrap_or_default()
            };
            let detector = Arc::new(Mutex::new(detection::detector::GameDetector::new(
                initial_execs,
            )));

            let (initial_tracking, initial_polling) = {
                let guard = db_conn.lock().unwrap();
                let db = guard.as_ref().unwrap();
                (
                    settings::get_tracking_enabled(db).unwrap_or(true),
                    settings::get_polling_interval(db).unwrap_or(2),
                )
            };

            let auto_discovery_enabled_val = if let Ok(guard) = db_conn.lock() {
                guard
                    .as_ref()
                    .and_then(|db| {
                        crate::settings::get_setting(db, "auto_discovery_enabled")
                            .ok()
                            .flatten()
                    })
                    .unwrap_or("true".to_string())
                    == "true"
            } else {
                true
            };

            let discovery_interval_val = if let Ok(guard) = db_conn.lock() {
                guard
                    .as_ref()
                    .and_then(|db| {
                        crate::settings::get_setting(db, "auto_discovery_interval")
                            .ok()
                            .flatten()
                    })
                    .unwrap_or("1800".to_string())
                    .parse::<u64>()
                    .unwrap_or(1800)
            } else {
                1800
            };

            let tracking_enabled = Arc::new(std::sync::atomic::AtomicBool::new(initial_tracking));
            let polling_interval_secs =
                Arc::new(std::sync::atomic::AtomicU64::new(initial_polling));
            let auto_discovery_enabled = Arc::new(std::sync::atomic::AtomicBool::new(
                auto_discovery_enabled_val,
            ));
            let discovery_interval_secs =
                Arc::new(std::sync::atomic::AtomicU64::new(discovery_interval_val));

            app.manage(AppState {
                db: db_conn.clone(),
                active_processes: active_processes.clone(),
                active_games: active_games.clone(),
                session_tracker: session_tracker.clone(),
                detector: detector.clone(),
                tracking_enabled: tracking_enabled.clone(),
                polling_interval_secs: polling_interval_secs.clone(),
                auto_discovery_enabled: auto_discovery_enabled.clone(),
                discovery_interval_secs: discovery_interval_secs.clone(),
            });

            // Start Discovery Thread
            let discovery_db_conn = db_conn.clone();
            let discovery_enabled_flag = auto_discovery_enabled.clone();
            let discovery_interval_flag = discovery_interval_secs.clone();
            std::thread::spawn(move || {
                let manager = crate::discovery::manager::DiscoveryManager::new();
                loop {
                    let interval =
                        discovery_interval_flag.load(std::sync::atomic::Ordering::SeqCst);

                    if discovery_enabled_flag.load(std::sync::atomic::Ordering::SeqCst) {
                        if let Ok(mut guard) = discovery_db_conn.lock() {
                            if let Some(db) = guard.as_mut() {
                                let _ = manager.run_discovery(db);
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(interval));
                }
            });

            // Start Process Detection Thread
            let (tx, rx) = mpsc::channel();
            monitor::spawn_monitor_thread(tx, active_processes, polling_interval_secs);

            // Handle Process Events
            std::thread::spawn(move || {
                for event in rx {
                    match event {
                        monitor::ProcessEvent::Started { pid, name, path } => {
                            if !tracking_enabled.load(std::sync::atomic::Ordering::SeqCst) {
                                continue;
                            }

                            let identified = if let Ok(detector_guard) = detector.lock() {
                                detector_guard.identify(&name, path.as_deref())
                            } else {
                                None
                            };

                            if let Some(game_exec) = identified {
                                if let Ok(guard) = db_conn.lock() {
                                    if let Some(db) = guard.as_ref() {
                                        if let Ok(Some(game)) =
                                            database::get_game_by_id(db, game_exec.game_id)
                                        {
                                            if game.is_ignored {
                                                continue;
                                            }
                                            if let Ok(mut games) = active_games.lock() {
                                                games.insert(pid, game.clone());
                                            }
                                            if let Ok(mut tracker) = session_tracker.lock() {
                                                let _ = tracker.on_game_started(
                                                    pid,
                                                    game_exec.game_id,
                                                    chrono::Utc::now(),
                                                    db,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        monitor::ProcessEvent::Stopped { pid, .. } => {
                            if let Ok(mut games) = active_games.lock() {
                                games.remove(&pid);
                            }
                            if let Ok(guard) = db_conn.lock() {
                                if let Some(db) = guard.as_ref() {
                                    if let Ok(mut tracker) = session_tracker.lock() {
                                        let _ =
                                            tracker.on_process_stopped(pid, chrono::Utc::now(), db);
                                    }
                                }
                            }
                        }
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
                let db_arc = app_handle.state::<AppState>().db.clone();
                let tracker_arc = app_handle.state::<AppState>().session_tracker.clone();
                if let Ok(guard) = db_arc.lock() {
                    if let Some(db) = guard.as_ref() {
                        if let Ok(mut tracker) = tracker_arc.lock() {
                            let _ = tracker.close_all(chrono::Utc::now(), db);
                        }
                    }
                };
            }
        });
}
