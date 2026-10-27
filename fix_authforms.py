with open('src/components/AuthForms.tsx', 'r', encoding='utf-8') as f:
    content = f.read()

target = """    try {
      const { data, error } = await supabase.auth.signInWithOAuth({"""

replacement = """    try {
      console.log('[OAuth] Google sign-in started');
      const { data, error } = await supabase.auth.signInWithOAuth({"""

content = content.replace(target, replacement)

target2 = """        if (data.url.includes('mock-project.supabase.co')) {
          setError('Google Sign-In requires a real Supabase configuration. Please provide a real Supabase URL to enable OAuth.');
        } else {
          await openUrl(data.url);
          setMessage('Please complete the sign-in in your web browser.');
        }"""

replacement2 = """        if (data.url.includes('mock-project.supabase.co')) {
          setError('Google Sign-In requires a real Supabase configuration. Please provide a real Supabase URL to enable OAuth.');
        } else {
          console.log('[OAuth] OAuth URL generated');
          await openUrl(data.url);
          setMessage('Please complete the sign-in in your web browser.');
        }"""

content = content.replace(target2, replacement2)

with open('src/components/AuthForms.tsx', 'w', encoding='utf-8') as f:
    f.write(content)