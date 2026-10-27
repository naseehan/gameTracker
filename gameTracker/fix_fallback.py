with open('src/App.tsx', 'r', encoding='utf-8') as f:
    content = f.read()

target = """    // Fallback manual listener
    const unlistenFallbackPromise = listen<string>('scheme-request-received', async (event) => {
      const urlStr = event.payload;
      if (urlStr.includes('login-callback')) {
        try {
          const parsedUrl = new URL(urlStr);
          const code = parsedUrl.searchParams.get('code');
          if (code) {
            await supabase.auth.exchangeCodeForSession(code);
          } else {
            const hash = parsedUrl.hash.substring(1);
            const params = new URLSearchParams(hash);
            const access_token = params.get('access_token');
            const refresh_token = params.get('refresh_token');
            if (access_token) {
              const { error } = await supabase.auth.setSession({ 
                access_token, 
                refresh_token: refresh_token || '' 
              });
              if (error) {
                alert("Session error: " + error.message);
              } else {
                // Force a reload or UI update if needed, though Zustand should handle it
              }
            } else {
              alert("No access token found in URL!");
            }
          }
        } catch (e) {
          console.error('Failed to parse deep link URL', e);
        }
      }
    });"""

replacement = """    // Fallback manual listener
    const unlistenFallbackPromise = listen<string>('scheme-request-received', async (event) => {
      const urlStr = event.payload;
      if (urlStr.includes('login-callback')) {
        try {
          const parsedUrl = new URL(urlStr);
          const code = parsedUrl.searchParams.get('code');
          if (code) {
            const { error } = await supabase.auth.exchangeCodeForSession(code);
            if (error) alert("OAuth Code Error: " + error.message);
            else alert("Successfully logged in with code!");
          } else {
            const hash = parsedUrl.hash.substring(1);
            const params = new URLSearchParams(hash);
            const access_token = params.get('access_token');
            const refresh_token = params.get('refresh_token');
            if (access_token) {
              const { error, data } = await supabase.auth.setSession({ 
                access_token, 
                refresh_token: refresh_token || '' 
              });
              if (error) {
                alert("Session error: " + error.message);
              } else {
                alert("Login successful! Welcome " + (data?.user?.email || ""));
              }
            } else {
              alert("URL parsed, but no access_token found. URL: " + urlStr);
            }
          }
        } catch (e: any) {
          alert('Failed to parse deep link URL: ' + e.message);
        }
      }
    });"""

content = content.replace(target, replacement)

with open('src/App.tsx', 'w', encoding='utf-8') as f:
    f.write(content)