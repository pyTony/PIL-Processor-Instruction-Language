use super::*;
use crate::dos::{run,Stop};

/// 10 cylinders of 16 heads and 63 sectors; LBA n starts with n as a word.
fn disk()->Vec<u8>{
    let mut d=vec![0u8;10*16*63*512];
    for n in 0..10*16*63{d[n*512..n*512+2].copy_from_slice(&(n as u16).to_le_bytes());}
    d
}
/// A booted machine: a 1.44 MB diskette whose boot sector is `code`, and
/// the fixed disk above (in memory only).
fn machine(code:&[u8])->Loaded{
    let mut floppy=vec![0u8;1_474_560];
    floppy[..code.len()].copy_from_slice(code);floppy[510]=0x55;floppy[511]=0xaa;
    let mut p=boot(&floppy,0).unwrap();
    p.disks.attach(0x80,bios_disk::Disk::fixed(disk()).unwrap()).unwrap();
    bios_disk::install_fixed_table(&mut p).unwrap();
    p
}
fn out(p:&mut Loaded,port:u16,value:u8){Ata::io(p,port,false,false,u16::from(value)).unwrap().unwrap();}
fn inp(p:&mut Loaded,port:u16)->u8{Ata::io(p,port,true,false,0).unwrap().unwrap() as u8}
fn word(p:&mut Loaded)->u16{Ata::io(p,0x1f0,true,true,0).unwrap().unwrap()}
/// IRQ14 pending at the slave PIC (IRR bit 6), then taken.
fn irq(p:&mut Loaded)->bool{let r=p.speaker.pic.read(0xa0)&0x40!=0;if r{p.speaker.pic.acknowledge_slave(6);p.speaker.pic.write(0xa0,0x20).unwrap();p.speaker.pic.write(0x20,0x20).unwrap();}r}

#[test]
fn reset_identify_and_the_bios_description(){
    let mut p=machine(&[0xf4]);
    // CMOS: drive C: is extended type 47; INT 41h: 10 cylinders, 16 heads,
    // 63 sectors; one fixed disk in the BIOS data area.
    assert_eq!((ports::cmos(&p,0x12).unwrap(),ports::cmos(&p,0x19).unwrap()),(0xf0,47));
    let table=p.ram.load(0x41*4,MemWidth::Dword).unwrap();
    let at=(table>>16)*16+(table&0xffff);
    assert_eq!((p.ram.load(at,MemWidth::Word).unwrap(),p.ram.load(at+2,MemWidth::Byte).unwrap(),p.ram.load(at+14,MemWidth::Byte).unwrap()),(10,16,63));
    assert_eq!(p.ram.load(0x475,MemWidth::Byte).unwrap(),1);
    // Software reset: busy while SRST is set, then ready with diagnostics
    // passed and no interrupt.
    out(&mut p,0x3f6,4);assert_eq!(inp(&mut p,0x1f7),0x80);
    out(&mut p,0x3f6,0);assert_eq!((inp(&mut p,0x1f7),inp(&mut p,0x1f1)),(0x50,1));assert!(!irq(&mut p));
    // IDENTIFY DRIVE: data ready with IRQ14; geometry, capacity, model.
    out(&mut p,0x1f6,0xa0);out(&mut p,0x1f7,0xec);
    assert!(irq(&mut p));assert_eq!(inp(&mut p,0x1f7),0x58);
    let w:Vec<u16>=(0..256).map(|_|word(&mut p)).collect();
    assert_eq!((w[0],w[1],w[3],w[6]),(0x40,10,16,63));
    assert_eq!(u32::from(w[60])|u32::from(w[61])<<16,10*16*63);
    assert_eq!(&w[27..29],&[u16::from_be_bytes(*b"PI"),u16::from_be_bytes(*b"L ")]);
    assert_eq!(inp(&mut p,0x1f7),0x50);
    // The absent slave reads 0 and takes no command.
    out(&mut p,0x1f6,0xb0);assert_eq!(inp(&mut p,0x1f7),0);out(&mut p,0x1f7,0x10);assert!(!irq(&mut p));
}

