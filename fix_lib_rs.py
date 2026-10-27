with open('src-tauri/src/lib.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# I will rewrite the Builder chain.
import re

builder_regex = r"tauri::Builder::default\(\)[\s\S]*?\.setup\(\|app\| \{"
match = re.search(builder_regex, content)
if not match:
    print("Could not find Builder chain!")

new_builder = """tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            // Handled by deep-link feature automatically
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let _ = app.deep_link().register_all();
            }
"""

content = content.replace(match.group(0), new_builder)

with open('src-tauri/src/lib.rs', 'w', encoding='utf-8') as f:
    f.write(content)