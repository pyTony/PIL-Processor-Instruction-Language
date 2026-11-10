import sys
L = [l for l in open('src/dos_font8x8.txt').read().splitlines() if not l.startswith(';')]
g = {int(L[i * 9][:2], 16): L[i * 9 + 1:i * 9 + 9] for i in range(128)}
chars = sys.argv[1] if len(sys.argv) > 1 else 'aceinouyACEOUN'
rows = [''] * 8
for ch in chars:
    for r in range(8):
        rows[r] += g[ord(ch)][r] + '  '
print('  '.join(f'{c:<8}' for c in chars))
print('\n'.join(rows))
