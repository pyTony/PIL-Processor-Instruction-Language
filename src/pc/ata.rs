//! The AT hard-disk controller (WD1003 / ATA task file) at 1F0h-1F7h and
//! 3F6h with IRQ14, for an operating system that drives the disk itself
//! (Dine's DOS uses its own drives; a booted loader uses INT 13h). Only a
//! booted machine with a fixed disk has it; elsewhere the ports stay an
//! open bus. The master on the primary channel is BIOS drive 80h's image
//! (`bios_disk`), the slave drive 81h's when there is one. Both drives
//! take every task-file write, as on the cable; the drive bit (bit 4 of
//! the drive/head register) says which one answers reads and commands.
//! An absent slave reads 0 and ignores commands.
//!
//! Commands complete at once: data is ready (DRQ) and IRQ14 raised as soon
//! as the command byte is written. Commands follow ATA-1 (X3.221-1994):
//! recalibrate (1xh), read sectors (20h-23h), write sectors (30h-33h),
//! read verify (40h, 41h), seek (7xh), execute diagnostics (90h),
//! initialize drive parameters (91h), identify drive (ECh), and the power
//! and set-features commands (E0h-E6h, EFh) with no effect. Others are
//! aborted (status ERR, error ABRT). CHS addresses use the geometry set by
//! 91h (else the image's); bit 6 of the drive/head register selects LBA.
use super::*;

const BUSY:u8=0x80;const READY:u8=0x40;const SEEK:u8=0x10;const DRQ:u8=0x08;const ERR:u8=0x01;
/// Error register bits: ID not found, command aborted.
const IDNF:u8=0x10;const ABRT:u8=0x04;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Transfer{None,Read{left:u32},Write{left:u32},Identify}

#[derive(Clone,Debug)]
pub(crate) struct Ata{
    error:u8,count:u8,sector:u8,cylinder:u16,head:u8,status:u8,control:u8,
    buffer:Box<[u8;512]>,at:usize,transfer:Transfer,
    /// Heads and sectors per track from INITIALIZE DRIVE PARAMETERS, per
    /// drive (master, slave).
    geometry:[Option<(u32,u32)>;2],
}
impl Default for Ata{
    fn default()->Self{Self{error:1,count:1,sector:1,cylinder:0,head:0,status:READY|SEEK,control:0,buffer:Box::new([0;512]),at:0,transfer:Transfer::None,geometry:[None;2]}}
}

impl Ata{
    /// Port I/O for the controller; None for other ports or without a disk.
    /// `word` is a 16-bit access, valid at the data port only.
    pub(crate) fn io<F>(p:&mut Machine<F>,port:u16,input:bool,word:bool,value:u16)->Result<Option<u16>,String>{
        if !p.booted||p.disks.fixed().is_none()||!matches!(port,0x1f0..=0x1f7|0x3f6){return Ok(None);}
        if word!=(port==0x1f0){return Err(format!("{} ATA access to port {port:03X}h is not modelled",if word{"word"}else{"byte"}));}
        // An absent slave's registers read 0 and it ignores commands.
        let slave=p.ata.head&0x10!=0&&p.disks.fixed_unit(1).is_none();
        Ok(Some(match(port,input){
            (0x1f0,true)=>data_in(p)?,
            (0x1f0,false)=>{data_out(p,value)?;0},
            (0x1f1,true)=>if slave{0}else{u16::from(p.ata.error)},
            (0x1f2,true)=>u16::from(p.ata.count),
            (0x1f3,true)=>u16::from(p.ata.sector),
            (0x1f4,true)=>u16::from(p.ata.cylinder as u8),
            (0x1f5,true)=>p.ata.cylinder>>8,
            (0x1f6,true)=>u16::from(p.ata.head|0xa0),
            // Reading the status register clears INTRQ; the 8259A has
            // latched its edge already.
            (0x1f7|0x3f6,true)=>if slave{0}else{u16::from(p.ata.status)},
            (0x1f1,false)=>0, // features / write precompensation
            (0x1f2,false)=>{p.ata.count=value as u8;0},
            (0x1f3,false)=>{p.ata.sector=value as u8;0},
            (0x1f4,false)=>{p.ata.cylinder=p.ata.cylinder&0xff00|value&0xff;0},
            (0x1f5,false)=>{p.ata.cylinder=p.ata.cylinder&0x00ff|(value&0xff)<<8;0},
            (0x1f6,false)=>{p.ata.head=value as u8;0},
            (0x1f7,false)=>{if !slave{command(p,value as u8)?;}0},
            (0x3f6,false)=>{control(&mut p.ata,value as u8);0},
            _=>unreachable!(),
        }))
    }
}

