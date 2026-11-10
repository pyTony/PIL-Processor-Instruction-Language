#!/usr/bin/env python3
"""Compare all 1896 ANSI Mandelbrot cells: Python oracle, Dine and native Rust."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

from dos_terminal import Session

ROOT = Path(__file__).resolve().parents[1]


def s16(value):
    return (value + 32768) % 65536 - 32768


def escape(cx, cy):
    zr = zi = 0
    for n in range(15):
        rr, ii = s16((zr*zr) // 256), s16((zi*zi) // 256)
        if s16(rr + ii) > 1024:
            return 15 - n
        zr, zi = s16(rr - ii + cx), s16(2*s16((zr*zi) // 256) + cy)
    return 0


def reference():
    shades = b' @@%%**++==--::.'
    colours = [30, 96, 96, 94, 94, 95, 95, 91, 91, 93, 93, 92, 92, 36, 34, 30]
    assert escape(0, 0) == 0 and shades[0] == 32 and colours[0] == 30
    result = bytearray(b'\x1b[0;97;40m\x1b[2J\x1b[HPIL native ANSI Mandelbrot\r\n')
    for y in range(24):
        for x in range(79):
            k = escape(-512 + 10*x, -288 + 25*y)
            result += f'\x1b[{colours[k]}m'.encode('ascii') + bytes([shades[k]])
        result += b'\r\n'
    return bytes(result) + b'\x1b[0m\r\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, default=ROOT / 'target/debug' / ('pil.exe' if os.name == 'nt' else 'pil'))
    parser.add_argument('--native', type=Path, help='an already-built standalone Mandelbrot executable')
    parser.add_argument('--out', type=Path, default=ROOT / 'target/ansi-mandel-verified')
    args = parser.parse_args()
    engine, program = args.engine.resolve(), ROOT / 'programs/dos/MANDANSI.COM'
    expected = reference()
    assert len(expected) <= 16384
    args.out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='ansi-mandel-', dir=ROOT / 'target') as temp:
        temp = Path(temp)
        source = temp / 'mandelbrot.rs'
        subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], check=True)
        generated = source.read_text()
        assert not any(s in generated for s in ('mod guest', 'enum Token', 'InstructionBackend', 'DecodeCache', 'include!('))
        native = args.native.resolve() if args.native else temp / ('mandelbrot.exe' if os.name == 'nt' else 'mandelbrot')
        if not args.native:
            subprocess.run(['rustc', '--edition=2021', '-Awarnings', '-O', str(source), '-o', str(native)], check=True)
        source.unlink()
        started = time.perf_counter()
        result = subprocess.run([str(native)], capture_output=True, timeout=30)
        native_seconds = time.perf_counter() - started
        assert result.returncode == 0 and result.stderr == b'', result
        assert result.stdout == expected, 'native ANSI output differs from the independent 8.8 oracle'
        session = Session(engine, program, stop_log='')
        interpreted = bytearray()
        started = time.perf_counter()
        try:
            for _ in range(10):
                frame = session.send('run 1000000')
                interpreted.extend(frame['output'].encode('ascii'))
                if frame['status'] != 'running':
                    break
            assert frame['status'] == 'exit', frame.get('detail')
            assert bytes(interpreted) == expected, 'Dine ANSI output differs from the independent 8.8 oracle'
        finally:
            session.close()
        interpreted_seconds = time.perf_counter() - started
    (args.out / 'mandelbrot.ans').write_bytes(expected)
    (args.out / 'mandelbrot.txt').write_bytes(re.sub(rb'\x1b\[[0-9;]*[A-Za-z]', b'', expected))
    report = dict(width=79, height=24, cells=1896, ansi_bytes=len(expected),
                  guest_steps=frame['steps'], native_seconds=native_seconds,
                  dine_session_seconds=interpreted_seconds,
                  timing='one local wall-clock sample; native startup and Dine JSON/frame costs included; Rust compilation excluded')
    (args.out / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(f'PASS: 1896 coloured cells and all {len(expected)} ANSI bytes match Python, Dine and standalone Rust')
    print(f'Native {native_seconds:.4f}s; Dine session {interpreted_seconds:.4f}s; {frame["steps"]} guest steps (single sample, unequal frontend overhead)')


if __name__ == '__main__':
    main()
