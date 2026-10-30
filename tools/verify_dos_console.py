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
    def tty(page, ch):
        mov(0, 0x0e00 | ch); mov(3, page << 8); interrupt(0x10)
    def cursor(page, fail, want):
        mov(0, 0x0300); mov(3, page << 8); interrupt(0x10); check(2, want, fail)
    def read_at(page, pos, fail, want):
        position(page, pos); mov(0, 0x0800); mov(3, page << 8); interrupt(0x10); check(0, want, fail)
    def teletype(fail):
        # AH0E follows the active page; BH is ignored by SeaBIOS and DOSBox-X.
        mov(0, 0x0501); interrupt(0x10)
        position(1, 0x0105)
        # BEL is left out: DOSBox-X 2024.03 advances the cursor, IBM/SeaBIOS do not.
        for ch, want in ((8, 0x0104), (13, 0x0100), (10, 0x0200)):
            tty(1, ch); cursor(1, fail, want)
        position(1, 0x184f)
        mov(0, 0x0978); mov(3, 0x014e); mov(1, 1); interrupt(0x10)
        tty(1, 0x51); cursor(1, fail, 0x1800)
        read_at(1, 0x174f, fail, 0x4e51); read_at(1, 0x1800, fail, 0x4e20)
        mov(0, 0x0500); interrupt(0x10)
    case('teletype_controls_wrap_and_scroll', teletype)
    def scroll(fail):
        mov(0, 0x0501); interrupt(0x10)
        mov(0, 0x0f00); interrupt(0x10); check(0, 0x5003, fail)
        b.emit(bytes.fromhex('81e300ff')); check(3, 0x0100, fail)  # AND BX,FF00
        position(1, 0x0103); mov(0, 0x0942); mov(3, 0x0117); mov(1, 1); interrupt(0x10)
        position(1, 0x0203); mov(0, 0x0941); mov(3, 0x0117); mov(1, 1); interrupt(0x10)
        mov(0, 0x0601); mov(3, 0x1f00); mov(1, 0x0103); mov(2, 0x0205); interrupt(0x10)
        read_at(1, 0x0103, fail, 0x1741); read_at(1, 0x0203, fail, 0x1f20)
        mov(0, 0x0701); mov(3, 0x2e00); mov(1, 0x0103); mov(2, 0x0205); interrupt(0x10)
        read_at(1, 0x0103, fail, 0x2e20); read_at(1, 0x0203, fail, 0x1741)
        mov(0, 0x0500); interrupt(0x10)
    case('active_page_query_and_window_scroll', scroll)
    def dos_input(fail):
        mov(0, 0x40); b.emit(bytes.fromhex('8ec0'))
        b.emit(bytes.fromhex('26c7061a001e0026c7061c001e00'))
        for value in (0x1e61, 0x4800):
            mov(0, 0x0500); mov(1, value); interrupt(0x16)
        for ax, want in ((0x0b00, 0x0bff), (0x0800, 0x0861), (0x0700, 0x0700), (0x0700, 0x0748), (0x0b00, 0x0b00)):
            mov(0, ax); interrupt(0x21); check(0, want, fail)
        mov(0, 0x0600); mov(2, 0x00ff); interrupt(0x21); check(0, 0x0600, fail)
        b.emit(bytes.fromhex('9c5a81e24000')); check(2, 0x40, fail)  # ZF set: nothing ready
        mov(0, 0x0500); mov(1, 0x2e63); interrupt(0x16)
        mov(0, 0x0c00); interrupt(0x21); check(0, 0x0c00, fail)
        mov(0, 0x0b00); interrupt(0x21); check(0, 0x0b00, fail)
    case('dos_status_read_extended_poll_and_flush', dos_input)
    def line_input(fail):
        # Keys are stuffed first; the echo lands in the compared output, ended by LF.
        mov(0, 0x40); b.emit(bytes.fromhex('8ec0'))
        b.emit(bytes.fromhex('26c7061a001e0026c7061c001e00'))
        for value in (0x1e61, 0x3062, 0x0e08, 0x2e63, 0x1c0d, 0x1e61, 0x3062, 0x2e63, 0x1c0d):
            mov(0, 0x0500); mov(1, value); interrupt(0x16)
        for size, words in ((8, (0x6102, 0x0d63)), (3, (0x6102, 0x0d62))):
            b.emit(bytes([0xc6, 0x06, 0x00, 0xf0, size]))  # MOV BYTE [F000],size
            mov(2, 0xf000); mov(0, 0x0a00); interrupt(0x21)
            mov(0, 0x0200); mov(2, 0x000a); interrupt(0x21)
            for offset, want in zip((0xf001, 0xf003), words):
                b.emit(bytes([0xa1])); b.word(offset); check(0, want, fail)  # MOV AX,[offset]
    case('dos_buffered_line_edit_and_full_buffer', line_input)
    def handle_write(fail):
        # AH40 to STDOUT writes CX bytes with no terminator; AH51/AH62 return the PSP.
        for i, ch in enumerate(b'w$z\r\n'):
            b.emit(bytes([0xc6, 0x06, i, 0xf0, ch]))  # MOV BYTE [F000+i],ch
        mov(0, 0x4000); mov(3, 1); mov(1, 5); mov(2, 0xf000); interrupt(0x21)
        check(0, 5, fail)
        b.emit(bytes.fromhex('9c5a81e20100')); check(2, 0, fail)  # CF clear
        b.emit(bytes.fromhex('8cda'))  # MOV DX,DS
        for ah in (0x51, 0x62):
            mov(0, ah << 8); mov(3, 0); interrupt(0x21)
            b.emit(bytes.fromhex('39d3')); b.branch(fail, True)  # CMP BX,DX
    case('dos_handle_write_and_psp_query', handle_write)
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
        assert report['passed'] == len(tests) - int(negative), report
        assert report['failed'] == int(negative), report
        assert f'STOP Exit({int(negative)})' in result.stdout, result.stdout
    print(f'PASS: {len(tests)} BIOS console fixtures and deliberate readback failure')
