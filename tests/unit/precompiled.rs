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
