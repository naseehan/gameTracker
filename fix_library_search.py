import re
with open('src/pages/Library.tsx', 'r') as f:
    c = f.read()

c = c.replace('placeholder="Search library..."', 'placeholder="Search library..."\n            aria-label="Search library"')

with open('src/pages/Library.tsx', 'w') as f:
    f.write(c)