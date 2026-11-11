#!/usr/bin/env python3
"""Build the original MANDANSI.COM fixture without an assembler dependency."""
from pathlib import Path
import argparse
from dos_fixture import Image

ROOT = Path(__file__).resolve().parents[1]
WIDTH, HEIGHT, LIMIT = 79, 24, 15
SHADES = b' @@%%**++==--::.'
COLOURS = (30, 96, 96, 94, 94, 95, 95, 91, 91, 93, 93, 92, 92, 36, 34, 30)
HEADER = b'\x1b[0;97;40m\x1b[2J\x1b[HPIL native ANSI Mandelbrot\r\n'


def build():
    a = Image(['; Original 79x24 ANSI Mandelbrot, signed 8.8 fixed point.',
               "; Based on this project's original mandel.asm arithmetic.",
               '; Requires a 386 and an ANSI terminal (ANSI.SYS on DOS).',
               '; Build: nasm -f bin mandansi.asm -o MANDANSI.COM',
               'bits 16', 'org 100h'])
    def string(label):
        a.address('ba', label, '', f'mov dx,{label}')
        a.emit('b409 cd21', 'mov ah,9\n    int 21h')
    string('header')
    a.address('c706', 'cy', 'e0fe', 'mov word [cy],-288')
    a.address('c606', 'row', '18', 'mov byte [row],24')
    a.label('next_row')
    a.address('c706', 'cx_', '00fe', 'mov word [cx_],-512')
    a.address('c606', 'col', '4f', 'mov byte [col],79')
    a.label('next_col')
    a.emit('31f6 31ff b90f00', 'xor si,si\n    xor di,di\n    mov cx,15')
    a.label('iterate')
    a.emit('89f0 f7e8', 'mov ax,si\n    imul ax')
    a.branch('e8', 'fix', 'call fix')
    a.emit('89c3 89f8 f7e8', 'mov bx,ax\n    mov ax,di\n    imul ax')
    a.branch('e8', 'fix', 'call fix')
    a.emit('89da 01c2 81fa0004', 'mov dx,bx\n    add dx,ax\n    cmp dx,1024')
    a.branch('0f8f', 'pixel', 'jg near pixel')
    a.emit('29c3 89f0 f7ef', 'sub bx,ax\n    mov ax,si\n    imul di')
    a.branch('e8', 'fix', 'call fix')
    a.emit('d1e0', 'shl ax,1')
    a.address('0306', 'cy', '', 'add ax,[cy]')
    a.emit('89c7', 'mov di,ax')
    a.address('031e', 'cx_', '', 'add bx,[cx_]')
    a.emit('89de', 'mov si,bx')
    a.branch('e2', 'iterate', 'loop iterate', 1)
    a.label('pixel')
    a.emit('89cb d1e3 01cb d1e3 01cb', 'mov bx,cx\n    shl bx,1\n    add bx,cx\n    shl bx,1\n    add bx,cx')
    a.address('ba', 'palette', '', 'mov dx,palette')
    a.emit('01da b409 cd21', 'add dx,bx\n    mov ah,9\n    int 21h')
    a.address('8306', 'cx_', '0a', 'add word [cx_],10')
    a.address('fe0e', 'col', '', 'dec byte [col]')
    a.branch('0f85', 'next_col', 'jnz near next_col')
    string('crlf')
    a.address('8306', 'cy', '19', 'add word [cy],25')
    a.address('fe0e', 'row', '', 'dec byte [row]')
    a.branch('0f85', 'next_row', 'jnz near next_row')
    string('reset')
    a.emit('b8004c cd21', 'mov ax,4c00h\n    int 21h')
    a.label('fix')
    a.emit('88e0 88d4 c3', 'mov al,ah\n    mov ah,dl\n    ret')
    a.data('header', HEADER + b'$')
    a.data('crlf', b'\r\n$')
    a.data('reset', b'\x1b[0m\r\n$')
    a.data('palette', b''.join(b'\x1b[' + str(colour).encode() + b'm' + bytes([shade]) + b'$'
                              for colour, shade in zip(COLOURS, SHADES)))
    for label, data in [('cy', bytes(2)), ('cx_', bytes(2)), ('row', bytes(1)), ('col', bytes(1))]:
        a.data(label, data)
    return a.finish()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    binary, source = build()
    for name, data in [('MANDANSI.COM', binary), ('mandansi.asm', source.encode())]:
        path = ROOT / 'programs/dos' / name
        if args.check:
            assert path.read_bytes() == data, f'rebuild {path}'
        else:
            path.write_bytes(data)
    print(f'{"Verified" if args.check else "Built"} original ANSI Mandelbrot: {len(binary)} bytes, {WIDTH}x{HEIGHT}')


if __name__ == '__main__':
    main()
