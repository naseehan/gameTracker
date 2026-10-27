with open('src-tauri/src/lib.rs', 'r', encoding='utf-8') as f:
    content = f.read()

target = """    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())"""

replacement = """    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            // Nothing to do manually here; the deep-link plugin handles it
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())"""

content = content.replace(target, replacement)

with open('src-tauri/src/lib.rs', 'w', encoding='utf-8') as f:
    f.write(content)