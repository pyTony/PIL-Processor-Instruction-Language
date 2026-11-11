#!/usr/bin/env python3
"""Verify local interpreter fallback after a code write in a linked PIL engine."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, default=ROOT / 'target/debug' / ('pil.exe' if os.name == 'nt' else 'pil'))
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='smc-policy-', dir=ROOT / 'target') as temp:
        directory = Path(temp)
        program, source = directory / 'SMC.COM', directory / 'backend.rs'
        # First visit: INC AX. Patch it to INC DX, then take a second visit.
        program.write_bytes(bytes.fromhex('b90200 40 49 7408 2ec606030142 ebf4 b44c cd21'))
        subprocess.run([str(args.engine.resolve()), '--emit-dos-backend', str(program), str(source), '--steps', '1000'], check=True)
        with source.open('a', encoding='utf-8') as file:
            file.write(r'''
#[cfg(test)] mod smc_policy {
 use super::*;
 use crate::dos::InstructionBackend;
 #[test] fn code_write_runs_changed_instruction_in_interpreter_and_keeps_other_native_sites(){
  let mut a=dos::load(ORIGINAL,"SMC.COM").unwrap();let mut b=a.clone();
  let mut x=crate::host_io::CaptureOutput::new(16384);let mut y=crate::host_io::CaptureOutput::new(16384);
  let mut backend=Backend::default();
  let left=dos::run_with_backend(a,100,&mut x,None).unwrap();
  let right=dos::run_with_backend(b,100,&mut y,Some(&mut backend)).unwrap();
  assert_eq!(left.stop,dos::Stop::Exit(1));assert_eq!(left.stop,right.stop);
  assert_eq!(left.loaded.cpu,right.loaded.cpu);assert_eq!(left.loaded.image,right.loaded.image);
  assert_eq!(left.loaded.ram.bytes(),right.loaded.ram.bytes());
  assert_eq!(right.loaded.cpu.regs[2]&0xffff,1);assert!(backend.block_stats().1>0);
  // Mutate the same instruction directly to inspect both guard decisions.
  a=dos::load(ORIGINAL,"SMC.COM").unwrap();let mut backend=Backend::default();
  let at=a.image.segments[1].base+0x103;
  a.ram.store(at,crate::MemWidth::Byte,0x42).unwrap();
  assert!(!backend.may_execute(&a.active_program,&a.image,&a.ram,0x103));
  assert!(backend.may_execute(&a.active_program,&a.image,&a.ram,0x104),"unchanged instruction on the same page stays eligible");
  a.cpu.eip=0x104;let d=std::rc::Rc::new(a.image.decode(&a.ram,a.cpu.eip).unwrap());
  let mut machine=a.image.segment_state();
  assert!(backend.execute(&a.active_program,&a.image,&d,&mut a.cpu,&mut a.ram,&mut machine).unwrap().is_ok());
 }
}
''')
        env = os.environ.copy()
        env['PIL_SHADOW_SOURCE'] = str(source.resolve())
        env['PIL_DISABLE_SHADOW'] = '1'
        env['CARGO_TARGET_DIR'] = str(ROOT / 'target/smc-policy-build')
        subprocess.run(['cargo', 'test', '--locked', 'shadow_backend::', '--', '--nocapture'], cwd=ROOT, env=env, check=True)
    print('PASS: code writes are allowed; changed instructions fall back locally and unchanged sites continue natively')


if __name__ == '__main__':
    main()
