import re
with open('src/pages/CurrentlyPlaying.tsx', 'r') as f:
    c = f.read()

# Make the card interactive for keyboard
target = 'className="relative bg-gray-900 rounded-2xl border border-gray-800 overflow-hidden shadow-2xl flex flex-col md:flex-row group transition-colors hover:border-indigo-500/50"'
replacement = 'className="relative bg-gray-900 rounded-2xl border border-gray-800 overflow-hidden shadow-2xl flex flex-col md:flex-row group transition-colors hover:border-indigo-500/50 focus-within:ring-2 focus-within:ring-indigo-500"'
c = c.replace(target, replacement)

with open('src/pages/CurrentlyPlaying.tsx', 'w') as f:
    f.write(c)