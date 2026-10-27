import re

with open('src/App.tsx', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace the imports
content = content.replace("import { onOpenUrl } from '@tauri-apps/plugin-deep-link';", "import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';")
content = content.replace("import { listen } from '@tauri-apps/api/event';", "")

# Define the new useEffect body
new_effect = """  useEffect(() => {
    initialize();
    invoke<boolean>('get_onboarding_status')
      .then(status => setIsOnboarded(status))
      .catch(console.error);

    const handleOAuthCallback = async (urlString: string) => {
      console.log('[OAuth] Deep link received');
      try {
        const parsedUrl = new URL(urlString);
        
        // 1. Validate gametracker scheme
        if (parsedUrl.protocol !== 'gametracker:') return;
        
        // 2. Validate login-callback host/path
        if (!parsedUrl.pathname.includes('login-callback') && !parsedUrl.host.includes('login-callback')) return;
        
        console.log('[OAuth] OAuth callback detected');
        
        // 3. Check for OAuth errors
        const errorDesc = parsedUrl.searchParams.get('error_description');
        if (errorDesc) {
          console.error('[OAuth] OAuth error returned from provider:', errorDesc);
          return;
        }

        // 4. Check for code (PKCE flow)
        const code = parsedUrl.searchParams.get('code');
        if (code) {
          console.log('[OAuth] Authorization code detected');
          console.log('[OAuth] Exchanging authorization code');
          
          const { error } = await supabase.auth.exchangeCodeForSession(code);
          if (error) {
            console.error('[OAuth] Session exchange failed:', error.message);
          } else {
            console.log('[OAuth] Session exchange successful');
            // Supabase auth state change listener will automatically handle the rest
          }
        }
      } catch (e: any) {
        console.error('[OAuth] Failed to parse URL:', e.message);
      }
    };

    // Handle cold start (GameTracker was closed)
    getCurrent().then(urls => {
      if (urls && urls.length > 0) {
        for (const urlStr of urls) {
          handleOAuthCallback(urlStr);
        }
      }
    }).catch(e => console.error('Failed to get current deep links:', e));

    // Listen for Tauri Deep Links (GameTracker is already running)
    const unlistenPromise = onOpenUrl(async (urls) => {
      for (const urlStr of urls) {
        handleOAuthCallback(urlStr);
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);"""

# Extract everything between `useEffect(() => {` and `  }, []);`
effect_regex = r"  useEffect\(\(\) => \{[\s\S]*?  \}, \[\]\);"
content = re.sub(effect_regex, new_effect, content)

with open('src/App.tsx', 'w', encoding='utf-8') as f:
    f.write(content)