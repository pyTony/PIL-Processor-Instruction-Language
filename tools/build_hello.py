#!/usr/bin/env python3
"""Build the original MOV AL / OUT 0xe9 / HLT Hello-world fixture."""
from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
code=bytearray();listing=[]
for byte in b'Hello, world!\n':
    ip=0x100+len(code)
    listing.extend([f'{ip:04x}  b0 {byte:02x}        MOV AL,0x{byte:02x}',f'{ip+2:04x}  e6 e9        OUT 0xe9,AL'])
    code.extend((0xb0,byte,0xe6,0xe9))
listing.append(f'{0x100+len(code):04x}  f4           HLT ; monitor stop');code.append(0xf4)
for name,text in {'hello.hex':code.hex(' ')+'\n','hello.lst':'\n'.join(listing)+'\n'}.items():
    path=root/'programs'/name
    if '--check' in sys.argv:
        if path.read_text()!=text:raise SystemExit(f'stale fixture: {path}')
    else:path.write_text(text)
print(f'{len(code)} Hello-world guest bytes; fixtures '+('checked' if '--check' in sys.argv else 'written'))
