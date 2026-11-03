//! Finds cover art for games that don't have one. Steam games use Steam's local library
//! cache, or its CDN; other games use SteamGridDB when the user has added an API key.
//! Custom covers are never replaced.

use crate::commands::AppState;
use crate::lock;
use chrono::{Duration, Utc};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tauri::Manager;

/// Games without a cover are looked up again after this long, in case art appeared.
const RETRY_AFTER_DAYS: i64 = 7;
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
const STEAMGRIDDB_API: &str = "https://www.steamgriddb.com/api/v2";
/// Keyring entry holding the user's SteamGridDB API key.
pub const STEAMGRIDDB_KEY_ENTRY: &str = "steamgriddb_api_key";

struct CoverJob {
    game_id: i64,
    name: String,
    steam_app_id: Option<String>,
}

pub fn steamgriddb_key() -> Option<String> {
    crate::auth::get_keyring_entry(STEAMGRIDDB_KEY_ENTRY)
        .ok()?
        .get_password()
        .ok()
        .filter(|key| !key.trim().is_empty())
}

/// Looks up covers for games that need one, on a background thread. Does nothing if a
/// lookup is already running.
pub fn spawn_cover_lookup(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        if state.cover_lookup_running.swap(true, Ordering::SeqCst) {
            return;
        }
        let found = fetch_missing_covers(&state);
        state.cover_lookup_running.store(false, Ordering::SeqCst);
        if found > 0 {
            log::info!("Found cover art for {} games", found);
            crate::events::library_changed(&app);
        }
    });
}

/// Makes every game without a cover eligible for lookup again (e.g. after the user adds
/// a SteamGridDB key).
pub fn reset_lookups(db: &Connection) -> rusqlite::Result<()> {
    db.execute("UPDATE games SET cover_lookup_at = NULL WHERE cover_path IS NULL", [])?;
    Ok(())
}

fn fetch_missing_covers(state: &AppState) -> usize {
    let Ok(covers_dir) = state.app.path().app_data_dir().map(|d| d.join("covers")) else {
        return 0;
    };
    let jobs = match games_needing_covers(&lock(&state.db)) {
        Ok(jobs) => jobs,
        Err(e) => {
            log::error!("Failed to list games needing covers: {}", e);
            return 0;
        }
    };
    if jobs.is_empty() {
        return 0;
    }
    if std::fs::create_dir_all(&covers_dir).is_err() {
        return 0;
    }

    let client = match reqwest::blocking::Client::builder()
        .tls_backend_native()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(concat!("GameTracker/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            log::error!("Failed to create HTTP client: {}", e);
            return 0;
        }
    };
    let api_key = steamgriddb_key();
    let steam_cache = crate::discovery::steam::steam_install_path()
        .map(|steam| steam.join("appcache").join("librarycache"));

    let mut found = 0;
    for job in jobs {
        // Network and disk work happens without the database lock.
        let image = job
            .steam_app_id
            .as_deref()
            .and_then(|app_id| {
                steam_cache
                    .as_deref()
                    .and_then(|cache| steam_cached_cover(cache, app_id))
                    .or_else(|| steam_cdn_cover(&client, app_id))
            })
            .or_else(|| {
                api_key
                    .as_deref()
                    .and_then(|key| steamgriddb_cover(&client, key, &job))
            });

        let saved = image.and_then(|(bytes, extension)| {
            let path = covers_dir.join(format!(
                "{}-auto-{}.{}",
                job.game_id,
                Utc::now().timestamp_millis(),
                extension
            ));
            std::fs::write(&path, bytes).ok().map(|_| path)
        });

        let db = lock(&state.db);
        let result = match &saved {
            // Only fill an empty cover: the user may have picked one meanwhile.
            Some(path) => db.execute(
                "UPDATE games SET cover_path = ?1, cover_lookup_at = ?2 WHERE id = ?3 AND cover_path IS NULL",
                rusqlite::params![path.to_string_lossy(), Utc::now(), job.game_id],
            ),
            None => db.execute(
                "UPDATE games SET cover_lookup_at = ?1 WHERE id = ?2",
                rusqlite::params![Utc::now(), job.game_id],
            ),
        };
        match result {
            Ok(1) if saved.is_some() => found += 1,
            Ok(_) => {
                if let Some(path) = saved {
                    let _ = std::fs::remove_file(path);
                }
            }
            Err(e) => log::error!("Failed to save cover for game {}: {}", job.game_id, e),
        }
    }
    found
}