/// Device control: bit 2 holds the controller in reset (SRST), bit 1 masks
/// INTRQ (nIEN). Leaving reset ends any command and reports diagnostics
/// passed (error 01h) with the task file at its reset values.
fn control(a:&mut Ata,value:u8){
    let was=a.control;a.control=value;
    if value&4!=0{a.status=BUSY;a.transfer=Transfer::None;}
    else if was&4!=0{
        let geometry=a.geometry;*a=Ata{control:value,geometry,..Ata::default()};
    }
}

fn interrupt<F>(p:&mut Machine<F>){if p.ata.control&2==0{p.speaker.pic.raise_slave(6);}}

/// The selected drive: 0 master, 1 slave.
fn unit<F>(p:&Machine<F>)->u8{p.ata.head>>4&1}
/// Heads and sectors per track for CHS addresses, and the image's sectors,
/// of the selected drive.
fn layout<F>(p:&Machine<F>)->(u32,u32,u32){
    let (c,h,s)=p.disks.fixed_unit(unit(p)).map_or((0,1,1),|d|d.geometry());
    let (heads,sectors)=p.ata.geometry[usize::from(unit(p))].unwrap_or((h,s));
    (heads,sectors,c*h*s)
}
/// The sector the task file addresses, if the disk has it.
fn target<F>(p:&Machine<F>)->Option<u32>{
    let a=&p.ata;let(heads,sectors,total)=layout(p);
    let lba=if a.head&0x40!=0{u32::from(a.head&0x0f)<<24|u32::from(a.cylinder)<<8|u32::from(a.sector)}else{
        let(h,s)=(u32::from(a.head&0x0f),u32::from(a.sector));
        if h>=heads||s==0||s>sectors{return None;}
        (u32::from(a.cylinder)*heads+h)*sectors+s-1
    };
    (lba<total).then_some(lba)
}
/// Move the task file on to the next sector.
fn advance<F>(p:&mut Machine<F>){
    let(heads,sectors,_)=layout(p);let a=&mut p.ata;
    a.count=a.count.wrapping_sub(1);
    if a.head&0x40!=0{
        let lba=(u32::from(a.head&0x0f)<<24|u32::from(a.cylinder)<<8|u32::from(a.sector))+1;
        a.sector=lba as u8;a.cylinder=(lba>>8) as u16;a.head=a.head&0xf0|(lba>>24&0x0f) as u8;
    }else if u32::from(a.sector)<sectors{a.sector+=1;}
    else{
        a.sector=1;
        if u32::from(a.head&0x0f)+1<heads{a.head+=1;}else{a.head&=0xf0;a.cylinder=a.cylinder.wrapping_add(1);}
    }
}
fn fail<F>(p:&mut Machine<F>,error:u8){p.ata.error=error;p.ata.status=READY|SEEK|ERR;p.ata.transfer=Transfer::None;interrupt(p);}
fn done<F>(p:&mut Machine<F>){p.ata.status=READY|SEEK;p.ata.transfer=Transfer::None;interrupt(p);}
/// Load the addressed sector for the host to read, and announce it.
fn load_sector<F>(p:&mut Machine<F>,left:u32)->Result<(),String>{
    let Some(lba)=target(p)else{fail(p,IDNF);return Ok(())};
    // `target` keeps LBAs inside the whole cylinders the image holds.
    let Some(data)=p.disks.sector(0x80|unit(p),lba)else{
        p.ata.transfer=Transfer::None;p.ata.status=READY|SEEK;
        return Err(format!("fixed disk sector {lba} outside the image"));
    };
    p.ata.buffer.copy_from_slice(data);p.ata.at=0;
    p.ata.transfer=Transfer::Read{left};p.ata.status=READY|SEEK|DRQ;interrupt(p);Ok(())
}

