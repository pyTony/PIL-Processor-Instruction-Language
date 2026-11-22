#!/usr/bin/env python3
"""Compile original COM/MZ fixtures and run native applications without inputs.

Checks the native executable's shared-library path and protocol, including zero
interpreter fallback for these fixtures. It does not certify arbitrary binaries.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def check(engine, runtime, work, name, data, tail, expected, exit_code):
    source = work / name
    source.write_bytes(data)
    output = work / 'output'
    subprocess.run([sys.executable, str(ROOT / 'tools' / 'compile_application.py'),
                    str(source), '--engine', str(engine), '--runtime', str(runtime),
                    '--output-dir', str(output)], cwd=ROOT, check=True)
    suffix = '.exe' if os.name == 'nt' else ''
    executable = output / ('Native_' + source.stem + suffix)
    metadata = json.loads(executable.with_suffix('.native.json').read_text())
    assert metadata['original_file_required_at_runtime'] is False
    assert metadata['retained_executable_resources'] is True
    assert 'shared_runtime_dir' in metadata and metadata['shared_runtime_dir']
    assert not Path(metadata['shared_runtime_dir']).is_absolute(), metadata['shared_runtime_dir']
    assert not (output / runtime.name).exists(), f'{runtime.name} should not be copied by default'
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

        # Verify dine.latest.json pointer resolution via DINE_RUNTIME=<dir>.
        suffix = '.exe' if os.name == 'nt' else ''
        proofc = work / 'output' / f'Native_PROOFC{suffix}'
        config = work / 'empty.conf'
        latest_dir = work / 'latest-dir'
        version_dir = latest_dir / 'builds' / 'latest'
        version_dir.mkdir(parents=True)
        shutil.copy2(args.runtime.resolve(), version_dir / args.runtime.name)
        (latest_dir / 'dine.latest.json').write_text(json.dumps({
            'format': 1,
            'shared_runtime': args.runtime.name,
            'shared_runtime_dir': 'builds/latest',
        }) + '\n', encoding='utf-8')
        env_latest = {**os.environ, 'DINE_TEXT': 'headless', 'DINE_PROTOCOL': '0',
                      'PILDOS_CONFIG': str(config), 'DINE_RUNTIME': str(latest_dir)}
        result = subprocess.run([str(proofc)], cwd=work, env=env_latest,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
        assert result.returncode == 42, (result.returncode, result.stderr)
        assert result.stdout.replace(b'\r\n', b'\n') == b'Native proof\n', result.stdout

        # Verify --copy-runtime standalone mode (works even without .native.json) and --force.
        standalone_src = work / 'PROOFC.COM'
        standalone_src.write_bytes(bytes.fromhex('ba0c01b409cd21b82a4ccd21') + b'Native proof\r\n$')
        standalone_out = work / 'standalone'
        subprocess.run([sys.executable, str(ROOT / 'tools' / 'compile_application.py'),
                        str(standalone_src), '--engine', str(args.engine.resolve()),
                        '--runtime', str(args.runtime.resolve()), '--copy-runtime',
                        '--output-dir', str(standalone_out)], cwd=ROOT, check=True)
        subprocess.run([sys.executable, str(ROOT / 'tools' / 'compile_application.py'),
                        str(standalone_src), '--engine', str(args.engine.resolve()),
                        '--runtime', str(args.runtime.resolve()), '--copy-runtime', '--force',
                        '--output-dir', str(standalone_out)], cwd=ROOT, check=True)
        standalone_src.unlink()
        standalone_exe = standalone_out / f'Native_PROOFC{suffix}'
        standalone_meta = standalone_exe.with_suffix('.native.json')
        assert json.loads(standalone_meta.read_text())['shared_runtime_dir'] == '.'
        assert (standalone_out / args.runtime.name).is_file()
        standalone_meta.unlink()
        env_plain = {**os.environ, 'DINE_TEXT': 'headless', 'DINE_PROTOCOL': '0',
                     'PILDOS_CONFIG': str(config)}
        result = subprocess.run([str(standalone_exe)], cwd=work, env=env_plain,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
        assert result.returncode == 42, (result.returncode, result.stderr)
        assert result.stdout.replace(b'\r\n', b'\n') == b'Native proof\n', result.stdout
    print('Native application integration checks passed.')


if __name__ == '__main__':
    main()
