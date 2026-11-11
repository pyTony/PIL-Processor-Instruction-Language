#!/usr/bin/env python3
"""Verify shared MZ loading, relocation, fixed-CS discovery and live segments."""
import argparse
import os
from pathlib import Path
import struct
import subprocess
import tempfile
from build_mz_demo import build, mz
from dos_terminal import Session

ROOT=Path(__file__).resolve().parents[1]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine',type=Path,default=ROOT/'target/debug'/('pil.exe' if os.name=='nt' else 'pil'))
    args=parser.parse_args();engine=args.engine.resolve()
    with tempfile.TemporaryDirectory(prefix='mz-native-',dir=ROOT/'target') as tmp:
        out=Path(tmp);program=out/'args.exe';program.write_bytes(build())
        source=out/'args.rs'
        # Keep input/output separate on Windows: the DOS EXE is never executed.
        binary=out/('native-args.exe' if os.name=='nt' else 'native-args')
        subprocess.run([str(engine),'--emit-dos-standalone',str(program),str(source)],check=True)
        subprocess.run(['rustc','--edition=2021','-Awarnings','-O',str(source),'-o',str(binary)],check=True)
        for text in [None,'Hello from PIL','tämä on demo','Çüé','   ']:
            expected=('PIL MZ: '+(' '+text if text is not None else '')+'\r\n').encode()
            result=subprocess.run([str(binary),*([text] if text is not None else [])],capture_output=True,timeout=30)
            assert (result.returncode,result.stdout,result.stderr)==(0,expected,b''),(text,result)
            session=Session(engine,program,args=text,stop_log='')
            try:
                frame=session.send('run 100000');assert frame['status']=='exit' and frame['output'].encode()==expected,frame
            finally:session.close()
        with source.open('a') as file:
            file.write(r'''
#[cfg(test)] mod mz_state {
 use super::*;
 #[test] fn relocation_live_segments_and_independent_stack(){
  let mut c=initial_context().unwrap();let psp=c.machine.selectors[3];
  assert_eq!(c.eip,3);assert_eq!(c.machine.selectors[1],psp+0x11);assert_eq!(c.machine.selectors[2],psp+0x30);
  let module=u32::from(psp+0x10)*16;let ds=psp+0x18;
  assert_eq!(c.ram.load(module+24,MemWidth::Word).unwrap(),u32::from(ds));
  // The relocated immediate is translated code; unreachable prefix stays data.
  assert!(c.ram.store(module+24,MemWidth::Byte,0).is_err());c.ram.store(module,MemWidth::Byte,0x90).unwrap();
  let initial_sp=c.regs[4];let mut output=host_io::CaptureOutput::new(16384);
  for _ in 0..5 {assert_eq!(step(&mut c,&mut output).unwrap(),None);}
  assert_eq!(c.regs[0]&255,u32::from(b'P'));assert_eq!(c.machine.selectors[3],ds);assert_eq!(c.machine.selectors[0],psp);
  for _ in 0..1000 {
   if let Some(code)=step(&mut c,&mut output).unwrap(){assert_eq!(code,0);assert_eq!(c.regs[4],initial_sp);assert!(c.calls.is_empty());return;}
  }
  panic!("MZ did not exit");
 }
}
''')
        test=out/('state.exe' if os.name=='nt' else 'state')
        subprocess.run(['rustc','--edition=2021','-Awarnings','-O','--test',str(source),'-o',str(test)],check=True)
        subprocess.run([str(test)],check=True)
        hello=out/'hello.exe'
        subprocess.run([str(engine),'--write-dos-hello','exe',str(hello)],check=True)
        raw=hello.read_bytes();wrapped=bytearray(raw);struct.pack_into('<H',wrapped,20,0x100);struct.pack_into('<H',wrapped,22,0xfff0)
        zm=bytearray(raw);zm[:2]=b'ZM'
        for n,data in enumerate([raw,wrapped,zm]):
            path=out/f'hello{n}.exe';path.write_bytes(data);rs=out/f'hello{n}.rs'
            subprocess.run([str(engine),'--emit-dos-standalone',str(path),str(rs)],check=True)
            subprocess.run(['rustc','--edition=2021','-Awarnings','-O',str(rs),'-o',str(binary)],check=True)
            result=subprocess.run([str(binary)],capture_output=True,timeout=30)
            assert (result.returncode,result.stdout,result.stderr)==(0,b'Hello, DOS!\r\n',b'')
        # Bad relocation, an unsupported new-format header and branches outside
        # the loaded module must fail before any Rust output is created.
        bad_reloc=bytearray(build());struct.pack_into('<H',bad_reloc,28,0xffff)
        pe=bytearray(128);pe[:2]=b'MZ'
        struct.pack_into('<HHHH',pe,2,128,1,0,4);struct.pack_into('<H',pe,12,64)
        struct.pack_into('<I',pe,60,64);pe[64:66]=b'PE'
        for n,data in enumerate([bad_reloc,pe,mz(bytes.fromhex('e9fcff')),mz(bytes.fromhex('e9fd7f'))]):
            path=out/f'bad{n}.exe';path.write_bytes(data);rs=out/f'bad{n}.rs'
            result=subprocess.run([str(engine),'--emit-dos-standalone',str(path),str(rs)],capture_output=True)
            assert result.returncode!=0 and not rs.exists(),(n,result)
    print('PASS: MZ/ZM and wrapped CS entries, relocation/live DS/ES, independent stack, Unicode arguments and invalid-image rejection')


if __name__=='__main__':main()
