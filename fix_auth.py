import re

with open('src/components/AuthForms.tsx', 'r', encoding='utf-8') as f:
    content = f.read()

# Add import
content = content.replace("import { Loader2, AlertCircle, Check } from 'lucide-react';", 
                          "import { Loader2, AlertCircle, Check } from 'lucide-react';\nimport { open } from '@tauri-apps/plugin-opener';")

# Replace handleGoogleSignIn
target = """  const handleGoogleSignIn = async () => {
    try {
      setError(null);
      setLoading(true);
      // NOTE: Deep linking for Tauri is tricky with standard supabase-js. 
      // Supabase supports standard OAuth flows. We will initiate it here.
      const { error } = await supabase.auth.signInWithOAuth({
        provider: 'google',
        options: {
          redirectTo: 'gametracker://login-callback', // Dummy schema assuming desktop deep link
        }
      });
      if (error) throw error;
      // The browser will open and return via deep link. The actual handler for deep linking 
      // is outside the scope of just this button, but this initiates the flow.
    } catch (err: any) {
      setError('Unable to connect to Google authentication.');
      setLoading(false);
    }
  };"""

replacement = """  const handleGoogleSignIn = async () => {
    try {
      setError(null);
      setLoading(true);
      
      const { data, error } = await supabase.auth.signInWithOAuth({
        provider: 'google',
        options: {
          redirectTo: 'gametracker://login-callback',
          skipBrowserRedirect: true,
        }
      });
      
      if (error) throw error;
      
      if (data?.url) {
        if (data.url.includes('mock-project.supabase.co')) {
          setError('Google Sign-In requires a real Supabase configuration. Please provide a real Supabase URL to enable OAuth.');
        } else {
          await open(data.url);
          setMessage('Please complete the sign-in in your web browser.');
        }
      }
    } catch (err: any) {
      setError('Unable to connect to Google authentication.');
    } finally {
      setLoading(false);
    }
  };"""

content = content.replace(target, replacement)

with open('src/components/AuthForms.tsx', 'w', encoding='utf-8') as f:
    f.write(content)