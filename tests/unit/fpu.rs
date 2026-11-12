use super::*;
use crate::{dos, memory::MemWidth};

fn instruction(bytes: &[u8]) -> Instruction {
    decode(|at| bytes.get(at as usize).copied().ok_or(())).unwrap().unwrap()
}

#[test]
fn fpu_envelope_lengths_cover_register_memory_sib_and_wait() {
    for (bytes, length) in [
        (&[0xdb,0xe3][..],2), (&[0xdf,0xe0][..],2),
        (&[0xd9,0x3e,0x00,0x02][..],4),
        (&[0x26,0xd9,0x7e,0xfe][..],4),
        (&[0x67,0xd9,0x3c,0x25,0,0,0,0][..],8),
        (&[0x67,0xdd,0xbc,0x8d,0,0,0,0][..],8),
        (&[0x66,0x9b,0xdb,0xe3][..],2),
    ] {
        let decoded=instruction(bytes);
        assert_eq!(decoded.length,length);
        assert_eq!(&decoded.bytes[..length as usize],&bytes[..length as usize]);
    }
}

#[test]
fn fpu_envelope_fetches_no_following_instruction() {
    assert_eq!(instruction(&[0x9b]).length,1);
    assert_eq!(instruction(&[0xdb,0xe3]).length,2);
    assert!(decode(|at| [0xd9].get(at as usize).copied().ok_or(())).is_err());
    assert!(decode(|at| [0x67,0xd9,0x3c].get(at as usize).copied().ok_or(())).is_err());
    for bytes in [&[0xf0,0xdb,0xe3][..],&[0xf3,0xdb,0xe3][..],&[0x66,0x66,0xdb,0xe3][..],&[0x90][..]] {
        assert!(decode(|at| bytes.get(at as usize).copied().ok_or(())).unwrap().is_none());
    }
}

#[test]
fn fpu_all_modrm_envelopes_consume_expected_displacements() {
    for addr32 in [false,true] {
        for m in 0..=255u8 {
            let mode=m>>6;let rm=m&7;
            let prefix=usize::from(addr32);
            let sib=usize::from(addr32&&mode!=3&&rm==4);
            // SIB=25h has no base, so mod=00 needs disp32.
            let displacement=match mode {
                3=>0,1=>1,2=>if addr32{4}else{2},
                _=>if addr32&&(rm==5||rm==4){4}else if !addr32&&rm==6{2}else{0},
            };
            let length=prefix+2+sib+displacement;
            let mut bytes=vec![];
            if addr32{bytes.push(0x67);}
            bytes.extend([0xd9,m]);
            if sib!=0{bytes.push(0x25);}
            bytes.resize(length,0);
            assert_eq!(instruction(&bytes).length as usize,length);
        }
    }
}

#[test]
fn fpu_esc_and_wait_obey_all_em_mp_ts_combinations_without_effects() {
    let (mut image,mut ram)=Image::load(&[0x90]).unwrap();
    let mut cpu=Cpu::default();cpu.regs=[0xa5a55a5a;8];cpu.eip=0x100;cpu.eflags=0xad7;
    let original=cpu.clone();let original_ram=ram.clone();
    let mut fpu=Fpu::default();
    assert!(!fpu.available());assert_eq!(fpu.equipment(0xffff),0xfffd);
    for msw in 0..16 {
        image.msw=msw;
        for bytes in [&[0xdb,0xe3][..],&[0xdf,0xe0][..],&[0xd9,0x3e,0,2][..],&[0x9b][..]] {
            let ins=instruction(bytes);
            let trap=if ins.opcode==0x9b{msw&2!=0&&msw&8!=0}else{msw&4!=0||msw&8!=0};
            assert_eq!(fpu.execute(&ins,&mut cpu,&mut ram,&image),
                if trap{Outcome::NotAvailable}else{Outcome::Completed});
            assert_eq!(cpu,original);assert_eq!(ram,original_ram);
        }
    }
}

