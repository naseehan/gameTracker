import re

with open('src/store/authStore.ts', 'r', encoding='utf-8') as f:
    content = f.read()

target = """// Use placeholder credentials since we don't have a real Supabase instance.
// In a real app, these would be provided by env vars.
const SUPABASE_URL = 'https://mock-project.supabase.co';
const SUPABASE_ANON_KEY = 'mock-anon-key';"""

replacement = """// Load Supabase credentials from environment variables, fallback to mock credentials for local dev
const SUPABASE_URL = import.meta.env.VITE_SUPABASE_URL || 'https://mock-project.supabase.co';
const SUPABASE_ANON_KEY = import.meta.env.VITE_SUPABASE_ANON_KEY || 'mock-anon-key';"""

content = content.replace(target, replacement)

with open('src/store/authStore.ts', 'w', encoding='utf-8') as f:
    f.write(content)