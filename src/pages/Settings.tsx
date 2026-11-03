import { ReactNode, useEffect, useState } from 'react';
import { useSettingsStore } from '../store/settingsStore';
import {
  Shield, Database, Activity, Save, Download, Trash2, Power, Search, Clock, User, Check,
  Image, Undo2, LifeBuoy, ExternalLink, RefreshCw,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { Dropdown } from '../components/Dropdown';
import { Toggle } from '../components/Toggle';
import { useAuthStore } from '../store/authStore';
import { AuthForms } from '../components/AuthForms';
import { confirmAction } from '../store/confirmStore';
import { toast } from '../store/toastStore';
import { reportError } from '../utils/errors';
import { formatPlaytime } from '../utils/formatters';
import { useBackendEvent } from '../hooks/useBackendEvent';
import { RemovedGame, UpdateStatus } from '../types';

function Section({ icon, title, children }: { icon: ReactNode; title: string; children: ReactNode }) {
  return (
    <section>
      <div className="flex items-center space-x-3 mb-6">
        {icon}
        <h2 className="text-xl font-semibold text-white tracking-tight">{title}</h2>
      </div>
      <div className="bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden shadow-sm divide-y divide-gray-800/50">
        {children}
      </div>
    </section>
  );
}

function Row({ title, description, children }: { title: string; description?: ReactNode; children?: ReactNode }) {
  return (
    <div className="p-6 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
      <div className="pr-8">
        <h3 className="text-white font-medium text-base">{title}</h3>
        {description && <div className="text-sm text-gray-400 mt-1">{description}</div>}
      </div>
      {children && <div className="flex-shrink-0">{children}</div>}
    </div>
  );
}

const buttonClass =
  'flex items-center justify-center gap-2 px-5 py-2.5 bg-gray-800 hover:bg-gray-700 text-white rounded-lg border border-gray-700 transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-500 text-sm font-medium disabled:opacity-50 min-w-[140px]';
const dangerButtonClass =
  'flex items-center justify-center gap-2 px-5 py-2.5 bg-red-900/30 hover:bg-red-900/50 text-red-300 rounded-lg border border-red-900/50 transition-colors focus:outline-none focus:ring-2 focus:ring-red-500 text-sm font-medium disabled:opacity-50 min-w-[160px]';

export default function Settings() {
  const { trackingEnabled, pollingIntervalSeconds, autoDiscoveryEnabled, autoDiscoveryInterval, discoverySteam, discoveryEpic, discoveryGog, fetchSettings, updateSetting } = useSettingsStore();
  const { session, user, signOut } = useAuthStore();
  const [showAuthModal, setShowAuthModal] = useState<'login' | 'signup' | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [autostartEnabled, setAutostartEnabled] = useState(false);

  useEffect(() => {
    fetchSettings();
    invoke<boolean>('get_autostart_enabled')
      .then(setAutostartEnabled)
      .catch((err) => reportError("Couldn't read the startup setting", err));
  }, [fetchSettings]);

  // Auto-close the auth modal when OAuth login completes via deep link
  useEffect(() => {
    if (session && showAuthModal) {
      setShowAuthModal(null);
    }
  }, [session, showAuthModal]);

  const change = (key: string, value: string) =>
    updateSetting(key, value).catch((err) => reportError("Couldn't save the setting", err));

  const handleAutostartChange = async (enabled: boolean) => {
    try {
      await invoke('set_autostart_enabled', { enabled });
      setAutostartEnabled(enabled);
    } catch (err) {
      reportError("Couldn't change the startup setting", err);
    }
  };

  /** Runs a data action with a busy state and reports failure. */
  const run = async (name: string, action: () => Promise<void>, failure: string) => {
    setBusy(name);
    try {
      await action();
    } catch (err) {
      reportError(failure, err);
    } finally {
      setBusy(null);
    }
  };

  const handleExport = () =>
    run('export', async () => {
      if (await invoke<boolean>('export_data')) toast.success('Data exported.');
    }, "Couldn't export your data");

  const handleImport = async () => {
    const confirmed = await confirmAction({
      title: 'Import data?',
      message: 'Everything in GameTracker now (games, play history and settings) is replaced by the file you choose.',
      confirmLabel: 'Choose file',
      danger: true,
    });
    if (!confirmed) return;
    await run('import', async () => {
      if (await invoke<boolean>('import_data')) {
        toast.success('Data imported.');
        fetchSettings();
      }
    }, "Couldn't import the file");
  };

  const handleDeleteHistory = async () => {
    const confirmed = await confirmAction({
      title: 'Delete all play history?',
      message: 'Every recorded session and all playtime are permanently removed. Your games stay in the library.',
      confirmLabel: 'Delete history',
      danger: true,
    });
    if (!confirmed) return;
    await run('delete', async () => {
      await invoke('delete_all_history');
      toast.success('Play history deleted.');
    }, "Couldn't delete play history");
  };

  const handleReset = async () => {
    const confirmed = await confirmAction({
      title: 'Reset GameTracker?',
      message: 'This permanently deletes your games, play history, covers and settings. It cannot be undone unless you exported your data first.',
      confirmLabel: 'Reset everything',
      danger: true,
    });
    if (!confirmed) return;
    await run('reset', async () => {
      await invoke('reset_application');
      window.location.reload();
    }, "Couldn't reset GameTracker");
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-gray-950 overflow-y-auto">
      <header className="px-8 py-10 border-b border-gray-800/50 bg-gray-900/30 flex-shrink-0">
        <div className="max-w-4xl mx-auto">
          <h1 className="text-3xl font-bold text-white mb-3 tracking-tight">Settings</h1>
          <p className="text-gray-400 text-lg">Manage tracking, your library and your data.</p>
        </div>
      </header>

      <div className="flex-1 p-8">
        <div className="max-w-4xl mx-auto space-y-12 pb-8">

          <Section icon={<Activity className="text-indigo-400" size={24} />} title="Tracking">
            <Row title="Automatic game tracking" description="Detect and track playtime when games run.">
              <Toggle checked={trackingEnabled} onChange={(checked) => change('tracking_enabled', checked.toString())} ariaLabel="Toggle automatic game tracking" />
            </Row>
            <Row
              title="Start with Windows"
              description="Open in the system tray when you sign in, so games are tracked without opening the app. Closing the window also keeps tracking in the tray; quit from the tray icon to stop."
            >
              <Toggle checked={autostartEnabled} onChange={handleAutostartChange} ariaLabel="Toggle starting with Windows" />
            </Row>
            <Row title="Detection interval" description="How often running programs are checked. Shorter catches games sooner but uses a little more CPU.">
              <div className="w-48">
                <Dropdown
                  value={pollingIntervalSeconds.toString()}
                  onChange={(v) => change('polling_interval_seconds', v)}
                  icon={<Clock size={16} className="text-gray-500" />}
                  options={[
                    { value: '2', label: '2 seconds (Default)' },
                    { value: '5', label: '5 seconds' },
                    { value: '10', label: '10 seconds' },
                  ]}
                />
              </div>
            </Row>
          </Section>

          <Section icon={<Search className="text-green-400" size={24} />} title="Game Discovery">
            <Row title="Automatic discovery" description="Regularly check your game launchers for newly installed games.">
              <Toggle checked={autoDiscoveryEnabled} onChange={(checked) => change('auto_discovery_enabled', checked.toString())} ariaLabel="Toggle automatic game discovery" />
            </Row>
            <div className="p-6">
              <h4 className="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-4">Launchers</h4>
              <div className="space-y-4">
                {[
                  { label: 'Steam', key: 'discovery_provider_steam', value: discoverySteam },
                  { label: 'Epic Games', key: 'discovery_provider_epic_games', value: discoveryEpic },
                  { label: 'GOG', key: 'discovery_provider_gog', value: discoveryGog },
                ].map((provider) => (
                  <div key={provider.key} className="flex items-center justify-between">
                    <span className="text-gray-300 font-medium">{provider.label}</span>
                    <Toggle checked={provider.value} onChange={(checked) => change(provider.key, checked.toString())} ariaLabel={`Toggle ${provider.label} discovery`} />
                  </div>
                ))}
              </div>
            </div>
            <Row title="Discovery interval" description="How often your launchers are checked for new installations.">
              <div className="w-48">
                <Dropdown
                  value={autoDiscoveryInterval.toString()}
                  onChange={(v) => change('auto_discovery_interval', v)}
                  icon={<Clock size={16} className="text-gray-500" />}
                  options={[
                    { value: '600', label: '10 minutes' },
                    { value: '1800', label: '30 minutes (Default)' },
                    { value: '3600', label: '1 hour' },
                    { value: '7200', label: '2 hours' },
                  ]}
                />
              </div>
            </Row>
          </Section>

          <Section icon={<Image className="text-purple-400" size={24} />} title="Cover Art">
            <CoverArtSettings />
          </Section>

          <Section icon={<Undo2 className="text-amber-400" size={24} />} title="Removed Games">
            <RemovedGames />
          </Section>

          <Section icon={<User className="text-blue-400" size={24} />} title="Account">
            <div className="p-6">
              {session ? (
                <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                  <div className="flex items-center space-x-4">
                    <div className="w-12 h-12 bg-indigo-900 text-indigo-200 rounded-full flex items-center justify-center font-bold text-lg border border-indigo-500/30">
                      {user?.email?.charAt(0).toUpperCase() || 'U'}
                    </div>
                    <div>
                      <h3 className="text-white font-medium text-base">{user?.email}</h3>
                      <p className="text-sm text-green-400 mt-1 flex items-center">
                        <Check size={14} className="mr-1" /> Signed in
                      </p>
                    </div>
                  </div>
                  <button onClick={() => signOut()} className={buttonClass}>Sign out</button>
                </div>
              ) : (
                <div className="flex flex-col space-y-4">
                  <p className="text-gray-300 text-sm">
                    An account isn't needed to use GameTracker. It's for future features such as syncing between PCs.
                  </p>
                  <div className="flex flex-wrap gap-3">
                    <button onClick={() => setShowAuthModal('signup')} className="px-5 py-2.5 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg transition-colors font-medium text-sm">
                      Create account
                    </button>
                    <button onClick={() => setShowAuthModal('login')} className={buttonClass}>Sign in</button>
                  </div>
                </div>
              )}
            </div>
          </Section>

          <Section icon={<Shield className="text-indigo-400" size={24} />} title="Privacy">
            <div className="p-6 space-y-3 text-sm text-gray-400">
              <p className="text-gray-300 font-medium">Your play history stays on this PC.</p>
              <p>
                GameTracker reads the names of running programs to recognise games. It stores your library and sessions in a
                database on this PC and never uploads them. Signing in doesn't upload anything either.
              </p>
              <p>
                To find cover art it contacts Steam's image servers, and SteamGridDB if you add a key, sending only the
                game's name or Steam ID.
              </p>
            </div>
          </Section>

          <Section icon={<Database className="text-blue-400" size={24} />} title="Data">
            <Row title="Export data" description="Save your library, play history and settings to a file.">
              <button onClick={handleExport} disabled={busy !== null} className={buttonClass}>
                <Download size={18} /> {busy === 'export' ? 'Exporting...' : 'Export'}
              </button>
            </Row>
            <Row title="Import data" description="Restore from an exported file. This replaces everything in GameTracker now.">
              <button onClick={handleImport} disabled={busy !== null} className={buttonClass}>
                <Save size={18} /> {busy === 'import' ? 'Importing...' : 'Import'}
              </button>
            </Row>
            <div className="bg-red-950/20">
              <Row title="Delete play history" description="Remove every recorded session. Games stay in your library.">
                <button onClick={handleDeleteHistory} disabled={busy !== null} className={dangerButtonClass}>
                  <Trash2 size={18} /> {busy === 'delete' ? 'Deleting...' : 'Delete history'}
                </button>
              </Row>
              <Row title="Reset GameTracker" description="Permanently delete your games, history, covers and settings.">
                <button onClick={handleReset} disabled={busy !== null} className="flex items-center justify-center gap-2 px-5 py-2.5 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-colors text-sm font-medium disabled:opacity-50 min-w-[160px]">
                  <Power size={18} /> {busy === 'reset' ? 'Resetting...' : 'Reset'}
                </button>
              </Row>
            </div>
          </Section>

          <Section icon={<LifeBuoy className="text-gray-400" size={24} />} title="Help & About">
            <AboutSettings />
          </Section>
        </div>
      </div>

      {showAuthModal && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-gray-950/80 backdrop-blur-sm"
          onKeyDown={(e) => e.key === 'Escape' && setShowAuthModal(null)}
        >
          <AuthForms
            initialMode={showAuthModal}
            onSuccess={() => setShowAuthModal(null)}
            onCancel={() => setShowAuthModal(null)}
          />
        </div>
      )}
    </div>
  );
}