fn games_needing_covers(db: &Connection) -> rusqlite::Result<Vec<CoverJob>> {
    let retry_before = Utc::now() - Duration::days(RETRY_AFTER_DAYS);
    let mut stmt = db.prepare(
        "SELECT g.id, g.name,
                (SELECT external_id FROM game_platforms WHERE game_id = g.id AND platform = 'Steam' LIMIT 1)
         FROM games g
         WHERE g.cover_path IS NULL AND g.is_removed = 0
           AND (g.cover_lookup_at IS NULL OR g.cover_lookup_at < ?1)",
    )?;
    let rows = stmt.query_map([retry_before], |r| {
        Ok(CoverJob {
            game_id: r.get(0)?,
            name: r.get(1)?,
            steam_app_id: r.get(2)?,
        })
    })?;
    rows.collect()
}

/// Steam keeps library art on disk, either as `<appid>_library_600x900.jpg` or, in newer
/// clients, in a folder per app.
fn steam_cached_cover(cache: &Path, app_id: &str) -> Option<(Vec<u8>, String)> {
    let flat = cache.join(format!("{}_library_600x900.jpg", app_id));
    let candidate: Option<PathBuf> = if flat.is_file() {
        Some(flat)
    } else {
        walkdir::WalkDir::new(cache.join(app_id))
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.into_path())
            .find(|p| {
                p.file_name()
                    .is_some_and(|n| n.eq_ignore_ascii_case("library_600x900.jpg"))
            })
    };
    let bytes = std::fs::read(candidate?).ok()?;
    (bytes.len() <= MAX_IMAGE_BYTES).then(|| (bytes, "jpg".to_string()))
}

fn steam_cdn_cover(client: &reqwest::blocking::Client, app_id: &str) -> Option<(Vec<u8>, String)> {
    [
        format!("https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{}/library_600x900_2x.jpg", app_id),
        format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{}/library_600x900.jpg", app_id),
    ]
    .iter()
    .find_map(|url| download_image(client, url))
}

fn steamgriddb_cover(
    client: &reqwest::blocking::Client,
    api_key: &str,
    job: &CoverJob,
) -> Option<(Vec<u8>, String)> {
    let grids_url = match &job.steam_app_id {
        Some(app_id) => format!("{}/grids/steam/{}?dimensions=600x900", STEAMGRIDDB_API, app_id),
        None => {
            let mut search = reqwest::Url::parse(&format!("{}/search/autocomplete", STEAMGRIDDB_API)).ok()?;
            search.path_segments_mut().ok()?.push(&job.name);
            let results: serde_json::Value = client
                .get(search)
                .bearer_auth(api_key)
                .send()
                .ok()?
                .error_for_status()
                .ok()?
                .json()
                .ok()?;
            let game_id = results["data"][0]["id"].as_i64()?;
            format!("{}/grids/game/{}?dimensions=600x900", STEAMGRIDDB_API, game_id)
        }
    };
    let grids: serde_json::Value = client
        .get(grids_url)
        .bearer_auth(api_key)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .ok()?;
    let image_url = grids["data"][0]["url"].as_str()?;
    download_image(client, image_url)
}

fn download_image(client: &reqwest::blocking::Client, url: &str) -> Option<(Vec<u8>, String)> {
    let response = client.get(url).send().ok()?.error_for_status().ok()?;
    let extension = match response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)?
        .to_str()
        .ok()?
    {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => return None,
    };
    let bytes = response.bytes().ok()?;
    (bytes.len() <= MAX_IMAGE_BYTES).then(|| (bytes.to_vec(), extension.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finds_steam_cached_cover_in_both_layouts() {
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(cache.path().join("620_library_600x900.jpg"), b"old").unwrap();
        let nested = cache.path().join("1145360").join("abc123");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("library_600x900.jpg"), b"new").unwrap();

        assert_eq!(steam_cached_cover(cache.path(), "620").unwrap().0, b"old");
        assert_eq!(steam_cached_cover(cache.path(), "1145360").unwrap().0, b"new");
        assert!(steam_cached_cover(cache.path(), "999").is_none());
    }

    #[test]
    fn test_games_needing_covers() {
        let db = Connection::open_in_memory().unwrap();
        crate::database::run_migrations(&db).unwrap();
        db.execute_batch(
            "INSERT INTO games (id, name, normalized_name) VALUES (1, 'Hades', 'hades');
             INSERT INTO games (id, name, normalized_name, cover_path) VALUES (2, 'Celeste', 'celeste', 'c.png');
             INSERT INTO games (id, name, normalized_name, is_removed) VALUES (3, 'Old', 'old', 1);
             INSERT INTO game_platforms (game_id, platform, external_id) VALUES (1, 'Steam', '1145360');",
        )
        .unwrap();

        let jobs = games_needing_covers(&db).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].steam_app_id.as_deref(), Some("1145360"));

        db.execute("UPDATE games SET cover_lookup_at = ?1 WHERE id = 1", [Utc::now()])
            .unwrap();
        assert!(games_needing_covers(&db).unwrap().is_empty());
        reset_lookups(&db).unwrap();
        assert_eq!(games_needing_covers(&db).unwrap().len(), 1);
    }
}
