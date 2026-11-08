#!/usr/bin/env python3
"""Run programs/dos/MANDVGA.COM to its key wait and check every pixel.

The program draws 320x240 points as 2x2 blocks in VGA mode 12h through the
graphics controller (write mode 2, bit mask, latch reads). The reference
repeats its 4.12 fixed-point arithmetic in Python (escape when |zr| or |zi|
reaches 2.0 or the squared magnitude passes 4.0, 30 iterations) and
compares the four planes: colour 0 inside, otherwise 1 + iterations mod 15.
The 30th text row must hold the caption in the 8x8 ROM font, each row shown
twice in the 16-line cells, in colour 7. Any key must return to text mode.
"""
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dos_terminal import Session, find_engine

root = Path(__file__).resolve().parents[1]
engine = root / 'target' / 'release' / ('pil.exe' if sys.platform == 'win32' else 'pil')
CAPTION = b'VGA 640x480 Mandelbrot  any key=exit'
STEP = 42


def colour(px, py):
    cx, cy = -9216 + STEP * px, -120 * STEP + STEP * py
    zr = zi = 0
    for k in range(30):
        if not -8192 < zr < 8192 or not -8192 < zi < 8192:
            return 1 + k % 15
        zr2, zi2 = (zr * zr) >> 12, (zi * zi) >> 12
        if zr2 + zi2 > 16384:
            return 1 + k % 15
        zr, zi = zr2 - zi2 + cx, ((zr * zi) >> 12) * 2 + cy
    return 0


def font():
    lines = [l for l in (root / 'src' / 'dos_font8x8.txt').read_text(encoding='utf-8').splitlines() if not l.startswith(';')]
    return {int(lines[n * 9][:2], 16): [int(r.replace('#', '1').replace('.', '0'), 2) for r in lines[n * 9 + 1:n * 9 + 9]]
            for n in range(128)}


session = Session(find_engine(str(engine)), root / 'programs' / 'dos' / 'MANDVGA.COM')
try:
    for _ in range(120):
        reply = session.send('run 1000000')
        if reply['status'] != 'running':
            break
    assert reply['status'] == 'waiting_input', reply.get('detail')
    assert (reply['planar_mode'], reply['planar_width'], reply['planar_height']) == (0x12, 640, 480)
    data = bytes.fromhex(reply['graphics_planar'])
    size = 80 * 480
    planes = [data[n * size:(n + 1) * size] for n in range(4)]

    def pixel(x, y):
        return sum(((planes[n][y * 80 + x // 8] >> (7 - x % 8)) & 1) << n for n in range(4))

    wrong = []
    for py in range(232):
        row = [colour(px, py) for px in range(320)]
        for y in (2 * py, 2 * py + 1):
            wrong += [(x, y) for x in range(640) if pixel(x, y) != row[x // 2]]
    assert not wrong, f'{len(wrong)} pixels differ, first {wrong[:5]}'
    glyphs = font()
    for col, ch in enumerate(CAPTION):
        for r in range(16):
            got = [pixel(col * 8 + b, 464 + r) for b in range(8)]
            want = [7 if glyphs[ch][r // 2] & (0x80 >> b) else 0 for b in range(8)]
            assert got == want, f'caption column {col} row {r}: {got} != {want}'
    assert reply['cursor'] == [29, len(CAPTION)], reply['cursor']
    session.send('key 13 28')
    reply = session.send('run 100000')
    assert reply['status'] == 'exit' and reply['detail'] == 'exit code 0', reply
    assert 'graphics_planar' not in reply, 'text mode was not restored'
    print(f'PASS: {640 * 464} reference pixels in four planes, {len(CAPTION)}-character caption, key exit')
finally:
    session.close()
