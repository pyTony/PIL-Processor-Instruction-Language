#!/usr/bin/env python3
"""Compile a DOS application's prepared calls against the shared Dine library.

Unlike the byte-loader launcher, this emits an application address dispatcher
and native-form arguments ahead of time. Relocation is completed by the compiler.
Executable resources remain data; code loaded or modified later uses the shared
reference path. Inspect measured coverage before claiming complete native code.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
_startup_spec = importlib.util.spec_from_file_location("pil_runtime_startup", Path(__file__).with_name("runtime_startup.py"))
runtime_startup = importlib.util.module_from_spec(_startup_spec)
_startup_spec.loader.exec_module(runtime_startup)


def run(command):
    subprocess.run([str(arg) for arg in command], cwd=ROOT, check=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input', type=Path)
    parser.add_argument('--output-dir', type=Path, default=ROOT / 'programs' / 'compiled')
    parser.add_argument('--engine', type=Path, help='existing application compiler executable')
    parser.add_argument('--runtime', type=Path, help='existing shared Dine library')
    parser.add_argument('--copy-runtime', action='store_true',
                        help='copy the shared library beside the native executable for standalone use')
    parser.add_argument('--force', action='store_true',
                        help='replace an existing native application output in the output directory')
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
    if (executable.exists() or metadata.exists()) and not args.force:
        parser.error('native application output already exists; choose a new output directory or pass --force')
    default_target = (ROOT / 'target').resolve()
    if args.engine is None:
        if target == default_target:
            engine = runtime_startup.ensure(ROOT, engine, 'cli')
        else:
            run(['cargo', 'build', '--release', '--locked', '--bin', 'pil'])
    if args.runtime is None:
        if target == default_target:
            runtime = runtime_startup.ensure(ROOT, runtime, 'library')
        else:
            run(['cargo', 'build', '--release', '--locked', '--lib', '--features', 'dine-library'])
    identity = hashlib.sha256(source.read_bytes()).hexdigest()
    build = target / 'native-applications' / (source.stem + '-' + identity[:16])
    build.mkdir(parents=True, exist_ok=True)
    rust_source = build / 'application.rs'
    prepared = rust_source.with_suffix('.pilapp')
    # Only disposable compiler output under this exact build directory is replaced.
    for path in (rust_source, prepared, rust_source.with_suffix('.pilmap')):
        if path.exists():
            path.unlink()
    run([engine, '--emit-native-application', source, rust_source])
    output.mkdir(parents=True, exist_ok=True)
    staged_executable = build / ('application' + binary_suffix)
    command = ['rustc', '--edition=2021', '--crate-name', 'native_application',
               '-C', 'opt-level=1', rust_source, '-o', staged_executable]
    run(command)
    if args.copy_runtime:
        destination = output / runtime.name
        if destination.resolve() != runtime.resolve():
            if destination.exists():
                same = hashlib.sha256(destination.read_bytes()).digest() == hashlib.sha256(runtime.read_bytes()).digest()
                if not same and not args.force:
                    parser.error('output directory contains a different shared runtime; choose a new output directory')
                if not same and args.force:
                    shutil.copy2(runtime, destination)
            else:
                shutil.copy2(runtime, destination)
            if target == default_target:
                runtime_startup.record_copy(ROOT, runtime, destination)
        shared_runtime_dir = '.'
    else:
        try:
            shared_runtime_dir = Path(os.path.relpath(runtime.parent, output)).as_posix()
        except ValueError:
            shared_runtime_dir = runtime.parent.as_posix()
    shutil.copy2(staged_executable, executable)
    record = {
        'format': 1,
        'input_name': source.name,
        'input_sha256': identity,
        'application': executable.name,
        'shared_runtime': runtime.name,
        'shared_runtime_dir': shared_runtime_dir,
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
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1)
