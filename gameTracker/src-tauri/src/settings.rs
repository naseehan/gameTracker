use rusqlite::{Connection, OptionalExtension, Result};

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        rusqlite::params![key],
        |r| r.get(0),
    )
    .optional()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

pub fn get_tracking_enabled(conn: &Connection) -> Result<bool> {
    let val = get_setting(conn, "tracking_enabled")?.unwrap_or_else(|| "true".to_string());
    Ok(val == "true")
}

pub fn get_polling_interval(conn: &Connection) -> Result<u64> {
    let val = get_setting(conn, "polling_interval_seconds")?.unwrap_or_else(|| "2".to_string());
    Ok(val.parse::<u64>().unwrap_or(2))
}

pub fn set_game_ignored(conn: &Connection, game_id: i64, is_ignored: bool) -> Result<()> {
    conn.execute(
        "UPDATE games SET is_ignored = ?1 WHERE id = ?2",
        rusqlite::params![is_ignored, game_id],
    )?;
    Ok(())
}
