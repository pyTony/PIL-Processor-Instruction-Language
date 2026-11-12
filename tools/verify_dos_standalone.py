#!/usr/bin/env python3
"""Build and run standalone DOS fixtures without Cargo or the guest files."""
import argparse
from pathlib import Path
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def condition_matrix():
    """Check every Jcc taken and untaken using an independent subtraction oracle."""
    def conditions(a, b):
        result = (a - b) & 0xffff
        cf, zf, sf = a < b, result == 0, bool(result & 0x8000)
        of = bool((a ^ b) & (a ^ result) & 0x8000)
        pf = (result & 255).bit_count() % 2 == 0
        return (of, not of, cf, not cf, zf, not zf, cf or zf,
                not cf and not zf, sf, not sf, pf, not pf,
                sf != of, sf == of, zf or sf != of, not zf and sf == of)

    operands = [(a, b) for a in (0, 1, 0x7fff, 0x8000, 0xffff)
                for b in (0, 1, 0x7fff, 0x8000, 0xffff)]
    code, patches = bytearray(), []
    for condition in range(16):
        for expected in (False, True):
            a, b = next((a, b) for a, b in operands if conditions(a, b)[condition] == expected)
            code += b'\xb8' + a.to_bytes(2, 'little')
            code += b'\xbb' + b.to_bytes(2, 'little') + b'\x39\xd8'
            # Near Jcc goes to a shared failure exit only for the wrong result.
            code += bytes((0x0f, 0x80 + (condition ^ int(expected))))
            patches.append(len(code))
            code += b'\0\0'
    code += bytes.fromhex('b8004c cd21')
    failure = len(code)
    code += bytes.fromhex('b8ff4c cd21')
    for at in patches:
        code[at:at + 2] = (failure - at - 2).to_bytes(2, 'little', signed=True)
    return bytes(code)


def memory_sum_demo():
    """Sum five words from guest RAM, then select a decimal string by the result."""
    code = bytearray.fromhex('be0000 b90500 31c0 0304 83c602 e2f9 89c3 d1e0 d1e0 01d8 89c2 81c20000 b409 cd21 b8004c cd21')
    data = bytes.fromhex('01000200030004000500')
    code[1:3] = (0x100 + len(code)).to_bytes(2, 'little')
    table = code.index(bytes.fromhex('81c20000')) + 2
    code[table:table + 2] = (0x100 + len(code) + len(data)).to_bytes(2, 'little')
    return bytes(code) + data + b''.join(f'{n:02}\r\n$'.encode('ascii') for n in range(16))


def memory_fixtures():
    """Original COM fixtures; expected values are independent of the PIL emitter."""
    def data_at_end(code, pointer_at, data, expected):
        code = bytearray.fromhex(code)
        code[pointer_at:pointer_at + 2] = (0x100 + len(code)).to_bytes(2, 'little')
        return bytes(code) + data, b'', expected

    return [
        data_at_end('a00000 b44c cd21', 1, b'\x29', 41),
        data_at_end('8a260000 8ac4 b44c cd21', 2, b'\x2a', 42),
        data_at_end('a10000 3d3412 0f95c0 b44c cd21', 1, bytes.fromhex('3412'), 0),
        data_at_end('66a10000 663d78563412 0f95c0 b44c cd21', 2, bytes.fromhex('78563412'), 0),
        data_at_end('660fb6060000 663d89000000 0f95c0 b44c cd21', 4, b'\x89', 0),
        data_at_end('660fbe060000 663d89ffffff 0f95c0 b44c cd21', 4, b'\x89', 0),
        data_at_end('be0000 b90500 31c0 0304 83c602 e2f9 b44c cd21', 1,
                    bytes.fromhex('01000200030004000500'), 15),
        # 32-bit SIB with a signed displacement, reading a dword at data+4.
        data_at_end('66bb00000000 66be02000000 66678b44b3fc 663dbebafeca 0f95c0 b44c cd21',
                    2, bytes.fromhex('00000000 bebafeca'), 0),
        # Address size 32, operand size 16: retain the upper EAX half.
        data_at_end('66bb00000000 66be02000000 66b80000cdab 678b44b3fc 663defbecdab 0f95c0 b44c cd21',
                    2, bytes.fromhex('00000000 efbe'), 0),
        data_at_end('bd0000 8b4600 3d3412 0f95c0 b44c cd21', 1, bytes.fromhex('3412'), 0),
        data_at_end('64a10000 3d3412 0f95c0 b44c cd21', 2, bytes.fromhex('3412'), 0),
        # 16-bit effective-address sum wraps before the segmented read.
        data_at_end('bbffff be0000 83c602 8a00 b44c cd21', 4, bytes.fromhex('002a'), 42),
        (memory_sum_demo(), b'15\r\n', 0),
    ]


