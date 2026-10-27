with open('src/App.tsx', 'r', encoding='utf-8') as f:
    content = f.read()

target_import = """import { useNavigationStore } from "./store/navigationStore";
import { useAuthStore } from "./store/authStore";"""

replacement_import = """import { useNavigationStore } from "./store/navigationStore";
import { useAuthStore, supabase } from "./store/authStore";
import { onOpenUrl } from '@tauri-apps/plugin-deep-link';"""

content = content.replace(target_import, replacement_import)

target_hook = """  useEffect(() => {
    initialize();
    invoke<boolean>('get_onboarding_status')
      .then(status => setIsOnboarded(status))
      .catch(console.error);
  }, []);"""

replacement_hook = """  useEffect(() => {
    initialize();
    invoke<boolean>('get_onboarding_status')
      .then(status => setIsOnboarded(status))
      .catch(console.error);
      
    // Listen for Tauri Deep Links
    const unlistenPromise = onOpenUrl(async (urls) => {
      for (const urlStr of urls) {
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
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);"""

content = content.replace(target_hook, replacement_hook)

with open('src/App.tsx', 'w', encoding='utf-8') as f:
    f.write(content)