#!/usr/bin/env python3
"""Profile a DOS executable, then build all its observed blocks together."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('program', type=Path)
    parser.add_argument('--engine', type=Path)
    parser.add_argument('--verify', action='store_true', help='run generated guard/state tests; for Mandelbrot, compare full VGA output too')
    args = parser.parse_args()
    program = args.program.resolve()
    if program.suffix.lower() not in ('.com', '.exe') or not program.is_file():
        parser.error('program must be an existing DOS .COM or .EXE')
    compiler = subprocess.check_output(['rustc', '-vV'], text=True)
    host = next(line.split(': ', 1)[1] for line in compiler.splitlines() if line.startswith('host: '))
    cfg = subprocess.check_output(['rustc', '--print', 'cfg', '--target', host], text=True)
    settings = dict(line.split('=', 1) for line in cfg.splitlines() if '=' in line)
    target_os, arch = (settings[key].strip('"') for key in ('target_os', 'target_arch'))
    suffix = '.exe' if target_os == 'windows' else ''
    digest = hashlib.sha256(program.read_bytes()).hexdigest()
    directory = ROOT / 'shadow' / f'{target_os}-{arch}' / digest
    directory.mkdir(parents=True, exist_ok=True)
    build = ROOT / 'target/dos-shadow-build' / digest
    # The profiling engine must remain an interpreter build, separate from
    # the linked native build. Never overwrite an engine currently running.
    env = os.environ.copy()
    env.pop('PIL_SHADOW_SOURCE', None)
    env.pop('CARGO_BUILD_TARGET', None)
    env['CARGO_TARGET_DIR'] = str(ROOT / 'target')
    if args.engine:
        engine = args.engine.resolve()
    else:
        subprocess.run(['cargo', 'build', '--release', '--locked', '--target', host], cwd=ROOT, env=env, check=True)
        engine = ROOT / 'target' / host / 'release' / f'pil{suffix}'
    with tempfile.TemporaryDirectory(prefix='profile-', dir=directory) as temporary:
        source = Path(temporary) / 'backend.rs'
        result = subprocess.run([str(engine), '--emit-dos-backend', str(program), str(source)], cwd=ROOT, text=True, stdout=subprocess.PIPE, check=True)
        print(result.stdout, end='', flush=True)
        source.replace(directory / 'backend.rs')
    env['PIL_SHADOW_SOURCE'] = str(directory / 'backend.rs')
    env['CARGO_TARGET_DIR'] = str(build)
    command = ['cargo', 'build', '--release', '--locked', '--target', host]
    subprocess.run(command, cwd=ROOT, env=env, check=True)
    built = build / host / 'release' / f'pil{suffix}'
    native = directory / f'pil-{program.stem.lower()}{suffix}'
    pending = directory / f'engine.pending{suffix}'
    shutil.copyfile(built, pending)
    shutil.copymode(built, pending)
    pending.replace(native)
    manifest = {'format': 1, 'target_os': target_os, 'target_cpu': arch, 'target_triple': host,
                'guest_name': program.name, 'guest_sha256': digest, 'rustc': compiler,
                'source': 'backend.rs', 'source_sha256': hashlib.sha256((directory / 'backend.rs').read_bytes()).hexdigest(),
                'engine': native.name, 'engine_sha256': hashlib.sha256(native.read_bytes()).hexdigest(),
                'profiling_result': result.stdout.strip(), 'execution': 'observed compiled blocks plus interpreter fallback; linked Dine service',
                'automatic_loading': False}
    (directory / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    if args.verify:
        subprocess.run(['cargo', 'test', '--release', '--locked', '--target', host, 'shadow_backend::guard_tests', '--', '--nocapture'], cwd=ROOT, env=env, check=True)
        if program.name.upper() == 'MANDVGA.COM':
            subprocess.run(['python', 'tools/verify_dos_shadow.py', '--native-engine', str(native), '--engine', str(engine), '--program', str(program)], cwd=ROOT, check=True)
    print(f'Shadow service: {native}', flush=True)
    print('Use with tools/dos_terminal.py --engine PATH --headless PROGRAM', flush=True)


if __name__ == '__main__':
    main()
