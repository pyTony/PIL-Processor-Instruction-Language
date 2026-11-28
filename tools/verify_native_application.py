#!/usr/bin/env python3
"""Compile original COM/MZ fixtures and run native applications without inputs.

Checks the native executable's shared-library path and protocol, including zero
interpreter fallback for these fixtures. It does not certify arbitrary binaries.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def check(engine, runtime, work, name, data, tail, expected, exit_code, pil=False):
    source = work / name
    source.write_bytes(data)
    output = work / ('output-pil' if pil else 'output')
    cmd = [sys.executable, str(ROOT / 'tools' / 'compile_application.py'),
           str(source), '--engine', str(engine), '--runtime', str(runtime),
           '--output-dir', str(output)]
    if pil:
        cmd.append('--pil')
    subprocess.run(cmd, cwd=ROOT, check=True)
    suffix = '.exe' if os.name == 'nt' else ''
    executable = output / ('Native_' + source.stem + suffix)
    if not pil:
        metadata = json.loads(executable.with_suffix('.native.json').read_text())
        assert metadata['original_file_required_at_runtime'] is False
        assert metadata['retained_executable_resources'] is True
    source.unlink()
    assert not source.exists()
    config = work / 'empty.conf'
    config.write_text('', encoding='utf-8')
    env = {**os.environ, 'DINE_TEXT': 'headless', 'DINE_PROTOCOL': '0', 'PILDOS_CONFIG': str(config)}
    result = subprocess.run([str(executable), *tail], cwd=work, env=env,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
    assert result.returncode == exit_code, (result.returncode, result.stderr)
    assert result.stdout.replace(b'\r\n', b'\n') == expected.encode(), result.stdout
    process = subprocess.Popen([str(executable), *tail], cwd=work,
                               env={**env,'DINE_PROTOCOL':'1'}, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, encoding='utf-8')
    try:
        initial = json.loads(process.stdout.readline())
        assert initial['steps'] == 0
        process.stdin.write('run 10000\n')
        process.stdin.flush()
        reply = json.loads(process.stdout.readline())
        assert reply['ok'] and reply['status'] == 'exit', reply
        assert reply['detail'] == f'exit code {exit_code}', reply
        assert reply['interpreter_instructions'] == 0, reply
        assert 0 < reply['native_total_instructions'] <= reply['steps'], reply
        process.stdin.write('quit\n')
        process.stdin.flush()
        json.loads(process.stdout.readline())
        process.wait(timeout=10)
        assert process.returncode == 0
        print(f'{name}: native executable + shared Dine, no external input, '
              f"{reply['native_total_instructions']} native PIL retirements, 0 interpreted")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        for stream in (process.stdin, process.stdout, process.stderr):
            stream.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, required=True)
    parser.add_argument('--runtime', type=Path, required=True)
    args = parser.parse_args()
    target = ROOT / 'target'
    target.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='native-proof-', dir=target) as directory:
        work = Path(directory)
        check(args.engine.resolve(), args.runtime.resolve(), work, 'PROOFC.COM',
              bytes.fromhex('ba0c01b409cd21b82a4ccd21') + b'Native proof\r\n$',
              [], 'Native proof\n', 42)
        check(args.engine.resolve(), args.runtime.resolve(), work, 'PROOFM.EXE',
              (ROOT / 'programs' / 'dos' / 'MZARGS.EXE').read_bytes(),
              ['A', 'B'], 'PIL MZ:  A B\n', 0)
        check(args.engine.resolve(), args.runtime.resolve(), work, 'PROOFC.COM',
              bytes.fromhex('ba0c01b409cd21b82a4ccd21') + b'Native proof\r\n$',
              [], 'Native proof\n', 42, pil=True)
        check(args.engine.resolve(), args.runtime.resolve(), work, 'PROOFM.EXE',
              (ROOT / 'programs' / 'dos' / 'MZARGS.EXE').read_bytes(),
              ['A', 'B'], 'PIL MZ:  A B\n', 0, pil=True)
        # Self-modifying code: patches INC AX (0x40) at 0x0109 to INC CX (0x41), exits with CL=1.
        check(args.engine.resolve(), args.runtime.resolve(), work, 'PROOFSMC.COM',
              bytes.fromhex('b90000b041a209019040b44c88c8cd21'),
              [], '', 1, pil=True)
    print('Native application integration checks passed.')


if __name__ == '__main__':
    main()
