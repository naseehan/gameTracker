with open('src-tauri/src/lib.rs', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace("        .plugin(tauri_plugin_fs::init())\n", "")

with open('src-tauri/src/lib.rs', 'w', encoding='utf-8') as f:
    f.write(content)