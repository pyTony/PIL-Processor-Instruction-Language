#!/usr/bin/env python3
"""Run programs/dos/MANDCGA.COM to its key wait and check every pixel.

The reference repeats the program's 4.12 fixed-point arithmetic in Python
integers (floor shifts, escape when |zr| or |zi| reaches 2.0 or the squared
magnitude passes 4.0, 32 iterations). The caption on the last character row
must be the ROM-font glyphs from src/dos_font8x8.txt in colour 3, which is
FreeDOS's INT 29h colour 7 in a four-colour mode. P must switch the CGA
palette and any other key must return to text mode with exit code 0.
"""
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dos_terminal import Session, find_engine

root = Path(__file__).resolve().parents[1]
engine = root / 'target' / 'release' / ('pil.exe' if sys.platform == 'win32' else 'pil')
CAPTION = b'CGA Mandelbrot  P=palette  other=exit'


def colour(x, y):
    cx, cy = -9216 + 42 * x, -5120 + 51 * y
    zr = zi = 0
    for k in range(32):
        if not -8192 < zr < 8192 or not -8192 < zi < 8192:
            return 1 + k % 3
        zr2, zi2 = (zr * zr) >> 12, (zi * zi) >> 12
        if zr2 + zi2 > 16384:
            return 1 + k % 3
        zr, zi = zr2 - zi2 + cx, ((zr * zi) >> 12) * 2 + cy
    return 0


def font():
    lines = [l for l in (root / 'src' / 'dos_font8x8.txt').read_text().splitlines() if not l.startswith(';')]
    return {int(lines[n * 9][:2], 16): [int(r.replace('#', '1').replace('.', '0'), 2) for r in lines[n * 9 + 1:n * 9 + 9]]
            for n in range(128)}


def pixel(frame, x, y):
    return (frame[(y & 1) * 0x2000 + (y // 2) * 80 + x // 4] >> (6 - (x % 4) * 2)) & 3


session = Session(find_engine(str(engine)), root / 'programs' / 'dos' / 'MANDCGA.COM')
try:
    for _ in range(80):
        reply = session.send('run 1000000')
        if reply['status'] != 'running':
            break
    assert reply['status'] == 'waiting_input', reply.get('detail')
    assert reply['cga_mode'] == 4 and reply['cga_color'] == 0x30, (reply['cga_mode'], reply['cga_color'])
    frame = bytes.fromhex(reply['graphics_cga'])
    wrong = [(x, y) for y in range(192) for x in range(320) if pixel(frame, x, y) != colour(x, y)]
    assert not wrong, f'{len(wrong)} pixels differ, first {wrong[:5]}'
    glyphs = font()
    text = CAPTION + b' ' * (40 - len(CAPTION))
    for col, ch in enumerate(text):
        rows = glyphs[ch] if col < len(CAPTION) else None
        for r in range(8):
            got = [pixel(frame, col * 8 + b, 192 + r) for b in range(8)]
            want = [3 if rows and rows[r] & (0x80 >> b) else (0 if rows else colour(col * 8 + b, 192 + r)) for b in range(8)]
            assert got == want, f'caption column {col} row {r}: {got} != {want}'
    assert reply['cursor'] == [24, len(CAPTION)], reply['cursor']
    session.send('key 112 25')
    reply = session.send('run 100000')
    assert reply['status'] == 'waiting_input' and reply['cga_color'] == 0x10, reply['cga_color']
    assert bytes.fromhex(reply['graphics_cga']) == frame, 'palette switch changed the framebuffer'
    session.send('key 13 28')
    reply = session.send('run 100000')
    assert reply['status'] == 'exit' and reply['detail'] == 'exit code 0', reply
    assert 'graphics_cga' not in reply, 'text mode was not restored'
    print(f'PASS: {320 * 192} reference pixels, {len(CAPTION)}-character ROM-font caption, palette key and text-mode exit')
finally:
    session.close()
