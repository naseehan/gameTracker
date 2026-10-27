import json

with open('src-tauri/tauri.conf.json', 'r', encoding='utf-8') as f:
    config = json.load(f)

if 'plugins' not in config:
    config['plugins'] = {}

config['plugins']['deep-link'] = {
    'desktop': {
        'schemes': ['gametracker']
    }
}

with open('src-tauri/tauri.conf.json', 'w', encoding='utf-8') as f:
    json.dump(config, f, indent=2)