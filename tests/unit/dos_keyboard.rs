//! Keyboard IRQ1 to a program's INT 09h handler, the BIOS INT 09h entry,
//! Ctrl+Break through INT 1Bh, and the VGA controllers outside planar modes.
use super::*;
use super::super::{load,run,Stop,PSP};
fn code_at(bytes:&[(usize,&[u8])])->Vec<u8>{
    let mut code=vec![0x90;0x80];
    for(at,b)in bytes{if code.len()<at+b.len(){code.resize(at+b.len(),0x90);}code[*at..at+b.len()].copy_from_slice(b);}
    code
}
const BASE:u32=(PSP as u32)*16;
// AH35/AH25: save the old INT 09h vector at 0200h, point it at 0140h.
const HOOK09:&[u8]=&[0xb8,0x09,0x35,0xcd,0x21,0x89,0x1e,0x00,0x02,0x8c,0x06,0x02,0x02,0xba,0x40,0x01,0xb8,0x09,0x25,0xcd,0x21];
// PUSH AX; PUSH BX; IN AL,60h; MOV BL,[CS:0211h]; XOR BH,BH;
// MOV [CS:BX+0212h],AL; INC BYTE [CS:0211h]; POP BX; POP AX; JMP FAR [CS:0200h]
const RECORDER:&[u8]=&[0x50,0x53,0xe4,0x60,0x2e,0x8a,0x1e,0x11,0x02,0x32,0xff,0x2e,0x88,0x87,0x12,0x02,0x2e,0xfe,0x06,0x11,0x02,0x5b,0x58,0x2e,0xff,0x2e,0x00,0x02];
fn program(after:&[u8])->Loaded{
    let mut main=HOOK09.to_vec();main.extend_from_slice(after);
    let mut p=load(&code_at(&[(0,&main),(0x40,RECORDER)]),"kbd.com").unwrap();p.cpu.eflags=0x202;p
}
fn recorded(p:&Loaded)->Vec<u8>{
    let n=p.ram.load(BASE+0x211,MemWidth::Byte).unwrap();
    (0..n).map(|k|p.ram.load(BASE+0x212+k,MemWidth::Byte).unwrap()as u8).collect()
}
fn ring(p:&Loaded)->Vec<u32>{
    let(mut head,tail)=(p.ram.load(0x41a,MemWidth::Word).unwrap(),p.ram.load(0x41c,MemWidth::Word).unwrap());
    let mut words=Vec::new();
    while head!=tail{words.push(p.ram.load(0x400+head,MemWidth::Word).unwrap());head=if head==0x3c{0x1e}else{head+2};}
    words
}
fn in_service(p:&mut Loaded)->u8{p.speaker.pic.write(0x20,0x0b).unwrap();let v=p.speaker.pic.read(0x20);p.speaker.pic.write(0x20,0x0a).unwrap();v}
#[test]
fn a_key_reaches_the_program_handler_through_port_60h_and_the_chained_bios_stores_its_word(){
    for cli in [false,true]{
        // STI or CLI, then INT 16h AH=00h; CLI; HLT.
        let mut p=program(&[if cli{0xfa}else{0xfb},0xb4,0x00,0xcd,0x16,0xfa,0xf4]);
        let r=run(p,100).unwrap();assert!(matches!(r.stop,Stop::WaitingForInput),"{:?}",r.stop);
        p=r.loaded;assert!(hooked(&p).unwrap());
        assert!(key(&mut p,0x1e61).unwrap());assert!(p.speaker.kbd.pending());
        let r=run(p,200).unwrap();assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
        let mut p=r.loaded;
        assert_eq!(p.cpu.regs[0]&0xffff,0x1e61,"INT 16h read the word the BIOS handler stored");
        // With IF=0 only the blocking read lets the make code in (the BIOS's
        // own STI); the release waits for the program to enable interrupts.
        let (codes,irqs)=if cli{(vec![0x1e],1)}else{(vec![0x1e,0x9e],2)};
        assert_eq!(recorded(&p),codes,"make, then release, through port 60h");
        assert_eq!(p.speaker.kbd.pending(),cli);
        assert_eq!((r.instructions.get("IRQ1 to INT 09h"),r.instructions.get("BIOS INT 09h (keyboard)")),(Some(&irqs),Some(&irqs)));
        assert!(ring(&p).is_empty());assert_eq!(in_service(&mut p)&2,0,"the BIOS handler sent EOI");
        assert!(!p.speaker.kbd.full(),"the program took each code");
    }
}
#[test]
fn ctrl_break_presses_ctrl_and_the_bios_handler_empties_the_ring_and_flags_the_break(){
    let mut p=program(&[0xfb,0xeb,0xfe]);
    let r=run(p,50).unwrap();p=r.loaded;
    console::enqueue(&mut p,0x1e61).unwrap();
    assert!(ctrl_break(&mut p).unwrap());
    let r=run(p,200).unwrap();assert!(matches!(r.stop,Stop::Budget));
    let mut p=r.loaded;
    assert_eq!(recorded(&p),vec![0x1d,0x46,0xc6,0x9d]);
    assert_eq!(ring(&p),vec![0],"emptied, then one 0000h word");
    assert_eq!(p.ram.load(0x471,MemWidth::Byte).unwrap()&0x80,0x80);
    assert_eq!(p.ram.load(0x417,MemWidth::Byte).unwrap()&4,0,"Ctrl went up again");
    assert_eq!(in_service(&mut p)&2,0);
    // With Ctrl already held (the host reported it), only Scroll Lock moves.
    p.ram.store(0x417,MemWidth::Byte,4).unwrap();p.ram.store(BASE+0x211,MemWidth::Byte,0).unwrap();
    assert!(ctrl_break(&mut p).unwrap());
    let r=run(p,200).unwrap();assert_eq!(recorded(&r.loaded),vec![0x46,0xc6]);
}
#[test]
fn a_program_int1bh_handler_runs_from_the_bios_keyboard_handler_which_then_sends_eoi(){
    // Hook INT 1Bh with INC BYTE [CS:0220h]; IRET at 0150h; STI; MOV AX,1234h; JMP $.
    let code=code_at(&[(0,&[0xba,0x50,0x01,0xb8,0x1b,0x25,0xcd,0x21,0xfb,0xb8,0x34,0x12,0xeb,0xfe]),(0x50,&[0x2e,0xfe,0x06,0x20,0x02,0xcf])]);
    let mut p=load(&code,"brk.com").unwrap();p.cpu.eflags=0x202;
    let r=run(p,20).unwrap();p=r.loaded;
    assert!(!hooked(&p).unwrap());assert!(vectors::reachable(&p,0x1b).unwrap());
    let sp=p.cpu.regs[4]&0xffff;
    assert!(ctrl_break(&mut p).unwrap());
    let r=run(p,200).unwrap();assert!(matches!(r.stop,Stop::Budget));
    let mut p=r.loaded;
    assert_eq!(p.ram.load(BASE+0x220,MemWidth::Byte).unwrap(),1,"INT 1Bh ran once");
    assert_eq!(r.instructions.get("BIOS INT 09h (Ctrl+Break, INT 1Bh)"),Some(&1));
    assert_eq!((p.cpu.regs[0]&0xffff,p.cpu.regs[4]&0xffff,p.cpu.eip),(0x1234,sp,0x10c));
    assert_eq!(ring(&p),vec![0]);assert_eq!(in_service(&mut p)&2,0,"the wrapper's EOI");
}
#[test]
fn irq1_waits_while_masked_in_service_behind_irq0_or_with_if_clear(){
    let mut p=program(&[0xfb,0xeb,0xfe]);
    let r=run(p,50).unwrap();p=r.loaded;
    key(&mut p,0x1e61).unwrap();
    p.speaker.pic.write(0x21,2).unwrap();assert!(!interrupt(&mut p,false).unwrap());
    p.speaker.pic.write(0x21,0).unwrap();p.speaker.pic.raise_timer();p.speaker.pic.acknowledge_timer();
    assert!(!interrupt(&mut p,false).unwrap(),"IRQ0 in service has priority");
    p.speaker.pic.finish_timer();p.cpu.eflags&=!0x200;
    assert!(!interrupt(&mut p,false).unwrap());
    let before=(p.cpu.clone(),p.speaker.kbd.clone());
    assert!(interrupt(&mut p,true).unwrap(),"the BIOS's own STI while it waits");
    assert_ne!(before.1,p.speaker.kbd);assert_eq!((p.cpu.eip,p.speaker.kbd.read()),(0x140,0x1e));
    // A full queue refuses more keys whole.
    let mut q=program(&[]);for _ in 0..16{assert!(key(&mut q,0x1e61).unwrap());}
    assert!(!key(&mut q,0x1e61).unwrap());assert!(!ctrl_break(&mut q).unwrap());
}
#[test]
fn bios_scan_codes_map_back_to_the_physical_keys(){
    for(scan,raw)in [(0x1e,0x1e),(0x3b,0x3b),(0x54,0x3b),(0x5e,0x3b),(0x71,0x44),(0x73,0x4b),(0x78,0x02),(0x83,0x0d),
        (0x85,0x57),(0x8c,0x58),(0x8d,0x48),(0x94,0x0f),(0x97,0x47),(0xa3,0x53),(0xa6,0x1c),(0,0xb8)]{
        assert_eq!(make(scan),raw,"scan {scan:02x}");
    }
}
#[test]
fn vga_controllers_outside_planar_modes_keep_steering_registers_and_refuse_layout_changes(){
    let mut p=load(&[0xcd,0x20],"vga.com").unwrap();
    let io=|p:&mut Loaded,port:u16,input:bool,value:u16,word:bool|super::super::planar::io(p,port,input,value,word);
    // GW-BASIC's sequence before SCREEN 9, in text mode 3.
    io(&mut p,0x3c4,false,0x0f02,true).unwrap();
    for v in [0x0001u16,0x0002,0x0f07,0x0005]{io(&mut p,0x3ce,false,v,true).unwrap();}
    io(&mut p,0x3c4,false,2,false).unwrap();assert_eq!(io(&mut p,0x3c5,true,0,false).unwrap(),Some(0x0f));
    io(&mut p,0x3ce,false,5,false).unwrap();assert_eq!(io(&mut p,0x3cf,true,0,false).unwrap(),Some(0));
    // Chain-4, graphics mode and the 256-colour bit stay refused, all or nothing
    // (the text-mode font window is in dos_vga_font's tests).
    let old=p.vga.idle.clone();
    assert!(io(&mut p,0x3c4,false,0x0e04,true).is_err());
    assert!(io(&mut p,0x3ce,false,0x0506,true).is_err());
    assert!(io(&mut p,0x3ce,false,0x4005,true).is_err());
    assert_eq!(p.vga.idle,old);
    // A mode set loads the mode's own values again.
    p.cpu.regs[0]=0x0003;console::display(&mut p,0).unwrap();
    assert_eq!(p.vga.idle,super::super::vga::Controllers::standard(3));
}
