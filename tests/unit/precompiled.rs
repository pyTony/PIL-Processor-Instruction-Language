use super::*;
use crate::dos::{self,Stop};
use crate::guest::Operation;
use crate::host_io::{CaptureOutput,Output};

fn pil_program(bytes:&[u8])->Program{
    let p=dos::load(bytes,"FORM.COM").unwrap();
    match p.image.decode(&p.ram,p.cpu.eip).unwrap().operation{Operation::Pil{program,..}=>program,o=>panic!("not PIL: {o:?}")}
}

#[test]
fn immediates_share_a_form_and_fixed_constants_are_checked(){
    if forms()==0{return;}
    // ADD AX,1234h and ADD AX,5678h: one form, two constant values.
    let a=resolve(&pil_program(&[0x05,0x34,0x12,0xcd,0x20])).expect("ADD AX,imm16 is precompiled");
    let b=resolve(&pil_program(&[0x05,0x78,0x56,0xcd,0x20])).expect("ADD AX,imm16 is precompiled");
    assert!(std::ptr::fn_addr_eq(a.form,b.form));
    assert_ne!(a.consts,b.consts);
    assert!(a.consts.contains(&0x1234)&&b.consts.contains(&0x5678));
    // PUSH AX: its stack step is a fixed literal in the form. A program
    // with another value there must not run that form.
    let mut push=pil_program(&[0x50,0xcd,0x20]);
    assert!(resolve(&push).is_some());
    let (_,consts)=shape(&push);
    let &(_,fixed,_)=forms::FORMS.get(*table().get(&key(&shape(&push).0)).unwrap()).unwrap();
    let slot=fixed.iter().position(Option::is_some).expect("a fixed constant");
    let mut n=0;
    for t in &mut push.tokens{if let Token::Const(_,v)=t{if n==slot{v.bits=(consts[slot]+1)&crate::mask(v.width);}n+=1;}}
    assert!(resolve(&push).is_none());
}

/// Every form reachable from a sample of encodings agrees with the
/// interpreter on full state, including faults, from varied register and
/// flag values. The monitor image keeps RAM at 4 KiB, so many memory
/// operands fault: both sides must fault the same way.
#[test]
fn forms_agree_with_the_interpreter_on_full_state(){
    if forms()==0{return;}
    // Decode as Dine does; execute against the 4 KiB monitor RAM.
    let dos=dos::load(&[0x90],"FORM.COM").unwrap();
    let (image,ram)=guest::Image::load(&[0x90]).unwrap();
    let mut code=dos.ram.clone();let at=dos.image.segments[1].base+0x100;
    let mut seed=0x2545_f491_4f6c_dd1du64;
    let mut next=||{seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;seed};
    let mut checked=0;
    for prefix in [&[][..],&[0x66][..],&[0x67][..],&[0x26][..]]{
        for opcode in 0..512u32{for modrm in [0x00u8,0x06,0x07,0x47,0x5e,0x87,0xc0,0xc3,0xd1,0xe9,0xf7]{
            let mut bytes=prefix.to_vec();if opcode>=256{bytes.push(0x0f);}bytes.push(opcode as u8);bytes.push(modrm);
            bytes.extend_from_slice(&[0x10,0x01,0x22,0x00,0x05,0x00]);
            code.write_bytes(at,&bytes).unwrap();
            let Ok(decoded)=dos.image.decode(&code,0x100)else{continue};
            let Operation::Pil{program,dst,src,flow,address}=&decoded.operation else{continue};
            let Some(native)=decoded.native()else{continue};
            for _ in 0..4{
                let mut cpu=crate::Cpu::default();
                for r in &mut cpu.regs{*r=(next() as u32)&0x8000_03ff;}
                cpu.regs[4]=0x200+((next() as u32)&0x1fe);
                cpu.eflags=0x2|((next() as u32)&0x0cd5);cpu.eip=0x100;
                let mut ram_a=ram.clone();ram_a.write_bytes(0x300,&next().to_le_bytes()).unwrap();let mut ram_b=ram_a.clone();
                let mut machine_a=image.segment_state();let mut machine_b=machine_a;
                let mut native_cpu=cpu.clone();
                let got=(native.form)(&mut native_cpu.regs,&mut native_cpu.eflags,&mut native_cpu.undefined_flags,&mut native_cpu.eip,(dst.register,dst.low),(src.register,src.low),*flow,&mut ram_a,*address,&mut machine_a,&native.consts);
                let mut lazy=crate::LazyFlags::None;
                let want=crate::execute_machine_lazy(program,&mut cpu,*dst,*src,*flow,&mut ram_b,*address,&mut machine_b,&mut lazy).map(|_|());
                lazy.flush(&mut cpu.eflags,&mut cpu.undefined_flags);
                assert_eq!(got,want,"{} {bytes:02x?}",decoded.label);
                assert_eq!(native_cpu,cpu,"{} {bytes:02x?}",decoded.label);
                assert_eq!(machine_a,machine_b,"{} {bytes:02x?}",decoded.label);
                assert!(ram_a==ram_b,"{} {bytes:02x?}: RAM differs",decoded.label);
                checked+=1;
            }
        }}
    }
    assert!(checked>10_000,"only {checked} cases");
}

