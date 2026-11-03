import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { ActiveSessionInfo } from '../types';

interface SessionsState {
  active: ActiveSessionInfo[];
  loaded: boolean;
  fetchActive: () => Promise<void>;
}

/** Games being played right now. Kept up to date by useBackendSync. */
export const useSessionsStore = create<SessionsState>((set) => ({
  active: [],
  loaded: false,
  fetchActive: async () => {
    const active = await invoke<ActiveSessionInfo[]>('get_active_sessions');
    set({ active, loaded: true });
  },
}));