#[test]
fn fpu_absent_probe_preserves_ax_and_memory_and_reaches_dos_exit() {
    let mut p=dos::load(&[0x9b,0xdb,0xe3,0xd9,0x3e,0,2,0xdf,0xe0,0x9b,0xcd,0x20],"fpu.com").unwrap();
    p.cpu.regs[0]=0xaabbffff;
    let at=p.image.segments[3].base+0x200;
    p.ram.store(at,MemWidth::Word,0xa55a).unwrap();
    let result=dos::run(p,20).unwrap();
    assert!(matches!(result.stop,dos::Stop::Exit(0)));
    assert_eq!(result.loaded.cpu.regs[0],0xaabbffff);
    assert_eq!(result.loaded.ram.load(at,MemWidth::Word).unwrap(),0xa55a);
}

#[test]
fn fpu_nm_without_handler_reports_unavailable_at_original_ip() {
    let mut p=dos::load(&[0xdb,0xe3],"fpu.com").unwrap();
    p.image.msw|=4;
    let original=p.cpu.clone();let ram=p.ram.clone();
    let result=dos::run(p,2).unwrap();
    match result.stop {
        dos::Stop::Fault(detail)=>{
            assert!(detail.contains("80387 unavailable"));
            assert!(detail.contains("#NM (vector 07h)"));
            assert!(detail.contains("bytes=db e3"));
        },
        other=>panic!("{other:?}"),
    }
    assert_eq!(result.loaded.cpu,original);assert_eq!(result.loaded.ram,ram);
    assert_eq!(result.completed,0);
}

#[test]
fn fpu_nm_guest_handler_receives_prefix_ip_without_retiring_instruction() {
    let mut p=dos::load(&[0x66,0xdb,0xe3,0x90,0x90,0x90,0x90,0x90,0xf4],"fpu.com").unwrap();
    p.image.msw|=4;
    let cs=p.selectors[1];
    p.ram.store(7*4,MemWidth::Dword,(u32::from(cs)<<16)|0x108).unwrap();
    let ss=p.image.segments[2].base;
    let sp=p.cpu.regs[4]&0xffff;
    let result=dos::run(p,1).unwrap();
    assert!(matches!(result.stop,dos::Stop::Budget));
    assert_eq!(result.loaded.cpu.eip,0x108);
    assert_eq!(result.loaded.ram.load(ss+sp-6,MemWidth::Word).unwrap(),0x100);
    assert_eq!(result.completed,0);
    assert_eq!(result.loaded.cpu.eflags&0x300,0);
}

#[test]
fn fpu_backend_survives_session_slices_and_reports_absence_in_bios() {
    let p=dos::load(&[0xdb,0xe3,0x9b,0xcd,0x11,0xcd,0x20],"fpu.com").unwrap();
    let first=dos::run(p,1).unwrap();
    assert!(matches!(first.stop,dos::Stop::Budget));
    assert_eq!(first.loaded.cpu.eip,0x102);
    let result=dos::run(first.loaded.clone(),10).unwrap();
    assert!(matches!(result.stop,dos::Stop::Exit(0)));
    assert_eq!(result.loaded.cpu.regs[0]&2,0);
    assert!(!result.loaded.fpu.available());
}


#[test]
fn present_80387_executes_register_stack_com_program() {
    // FNINIT; FLD1; FLDZ; FXCH ST(1); FSTP ST(1); FNSTSW AX; INT 20h.
    let bytes=[0xdb,0xe3, 0xd9,0xe8, 0xd9,0xee, 0xd9,0xc9,
               0xdd,0xd9, 0xdf,0xe0, 0xcd,0x20];
    let mut p=dos::load(&bytes,"x87-register.com").unwrap();
    p.fpu=Fpu::present();
    p.cpu.regs[0]=0xabcd_ffff;
    let result=dos::run(p,20).unwrap();
    assert!(matches!(result.stop,dos::Stop::Exit(0)), "{:?}",result.stop);
    assert_eq!(result.loaded.cpu.regs[0],0xabcd_3800); // TOP=7
    let state=result.loaded.fpu.state().unwrap();
    assert_eq!(state.control,0x037f);
    assert_eq!(state.top(),7);
    assert_eq!(state.st(0),Some(Extended80::from_bits(1u64<<63,0x3fff)));
    assert_eq!(state.st(1),None);
}

