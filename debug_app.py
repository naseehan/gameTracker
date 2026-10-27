with open('src/App.tsx', 'r', encoding='utf-8') as f:
    content = f.read()

target = """    // Listen for Tauri Deep Links
    const unlistenPromise = onOpenUrl(async (urls) => {
      for (const urlStr of urls) {
        if (urlStr.includes('login-callback')) {"""

replacement = """    // Listen for Tauri Deep Links
    const unlistenPromise = onOpenUrl(async (urls) => {
      alert("Deep link received! " + urls.join(", "));
      for (const urlStr of urls) {
        if (urlStr.includes('login-callback')) {"""

content = content.replace(target, replacement)

with open('src/App.tsx', 'w', encoding='utf-8') as f:
    f.write(content)