fn load_with(bytes:&[u8],name:&str,mode:Mode)->dos::Loaded{
    let mut p=dos::load(bytes,name).unwrap();p.blocks.mode=Some(mode);p
}

#[test]
fn native_counts_exclude_block_fallback_and_include_single_dispatch() {
    if forms()==0{return;}
    let bytes=[0xb8,7,0,0x40,0xcd,0x20]; // MOV AX,7; INC AX; INT 20h.
    let mut p=load_with(&bytes,"COUNTERS.COM",Mode::On);
    // Model a table with an unavailable form, without depending on a gap
    // in the generated instruction inventory.
    p.cache.fetch(&p.image,&p.ram,0x103).unwrap().native.set(None).unwrap();
    let mut out=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let run=dos::run_with_backend(p,20,&mut out,None).unwrap();
    assert_eq!(run.stop,Stop::Exit(0));
    assert_eq!(run.loaded.cpu.regs[0],8);
    assert_eq!(run.loaded.blocks.instructions,2);
    assert_eq!(run.loaded.blocks.native_instructions,1);
    assert_eq!(run.loaded.blocks.interpreted_instructions,1);

    let mut p=load_with(&bytes,"COUNTERS.COM",Mode::On);
    p.image.ss_shadow=1; // The first instruction cannot enter a block.
    let run=dos::run_with_backend(p,20,&mut out,None).unwrap();
    assert_eq!(run.stop,Stop::Exit(0));
    assert_eq!(run.loaded.blocks.single_instructions,1);
    assert_eq!(run.loaded.blocks.native_instructions,1);
    assert_eq!(run.loaded.blocks.interpreted_instructions,0);
}

#[test]
fn required_native_stops_before_missing_form_without_interpreting_it() {
    let bytes=[0xb8,7,0,0x40,0xcd,0x20];
    let mut p=load_with(&bytes,"REQUIRED.COM",Mode::Required);
    let missing=if forms()==0{0x100}else{0x103};
    p.cache.fetch(&p.image,&p.ram,missing).unwrap().native.set(None).unwrap();
    let mut out=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let run=dos::run_with_backend(p,20,&mut out,None).unwrap();
    assert!(matches!(&run.stop,Stop::Service(s) if s.contains("interpreter fallback refused")),"{:?}",run.stop);
    assert_eq!(run.loaded.cpu.eip,missing);
    assert_eq!(run.loaded.cpu.regs[0],if forms()==0{0}else{7});
    assert_eq!(run.loaded.blocks.interpreted_instructions,0);
}