#[test]
fn present_80387_reports_equipment_and_persists_across_slices() {
    let bytes=[0xdb,0xe3, 0xd9,0xe8, 0xdf,0xe0, 0xcd,0x11, 0xcd,0x20];
    let mut p=dos::load(&bytes,"x87-slice.com").unwrap();
    p.fpu=Fpu::present();
    let first=dos::run(p,2).unwrap();
    assert!(matches!(first.stop,dos::Stop::Budget));
    assert_eq!(first.loaded.fpu.state().unwrap().top(),7);
    assert_eq!(first.loaded.fpu.state().unwrap().st(0),
               Some(Extended80::from_bits(1u64<<63,0x3fff)));
    let result=dos::run(first.loaded,10).unwrap();
    assert!(matches!(result.stop,dos::Stop::Exit(0)), "{:?}",result.stop);
    assert_eq!(result.loaded.cpu.regs[0]&0xffff,0x27); // BIOS INT11h
    assert_eq!(result.loaded.fpu.state().unwrap().top(),7);
}

#[test]
fn present_80387_rejects_unimplemented_division_without_retiring() {
    let mut p=dos::load(&[0xdb,0xe3,0xd8,0xf0,0xcd,0x20],"x87-unsupported.com").unwrap();
    p.fpu=Fpu::present();
    let result=dos::run(p,20).unwrap();
    match result.stop {
        dos::Stop::Fault(message)=>{
            assert!(message.contains("80387 unsupported instruction"),"{message}");
            assert!(message.contains("d8 f0"),"{message}");
        },
        other=>panic!("{other:?}"),
    }
    assert_eq!(result.completed,1);
    assert_eq!(result.loaded.cpu.eip,0x102);
    assert_eq!(result.loaded.fpu.state().unwrap().tag_word(),0xffff);
}

#[test]
fn present_80387_stack_overflow_is_reported_without_mutation() {
    let mut code=vec![0xdb,0xe3];
    for _ in 0..9 { code.extend([0xd9,0xe8]); }
    code.extend([0xcd,0x20]);
    let mut p=dos::load(&code,"x87-overflow.com").unwrap();
    p.fpu=Fpu::present();
    let result=dos::run(p,30).unwrap();
    assert!(matches!(result.stop,dos::Stop::Fault(ref s) if s.contains("stack fault (Overflow)")));
    assert_eq!(result.completed,9); // FNINIT + eight successful pushes
    assert_eq!(result.loaded.cpu.eip,0x112);
    assert_eq!(result.loaded.fpu.state().unwrap().top(),0);
    assert_eq!(result.loaded.fpu.state().unwrap().tag_word(),0);
}

#[test]
fn present_80387_em_trap_precedes_execution() {
    let mut p=dos::load(&[0xdb,0xe3],"x87-em.com").unwrap();
    p.fpu=Fpu::present();
    p.image.msw|=4; // EM (emulation): #NM even with device installed
    let result=dos::run(p,2).unwrap();
    assert!(matches!(result.stop,dos::Stop::Fault(ref s) if s.contains("#NM")));
    assert_eq!(result.loaded.cpu.eip,0x100);
    assert_eq!(result.loaded.fpu.state().unwrap().tag_word(),0xffff);
}


