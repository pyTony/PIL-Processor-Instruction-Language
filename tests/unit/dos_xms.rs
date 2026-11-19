use super::*;
use crate::dos::{load,run,Stop};

/// One driver call with AX, BX and DX (upper halves kept); returns (AX, BL).
fn x(p:&mut Loaded,ax:u16,bx:u16,dx:u16)->(u16,u8){
    for(reg,value)in [(0,ax),(3,bx),(2,dx)]{p.cpu.regs[reg]=(p.cpu.regs[reg]&0xffff0000)|u32::from(value);}
    call(p,(ax>>8)as u8).unwrap();
    (p.cpu.regs[0]as u16,p.cpu.regs[3]as u8)
}
fn dx(p:&Loaded)->u16{p.cpu.regs[2]as u16}
fn fresh()->Loaded{let mut p=load(&[0xfa,0xf4],"xms.com").unwrap();p.select_xms(true).unwrap();p}
/// Writes an XMS move structure at DS:0200h and points SI at it.
fn structure(p:&mut Loaded,length:u32,src:(u16,u32),dst:(u16,u32)){
    let base=p.image.segments[SegmentRegister::Ds.index()].base+0x200;
    let mut s=length.to_le_bytes().to_vec();
    s.extend(src.0.to_le_bytes());s.extend(src.1.to_le_bytes());s.extend(dst.0.to_le_bytes());s.extend(dst.1.to_le_bytes());
    p.ram.write_bytes(base,&s).unwrap();p.cpu.regs[6]=0x200;
}
fn far(segment:u16,offset:u16)->u32{u32::from(segment)<<16|u32::from(offset)}

#[test]
fn xms_version_query_allocate_lock_and_free(){
    let mut p=fresh();
    // 00h: XMS 3.00, the HMA exists. Upper register halves stay.
    p.cpu.regs[0]=0xabcd0000;assert_eq!(x(&mut p,0x0000,0,0),(0x0300,0));assert_eq!((p.cpu.regs[0]>>16,p.cpu.regs[3]&0xffff,dx(&p)),(0xabcd,0x0300,1));
    // 08h: 16 MiB less 1 MiB and the HMA, in one free block.
    assert_eq!(x(&mut p,0x0800,0xff,0),(15296,0));assert_eq!(dx(&p),15296);
    // 09h: 64 KiB and 0 KiB blocks; 0Ch locks the first at 1 MiB + 64 KiB.
    assert_eq!(x(&mut p,0x0900,0,64),(1,0));let a=dx(&p);
    assert_eq!(x(&mut p,0x0900,0,0),(1,0));let zero=dx(&p);assert_ne!(a,zero);
    assert_eq!(x(&mut p,0x0c00,0,a).0,1);assert_eq!((dx(&p),p.cpu.regs[3]as u16),(0x0011,0x0000));
    assert_eq!(x(&mut p,0x0c00,0,zero).0,1);
    // 0Eh: lock count, free handles (48 less 2), size.
    assert_eq!(x(&mut p,0x0e00,0,a),(1,46));assert_eq!(((p.cpu.regs[3]>>8)as u8,dx(&p)),(1,64));
    assert_eq!(x(&mut p,0x0800,0,0),(15232,0));
    // A locked block cannot be freed; unlock twice is AAh the second time.
    assert_eq!(x(&mut p,0x0a00,0,a),(0,0xab));
    assert_eq!(x(&mut p,0x0d00,0,a),(1,0));assert_eq!(x(&mut p,0x0d00,0,a),(0,0xaa));
    assert_eq!(x(&mut p,0x0a00,0,a),(1,0));assert_eq!(x(&mut p,0x0a00,0,a),(0,0xa2));
    // Invalid handles: 0, past the table, never allocated.
    for h in [0,49,0xffff,7]{assert_eq!(x(&mut p,0x0c00,0,h),(0,0xa2),"{h}");assert_eq!(x(&mut p,0x0e00,0,h),(0,0xa2));}
    assert_eq!(x(&mut p,0x0800,0,0),(15296,0));
    // Too large: A0h; then every byte allocated: 08h says so with AX=DX=0.
    assert_eq!(x(&mut p,0x0900,0,15297),(0,0xa0));
    assert_eq!(x(&mut p,0x0900,0,15296).0,1);assert_eq!(x(&mut p,0x0800,0,0),(0,0xa0));assert_eq!(dx(&p),0);
}

