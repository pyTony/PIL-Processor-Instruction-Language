#!/usr/bin/env python3
"""Compare the included GW-BASIC training workload with a shadow bundle."""
import argparse
import os
from pathlib import Path
import time
import statistics
from dos_terminal import Session, screen_rows

ROOT = Path(__file__).resolve().parents[1]


def run(engine, program, keys):
    session = Session(engine, program, stop_log='')
    started = time.perf_counter()
    try:
        frame = session.send('run 500000')
        for byte in keys:
            while True:
                reply = session.send(f'key {byte} {28 if byte == 13 else 0}')
                if reply['ok']: break
                if 'buffer full' not in reply.get('text', ''): raise RuntimeError(reply)
                frame = session.send('run 10000')
            frame = session.send('run 10000')
        for _ in range(30):
            frame = session.send('run 1000000')
            if frame['status'] != 'running': break
        elapsed = time.perf_counter() - started
        text = '\n'.join(row.rstrip() for row, _ in screen_rows(frame))
        assert frame['status'] == 'exit', (frame['status'], frame.get('detail'), text)
        assert '125250' in text, text
        return elapsed, frame
    finally:
        session.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-engine', type=Path, required=True)
    parser.add_argument('--engine', type=Path, default=ROOT / 'target/release' / ('pil.exe' if os.name == 'nt' else 'pil'))
    parser.add_argument('--program', type=Path, default=ROOT / 'driveD/GWBASIC.EXE')
    parser.add_argument('--samples', type=int, default=3)
    args = parser.parse_args()
    if args.samples < 1: parser.error('samples must be positive')
    keys = (ROOT / 'programs/dos/training/GWBASIC.KEYS').read_text().replace('\n', '\r').encode('ascii')
    reference = None
    times = {'interpreter': [], 'bundle': []}
    for sample in range(args.samples):
        order = [('interpreter', args.engine), ('bundle', args.native_engine)]
        if sample % 2: order.reverse()
        for label, engine in order:
            elapsed, frame = run(engine.resolve(), args.program.resolve(), keys)
            if reference is None: reference = frame
            for key in ('status', 'detail', 'steps', 'registers', 'selectors', 'eflags', 'msw', 'cursor', 'screen'):
                assert frame.get(key) == reference.get(key), f'different {key}'
            if label == 'bundle': assert frame.get('native_blocks', 0) > 0, 'no compiled blocks executed'
            times[label].append(elapsed)
            print(f'{label}: {elapsed:.4f}s; compiled blocks={frame.get("native_blocks", 0)}; compiled instructions={frame.get("native_block_instructions", 0)}', flush=True)
    for label, values in times.items(): print(f'{label} median: {statistics.median(values):.4f}s')
    print('PASS: GW-BASIC sum, exit and reported machine state match')


if __name__ == '__main__':
    main()
