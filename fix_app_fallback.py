with open('src/App.tsx', 'r', encoding='utf-8') as f:
    content = f.read()

import_target = """import { onOpenUrl } from '@tauri-apps/plugin-deep-link';"""
import_replacement = """import { onOpenUrl } from '@tauri-apps/plugin-deep-link';\nimport { listen } from '@tauri-apps/api/event';"""

content = content.replace(import_target, import_replacement)

target = """    // Listen for Tauri Deep Links
    const unlistenPromise = onOpenUrl(async (urls) => {"""

replacement = """    // Fallback manual listener
    const unlistenFallbackPromise = listen<string>('scheme-request-received', async (event) => {
      alert("Fallback event received: " + event.payload);
      const urlStr = event.payload;
      if (urlStr.includes('login-callback')) {
        try {
          const parsedUrl = new URL(urlStr);
          const code = parsedUrl.searchParams.get('code');
          if (code) {
            await supabase.auth.exchangeCodeForSession(code);
          } else {
            const hash = parsedUrl.hash.substring(1);
            const params = new URLSearchParams(hash);
            const access_token = params.get('access_token');
            const refresh_token = params.get('refresh_token');
            if (access_token && refresh_token) {
              await supabase.auth.setSession({ access_token, refresh_token });
            }
          }
        } catch (e) {
          console.error('Failed to parse deep link URL', e);
        }
      }
    });

    // Listen for Tauri Deep Links
    const unlistenPromise = onOpenUrl(async (urls) => {"""

content = content.replace(target, replacement)

return_target = """      unlistenPromise.then((unlisten) => unlisten());
    };"""
return_replacement = """      unlistenPromise.then((unlisten) => unlisten());
      unlistenFallbackPromise.then((unlisten) => unlisten());
    };"""

content = content.replace(return_target, return_replacement)

with open('src/App.tsx', 'w', encoding='utf-8') as f:
    f.write(content)