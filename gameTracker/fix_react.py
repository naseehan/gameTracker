import re

with open('src/pages/GameDetails.tsx', 'r') as f:
    c = f.read()

# Fix className template literal
c = re.sub(r'className=\{px-3 py-1\.5 rounded-lg text-xs font-medium transition-colors border \}',
    'className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors border ${game.is_ignored ? "bg-indigo-600 border-indigo-500 text-white hover:bg-indigo-500" : "bg-gray-800 border-gray-700 text-gray-300 hover:bg-gray-700 hover:text-white"}`}', c)

# Fix confirm string
c = c.replace('if (confirm(Remove "" from library?))', 'if (confirm(`Remove "${game.name}" from library?`))')

# Ensure relative positioning on the cover container
c = c.replace('className="w-32 h-48 rounded-lg shadow-2xl overflow-hidden bg-gray-800 border border-gray-700 flex-shrink-0"', 
    'className="relative w-32 h-48 rounded-lg shadow-2xl overflow-hidden bg-gray-800 border border-gray-700 flex-shrink-0"')

with open('src/pages/GameDetails.tsx', 'w') as f:
    f.write(c)