"""Keep programs/dos/versions.sha256 current.

The launchers copy the project's own DOS programs (every file directly in
programs/dos) into the drive folder chosen by `own_programs` in pildos.conf
(default D:), so a pull reaches what DOS runs there. They replace a file
already in that folder only when its SHA-256 is listed here for that name:
an earlier version of ours. Anything else with the same name is kept.

This file lists, per name, the SHA-256 of the current file and of every
version git history has of programs/dos/NAME (and of the same name directly
in driveD/, which git tracked before November 2026). Text files are also
listed with CRLF line ends, as a Windows checkout without eol=lf wrote them.
Lines already in the manifest stay, so versions that never reached git can
be listed by hand (with a # comment above them).

Usage:
  python tools/dos_program_versions.py          rewrite the manifest (needs full history)
  python tools/dos_program_versions.py --check  fail if a current file's hash is missing
Runs on Windows and Linux with the standard library only.
"""
import hashlib
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FOLDER = ROOT / 'programs' / 'dos'
MANIFEST = FOLDER / 'versions.sha256'
BINARY_SUFFIXES = {'.com', '.exe'}


def current_files():
    """Name -> bytes for every file the launchers copy."""
    return {p.name: p.read_bytes() for p in sorted(FOLDER.iterdir())
            if p.is_file() and p.name != MANIFEST.name and not p.name.startswith('.')}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def variants(name, data):
    """The bytes as stored, and with CRLF line ends for text files."""
    yield data
    if Path(name).suffix.lower() not in BINARY_SUFFIXES and b'\0' not in data:
        crlf = data.replace(b'\r\n', b'\n').replace(b'\n', b'\r\n')
        if crlf != data:
            yield crlf


def read_manifest():
    """Comment lines and (hash, name) entries, in file order."""
    if not MANIFEST.exists():
        return []
    entries = []
    for number, line in enumerate(MANIFEST.read_text(encoding='utf-8').splitlines(), 1):
        if not line.strip() or line.startswith('#'):
            entries.append(line)
            continue
        parts = line.split(None, 1)
        if len(parts) != 2 or len(parts[0]) != 64:
            sys.exit(f'{MANIFEST.name}:{number}: expected SHA256  NAME')
        entries.append((parts[0].lower(), parts[1].strip()))
    return entries


def git(*args, binary=False):
    result = subprocess.run(['git', *args], cwd=ROOT, capture_output=True, check=True)
    return result.stdout if binary else result.stdout.decode('utf-8')


def history(names):
    """(hash, name) of every committed version of these names."""
    if git('rev-parse', '--is-shallow-repository').strip() == 'true':
        sys.exit('shallow clone: run git fetch --unshallow first')
    wanted = {n.upper(): n for n in names}
    blobs = set()
    raw = git('log', '--all', '--raw', '--no-abbrev', '--format=', '--', 'programs/dos', 'driveD')
    for line in raw.splitlines():
        if not line.startswith(':') or '\t' not in line:
            continue
        meta, path = line.split('\t', 1)
        new = meta.split()[3]
        parts = path.split('/')
        top = len(parts) == 3 and parts[:2] == ['programs', 'dos'] or len(parts) == 2 and parts[0] == 'driveD'
        if top and parts[-1].upper() in wanted and set(new) != {'0'}:
            blobs.add((new, wanted[parts[-1].upper()]))
    found = set()
    for blob, name in sorted(blobs):
        data = git('cat-file', 'blob', blob, binary=True)
        found.update((digest(v), name) for v in variants(name, data))
    return found


def main(argv):
    check = argv == ['--check']
    if argv and not check:
        sys.exit(__doc__)
    files = current_files()
    entries = read_manifest()
    listed = {e for e in entries if isinstance(e, tuple)}
    if check:
        missing = [n for n, d in files.items() if (digest(d), n) not in listed]
        if missing:
            sys.exit(f'{MANIFEST.relative_to(ROOT).as_posix()} lacks the current version of: '
                     + ', '.join(missing) + '\nRun: python tools/dos_program_versions.py')
        print(f'{len(files)} programs listed in {MANIFEST.name}')
        return
    new = set()
    for name, data in files.items():
        new.update((digest(v), name) for v in variants(name, data))
    new |= history(files)
    new -= listed
    header = [] if entries else [
        '# SHA-256 of every version of each file in programs/dos, current and earlier.',
        '# The launchers replace a same-named file on the own_programs drive only when',
        '# its hash is listed here. Maintained by tools/dos_program_versions.py.',
    ]
    lines = header + [e if isinstance(e, str) else f'{e[0]}  {e[1]}' for e in entries]
    lines += [f'{h}  {n}' for h, n in sorted(new, key=lambda e: (e[1].upper(), e[0]))]
    MANIFEST.write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    print(f'{MANIFEST.relative_to(ROOT).as_posix()}: {len(new)} new entries')


if __name__ == '__main__':
    main(sys.argv[1:])
