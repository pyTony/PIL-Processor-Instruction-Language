#!/usr/bin/env python3
"""Compile a DOS application's prepared calls against the shared Dine library.

Unlike the byte-loader launcher, this emits an application address dispatcher
and native-form arguments ahead of time. Relocation is completed by the compiler.
Executable resources remain data; code loaded or modified later uses the shared
reference path. Inspect measured coverage before claiming complete native code.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def run(command):
    subprocess.run([str(arg) for arg in command], cwd=ROOT, check=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input', type=Path)
    parser.add_argument('--output-dir', type=Path, default=ROOT / 'programs' / 'compiled')
    parser.add_argument('--engine', type=Path, help='existing application compiler executable')
    parser.add_argument('--runtime', type=Path, help='existing shared Dine library')
    parser.add_argument('--pil', action='store_true',
                        help='emit whole-block PIL-lowered Rust (opt-level=3) instead of PILAPP02 metadata')
    args = parser.parse_args(argv)
    source = args.input.resolve(strict=True)
    if not source.is_file():
        parser.error('input must be a file')
    target = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target')).resolve()
    binary_suffix = '.exe' if os.name == 'nt' else ''
    library_name = 'dine.dll' if os.name == 'nt' else 'libdine.dylib' if sys.platform == 'darwin' else 'libdine.so'
    engine = args.engine.resolve(strict=True) if args.engine else target / 'release' / ('pil' + binary_suffix)
    runtime = args.runtime.resolve(strict=True) if args.runtime else target / 'release' / library_name
    if runtime.name != library_name:
        parser.error('runtime must retain its standard shared-library filename')
    output = args.output_dir.resolve()
    executable = output / ('Native_' + source.stem + binary_suffix)
    metadata = executable.with_suffix('.native.json')
    if executable.exists() or metadata.exists():
        parser.error('native application output already exists; choose a new output directory')
    destination = output / runtime.name
    if destination.exists() and destination.resolve() != runtime.resolve() and hashlib.sha256(destination.read_bytes()).digest() != hashlib.sha256(runtime.read_bytes()).digest():
        parser.error('output directory contains a different shared runtime; choose a new output directory')
    if args.engine is None:
        run(['cargo', 'build', '--release', '--locked', '--bin', 'pil'])
    if args.runtime is None:
        run(['cargo', 'build', '--release', '--locked', '--lib', '--features', 'dine-library'])
    identity = hashlib.sha256(source.read_bytes()).hexdigest()
    build = target / 'native-applications' / (source.stem + '-' + identity[:16])
    build.mkdir(parents=True, exist_ok=True)
    rust_source = build / 'application.rs'
    prepared = rust_source.with_suffix('.pilapp')
    # Only disposable compiler output under this exact build directory is replaced.
    for path in (rust_source, prepared, rust_source.with_suffix('.pilmap'), rust_source.with_suffix('.pilimg')):
        if path.exists():
            path.unlink()
    emit_flag = '--emit-native-pil' if args.pil else '--emit-native-application'
    run([engine, emit_flag, source, rust_source])
    output.mkdir(parents=True, exist_ok=True)
    staged_executable = build / ('application' + binary_suffix)
    if args.pil:
        if rust_source.stat().st_size > 200_000:
            opt_flags = ['-C', 'opt-level=2', '-C', 'codegen-units=8']
        else:
            opt_flags = ['-C', 'opt-level=3', '-C', 'lto=fat', '-C', 'codegen-units=1']
    else:
        opt_flags = ['-C', 'opt-level=s', '-C', 'lto=fat', '-C', 'codegen-units=1']
    command = ['rustc', '--edition=2021', '--crate-name', 'native_application',
               *opt_flags, '-C', 'strip=symbols', '-C', 'panic=abort',
               '-L', 'native=' + str(runtime.parent),
               rust_source, '-o', staged_executable]
    if os.name != 'nt':
        command += ['-C', 'link-arg=-Wl,-rpath,@loader_path' if sys.platform == 'darwin'
                    else 'link-arg=-Wl,-rpath,$ORIGIN']
    run(command)
    if destination.resolve() != runtime.resolve():
        if not destination.exists():
            shutil.copy2(runtime, destination)
    shutil.copy2(staged_executable, executable)
    record = {
        'format': 1,
        'input_name': source.name,
        'input_sha256': identity,
        'application': executable.name,
        'shared_runtime': runtime.name,
        'startup_relocation': 'completed during compilation',
        'application_dispatch': 'native address dispatch to shared native CPU forms',
        'original_file_required_at_runtime': False,
        'retained_executable_resources': True,
        'original_input_bytes_retained_as_resources': True,
        'modified_or_generated_code': 'separate shared interpreter path',
        'complete_native_coverage': 'must be measured; not asserted by this build',
    }
    metadata.write_text(json.dumps(record, indent=2) + '\n', encoding='utf-8')
    print(executable)
    return 0


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1)
