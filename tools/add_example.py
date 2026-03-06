import re

with open(r'E:\Entwicklung\rust\radical-roguelike\src\vocab.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Add example: "" before closing brace on VocabEntry lines
content = re.sub(
    r'(hsk:\s*\d+)\s*\}',
    r'\1, example: "" }',
    content
)

with open(r'E:\Entwicklung\rust\radical-roguelike\src\vocab.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Done')