def verify_read_faults(engine, directory):
    """Compile the actual emitted load and assert precise faults/live segments."""
    program, source = directory / 'faults.com', directory / 'faults.rs'
    program.write_bytes(bytes.fromhex('8b4600 648b4600 668b4600 b44c cd21'))
    subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], check=True)
    with source.open('a', encoding='utf-8') as file:
        file.write(r'''
#[cfg(test)] mod read_contract {
 use super::*;
 fn fault(mut c:dos_runtime::Context,kind:&str){
  c.regs[0]=0xabcdef01;c.flags=0xad7;c.undefined=0x800;
  let before=(c.regs,c.flags,c.undefined,c.eip,c.machine,c.ram.clone());
  let mut out=host_io::CaptureOutput::new(16384);
  let error=step(&mut c,&mut out).unwrap_err();assert!(error.contains(kind),"{error}");
  assert_eq!((c.regs,c.flags,c.undefined,c.eip,c.machine,c.ram),before);
  assert!(host_io::Output::bytes(&out).is_empty());
 }
 #[test] fn stack_segment_fault_is_precise(){
  let mut c=initial_context().unwrap();c.regs[5]=0xffff;fault(c,"StackSegment");
 }
 #[test] fn override_segment_fault_is_precise(){
  let mut c=initial_context().unwrap();c.eip=0x103;c.regs[5]=0xffff;fault(c,"GeneralProtection");
 }
 #[test] fn dword_segment_fault_is_precise(){
  let mut c=initial_context().unwrap();c.eip=0x107;c.regs[5]=0xfffd;fault(c,"StackSegment");
 }
 #[test] fn bus_fault_is_precise(){
  let mut c=initial_context().unwrap();c.ram.set_a20_enabled(true);c.regs[5]=0;c.machine.segments[2].base=c.ram.bytes().len() as u32;
  fault(c,"MemoryAccess");
 }
 #[test] fn exact_segment_end_and_readonly_ram_work_with_live_ss(){
  let mut c=initial_context().unwrap();c.regs[5]=0xfffe;c.regs[0]=0xabcd0000;
  c.machine.segments[2].base=0x20000;let at=0x2fffe;
  c.ram.store(at,MemWidth::Word,0x1234).unwrap();c.ram.protect(at,2).unwrap();
  let flags=c.flags;let machine=c.machine;let ram=c.ram.clone();
  let mut out=host_io::CaptureOutput::new(16384);assert_eq!(step(&mut c,&mut out).unwrap(),None);
  assert_eq!(c.regs[0],0xabcd1234);assert_eq!(c.eip,0x103);
  assert_eq!(c.flags,flags);assert_eq!(c.machine,machine);assert_eq!(c.ram,ram);
 }
 #[test] fn missing_live_segment_is_rejected(){
  let mut c=initial_context().unwrap();c.regs[5]=0;c.machine.segments[2].access=SegmentAccess::Unusable;
  fault(c,"InvalidContext");
 }
 #[test] fn override_uses_live_fs_instead_of_bp_default_ss(){
  let mut c=initial_context().unwrap();c.eip=0x103;c.regs[5]=0x200;
  c.machine.segments[2].base=0x30000;c.machine.segments[4].base=0x40000;
  c.ram.store(0x30200,MemWidth::Word,0x1234).unwrap();
  c.ram.store(0x40200,MemWidth::Word,0x5678).unwrap();
  let mut out=host_io::CaptureOutput::new(16384);step(&mut c,&mut out).unwrap();
  assert_eq!(c.regs[0]&0xffff,0x5678);
 }
 #[test] fn initial_a20_wrap_is_preserved(){
  let mut c=initial_context().unwrap();assert!(!c.ram.a20_enabled());
  c.machine.segments[2].base=0x100000;c.regs[5]=0x200;
  c.ram.store(0x200,MemWidth::Word,0x5678).unwrap();
  let mut out=host_io::CaptureOutput::new(16384);step(&mut c,&mut out).unwrap();
  assert_eq!(c.regs[0]&0xffff,0x5678);
 }
}
''')
    binary = directory / ('read-tests.exe' if os.name == 'nt' else 'read-tests')
    subprocess.run(['rustc', '--edition=2021', '-Awarnings', '--test', str(source), '-o', str(binary)], check=True)
    subprocess.run([str(binary)], check=True, timeout=30)


