with open('src/store/authStore.ts', 'r', encoding='utf-8') as f:
    content = f.read()

target_storage = """const tauriSecureStorage = {
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
};"""

replacement_storage = """const tauriSecureStorage = {
  getItem: async (key: string): Promise<string | null> => {
    try {
      const token = await invoke<string | null>('get_auth_token', { key });
      return token;
    } catch (e) {
      console.error('Error getting auth token:', e);
      return null;
    }
  },
  setItem: async (key: string, value: string): Promise<void> => {
    try {
      await invoke('store_auth_token', { key, token: value });
    } catch (e) {
      console.error('Error storing auth token:', e);
    }
  },
  removeItem: async (key: string): Promise<void> => {
    try {
      await invoke('delete_auth_token', { key });
    } catch (e) {
      console.error('Error deleting auth token:', e);
    }
  },
};"""

content = content.replace(target_storage, replacement_storage)

target_auth = """    storage: tauriSecureStorage,
    autoRefreshToken: true,
    persistSession: true,
    detectSessionInUrl: true,"""

replacement_auth = """    storage: tauriSecureStorage,
    autoRefreshToken: true,
    persistSession: true,
    detectSessionInUrl: false,
    flowType: 'pkce',"""

content = content.replace(target_auth, replacement_auth)

with open('src/store/authStore.ts', 'w', encoding='utf-8') as f:
    f.write(content)