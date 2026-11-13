#!/usr/bin/env python3
"""Run the project's own DOS demos in Dine and compare with Python oracles.

FACTOR.COM (0!..99!, argument parsing), RECURFIB.COM (recursive Fibonacci,
regenerated from tools/build_recursive_fib.py) and the MZ argument demo
(tools/build_mz_demo.py, relocation and command tails). Every program runs
through the same `pil --dos-session` engine as any other DOS program.
"""
import argparse
import math
import os
from pathlib import Path
import tempfile

from build_mz_demo import build as build_mz
from build_recursive_fib import USAGE as FIB_USAGE, build as build_fib
from dos_terminal import Session

ROOT = Path(__file__).resolve().parents[1]
FACTOR_USAGE = b'Usage: FACTOR [n]   prints 0! .. n! for n up to 99 (default 25)\r\n'


def factorials(n):
    return ''.join(f'{i:2d}! = {math.factorial(i)}\r\n' for i in range(n + 1)).encode('ascii')


def fibonacci(n):
    a, b, lines = 0, 1, []
    for i in range(n + 1):
        lines.append(f'{i} = {a}\r\n')
        a, b = b, a + b
    return ''.join(lines).encode()


def dine(engine, program, text):
    session = Session(engine, program, args=text, stop_log='')
    output = bytearray()
    try:
        for _ in range(30):
            frame = session.send('run 1000000')
            output.extend(frame['output'].encode('utf-8'))
            if frame['status'] != 'running':
                break
        return frame, bytes(output)
    finally:
        session.close()


def check(engine, program, cases):
    for text, expected in cases:
        frame, output = dine(engine, program, text)
        assert frame['status'] == 'exit' and output == expected, (program.name, text, frame.get('detail'), output[:200])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, default=ROOT / 'target/debug' / ('pil.exe' if os.name == 'nt' else 'pil'))
    args = parser.parse_args()
    engine = args.engine.resolve()
    check(engine, ROOT / 'programs/dos/FACTOR.COM',
          [(text, FACTOR_USAGE if n is None else factorials(n)) for text, n in
           [(None, 25), ('0', 0), ('1', 1), ('10', 10), ('50', 50), ('99', 99), ('09', 9), ('   ', 25),
            ('100', None), ('abc', None), ('-1', None), ('3x', None), ('1 2', None)]])
    fib = ROOT / 'programs/dos/RECURFIB.COM'
    binary, asm = build_fib()
    assert fib.read_bytes() == binary and (ROOT / 'programs/dos/recurfib.asm').read_text() == asm
    check(engine, fib, [(text, FIB_USAGE if n is None else fibonacci(n)) for text, n in
                        [(None, 12), ('0', 0), ('1', 1), ('8', 8), ('20', 20), ('  ', 12), ('21', None), ('abc', None)]])
    with tempfile.TemporaryDirectory(prefix='mz-demo-') as temp:
        program = Path(temp) / 'ARGS.EXE'
        program.write_bytes(build_mz())
        check(engine, program, [(text, ('PIL MZ: ' + (' ' + text if text is not None else '') + '\r\n').encode('utf-8'))
                                for text in [None, 'Hello from PIL', '   ']])
    print('PASS: FACTOR, RECURFIB and the MZ argument demo match their Python oracles in Dine')


if __name__ == '__main__':
    main()