def memory_write_demo():
    """Fill a writable output buffer with A..J, then print it through AH09."""
    code = bytearray.fromhex('bf0000 b90a00 b041 8805 47 40 e2fa ba0000 b409 cd21 b8004c cd21')
    address = (0x100 + len(code)).to_bytes(2, 'little')
    code[1:3] = address
    pointer = code.index(b'\xba\0\0') + 1
    code[pointer:pointer + 2] = address
    return bytes(code) + b'..........\r\n$'


def write_fixtures():
    def build(hexcode, data, expected):
        # FEED marks each little-endian 16-bit reference to the trailing data.
        code = bytes.fromhex(hexcode)
        code = code.replace(bytes.fromhex('feed'), (0x100 + len(code)).to_bytes(2, 'little'))
        return code + data, b'', expected
    return [
        build('c606feed2a a0feed b44c cd21', b'\0', 42),
        build('b83412 a3feed a1feed 3d3412 0f95c0 b44c cd21', b'\0\0', 0),
        build('66c706feed78563412 66a1feed 663d78563412 0f95c0 b44c cd21', b'\0'*4, 0),
        build('bbfeed b005 8807 80070a 8a07 b44c cd21', b'\0', 15),
        build('b007 8606feed 3c2a 0f95c3 a0feed 3c07 0f95c0 08d8 b44c cd21', b'\x2a', 0),
        build('66bbfeed0000 66be02000000 66b878563412 66678944b3fc 66678b44b3fc 663d78563412 0f95c0 b44c cd21', b'\0'*8, 0),
        build('bdfeed b83412 64894600 648b4600 3d3412 0f95c0 b44c cd21', b'\0\0', 0),
        (memory_write_demo(), b'ABCDEFGHIJ\r\n', 0),
    ]


