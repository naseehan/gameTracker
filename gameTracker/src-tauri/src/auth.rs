use keyring::Entry;

pub fn get_keyring_entry() -> Result<Entry, String> {
    Entry::new("GameTracker", "auth_token").map_err(|e| format!("Failed to access keychain: {}", e))
}

#[tauri::command]
pub fn store_auth_token(token: String) -> Result<(), String> {
    let entry = get_keyring_entry()?;
    entry
        .set_password(&token)
        .map_err(|e| format!("Failed to save token securely: {}", e))
}

#[tauri::command]
pub fn get_auth_token() -> Result<Option<String>, String> {
    let entry = get_keyring_entry()?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to retrieve token securely: {}", e)),
    }
}

#[tauri::command]
pub fn delete_auth_token() -> Result<(), String> {
    let entry = get_keyring_entry()?;
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete token securely: {}", e)),
    }
}