#[test]
fn interpreter_retirements_and_native_faults_are_counted_separately() {
    let mut out=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let run=dos::run_with_backend(load_with(&[0x40,0xcd,0x20],"COUNT.COM",Mode::Off),20,&mut out,None).unwrap();
    assert_eq!(run.stop,Stop::Exit(0));
    assert_eq!(run.loaded.blocks.interpreted_instructions,1);
    assert_eq!(run.loaded.blocks.native_instructions+run.loaded.blocks.single_instructions,0);
    if forms()==0{return;}
    // MOV AX,7; MOV [0200h],AX. A refused store must not count as retired.
    let mut p=load_with(&[0xb8,7,0,0xa3,0,2],"FAULTCOUNT.COM",Mode::On);
    let at=p.image.segments[3].base+0x200;
    p.ram.protect(at,2).unwrap();
    let run=dos::run_with_backend(p,20,&mut out,None).unwrap();
    assert!(matches!(run.stop,Stop::Fault(_)),"{:?}",run.stop);
    assert_eq!(run.loaded.cpu.eip,0x103);
    assert_eq!(run.loaded.blocks.native_instructions,1);
    assert_eq!(run.loaded.blocks.interpreted_instructions,0);
}

#[test]
fn partial_repeat_elements_do_not_inflate_retired_instruction_counts() {
    // MOV CX,3; MOV DI,0300h; REP STOSB; INT 20h.
    let bytes=[0xb9,3,0,0xbf,0,3,0xf3,0xaa,0xcd,0x20];
    for mode in [Mode::Off,Mode::On]{
        let mut p=load_with(&bytes,"REPCOUNT.COM",mode);
        let mut out=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
        let first=dos::run_with_backend(p,3,&mut out,None).unwrap();
        assert_eq!(first.completed,2);
        assert_eq!(first.loaded.cpu.regs[1],2);
        let counters=&first.loaded.blocks;
        assert_eq!(counters.native_instructions+counters.single_instructions+counters.interpreted_instructions,2);
        p=first.loaded;
        let last=dos::run_with_backend(p,20,&mut out,None).unwrap();
        assert_eq!(last.stop,Stop::Exit(0));
        let counters=&last.loaded.blocks;
        assert_eq!(counters.native_instructions+counters.single_instructions+counters.interpreted_instructions,3);
    }
}

/// Blocks and the interpreter reach the same state after every slice, with
/// slice budgets that cut blocks at odd places.
fn same_run(bytes:&[u8],name:&str,slices:usize){
    let mut a=load_with(bytes,name,Mode::Off);let mut b=load_with(bytes,name,Mode::On);
    let mut out_a=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);let mut out_b=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    for budget in [1usize,3,7,16,17,100,255,256,1000,4099].into_iter().cycle().take(slices){
        let left=dos::run_with_backend(a,budget,&mut out_a,None).unwrap();
        let right=dos::run_with_backend(b,budget,&mut out_b,None).unwrap();
        assert_eq!(left.loaded.cpu,right.loaded.cpu,"{name}");assert_eq!(left.loaded.image,right.loaded.image,"{name}");
        assert!(left.loaded.ram.bytes()==right.loaded.ram.bytes(),"{name}: RAM differs");
        assert_eq!(left.stop,right.stop,"{name}");assert_eq!(left.completed,right.completed,"{name}");
        assert_eq!(left.pil_steps,right.pil_steps,"{name}");assert_eq!(left.instructions,right.instructions,"{name}");
        assert_eq!(out_a.bytes(),out_b.bytes(),"{name}");
        if !matches!(left.stop,Stop::Budget){return;}
        a=left.loaded;b=right.loaded;b.blocks.mode=Some(Mode::On);a.blocks.mode=Some(Mode::Off);
    }
}

#[test]
fn blocks_match_the_interpreter_on_the_dos_fixtures(){
    same_run(include_bytes!("../../programs/dos/FACTOR.COM"),"FACTOR.COM",2000);
    same_run(include_bytes!("../../programs/dos/RECURFIB.COM"),"RECURFIB.COM",2000);
    same_run(include_bytes!("../../programs/dos/MANDEL.COM"),"MANDEL.COM",400);
    same_run(include_bytes!("../../programs/dos/MANDBBC.COM"),"MANDBBC.COM",400);
}

