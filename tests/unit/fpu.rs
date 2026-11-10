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
fn present_80387_rejects_unimplemented_arithmetic_without_retiring() {
    let mut p=dos::load(&[0xdb,0xe3,0xd8,0xc0,0xcd,0x20],"x87-unsupported.com").unwrap();
    p.fpu=Fpu::present();
    let result=dos::run(p,20).unwrap();
    match result.stop {
        dos::Stop::Fault(message)=>{
            assert!(message.contains("80387 unsupported instruction"),"{message}");
            assert!(message.contains("d8 c0"),"{message}");
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
