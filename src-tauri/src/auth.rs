use keyring::Entry;
use std::io::{Read, Write};
use std::net::TcpListener;
use tauri::{AppHandle, Emitter, Runtime};

pub fn get_keyring_entry(key: &str) -> Result<Entry, String> {
    Entry::new("GameTracker", key).map_err(|e| format!("Failed to access keychain: {}", e))
}

#[tauri::command]
pub fn store_auth_token(key: String, token: String) -> Result<(), String> {
    let entry = get_keyring_entry(&key)?;
    entry
        .set_password(&token)
        .map_err(|e| format!("Failed to save token securely: {}", e))
}

#[tauri::command]
pub fn get_auth_token(key: String) -> Result<Option<String>, String> {
    let entry = get_keyring_entry(&key)?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to retrieve token securely: {}", e)),
    }
}

#[tauri::command]
pub fn delete_auth_token(key: String) -> Result<(), String> {
    let entry = get_keyring_entry(&key)?;
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete token securely: {}", e)),
    }
}

/// Starts a one-shot local HTTP server on a random available port.
/// When Supabase redirects to http://127.0.0.1:{port}/callback?code=...,
/// this server serves a success page to the browser and emits the code
/// to the Tauri frontend via the "oauth-callback" event.
#[tauri::command]
pub fn start_oauth_listener<R: Runtime>(app: AppHandle<R>) -> Result<u16, String> {
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|e| format!("Failed to bind: {}", e))?;

    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get address: {}", e))?
        .port();

    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();

            // Extract the request path from "GET /callback?code=xxx HTTP/1.1"
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/");

            // Parse query string manually to avoid extra dependency
            let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
            let get_param = |key: &str| -> Option<String> {
                query.split('&').find_map(|pair| {
                    let (k, v) = pair.split_once('=')?;
                    if k == key {
                        Some(v.replace('+', " "))
                    } else {
                        None
                    }
                })
            };

            let code = get_param("code");
            let error_desc = get_param("error_description");

            // Serve a nice success/failure page to the browser
            let html = if code.is_some() {
                r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Login Successful — GameTracker</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
      background: #0f172a;
      color: #f1f5f9;
      display: flex;
      align-items: center;
      justify-content: center;
      height: 100vh;
    }
    .card {
      text-align: center;
      padding: 48px;
      background: #1e293b;
      border: 1px solid #334155;
      border-radius: 16px;
      max-width: 400px;
      width: 90%;
    }
    .icon {
      width: 64px;
      height: 64px;
      background: #22c55e22;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      margin: 0 auto 24px;
      font-size: 32px;
    }
    h1 { font-size: 22px; font-weight: 700; margin-bottom: 12px; }
    p { color: #94a3b8; font-size: 14px; line-height: 1.6; }
  </style>
  <script>
    // Attempt to close the tab. Works if opened by JS (some browsers may block).
    setTimeout(() => { try { window.close(); } catch(e) {} }, 1200);
  </script>
</head>
<body>
  <div class="card">
    <div class="icon">✓</div>
    <h1>Login Successful!</h1>
    <p>You're now signed in to GameTracker.<br>You can close this tab and return to the app.</p>
  </div>
</body>
</html>"#
            } else {
                r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Login Failed — GameTracker</title>
  <style>
    body { font-family: sans-serif; background: #0f172a; color: #f1f5f9;
           display: flex; align-items: center; justify-content: center; height: 100vh; }
    .card { text-align: center; padding: 48px; background: #1e293b;
            border: 1px solid #7f1d1d; border-radius: 16px; max-width: 400px; }
    h1 { color: #f87171; }
    p { color: #94a3b8; margin-top: 12px; font-size: 14px; }
  </style>
</head>
<body>
  <div class="card">
    <h1>Login Failed</h1>
    <p>Something went wrong. Please close this tab and try again in GameTracker.</p>
  </div>
</body>
</html>"#
            };

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(),
                html
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();

            // Emit result to frontend — never log the code value itself
            let payload = serde_json::json!({
                "code": code,
                "error": error_desc
            });
            let _ = app.emit("oauth-callback", payload);
        }
    });

    Ok(port)
}
