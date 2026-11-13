"""Keep program-specific code out of PIL's general source files.

PIL's CPU, Dine services, BIOS/VGA modules and the DOS terminal must model
behaviour generally: they must not branch on, or be justified by, one guest
program (its name, file name, hash, PSP, command line or addresses). Program
history belongs in docs/, tests, fixtures and launchers.

This check scans the general files and fails on:
  * names of guest programs (third-party ones and the project's own demos),
  * string literals that name a DOS executable or source file (X.COM, X.EXE,
    X.BAS, ...),
  * 64-hex-digit literals (SHA-256 hashes of guest files).

Every script in tools/ is general unless it is a test, benchmark or fixture
builder (verify_*, bench_*, the demo builders). The precompilers (the mixed
shadow backend and the full standalone native translator, with their build
tools) are general files too: they compile any
program, so only their generated output, which is never in the repository, is
about one program. Exempt: Rust #[cfg(test)] items, docs, tests, fixtures
and launchers. Anything else must be listed in
tools/program_specific_allow.txt with a reason.

Usage: python tools/check_program_specific.py [--list]
Runs on Windows and Linux with the standard library only.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ALLOW = ROOT / 'tools' / 'program_specific_allow.txt'

# General files: everything under src/ and isa/, build.rs, and every script
# in tools/ (Python, PowerShell, shell, cmd): the terminal, the debugger, the
# precompiler build tools (Tony, 2026-11-13: precompilation, not a JIT, and
# not exempt) and the packaging scripts.
GENERAL = ['src/**/*.rs', 'isa/**/*.pil', 'build.rs',
           'tools/*.py', 'tools/*.ps1', 'tools/*.sh', 'tools/*.cmd', 'tools/*.bat']

# Scripts that are tests, benchmarks or fixture builders for one program by
# design: a test names what it tests. Matched against the file name.
TEST_SCRIPTS = re.compile(r'^(verify_.*|bench_.*|build_(ansi_mandel|recursive_fib|mz_demo|hello|numeric|selftest)|'
                          r'dos_fixture|check_program_specific)\.py$')

# Guest programs. Matched case-insensitively as whole words.
PROGRAMS = [
    # Third-party programs run under Dine.
    r'quake', r'doom', r'wolf(?:enstein|3d)?', r'keen', r'cwsdpmi', r'dos/?4gw',
    r'topbench', r'sysinfo', r'norton', r'turbo', r'borland', r'tpascal',
    r'gw-?basic', r'qbasic', r'bbc\s*basic', r'freecom', r'4dos', r'cube[23]',
    r'lines\.exe', r'landmark', r'checkit', r'dosbench', r'pkzip', r'arj',
    # The project's own demo programs.
    r'mand(?:vga|cga|cyc|bbc|ansi|elfp)', r'recurfib', r'timeit', r'mzargs',
]
NAME = re.compile(r'(?<![A-Za-z0-9_])(?:' + '|'.join(PROGRAMS) + r')(?![A-Za-z0-9_])', re.I)
FILE = re.compile(r'''["'][^"'\n]*?[A-Za-z0-9_~$-]+\.(?:com|exe|bas|bat|pas|sys|ovl)["'\\]''', re.I)
# Host executables (PIL itself, the Windows shell) are not guest programs.
HOST = re.compile(r'(?:^|[/\\"\'$])(?:pil|pwsh|powershell)\.exe["\']$', re.I)
HASH = re.compile(r'(?<![0-9A-Fa-f])[0-9A-Fa-f]{64}(?![0-9A-Fa-f])')


def load_allow():
    """Each line: PATH | TEXT | REASON. A finding is allowed when its line in
    PATH contains TEXT. Lines starting with # are comments."""
    allow = []
    for number, line in enumerate(ALLOW.read_text(encoding='utf-8').splitlines(), 1):
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        parts = [part.strip() for part in line.split('|')]
        if len(parts) != 3 or not all(parts):
            sys.exit(f'{ALLOW.name}:{number}: expected PATH | TEXT | REASON')
        allow.append((parts[0], parts[1], number))
    return allow


def strip_strings_and_comments(line):
    """Rust line without string/char literals and // comments, for brace counting."""
    line = re.sub(r'"(?:\\.|[^"\\])*"', '""', line)
    line = re.sub(r"'(?:\\.|[^'\\])'", "''", line)
    return line.split('//', 1)[0]


