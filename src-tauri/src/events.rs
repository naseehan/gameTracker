//! Events the UI listens to, so it refreshes when something changes instead of polling.

use tauri::{AppHandle, Emitter};

/// Games were added, removed, renamed, or their details or covers changed.
pub const LIBRARY_CHANGED: &str = "library-changed";
/// A session started or ended, or session history changed.
pub const SESSIONS_CHANGED: &str = "sessions-changed";

pub fn library_changed(app: &AppHandle) {
    if let Err(e) = app.emit(LIBRARY_CHANGED, ()) {
        log::warn!("Failed to emit {}: {}", LIBRARY_CHANGED, e);
    }
}

pub fn sessions_changed(app: &AppHandle) {
    if let Err(e) = app.emit(SESSIONS_CHANGED, ()) {
        log::warn!("Failed to emit {}: {}", SESSIONS_CHANGED, e);
    }
}
