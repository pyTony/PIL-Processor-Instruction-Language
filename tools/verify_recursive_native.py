#!/usr/bin/env python3
"""Check recursive DOS/native Fibonacci and generated guest-stack restoration."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile
from build_recursive_fib import USAGE, build
from dos_terminal import Session

ROOT=Path(__file__).resolve().parents[1]


def reference(n):
    a,b=0,1
    lines=[]
    for i in range(n+1):
        lines.append(f'{i} = {a}\r\n');a,b=b,a+b
    return ''.join(lines).encode()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine',type=Path,default=ROOT/'target/debug'/('pil.exe' if os.name=='nt' else 'pil'))
    args=parser.parse_args();engine=args.engine.resolve();program=ROOT/'programs/dos/RECURFIB.COM'
    binary,asm=build();assert program.read_bytes()==binary
    assert (ROOT/'programs/dos/recurfib.asm').read_text()==asm
    with tempfile.TemporaryDirectory(prefix='recursive-native-',dir=ROOT/'target') as tmp:
        out=Path(tmp);source=out/'recursive.rs';exe=out/('recursive.exe' if os.name=='nt' else 'recursive')
        subprocess.run([str(engine),'--emit-dos-standalone',str(program),str(source)],check=True)
        subprocess.run(['rustc','--edition=2021','-Awarnings','-O',str(source),'-o',str(exe)],check=True)
        for text,n in [(None,12),('0',0),('1',1),('2',2),('8',8),('12',12),('20',20),('  ',12),('21',None),('99',None),('100',None),('abc',None)]:
            expected,code=(USAGE,1) if n is None else (reference(n),0)
            result=subprocess.run([str(exe),*([text] if text is not None else [])],capture_output=True,timeout=30)
            assert (result.returncode,result.stdout,result.stderr)==(code,expected,b''),(text,result)
            session=Session(engine,program,args=text,stop_log='');captured=bytearray()
            try:
                for _ in range(30):
                    frame=session.send('run 1000000');captured.extend(frame['output'].encode())
                    if frame['status']!='running':break
                assert frame['status']=='exit' and captured==expected,(text,frame.get('detail'))
            finally:session.close()
        fib=0x100+binary.index(bytes.fromhex('83f80176'))
        with source.open('a') as file:
            file.write(r'''
#[cfg(test)] mod recursive_stack {
 use super::*;
 #[test] fn recursive_calls_restore_stack_and_saved_registers(){
  let mut c=initial_context().unwrap();let initial=c.regs[4];
  let mut output=host_io::CaptureOutput::new(16384);let mut depth=0;let mut frames=Vec::new();
  for _ in 0..1000000 {
   let prior=c.calls.len();
   if c.eip==FIB_ENTRY {frames.push((c.regs[3],c.regs[1],c.regs[4],prior));}
   let exit=step(&mut c,&mut output).unwrap();depth=depth.max(c.calls.len());
   if c.calls.len()<prior && frames.last().is_some_and(|f|f.3==prior){
    let (bx,cx,sp,_)=frames.pop().unwrap();assert_eq!((c.regs[3],c.regs[1]),(bx,cx));
    assert_eq!(c.regs[4],sp+2);
   }
   if let Some(code)=exit {assert_eq!(code,0);assert_eq!(c.regs[4],initial);assert!(c.calls.is_empty());assert!(frames.is_empty());assert!(depth>=12);return;}
  }
  panic!("recursive fixture did not exit");
 }
}
'''.replace('FIB_ENTRY',str(fib)))
        test=out/('stack.exe' if os.name=='nt' else 'stack')
        subprocess.run(['rustc','--edition=2021','-Awarnings','-O','--test',str(source),'-o',str(test)],check=True)
        subprocess.run([str(test)],check=True,timeout=30)
        # A separate 65-frame fixture proves discovery no longer depends on the
        # former 32-call analysis limit. The application above tests two calls.
        deep=out/'deep.com';deep.write_bytes(bytes.fromhex('b94000 e80200 cd20 83f900 7405 49 e8f7ff c3 c3'))
        deep_source=out/'deep.rs'
        subprocess.run([str(engine),'--emit-dos-standalone',str(deep),str(deep_source)],check=True)
        with deep_source.open('a') as file:
            file.write(r'''
#[cfg(test)] mod deep_stack {
 use super::*;
 #[test] fn sixty_five_frames_restore_sp(){
  let mut c=initial_context().unwrap();let initial=c.regs[4];let mut depth=0;
  let mut out=host_io::CaptureOutput::new(16384);
  for _ in 0..1000 {
   let exit=step(&mut c,&mut out).unwrap();depth=depth.max(c.calls.len());
   if let Some(code)=exit {assert_eq!(code,0);assert_eq!(depth,65);assert_eq!(c.regs[4],initial);assert_eq!(c.regs[1],0);assert!(c.calls.is_empty());return;}
  }
  panic!("deep recursion did not exit");
 }
}
''')
        subprocess.run(['rustc','--edition=2021','-Awarnings','-O','--test',str(deep_source),'-o',str(test)],check=True)
        subprocess.run([str(test)],check=True,timeout=30)
    print('PASS: recursive Fibonacci F(0)..F(20), default/invalid arguments match Python and Dine; nested guest stack/BX/CX restored')


if __name__=='__main__':main()
