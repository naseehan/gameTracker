use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};

/// Passed by the "start with Windows" entry, so the app starts in the tray.
pub const MINIMIZED_ARG: &str = "--minimized";

/// Shows the main window and brings it to the front, even if it's hidden or minimized.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Shows the games being tracked in the tray icon's tooltip.
pub fn set_now_playing(app: &AppHandle, games: &[String]) {
    let tooltip = if games.is_empty() {
        "GameTracker".to_string()
    } else {
        format!("GameTracker · Playing {}", games.join(", "))
    };
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

/// Adds the tray icon. The app keeps tracking games while it lives in the tray,
/// so quitting from here is the only way to stop it.
pub fn create_tray(app: &App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open GameTracker", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit (stops tracking)", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("GameTracker")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