#[test]
fn a_chs_read_crosses_to_the_next_head(){
    let mut p=machine(&[0xf4]);
    // Two sectors from cylinder 1, head 15, sector 63: LBA 2015 and 2016
    // (cylinder 2, head 0, sector 1).
    out(&mut p,0x1f2,2);out(&mut p,0x1f3,63);out(&mut p,0x1f4,1);out(&mut p,0x1f5,0);out(&mut p,0x1f6,0xaf);
    out(&mut p,0x1f7,0x20);
    for lba in [2015u16,2016]{
        assert!(irq(&mut p),"IRQ14 for sector {lba}");assert_eq!(inp(&mut p,0x1f7),0x58);
        let first=word(&mut p);for _ in 1..256{word(&mut p);}
        assert_eq!(first,lba);
    }
    assert_eq!(inp(&mut p,0x1f7),0x50);assert!(!irq(&mut p));
    // The task file names the last sector read.
    assert_eq!((inp(&mut p,0x1f3),inp(&mut p,0x1f4),inp(&mut p,0x1f6)&0x0f,inp(&mut p,0x1f2)),(1,2,0,0));
}

#[test]
fn an_lba_write_and_errors(){
    let mut p=machine(&[0xf4]);
    // WRITE SECTORS at LBA 100: DRQ at once, no IRQ until the sector is in.
    out(&mut p,0x1f2,1);out(&mut p,0x1f3,100);out(&mut p,0x1f4,0);out(&mut p,0x1f5,0);out(&mut p,0x1f6,0xe0);
    out(&mut p,0x1f7,0x30);assert!(!irq(&mut p));assert_eq!(inp(&mut p,0x1f7),0x58);
    for i in 0..256u16{Ata::io(&mut p,0x1f0,false,true,0xab00|i).unwrap();}
    assert!(irq(&mut p));assert_eq!(inp(&mut p,0x1f7),0x50);
    let s=p.disks.sector(0x80,100).unwrap();assert_eq!((s[0],s[1],s[511]),(0x00,0xab,0xab));
    // A sector past the disk: ID not found. An unknown command: aborted.
    out(&mut p,0x1f3,0x00);out(&mut p,0x1f4,0x00);out(&mut p,0x1f5,0x10);out(&mut p,0x1f6,0xe0);out(&mut p,0x1f7,0x20);
    assert!(irq(&mut p));assert_eq!((inp(&mut p,0x1f7),inp(&mut p,0x1f1)),(0x51,0x10));
    out(&mut p,0x1f7,0xc6);assert!(irq(&mut p));assert_eq!((inp(&mut p,0x1f7),inp(&mut p,0x1f1)),(0x51,0x04));
    // nIEN masks the interrupt.
    out(&mut p,0x3f6,2);out(&mut p,0x1f7,0x10);assert!(!irq(&mut p));
}

#[test]
fn irq14_reaches_the_guest_handler(){
    // INT 76h at a handler that sets [0600h] and sends EOI to both PICs;
    // RECALIBRATE; wait with interrupts on; CLI; HLT.
    let code=[
        0xfa,0xc7,0x06,0xd8,0x01,0x1d,0x7c,0xc7,0x06,0xda,0x01,0x00,0x00,
        0xba,0xf7,0x01,0xb0,0x10,0xee,0xfb,
        0x80,0x3e,0x00,0x06,0x00,0x74,0xf9,0xfa,0xf4,
        0xc6,0x06,0x00,0x06,0x01,0xb0,0x20,0xe6,0xa0,0xe6,0x20,0xcf,
    ];
    let r=run(machine(&code),1000).unwrap();
    assert!(matches!(r.stop,Stop::Monitor(_)),"{:?}",r.stop);
    assert_eq!(r.loaded.ram.load(0x600,MemWidth::Byte).unwrap(),1);
    assert_eq!(r.loaded.speaker.pic.read(0xa0),0,"no request left");
}

#[test]
fn the_controller_belongs_to_a_booted_machine_with_a_fixed_disk(){
    let mut floppy=vec![0u8;1_474_560];floppy[510]=0x55;floppy[511]=0xaa;
    let mut p=boot(&floppy,0).unwrap();
    assert!(Ata::io(&mut p,0x1f7,true,false,0).unwrap().is_none());
    assert_eq!(ports::cmos(&p,0x12).unwrap(),0);
    let mut q=load(&[0xcd,0x20],"T").unwrap();
    assert!(Ata::io(&mut q,0x1f7,true,false,0).unwrap().is_none());
}