def verify_write_faults(engine, directory):
    program, source = directory / 'stores.com', directory / 'stores.rs'
    program.write_bytes(bytes.fromhex('8907 830701 8707 894600 668907 c12700 b44c cd21'))
    subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], check=True)
    with source.open('a', encoding='utf-8') as file:
        file.write(r'''
#[cfg(test)] mod store_contract {
 use super::*;
 fn rejected(mut c:dos_runtime::Context,kind:&str){
  c.flags=0xad7;c.undefined=0x800;
  let before=(c.regs,c.flags,c.undefined,c.eip,c.machine,c.ram.clone());
  let mut out=host_io::CaptureOutput::new(16384);
  let error=step(&mut c,&mut out).unwrap_err();assert!(error.contains(kind),"{error}");
  assert_eq!((c.regs,c.flags,c.undefined,c.eip,c.machine,c.ram),before);
 }
 #[test] fn translated_code_rejects_stores_rmw_and_xchg_without_effects(){
  for eip in [0x100,0x102,0x105,0x10a]{
   let mut c=initial_context().unwrap();c.eip=eip;c.regs[3]=0x100;c.regs[0]=0xabcd1234;
   rejected(c,"MemoryAccess");
  }
 }
 #[test] fn same_value_and_partial_overlap_stores_are_rejected(){
  for offset in [0xff,0x100]{
   let mut c=initial_context().unwrap();c.regs[3]=offset;
   let at=c.machine.segments[3].base+offset;
   c.regs[0]=c.ram.load(at,MemWidth::Word).unwrap();
   rejected(c,"MemoryAccess");
  }
 }
 #[test] fn a20_alias_cannot_modify_translated_code(){
  let mut c=initial_context().unwrap();c.machine.segments[3].base+=0x100000;c.regs[3]=0x100;
  rejected(c,"MemoryAccess");
 }
 #[test] fn original_rom_and_runtime_readonly_spans_are_preserved(){
  let mut c=initial_context().unwrap();c.machine.segments[3].base=0xc0000;c.regs[3]=0;
  rejected(c,"MemoryAccess");
  let mut c=initial_context().unwrap();c.regs[3]=0x200;c.ram.protect(0x10200,2).unwrap();
  rejected(c,"MemoryAccess");
 }
 #[test] fn precise_ds_ss_dword_and_bus_faults(){
  let mut c=initial_context().unwrap();c.regs[3]=0xffff;rejected(c,"GeneralProtection");
  let mut c=initial_context().unwrap();c.eip=0x107;c.regs[5]=0xffff;rejected(c,"StackSegment");
  let mut c=initial_context().unwrap();c.eip=0x10a;c.regs[3]=0xfffd;rejected(c,"GeneralProtection");
  let mut c=initial_context().unwrap();c.ram.set_a20_enabled(true);
  c.machine.segments[3].base=c.ram.bytes().len() as u32;c.regs[3]=0;rejected(c,"MemoryAccess");
 }
 #[test] fn exact_segment_end_store_preserves_unwritten_bytes_and_registers(){
  let mut c=initial_context().unwrap();c.regs[0]=0xabcd1234;c.regs[3]=0xfffe;
  let regs=c.regs;let flags=c.flags;let machine=c.machine;let before=c.ram.bytes().to_vec();
  let at=c.machine.segments[3].base+0xfffe;
  let mut out=host_io::CaptureOutput::new(16384);assert_eq!(step(&mut c,&mut out).unwrap(),None);
  assert_eq!(c.ram.load(at,MemWidth::Word).unwrap(),0x1234);
  let mut expected=before;expected[at as usize..at as usize+2].copy_from_slice(&[0x34,0x12]);
  assert_eq!(c.ram.bytes(),expected);assert_eq!(c.regs,regs);assert_eq!(c.flags,flags);
  assert_eq!(c.machine,machine);assert_eq!(c.eip,0x102);
 }
 #[test] fn zero_count_shift_retains_existing_pil_store_checks(){
  let mut c=initial_context().unwrap();c.eip=0x10d;c.regs[3]=0x100;c.flags=0xad7;c.undefined=0x800;
  rejected(c,"MemoryAccess");
 }
}
''')
    binary = directory / ('store-tests.exe' if os.name == 'nt' else 'store-tests')
    subprocess.run(['rustc', '--edition=2021', '-Awarnings', '--test', str(source), '-o', str(binary)], check=True)
    subprocess.run([str(binary)], check=True, timeout=30)


def call_demo():
    code = bytearray.fromhex('ba0000 e80500 b8004c cd21 50 b409 cd21 58 c3')
    code[1:3] = (0x100 + len(code)).to_bytes(2, 'little')
    return bytes(code) + b'Hello from a subroutine!\r\n$'


