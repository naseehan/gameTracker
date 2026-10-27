import { useEffect, useState } from 'react';
import { useSettingsStore } from '../store/settingsStore';
import { Shield, Database, Activity, Save, Download, Trash2, Power, Search, Clock, User, Check } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { save, open } from '@tauri-apps/plugin-dialog';
import { useLibraryStore } from '../store/libraryStore';
import { Dropdown } from '../components/Dropdown';
import { Toggle } from '../components/Toggle';
import { useAuthStore } from '../store/authStore';
import { AuthForms } from '../components/AuthForms';

export default function Settings() {
  const { trackingEnabled, pollingIntervalSeconds, autoDiscoveryEnabled, autoDiscoveryInterval, discoverySteam, discoveryEpic, discoveryGog, fetchSettings, updateSetting } = useSettingsStore();
  const fetchGames = useLibraryStore(state => state.fetchGames);
  const { session, user, signOut } = useAuthStore();
  const [showAuthModal, setShowAuthModal] = useState<'login' | 'signup' | null>(null);
  
  const [isExporting, setIsExporting] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [isResetting, setIsResetting] = useState(false);

  useEffect(() => {
    fetchSettings();
  }, [fetchSettings]);

  // Auto-close the auth modal when OAuth login completes via deep link
  useEffect(() => {
    if (session && showAuthModal) {
      setShowAuthModal(null);
    }
  }, [session, showAuthModal]);

  const handleExport = async () => {
    try {
      setIsExporting(true);
      const filePath = await save({
        filters: [{ name: 'JSON', extensions: ['json'] }],
        defaultPath: 'gametracker_export.json'
      });
      if (filePath) {
        await invoke('export_data', { path: filePath });
        alert('Data exported successfully!');
      }
    } catch (err) {
      console.error(err);
      alert('Failed to export data: ' + String(err));
    } finally {
      setIsExporting(false);
    }
  };

  const handleImport = async () => {
    if (!confirm('Are you sure you want to import data? This will overwrite your current database.')) return;
    
    try {
      setIsImporting(true);
      const filePath = await open({
        filters: [{ name: 'JSON', extensions: ['json'] }],
        multiple: false
      });
      if (filePath) {
        await invoke('import_data', { path: filePath });
        alert('Data imported successfully!');
        fetchSettings();
        fetchGames();
      }
    } catch (err) {
      console.error(err);
      alert('Failed to import data: ' + String(err));
    } finally {
      setIsImporting(false);
    }
  };

  const handleDeleteHistory = async () => {
    if (!confirm('Delete all play history?\n\nThis will permanently remove all recorded gaming sessions and playtime. Games will remain in your library.')) return;
    
    try {
      setIsDeleting(true);
      await invoke('delete_all_history');
      alert('Play history deleted.');
      fetchGames();
    } catch (err) {
      console.error(err);
      alert('Failed to delete history: ' + String(err));
    } finally {
      setIsDeleting(false);
    }
  };

  const handleReset = async () => {
    if (!confirm('Reset Application?\n\nThis permanently deletes your games, playtime history, settings, and tracking data.\n\nThis cannot be undone unless you have an exported backup.')) return;
    
    try {
      setIsResetting(true);
      await invoke('reset_application');
      alert('Application reset. Please restart the app.');
      window.location.reload();
    } catch (err) {
      console.error(err);
      alert('Failed to reset application: ' + String(err));
    } finally {
      setIsResetting(false);
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-gray-950 overflow-y-auto">
      <header className="px-8 py-10 border-b border-gray-800/50 bg-gray-900/30 flex-shrink-0">
        <div className="max-w-4xl mx-auto">
          <h1 className="text-3xl font-bold text-white mb-3 tracking-tight">Settings</h1>
          <p className="text-gray-400 text-lg">Manage your preferences, privacy, and data.</p>
        </div>
      </header>

      <div className="flex-1 p-8">
        <div className="max-w-4xl mx-auto space-y-12">
          
          {/* ACCOUNT SECTION */}
          <section>
            <div className="flex items-center space-x-3 mb-6">
              <User className="text-blue-400" size={24} />
              <h2 className="text-xl font-semibold text-white tracking-tight">Account</h2>
            </div>
            
            <div className="bg-gray-900 border border-gray-800 rounded-2xl p-6 shadow-sm">
              {session ? (
                <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                  <div className="flex items-center space-x-4">
                    <div className="w-12 h-12 bg-indigo-900 text-indigo-200 rounded-full flex items-center justify-center font-bold text-lg border border-indigo-500/30">
                      {user?.email?.charAt(0).toUpperCase() || 'U'}
                    </div>
                    <div>
                      <h3 className="text-white font-medium text-base">{user?.email}</h3>
                      <p className="text-sm text-green-400 mt-1 flex items-center">
                        <Check size={14} className="mr-1" /> Account connected
                      </p>
                    </div>
                  </div>
                  <button 
                    onClick={() => signOut()}
                    className="px-5 py-2.5 bg-gray-800 hover:bg-gray-700 text-white rounded-lg border border-gray-700 transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  >
                    Log Out
                  </button>
                </div>
              ) : (
                <div className="flex flex-col space-y-4">
                  <div className="mb-2">
                    <p className="text-gray-300 text-sm">
                      Create a GameTracker account to enable future cloud and connected-platform features.
                    </p>
                    <p className="text-gray-500 text-xs mt-2">
                      You can continue using GameTracker locally without an account.
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-3">
                    <button 
                      onClick={() => setShowAuthModal('signup')}
                      className="px-5 py-2.5 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-500 font-medium text-sm"
                    >
                      Create Account
                    </button>
                    <button 
                      onClick={() => setShowAuthModal('login')}
                      className="px-5 py-2.5 bg-gray-800 hover:bg-gray-700 text-white rounded-lg border border-gray-700 transition-colors focus:outline-none focus:ring-2 focus:ring-gray-500 font-medium text-sm"
                    >
                      Sign In
                    </button>
                  </div>
                </div>
              )}
            </div>
          </section>

          {/* TRACKING SECTION */}
          <section>
            <div className="flex items-center space-x-3 mb-6">
              <Activity className="text-indigo-400" size={24} />
              <h2 className="text-xl font-semibold text-white tracking-tight">Tracking</h2>
            </div>
            
            <div className="bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden shadow-sm">
              <div className="p-6 flex items-center justify-between border-b border-gray-800/50">
                <div className="pr-8">
                  <h3 className="text-white font-medium text-base">Automatic Game Tracking</h3>
                  <p className="text-sm text-gray-400 mt-1">Detect and track playtime when games run.</p>
                </div>
                <Toggle 
                  checked={trackingEnabled}
                  onChange={(checked) => updateSetting('tracking_enabled', checked.toString())}
                  ariaLabel="Toggle automatic game tracking"
                />
              </div>

              <div className="p-6 flex items-center justify-between">
                <div className="pr-8">
                  <h3 className="text-white font-medium text-base">Detection Interval</h3>
                  <p className="text-sm text-gray-400 mt-1">How often running processes are checked.</p>
                </div>
                <div className="w-48 flex-shrink-0">
                  <Dropdown
                    value={pollingIntervalSeconds.toString()}
                    onChange={(v) => updateSetting('polling_interval_seconds', v)}
                    icon={<Clock size={16} className="text-gray-500" />}
                    options={[
                      { value: '1', label: '1 second' },
                      { value: '2', label: '2 seconds (Default)' },
                      { value: '3', label: '3 seconds' },
                      { value: '5', label: '5 seconds' },
                      { value: '10', label: '10 seconds' },
                    ]}
                  />
                </div>
              </div>
            </div>
          </section>

          {/* GAME DISCOVERY SECTION */}
          <section>
            <div className="flex items-center space-x-3 mb-6">
              <Search className="text-green-400" size={24} />
              <h2 className="text-xl font-semibold text-white tracking-tight">Game Discovery</h2>
            </div>
            
            <div className="bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden shadow-sm">
              <div className="p-6 flex items-center justify-between border-b border-gray-800/50">
                <div className="pr-8">
                  <h3 className="text-white font-medium text-base">Automatic Discovery</h3>
                  <p className="text-sm text-gray-400 mt-1">Periodically check supported game platforms for newly installed games.</p>
                </div>
                <Toggle 
                  checked={autoDiscoveryEnabled}
                  onChange={(checked) => updateSetting('auto_discovery_enabled', checked.toString())}
                  ariaLabel="Toggle automatic game discovery"
                />
              </div>

              <div className="p-6 border-b border-gray-800/50">
                <h4 className="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-4">Platform Providers</h4>
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <span className="text-gray-300 font-medium">Steam</span>
                    <Toggle 
                      checked={discoverySteam}
                      onChange={(checked) => updateSetting('discovery_provider_steam', checked.toString())}
                      ariaLabel="Toggle Steam discovery"
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-gray-300 font-medium">Epic Games</span>
                    <Toggle 
                      checked={discoveryEpic}
                      onChange={(checked) => updateSetting('discovery_provider_epic_games', checked.toString())}
                      ariaLabel="Toggle Epic Games discovery"
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-gray-300 font-medium">GOG</span>
                    <Toggle 
                      checked={discoveryGog}
                      onChange={(checked) => updateSetting('discovery_provider_gog', checked.toString())}
                      ariaLabel="Toggle GOG discovery"
                    />
                  </div>
                </div>
              </div>

              <div className="p-6 flex items-center justify-between">
                <div className="pr-8">
                  <h3 className="text-white font-medium text-base">Discovery Interval</h3>
                  <p className="text-sm text-gray-400 mt-1">How often supported game platforms are checked for new installations.</p>
                </div>
                <div className="w-48 flex-shrink-0">
                  <Dropdown
                    value={autoDiscoveryInterval.toString()}
                    onChange={(v) => updateSetting('auto_discovery_interval', v)}
                    icon={<Clock size={16} className="text-gray-500" />}
                    options={[
                      { value: '600', label: '10 minutes' },
                      { value: '1800', label: '30 minutes (Default)' },
                      { value: '3600', label: '1 hour' },
                      { value: '7200', label: '2 hours' },
                    ]}
                  />
                </div>
              </div>
            </div>
          </section>

          {/* PRIVACY SECTION */}
          <section>
            <div className="flex items-center space-x-3 mb-6">
              <Shield className="text-indigo-400" size={24} />
              <h2 className="text-xl font-semibold text-white tracking-tight">Privacy</h2>
            </div>
            
            <div className="bg-gray-900/50 border border-gray-800 rounded-2xl p-6">
              <p className="text-gray-300 font-medium mb-4">
                GameTracker is designed with a strictly <span className="text-indigo-400">local-first</span> approach.
              </p>
              <h4 className="text-gray-200 font-semibold mb-2 text-sm">WITHOUT AN ACCOUNT:</h4>
              <ul className="space-y-3 text-sm text-gray-400 mb-6">
                <li className="flex items-start"><span className="mr-2 text-gray-600">•</span><span>GameTracker remains local-first.</span></li>
                <li className="flex items-start"><span className="mr-2 text-gray-600">•</span><span>Game history remains on the device.</span></li>
                <li className="flex items-start"><span className="mr-2 text-gray-600">•</span><span>No account is required for tracking.</span></li>
              </ul>
              <h4 className="text-gray-200 font-semibold mb-2 text-sm">WITH AN ACCOUNT:</h4>
              <ul className="space-y-3 text-sm text-gray-400">
                <li className="flex items-start"><span className="mr-2 text-gray-600">•</span><span>The account is used for authentication and future connected features.</span></li>
                <li className="flex items-start"><span className="mr-2 text-gray-600">•</span><span>M15 does not automatically upload existing game history.</span></li>
                <li className="flex items-start"><span className="mr-2 text-gray-600">•</span><span>Future synchronization/features will require explicit implementation and user control.</span></li>
              </ul>
            </div>
          </section>

          {/* DATA MANAGEMENT SECTION */}
          <section>
            <div className="flex items-center space-x-3 mb-6">
              <Database className="text-blue-400" size={24} />
              <h2 className="text-xl font-semibold text-white tracking-tight">Data Management</h2>
            </div>

            <div className="bg-gray-900 border border-gray-800 rounded-2xl overflow-hidden shadow-sm">
              <div className="p-6 border-b border-gray-800/50 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                <div>
                  <h3 className="text-white font-medium text-base">Export Data</h3>
                  <p className="text-sm text-gray-400 mt-1">Save your game history and settings to a JSON file.</p>
                </div>
                <button 
                  onClick={handleExport}
                  disabled={isExporting}
                  className="flex items-center justify-center space-x-2 px-5 py-2.5 bg-gray-800 hover:bg-gray-700 text-white rounded-lg border border-gray-700 transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-500 min-w-[140px]"
                >
                  <Download size={18} />
                  <span>{isExporting ? 'Exporting...' : 'Export'}</span>
                </button>
              </div>

              <div className="p-6 border-b border-gray-800/50 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                <div>
                  <h3 className="text-white font-medium text-base">Import Data</h3>
                  <p className="text-sm text-gray-400 mt-1">Restore previously exported data (will overwrite current database).</p>
                </div>
                <button 
                  onClick={handleImport}
                  disabled={isImporting}
                  className="flex items-center justify-center space-x-2 px-5 py-2.5 bg-gray-800 hover:bg-gray-700 text-white rounded-lg border border-gray-700 transition-colors focus:outline-none focus:ring-2 focus:ring-indigo-500 min-w-[140px]"
                >
                  <Save size={18} />
                  <span>{isImporting ? 'Importing...' : 'Import'}</span>
                </button>
              </div>

              <div className="bg-red-950/20 p-6">
                <h4 className="text-xs font-semibold text-red-500 uppercase tracking-wider mb-4">Danger Zone</h4>
                
                <div className="space-y-6">
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                    <div>
                      <h3 className="text-red-400 font-medium text-base">Delete History</h3>
                      <p className="text-sm text-gray-400 mt-1">Remove all recorded gaming sessions and playtime (keeps games in library).</p>
                    </div>
                    <button 
                      onClick={handleDeleteHistory}
                      disabled={isDeleting}
                      className="flex items-center justify-center space-x-2 px-5 py-2.5 bg-red-900/30 hover:bg-red-900/50 text-red-400 rounded-lg border border-red-900/50 transition-colors focus:outline-none focus:ring-2 focus:ring-red-500 min-w-[160px]"
                    >
                      <Trash2 size={18} />
                      <span>{isDeleting ? 'Deleting...' : 'Delete History'}</span>
                    </button>
                  </div>

                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pt-6 border-t border-red-900/20">
                    <div>
                      <h3 className="text-red-400 font-medium text-base">Reset Application</h3>
                      <p className="text-sm text-gray-400 mt-1">Permanently delete your games, history, settings, and tracking data.</p>
                    </div>
                    <button 
                      onClick={handleReset}
                      disabled={isResetting}
                      className="flex items-center justify-center space-x-2 px-5 py-2.5 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-red-500 min-w-[160px]"
                    >
                      <Power size={18} />
                      <span>{isResetting ? 'Resetting...' : 'Reset Application'}</span>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Spacer for bottom padding */}
          <div className="h-8"></div>
        </div>
      </div>
      
      {showAuthModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-gray-950/80 backdrop-blur-sm">
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
