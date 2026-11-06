"""Small original DOS fixtures for CGA and mode-13h BIOS pixel access."""
from verify_dosbox_cpu import ComBuilder


def build_cga(negative=False):
    b = ComBuilder()
    cases = []

    def mov(reg, value):
        b.emit(bytes([0xb8 + reg])); b.word(value)

    def interrupt():
        b.emit(bytes.fromhex('cd10'))

    def check_al(value, fail):
        b.emit(bytes([0x3c, value])); b.branch(fail, True)

    def case(name, mode, x, y, color, segment, offset, byte_value):
        n = len(cases) + 1
        cases.append({'name': name})
        fail, done = f'fail_{n}', f'done_{n}'
        mov(0, mode); interrupt()
        mov(0, 0x0c00 | color); mov(3, 0); mov(1, x); mov(2, y); interrupt()
        mov(0, 0x0d00); interrupt(); check_al(color ^ int(negative and n == 1), fail)
        mov(0, segment); b.emit(bytes.fromhex('8ec0'))  # ES = video segment
        b.emit(bytes.fromhex('26a0')); b.word(offset); check_al(byte_value, fail)
        mov(0, 3); interrupt()
        b.puts(f'PASS {n:03d} {name}\r\n'); b.branch(done)
        b.label(fail); mov(0, 3); interrupt()
        b.puts(f'FAIL {n:03d} {name}\r\n')
        b.emit(bytes.fromhex('ff06')); b.ref('fail_count')
        b.label(done)

    case('cga_four_color_even', 4, 0, 0, 3, 0xb800, 0, 0xc0)
    case('cga_four_color_odd', 4, 319, 199, 2, 0xb800, 0x3f3f, 0x02)
    case('cga_mono_even', 6, 0, 0, 1, 0xb800, 0, 0x80)
    case('cga_mono_odd', 6, 639, 199, 1, 0xb800, 0x3f3f, 0x01)
    case('vga_eight_bit_pixel', 0x13, 319, 199, 0xaa, 0xa000, 63999, 0xaa)
    n = len(cases) + 1
    cases.append({'name': 'cga_bios_palette_state'})
    fail, done = f'fail_{n}', f'done_{n}'
    mov(0, 4); interrupt()
    mov(0, 0x0b00); mov(3, 0x0012); interrupt()
    mov(0, 0x0040); b.emit(bytes.fromhex('8ec0'))
    b.emit(bytes.fromhex('26a06600')); check_al(0x32, fail)
    mov(0, 0x0b00); mov(3, 0x0100); interrupt()
    b.emit(bytes.fromhex('26a06600')); check_al(0x12, fail)
    mov(0, 3); interrupt()
    b.puts(f'PASS {n:03d} cga_bios_palette_state\r\n'); b.branch(done)
    b.label(fail); mov(0, 3); interrupt()
    b.puts(f'FAIL {n:03d} cga_bios_palette_state\r\n')
    b.emit(bytes.fromhex('ff06')); b.ref('fail_count')
    b.label(done)
    b.puts(f'COMPLETE {len(cases)} CASES\r\n')
    b.emit(bytes.fromhex('a1')); b.ref('fail_count')
    b.emit(bytes.fromhex('09c07405b8014ccd21b8004ccd21'))
    return cases, b.finish()
