with open('src-tauri/src/lib.rs', 'r', encoding='utf-8') as f:
    content = f.read()

target = """            use tauri::Manager;
            for arg in argv {"""

replacement = """            use tauri::Manager;
            use tauri::Emitter;
            for arg in argv {"""

content = content.replace(target, replacement)

with open('src-tauri/src/lib.rs', 'w', encoding='utf-8') as f:
    f.write(content)