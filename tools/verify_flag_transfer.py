#!/usr/bin/env python3
"""Independent packed-byte oracle; does not compile Rust or execute hardware."""
from verify_descriptions import interpret,tokens


def main():
    checked=0
    lahf,sahf=tokens('lahf',8),tokens('sahf',8)
    for low_flags in range(4096):
        flags=0xa5a50000|low_flags
        for undefined in (0,1,0x8d5,0xffffffff):
            actual=interpret(lahf,0x12345678,0,8,flags,undefined)
            assert actual==(0x12345600|(flags&0xd5)|2,flags,undefined)
            checked+=1
    for packed in range(256):
        for flags in (0,0x202,0xad7,0xffffffff):
            for undefined in (0,1,0x8d5,0xffffffff):
                actual=interpret(sahf,0x12345678,packed,8,flags,undefined)
                assert actual==(0x12345678,(flags&~0xd5)|(packed&0xd5),undefined&~0xd5)
                checked+=1
    print(f'PASS: independent Python checked {checked:,} LAHF/SAHF description cases; register aliases are checked by Rust tests; no hardware claim.')


if __name__=='__main__':main()
