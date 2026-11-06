#!/usr/bin/env python3
"""Exercise the full mode-13h demo, palette animation and keyboard exit."""
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from dos_terminal import Session, find_engine

root = Path(__file__).resolve().parents[1]
engine = root / 'target' / 'release' / ('pil.exe' if sys.platform == 'win32' else 'pil')
session = Session(find_engine(str(engine)), root / 'programs' / 'dos' / 'MANDCYC.COM')
try:
    completed = None
    for _ in range(60):
        reply = session.send('run 1000000')
        if reply['status'] != 'running':
            raise SystemExit('Demo stopped: ' + reply.get('detail', ''))
        pixels = bytes.fromhex(reply['graphics13'])
        if pixels[-1] != 0 and 0 in pixels and len(set(pixels)) > 8:
            if completed and reply['graphics13'] == completed[0] and reply['palette'] != completed[1]:
                break
            completed = (reply['graphics13'], reply['palette'])
    else:
        raise SystemExit('Full frame and palette cycling not observed within 60 million steps')
    session.send('key 13 28')
    for _ in range(3):
        reply = session.send('run 1000000')
        if reply['status'] == 'exit':
            break
    assert reply['status'] == 'exit' and reply['detail'] == 'exit code 0', reply
    assert 'graphics13' not in reply, 'text mode was not restored'
    print('PASS: full frame, palette cycling, key exit and text mode restoration')
finally:
    session.close()
