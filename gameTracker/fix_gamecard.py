import re
with open('src/components/GameCard.tsx', 'r') as f:
    c = f.read()

c = c.replace('onClick={() => onClick(game)}', 'onClick={() => onClick(game)}\n      onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onClick(game); } }}\n      tabIndex={0}\n      role="button"')
c = c.replace('hover:scale-105', 'hover:scale-105 focus:scale-105 focus:outline-none focus:ring-2 focus:ring-indigo-500')

with open('src/components/GameCard.tsx', 'w') as f:
    f.write(c)