#[test]
fn xms_first_fit_reuses_gaps_and_runs_out_of_handles(){
    let mut p=fresh();
    let mut h=Vec::new();for _ in 0..3{x(&mut p,0x0900,0,100);h.push(dx(&p));}
    // Free the middle block: a 50 KiB block goes into its gap.
    x(&mut p,0x0a00,0,h[1]);x(&mut p,0x0900,0,50);let g=dx(&p);
    x(&mut p,0x0c00,0,g);assert_eq!(u32::from(dx(&p))<<16|(p.cpu.regs[3]&0xffff),(1088+100)*1024);
    // Largest and total after that: the tail, and the tail plus 50 KiB.
    assert_eq!(x(&mut p,0x0800,0,0).0,15296-300);assert_eq!(dx(&p),15296-300+50);
    // 48 handles in all; the 49th is A1h, even for 0 KiB.
    let mut fresh=fresh();for _ in 0..48{assert_eq!(x(&mut fresh,0x0900,0,0).0,1);}
    assert_eq!(x(&mut fresh,0x0900,0,0),(0,0xa1));assert_eq!(x(&mut fresh,0x0900,0,1),(0,0xa1));
}

#[test]
fn xms_move_between_conventional_and_extended_memory(){
    let mut p=fresh();assert!(!p.ram.a20_enabled());
    x(&mut p,0x0900,0,4);let h=dx(&p);
    let data:Vec<u8>=(0..64u32).map(|i|(i*5+1)as u8).collect();p.ram.write_bytes(0x30000,&data).unwrap();
    // Conventional 3000:0000 -> EMB offset 16.
    structure(&mut p,64,(0,far(0x3000,0)),(h,16));assert_eq!(x(&mut p,0x0b00,0,0),(1,0));
    assert_eq!(&p.ram.bytes()[0x110010..0x110050],&data[..]);assert!(!p.ram.a20_enabled(),"A20 stays as it was");
    // EMB offset 16 -> conventional 3100:0002 (segment:offset).
    structure(&mut p,64,(h,16),(0,far(0x3100,2)));assert_eq!(x(&mut p,0x0b00,0,0),(1,0));
    assert_eq!(&p.ram.bytes()[0x31002..0x31042],&data[..]);
    // Within one EMB, destination below the source (overlapping) is fine.
    structure(&mut p,64,(h,16),(h,8));assert_eq!(x(&mut p,0x0b00,0,0),(1,0));assert_eq!(&p.ram.bytes()[0x110008..0x110048],&data[..]);
    // A zero length succeeds and copies nothing.
    structure(&mut p,0,(h,0),(h,4096));let before=p.ram.clone();assert_eq!(x(&mut p,0x0b00,0,0),(1,0));assert!(p.ram==before);
}

#[test]
fn xms_move_refuses_bad_structures_without_writing(){
    let mut p=fresh();x(&mut p,0x0900,0,4);let h=dx(&p);
    for(length,src,dst,code)in [
        (3,(0,far(0x3000,0)),(h,0),0xa7),        // odd length
        (2,(9,0),(h,0),0xa3),                    // source handle
        (2,(h,0),(9,0),0xa5),                    // destination handle
        (2,(h,4097),(0,far(0x3000,0)),0xa4),     // source offset past the block
        (2,(0,far(0x3000,0)),(h,4097),0xa6),     // destination offset past it
        (4,(h,4094),(0,far(0x3000,0)),0xa7),     // length past the block
        (4,(0,far(0xffff,0xfffe)),(h,0),0xa7),   // past 10FFF0h (FFFF:FFFE is 10FFEEh)
        (64,(h,0),(h,32),0xa8),                  // forward overlap
    ]{
        structure(&mut p,length,src,dst);let before=p.ram.clone();
        assert_eq!(x(&mut p,0x0b00,0,0),(0,code),"{length} {src:x?} {dst:x?}");assert!(p.ram==before);
    }
    // The last two bytes below 10FFF0h still move.
    structure(&mut p,2,(0,far(0xffff,0xffee)),(h,0));assert_eq!(x(&mut p,0x0b00,0,0),(1,0));
}