function CoverArtSettings() {
  const [hasKey, setHasKey] = useState(false);
  const [key, setKey] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    invoke<boolean>('has_steamgriddb_key').then(setHasKey).catch(() => {});
  }, []);

  const save = async (value: string) => {
    setSaving(true);
    try {
      await invoke('set_steamgriddb_key', { key: value });
      setHasKey(value.trim() !== '');
      setKey('');
      toast.success(value.trim() ? 'Key saved. Looking for covers now.' : 'Key removed.');
    } catch (err) {
      reportError("Couldn't save the key", err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="p-6 space-y-4">
      <p className="text-sm text-gray-400">
        Covers for Steam games are found automatically. For Epic, GOG and games you added yourself, add a free{' '}
        <button
          onClick={() => openUrl('https://www.steamgriddb.com/profile/preferences/api').catch(() => {})}
          className="text-indigo-400 hover:text-indigo-300 inline-flex items-center gap-1"
        >
          SteamGridDB API key <ExternalLink size={12} />
        </button>
        . You can always pick your own cover from a game's page.
      </p>
      {hasKey ? (
        <div className="flex items-center justify-between gap-4">
          <span className="text-sm text-green-400 flex items-center gap-2"><Check size={16} /> SteamGridDB key saved</span>
          <button onClick={() => save('')} disabled={saving} className={buttonClass}>Remove key</button>
        </div>
      ) : (
        <form
          className="flex gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            if (key.trim()) save(key);
          }}
        >
          <input
            type="password"
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder="Paste your SteamGridDB API key"
            aria-label="SteamGridDB API key"
            autoComplete="off"
            className="flex-1 bg-gray-950 border border-gray-700 text-white rounded-lg px-4 py-2.5 focus:ring-2 focus:ring-indigo-500 focus:outline-none text-sm"
          />
          <button type="submit" disabled={saving || !key.trim()} className={buttonClass}>Save key</button>
        </form>
      )}
    </div>
  );
}

