"""Original BIOS console fixtures. Guest code is identical in PIL and DOSBox-X.

No CP/M implementation is copied: that project's Console/AnsiScreen separation
informed the design. This checks the bounded mode-3/legacy keyboard profile.
"""
from verify_dosbox_cpu import ComBuilder


def build_console(negative=False):
    b = ComBuilder()
    tests = []
    def mov(reg, value):
        b.emit(bytes([0xb8+reg])); b.word(value)
    def interrupt(vector):
        b.emit(bytes([0xcd, vector]))
    def check(reg, expected, fail):
        b.emit(bytes([0x81, 0xf8+reg])); b.word(expected); b.branch(fail, True)
    def case(name, body):
        i = len(tests)+1
        tests.append(dict(name=name))
        fail, done = f'fail_{i}', f'done_{i}'
        body(fail)
        b.puts(f'PASS {i:03d} {name}\r\n'); b.branch(done)
        b.label(fail); b.puts(f'FAIL {i:03d} {name}\r\n')
        b.emit(bytes.fromhex('ff06')); b.ref('fail_count')
        b.label(done)
    def position(page, pos):
        mov(0, 0x0200); mov(3, page << 8); mov(2, pos); interrupt(0x10)
    def screen(fail):
        # Page 1 prevents DOS status output on page 0 disturbing the fixture.
        position(1, 0x184f)
        mov(0, 0x0941); mov(3, 0x011e); mov(1, 1); interrupt(0x10)
        mov(0, 0x0800); interrupt(0x10); check(0, 0x1e41 ^ int(negative), fail)
        mov(0, 0x0300); interrupt(0x10); check(2, 0x184f, fail)
    case('last_cell_attribute_cursor', screen)
    def direct(fail):
        position(1, 0)
        mov(0, 0xb800); b.emit(bytes.fromhex('8ec0')) # MOV ES,AX
        b.emit(bytes.fromhex('26c7060010582f')) # ES:1000 = 2f58
        mov(0, 0x0800); interrupt(0x10); check(0, 0x2f58, fail)
        mov(0, 0x0a42); mov(1, 1); interrupt(0x10)
        b.emit(bytes.fromhex('26a10010')); check(0, 0x2f42, fail)
    case('direct_video_memory_and_preserve_attribute', direct)
    def repeated(fail):
        position(1, 0x004f)
        mov(0, 0x0951); mov(3, 0x014e); mov(1, 3); interrupt(0x10)
        for pos in (0x004f, 0x0100, 0x0101):
            position(1, pos); mov(0, 0x0800); interrupt(0x10); check(0, 0x4e51, fail)
    case('repeat_across_row', repeated)
    def key(fail):
        # Reset classic BDA ring, independent of host keystrokes/startup state.
        mov(0, 0x40); b.emit(bytes.fromhex('8ec0'))
        b.emit(bytes.fromhex('26c7061a001e0026c7061c001e00'))
        for value in (0x1e61, 0x4800):
            mov(0, 0x0500); mov(1, value); interrupt(0x16)
            b.emit(bytes.fromhex('25ff00')); check(0, 0, fail)
        for value in (0x1e61, 0x4800):
            for _ in range(2):
                mov(0, 0x0100); interrupt(0x16)
                b.emit(bytes.fromhex('9c5a81e24000')); check(2, 0, fail)
                check(0, value, fail)
            mov(0, 0); interrupt(0x16); check(0, value, fail)
        mov(0, 0x0100); interrupt(0x16)
        b.emit(bytes.fromhex('9c5a81e24000')); check(2, 0x40, fail)
    case('keyboard_peek_consume_extended_empty', key)
    b.puts(f'COMPLETE {len(tests)} CASES\r\n')
    b.emit(bytes.fromhex('a1')); b.ref('fail_count')
    b.emit(bytes.fromhex('09c07405b8014ccd21b8004ccd21'))
    return tests, b.finish()


if __name__ == '__main__':
    import argparse
    import re
    import subprocess
    from pathlib import Path
    from verify_dosbox_cpu import validate_output
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    for negative in (False, True):
        tests, code = build_console(negative)
        guest = args.out / ('negative.com' if negative else 'console.com')
        guest.write_bytes(code)
        result = subprocess.run([str(args.engine.resolve()), '--dos-probe', str(guest.resolve())],
                                capture_output=True, text=True, check=True, timeout=30)
        (guest.with_suffix('.txt')).write_text(result.stdout, encoding='utf-8')
        match = re.search(r'^OUTPUT \(escaped bytes\): ?(.*)$', result.stdout, re.M)
        text = bytes(match.group(1), 'ascii').decode('unicode_escape') if match else ''
        report = validate_output(text, tests, 'PIL')
        expected_errors = ['PIL: FAIL 001 last_cell_attribute_cursor'] if negative else []
        assert report['errors'] == expected_errors, report
        assert report['passed'] == (3 if negative else 4), report
        assert report['failed'] == int(negative), report
        assert f'STOP Exit({int(negative)})' in result.stdout, result.stdout
    print('PASS: four BIOS console fixtures and deliberate readback failure')
