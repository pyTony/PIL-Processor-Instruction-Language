#!/usr/bin/env python3
"""Compare fixed-step Landmark startup/calibration, without Tk or host timing.

This measures engine throughput, not Landmark's displayed MHz score. The
third-party LM60.EXE must be supplied locally and is never modified.
"""
import argparse
from pathlib import Path
import statistics
import time

from dos_terminal import Session, refresh_frame, run_slice

ROOT = Path(__file__).resolve().parents[1]


def run(engine, program, steps):
    session = Session(engine, program, args='/b /q /3', stop_log='')
    try:
        started = time.perf_counter()
        remaining = steps
        while remaining:
            budget = min(remaining, 1_000_000)
            reply = run_slice(session, budget)
            assert reply['status'] == 'running', (reply['status'], reply['detail'])
            remaining -= budget
        elapsed = time.perf_counter() - started
        return elapsed, refresh_frame(session, reply)
    finally:
        session.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, required=True, help='reference engine')
    parser.add_argument('--native-engine', type=Path, required=True)
    parser.add_argument('--program', type=Path, default=ROOT / 'driveD/dosbench/MARKS/LM60/LM60.EXE')
    parser.add_argument('--samples', type=int, default=3)
    parser.add_argument('--steps', type=int, default=5_000_000)
    args = parser.parse_args()
    if args.samples < 1 or args.steps < 1:
        parser.error('samples and steps must be positive')
    engines = [('reference', args.engine.resolve()), ('native', args.native_engine.resolve())]
    times = {label: [] for label, _ in engines}
    reference = None
    for sample in range(args.samples):
        for label, engine in (engines if sample % 2 == 0 else engines[::-1]):
            elapsed, frame = run(engine, args.program.resolve(), args.steps)
            if reference is None:
                reference = frame
            for key in ['status', 'detail', 'registers', 'selectors', 'eflags', 'msw', 'eip', 'steps', 'cursor', 'screen']:
                assert frame[key] == reference[key], f'different {key}'
            if label == 'native':
                assert frame['native_block_instructions'] > 0, 'no compiled blocks executed'
            times[label].append(elapsed)
            print(f'{label}: {elapsed:.4f}s; native instructions={frame["native_block_instructions"]}', flush=True)
    for label, samples in times.items():
        print(f'{label} median: {statistics.median(samples):.4f}s')
    print('PASS: fixed-step CPU, selectors, flags, screen and reported retirement count match')


if __name__ == '__main__':
    main()
