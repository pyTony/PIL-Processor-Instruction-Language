#!/usr/bin/env python3
"""Independent exchange values; physical aliases/commit faults are Rust regressions."""
from verify_descriptions import tokens,interpret

def main():
    checked=0
    for w in (8,16,32):
        m=(1<<w)-1;sign=1<<(w-1);code=tokens('xchg',w)
        values=range(256) if w==8 else (0,1,m,sign,sign-1,0x12345678&m)
        for a in values:
            for b in values:
                aa=(0xabcd0000&~m)|a;bb=(0x12340000&~m)|b
                for flags,undefined in ((0x202,0),(0xad7,0x8d5)):
                    source=[]
                    got=interpret(code,aa,bb,w,flags,undefined,source_output=source)
                    assert got==((aa&~m)|b,flags,undefined)
                    assert source==[(bb&~m)|a]
                    checked+=1
    print(f'PASS: independent Python checked {checked:,} exchange binding cases; physical aliases and fault retirement are checked separately.')

if __name__=='__main__':main()
