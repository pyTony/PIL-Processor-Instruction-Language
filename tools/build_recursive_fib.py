#!/usr/bin/env python3
"""Build the original recursive Fibonacci COM and matching NASM source."""
import argparse
from pathlib import Path
from dos_fixture import Image

ROOT=Path(__file__).resolve().parents[1]
USAGE=b'Usage: RECURFIB [n]   prints F(0)..F(n), n <= 20 (default 12)\r\n'


def build():
    a=Image(['; Original recursive Fibonacci: two recursive calls, saved BX/CX.',
             '; 8086, DOS AH02/AH09, 16-bit stack. F(20)=6765.',
             'bits 16','org 100h'])
    def branch(code,label,text,width=2):a.branch(code,label,text,width)
    def string(label):
        a.address('ba',label,'',f'mov dx,{label}')
        a.emit('b409 cd21','mov ah,9\n    int 21h')
    a.emit('be8100 31db 31c9','mov si,81h\n    xor bx,bx\n    xor cx,cx')
    a.label('skip')
    a.emit('ac 3c20','lodsb\n    cmp al,32')
    branch('74','skip','je short skip',1)
    a.label('number')
    a.emit('2c30 3c09',"sub al,'0'\n    cmp al,9")
    branch('77','parsed','ja short parsed',1)
    a.emit('93 b40a f6e4 01c3 41 83f902','xchg ax,bx\n    mov ah,10\n    mul ah\n    add bx,ax\n    inc cx\n    cmp cx,2')
    branch('77','usage','ja short usage',1)
    a.emit('ac','lodsb')
    branch('eb','number','jmp short number',1)
    a.label('parsed')
    a.emit('0430 3c0d',"add al,'0'\n    cmp al,13")
    branch('75','usage','jne short usage',1)
    a.emit('09c9','or cx,cx')
    branch('75','have','jnz short have',1)
    a.emit('bb0c00','mov bx,12')
    a.label('have')
    a.emit('83fb14','cmp bx,20')
    branch('77','usage','ja short usage',1)
    a.address('891e','limit','','mov [limit],bx')
    a.emit('31ed','xor bp,bp')
    a.label('line')
    a.emit('89e8','mov ax,bp')
    branch('e8','fib','call fib')
    a.emit('89c7 89e8','mov di,ax\n    mov ax,bp')
    branch('e8','decimal','call decimal')
    string('equals')
    a.emit('89f8','mov ax,di')
    branch('e8','decimal','call decimal')
    string('crlf')
    a.address('3b2e','limit','','cmp bp,[limit]')
    branch('74','done','je short done',1)
    a.emit('45','inc bp')
    branch('eb','line','jmp short line',1)
    a.label('done')
    a.emit('b8004c cd21','mov ax,4c00h\n    int 21h')
    a.label('usage')
    string('help')
    a.emit('b8014c cd21','mov ax,4c01h\n    int 21h')
    a.label('fib')
    a.emit('83f801','cmp ax,1')
    branch('76','base','jbe short base',1)
    a.emit('53 51 89c3 48','push bx\n    push cx\n    mov bx,ax\n    dec ax')
    branch('e8','fib','call fib')
    a.emit('89c1 89d8 83e802','mov cx,ax\n    mov ax,bx\n    sub ax,2')
    branch('e8','fib','call fib')
    a.emit('01c8 59 5b','add ax,cx\n    pop cx\n    pop bx')
    a.label('base')
    a.emit('c3','ret')
    a.label('decimal')
    a.emit('50 53 51 52 31c9 bb0a00','push ax\n    push bx\n    push cx\n    push dx\n    xor cx,cx\n    mov bx,10')
    a.label('digit')
    a.emit('31d2 f7f3 52 41 09c0','xor dx,dx\n    div bx\n    push dx\n    inc cx\n    or ax,ax')
    branch('75','digit','jnz short digit',1)
    a.label('print')
    a.emit('5a 80c230 b402 cd21',"pop dx\n    add dl,'0'\n    mov ah,2\n    int 21h")
    branch('e2','print','loop print',1)
    a.emit('5a 59 5b 58 c3','pop dx\n    pop cx\n    pop bx\n    pop ax\n    ret')
    a.data('equals',b' = $');a.data('crlf',b'\r\n$');a.data('help',USAGE+b'$');a.data('limit',bytes(2))
    return a.finish()


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    binary,source=build()
    for name,data in [('RECURFIB.COM',binary),('recurfib.asm',source.encode())]:
        path=ROOT/'programs/dos'/name
        if args.check:assert path.read_bytes()==data,f'rebuild {path}'
        else:path.write_bytes(data)
    print(f'{"Verified" if args.check else "Built"} recursive Fibonacci: {len(binary)} bytes')


if __name__=='__main__':main()
