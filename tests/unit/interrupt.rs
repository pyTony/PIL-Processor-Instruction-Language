//! Real-mode exception and interrupt frames, checked against hand-computed
//! stack bytes rather than against the code under test.
use super::*;
use crate::guest::Image;

fn machine(sp:u32,flags:u32)->(Cpu,Image,Ram){
    let (mut image,_)=Image::load(&[0x90]).unwrap();
    let mut ram=Ram::new(0x110000);
    // CS=1000h IP=0123h, SS=2000h.
    for (i,s) in [(1usize,0x1000u16),(2,0x2000)]{image.selectors[i]=s;image.segments[i]=SegmentCache::real_mode(s);}
    image.end=0x100000;image.start=0;
    ram.store(0x21*4,MemWidth::Dword,0x3000_0456).unwrap();
    let cpu=Cpu{regs:[0x11,0x22,0x33,0x44,sp,0x66,0x77,0x88],eflags:flags,undefined_flags:0,eip:0x123};
    (cpu,image,ram)
}
fn word(ram:&Ram,at:u32)->u32{ram.load(at,MemWidth::Word).unwrap()}

#[test]
fn reset_idtr_covers_the_256_vector_table_at_zero(){
    assert_eq!(RESET_IDTR,(0x3ff,0));
    let (_,image,_)=machine(0x100,2);
    assert_eq!(image.tables[1],RESET_IDTR);
}

#[test]
fn the_frame_holds_flags_cs_and_return_ip_and_if_tf_are_cleared(){
    let (mut cpu,mut image,mut ram)=machine(0xff00_0100,0x0000_0ad7|0x300);
    cpu.undefined_flags=0x300;
    deliver_real(&mut cpu,&mut image,&mut ram,0x21,0x0125).unwrap();
    // SS base 20000h: FLAGS at SP-2, CS at SP-4, IP at SP-6.
    assert_eq!(word(&ram,0x200fe),0x0bd7);
    assert_eq!(word(&ram,0x200fc),0x1000);
    assert_eq!(word(&ram,0x200fa),0x0125);
    assert_eq!(cpu.regs[4],0xff00_00fa,"only SP changes; the upper ESP half stays");
    assert_eq!(cpu.eflags,0x08d7,"IF and TF cleared, the rest kept");
    assert_eq!(cpu.undefined_flags,0);
    assert_eq!((image.selectors[1],image.segments[1],cpu.eip),(0x3000,SegmentCache::real_mode(0x3000),0x456));
    assert_eq!(&cpu.regs[..4],&[0x11,0x22,0x33,0x44]);
}

#[test]
fn even_stack_pointers_below_six_wrap_within_the_segment(){
    for sp in [0u32,2,4]{
        let (mut cpu,mut image,mut ram)=machine(sp,2);
        deliver_real(&mut cpu,&mut image,&mut ram,0x21,0x200).unwrap();
        let at=|down:u32|0x20000+(sp.wrapping_sub(down)&0xffff);
        assert_eq!((word(&ram,at(2)),word(&ram,at(4)),word(&ram,at(6))),(2,0x1000,0x200),"SP={sp}");
        assert_eq!(cpu.regs[4],sp.wrapping_sub(6)&0xffff);
    }
}

#[test]
fn odd_stack_pointers_below_six_shut_down_without_any_change(){
    for sp in [1u32,3,5]{
        let (mut cpu,mut image,mut ram)=machine(sp,0x202);
        let before=(cpu.clone(),image.clone(),ram.bytes().to_vec());
        let e=deliver_real(&mut cpu,&mut image,&mut ram,0x21,0x200).unwrap_err();
        assert!(e.starts_with("shutdown"),"{e}");
        assert_eq!((cpu,image,ram.bytes().to_vec()),before,"SP={sp}");
    }
    // SP=7 pushes at 5, 3 and 1: no word straddles FFFFh.
    let (mut cpu,mut image,mut ram)=machine(7,2);
    deliver_real(&mut cpu,&mut image,&mut ram,0x21,0x200).unwrap();
    assert_eq!(cpu.regs[4],1);
}

#[test]
fn the_vector_comes_from_idtr_and_its_limit_is_checked(){
    let (mut cpu,mut image,mut ram)=machine(0x100,2);
    // LIDT moved the table to 8000h with room for 22h vectors.
    image.tables[1]=(0x87,0x8000);
    ram.store(0x8000+0x21*4,MemWidth::Dword,0x4000_0010).unwrap();
    assert_eq!(real_vector(&image,&ram,0x21).unwrap(),(0x4000,0x10));
    let e=real_vector(&image,&ram,0x22).unwrap_err();
    assert!(e.contains("beyond the IDTR limit"),"{e}");
    deliver_real(&mut cpu,&mut image,&mut ram,0x21,0x200).unwrap();
    assert_eq!((image.selectors[1],cpu.eip),(0x4000,0x10));
}

#[test]
fn readonly_stack_memory_stops_before_any_write(){
    let (mut cpu,mut image,mut ram)=machine(0x100,2);
    ram.protect(0x200fa,2).unwrap();
    let before=ram.bytes().to_vec();
    assert!(deliver_real(&mut cpu,&mut image,&mut ram,0x21,0x200).is_err());
    assert_eq!(ram.bytes(),&before[..]);
    assert_eq!(cpu.regs[4],0x100);
}

#[test]
fn delivery_outside_real_mode_is_refused(){
    let (mut cpu,mut image,mut ram)=machine(0x100,2);
    image.real_mode=false;
    assert!(deliver_real(&mut cpu,&mut image,&mut ram,0x21,0x200).unwrap_err().contains("outside real mode"));
    let (mut cpu,mut image,mut ram)=machine(0x100,0x20002);
    assert!(deliver_real(&mut cpu,&mut image,&mut ram,0x21,0x200).is_err());
}

#[test]
fn fault_vectors_map_from_execution_errors(){
    assert_eq!(vector(&ExecError::Exception{eip:0,vector:0}),Some(0));
    assert_eq!(vector(&ExecError::StackSegment{eip:0,error_code:0}),Some(12));
    assert_eq!(vector(&ExecError::GeneralProtection{eip:0,error_code:0}),Some(13));
    assert_eq!(vector(&ExecError::MemoryAccess{eip:0,address:0,bytes:1,write:true}),None,"a RAM bus fault is a model stop");
    assert_eq!(vector(&ExecError::InvalidContext),None);
    assert_eq!(vector(&ExecError::UnsupportedControl{eip:0,register:0,value:1}),None);
}
