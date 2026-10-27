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
import { onOpenUrl } from '@tauri-apps/plugin-deep-link';
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
      
    // Fallback manual listener
    const unlistenFallbackPromise = listen<string>('scheme-request-received', async (event) => {
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
            if (access_token) {
              const { error } = await supabase.auth.setSession({ 
                access_token, 
                refresh_token: refresh_token || '' 
              });
              if (error) {
                alert("Session error: " + error.message);
              } else {
                // Force a reload or UI update if needed, though Zustand should handle it
              }
            } else {
              alert("No access token found in URL!");
            }
          }
        } catch (e) {
          console.error('Failed to parse deep link URL', e);
        }
      }
    });

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
      unlistenFallbackPromise.then((unlisten) => unlisten());
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