def rust_test_lines(lines):
    """Line numbers (0-based) inside #[cfg(test)] items."""
    skip = set()
    i = 0
    while i < len(lines):
        if lines[i].strip().startswith('#[cfg(test)]'):
            start = i
            depth = 0
            opened = False
            # A one-line attribute followed by an item (or the item on the same line).
            rest = lines[i].split('#[cfg(test)]', 1)[1]
            j = i
            text = rest
            while True:
                code = strip_strings_and_comments(text)
                depth += code.count('{') - code.count('}')
                opened = opened or '{' in code
                if (opened and depth <= 0) or (not opened and code.rstrip().endswith(';')):
                    break
                j += 1
                if j >= len(lines):
                    break
                text = lines[j]
            skip.update(range(start, j + 1))
            i = j + 1
        else:
            i += 1
    return skip


def scan():
    findings = []
    files = sorted({path for pattern in GENERAL for path in ROOT.glob(pattern) if path.is_file()})
    for path in files:
        rel = path.relative_to(ROOT).as_posix()
        if rel.startswith('tools/') and TEST_SCRIPTS.match(path.name):
            continue
        lines = path.read_text(encoding='utf-8').splitlines()
        skip = rust_test_lines(lines) if rel.endswith('.rs') else set()
        for number, line in enumerate(lines):
            if number in skip:
                continue
            for kind, pattern in (('program name', NAME), ('file name', FILE), ('hash', HASH)):
                for match in pattern.finditer(line):
                    if kind == 'file name' and HOST.search(match.group(0)):
                        continue
                    findings.append((rel, number + 1, kind, match.group(0), line.strip()))
    return findings


def self_test():
    """The patterns must catch the forms this check exists for, and skip test items."""
    catch = ['if name == "QUAKE.EXE" {', '// GW-BASIC needs this', 'if hash == "' + 'ab' * 32 + '"',
             "is_shell = 'COMMAND.COM' in program", '    // Turbo Pascal expects AX=0 here']
    skip = ['let lines = text.lines();', 'mode 12h has 480 lines', 'fn mandelbrot_guest()']
    for line in catch:
        assert any(p.search(line) for p in (NAME, FILE, HASH)), line
    for line in skip:
        assert not any(p.search(line) for p in (NAME, FILE, HASH)), line
    assert TEST_SCRIPTS.match('verify_dos_games.py') and not TEST_SCRIPTS.match('pil_debugger.py')
    for script in ('tools/pil_debugger.py', 'tools/build_dos_shadow.py', 'tools/dos_terminal.py'):
        assert script in {p.relative_to(ROOT).as_posix() for g in GENERAL for p in ROOT.glob(g)}, script
    rust = ['fn general() {}', '#[cfg(test)]', 'mod tests {', '    fn t() { load("X.COM"); }', '}', 'fn after() {}']
    assert rust_test_lines(rust) == {1, 2, 3, 4}, rust_test_lines(rust)
    assert rust_test_lines(['#[cfg(test)]', '#[path="x.rs"] mod tests;', 'fn f() {}']) == {0, 1}
    print('check_program_specific self-test passed')


def main(argv):
    if '--self-test' in argv:
        self_test()
    allow = load_allow()
    used = set()
    bad = []
    for rel, number, kind, text, line in scan():
        hit = next((entry for entry in allow if entry[0] == rel and entry[1] in line), None)
        if hit:
            used.add(hit[2])
            if '--list' in argv:
                print(f'allowed {rel}:{number}: {kind} {text!r}')
            continue
        bad.append(f'{rel}:{number}: {kind} {text!r}: {line[:120]}')
    stale = [entry for entry in allow if entry[2] not in used]
    for path, text, number in stale:
        bad.append(f'{ALLOW.name}:{number}: allow-list entry no longer matches ({path} | {text})')
    if bad:
        print('Program-specific code in general files (see CLAUDE.md, "No program-specific code"):')
        for entry in bad:
            print('  ' + entry)
        print('Model the DOS/BIOS/CPU behaviour generally, move program history to docs/ or a test,'
              ' or, for an interface name, add PATH | TEXT | REASON to tools/program_specific_allow.txt.')
        return 1
    print(f'No program-specific code in general files ({len(used)} allow-list entries used).')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
