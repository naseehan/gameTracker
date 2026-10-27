import re
with open('src-tauri/src/stats/game_queries.rs', 'r') as f:
    c = f.read()

c = re.sub(r'(fn test_get_game_sessions\(\) \{\s*let db = setup_test_db\(\);\s*let now = Utc::now\(\);)',
    r'\1\n        db.execute("INSERT OR IGNORE INTO games (id, name, normalized_name) VALUES (1, \'Test\', \'test\')", []).unwrap();', c)

with open('src-tauri/src/stats/game_queries.rs', 'w') as f:
    f.write(c)