#[test]
fn present_80387_dos_loads_and_stores_ieee_f32_f64() {
    // FNINIT; FLD m64 [0200h]; FSTP m64 [0208h];
    // FLD m32 [0210h]; FSTP m32 [0214h]; INT 20h.
    let bytes=[0xdb,0xe3, 0xdd,0x06,0x00,0x02, 0xdd,0x1e,0x08,0x02,
               0xd9,0x06,0x10,0x02, 0xd9,0x1e,0x14,0x02, 0xcd,0x20];
    let mut p=dos::load(&bytes,"x87-ieee.com").unwrap();
    p.fpu=Fpu::present();
    let base=p.image.segments[3].base;
    let value64=0xbff8_0000_0000_0000u64; // -1.5
    p.ram.store(base+0x200,MemWidth::Dword,value64 as u32).unwrap();
    p.ram.store(base+0x204,MemWidth::Dword,(value64>>32)as u32).unwrap();
    p.ram.store(base+0x210,MemWidth::Dword,0x3f80_0001).unwrap();
    let result=dos::run(p,20).unwrap();
    assert!(matches!(result.stop,dos::Stop::Exit(0)),"{:?}",result.stop);
    let low=result.loaded.ram.load(base+0x208,MemWidth::Dword).unwrap()as u64;
    let high=result.loaded.ram.load(base+0x20c,MemWidth::Dword).unwrap()as u64;
    assert_eq!(low|(high<<32),value64);
    assert_eq!(result.loaded.ram.load(base+0x214,MemWidth::Dword).unwrap(),0x3f80_0001);
    assert_eq!(result.loaded.fpu.state().unwrap().top(),0);
    assert_eq!(result.loaded.fpu.state().unwrap().tag_word(),0xffff);
}

#[test]
fn present_80387_fstp_m32_rounds_ties_to_even_and_sets_precision_flag() {
    // FLD m64; FSTP m32; INT 20h. The double is 1+2^-24,
    // exactly between 1.0f32 and the next float; round to even 1.0.
    let bytes=[0xdd,0x06,0x00,0x02, 0xd9,0x1e,0x08,0x02, 0xcd,0x20];
    let mut p=dos::load(&bytes,"x87-round.com").unwrap();
    p.fpu=Fpu::present();
    let base=p.image.segments[3].base;
    p.ram.store(base+0x200,MemWidth::Dword,0x1000_0000).unwrap();
    p.ram.store(base+0x204,MemWidth::Dword,0x3ff0_0000).unwrap();
    let result=dos::run(p,15).unwrap();
    assert!(matches!(result.stop,dos::Stop::Exit(0)),"{:?}",result.stop);
    assert_eq!(result.loaded.ram.load(base+0x208,MemWidth::Dword).unwrap(),0x3f80_0000);
    assert_eq!(result.loaded.fpu.state().unwrap().status&0x20,0x20);
}

#[test]
fn present_80387_32bit_sib_addressing_uses_reg_scale_and_disp32() {
    // 67 DD 04 8D 00 02 00 00 = FLD m64 [ECX*4+0200h]
    // followed by FSTP m64 [0200h], so output overwrites the first slot.
    let bytes=[0x67,0xdd,0x04,0x8d,0,2,0,0, 0xdd,0x1e,0,2, 0xcd,0x20];
    let mut p=dos::load(&bytes,"x87-sib.com").unwrap();
    p.fpu=Fpu::present();p.cpu.regs[1]=2;
    let base=p.image.segments[3].base;
    p.ram.store(base+0x208,MemWidth::Dword,0x0000_0000).unwrap();
    p.ram.store(base+0x20c,MemWidth::Dword,0x4008_0000).unwrap(); // 3.0
    let result=dos::run(p,15).unwrap();
    assert!(matches!(result.stop,dos::Stop::Exit(0)),"{:?}",result.stop);
    assert_eq!(result.loaded.ram.load(base+0x200,MemWidth::Dword).unwrap(),0);
    assert_eq!(result.loaded.ram.load(base+0x204,MemWidth::Dword).unwrap(),0x4008_0000);
}