#[test]
fn xms_a20_global_local_and_the_hma(){
    let mut p=fresh();
    assert_eq!(x(&mut p,0x0700,0xff,0),(0,0));
    assert_eq!(x(&mut p,0x0300,0,0),(1,0));assert!(p.ram.a20_enabled());assert_eq!(x(&mut p,0x0700,0,0).0,1);
    assert_eq!(x(&mut p,0x0400,0,0),(1,0));assert!(!p.ram.a20_enabled());
    // Local enables are counted: global disable and the first local disable
    // leave it on (94h); the last local disable turns it off.
    x(&mut p,0x0500,0,0);x(&mut p,0x0500,0,0);assert!(p.ram.a20_enabled());
    assert_eq!(x(&mut p,0x0400,0,0),(0,0x94));assert_eq!(x(&mut p,0x0600,0,0),(0,0x94));assert!(p.ram.a20_enabled());
    assert_eq!(x(&mut p,0x0600,0,0),(1,0));assert!(!p.ram.a20_enabled());
    assert_eq!(x(&mut p,0x0600,0,0),(0,0x82));
    // The HMA: one owner at a time.
    assert_eq!(x(&mut p,0x0200,0,0),(0,0x93));
    assert_eq!(x(&mut p,0x0100,0,0xffff),(1,0));assert_eq!(x(&mut p,0x0100,0,0xffff),(0,0x91));
    assert_eq!(x(&mut p,0x0200,0,0),(1,0));
    // DOS holds every UMB: 10h finds none (DX=0), 11h/12h name no block.
    assert_eq!(x(&mut p,0x1000,0,0x10),(0,0xb1));assert_eq!(dx(&p),0);
    for ax in [0x1100,0x1200]{assert_eq!(x(&mut p,ax,0,0xc801),(0,0xb2),"{ax:x}");}
    // Unknown functions: not implemented.
    for ax in [0x2000,0x8a00]{assert_eq!(x(&mut p,ax,0,0x10),(0,0x80),"{ax:x}");}
}

#[test]
fn xms_reallocate_in_place_by_moving_and_when_locked(){
    let mut p=fresh();
    x(&mut p,0x0900,0,4);let a=dx(&p);x(&mut p,0x0900,0,4);let b=dx(&p);
    let addr=|p:&mut Loaded,h|{x(p,0x0c00,0,h);let at=u32::from(dx(p))<<16|(p.cpu.regs[3]&0xffff);x(p,0x0d00,0,h);at};
    let first=addr(&mut p,a);p.ram.set_a20_enabled(true);p.ram.write_bytes(first,b"block A!").unwrap();p.ram.set_a20_enabled(false);
    // A is followed by B: growing it moves it (after B), contents kept.
    assert_eq!(x(&mut p,0x0f00,8,a),(1,0));let moved=addr(&mut p,a);assert_eq!(moved,(1088+8)*1024);
    assert_eq!(&p.ram.bytes()[moved as usize..][..8],b"block A!");
    // B is now last but one? It is followed by A: free A, then B grows in place.
    x(&mut p,0x0a00,0,a);let at=addr(&mut p,b);assert_eq!(x(&mut p,0x0f00,100,b),(1,0));assert_eq!(addr(&mut p,b),at);
    // Shrinking keeps the place; a locked block is ABh.
    assert_eq!(x(&mut p,0x0f00,1,b),(1,0));assert_eq!(addr(&mut p,b),at);
    x(&mut p,0x0c00,0,b);assert_eq!(x(&mut p,0x0f00,2,b),(0,0xab));x(&mut p,0x0d00,0,b);
    // Too large: A0h, block unchanged.
    assert_eq!(x(&mut p,0x0f00,0xffff,b),(0,0xa0));assert_eq!(x(&mut p,0x0e00,0,b).0,1);assert_eq!(dx(&p),1);
    assert_eq!(x(&mut p,0x0f00,1,99),(0,0xa2));
}

#[test]
fn xms_32_bit_functions(){
    let mut p=fresh();
    // 88h: EAX largest, EDX total, ECX highest address (end of RAM - 1).
    p.cpu.regs[0]=0x8800;call(&mut p,0x88).unwrap();assert_eq!((p.cpu.regs[0],p.cpu.regs[2],p.cpu.regs[1],p.cpu.regs[3]&0xff),(15296,15296,0xffffff,0));
    // 89h takes all of EDX; 8Eh returns CX free handles and EDX size.
    p.cpu.regs[2]=0x0001_0000;p.cpu.regs[0]=0x8900;call(&mut p,0x89).unwrap();assert_eq!(p.cpu.regs[0]&0xffff,0);assert_eq!(p.cpu.regs[3]&0xff,0xa0);
    p.cpu.regs[2]=200;call(&mut p,0x89).unwrap();assert_eq!(p.cpu.regs[0]&0xffff,1);let h=p.cpu.regs[2]&0xffff;
    p.cpu.regs[2]=h;call(&mut p,0x8e).unwrap();assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.regs[1]&0xffff,p.cpu.regs[2],(p.cpu.regs[3]>>8)&255),(1,47,200,0));
    // 8Fh: EBX new size.
    p.cpu.regs[3]=300;p.cpu.regs[2]=h;call(&mut p,0x8f).unwrap();assert_eq!(p.cpu.regs[0]&0xffff,1);
    p.cpu.regs[2]=h;call(&mut p,0x8e).unwrap();assert_eq!(p.cpu.regs[2],300);
}

