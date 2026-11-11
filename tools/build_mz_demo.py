#!/usr/bin/env python3
"""Build an original relocated MZ argument demo with separate CS/DS/SS."""
import argparse
from pathlib import Path
import struct
from dos_fixture import Image

ROOT=Path(__file__).resolve().parents[1]
HEADER=b'PIL MZ: $'


def mz(module, *, cs=0, ip=0, ss=0, sp=0x100, relocations=()):
    """An ordinary DOS MZ container; relocation offsets refer to the module."""
    header_size=((28+4*len(relocations)+15)//16)*16
    header=bytearray(header_size);header[:2]=b'MZ';size=header_size+len(module)
    for at,value in [(2,size%512),(4,(size+511)//512),(6,len(relocations)),(8,header_size//16),
                     (10,64),(12,64),(14,ss),(16,sp),(20,ip),(22,cs),(24,28)]:
        struct.pack_into('<H',header,at,value)
    for n,(offset,segment) in enumerate(relocations):struct.pack_into('<HH',header,28+4*n,offset,segment)
    return bytes(header)+module


def build():
    # Module offset 19 is CS=1:IP=3. The data paragraph immediate at offset
    # 24 is relocated by DOS. ES retains the initial PSP to read arguments.
    a=Image();a.code.extend(b'\xcc'*19)
    a.emit('8cd8 8ec0 b80800 8ed8','mov ax,ds; mov es,ax; mov ax,data; mov ds,ax')
    a.emit('a00000','mov al,[0]')  # Exercises the native wrapper's live DS cache.
    a.branch('e8','print_header','call print_header')
    a.emit('be8100','mov si,81h')
    a.label('next')
    a.emit('26ac 3c0d','es lodsb; cmp al,13')
    a.branch('74','done','je done',1)
    a.emit('88c2 b402 cd21','mov dl,al; mov ah,2; int 21h')
    a.branch('eb','next','jmp next',1)
    a.label('done')
    a.emit('ba0900 b409 cd21 b8004c cd21','mov dx,9; mov ah,9; int 21h; mov ax,4c00h; int 21h')
    a.label('print_header')
    a.emit('ba0000 b409 cd21 c3','mov dx,0; mov ah,9; int 21h; ret')
    module,_=a.finish();assert len(module)<=128
    module=module.ljust(128,b'\xcc')+HEADER+b'\r\n$'
    return mz(module,cs=1,ip=3,ss=0x20,relocations=[(24,0)])


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    path=ROOT/'programs/dos/MZARGS.EXE';data=build()
    if args.check:assert path.read_bytes()==data,f'rebuild {path}'
    else:path.write_bytes(data)
    print(f'{"Verified" if args.check else "Built"} original relocated MZ: {len(data)} bytes')


if __name__=='__main__':main()
