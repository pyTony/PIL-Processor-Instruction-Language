#!/usr/bin/env python3
"""Original integer guest fixtures, using existing 386 monitor instructions."""
from pathlib import Path
import math,sys
ROOT=Path(__file__).resolve().parents[1]
class Image:
    def __init__(self):self.code=bytearray();self.listing=[];self.labels={};self.fixups=[]
    def emit(self,text,assembly):
        data=bytes.fromhex(text);self.listing.append((len(self.code),data,assembly));self.code.extend(data)
    def mov(self,register,value):self.emit('66 '+bytes([0xb8+register]).hex()+' '+value.to_bytes(4,'little').hex(' '),f'MOV r{register},0x{value:x}')
    def label(self,name):self.labels[name]=len(self.code)
    def jne(self,label):self.fixups.append((len(self.code)+1,label));self.emit('75 00',f'JNE {label}')
    def finish(self):
        for at,label in self.fixups:
            displacement=self.labels[label]-(at+1)
            assert -128<=displacement<=127
            self.code[at]=displacement&255
        listing=''.join(f'{0x100+at:04x}  {self.code[at:at+len(data)].hex(" "):<24} {assembly}\n' for at,data,assembly in self.listing)
        return self.code.hex(' ')+'\n',listing
f=Image()
for r,v in [(6,1),(7,0x400),(3,1),(5,13),(0,1),(4,4)]:f.mov(r,v)
f.label('factorial')
f.emit('66 89 c2','MOV EDX,EAX ; previous factorial')
f.emit('66 89 f1','MOV ECX,ESI ; multiply by n using n additions')
f.mov(0,0)
f.label('multiply')
f.emit('66 01 d0','ADD EAX,EDX')
f.emit('66 29 d9','SUB ECX,EBX')
f.jne('multiply')
f.emit('66 89 05','MOV dword DS:[DI],EAX ; linear 0x500 onward')
f.emit('66 01 e7','ADD EDI,ESP ; next result slot')
f.emit('66 01 de','ADD ESI,EBX ; next n')
f.emit('66 39 ee','CMP ESI,EBP ; stop at n=13')
f.jne('factorial')
f.emit('f4','HLT ; expected EAX=479001600 (12!)')
l=Image()
for r,v in [(1,65536),(0,0),(3,1)]:l.mov(r,v)
l.label('count')
l.emit('66 01 d8','ADD EAX,EBX')
l.emit('66 29 d9','SUB ECX,EBX')
l.jne('count')
l.emit('f4','HLT ; expected EAX=65536 and ECX=0')
for name,image in [('factorials',f),('loop',l)]:
    text,listing=image.finish()
    for suffix,data in [('hex',text),('lst',listing)]:
        path=ROOT/'programs'/f'{name}.{suffix}'
        if '--check' in sys.argv:
            if path.read_text()!=data:raise SystemExit(f'stale fixture: {path}')
        else:path.write_text(data)
    print(f'{name}: {len(image.code)} guest bytes')
# Independent numeric reference, not an emulation of guest instructions.
assert [math.factorial(n) for n in range(1,13)]==[1,2,6,24,120,720,5040,40320,362880,3628800,39916800,479001600]
print('Reference: 1!..12!; loop 65536 iterations; 336 / 196611 completed guest steps')
