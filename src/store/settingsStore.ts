import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface SettingsState {
  trackingEnabled: boolean;
  pollingIntervalSeconds: number;
  autoDiscoveryEnabled: boolean;
  autoDiscoveryInterval: number;
  discoverySteam: boolean;
  discoveryEpic: boolean;
  discoveryGog: boolean;
  isLoading: boolean;
  error: string | null;
  fetchSettings: () => Promise<void>;
  updateSetting: (key: string, value: string) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  trackingEnabled: true,
  pollingIntervalSeconds: 2,
  autoDiscoveryEnabled: true,
  autoDiscoveryInterval: 1800,
  discoverySteam: true,
  discoveryEpic: true,
  discoveryGog: true,
  isLoading: false,
  error: null,

  fetchSettings: async () => {
    set({ isLoading: true, error: null });
    try {
      const tracking = await invoke<string | null>('get_setting', { key: 'tracking_enabled' });
      const polling = await invoke<string | null>('get_setting', { key: 'polling_interval_seconds' });
      const autoDiscovery = await invoke<string | null>('get_setting', { key: 'auto_discovery_enabled' });
      const autoDiscoveryInterval = await invoke<string | null>('get_setting', { key: 'auto_discovery_interval' });
      const discoverySteam = await invoke<string | null>('get_setting', { key: 'discovery_provider_steam' });
      const discoveryEpic = await invoke<string | null>('get_setting', { key: 'discovery_provider_epic_games' });
      const discoveryGog = await invoke<string | null>('get_setting', { key: 'discovery_provider_gog' });
      
      set({
        trackingEnabled: tracking !== 'false',
        pollingIntervalSeconds: polling ? parseInt(polling, 10) : 2,
        autoDiscoveryEnabled: autoDiscovery !== 'false',
        autoDiscoveryInterval: autoDiscoveryInterval ? parseInt(autoDiscoveryInterval, 10) : 1800,
        discoverySteam: discoverySteam !== 'false',
        discoveryEpic: discoveryEpic !== 'false',
        discoveryGog: discoveryGog !== 'false',
        isLoading: false
      });
    } catch (err: any) {
      set({ error: err.toString(), isLoading: false });
    }
  },

  updateSetting: async (key: string, value: string) => {
    try {
      await invoke('update_setting', { key, value });
      if (key === 'tracking_enabled') {
        set({ trackingEnabled: value === 'true' });
      } else if (key === 'polling_interval_seconds') {
        set({ pollingIntervalSeconds: parseInt(value, 10) });
      } else if (key === 'auto_discovery_enabled') {
        set({ autoDiscoveryEnabled: value === 'true' });
      } else if (key === 'auto_discovery_interval') {
        set({ autoDiscoveryInterval: parseInt(value, 10) });
      } else if (key === 'discovery_provider_steam') {
        set({ discoverySteam: value === 'true' });
      } else if (key === 'discovery_provider_epic_games') {
        set({ discoveryEpic: value === 'true' });
      } else if (key === 'discovery_provider_gog') {
        set({ discoveryGog: value === 'true' });
      }
    } catch (err: any) {
      set({ error: err.toString() });
      throw err;
    }
  }
}));

