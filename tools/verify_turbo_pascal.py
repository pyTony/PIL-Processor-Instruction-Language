#!/usr/bin/env python3
"""Exercise a locally supplied Pascal IDE without publishing third-party files.

Use --native-executable for an ahead-of-time application importing Dine, or
--engine for the original DOS executable on the shared runtime. All writes,
settings, swap files and original Pascal fixtures stay in a fresh test folder.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from dos_terminal import Session, paste_keys, screen_rows

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ("program Proof;\r\nvar i: integer; total: longint;\r\nbegin\r\n"
          "  total := 0;\r\n  for i := 1 to 100 do total := total + i;\r\n"
          "  writeln('PIL native Pascal: ', total);\r\nend.\r\n")


class NativeSession(Session):
    def __init__(self, executable, directory, config):
        env = {**os.environ, 'PILDOS_CONFIG': str(config), 'DINE_PROTOCOL': '1'}
        self.process = subprocess.Popen(
            [str(executable), 'PROOF.PAS', '/SC:\\'], cwd=directory, env=env,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, encoding='utf-8', creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        self.seq = 0
        self.com1_out = b''
        self.last = self._read()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tp-bin', type=Path, required=True)
    choice = parser.add_mutually_exclusive_group(required=True)
    choice.add_argument('--native-executable', type=Path)
    choice.add_argument('--engine', type=Path)
    parser.add_argument('--out', type=Path, default=ROOT / 'target' / 'pascal-acceptance')
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix='session-', dir=args.out.resolve()))
    for name in ('TURBO.TPL', 'TURBO.TPH'):
        shutil.copy2(args.tp_bin / name, directory / name)
    source = directory / 'PROOF.PAS'
    source.write_bytes(SOURCE.encode('ascii'))
    config = directory / 'empty.conf'
    config.write_text('', encoding='utf-8')
    if args.native_executable:
        session = NativeSession(args.native_executable.resolve(strict=True), directory, config)
        assert not (directory / 'TURBO.EXE').exists()
    else:
        shutil.copy2(args.tp_bin / 'TURBO.EXE', directory / 'TURBO.EXE')
        session = Session(args.engine.resolve(strict=True), directory / 'TURBO.EXE',
                          mount=directory, args='PROOF.PAS /SC:\\', config=config, stop_log='')
    evidence = []

    def advance(phase, slices=2):
        for _ in range(slices):
            reply = session.send('run 1000000 buffered')
            assert reply['ok'], reply
            if reply['status'] != 'running':
                break
        reply = session.send('snapshot')
        rows = '\n'.join(text.rstrip() for text, _ in screen_rows(reply)).rstrip()
        entry = {'phase': phase, **{key: reply[key] for key in (
            'status', 'detail', 'steps', 'native_total_instructions',
            'interpreter_instructions')}, 'screen': rows}
        entry.update({key:value for key,value in reply.items() if key.startswith('prepared_reference_')})
        evidence.append(entry)
        print(json.dumps({key: value for key, value in entry.items() if key not in ('screen','prepared_reference_sites')}), flush=True)
        assert reply['status'] in ('running', 'exit'), entry
        return reply, rows

    def key(ascii_code, scan, modifiers=0):
        assert session.send(f'shift {modifiers}')['ok']
        assert session.send(f'key {ascii_code} {scan}')['ok']
        session.send('run 10000 buffered')
        assert session.send('shift 0')['ok']

    try:
        reply, rows = advance('startup')
        assert 'PROOF.PAS' in rows and 'program Proof;' in rows, rows
        for ascii_code, scan in paste_keys('{edited}'):
            key(ascii_code, scan)
        key(0, 60)  # F2: save.
        advance('saved-source', 1)
        assert source.read_bytes().startswith(b'{edited}program Proof;')
        for cycle in range(2):
            key(0, 102, 4)  # Ctrl+F9: compile/run.
            advance(f'compile-run-{cycle + 1}', 3)
            key(0, 108, 8)  # Alt+F5: inspect the actual user screen.
            _, rows = advance(f'expected-output-{cycle + 1}', 1)
            assert 'PIL native Pascal: 5050' in rows, rows
            key(13, 28)
            _, rows = advance(f'return-to-editor-{cycle + 1}', 1)
            assert 'PROOF.PAS' in rows and 'program Proof;' in rows, rows
        key(0, 45, 8)  # Alt+X.
        reply, _ = advance('clean-exit')
        assert reply['status'] == 'exit' and reply['detail'] == 'exit code 0', reply
        if args.native_executable:
            assert not (directory / 'TURBO.EXE').exists()
        print('PASS: source edited and saved; two compile/run/return cycles; clean exit.', flush=True)
    finally:
        session.close()
        (directory / 'evidence.json').write_text(
            json.dumps(evidence, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(directory / 'evidence.json')


if __name__ == '__main__':
    main()
