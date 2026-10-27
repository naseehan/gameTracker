with open('src/App.tsx', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace('alert("Fallback event received: " + event.payload);\n', '')
content = content.replace('alert("Deep link received! " + urls.join(", "));\n', '')

with open('src/App.tsx', 'w', encoding='utf-8') as f:
    f.write(content)