/// 50 rounds of: REP MOVSW of 512 words, then DS toggles by one paragraph
/// and a subroutine adds BX into [3000h] in that DS. The subroutine's block
/// is entered with two segment states in turn.
const REP_AND_TWO_DS:&[u8]=&[
    0xba,0x32,0x00,             // MOV DX,50
    0xb9,0x00,0x02,             // again: MOV CX,0200h
    0xbe,0x00,0x10,             // MOV SI,1000h
    0xbf,0x00,0x20,             // MOV DI,2000h
    0xfc,                       // CLD
    0xf3,0xa5,                  // REP MOVSW
    0x8c,0xd8,                  // MOV AX,DS
    0x35,0x01,0x00,             // XOR AX,1
    0x8e,0xd8,                  // MOV DS,AX
    0xe8,0x08,0x00,             // CALL sub
    0x4a,                       // DEC DX
    0x75,0xe7,                  // JNZ again
    0xb8,0x00,0x4c,0xcd,0x21,   // MOV AX,4C00h; INT 21h
    0x01,0x1e,0x00,0x30,        // sub: ADD [3000h],BX
    0x43,0xc3,                  // INC BX; RET
];

#[test]
fn rep_batches_and_block_variants_match_one_step_at_a_time(){
    same_run(REP_AND_TWO_DS,"REPDS.COM",400);
    // A one-step budget never batches REP elements; one long run does.
    for mode in [Mode::Off,Mode::On]{
        let mut out=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
        let whole=dos::run_with_backend(load_with(REP_AND_TWO_DS,"REPDS.COM",mode),100_000,&mut out,None).unwrap();
        assert_eq!(whole.stop,Stop::Exit(0));
        let mut p=load_with(REP_AND_TWO_DS,"REPDS.COM",mode);let mut steps=0;let mut completed=0;
        let last=loop{
            let r=dos::run_with_backend(p,1,&mut out,None).unwrap();
            steps+=1;completed+=r.completed;
            if r.stop!=Stop::Budget{break r;}
            p=r.loaded;p.blocks.mode=Some(mode);
        };
        assert_eq!(last.stop,Stop::Exit(0));
        assert_eq!(last.loaded.cpu,whole.loaded.cpu,"{mode:?}");
        assert!(last.loaded.ram.bytes()==whole.loaded.ram.bytes(),"{mode:?}: RAM differs");
        assert_eq!(completed,whole.completed,"{mode:?}");
        // 50 x (512 elements + 13 instructions) + 3 at the start and end.
        assert!(steps>50*512,"{mode:?}: {steps}");
    }
}

#[test]
fn blocks_see_code_the_program_rewrites(){
    // MOV CX,5; again: MOV [0108h],CL; MOV AX,0 (its immediate is rewritten);
    // ADD BX,AX; LOOP again; MOV AX,4C00h; INT 21h.
    let bytes=[0xb9,5,0,0x88,0x0e,0x08,0x01,0xb8,0,0,0x01,0xc3,0xe2,0xf5,0xb8,0,0x4c,0xcd,0x21];
    for mode in [Mode::Off,Mode::On]{
        let mut out=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
        let r=dos::run_with_backend(load_with(&bytes,"SMC.COM",mode),10_000,&mut out,None).unwrap();
        assert_eq!(r.stop,Stop::Exit(0));assert_eq!(r.loaded.cpu.regs[3]&0xffff,15,"{mode:?}");
    }
    same_run(&bytes,"SMC.COM",50);
}

#[test]
fn verify_mode_runs_both_backends_in_lockstep(){
    if forms()==0{return;}
    let mut out=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let r=dos::run_with_backend(load_with(include_bytes!("../../programs/dos/FACTOR.COM"),"FACTOR.COM",Mode::Verify),2_000,&mut out,None).unwrap();
    assert!(r.pil_steps>100);
}