#[test]
fn a_fixed_disk_file_gets_its_written_sectors(){
    let path=std::env::temp_dir().join(format!("pil-ata-{}.img",std::process::id()));
    std::fs::write(&path,disk()).unwrap();
    let mut floppy=vec![0u8;1_474_560];floppy[510]=0x55;floppy[511]=0xaa;
    let mut p=boot_with_hard_disk(Some(&floppy),&path,0).unwrap();
    out(&mut p,0x1f2,1);out(&mut p,0x1f3,7);out(&mut p,0x1f6,0xe0);out(&mut p,0x1f7,0x30);
    for _ in 0..256{Ata::io(&mut p,0x1f0,false,true,0x5a5a).unwrap();}
    let file=std::fs::read(&path).unwrap();std::fs::remove_file(&path).unwrap();
    assert!(file[7*512..8*512].iter().all(|&b|b==0x5a));
    assert_eq!(&file[6*512..6*512+2],&6u16.to_le_bytes());
}

#[test]
fn a_second_fixed_disk_is_the_primary_slave(){
    let mut p=machine(&[0xf4]);
    // Disk 81h: 5 cylinders, LBA n starts with 1000+n.
    let mut second=vec![0u8;5*16*63*512];
    for n in 0..5*16*63{second[n*512..n*512+2].copy_from_slice(&(1000+n as u16).to_le_bytes());}
    p.disks.attach(0x81,bios_disk::Disk::fixed(second).unwrap()).unwrap();
    bios_disk::install_fixed_table(&mut p).unwrap();
    // CMOS: C: and D: extended type 47; INT 46h: 5 cylinders; two disks.
    assert_eq!((ports::cmos(&p,0x12).unwrap(),ports::cmos(&p,0x19).unwrap(),ports::cmos(&p,0x1a).unwrap()),(0xff,47,47));
    let table=p.ram.load(0x46*4,MemWidth::Dword).unwrap();
    let at=(table>>16)*16+(table&0xffff);
    assert_eq!((p.ram.load(at,MemWidth::Word).unwrap(),p.ram.load(at+2,MemWidth::Byte).unwrap(),p.ram.load(at+14,MemWidth::Byte).unwrap()),(5,16,63));
    assert_ne!(at,{let t=p.ram.load(0x41*4,MemWidth::Dword).unwrap();(t>>16)*16+(t&0xffff)});
    assert_eq!(p.ram.load(0x475,MemWidth::Byte).unwrap(),2);
    // INT 13h: AH=15h on 81h is a fixed disk (03h) of 5*16*63 sectors;
    // AH=08h counts two fixed disks.
    p.cpu.regs[0]=0x1500;p.cpu.regs[2]=0x81;bios_disk::int13(&mut p).unwrap();
    assert_eq!((p.cpu.regs[0]>>8&0xff,p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff),(3,0,5*16*63));
    p.cpu.regs[0]=0x0800;p.cpu.regs[2]=0x81;bios_disk::int13(&mut p).unwrap();
    assert_eq!((p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff),(0x043f,0x0f02));
    // The slave answers IDENTIFY with its own geometry.
    out(&mut p,0x1f6,0xb0);assert_eq!(inp(&mut p,0x1f7),0x50);out(&mut p,0x1f7,0xec);
    assert!(irq(&mut p));
    let w:Vec<u16>=(0..256).map(|_|word(&mut p)).collect();
    assert_eq!((w[1],w[3],w[6]),(5,16,63));
    // INITIALIZE DRIVE PARAMETERS on the slave (8 heads, 32 sectors) leaves
    // the master's geometry alone.
    out(&mut p,0x1f2,32);out(&mut p,0x1f6,0xb7);out(&mut p,0x1f7,0x91);assert!(irq(&mut p));
    // A CHS read from each: cylinder 0, head 1, sector 1 is LBA 32 on the
    // slave and LBA 63 on the master.
    for(head,first)in[(0xb1u8,1032u16),(0xa1,63)]{
        out(&mut p,0x1f2,1);out(&mut p,0x1f3,1);out(&mut p,0x1f4,0);out(&mut p,0x1f5,0);out(&mut p,0x1f6,head);
        out(&mut p,0x1f7,0x20);assert!(irq(&mut p));
        let w:Vec<u16>=(0..256).map(|_|word(&mut p)).collect();assert_eq!(w[0],first);
    }
    // A write to the slave reaches disk 81h only.
    out(&mut p,0x1f2,1);out(&mut p,0x1f3,9);out(&mut p,0x1f4,0);out(&mut p,0x1f5,0);out(&mut p,0x1f6,0xf0);out(&mut p,0x1f7,0x30);
    for _ in 0..256{Ata::io(&mut p,0x1f0,false,true,0x7777).unwrap();}
    assert!(irq(&mut p));
    assert!(p.disks.sector(0x81,9).unwrap().iter().all(|&b|b==0x77));
    assert_eq!(&p.disks.sector(0x80,9).unwrap()[..2],&9u16.to_le_bytes());
}
