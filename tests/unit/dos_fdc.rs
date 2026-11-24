use super::*;
use crate::pc::fdc::*;

/// A booted machine with a 1.44 MB diskette whose sector at LBA n starts
/// with the byte n (mod 256).
fn machine()->Loaded{
    let mut disk=vec![0u8;1_474_560];
    for n in 0..2880{disk[n*512]=n as u8;}
    disk[510]=0x55;disk[511]=0xaa;
    boot::boot(&disk,0).unwrap()
}
fn out(p:&mut Loaded,port:u16,value:u8){assert!(Fdc::io(p,port,false,value).unwrap().is_some()||matches!(port,0x0a..=0x0f));}
fn inp(p:&mut Loaded,port:u16)->u8{Fdc::io(p,port,true,0).unwrap().unwrap()}
fn command(p:&mut Loaded,bytes:&[u8]){for &b in bytes{assert_eq!(inp(p,0x3f4)&0xc0,0x80,"ready for a command byte");out(p,0x3f5,b);}}
/// Out of reset with DMA/IRQ and drive A:'s motor on, the four reset
/// statuses taken.
fn ready(p:&mut Loaded){out(p,0x3f2,0x1c);for _ in 0..4{command(p,&[0x08]);result(p);}p.fdc.irq=false;}
fn result(p:&mut Loaded)->Vec<u8>{let mut r=Vec::new();while inp(p,0x3f4)&0xd0==0xd0{r.push(inp(p,0x3f5));}r}

#[test]
fn reset_reports_four_drives_and_raises_irq6(){
    let mut p=machine();
    out(&mut p,0x3f2,0x00);out(&mut p,0x3f2,0x0c);
    assert!(p.fdc.irq);
    for d in 0..4{command(&mut p,&[0x08]);assert_eq!(result(&mut p),vec![0xc0|d,0]);}
    // Nothing pending: an invalid-command status.
    command(&mut p,&[0x08]);assert_eq!(result(&mut p),vec![0x80]);
    // Without DMA/IRQ enabled (DOR bit 3) a reset raises nothing.
    let mut q=machine();out(&mut q,0x3f2,0x00);out(&mut q,0x3f2,0x04);assert!(!q.fdc.irq);
}

#[test]
fn recalibrate_seek_and_sense_drive_status(){
    let mut p=machine();ready(&mut p);
    command(&mut p,&[0x03,0xdf,0x02]);assert!(result(&mut p).is_empty());
    command(&mut p,&[0x0f,0x00,0x05]);assert!(p.fdc.irq);
    command(&mut p,&[0x08]);assert_eq!(result(&mut p),vec![0x20,5]);
    command(&mut p,&[0x04,0x00]);assert_eq!(result(&mut p),vec![0x28],"ready, two-sided, not track 0");
    command(&mut p,&[0x07,0x00]);command(&mut p,&[0x08]);assert_eq!(result(&mut p),vec![0x20,0]);
    command(&mut p,&[0x04,0x00]);assert_eq!(result(&mut p),vec![0x38],"track 0");
    // No drive B: the recalibration fails (equipment check).
    command(&mut p,&[0x07,0x01]);command(&mut p,&[0x08]);assert_eq!(result(&mut p),vec![0x71,0]);
}

#[test]
fn a_multi_track_read_fills_memory_through_dma_channel_2(){
    let mut p=machine();ready(&mut p);
    // DMA channel 2: write to memory (mode 46h), 20000h, 36 sectors.
    out(&mut p,0x0c,0);out(&mut p,0x0b,0x46);
    for b in [0x00,0x00]{out(&mut p,0x04,b);}
    for b in [0xff,0x47]{out(&mut p,0x05,b);}
    out(&mut p,0x81,0x02);out(&mut p,0x0a,0x02);
    // READ DATA, MT and MFM: cylinder 1, head 0, sectors 1-18, then side 1.
    command(&mut p,&[0xe6,0x00,0x01,0x00,0x01,0x02,0x12,0x1b,0xff]);
    assert!(p.fdc.irq);
    let r=result(&mut p);
    assert_eq!(r,vec![0x04,0,0,2,0,1,2],"normal end on terminal count, next ID cylinder 2");
    // Cylinder 1 starts at LBA 36; the 36 sectors are LBA 36-71 in order.
    for k in 0..36u32{assert_eq!(p.ram.load(0x20000+k*512,MemWidth::Byte).unwrap(),36+k,"sector {k}");}
    // A single-side read to the end of the track without terminal count
    // ends abnormally (end of cylinder).
    for b in [0x00,0x00]{out(&mut p,0x04,b);}
    for b in [0xff,0xff]{out(&mut p,0x05,b);}
    command(&mut p,&[0x46,0x00,0x00,0x00,0x11,0x02,0x12,0x1b,0xff]);
    assert_eq!(result(&mut p),vec![0x40,0x80,0,1,0,1,2],"end of cylinder, next ID C+1, H, R=1");
}

/// Without MT a transfer that ends at EOT names cylinder C+1, the same
/// head and sector 1 (uPD765 result ID table), here with terminal count.
#[test]
fn a_single_track_read_to_eot_names_the_next_cylinder(){
    let mut p=machine();ready(&mut p);
    out(&mut p,0x0c,0);out(&mut p,0x0b,0x46);
    for b in [0x00,0x00]{out(&mut p,0x04,b);}
    for b in [0xff,0x03]{out(&mut p,0x05,b);}
    out(&mut p,0x81,0x02);out(&mut p,0x0a,0x02);
    // READ DATA, no MT: cylinder 0, head 1, sectors 17-18.
    command(&mut p,&[0x46,0x04,0x00,0x01,0x11,0x02,0x12,0x1b,0xff]);
    assert_eq!(result(&mut p),vec![0x04,0,0,1,1,1,2]);
    assert_eq!(p.ram.load(0x20000,MemWidth::Byte).unwrap(),18+16,"LBA 34");
    // A master clear leaves the page register.
    out(&mut p,0x0d,0);assert_eq!(inp(&mut p,0x81),0x02);
}

#[test]
fn the_controller_belongs_to_a_booted_machine_only(){
    let mut p=load(&[0xcd,0x20],"T.COM").unwrap();
    assert!(Fdc::io(&mut p,0x3f4,true,0).unwrap().is_none());
}
