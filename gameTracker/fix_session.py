with open('src/App.tsx', 'r', encoding='utf-8') as f:
    content = f.read()

target = """            if (access_token && refresh_token) {
              await supabase.auth.setSession({ access_token, refresh_token });
            }"""

replacement = """            if (access_token) {
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
            }"""

content = content.replace(target, replacement)

with open('src/App.tsx', 'w', encoding='utf-8') as f:
    f.write(content)