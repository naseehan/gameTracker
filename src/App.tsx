import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Layout } from "./components/Layout";
import { Home } from "./pages/Home";
import { Library } from "./pages/Library";
import { Statistics } from "./pages/Statistics";
import { Onboarding } from "./pages/Onboarding";
import Settings from "./pages/Settings";
import { GameDetails } from "./pages/GameDetails";
import { Toaster } from "./components/Toaster";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { useNavigationStore } from "./store/navigationStore";
import { useAuthStore, supabase } from "./store/authStore";
import { toast } from "./store/toastStore";
import { describeError, reportError } from "./utils/errors";
import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';
import { listen } from '@tauri-apps/api/event';
import { AlertCircle } from "lucide-react";

async function exchangeOAuthCode(code: string) {
  const { error } = await supabase.auth.exchangeCodeForSession(code);
  if (error) {
    reportError('Sign-in failed', error.message);
  } else {
    toast.success('Signed in.');
  }
}

export default function App() {
  const { currentRoute, goBack } = useNavigationStore();
  const [isOnboarded, setIsOnboarded] = useState<boolean | null>(null);
  const [startupError, setStartupError] = useState<string | null>(null);
  const { initialize, initialized } = useAuthStore();

  const loadOnboardingStatus = useCallback(() => {
    setStartupError(null);
    invoke<boolean>('get_onboarding_status')
      .then(setIsOnboarded)
      .catch((err) => setStartupError(describeError(err)));
  }, []);

  useEffect(() => {
    initialize();
    loadOnboardingStatus();

    const handleOAuthCallback = async (urlString: string) => {
      try {
        const parsedUrl = new URL(urlString);
        if (parsedUrl.protocol !== 'gametracker:') return;
        if (!parsedUrl.pathname.includes('login-callback') && !parsedUrl.host.includes('login-callback')) return;

        const errorDesc = parsedUrl.searchParams.get('error_description');
        if (errorDesc) {
          reportError('Sign-in failed', errorDesc);
          return;
        }

        const code = parsedUrl.searchParams.get('code');
        if (code) await exchangeOAuthCode(code);
      } catch (e) {
        reportError('Sign-in failed', e);
      }
    };

    // Primary path: local HTTP server callback (delivers code via Tauri event)
    const unlistenOAuthPromise = listen<{ code: string | null; error: string | null }>(
      'oauth-callback',
      async (event) => {
        const { code, error } = event.payload;
        if (error) {
          reportError('Sign-in failed', error);
          return;
        }
        if (code) await exchangeOAuthCode(code);
      }
    );

    // Fallback: deep link for cold-start (app was closed when link fired)
    getCurrent().then(urls => {
      for (const urlStr of urls ?? []) {
        handleOAuthCallback(urlStr.toString());
      }
    }).catch(() => {});

    const unlistenDeepLinkPromise = onOpenUrl(async (urls) => {
      for (const urlStr of urls) {
        handleOAuthCallback(urlStr.toString());
      }
    });

    const unlistenUpdatePromise = listen<string>('update-available', (event) => {
      toast.info(`GameTracker ${event.payload} is available.`, {
        label: 'Install',
        onClick: () => {
          toast.info('Downloading the update. GameTracker will restart when it is ready.');
          invoke('install_update').catch((e) => reportError("Couldn't install the update", e));
        },
      });
    });

    // The mouse's back button goes back, like in a browser.
    const onMouseUp = (e: MouseEvent) => {
      if (e.button === 3) goBack();
    };
    window.addEventListener('mouseup', onMouseUp);

    return () => {
      unlistenOAuthPromise.then((unlisten) => unlisten());
      unlistenDeepLinkPromise.then((unlisten) => unlisten());
      unlistenUpdatePromise.then((unlisten) => unlisten());
      window.removeEventListener('mouseup', onMouseUp);
    };
  }, []);

  let content;
  if (startupError) {
    content = (
      <div className="h-screen flex items-center justify-center bg-gray-950 text-white p-8">
        <div className="max-w-md text-center">
          <AlertCircle size={40} className="text-red-400 mx-auto mb-4" />
          <h1 className="text-xl font-bold mb-2">GameTracker couldn't start</h1>
          <p className="text-gray-400 text-sm mb-6 break-words">{startupError}</p>
          <div className="flex justify-center gap-3">
            <button onClick={loadOnboardingStatus} className="px-5 py-2.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm font-medium">
              Try again
            </button>
            <button
              onClick={() => invoke('open_log_folder').catch(() => {})}
              className="px-5 py-2.5 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-lg text-sm font-medium"
            >
              Open log folder
            </button>
          </div>
        </div>
      </div>
    );
  } else if (isOnboarded === null || !initialized) {
    content = (
      <div className="h-screen flex items-center justify-center bg-gray-950">
        <div className="animate-spin w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full" aria-label="Loading"></div>
      </div>
    );
  } else if (!isOnboarded) {
    content = <Onboarding onComplete={() => setIsOnboarded(true)} />;
  } else {
    content = (
      <Layout>
        {currentRoute.type === 'home' && <Home />}
        {currentRoute.type === 'library' && <Library />}
        {currentRoute.type === 'stats' && <Statistics />}
        {currentRoute.type === 'settings' && <Settings />}
        {currentRoute.type === 'game_details' && <GameDetails key={currentRoute.gameId} gameId={currentRoute.gameId} />}
      </Layout>
    );
  }

  return (
    <>
      {content}
      <Toaster />
      <ConfirmDialog />
    </>
  );
}
