import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Layout } from "./components/Layout";
import { Library } from "./pages/Library";
import { RecentlyPlayed } from "./pages/RecentlyPlayed";
import { Statistics } from "./pages/Statistics";
import { Onboarding } from "./pages/Onboarding";
import Settings from "./pages/Settings";
import { GameDetails } from "./pages/GameDetails";
import { CurrentlyPlaying } from "./pages/CurrentlyPlaying";
import { useNavigationStore } from "./store/navigationStore";
import { useAuthStore, supabase } from "./store/authStore";
import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';
import { listen } from '@tauri-apps/api/event';


export default function App() {
  const { currentRoute } = useNavigationStore();
  const [isOnboarded, setIsOnboarded] = useState<boolean | null>(null);
  const { initialize, initialized } = useAuthStore();

  useEffect(() => {
    initialize();
    invoke<boolean>('get_onboarding_status')
      .then(status => setIsOnboarded(status))
      .catch(console.error);

    const handleOAuthCallback = async (urlString: string) => {
      console.log('[OAuth] Deep link received');
      try {
        const parsedUrl = new URL(urlString);
        if (parsedUrl.protocol !== 'gametracker:') return;
        if (!parsedUrl.pathname.includes('login-callback') && !parsedUrl.host.includes('login-callback')) return;

        console.log('[OAuth] OAuth callback detected');

        const errorDesc = parsedUrl.searchParams.get('error_description');
        if (errorDesc) {
          console.error('[OAuth] OAuth error returned from provider:', errorDesc);
          return;
        }

        const code = parsedUrl.searchParams.get('code');
        if (code) {
          console.log('[OAuth] Authorization code detected, exchanging');
          const { error } = await supabase.auth.exchangeCodeForSession(code);
          if (error) {
            console.error('[OAuth] Session exchange failed:', error.message);
          } else {
            console.log('[OAuth] Session exchange successful');
          }
        }
      } catch (e: any) {
        console.error('[OAuth] Failed to parse URL:', e.message);
      }
    };

    // Primary path: local HTTP server callback (delivers code via Tauri event)
    const unlistenOAuthPromise = listen<{ code: string | null; error: string | null }>(
      'oauth-callback',
      async (event) => {
        const { code, error } = event.payload;
        if (error) {
          console.error('[OAuth] Error from provider:', error);
          return;
        }
        if (code) {
          console.log('[OAuth] Authorization code received via local server');
          const { error: exchangeError } = await supabase.auth.exchangeCodeForSession(code);
          if (exchangeError) {
            console.error('[OAuth] Session exchange failed:', exchangeError.message);
          } else {
            console.log('[OAuth] Session exchange successful');
          }
        }
      }
    );

    // Fallback: deep link for cold-start (app was closed when link fired)
    getCurrent().then(urls => {
      if (urls && urls.length > 0) {
        for (const urlStr of urls) {
          handleOAuthCallback(urlStr.toString());
        }
      }
    }).catch(e => console.error('Failed to get current deep links:', e));

    const unlistenDeepLinkPromise = onOpenUrl(async (urls) => {
      for (const urlStr of urls) {
        handleOAuthCallback(urlStr.toString());
      }
    });

    return () => {
      unlistenOAuthPromise.then((unlisten) => unlisten());
      unlistenDeepLinkPromise.then((unlisten) => unlisten());
    };
  }, []);

  if (isOnboarded === null || !initialized) {
    return <div className="h-screen flex items-center justify-center bg-gray-950 text-white">Loading...</div>;
  }

  if (!isOnboarded) {
    return <Onboarding onComplete={() => setIsOnboarded(true)} />;
  }

  return (
    <Layout>
      {currentRoute.type === 'library' && <Library />}
      {currentRoute.type === 'recent' && <RecentlyPlayed />}
      
      {currentRoute.type === 'playing' && <CurrentlyPlaying />}
      {currentRoute.type === 'stats' && <Statistics />}
      {currentRoute.type === 'settings' && <Settings />}
      {currentRoute.type === 'game_details' && <GameDetails gameId={currentRoute.gameId} />}
    </Layout>
  );
}