/// MOV CX,40; again: CALL sub; ADD AX,0000h (the callee rewrites this
/// immediate); ADD BX,AX; LOOP again; MOV AX,4C00h; INT 21h.
/// sub: MOV AX,CX; MOV [0107h],CL; RET.
const CALL_SMC:[u8;25]=[0xb9,40,0,0xe8,0x0c,0,0x05,0,0,0x01,0xc3,0xe2,0xf6,0xb8,0,0x4c,0xcd,0x21,0x89,0xc8,0x88,0x0e,0x07,0x01,0xc3];

#[test]
fn chained_blocks_follow_call_and_ret_and_see_code_the_callee_rewrites(){
    for mode in [Mode::Off,Mode::On]{
        let mut out=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
        let r=dos::run_with_backend(load_with(&CALL_SMC,"CALLSMC.COM",mode),10_000,&mut out,None).unwrap();
        assert_eq!(r.stop,Stop::Exit(0),"{mode:?}");
        // Each pass adds CX twice: from AX=CX and from the rewritten immediate.
        assert_eq!(r.loaded.cpu.regs[3]&0xffff,2*(1..=40).sum::<u32>(),"{mode:?}");
        if mode==Mode::On{
            let b=&r.loaded.blocks;
            assert!(b.instructions>150,"blocks ran {} instructions",b.instructions);
            assert!(b.runs*4<b.blocks,"blocks chained: {} entries in {} runs",b.blocks,b.runs);
        }
    }
    same_run(&CALL_SMC,"CALLSMC.COM",60);
}

#[test]
fn a_fault_inside_a_chain_is_taken_where_the_interpreter_takes_it(){
    // MOV CX,5; again: CALL sub; LOOP again; MOV AX,4C00h; INT 21h.
    // sub: XOR DX,DX; MOV AX,100; MOV BX,CX; SUB BX,3; DIV BX; RET.
    // The third call divides by zero: the default INT 0 handler reports it
    // and ends the program with errorlevel 127.
    let bytes=[0xb9,5,0,0xe8,7,0,0xe2,0xfb,0xb8,0,0x4c,0xcd,0x21,0x31,0xd2,0xb8,100,0,0x89,0xcb,0x83,0xeb,3,0xf7,0xf3,0xc3];
    let mut out=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let r=dos::run_with_backend(load_with(&bytes,"DIVCHAIN.COM",Mode::On),10_000,&mut out,None).unwrap();
    assert_eq!(r.stop,Stop::Exit(127));
    same_run(&bytes,"DIVCHAIN.COM",60);
}

#[test]
fn chained_blocks_match_the_interpreter_on_the_graphics_fixtures(){
    same_run(include_bytes!("../../programs/dos/MANDCGA.COM"),"MANDCGA.COM",300);
    same_run(include_bytes!("../../programs/dos/MANDVGA.COM"),"MANDVGA.COM",300);
}

/// Compact builds must not link generated native forms; unsupported forms
/// automatically use the same generic PIL execution engine in either mode.
#[cfg(feature = "compact-interpreter")]
#[test]
fn compact_build_omits_generated_forms_and_uses_interpreter() {
    assert_eq!(forms(), 0);
    let program = pil_program(&[0x05, 0x34, 0x12, 0xcd, 0x20]);
    assert!(resolve(&program).is_none());
    same_run(include_bytes!("../../programs/dos/FACTOR.COM"), "FACTOR.COM", 40);
}


#[test]
fn fused_subtract_branch_loop_preserves_partial_slices() {
    // MOV CX,1000; SUB CX,1; JNZ back to SUB; INT 20h.
    let bytes = [0xb9,0xe8,3,0x83,0xe9,1,0x75,0xfb,0xcd,0x20];
    same_run(&bytes, "COUNT.COM", 50);
    let mut output = CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let run = dos::run_with_backend(load_with(&bytes, "COUNT.COM", Mode::On),
        3000, &mut output, None).unwrap();
    assert_eq!(run.stop, Stop::Exit(0));
    assert!(run.loaded.blocks.compare_branches > 900);
}

