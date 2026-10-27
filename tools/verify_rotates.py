#!/usr/bin/env python3
"""Independent bit-at-a-time rotate/CF-control oracle for the PIL descriptions."""
from verify_descriptions import interpret,tokens


def expected(op,a,count,w,flags,undefined):
    n=count&31
    if n==0:return a,flags,undefined
    m=(1<<w)-1;r=a&m;carry=0
    for _ in range(n):
        if op=='rol':
            carry=r>>(w-1)
            r=((r*2)&m)|carry
        else:
            carry=r&1
            r=(r//2)|(carry<<(w-1))
    overflow=0
    if n==1:
        overflow=((r>>(w-1))^carry) if op=='rol' else ((r>>(w-1))^(r>>(w-2)))&1
    return (a&~m)|r,(flags&~0x801)|carry|(overflow<<11),(undefined&~0x801)|(0 if n==1 else 0x800)


def main():
    checked=0
    for op in ('rol','ror'):
        for w in (8,16,32):
            code=tokens(op,w);m=(1<<w)-1;sign=1<<(w-1)
            values=range(256) if w==8 else (0,1,3,m,sign,sign-1,0xa5a55a5a,sign+1)
            for a in values:
                a=((0xabcd0000&~m)|(a&m)) if w<32 else a
                for count in range(256):
                    for flags,undefined in ((0x202,0),(0xad7,0x8d5)):
                        actual=interpret(code,a,count,w,flags,undefined)
                        want=expected(op,a,count,w,flags,undefined)
                        assert actual==want,(op,w,hex(a),count,actual,want)
                        checked+=1
    for op,carry in (('clc',0),('stc',1)):
        for w in (8,16,32):
            code=tokens(op,w)
            for flags in (0,1,0x202,0xffffffff):
                for undefined in (0,0x8d5):
                    a=0x12345678
                    assert interpret(code,a,0,w,flags,undefined)==(a,(flags&~1)|carry,undefined&~1)
                    checked+=1
    print(f'PASS: independent Python checked {checked:,} rotate/CF-control description cases; no Rust or hardware claim.')


if __name__=='__main__':main()