def stack_fixtures():
    return [
        (bytes.fromhex('683412 58 3d3412 0f95c0 b44c cd21'), b'', 0),
        (bytes.fromhex('6aff 58 3dffff 0f95c0 b44c cd21'), b'', 0),
        (bytes.fromhex('66b878563412 6650 66b800000000 6658 663d78563412 0f95c0 b44c cd21'), b'', 0),
        (bytes.fromhex('89e3 54 58 39d8 0f95c0 b44c cd21'), b'', 0),
        (bytes.fromhex('68f0ff 5c 89e0 3df0ff 0f95c0 b44c cd21'), b'', 0),
        (bytes.fromhex('31c0 e80400 b44c cd21 40 c3'), b'', 1),
        (bytes.fromhex('31c0 66e804000000 b44c cd21 40 66c3'), b'', 1),
        (bytes.fromhex('31c0 e80400 b44c cd21 50 e80400 5b 01d8 c3 b80500 c3'), b'', 5),
        (bytes.fromhex('31c0 e80700 e80400 b44c cd21 40 c3'), b'', 2),
        (bytes.fromhex('6a07 e80400 b44c cd21 89e5 8b4602 c20200'), b'', 7),
        (bytes.fromhex('31c0 e80400 b44c cd21 66e802000000 40 c3 40 66c3'), b'', 2),
        (call_demo(), b'Hello from a subroutine!\r\n', 0),
    ]


def verify_stack_faults(engine, directory):
    program, source = directory / 'calls.com', directory / 'calls.rs'
    program.write_bytes(bytes.fromhex('e80a00 66e807000000 b44c cd21 50 5b c3 6650 665b 66c3'))
    subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], check=True)
    with source.open('a', encoding='utf-8') as file:
        file.write(r'''
#[cfg(test)] mod stack_contract {
 use super::*;
 fn rejected(mut c:dos_runtime::Context,kind:&str){
  c.flags=0xad7;c.undefined=0x800;
  let before=(c.regs,c.flags,c.undefined,c.eip,c.machine,c.ram.clone(),c.calls.clone());
  let mut out=host_io::CaptureOutput::new(16384);
  let error=step(&mut c,&mut out).unwrap_err();assert!(error.contains(kind),"{error}");
  assert_eq!((c.regs,c.flags,c.undefined,c.eip,c.machine,c.ram,c.calls),before);
 }
 #[test] fn call_pushes_return_before_callee_and_ret_restores_caller(){
  let mut c=initial_context().unwrap();let sp=c.regs[4];let flags=c.flags;
  let mut out=host_io::CaptureOutput::new(16384);
  step(&mut c,&mut out).unwrap();assert_eq!(c.eip,0x10d);assert_eq!(c.calls,vec![(0x103,16)]);
  assert_eq!(c.regs[4],sp-2);assert_eq!(c.ram.load(0x10000+sp-2,MemWidth::Word).unwrap(),0x103);
  for _ in 0..3{step(&mut c,&mut out).unwrap();}
  assert_eq!(c.eip,0x103);assert_eq!(c.regs[4],sp);assert!(c.calls.is_empty());assert_eq!(c.flags,flags);
 }
 #[test] fn wide_call_and_ret_keep_a_16_bit_stack(){
  let mut c=initial_context().unwrap();c.eip=0x103;let sp=c.regs[4];
  let mut out=host_io::CaptureOutput::new(16384);step(&mut c,&mut out).unwrap();
  assert_eq!(c.eip,0x110);assert_eq!(c.regs[4],sp-4);assert_eq!(c.calls,vec![(0x109,32)]);
  assert_eq!(c.ram.load(0x10000+sp-4,MemWidth::Dword).unwrap(),0x109);
  for _ in 0..3{step(&mut c,&mut out).unwrap();}
  assert_eq!(c.eip,0x109);assert_eq!(c.regs[4],sp);assert!(c.calls.is_empty());
 }
 #[test] fn failed_call_does_not_push_metadata_or_commit_any_state(){
  for (sp,kind) in [(1,"StackSegment"),(0x102,"MemoryAccess")]{
   let mut c=initial_context().unwrap();c.regs[4]=sp;rejected(c,kind);
  }
 }
 #[test] fn push_and_pop_faults_keep_state(){
  let mut c=initial_context().unwrap();c.eip=0x10d;c.regs[4]=0x102;rejected(c,"MemoryAccess");
  let mut c=initial_context().unwrap();c.eip=0x10e;c.regs[4]=0xffff;rejected(c,"StackSegment");
 }
 #[test] fn ret_faults_and_unproven_targets_keep_state_and_call_metadata(){
  let mut c=initial_context().unwrap();c.eip=0x10f;c.regs[4]=0xffff;c.calls.push((0x103,16));
  rejected(c,"StackSegment");
  let mut c=initial_context().unwrap();c.eip=0x10f;c.regs[4]=0x200;c.calls.push((0x103,16));
  c.ram.store(0x10200,MemWidth::Word,0x109).unwrap();rejected(c,"unproven standalone return target");
  let mut c=initial_context().unwrap();c.eip=0x10f;rejected(c,"no proven caller");
  let mut c=initial_context().unwrap();c.eip=0x114;c.calls.push((0x103,16));rejected(c,"operand widths differ");
 }
}
''')
    binary = directory / ('stack-tests.exe' if os.name == 'nt' else 'stack-tests')
    subprocess.run(['rustc', '--edition=2021', '-Awarnings', '--test', str(source), '-o', str(binary)], check=True)
    subprocess.run([str(binary)], check=True, timeout=30)


