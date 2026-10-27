with open('src/components/AuthForms.tsx', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace("import { open } from '@tauri-apps/plugin-opener';", "import { openUrl } from '@tauri-apps/plugin-opener';")
content = content.replace("await open(data.url);", "await openUrl(data.url);")

with open('src/components/AuthForms.tsx', 'w', encoding='utf-8') as f:
    f.write(content)