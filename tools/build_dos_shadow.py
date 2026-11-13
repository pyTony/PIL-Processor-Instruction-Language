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
    parser.add_argument('--output-dir', type=Path, help='separate shadow directory for experiments')
    parser.add_argument('--steps', type=int, default=2_000_000, help='profiling step budget (default 2000000; maximum 120000000)')
    parser.add_argument('--sites', type=int, default=512, help='maximum compiled instruction sites (default 512 hottest sites)')
    parser.add_argument('--keys', type=Path, help='ASCII training keystrokes')
    parser.add_argument('--profile-clock', type=int, metavar='UTC_SECONDS',
                        help='enable real-time BIOS/PIT timer interrupts during profiling; offset in seconds east of UTC')
    parser.add_argument('--train-mount', type=Path, metavar='DIR',
                        help='copy game files to a disposable writable DOS drive for isolated training')
    parser.add_argument('--compare-vga', action='store_true',
                        help='with --verify: run the guest to its first key wait in both engines, compare the full '
                             'session state including VGA mode 12h, then Enter must exit to text mode')
    parser.add_argument('--verify', action='store_true', help='run the generated guard/state tests')
    args = parser.parse_args()
    if args.compare_vga and not args.verify:
        parser.error('--compare-vga needs --verify')
    if not 1 <= args.steps <= 120_000_000 or not 1 <= args.sites <= 65_536:
        parser.error('--steps must be 1..120000000 and --sites 1..65536')
    if args.profile_clock is not None and not -50_400 <= args.profile_clock <= 50_400:
        parser.error('--profile-clock must be -50400..50400 seconds')
    if args.train_mount is not None and not args.train_mount.is_dir():
        parser.error('--train-mount must be a directory')
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
    if args.output_dir:
        directory = args.output_dir.resolve()
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
        print(f'Profiling {program.name}: at most {args.steps:,} steps; retaining {args.sites} sites', flush=True)
        profile = [str(engine), '--emit-dos-backend', str(program), str(source), '--steps', str(args.steps), '--sites', str(args.sites)]
        if args.profile_clock is not None:
            profile += ['--clock-offset', str(args.profile_clock)]
        if args.train_mount is not None:
            # DOS folder mounts permit writes. Never expose the original folder
            # to a profiling guest, even if that guest modifies or deletes files.
            train_copy = Path(temporary) / 'dos-drive'
            shutil.copytree(args.train_mount.resolve(), train_copy, symlinks=True)
            profile += ['--mount', str(train_copy)]
        if args.keys:
            profile += ['--keys', str(args.keys.resolve())]
        result = subprocess.run(profile, cwd=ROOT, text=True, stdout=subprocess.PIPE, check=True)
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
    manifest = {'format': 1, 'target_os': target_os, 'target_cpu': arch, 'target_triple': host,
                'guest_name': program.name, 'guest_sha256': digest, 'rustc': compiler,
                'source': 'backend.rs', 'source_sha256': hashlib.sha256((directory / 'backend.rs').read_bytes()).hexdigest(),
                'engine': native.name, 'engine_sha256': hashlib.sha256(pending.read_bytes()).hexdigest(),
                'profiling_result': result.stdout.strip(), 'execution': 'observed compiled blocks plus interpreter fallback; linked Dine service',
                'profiling_steps': args.steps, 'max_sites': args.sites,
                'profile_clock_offset_seconds': args.profile_clock,
                'training_mount_used': args.train_mount is not None,
                'automatic_loading': False, 'guard_tests_verified': args.verify}
    if args.verify:
        subprocess.run(['cargo', 'test', '--release', '--locked', '--target', host, 'shadow_backend::guard_tests', '--', '--nocapture'], cwd=ROOT, env=env, check=True)
        if args.compare_vga:
            subprocess.run(['python', 'tools/verify_dos_shadow.py', '--native-engine', str(pending), '--engine', str(engine), '--program', str(program)], cwd=ROOT, check=True)
    # Publish only after requested verification passes. A failed build/test
    # must not replace the last usable engine.
    pending.replace(native)
    (directory / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    print(f'Shadow service: {native}', flush=True)
    print('Use with tools/dos_terminal.py --engine PATH --headless PROGRAM', flush=True)


if __name__ == '__main__':
    main()
