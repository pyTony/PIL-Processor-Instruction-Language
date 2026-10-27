#!/usr/bin/env python3
"""Original executable fixtures and CLI checks, independent of Rust loader helpers."""
from pathlib import Path
import os,struct,subprocess,tempfile
root=Path(__file__).resolve().parents[1]

def cli(*args):
    return subprocess.check_output(['cargo','run','--quiet','--locked','--',*map(str,args)],cwd=root,text=True,stderr=subprocess.STDOUT)

with tempfile.TemporaryDirectory(prefix='pil-dos-') as temp:
    directory=Path(temp);com=directory/'hello world.com';exe=directory/'hello.exe'
    cli('--write-dos-hello','com',com);cli('--write-dos-hello','exe',exe)
    want=bytes.fromhex('ba0c01 b409 cd21 b8004c cd21')+b'Hello, DOS!\r\n$'
    assert com.read_bytes()==want
    data=exe.read_bytes();words=struct.unpack_from('<14H',data)
    assert words==(0x5a4d,len(data),1,0,2,0,0xffff,0,0xfffe,0,0,0,28,0)
    assert data[32:]==want
    text=cli('--dos-probe',com,exe)
    assert 'SUMMARY imported=2/2 exited=2' in text,text
    assert text.count('OUTPUT (escaped bytes): Hello, DOS!\\r\\n')==2,text
    assert text.count('completed=5 PIL=3 DOS_calls=2')==2,text
    # Independently authored MZ with one relocation and conventional startup.
    h=bytearray(32);struct.pack_into('<14H',h,0,0x5a4d,38,1,1,2,0,0xffff,0,0x100,0,0,0,28,0)
    struct.pack_into('<HH',h,28,1,0)
    gap=directory/'startup.exe';gap.write_bytes(h+bytes.fromhex('b80000 8ed8 f4'))
    text=cli('--dos-probe',gap)
    assert 'opcode 8e' in text and 'file_offset=Some(35)' in text and 'EAX=00001010' in text,text
    assert 'completed=1 PIL=1 DOS_calls=0' in text,text
    loop=directory/'loop.com';loop.write_bytes(bytes.fromhex('ebfe'))
    text=cli('--dos-probe','--steps','17',loop)
    assert 'STOP Budget' in text and 'completed=17 PIL=17 DOS_calls=0' in text,text
    # Fixture creation must refuse to overwrite any user file.
    result=subprocess.run(['cargo','run','--quiet','--locked','--','--write-dos-hello','com',str(com)],cwd=root,capture_output=True)
    assert result.returncode!=0 and com.read_bytes()==want
print('PASS: independent DOS CLI fixtures checked COM/MZ hello, relocation/startup gap, bounded loop and overwrite refusal; no external executables imported.')
