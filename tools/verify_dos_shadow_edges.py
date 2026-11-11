#!/usr/bin/env python3
"""Compile an original MZ fixture covering offset 0Fh and far-call chaining."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

from dos_terminal import Session

ROOT = Path(__file__).resolve().parents[1]


def fixture():
    # Both segments execute at offset 0Fh. The caller is a far CALL, while
    # the callee sets CX and RETFs. A stale CS byte guard repeats the CALL.
    header, code = bytearray(32), bytearray(64)
    header[:2] = b'MZ'
    for offset, value in [(2, 96), (4, 1), (6, 1), (8, 2), (12, 0xffff),
                          (16, 0xfffe), (20, 15), (24, 28), (28, 18)]:
        struct.pack_into('<H', header, offset, value)
    code[15:25] = bytes.fromhex('9a 0f 00 02 00 b8 00 4c cd 21')
    code[47:51] = bytes.fromhex('b9 78 56 cb')
    return bytes(header + code)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, required=True)
    args = parser.parse_args()
    engine = args.engine.resolve()
    with tempfile.TemporaryDirectory(prefix='shadow-edges-', dir=ROOT / 'target') as temp:
        program = Path(temp) / 'EDGES.EXE'
        program.write_bytes(fixture())
        subprocess.run([sys.executable, str(ROOT / 'tools/build_dos_shadow.py'),
                        str(program), '--engine', str(engine), '--steps', '1000',
                        '--sites', '16', '--verify'], cwd=ROOT, check=True)
        digest = hashlib.sha256(program.read_bytes()).hexdigest()
        manifests = list((ROOT / 'shadow').glob(f'*/{digest}/manifest.json'))
        compiler = subprocess.check_output(['rustc', '-vV'], text=True)
        host = next(line.split(': ', 1)[1] for line in compiler.splitlines() if line.startswith('host: '))
        manifest_path = next(p for p in manifests if
                             json.loads(p.read_text()).get('guard_tests_verified') and
                             json.loads(p.read_text()).get('target_triple') == host)
        manifest = json.loads(manifest_path.read_text())
        native = manifest_path.parent / manifest['engine']
        source = (manifest_path.parent / 'backend.rs').read_text()
        assert 'mod ins_f {' in source and 'mod if {' not in source
        frames = []
        for executable in [engine, native]:
            session = Session(executable, program, stop_log='')
            try:
                frames.append(session.send('run 1000'))
            finally:
                session.close()
        for key in ['status', 'detail', 'registers', 'selectors', 'eflags', 'steps', 'screen']:
            assert frames[0][key] == frames[1][key], key
        assert frames[1]['status'] == 'exit' and frames[1]['registers'][1] == 0x5678
        assert frames[1]['native_block_instructions'] > 0
    print('PASS: offset 0Fh compiles; far-call/return state and exit match')


if __name__ == '__main__':
    main()
