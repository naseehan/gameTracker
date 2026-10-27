import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { createClient, Session, User } from '@supabase/supabase-js';

// Custom Storage adapter for Supabase using Tauri keyring commands
const tauriSecureStorage = {
  getItem: async (_key: string): Promise<string | null> => {
    try {
      const token = await invoke<string | null>('get_auth_token');
      return token;
    } catch (e) {
      console.error('Error getting auth token:', e);
      return null;
    }
  },
  setItem: async (_key: string, value: string): Promise<void> => {
    try {
      await invoke('store_auth_token', { token: value });
    } catch (e) {
      console.error('Error storing auth token:', e);
    }
  },
  removeItem: async (_key: string): Promise<void> => {
    try {
      await invoke('delete_auth_token');
    } catch (e) {
      console.error('Error deleting auth token:', e);
    }
  },
};

// Load Supabase credentials from environment variables, fallback to mock credentials for local dev
const SUPABASE_URL = import.meta.env.VITE_SUPABASE_URL || 'https://mock-project.supabase.co';
const SUPABASE_ANON_KEY = import.meta.env.VITE_SUPABASE_ANON_KEY || 'mock-anon-key';

export const supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY, {
  auth: {
    storage: tauriSecureStorage,
    autoRefreshToken: true,
    persistSession: true,
    detectSessionInUrl: true,
  },
});

interface AuthState {
  user: User | null;
  session: Session | null;
  loading: boolean;
  initialized: boolean;
  initialize: () => Promise<void>;
  signOut: () => Promise<void>;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  session: null,
  loading: true,
  initialized: false,

  initialize: async () => {
    try {
      const { data: { session } } = await supabase.auth.getSession();
      set({ session, user: session?.user ?? null, initialized: true, loading: false });
      
      supabase.auth.onAuthStateChange((_event, session) => {
        set({ session, user: session?.user ?? null });
      });
    } catch (error) {
      console.error('Failed to initialize auth:', error);
      set({ initialized: true, loading: false });
    }
  },

  signOut: async () => {
    try {
      await supabase.auth.signOut();
      set({ user: null, session: null });
    } catch (error) {
      console.error('Error signing out:', error);
    }
  },
}));
