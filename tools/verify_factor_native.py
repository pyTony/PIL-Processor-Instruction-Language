#!/usr/bin/env python3
"""Compile original FACTOR.COM and compare exact output with Python and Dine."""
import argparse
import math
import os
from pathlib import Path
import subprocess
import tempfile

from dos_terminal import Session

ROOT = Path(__file__).resolve().parents[1]
USAGE = b'Usage: FACTOR [n]   prints 0! .. n! for n up to 99 (default 25)\r\n'


def reference(n):
    return ''.join(f'{i:2d}! = {math.factorial(i)}\r\n' for i in range(n + 1)).encode('ascii')


def fault_tests(engine, directory):
    program, source = directory / 'faults.com', directory / 'faults.rs'
    # LODSB; DIV/IDIV byte/word/dword; LODSW/LODSD; exit.
    program.write_bytes(bytes.fromhex('ac f6f3 f6fb f7f3 f7fb 66f7f3 66f7fb ad 66ad b44c cd21'))
    subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], check=True)
    with source.open('a', encoding='utf-8') as file:
        file.write(r'''
#[cfg(test)] mod scalar_faults {
 use super::*;
 fn rejected(mut c:dos_runtime::Context,kind:&str){
  c.flags=0xad7;c.undefined=0x800;
  let before=(c.regs,c.flags,c.undefined,c.eip,c.machine,c.ram.clone(),c.calls.clone());
  let mut out=host_io::CaptureOutput::new(16384);
  let error=step(&mut c,&mut out).unwrap_err();assert!(error.contains(kind),"{error}");
  assert_eq!((c.regs,c.flags,c.undefined,c.eip,c.machine,c.ram,c.calls),before);
 }
 #[test] fn division_by_zero_does_not_commit_or_reach_host_division(){
  for eip in [0x101,0x103,0x105,0x107,0x109,0x10c]{
   let mut c=initial_context().unwrap();c.eip=eip;c.regs[0]=0x1234;c.regs[3]=0;
   rejected(c,"Exception");
  }
 }
 #[test] fn unsigned_quotient_overflow_preserves_all_state(){
  for (eip,ax,dx) in [(0x101,256,0),(0x105,0,1),(0x109,0,1)]{
   let mut c=initial_context().unwrap();c.eip=eip;c.regs[0]=ax;c.regs[2]=dx;c.regs[3]=1;
   rejected(c,"Exception");
  }
 }
 #[test] fn signed_minimum_over_minus_one_preserves_all_state(){
  for (eip,ax,dx) in [(0x103,0xff80,0),(0x107,0x8000,0xffff),(0x10c,0x80000000,0xffffffff)]{
   let mut c=initial_context().unwrap();c.eip=eip;c.regs[0]=ax;c.regs[2]=dx;c.regs[3]=0xffffffff;
   rejected(c,"Exception");
  }
 }
 #[test] fn valid_division_results_match_integer_expectations(){
  for (eip,ax,dx,bx,want_ax,want_dx) in [
   (0x101,0x123403e8,0xabcdef01,10,0x12340064,0xabcdef01),
   (0x103,0x1234fff9,0xabcdef01,3,0x1234fffe,0xabcdef01),
   (0x105,0xabcd0000,0xcdef0001,1000,0xabcd0041,0xcdef0218),
   (0x107,0xabcdfff9,0xcdefffff,3,0xabcdfffe,0xcdefffff),
   (0x109,0,1,1000,4294967,296),
   (0x10c,0xfffffff9,0xffffffff,3,0xfffffffe,0xffffffff),
  ]{
   let mut c=initial_context().unwrap();c.eip=eip;c.regs[0]=ax;c.regs[2]=dx;c.regs[3]=bx;
   let mut out=host_io::CaptureOutput::new(16384);step(&mut c,&mut out).unwrap();
   assert_eq!((c.regs[0],c.regs[2]),(want_ax,want_dx),"at {eip:x}");
  }
 }
 #[test] fn lods_uses_df_and_preserves_unwritten_register_bits(){
  for (df,expected) in [(0,0x201),(0x400,0x1ff)]{
   let mut c=initial_context().unwrap();c.regs[6]=0xabcd0200;c.regs[0]=0x12345678;c.flags=0x202|df;
   c.ram.store(0x10200,MemWidth::Byte,0x9a).unwrap();let flags=c.flags;
   let mut out=host_io::CaptureOutput::new(16384);step(&mut c,&mut out).unwrap();
   assert_eq!(c.regs[0],0x1234569a);assert_eq!(c.regs[6],0xabcd0000|expected);assert_eq!(c.flags,flags);
  }
 }
 #[test] fn faulting_lods_keeps_accumulator_and_pointer(){
  for (eip,offset) in [(0x10f,0xffff),(0x110,0xfffd)]{
   let mut c=initial_context().unwrap();c.eip=eip;c.regs[6]=offset;c.regs[0]=0x12345678;
   rejected(c,"GeneralProtection");
  }
 }
}
''')
    binary = directory / ('fault-tests.exe' if os.name == 'nt' else 'fault-tests')
    subprocess.run(['rustc', '--edition=2021', '-Awarnings', '--test', str(source), '-o', str(binary)], check=True)
    subprocess.run([str(binary)], check=True, timeout=30)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, default=ROOT / 'target/debug' / ('pil.exe' if os.name == 'nt' else 'pil'))
    args = parser.parse_args()
    engine, program = args.engine.resolve(), ROOT / 'programs/dos/FACTOR.COM'
    with tempfile.TemporaryDirectory(prefix='factor-native-', dir=ROOT / 'target') as temp:
        directory = Path(temp)
        source = directory / 'factor.rs'
        binary = directory / ('factor.exe' if os.name == 'nt' else 'factor')
        subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], check=True)
        subprocess.run(['rustc', '--edition=2021', '-Awarnings', '-O', str(source), '-o', str(binary)], check=True)
        source.unlink()
        cases = [(None,25),('0',0),('1',1),('10',10),('25',25),('50',50),('99',99),('09',9),('   ',25),
                 ('100',None),('abc',None),('-1',None),('3x',None),('1 2',None),('ä',None)]
        for text, n in cases:
            expected, code = (USAGE, 1) if n is None else (reference(n), 0)
            result = subprocess.run([str(binary), *([] if text is None else [text])], capture_output=True, timeout=30)
            assert (result.returncode, result.stdout, result.stderr) == (code,expected,b''), (text,result)
            session = Session(engine, program, args=text, stop_log='')
            output = bytearray()
            try:
                for _ in range(20):
                    frame = session.send('run 1000000')
                    output.extend(frame['output'].encode('ascii'))
                    if frame['status'] != 'running': break
                assert frame['status']=='exit' and bytes(output)==expected,(text,frame.get('detail'))
            finally:
                session.close()
        fault_tests(engine,directory)
    print('PASS: FACTOR 0!..99!, default/edge/invalid arguments match Python and Dine; scalar loads and precise divide faults pass')


if __name__ == '__main__':
    main()