#[test]
fn present_80387_faulted_fstp_keeps_memory_and_stack_intact() {
    let bytes=[0xd9,0xe8, 0xdd,0x1e,0x00,0x02, 0xcd,0x20];
    let mut p=dos::load(&bytes,"x87-protected.com").unwrap();
    p.fpu=Fpu::present();
    let base=p.image.segments[3].base;
    p.ram.store(base+0x200,MemWidth::Dword,0xdeadbeef).unwrap();
    p.ram.store(base+0x204,MemWidth::Dword,0xcafebabe).unwrap();
    p.ram.protect(base+0x205,1).unwrap();
    let result=dos::run(p,15).unwrap();
    assert!(matches!(result.stop,dos::Stop::Fault(ref s) if s.contains("80387 memory operand")));
    assert_eq!(result.completed,1);
    assert_eq!(result.loaded.cpu.eip,0x102);
    assert_eq!(result.loaded.ram.load(base+0x200,MemWidth::Dword).unwrap(),0xdeadbeef);
    assert_eq!(result.loaded.ram.load(base+0x204,MemWidth::Dword).unwrap(),0xcafebabe);
    assert_eq!(result.loaded.fpu.state().unwrap().st(0),
               Some(Extended80::from_bits(1u64<<63,0x3fff)));
}


#[test]
fn runtime_fpu_selection_is_opt_in_updates_bios_and_rejects_late_changes(){
    // FNINIT; FLD1; INT 11h; INT 20h. The opt-in is a protocol command,
    // not a test-only Loaded field assignment.
    let code=[0xdb,0xe3,0xd9,0xe8,0xcd,0x11,0xcd,0x20];
    let mut default=dos::session::Session::new(dos::load(&code,"fpu-session.com").unwrap());
    let (snapshot,_)=default.command(1,"snapshot").unwrap();
    assert!(snapshot.contains("\"fpu_present\":false"),"{snapshot}");
    let (reply,_)=default.command(2,"run 16").unwrap();
    assert!(reply.contains("\"status\":\"exit\""),"{reply}");
    assert_eq!(default.loaded().unwrap().cpu.regs[0]&2,0);

    let mut enabled=dos::session::Session::new(dos::load(&code,"fpu-session.com").unwrap());
    let before=enabled.loaded().unwrap().ram.load(0x410,MemWidth::Word).unwrap();
    let (bad,_)=enabled.command(1,"fpu 8087").unwrap();
    assert!(bad.contains("\"ok\":false"),"{bad}");
    assert!(!enabled.loaded().unwrap().fpu.available());
    let (good,_)=enabled.command(2,"fpu 80387").unwrap();
    assert!(good.contains("\"ok\":true")&&good.contains("\"fpu_present\":true"),"{good}");
    assert_eq!(enabled.loaded().unwrap().ram.load(0x410,MemWidth::Word).unwrap(),before|2);
    let (done,_)=enabled.command(3,"run 16").unwrap();
    assert!(done.contains("\"status\":\"exit\""),"{done}");
    assert_eq!(enabled.loaded().unwrap().cpu.regs[0]&2,2);
    assert_eq!(enabled.loaded().unwrap().fpu.state().unwrap().st(0),
               Some(Extended80::from_bits(1u64<<63,0x3fff)));
    let (late,_)=enabled.command(4,"fpu none").unwrap();
    assert!(late.contains("\"ok\":false"),"{late}");
    assert!(enabled.loaded().unwrap().fpu.available());
}

#[test]
fn runtime_fpu_selection_rejects_switch_after_a_waiting_run_without_retirement(){
    let mut session=dos::session::Session::new(dos::load(&[0xcd,0x16,0xcd,0x20],"wait.com").unwrap());
    let (reply,_)=session.command(1,"run 1").unwrap();
    assert!(reply.contains("\"status\":\"waiting_input\""),"{reply}");
    let (reply,_)=session.command(2,"fpu 80387").unwrap();
    assert!(reply.contains("\"ok\":false"),"{reply}");
    assert!(!session.loaded().unwrap().fpu.available());
}