/// A boot sector (original, hand-assembled) that enters 32-bit protected
/// mode with paging and then loops: CALL to linear 50000h, whose page maps
/// frame 40000h (MOV EAX,1; RET) and 41000h (MOV EAX,2; RET) in turn (the
/// PTE is rewritten and CR3 reloaded each round), and a store through
/// linear 20000h, an alias of the code's own page, rewrites the immediate
/// of the ADD after it. IF stays clear, so the run does not depend on the
/// host clock.
const PAGED_32:&[u8]=&[
    0xfa,                                           // 7c00 cli
    0x31,0xc0,                                      // 7c01 xor ax,ax
    0x8e,0xd8,                                      // 7c03 mov ds,ax
    0x0f,0x01,0x16,0xe0,0x7c,                       // 7c05 lgdt [gdtr]
    0x0f,0x20,0xc0,                                 // 7c0a mov eax,cr0
    0x0c,0x01,                                      // 7c0d or al,1
    0x0f,0x22,0xc0,                                 // 7c0f mov cr0,eax
    0x66,0xea,0x1a,0x7c,0x00,0x00,0x08,0x00,        // 7c12 jmp dword 08h:pm32
    0x66,0xb8,0x10,0x00,                            // 7c1a mov ax,10h
    0x8e,0xd8,                                      // 7c1e mov ds,ax
    0x8e,0xc0,                                      // 7c20 mov es,ax
    0x8e,0xd0,                                      // 7c22 mov ss,ax
    0xbc,0x00,0x90,0x00,0x00,                       // 7c24 mov esp,9000h
    0xbf,0x00,0x10,0x01,0x00,                       // 7c29 mov edi,11000h
    0xb8,0x03,0x00,0x00,0x00,                       // 7c2e mov eax,3
    0xb9,0x00,0x04,0x00,0x00,                       // 7c33 mov ecx,1024
    0xab,                                           // 7c38 stosd
    0x05,0x00,0x10,0x00,0x00,                       // 7c39 add eax,1000h
    0xe2,0xf8,                                      // 7c3e loop fill
    0xc7,0x05,0x00,0x00,0x01,0x00,0x03,0x10,0x01,0x00,// 7c40 mov dword [10000h],11003h  ; PDE 0
    0xc7,0x05,0x80,0x10,0x01,0x00,0x03,0x70,0x00,0x00,// 7c4a mov dword [11080h],7003h  ; linear 20000h -> frame 7000h (this code)
    0xc7,0x05,0x00,0x00,0x04,0x00,0xb8,0x01,0x00,0x00,// 7c54 mov dword [40000h] ; MOV EAX,1
    0xc7,0x05,0x04,0x00,0x04,0x00,0x00,0xc3,0x00,0x00,// 7c5e mov dword [40004h] ; RET
    0xc7,0x05,0x00,0x10,0x04,0x00,0xb8,0x02,0x00,0x00,// 7c68 mov dword [41000h] ; MOV EAX,2
    0xc7,0x05,0x04,0x10,0x04,0x00,0x00,0xc3,0x00,0x00,// 7c72 mov dword [41004h] ; RET
    0xc7,0x05,0x40,0x11,0x01,0x00,0x03,0x00,0x04,0x00,// 7c7c mov dword [11140h],40003h ; linear 50000h -> frame 40000h
    0xb8,0x00,0x00,0x01,0x00,                       // 7c86 mov eax,10000h
    0x0f,0x22,0xd8,                                 // 7c8b mov cr3,eax
    0x0f,0x20,0xc0,                                 // 7c8e mov eax,cr0
    0x0d,0x00,0x00,0x00,0x80,                       // 7c91 or eax,80000000h
    0x0f,0x22,0xc0,                                 // 7c96 mov cr0,eax
    0x31,0xdb,                                      // 7c99 xor ebx,ebx
    0x31,0xf6,                                      // 7c9b xor esi,esi
    0xe8,0x5e,0x83,0x04,0x00,                       // 7c9d call 50000h
    0x01,0xc3,                                      // 7ca2 add ebx,eax
    0x81,0x35,0x40,0x11,0x01,0x00,0x00,0x10,0x00,0x00,// 7ca4 xor dword [11140h],1000h ; the other frame
    0x0f,0x20,0xd8,                                 // 7cae mov eax,cr3
    0x0f,0x22,0xd8,                                 // 7cb1 mov cr3,eax
    0x46,                                           // 7cb4 inc esi
    0x89,0xf0,                                      // 7cb5 mov eax,esi
    0xa2,0xbe,0x0c,0x02,0x00,                       // 7cb7 mov [alias of imm],al
    0x83,0xc3,                                      // 7cbc add ebx,imm8 (rewritten)
    0x00,                                           // 7cbe   imm8
    0x89,0x1d,0x00,0x00,0x30,0x00,                  // 7cbf mov [300000h],ebx
    0xeb,0xd6,                                      // 7cc5 jmp again
    0x90,                                           // 7cc7 nop
    0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,        // 7cc8 null descriptor
    0xff,0xff,0x00,0x00,0x00,0x9a,0xcf,0x00,        // 7cd0 08h: code, base 0, 4 GiB, 32-bit
    0xff,0xff,0x00,0x00,0x00,0x92,0xcf,0x00,        // 7cd8 10h: data, base 0, 4 GiB
    0x17,0x00,0xc8,0x7c,0x00,0x00,                  // 7ce0 GDTR
];

