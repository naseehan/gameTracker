//! Automatic updates. They stay off until `plugins.updater` in tauri.conf.json has a
//! public key and an endpoint; RELEASING.md explains how to set them up.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

pub const UPDATE_AVAILABLE: &str = "update-available";

#[derive(Serialize)]
pub struct UpdateStatus {
    pub configured: bool,
    pub current_version: String,
    pub available_version: Option<String>,
    pub notes: Option<String>,
}

pub fn is_configured(app: &AppHandle) -> bool {
    app.config().plugins.0.get("updater").is_some_and(|updater| {
        let has_key = updater["pubkey"].as_str().is_some_and(|k| !k.trim().is_empty());
        let has_endpoint = updater["endpoints"].as_array().is_some_and(|e| !e.is_empty());
        has_key && has_endpoint
    })
}

pub async fn check(app: &AppHandle) -> Result<UpdateStatus, String> {
    let current_version = app.package_info().version.to_string();
    if !is_configured(app) {
        return Ok(UpdateStatus {
            configured: false,
            current_version,
            available_version: None,
            notes: None,
        });
    }
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| format!("Couldn't check for updates: {}", e))?;
    Ok(UpdateStatus {
        configured: true,
        current_version,
        available_version: update.as_ref().map(|u| u.version.clone()),
        notes: update.and_then(|u| u.body),
    })
}

/// Downloads and installs the latest version, then restarts the app.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    if !is_configured(app) {
        return Err("Updates aren't set up for this build.".to_string());
    }
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| format!("Couldn't check for updates: {}", e))?
        .ok_or("You're already on the latest version.")?;
    log::info!("Installing update {}", update.version);
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("Couldn't install the update: {}", e))?;
    app.restart()
}

/// Checks once at startup and tells the UI when a new version is out.
pub fn check_in_background(app: AppHandle) {
    if !is_configured(&app) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        match check(&app).await {
            Ok(UpdateStatus {
                available_version: Some(version),
                ..
            }) => {
                let _ = app.emit(UPDATE_AVAILABLE, version);
            }
            Ok(_) => {}
            Err(e) => log::warn!("{}", e),
        }
    });
}