#[test]
fn present_80387_dos_register_arithmetic_pop_and_ieee_store() {
    // FNINIT; FLD1; FLD1; FADDP => 2; FLD1; FLD1; FADDP => 2;
    // FMULP ST(1),ST(0) => 4; FSTP m64; INT 20h.
    let bytes=[0xdb,0xe3, 0xd9,0xe8, 0xd9,0xe8, 0xde,0xc1,
               0xd9,0xe8, 0xd9,0xe8, 0xde,0xc1, 0xde,0xc9,
               0xdd,0x1e,0x00,0x02, 0xcd,0x20];
    let mut p=dos::load(&bytes,"x87-arith.com").unwrap();
    p.fpu=Fpu::present();
    let base=p.image.segments[3].base;
    let result=dos::run(p,32).unwrap();
    assert!(matches!(result.stop,dos::Stop::Exit(0)),"{:?}",result.stop);
    let low=u64::from(result.loaded.ram.load(base+0x200,MemWidth::Dword).unwrap());
    let high=u64::from(result.loaded.ram.load(base+0x204,MemWidth::Dword).unwrap());
    assert_eq!(low|(high<<32),0x4010_0000_0000_0000); // 4.0f64
    assert_eq!(result.loaded.fpu.state().unwrap().tag_word(),0xffff);
}

#[test]
fn present_80387_register_subtract_and_reverse_subtract() {
    // FLDZ; FLD1; FSUB ST0,ST1 = 1; FSTP m32
    // FLD1; FLDZ; FSUBR ST0,ST1 = 1; FSTP m32; INT 20h
    let bytes=[0xd9,0xee, 0xd9,0xe8, 0xd8,0xe1,
               0xd9,0x1e,0x00,0x02,
               0xd9,0xe8, 0xd9,0xee, 0xd8,0xe9,
               0xd9,0x1e,0x04,0x02, 0xcd,0x20];
    let mut p=dos::load(&bytes,"x87-sub.com").unwrap();
    p.fpu=Fpu::present();
    let base=p.image.segments[3].base;
    let result=dos::run(p,32).unwrap();
    assert!(matches!(result.stop,dos::Stop::Exit(0)),"{:?}",result.stop);
    assert_eq!(result.loaded.ram.load(base+0x200,MemWidth::Dword).unwrap(),0x3f80_0000);
    assert_eq!(result.loaded.ram.load(base+0x204,MemWidth::Dword).unwrap(),0x3f80_0000);
}

#[test]
fn present_80387_precision_masking_is_atomic_and_sets_pe_when_masked() {
    // 1 + 2^-64 is halfway between two extended80 numbers: tie to even 1.
    // D8 C1 is FADD ST(0), ST(1), using an already populated FPU stack.
    let mut p=dos::load(&[0xd8,0xc1,0xcd,0x20],"x87-pe.com").unwrap();
    let one=Extended80::from_bits(1u64<<63,0x3fff);
    let half_ulp=Extended80::from_bits(1u64<<63,0x3fff-64);
    let mut backend=PresentFpu::default();
    backend.state.push(half_ulp).unwrap();
    backend.state.push(one).unwrap();
    backend.state.control &= !0x0020; // precision exception unmasked
    let snapshot=backend.state.clone();
    p.fpu=Fpu::Present(backend);
    let rejected=dos::run(p.clone(),8).unwrap();
    assert!(matches!(rejected.stop,dos::Stop::Fault(ref s) if s.contains("80387 unsupported")));
    assert_eq!(rejected.completed,0);
    assert_eq!(rejected.loaded.cpu.eip,0x100);
    assert_eq!(rejected.loaded.fpu.state().unwrap(),&snapshot);
    if let Fpu::Present(ref mut backend)=p.fpu {
        backend.state.control|=0x0020;
    }
    let result=dos::run(p,8).unwrap();
    assert!(matches!(result.stop,dos::Stop::Exit(0)),"{:?}",result.stop);
    let state=result.loaded.fpu.state().unwrap();
    assert_eq!(state.st(0),Some(one));
    assert_eq!(state.st(1),Some(half_ulp));
    assert_eq!(state.status&0x20,0x20); // PE recorded, masked
    assert_eq!(state.status&0x200,0);  // C1: tied downward to even
}