#[test]
fn xms_program_finds_the_driver_and_calls_it_through_a_hook(){
    // MOV AX,4300h; INT 2Fh; MOV CL,AL; MOV AX,4310h; INT 2Fh;
    // MOV [0140h],BX; MOV [0142h],ES; MOV AH,09h; MOV DX,40h; CALL FAR [0140h];
    // MOV SI,DX; MOV AH,0Ch; CALL FAR [0140h]; CLI; HLT
    let mut code=vec![0xb8,0x00,0x43,0xcd,0x2f,0x88,0xc1,0xb8,0x10,0x43,0xcd,0x2f,0x89,0x1e,0x40,0x01,0x8c,0x06,0x42,0x01,
        0xb4,0x09,0xba,0x40,0x00,0xff,0x1e,0x40,0x01,0x89,0xd6,0xb4,0x0c,0xff,0x1e,0x40,0x01,0xfa,0xf4];
    code.resize(0x48,0);
    let r=run(load(&code,"xmsrun.com").unwrap(),40).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    let c=&r.loaded.cpu;
    assert_eq!((c.regs[1]&0xff,c.regs[6]&0xffff,c.regs[0]&0xffff,c.regs[2]&0xffff,c.regs[3]&0xffff),(0x80,1,1,0x0011,0));
    assert_eq!(r.loaded.selectors[SegmentRegister::Es.index()],0x0070);
    assert_eq!(r.instructions.get("XMS driver AH09"),Some(&1));assert_eq!(r.instructions.get("XMS driver AH0c"),Some(&1));
    // A hook patches the entry with JMP FAR to its own handler, which counts
    // the call and chains to entry+5.
    // MOV AX,4310h; INT 2Fh; MOV DI,BX; MOV AL,EAh; STOSB; MOV AX,0130h; STOSW;
    // MOV AX,CS; STOSW; MOV [0140h],BX; MOV [0142h],ES; MOV AH,00h;
    // CALL FAR [0140h]; CLI; HLT; then at 0130h: INC BYTE [0144h];
    // JMP FAR 0070:03D5h
    let mut code=vec![0xb8,0x10,0x43,0xcd,0x2f,0x89,0xdf,0xb0,0xea,0xaa,0xb8,0x30,0x01,0xab,0x8c,0xc8,0xab,
        0x89,0x1e,0x40,0x01,0x8c,0x06,0x42,0x01,0xb4,0x00,0xff,0x1e,0x40,0x01,0xfa,0xf4];
    code.resize(0x30,0);code.extend([0xfe,0x06,0x44,0x01,0xea,0xd5,0x03,0x70,0x00]);code.resize(0x48,0);
    let r=run(load(&code,"xmshook.com").unwrap(),60).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    let ds=r.loaded.image.segments[SegmentRegister::Ds.index()].base;
    assert_eq!((r.loaded.cpu.regs[0]&0xffff,r.loaded.cpu.regs[3]&0xffff,r.loaded.ram.bytes()[(ds+0x144)as usize]),(0x0300,0x0300,1));
}

#[test]
fn xms_entry_from_v86_mode_stops_and_removed_driver_is_not_answered(){
    let mut p=fresh();
    let cs=SegmentRegister::Cs.index();p.selectors[cs]=0x70;p.image.selectors[cs]=0x70;p.image.segments[cs]=SegmentCache::real_mode(0x70);p.cpu.eip=0x3d5;
    p.image.real_mode=false;p.cpu.eflags|=0x20000;
    assert!(trapped(&mut p).unwrap_err().contains("V86"));
    p.image.real_mode=true;p.cpu.eflags&=!0x20000;p.select_xms(false).unwrap();
    assert!(trapped(&mut p).unwrap_err().contains("no XMS driver"));
    p.cpu.regs[0]=0x4300;assert!(!multiplex(&mut p).unwrap());assert_eq!(p.cpu.regs[0],0x4300);
}
