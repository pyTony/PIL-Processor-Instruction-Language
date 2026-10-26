#!/usr/bin/env python3
"""Reproduce the original monitor test image; no external assembler or code."""
from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
code=bytearray();listing=[]
def emit(text,assembly):
    data=bytes.fromhex(text);listing.append(f'{0x100+len(code):04x}  {data.hex(" "):<26} {assembly}')
    code.extend(data)
def expect():
    emit('74 04','JE next check (skip failure stop)')
    emit('ba ad de','MOV DX,0xdead ; failure')
    emit('f4','HLT ; monitor stop')
emit('b8 ff ff','MOV AX,0xffff');emit('b9 01 00','MOV CX,1');emit('01 c8','ADD AX,CX')
emit('0f 92 c3','SETB BL');emit('0f 94 c7','SETE BH');emit('ba 01 01','MOV DX,0x0101');emit('39 d3','CMP BX,DX');expect()
emit('39 c8','CMP AX,CX ; CF=1');emit('11 c8','ADC AX,CX');emit('ba 02 00','MOV DX,2');emit('39 d0','CMP AX,DX');expect()
emit('39 d1','CMP CX,DX ; CF=1');emit('19 c8','SBB AX,CX');emit('ba 00 00','MOV DX,0');emit('39 d0','CMP AX,DX');expect()
emit('b0 f0','MOV AL,0xf0');emit('b1 0f','MOV CL,0x0f');emit('20 c8','AND AL,CL');emit('0f 94 c3','SETE BL');emit('b1 01','MOV CL,1');emit('38 cb','CMP BL,CL');expect()
emit('b9 03 00','MOV CX,3');emit('ba 01 00','MOV DX,1');emit('29 d1','SUB CX,DX');emit('75 fc','JNE -4 ; three iterations')
emit('66 b8 78 56 34 12','MOV EAX,0x12345678');emit('66 bb 00 04 00 00','MOV EBX,0x400');emit('be 00 00','MOV SI,0')
emit('66 89 00','MOV dword DS:[BX+SI],EAX');emit('66 8b 10','MOV EDX,dword DS:[BX+SI]');emit('66 39 d0','CMP EAX,EDX');expect()
emit('bd 00 04','MOV BP,0x400');emit('66 89 46 00','MOV dword SS:[BP+0],EAX');emit('66 8b 56 00','MOV EDX,dword SS:[BP+0]');emit('66 39 d0','CMP EAX,EDX');expect()
emit('64 66 89 07','MOV dword FS:[BX],EAX');emit('64 66 8b 17','MOV EDX,dword FS:[BX]');emit('66 39 d0','CMP EAX,EDX');expect()
emit('66 be 02 00 00 00','MOV ESI,2');emit('67 66 89 44 b3 fc','MOV dword DS:[EBX+ESI*4-4],EAX');emit('67 66 8b 54 b3 fc','MOV EDX,dword DS:[EBX+ESI*4-4]');emit('66 39 d0','CMP EAX,EDX');expect()
emit('b9 01 00','MOV CX,1');emit('01 0f','ADD word DS:[BX],CX');emit('8b 17','MOV DX,word DS:[BX]');emit('b8 79 56','MOV AX,0x5679');emit('39 d0','CMP AX,DX');expect()
emit('e4 e9','IN AL,0xe9 ; wait for mock RX');emit('e6 e9','OUT 0xe9,AL ; mock TX')
emit('ba 0d 60','MOV DX,0x600d ; success');emit('f4','HLT ; monitor stop')
outputs={'selftest.hex':code.hex(' ')+'\n','selftest.lst':'\n'.join(listing)+'\n'}
for name,text in outputs.items():
    path=root/'programs'/name
    if '--check' in sys.argv:
        if path.read_text()!=text:raise SystemExit(f'stale fixture: {path}')
    else:path.write_text(text)
print(f'{len(code)} guest bytes; fixtures '+('checked' if '--check' in sys.argv else 'written'))