def argument_demo():
    # Read PSP:80 count, skip its leading space and emit the remaining bytes.
    return bytes.fromhex('31c9 8a0e8000 e30f 49 e30c be8200 8a14 b402 cd21 46 e2f7 b20d b402 cd21 b20a cd21 b8004c cd21')


def verify_arguments(engine, directory):
    from dos_terminal import Session
    program, source = directory / 'arguments.com', directory / 'arguments.rs'
    binary = directory / ('arguments.exe' if os.name == 'nt' else 'arguments')
    program.write_bytes(argument_demo())
    subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], check=True)
    subprocess.run(['rustc', '--edition=2021', '-Awarnings', '-O', str(source), '-o', str(binary)], check=True)
    cases = [[], [''], ['Hello', 'PIL'], ['two words'], ['a', '', 'b'], ['x'*125], ['  leading and trailing  '],
             ['tämä', 'on', 'demo'], ['ä'*125], ['ÅÄÖ åäö'], ['ÇüéαΩπ'], ['A\u00a0B']]
    for args in cases:
        text = ' '.join(args)
        session = Session(engine, program, args=text if args else None, stop_log='')
        try:
            frame = session.send('run 10000')
            assert frame['status'] == 'exit', frame
            assert frame['output'].encode('utf-8') == text.encode('utf-8') + b'\r\n', frame
        finally:
            session.close()
    program.unlink(); source.unlink()
    env = os.environ.copy(); env.pop('PIL_DOS_RAW_OUTPUT', None)
    for args in cases:
        result = subprocess.run([str(binary), *args], capture_output=True, timeout=10, env=env)
        assert (result.returncode, result.stdout, result.stderr) == (0, ' '.join(args).encode('utf-8') + b'\r\n', b''), result
    env['PIL_DOS_RAW_OUTPUT'] = '1'
    for args in cases:
        result = subprocess.run([str(binary), *args], capture_output=True, timeout=10, env=env)
        assert (result.returncode, result.stdout, result.stderr) == (0, ' '.join(args).encode('cp437') + b'\r\n', b''), result
    for args in [['x'*126], ['bad\nline'], ['🙂'], ['€']]:
        result = subprocess.run([str(binary), *args], capture_output=True, timeout=10)
        assert result.returncode == 1 and result.stdout == b'' and b'command line' in result.stderr, result
    print(f'PASS: {len(cases)} Unicode/CP437 command-line cases match Dine; UTF-8 and raw CP437 output checked')