function RemovedGames() {
  const [games, setGames] = useState<RemovedGame[] | null>(null);

  const load = () => {
    invoke<RemovedGame[]>('get_removed_games')
      .then(setGames)
      .catch((err) => reportError("Couldn't load removed games", err));
  };
  useEffect(load, []);
  useBackendEvent(['library-changed'], load);

  const restore = async (game: RemovedGame) => {
    try {
      await invoke('restore_game', { gameId: game.id });
      toast.success(`${game.name} is back in your library.`);
    } catch (err) {
      reportError(`Couldn't restore ${game.name}`, err);
    }
  };

  if (!games) return null;
  if (games.length === 0) {
    return <p className="p-6 text-sm text-gray-500">No removed games. Games you remove keep their history and can be restored here.</p>;
  }
  return (
    <ul>
      {games.map((game) => (
        <li key={game.id} className="px-6 py-4 flex items-center justify-between gap-4">
          <div>
            <p className="text-white font-medium">{game.name}</p>
            <p className="text-xs text-gray-500">{formatPlaytime(game.total_playtime_seconds)} played</p>
          </div>
          <button onClick={() => restore(game)} className={buttonClass}>
            <Undo2 size={16} /> Restore
          </button>
        </li>
      ))}
    </ul>
  );
}

function AboutSettings() {
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);

  const check = async () => {
    setChecking(true);
    try {
      setStatus(await invoke<UpdateStatus>('get_update_status'));
    } catch (err) {
      reportError("Couldn't check for updates", err);
    } finally {
      setChecking(false);
    }
  };
  useEffect(() => {
    check();
  }, []);

  const install = async () => {
    setInstalling(true);
    try {
      await invoke('install_update');
    } catch (err) {
      reportError("Couldn't install the update", err);
      setInstalling(false);
    }
  };

  let updateText = 'Checking for updates...';
  if (status && !status.configured) updateText = 'Automatic updates are not set up for this build.';
  else if (status?.available_version) updateText = `Version ${status.available_version} is available.`;
  else if (status) updateText = "You're on the latest version.";

  return (
    <>
      <Row title={`GameTracker ${status?.current_version ?? ''}`} description={updateText}>
        {status?.available_version ? (
          <button onClick={install} disabled={installing} className="px-5 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg text-sm font-medium disabled:opacity-50">
            {installing ? 'Installing...' : 'Install and restart'}
          </button>
        ) : (
          status?.configured && (
            <button onClick={check} disabled={checking} className={buttonClass}>
              <RefreshCw size={16} /> {checking ? 'Checking...' : 'Check again'}
            </button>
          )
        )}
      </Row>
      <Row title="Log files" description="If something goes wrong, these files help find out why. Nothing is sent anywhere.">
        <button onClick={() => invoke('open_log_folder').catch((err) => reportError("Couldn't open the log folder", err))} className={buttonClass}>
          Open log folder
        </button>
      </Row>
    </>
  );
}
