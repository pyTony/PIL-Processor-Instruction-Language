use crate::precompiled::*;
use crate::{guest,Program,Token};
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
    let code_base=dos.image.segments[1].base;
    let code32=crate::address::SegmentCache{base:code_base,limit:0xffff_ffff,access:crate::address::SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:0}};
    let exec_code32=crate::address::SegmentCache{base:0,limit:0xffff_ffff,access:crate::address::SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:0}};
    let data32=crate::address::SegmentCache{base:0,limit:0xffff_ffff,access:crate::address::SegmentAccess::Data{writable:true,expand_down:false,big:true,dpl:0}};
    let mut pm_decode=dos.image.clone();
    pm_decode.real_mode=false;pm_decode.msw=0x11;pm_decode.controls[0]=0x11;
    pm_decode.segments=[data32,code32,data32,data32,data32,data32];
    pm_decode.selectors=[0x10,0x08,0x10,0x10,0x10,0x10];
    let mut pm_exec=image.clone();
    pm_exec.real_mode=false;pm_exec.msw=0x11;pm_exec.controls[0]=0x11;
    pm_exec.segments=[data32,exec_code32,data32,data32,data32,data32];
    pm_exec.selectors=[0x10,0x08,0x10,0x10,0x10,0x10];
    pm_exec.tables[0]=(0x27,0x500);
    let mut pm_ram=ram.clone();
    // GDT at 0500h: null, 08h 32-bit code, 10h 32-bit data, 18h 16-bit data, 20h ring-3 data.
    for (i,desc) in [0u64,0x00cf_9a00_0000_ffff,0x00cf_9200_0000_ffff,0x0000_9200_0000_ffff,0x00cf_f200_0000_ffff].into_iter().enumerate(){
        pm_ram.write_bytes(0x500+(i as u32)*8,&desc.to_le_bytes()).unwrap();
    }
    let mut code=dos.ram.clone();let at=code_base+0x100;
    let mut seed=0x2545_f491_4f6c_dd1du64;
    let mut next=||{seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;seed};
    let mut checked=0;
    for (decode_image,exec_image,base_ram) in [(&dos.image,&image,&ram),(&pm_decode,&pm_exec,&pm_ram)]{
    for prefix in [&[][..],&[0x66][..],&[0x67][..],&[0x26][..]]{
        for opcode in 0..512u32{for modrm in [0x00u8,0x04,0x06,0x07,0x47,0x5e,0x87,0xc0,0xc3,0xd1,0xd8,0xe9,0xf7]{
            let mut bytes=prefix.to_vec();if opcode>=256{bytes.push(0x0f);}bytes.push(opcode as u8);bytes.push(modrm);
            bytes.extend_from_slice(&[0x10,0x01,0x22,0x00,0x05,0x00]);
            code.write_bytes(at,&bytes).unwrap();
            let Ok(decoded)=decode_image.decode(&code,0x100)else{continue};
            let Operation::Pil{program,dst,src,flow,address}=&decoded.operation else{continue};
            let Some(native)=decoded.native()else{continue};
            for _ in 0..4{
                let mut cpu=crate::Cpu::default();
                for r in &mut cpu.regs{*r=(next() as u32)&0x8000_03ff;}
                cpu.regs[4]=0x200+((next() as u32)&0x1fc);
                cpu.eflags=0x2|((next() as u32)&0x0cd5);cpu.eip=0x100;
                let mut ram_a=base_ram.clone();ram_a.write_bytes(0x300,&next().to_le_bytes()).unwrap();let mut ram_b=ram_a.clone();
                let mut machine_a=exec_image.segment_state();let mut machine_b=machine_a;
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
    }
    assert!(checked>if cfg!(feature = "common-forms"){2_000}else{20_000},"only {checked} cases");
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

#[cfg(all(feature = "common-forms", not(feature = "compact-interpreter")))]
#[test]
fn common_forms_build_compiles_core_subset_and_falls_back_cleanly() {
    let count = forms();
    assert!((200..1200).contains(&count), "expected 200..1200 common forms, got {count}");
    // Common real-mode instructions resolve to compiled forms.
    for sample in [
        &[0x05u8, 0x34, 0x12, 0xcd, 0x20][..], // ADD AX, 1234h
        &[0x8b, 0x07, 0xcd, 0x20],             // MOV AX, [BX]
        &[0x50, 0xcd, 0x20],                   // PUSH AX
        &[0x75, 0x02, 0x90, 0x90, 0xcd, 0x20], // JNZ +2
        &[0xd1, 0xe0, 0xcd, 0x20],             // SHL AX, 1
    ] {
        assert!(resolve(&pil_program(sample)).is_some(), "expected common real-mode form for {sample:02x?}");
    }
    // Common 32-bit protected-mode instructions also resolve.
    let mut pm = dos::load(&[0x90], "PMCOMMON.COM").unwrap().image;
    pm.real_mode = false; pm.msw = 0x11; pm.controls[0] = 0x11;
    let code32 = crate::address::SegmentCache { base: pm.segments[1].base, limit: 0xffff_ffff, access: crate::address::SegmentAccess::Code { readable: true, conforming: false, big: true, dpl: 0 } };
    let data32 = crate::address::SegmentCache { base: 0, limit: 0xffff_ffff, access: crate::address::SegmentAccess::Data { writable: true, expand_down: false, big: true, dpl: 0 } };
    pm.segments = [data32, code32, data32, data32, data32, data32];
    pm.selectors = [0x10, 0x08, 0x10, 0x10, 0x10, 0x10];
    let mut pm_ram = crate::Ram::new(0x10_0000);
    let at = pm.segments[1].base + 0x100;
    for sample in [
        &[0x01u8, 0xd8][..],             // ADD EAX, EBX
        &[0x8b, 0x44, 0x24, 0x04],       // MOV EAX, [ESP+4]
        &[0x55],                         // PUSH EBP
        &[0xe8, 0x10, 0x00, 0x00, 0x00], // CALL rel32
        &[0xc3],                         // RET
    ] {
        pm_ram.write_bytes(at, sample).unwrap();
        let dec = pm.decode(&pm_ram, 0x100).unwrap();
        let Operation::Pil { program, .. } = &dec.operation else { panic!("not PIL: {:?}", dec.operation) };
        assert!(resolve(program).is_some(), "expected common 32-bit PM form for {} ({sample:02x?})", dec.label);
    }
    // Rare/system instructions are excluded and fall back to the interpreter.
    for rare in [
        &[0x37u8, 0xcd, 0x20][..],             // AAA
        &[0x62, 0x06, 0x00, 0x02, 0xcd, 0x20], // BOUND AX, [0200h]
        &[0xc8, 0x04, 0x00, 0x00, 0xcd, 0x20], // ENTER 4, 0
    ] {
        assert!(resolve(&pil_program(rare)).is_none(), "expected rare real-mode form to be omitted: {rare:02x?}");
    }
    for sys in [
        &[0x0fu8, 0x01, 0x16, 0x00, 0x02, 0x00, 0x00][..], // LGDT [0200h]
        &[0x0f, 0x20, 0xc0],                               // MOV EAX, CR0
    ] {
        pm_ram.write_bytes(at, sys).unwrap();
        let dec = pm.decode(&pm_ram, 0x100).unwrap();
        let Operation::Pil { program, .. } = &dec.operation else { panic!("not PIL: {:?}", dec.operation) };
        assert!(resolve(program).is_none(), "expected system form to be omitted: {}", dec.label);
    }
    same_run(include_bytes!("../../programs/dos/FACTOR.COM"), "FACTOR.COM", 40);
    let mut out = CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let run = dos::run_with_backend(load_with(include_bytes!("../../programs/dos/FACTOR.COM"), "FACTOR.COM", Mode::On), 10_000, &mut out, None).unwrap();
    assert_eq!(run.stop, Stop::Exit(0));
    assert!(run.loaded.blocks.native_instructions > 0);
    assert_eq!(run.loaded.blocks.interpreted_instructions, 0);
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
    // Between runs front ends see physical memory (Ram::host_access):
    // 20CBEh is the code page's alias only through the guest's page tables.
    assert_eq!(b.ram.load(0x20cbe,crate::MemWidth::Byte).unwrap(),0);
    assert_ne!(b.ram.load(0x7cbe,crate::MemWidth::Byte).unwrap(),0);
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
    if forms()>0 && !cfg!(feature = "common-forms"){
        assert_eq!(b.blocks.interpreted_instructions,0,"all paged 32-bit instructions use precompiled forms");
        assert!(b.blocks.native_instructions>5000);
        let mut out_req=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
        let req=dos::run_with_backend(booted(PAGED_32,Mode::Required),10_000,&mut out_req,None).unwrap();
        assert_eq!(req.stop,Stop::Budget);
        assert!(!req.loaded.image.real_mode&&req.loaded.image.paging);
        assert_eq!(req.loaded.blocks.interpreted_instructions,0);
    }
}

#[test]
fn rom_blocks_match_the_interpreter_across_real_and_32_bit_protected_mode(){
    // 64 KiB ROM at F0000h: reset at F000:FFF0 jumps to F000:0100, builds a
    // GDT at 0000:0500 and a 32-bit RAM helper at 0000:0600, enters 32-bit
    // protected mode at 08h:000F0160 (in ROM), calls the RAM helper and a ROM
    // helper (PUSH/MOV EBP/LEA/ARPL/LSL/LEAVE/RET on a 32-bit stack) in a
    // loop, then halts with IF=0.
    let mut rom=vec![0xf4u8;0x10000];
    rom[0xfff0..0xfff5].copy_from_slice(&[0xea,0x00,0x01,0x00,0xf0]); // jmp F000:0100
    let setup:&[u8]=&[
        0xfa,                                           // 0100 cli
        0x31,0xc0,                                      // 0101 xor ax,ax
        0x8e,0xd8,                                      // 0103 mov ds,ax
        0x8e,0xc0,                                      // 0105 mov es,ax
        // GDT at 0500h: null, 08h flat 32-bit code, 10h ring-0 data, 18h ring-3 data
        0xbf,0x00,0x05,                                 // 0107 mov di,0500h
        0x66,0x31,0xc0,                                 // 010a xor eax,eax
        0x66,0xab,0x66,0xab,                            // 010d stosd; stosd
        0x66,0xb8,0xff,0xff,0x00,0x00,0x66,0xab,        // 0111 dd 0000ffffh
        0x66,0xb8,0x00,0x9a,0xcf,0x00,0x66,0xab,        // 0119 dd 00cf9a00h
        0x66,0xb8,0xff,0xff,0x00,0x00,0x66,0xab,        // 0121 dd 0000ffffh
        0x66,0xb8,0x00,0x92,0xcf,0x00,0x66,0xab,        // 0129 dd 00cf9200h
        0x66,0xb8,0xff,0xff,0x00,0x00,0x66,0xab,        // 0131 dd 0000ffffh
        0x66,0xb8,0x00,0xf2,0xcf,0x00,0x66,0xab,        // 0139 dd 00cff200h
        // GDTR at 0530h: limit 1fh, base 00000500h
        0xc7,0x06,0x30,0x05,0x1f,0x00,                  // 0141 mov word [0530h],1fh
        0x66,0xc7,0x06,0x32,0x05,0x00,0x05,0x00,0x00,   // 0147 mov dword [0532h],500h
        // Copy 9-byte 32-bit subroutine to 0600h:
        // 55 (push ebp); 89 e5 (mov ebp,esp); 8d 44 48 05 (lea eax,[eax+ecx*2+5]); c9 (leave); c3 (ret)
        0x66,0xc7,0x06,0x00,0x06,0x55,0x89,0xe5,0x8d,   // 0150
        0x66,0xc7,0x06,0x04,0x06,0x44,0x48,0x05,0xc9,   // 0159
        0xc6,0x06,0x08,0x06,0xc3,                       // 0162
        0x0f,0x01,0x16,0x30,0x05,                       // 0167 lgdt [0530h]
        0x0f,0x20,0xc0,                                 // 016c mov eax,cr0
        0x0c,0x01,                                      // 016f or al,1
        0x0f,0x22,0xc0,                                 // 0171 mov cr0,eax
        0x66,0xea,0x80,0x01,0x0f,0x00,0x08,0x00,        // 0174 jmp dword 08h:000F0180h
    ];
    rom[0x0100..0x0100+setup.len()].copy_from_slice(setup);
    let pm32:&[u8]=&[
        0x66,0xb8,0x10,0x00,                            // F0180 mov ax,10h
        0x8e,0xd8,0x8e,0xc0,0x8e,0xd0,                  // F0184 mov ds,ax; mov es,ax; mov ss,ax
        0xbc,0x00,0x20,0x00,0x00,                       // F018a mov esp,2000h
        0x31,0xc0,                                      // F018f xor eax,eax
        0xb9,0x14,0x00,0x00,0x00,                       // F0191 mov ecx,20
        0xe8,0x65,0x04,0xf1,0xff,                       // F0196 again: call 00000600h (disp = 0600 - F019B = FFF10465)
        0xe8,0x09,0x00,0x00,0x00,                       // F019b call F01A9h
        0xe2,0xf4,                                      // F01a0 loop again
        0x89,0x05,0x00,0x07,0x00,0x00,                  // F01a2 mov [0700h],eax
        0xf4,                                           // F01a8 hlt
        // ROM helper at F01A9h: exercise ARPL and LSL in 32-bit protected mode
        0x53,                                           // F01a9 push ebx
        0x66,0xbb,0x18,0x00,                            // F01aa mov bx,18h (DPL=3 data, RPL=0)
        0x66,0xba,0x1b,0x00,                            // F01ae mov dx,1bh (RPL=3)
        0x63,0xd3,                                      // F01b2 arpl bx,dx (bx -> 1bh, ZF=1)
        0x0f,0x03,0xd3,                                 // F01b4 lsl edx,ebx (edx -> ffffffffh)
        0x29,0xd0,                                      // F01b7 sub eax,edx (eax += 1)
        0x5b,                                           // F01b9 pop ebx
        0xc3,                                           // F01ba ret
    ];
    rom[0x0180..0x0180+pm32.len()].copy_from_slice(pm32);

    let make_rom=|mode:Mode|{
        let mut p=dos::load(&[0x90],"ROM").unwrap();
        let mut ram=crate::Ram::new(0x100000);
        ram.write_bytes(0xf0000,&rom).unwrap();
        ram.protect(0xf0000,rom.len()).unwrap();
        p.ram=ram;p.rom_mode=true;p.blocks.rom=true;p.blocks.mode=Some(mode);
        p.cpu.regs=[0;8];p.image.system=[(0,0);guest::SYSTEM_RANGES];
        p.image.fetch_base=0;p.image.start=0;p.image.end=0x100000;
        let selectors=[0,0xf000,0,0,0,0];
        p.selectors=selectors;p.image.selectors=selectors;
        p.image.segments=selectors.map(crate::address::SegmentCache::real_mode);
        p.cpu.eip=0xfff0;p.cpu.eflags=0x0002;
        p
    };
    let mut out_off=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let mut out_on=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let ref_run=dos::run_with_backend(make_rom(Mode::Off),10_000,&mut out_off,None).unwrap();
    let on_run=dos::run_with_backend(make_rom(Mode::On),10_000,&mut out_on,None).unwrap();
    assert!(matches!(ref_run.stop,Stop::Monitor(_)),"{:?}",ref_run.stop);
    assert_eq!(on_run.stop,ref_run.stop);
    assert_eq!(on_run.completed,ref_run.completed);
    assert_eq!(on_run.instructions,ref_run.instructions);
    assert_eq!(on_run.loaded.cpu,ref_run.loaded.cpu);
    assert_eq!(on_run.loaded.image,ref_run.loaded.image);
    assert!(on_run.loaded.ram.bytes()==ref_run.loaded.ram.bytes());
    // 20 rounds: sum_{c=1..20} (2*c + 5 + 1) = 2*210 + 120 = 540.
    assert_eq!(on_run.loaded.ram.load(0x700,crate::MemWidth::Dword).unwrap(),540);
    assert!(on_run.loaded.blocks.blocks>20);
    if forms()>0 && !cfg!(feature = "common-forms"){
        assert_eq!(on_run.loaded.blocks.interpreted_instructions,0);
        let mut out_req=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
        let req_run=dos::run_with_backend(make_rom(Mode::Required),10_000,&mut out_req,None).unwrap();
        assert_eq!(req_run.stop,ref_run.stop);
        assert_eq!(req_run.loaded.cpu,ref_run.loaded.cpu);
        assert_eq!(req_run.loaded.blocks.interpreted_instructions,0);
    }
}

#[test]
fn x87_loops_and_irq13_exceptions_run_inside_blocks_in_lockstep_with_the_interpreter(){
    // 1. Tight x87 loop: FNINIT; MOV CX,30; FLDZ;
    //    again: FLD1; FADD m32 [0200h] (0.5); FMULP ST(1),ST; FADD m32 [0204h] (1.25);
    //    FLD ST(0); FCOMP m32 [0208h] (100.0); FNSTSW AX; TEST AH,01h; JZ clamped;
    //    JMP next; clamped: FSTP ST(0); FLD1; next: LOOP again; FSTP m64 [0210h]; INT 20h.
    let loop_code:&[u8]=&[
        0xdb,0xe3,                      // 0100 fninit
        0xb9,0x1e,0x00,                 // 0102 mov cx,30
        0xd9,0xee,                      // 0105 fldz
        0xd9,0xe8,                      // 0107 again: fld1
        0xd8,0x06,0x00,0x02,            // 0109 fadd dword [0200h]
        0xde,0xc9,                      // 010d fmulp st(1),st
        0xd8,0x06,0x04,0x02,            // 010f fadd dword [0204h]
        0xd9,0xc0,                      // 0113 fld st(0)
        0xd8,0x1e,0x08,0x02,            // 0115 fcomp dword [0208h]
        0xdf,0xe0,                      // 0119 fnstsw ax
        0xf6,0xc4,0x01,                 // 011b test ah,01h (C0: < 100.0)
        0x74,0x02,                      // 011e jz clamped
        0xeb,0x04,                      // 0120 jmp next
        0xdd,0xd8,                      // 0122 clamped: fstp st(0)
        0xd9,0xe8,                      // 0124 fld1
        0xe2,0xdf,                      // 0126 next: loop again (0107h)
        0xdd,0x1e,0x10,0x02,            // 0128 fstp qword [0210h]
        0xcd,0x20,                      // 012c int 20h
    ];
    let make_fpu_run=|mode:Mode|{
        let mut p=load_with(loop_code,"X87LOOP.COM",mode);
        p.fpu=crate::fpu::Fpu::present();
        let base=p.image.segments[3].base;
        p.ram.store(base+0x200,crate::MemWidth::Dword,0.5f32.to_bits()).unwrap();
        p.ram.store(base+0x204,crate::MemWidth::Dword,1.25f32.to_bits()).unwrap();
        p.ram.store(base+0x208,crate::MemWidth::Dword,100.0f32.to_bits()).unwrap();
        p
    };
    let mut a=make_fpu_run(Mode::Off);let mut b=make_fpu_run(Mode::On);
    let mut out_a=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let mut out_b=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    for budget in [1usize,2,5,9,16,17,63,255,256,1000].into_iter().cycle().take(100){
        let left=dos::run_with_backend(a,budget,&mut out_a,None).unwrap();
        let right=dos::run_with_backend(b,budget,&mut out_b,None).unwrap();
        assert_eq!(left.stop,right.stop);
        assert_eq!(left.completed,right.completed);
        assert_eq!(left.pil_steps,right.pil_steps);
        assert_eq!(left.instructions,right.instructions);
        assert_eq!(left.loaded.cpu,right.loaded.cpu);
        assert_eq!(left.loaded.fpu.state(),right.loaded.fpu.state());
        assert!(left.loaded.ram.bytes()==right.loaded.ram.bytes());
        if left.stop!=Stop::Budget{
            assert_eq!(left.stop,Stop::Exit(0));
            // Dozens of x87 instructions retired inside blocks alongside PIL instructions.
            assert!(right.loaded.blocks.instructions>right.loaded.blocks.native_instructions+right.loaded.blocks.interpreted_instructions+100);
            break;
        }
        a=left.loaded;b=right.loaded;a.blocks.mode=Some(Mode::Off);b.blocks.mode=Some(Mode::On);
    }

    // 2. Unmasked 80387 exception inside a block delivers IRQ13 (INT 75h) at
    //    the exact instruction boundary before the following FWAIT.
    //    Handler at 0132h: PUSH AX; FNCLEX; OUT F0h,AL; MOV AL,20h; OUT A0h,AL;
    //    OUT 20h,AL; INC word [0220h]; POP AX; IRET.
    let irq13_code:&[u8]=&[
        0xfa,                           // 0100 cli
        0xb8,0x32,0x01,                 // 0101 mov ax,0132h
        0x8c,0xca,                      // 0104 mov dx,cs
        0x31,0xdb,                      // 0106 xor bx,bx
        0x8e,0xc3,                      // 0108 mov es,bx
        0x26,0x89,0x06,0xd4,0x01,       // 010a mov es:[75h*4],ax
        0x26,0x89,0x16,0xd6,0x01,       // 010f mov es:[75h*4+2],dx
        0xb0,0xfb,0xe6,0x21,            // 0114 mov al,0fbh; out 21h,al
        0xb0,0xdf,0xe6,0xa1,            // 0118 mov al,0dfh; out a1h,al
        0xfb,                           // 011c sti
        0x90,                           // 011d nop
        0xd9,0x2e,0x00,0x02,            // 011e fldcw [0200h]
        0xd9,0xee,                      // 0122 fldz
        0xd9,0xee,                      // 0124 fldz
        0xde,0xf9,                      // 0126 fdivp st(1),st (0/0 -> IE + IRQ13)
        0x9b,                           // 0128 fwait
        0xd9,0xe8,                      // 0129 fld1
        0xdd,0x1e,0x10,0x02,            // 012b fstp qword [0210h]
        0xcd,0x20,                      // 012f int 20h
        0x90,                           // 0131 nop
        0x50,0xdb,0xe2,0xe6,0xf0,       // 0132 push ax; fnclex; out f0h,al
        0xb0,0x20,0xe6,0xa0,0xe6,0x20,  // 0137 mov al,20h; out a0h,al; out 20h,al
        0xff,0x06,0x20,0x02,            // 013d inc word [0220h]
        0x58,0xcf,                      // 0141 pop ax; iret
    ];
    let make_irq13=|mode:Mode|{
        let mut p=load_with(irq13_code,"X87IRQ13.COM",mode);
        p.fpu=crate::fpu::Fpu::present();
        p.fpu.set_error_irq(true);
        let base=p.image.segments[3].base;
        p.ram.store(base+0x200,crate::MemWidth::Word,0x037e).unwrap(); // IM=0
        p
    };
    let mut out_off=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let mut out_on=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let left=dos::run_with_backend(make_irq13(Mode::Off),200,&mut out_off,None).unwrap();
    let right=dos::run_with_backend(make_irq13(Mode::On),200,&mut out_on,None).unwrap();
    assert_eq!(left.stop,Stop::Exit(0));
    assert_eq!(right.stop,Stop::Exit(0));
    assert_eq!(left.completed,right.completed);
    assert_eq!(left.pil_steps,right.pil_steps);
    assert_eq!(left.instructions,right.instructions);
    assert_eq!(left.loaded.cpu,right.loaded.cpu);
    assert_eq!(left.loaded.fpu.state(),right.loaded.fpu.state());
    assert!(left.loaded.ram.bytes()==right.loaded.ram.bytes());
    let base=right.loaded.image.segments[3].base;
    assert_eq!(right.loaded.ram.load(base+0x220,crate::MemWidth::Word).unwrap(),1);
    assert_eq!(right.loaded.ram.load(base+0x214,crate::MemWidth::Dword).unwrap(),0x3ff0_0000);
}

/// INC and DEC keep CF. A block that is only `dec cx; jnz` has no earlier
/// instruction of its own to write out the flags the previous block left
/// pending, so the fused pair has to keep that block's CF itself.
#[test]
fn fused_inc_and_dec_branches_keep_the_carry_of_the_previous_block(){
    const EXIT:[u8;5]=[0xb8,0,0x4c,0xcd,0x21];
    let programs:[(&str,Vec<u8>,u16);4]=[
        // MOV CX,5; MOV AX,1; MOV BX,2; CMP AX,BX (CF=1); JNE +0 | DEC CX; JNZ +0 | SBB DX,DX
        ("DEC keeps CF 1",vec![0xb9,5,0, 0xb8,1,0, 0xbb,2,0, 0x39,0xd8, 0x75,0x00, 0x49, 0x75,0x00, 0x19,0xd2],0xffff),
        // The same with INC CX.
        ("INC keeps CF 1",vec![0xb9,5,0, 0xb8,1,0, 0xbb,2,0, 0x39,0xd8, 0x75,0x00, 0x41, 0x75,0x00, 0x19,0xd2],0xffff),
        // STC; ... CMP AX,BX (2-1: CF=0); JNE +0 | INC CX; JNZ +0 | SBB DX,DX: the stale CF of STC must not show.
        ("INC keeps CF 0",vec![0xf9, 0xb9,5,0, 0xb8,2,0, 0xbb,1,0, 0x39,0xd8, 0x75,0x00, 0x41, 0x75,0x00, 0x19,0xd2],0x0000),
        // DEC ECX; JNZ rel32 with operand-size prefixes.
        ("DEC ECX with JNZ rel32",vec![0x66,0xb9,5,0,0,0, 0xb8,1,0, 0xbb,2,0, 0x39,0xd8, 0x75,0x00,
            0x66,0x49, 0x66,0x0f,0x85,0,0,0,0, 0x19,0xd2],0xffff),
    ];
    for (name,mut bytes,expected) in programs{
        bytes.extend_from_slice(&EXIT);
        for mode in [Mode::Off,Mode::On]{
            let mut out=CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
            let r=dos::run_with_backend(load_with(&bytes,"INCDECCF.COM",mode),10_000,&mut out,None).unwrap();
            assert_eq!(r.stop,Stop::Exit(0),"{name} {mode:?}");
            assert_eq!(r.loaded.cpu.regs[2]&0xffff,u32::from(expected),"{name} {mode:?}");
            // The pair really was fused, or the test would prove nothing.
            if mode==Mode::On{assert!(r.loaded.blocks.compare_branches>=2,"{name}: {}",r.loaded.blocks.compare_branches);}
        }
        same_run(&bytes,"INCDECCF.COM",40);
    }
}
