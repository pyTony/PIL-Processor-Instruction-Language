//! Port I/O through Dine: the FM alias ports and the PIT timer edges.
use super::*;
use crate::pc::ports::*;
#[test]
fn fm_alias_word_cycles_preserve_dsp_reply_and_upper_ax(){
    for opcode in [0xef,0xed]{
        let mut p=load(&[opcode],"fm-alias.com").unwrap();
        p.cpu.regs[0]=0xa5a5_1234;p.cpu.regs[2]=0x228;
        p.sb.io(0x226,false,1,&p.ram,&mut p.speaker.pic).unwrap();
        p.sb.io(0x226,false,0,&p.ram,&mut p.speaker.pic).unwrap();
        let ram=p.ram.clone();
        assert_eq!(io(&mut p),Ok(Some(1)));
        assert_eq!(p.cpu.regs[0],if opcode==0xed{0xa5a5_ffff}else{0xa5a5_1234});
        assert_eq!(p.ram,ram);
        assert_eq!(p.sb.io(0x22a,true,0,&p.ram,&mut p.speaker.pic).unwrap(),Some(0xaa));
    }
}
#[test]fn programmed_edges_drive_bios_ticks_and_obey_interrupt_shadows(){
    let mut p=load(&[0xfa,0xf4],"pit.com").unwrap();p.speaker.pit.command(0x34,0).unwrap();p.speaker.pit.write(0x0b,0).unwrap();p.speaker.pit.write(0xe9,0).unwrap();
    let now=p.speaker.pit.interrupts(1+3*0xe90b)as u32;assert_eq!(now,3);
    services::timer_interrupt_at(&mut p,now).unwrap();assert_eq!(p.timer_ticks,3);assert_eq!(p.ram.load(0x46c,MemWidth::Dword).unwrap(),3);
    for shadow in 0..3{p.cpu.eflags=if shadow==0{2}else{0x202};p.image.sti_shadow=u8::from(shadow==1);p.image.ss_shadow=u8::from(shadow==2);services::timer_interrupt_at(&mut p,4).unwrap();assert_eq!(p.timer_ticks,3);}
    p.image.ss_shadow=0;p.image.sti_shadow=0;p.cpu.eflags=0x202;services::timer_interrupt_at(&mut p,4).unwrap();assert_eq!(p.timer_ticks,4);
}
