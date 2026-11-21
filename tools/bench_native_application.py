#!/usr/bin/env python3
"""Time one DOS program as the original on Dine and as a native application.

Both run the same command line in a fresh folder holding the same support
files, through the same session protocol (`run N buffered` until the program
ends), so neither side's time includes process startup, a window, painting
or key waits. The original runs through `pil --dos-session` with the folder
as C:; the native application runs with DINE_PROTOCOL=1, copied (with its
shared runtime) into the folder so it finds the same files beside itself.
An empty configuration is used for both. Reports the median wall time of the
run loop, guest instructions, and how many the native side retired natively.

Example (user-supplied third-party files stay outside the repository):

  python tools/bench_native_application.py --original path/to/TPC.EXE \
      --native path/to/Native_TPC.exe --support path/to/TURBO.TPL \
      --source BIG.PAS --args "BIG.PAS" --runs 5
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import tempfile
import time

from dos_terminal import Session

ROOT = Path(__file__).resolve().parents[1]
SLICE = 'run 1000000 buffered'


class NativeSession(Session):
    def __init__(self, executable, directory, config, args):
        env = {**os.environ, 'PILDOS_CONFIG': str(config), 'DINE_PROTOCOL': '1'}
        self.process = subprocess.Popen(
            [str(executable), *args.split()], cwd=directory, env=env,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding='utf-8', creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        self.seq = 0
        self.com1_out = b''
        self.last = self._read()


def finish(session, limit):
    """Run to the end; return (seconds, last reply)."""
    start = time.perf_counter()
    for _ in range(limit):
        reply = session.send(SLICE)
        if not reply.get('ok'):
            raise RuntimeError(reply)
        if reply['status'] != 'running':
            break
    else:
        raise RuntimeError('program did not end within the slice limit')
    elapsed = time.perf_counter() - start
    return elapsed, session.send('snapshot')


def prepare(out, files):
    directory = Path(tempfile.mkdtemp(prefix='run-', dir=out))
    for path in files:
        shutil.copy2(path, directory / path.name)
    config = directory / 'empty.conf'
    config.write_text('', encoding='utf-8')
    return directory, config


def one(kind, args, files, limit):
    directory, config = prepare(args.out, files)
    if kind == 'original':
        program = directory / args.original.name
        shutil.copy2(args.original, program)
        session = Session(args.engine, program, mount=directory, args=args.args,
                          config=config, stop_log='')
    else:
        native = directory / args.native.name
        shutil.copy2(args.native, native)
        shutil.copy2(args.native.parent / args.runtime_name, directory / args.runtime_name)
        session = NativeSession(native, directory, config, args.args)
    try:
        elapsed, reply = finish(session, limit)
    finally:
        session.close()
    if reply['status'] != 'exit':
        raise RuntimeError(f'{kind}: {reply["status"]} {reply.get("detail")}')
    return {
        'kind': kind, 'seconds': elapsed, 'steps': reply.get('steps'),
        'native': reply.get('native_total_instructions'),
        'interpreted': reply.get('interpreter_instructions'),
        'exit': reply.get('exit_code', reply.get('detail')),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--original', type=Path, required=True, help='the DOS executable')
    parser.add_argument('--native', type=Path, required=True, help='its Native_ application')
    parser.add_argument('--engine', type=Path, help='pil executable (default target/release)')
    parser.add_argument('--support', type=Path, action='append', default=[],
                        help='file the program needs beside it (repeatable)')
    parser.add_argument('--source', type=Path, action='append', default=[],
                        help='input file for the command line (repeatable)')
    parser.add_argument('--args', default='', help='DOS command line')
    parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--slices', type=int, default=100000, help='safety limit of 1M-step slices')
    parser.add_argument('--out', type=Path, default=ROOT / 'target' / 'native-app-benchmarks')
    args = parser.parse_args()
    suffix = '.exe' if os.name == 'nt' else ''
    args.engine = (args.engine or ROOT / 'target' / 'release' / ('pil' + suffix)).resolve(strict=True)
    args.original = args.original.resolve(strict=True)
    args.native = args.native.resolve(strict=True)
    args.runtime_name = 'dine.dll' if os.name == 'nt' else 'libdine.so'
    if not 1 <= args.runs <= 50:
        parser.error('runs must be 1..50')
    args.out.mkdir(parents=True, exist_ok=True)
    args.out = args.out.resolve()
    files = [p.resolve(strict=True) for p in args.support + args.source]
    results = {'original': [], 'native': []}
    for _ in range(args.runs):
        # Alternate, so host load drifts affect both sides alike.
        for kind in ('original', 'native'):
            result = one(kind, args, files, args.slices)
            results[kind].append(result)
            print(json.dumps(result), flush=True)
    summary = {}
    for kind, runs in results.items():
        summary[kind] = {'median_seconds': statistics.median(r['seconds'] for r in runs),
                         'steps': runs[-1]['steps'], 'native_instructions': runs[-1]['native'],
                         'interpreted_instructions': runs[-1]['interpreted']}
    o, n = summary['original']['median_seconds'], summary['native']['median_seconds']
    summary['speedup'] = o / n if n else None
    print(json.dumps({'summary': summary}, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
