#!/usr/bin/env python3
"""Compare a linked DOS shadow service with the interpreter, including VGA.

This runs the original Mandelbrot to its keyboard wait, compares the full
session state exposed by the protocol, then checks key exit and text mode.
Build the native service with PIL_SHADOW_SOURCE first; see DOS_NATIVE.md.
"""
import argparse
import json
import os
from pathlib import Path
import statistics
import time

from dos_terminal import Session

ROOT = Path(__file__).resolve().parents[1]


def run(engine, program):
    session = Session(engine, program, stop_log='')
    try:
        started = time.perf_counter()
        for _ in range(120):
            frame = session.send('run 1000000')
            if frame['status'] != 'running':
                break
        elapsed = time.perf_counter() - started
        assert frame['status'] == 'waiting_input', frame.get('detail')
        assert frame.get('planar_mode') == 0x12, 'expected VGA mode 12h'
        session.send('key 13 28')
        exited = session.send('run 100000')
        assert exited['status'] == 'exit' and exited['detail'] == 'exit code 0', exited.get('detail')
        assert 'graphics_planar' not in exited, 'text mode not restored'
        return elapsed, frame, exited
    finally:
        session.close()


def compare(a, b):
    for key in ('status', 'detail', 'steps', 'cs', 'eip', 'registers', 'selectors',
                'eflags', 'msw', 'columns', 'cursor', 'screen', 'graphics_planar',
                'planar_mode', 'planar_width', 'planar_height', 'planar_rgb'):
        assert a.get(key) == b.get(key), f'different {key}'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-engine', type=Path, required=True)
    parser.add_argument('--engine', type=Path, default=ROOT / 'target/release' / ('pil.exe' if os.name == 'nt' else 'pil'))
    parser.add_argument('--program', type=Path, default=ROOT / 'programs/dos/MANDVGA.COM')
    parser.add_argument('--samples', type=int, default=3)
    parser.add_argument('--report', type=Path, default=ROOT / 'target/dos-shadow-comparison.json')
    args = parser.parse_args()
    if args.samples < 1:
        parser.error('--samples must be positive')
    times = {'interpreter': [], 'native': []}
    reference = None
    for sample in range(args.samples):
        for label, engine in [('interpreter', args.engine), ('native', args.native_engine)]:
            elapsed, frame, exited = run(engine.resolve(), args.program.resolve())
            if reference is None:
                reference = frame, exited
            compare(reference[0], frame)
            compare(reference[1], exited)
            if label == 'native':
                assert frame.get('native_blocks', 0) > 0, 'no compiled blocks executed'
            times[label].append(elapsed)
            print(f'{label} sample {sample + 1}: {elapsed:.6f}s; blocks={frame.get("native_blocks", 0)}; block_instructions={frame.get("native_block_instructions", 0)}', flush=True)
    medians = {key: statistics.median(values) for key, values in times.items()}
    result = {'samples_seconds': times, 'medians_seconds': medians,
              'speedup': medians['interpreter'] / medians['native'],
              'scope': 'DOS draw to key wait, including VGA and protocol frame transfer; excludes process setup and Rust build; key exit verified separately'}
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(result, indent=2), encoding='utf-8')
    print(f'PASS: VGA planes, caption, protocol CPU state and key exit match; speedup={result["speedup"]:.3f}x')


if __name__ == '__main__':
    main()
