#!/usr/bin/env python3
"""Independent DF expectations; Python description execution, no hardware claim."""
from verify_descriptions import interpret, tokens

def main():
    checked = 0
    for op, direction in (("cld", 0), ("std", 0x400)):
        for width in (8, 16, 32):
            program = tokens(op, width)
            for low in range(4096):
                flags = 0xa5a50000 | low
                for undefined in (0, 0x400, 0x8d5, 0xffffffff):
                    assert interpret(program, 0x12345678, 0, width, flags, undefined) == (
                        0x12345678, (flags & ~0x400) | direction, undefined & ~0x400)
                    checked += 1
    print(f"PASS: independent Python checked {checked:,} CLD/STD cases; no hardware claim.")

if __name__ == "__main__":
    main()