def verify_shared_blocks(engine, directory):
    program, source = directory / 'blocks.com', directory / 'blocks.rs'
    program.write_bytes(bytes.fromhex('b80100 40 bbffff 8b17 b44c cd21'))
    subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], check=True)
    with source.open('a', encoding='utf-8') as file:
        file.write(r'''
#[cfg(test)] mod block_contract {
 use super::*;
 #[test] fn block_fault_preserves_retired_prefix_and_faulting_instruction(){
  let mut a=initial_context().unwrap();let mut b=initial_context().unwrap();
  a.regs[0]=0xabcd0000;b.regs[0]=0xabcd0000;
  let mut x=host_io::CaptureOutput::new(16384);let mut y=host_io::CaptureOutput::new(16384);
  for _ in 0..3{assert_eq!(step(&mut a,&mut x).unwrap(),None);}
  let expected=step(&mut a,&mut x).unwrap_err();let actual=block_step(&mut b,&mut y).unwrap_err();
  assert_eq!(actual,expected);assert!(actual.contains("GeneralProtection"));
  assert_eq!(b.eip,0x107);assert_eq!(b.regs[0],0xabcd0002);
  assert_eq!((a.regs,a.flags,a.undefined,a.eip,a.machine,a.ram,a.calls),
             (b.regs,b.flags,b.undefined,b.eip,b.machine,b.ram,b.calls));
 }
 #[test] fn interior_entry_executes_only_its_single_instruction(){
  let mut c=initial_context().unwrap();c.eip=0x103;c.regs[0]=7;
  let mut out=host_io::CaptureOutput::new(16384);
  assert_eq!(block_step(&mut c,&mut out).unwrap(),None);
  assert_eq!(c.eip,0x104);assert_eq!(c.regs[0],8);assert_eq!(c.regs[3],0);
  assert_eq!(block_step(&mut c,&mut out).unwrap(),None);
  assert_eq!(c.eip,0x107);assert_eq!(c.regs[3],0xffff);
  assert!(block_step(&mut c,&mut out).unwrap_err().contains("GeneralProtection"));
 }
}
''')
    binary = directory / ('block-tests.exe' if os.name == 'nt' else 'block-tests')
    subprocess.run(['rustc', '--edition=2021', '-Awarnings', '--test', str(source), '-o', str(binary)], check=True)
    subprocess.run([str(binary)], check=True, timeout=30)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, default=ROOT / 'target/debug' / ('pil.exe' if os.name == 'nt' else 'pil'))
    args = parser.parse_args()
    engine = args.engine.resolve()
    (ROOT / 'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='pil-standalone-', dir=ROOT / 'target') as directory:
        directory = Path(directory)
        # AH09 returns AL='$'; the second fixture preserves that return as its
        # exit code. The third verifies generated arithmetic, not captured output.
        hello = bytes.fromhex('ba0c01 b409 cd21 b8004c cd21') + b'Hello, DOS!\r\n$'
        returned_al = bytes.fromhex('ba0b01 b409 cd21 b44c cd21') + b'AL return\r\n$'
        arithmetic = bytes.fromhex('b80500 83c007 b44c cd21')
        fixtures = [
            (hello, b'Hello, DOS!\r\n', 0),
            (returned_al, b'AL return\r\n', 0x24),
            (arithmetic, b'', 12),
            (bytes.fromhex('cd20'), b'', 0),
            # A backwards conditional branch must be discovered without
            # profiling and executed repeatedly by the generated program.
            (bytes.fromhex('b90300 49 75fd cd20'), b'', 0),
            # Print from a loop, then return AH09's AL value as the exit code.
            (bytes.fromhex('b90300 ba1001 b409 cd21 e2fa b44c cd21') + b'X$', b'XXX', 0x24),
            # Sum 1..10 using LOOP; the actual result becomes the exit code.
            (bytes.fromhex('b90a00 31c0 01c8 e2fc b44c cd21'), b'', 55),
            (condition_matrix(), b'', 0),
            # CX=0 while ECX=65536: JCXZ takes, address-size JECXZ does not.
            (bytes.fromhex('66b900000100 e305 b8ff4c cd21 67e305 b8004c cd21 b8ff4c cd21'), b'', 0),
            (bytes.fromhex('b80000 83f800 660f8405000000 b8ff4c cd21 b8004c cd21'), b'', 0),
            # Unconditional near jumps must not discover the skipped data.
            (bytes.fromhex('e90200 0fff cd20'), b'', 0),
            (bytes.fromhex('66e902000000 0fff cd20'), b'', 0),
        ]
        fixtures.extend(memory_fixtures())
        fixtures.extend(write_fixtures())
        fixtures.extend(stack_fixtures())
        fixtures.extend([
            (bytes.fromhex('b258 b402 cd21 b44c cd21'), b'X', 88),
            (bytes.fromhex('b209 b402 cd21 b44c cd21'), b' '*8, 32),
            (bytes.fromhex('b241 b402 cd21 b209 cd21 b44c cd21'), b'A' + b' '*7, 32),
            (bytes.fromhex('ba3000 b90a00 b402 cd21 42 e2fb b8004c cd21'), b'0123456789', 0),
        ])
        for index, (data, expected, code) in enumerate(fixtures):
            program = directory / f'input{index}.com'
            source = directory / f'compiled{index}.rs'
            binary = directory / (f'native{index}.exe' if os.name == 'nt' else f'native{index}')
            program.write_bytes(data)
            probe = subprocess.check_output([str(engine), '--dos-probe', str(program)], text=True)
            assert f'Exit({code})' in probe, probe
            escaped = expected.decode('ascii').replace('\r', '\\r').replace('\n', '\\n')
            if expected:
                assert f'OUTPUT (escaped bytes): {escaped}' in probe, probe
            subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], check=True)
            generated = source.read_text(encoding='utf-8')
            for forbidden in ('mod guest', 'enum Token', 'InstructionBackend', 'DecodeCache', 'include!(', 'include_str!('):
                assert forbidden not in generated, forbidden
            # A second emit must preserve the existing source.
            rejected = subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], capture_output=True)
            assert rejected.returncode != 0 and source.read_text(encoding='utf-8') == generated
            subprocess.run(['rustc', '--edition=2021', '-Awarnings', '-O', str(source), '-o', str(binary)], check=True)
            program.unlink()
            source.unlink()
            result = subprocess.run([str(binary)], cwd=directory, capture_output=True, timeout=10)
            assert (result.returncode, result.stdout, result.stderr) == (code, expected, b''), result
        verify_read_faults(engine, directory)
        verify_write_faults(engine, directory)
        verify_stack_faults(engine, directory)
        verify_shared_blocks(engine, directory)
        verify_arguments(engine, directory)
        for index, data in enumerate((bytes.fromhex('ebfe'), bytes.fromhex('b430 cd21'), bytes.fromhex('b8'), bytes.fromhex('90'))):
            program = directory / f'unsupported{index}.com'
            source = directory / f'rejected{index}.rs'
            program.write_bytes(data)
            result = subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], capture_output=True)
            assert result.returncode != 0 and not source.exists()
    print(f'PASS: {len(fixtures)} standalone applications match Dine output/exit without guest files; generated memory fault/segment/A20 tests pass')


if __name__ == '__main__':
    main()