fn command<F>(p:&mut Machine<F>,cmd:u8)->Result<(),String>{
    if p.ata.control&4!=0{return Ok(());}
    p.ata.error=0;
    let count=if p.ata.count==0{256}else{u32::from(p.ata.count)};
    match cmd{
        0x10..=0x1f=>{p.ata.cylinder=0;done(p);},
        0x20..=0x23=>load_sector(p,count)?,
        0x30..=0x33=>{
            if target(p).is_none(){fail(p,IDNF);return Ok(());}
            // The host writes the first sector without waiting for IRQ14.
            p.ata.at=0;p.ata.transfer=Transfer::Write{left:count};p.ata.status=READY|SEEK|DRQ;
        },
        0x40|0x41|0x70..=0x7f=>if target(p).is_some(){done(p)}else{fail(p,IDNF)},
        0x90=>{p.ata.error=1;done(p);},
        0x91=>{p.ata.geometry[usize::from(unit(p))]=Some((u32::from(p.ata.head&0x0f)+1,u32::from(p.ata.count)));done(p);},
        0xec=>{
            identify(p);p.ata.at=0;p.ata.transfer=Transfer::Identify;p.ata.status=READY|SEEK|DRQ;interrupt(p);
        },
        0xe0..=0xe6|0xef=>done(p),
        _=>fail(p,ABRT),
    }
    Ok(())
}

fn data_in<F>(p:&mut Machine<F>)->Result<u16,String>{
    if p.ata.status&DRQ==0||!matches!(p.ata.transfer,Transfer::Read{..}|Transfer::Identify){return Ok(0xffff);}
    let a=&mut p.ata;let v=u16::from_le_bytes([a.buffer[a.at],a.buffer[a.at+1]]);a.at+=2;
    if a.at==512{
        match a.transfer{
            Transfer::Read{left} if left>1=>{advance(p);load_sector(p,left-1)?;},
            Transfer::Read{..}=>{p.ata.status=READY|SEEK;p.ata.transfer=Transfer::None;p.ata.count=p.ata.count.wrapping_sub(1);},
            _=>{p.ata.status=READY|SEEK;p.ata.transfer=Transfer::None;},
        }
    }
    Ok(v)
}

fn data_out<F>(p:&mut Machine<F>,value:u16)->Result<(),String>{
    let Transfer::Write{left}=p.ata.transfer else{return Ok(())};
    if p.ata.status&DRQ==0{return Ok(());}
    let a=&mut p.ata;a.buffer[a.at..a.at+2].copy_from_slice(&value.to_le_bytes());a.at+=2;
    if a.at<512{return Ok(());}
    let Some(lba)=target(p)else{fail(p,IDNF);return Ok(())};
    let data=*p.ata.buffer;
    p.disks.write_sector(0x80|unit(p),lba,&data)?;
    if left>1{
        advance(p);
        if target(p).is_none(){fail(p,IDNF);return Ok(());}
        p.ata.at=0;p.ata.transfer=Transfer::Write{left:left-1};p.ata.status=READY|SEEK|DRQ;interrupt(p);
    }else{p.ata.count=p.ata.count.wrapping_sub(1);done(p);}
    Ok(())
}

/// IDENTIFY DRIVE data (ATA-1 word layout): fixed drive, the image's
/// geometry, serial, firmware and model strings (byte pairs swapped), LBA
/// supported, no READ/WRITE MULTIPLE, current geometry and capacity.
fn identify<F>(p:&mut Machine<F>){
    let (c,h,s)=p.disks.fixed_unit(unit(p)).map_or((0,1,1),|d|d.geometry());let total=c*h*s;
    let (ch,cs)=p.ata.geometry[usize::from(unit(p))].unwrap_or((h,s));
    let mut w=[0u16;256];
    w[0]=0x0040;w[1]=c as u16;w[3]=h as u16;w[4]=(512*s) as u16;w[5]=512;w[6]=s as u16;
    let text=|w:&mut [u16;256],at:usize,len:usize,t:&str|{
        let b:Vec<u8>=t.bytes().chain(std::iter::repeat(b' ')).take(len*2).collect();
        for i in 0..len{w[at+i]=u16::from(b[2*i])<<8|u16::from(b[2*i+1]);}
    };
    text(&mut w,10,10,"PIL0001");text(&mut w,23,4,"1.0");text(&mut w,27,20,"PIL AT FIXED DISK");
    w[20]=3;w[21]=1;w[49]=0x0200;w[51]=0x0200;w[53]=1;
    let current_cyl=(total/(ch*cs).max(1)).min(65535);
    w[54]=current_cyl as u16;w[55]=ch as u16;w[56]=cs as u16;
    let cap=current_cyl*ch*cs;w[57]=cap as u16;w[58]=(cap>>16) as u16;
    w[60]=total as u16;w[61]=(total>>16) as u16;
    for (i,v) in w.iter().enumerate(){p.ata.buffer[2*i..2*i+2].copy_from_slice(&v.to_le_bytes());}
}