#[test]
fn present_80387_memory_arithmetic_m32_m64_dos_com_roundtrip() {
    // FNINIT; FLD m64(1.5); FADD m64(2.25); FMUL m64(2.0);
    // FSUB m32(0.5); FSUBR m32(8.0); FSTP m64; INT 20h.
    // 8 - ((1.5+2.25)*2 - 0.5) = 1.0.
    let bytes=[0xdb,0xe3, 0xdd,0x06,0x00,0x02,
               0xdc,0x06,0x08,0x02, 0xdc,0x0e,0x10,0x02,
               0xd8,0x26,0x18,0x02, 0xd8,0x2e,0x1c,0x02,
               0xdd,0x1e,0x20,0x02, 0xcd,0x20];
    let mut p=dos::load(&bytes,"x87-memory-math.com").unwrap();
    p.fpu=Fpu::present();
    let base=p.image.segments[3].base;
    for (addr,value) in [(0x200,0x3ff8_0000_0000_0000u64),
                         (0x208,0x4002_0000_0000_0000),
                         (0x210,0x4000_0000_0000_0000)] {
        p.ram.store(base+addr,MemWidth::Dword,value as u32).unwrap();
        p.ram.store(base+addr+4,MemWidth::Dword,(value>>32) as u32).unwrap();
    }
    p.ram.store(base+0x218,MemWidth::Dword,0x3f00_0000).unwrap();
    p.ram.store(base+0x21c,MemWidth::Dword,0x4100_0000).unwrap();
    let result=dos::run(p,32).unwrap();
    assert!(matches!(result.stop,dos::Stop::Exit(0)),"{:?}",result.stop);
    assert_eq!(result.loaded.ram.load(base+0x220,MemWidth::Dword).unwrap(),0);
    assert_eq!(result.loaded.ram.load(base+0x224,MemWidth::Dword).unwrap(),0x3ff0_0000);
    assert_eq!(result.loaded.fpu.state().unwrap().tag_word(),0xffff);
}

#[test]
fn present_80387_memory_arithmetic_fault_preserves_stack_and_ip() {
    // FLD1; FADD m64 [0200h]; INT 20h. Last bytes exceed DS limit.
    let mut p=dos::load(&[0xd9,0xe8, 0xdc,0x06,0x00,0x02,
                          0xcd,0x20],"x87-memory-math-fault.com").unwrap();
    p.fpu=Fpu::present();
    let base=p.image.segments[3].base;
    p.ram.store(base+0x200,MemWidth::Dword,0).unwrap();
    p.ram.store(base+0x204,MemWidth::Dword,0x3ff0_0000).unwrap();
    p.image.segments[3].limit=0x205; // DS:0200..0207 crosses segment limit
    let result=dos::run(p,16).unwrap();
    assert!(matches!(result.stop,dos::Stop::Fault(ref s)
                     if s.contains("80387 memory operand")), "{:?}",result.stop);
    assert_eq!(result.completed,1);
    assert_eq!(result.loaded.cpu.eip,0x102);
    let state=result.loaded.fpu.state().unwrap();
    assert_eq!(state.st(0),Some(Extended80::from_bits(1u64<<63,0x3fff)));
    assert_eq!(state.status & 0x003f,0);
}
