#!/usr/bin/env python3
"""Smoke-test local DOS games on private copies; no game assets are bundled."""
import argparse
import hashlib
from pathlib import Path
import shutil
import tempfile
import time
from dos_terminal import Session


def verify(engine, program, seconds, doom=False):
    program = program.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix='pil-dos-game-') as temporary:
        mount = Path(temporary) / 'game'
        shutil.copytree(program.parent, mount)
        session = Session(engine, mount / program.name, mount=mount,
                          args='-nosound -nomusic -warp 1 1 -skill 3' if doom else None,
                          stop_log='')
        frames = set()
        started = time.monotonic()
        try:
            session.send('clock 0')
            while time.monotonic() - started < seconds:
                frame = session.send('run 1000000')
                if frame['status'] not in ('running', 'waiting_input', 'exited'):
                    raise RuntimeError(f"{program.name}: {frame['status']}: {frame.get('detail')}")
                pixels = frame.get('graphics13')
                if pixels and any(bytes.fromhex(pixels)):
                    frames.add(hashlib.sha256(pixels.encode()).hexdigest())
                if frame['status'] != 'running':
                    break
            if len(frames) < 2:
                raise RuntimeError(f'{program.name}: fewer than two distinct nonblank graphics frames')
            print(f"PASS {program.name}: {len(frames)} changing frames; {frame['steps']} guest steps; {frame['status']}")
        finally:
            session.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, required=True)
    parser.add_argument('--doom', type=Path)
    parser.add_argument('--3dbench', dest='bench', type=Path)
    parser.add_argument('--seconds', type=float, default=30)
    args = parser.parse_args()
    if not (args.doom or args.bench) or args.seconds <= 0:
        parser.error('supply a game path and positive --seconds')
    engine = args.engine.resolve(strict=True)
    for path, doom in [(args.doom, True), (args.bench, False)]:
        if path:
            verify(engine, path, args.seconds, doom)