fn booted(sector:&[u8],mode:Mode)->dos::Loaded{
    let mut disk=vec![0u8;1_474_560];
    disk[..sector.len()].copy_from_slice(sector);disk[510]=0x55;disk[511]=0xaa;
    let mut p=dos::boot(&disk,0).unwrap();p.blocks.mode=Some(mode);p
}

#[test]
fn blocks_match_the_interpreter_in_paged_32_bit_protected_mode(){
    let mut a=booted(PAGED_32,Mode::Off);let mut b=booted(PAGED_32,Mode::On);
    let mut out_a=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);let mut out_b=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    for budget in [1usize,3,7,16,17,100,255,256,1000,4099].into_iter().cycle().take(300){
        let left=dos::run_with_backend(a,budget,&mut out_a,None).unwrap();
        let right=dos::run_with_backend(b,budget,&mut out_b,None).unwrap();
        assert_eq!(left.stop,Stop::Budget);assert_eq!(right.stop,Stop::Budget);
        assert_eq!(left.loaded.cpu,right.loaded.cpu);assert_eq!(left.loaded.image,right.loaded.image);
        assert!(left.loaded.ram.bytes()==right.loaded.ram.bytes(),"RAM differs");
        assert_eq!(left.completed,right.completed);assert_eq!(left.instructions,right.instructions);
        a=left.loaded;b=right.loaded;a.blocks.mode=Some(Mode::Off);b.blocks.mode=Some(Mode::On);
    }
    assert!(!b.image.real_mode&&b.image.paging,"the loop runs with paging");
    // Rounds: EBX gains 1 or 2 from the mapped routine and the round number
    // (low byte) from the rewritten immediate.
    let rounds=b.cpu.regs[6];assert!(rounds>1000,"{rounds}");
    // The sum each round stores at 300000h (the ADD's imm8 is sign-extended):
    // the interpreter and the blocks both run through the decode cache, so
    // check it against the rounds.
    let sum=|r:u32|(1..=r).fold(0u32,|s,i|s.wrapping_add(if i%2==1{1}else{2}).wrapping_add(i as u8 as i8 as u32));
    for p in [&a,&b]{
        let stored=p.ram.load(0x300000,crate::MemWidth::Dword).unwrap();
        assert!(stored==sum(rounds)||stored==sum(rounds-1),"stored {stored:x} after {rounds} rounds");
    }
    assert!(b.blocks.blocks>1000,"blocks ran: {} entries",b.blocks.blocks);
}
