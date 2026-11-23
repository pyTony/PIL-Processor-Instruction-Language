//! The AT diskette controller (NEC uPD765 / Intel 82077 command set) and
//! DMA channel 2 that carries its data, for an operating system that drives
//! the controller itself (Dine's DOS and BIOS use INT 13h on the images).
//! Commands complete at once: the result and IRQ6 are there as soon as the
//! last command byte is written. Disks are the BIOS drives' images
//! (`bios_disk`), so a write changes the image in memory only.
//!
//! Ports: 3F2h digital output (drive select, reset, DMA/IRQ enable,
//! motors), 3F4h main status (read) / data-rate select (write), 3F5h data,
//! 3F7h digital input (disk change) / configuration control (write).
use super::*;
use std::collections::VecDeque;

/// DMA channel 2 of the 8237: address 04h, count 05h, page 81h, and its
/// share of the mask (0Ah, 0Fh), mode (0Bh), flip-flop (0Ch) and master
/// clear (0Dh) registers.
#[derive(Clone,Debug,Default)]
struct Dma2{base:u16,addr:u16,count:u16,left:u16,page:u8,mode:u8,masked:bool,flip:bool,terminal:bool}
impl Dma2{
    fn io(&mut self,port:u16,input:bool,value:u8)->Option<u8>{
        let word=|w:&mut u16,flip:&mut bool,v:u8|{*w=if *flip{*w&0x00ff|u16::from(v)<<8}else{*w&0xff00|u16::from(v)};*flip=!*flip;};
        Some(match(port,input){
            (0x04,false)=>{word(&mut self.base,&mut self.flip,value);self.addr=self.base;0},
            (0x05,false)=>{word(&mut self.count,&mut self.flip,value);self.left=self.count;self.terminal=false;0},
            (0x04,true)=>{let b=if self.flip{(self.addr>>8)as u8}else{self.addr as u8};self.flip=!self.flip;b},
            (0x05,true)=>{let b=if self.flip{(self.left>>8)as u8}else{self.left as u8};self.flip=!self.flip;b},
            (0x81,false)=>{self.page=value;0},
            (0x81,true)=>self.page,
            _=>return None,
        })
    }
    /// The shared registers: the channel-2 part of a write (reads of them
    /// belong to the channel-1 model).
    fn shared(&mut self,port:u16,value:u8){
        match port{
            0x0a if value&3==2=>self.masked=value&4!=0,
            0x0b if value&3==2=>self.mode=value,
            0x0c=>self.flip=false,
            // Master clear resets the 8237, not the page register (a
            // separate latch on the AT).
            0x0d=>*self=Self{page:self.page,..Self::default()},
            0x0e=>self.masked=false,
            0x0f=>self.masked=value&4!=0,
            _=>{},
        }
    }
    /// The linear address of byte `offset` of the transfer (the 16-bit
    /// address wraps inside its 64 KiB page).
    fn at(&self,offset:usize)->u32{(u32::from(self.page)<<16)|u32::from(self.addr.wrapping_add(offset as u16))}
    fn bytes_left(&self)->usize{usize::from(self.left)+1}
    fn advance(&mut self,n:usize){
        self.addr=self.addr.wrapping_add(n as u16);
        if n>=self.bytes_left(){self.left=0xffff;self.terminal=true;}else{self.left-=n as u16;}
    }
}

#[derive(Clone,Debug,Default)]
pub(crate) struct Fdc{
    dor:u8,
    dma:Dma2,
    command:Vec<u8>,
    result:VecDeque<u8>,
    /// Present cylinder of each drive's head.
    cylinder:[u8;4],
    /// Interrupt status for SENSE INTERRUPT STATUS: ST0 and cylinder.
    sense:VecDeque<(u8,u8)>,
    /// IRQ6 raised and not yet delivered.
    pub(crate) irq:bool,
}

/// Command bytes (opcode included) by the opcode's low five bits.
fn length(op:u8)->usize{
    match op&0x1f{0x02|0x05|0x06|0x09|0x0c|0x11|0x19|0x1d=>9,0x03=>3,0x04|0x07|0x0a=>2,0x0d=>6,0x0f=>3,0x08|0x10=>1,_=>1}
}

impl Fdc{
    /// Port I/O for the controller and DMA channel 2; None for other ports.
    pub(super) fn io(p:&mut Loaded,port:u16,input:bool,value:u8)->Result<Option<u8>,String>{
        // Dine's DOS reads disks through INT 13h; the controller is the
        // booted OS's.
        if !p.booted{return Ok(None);}
        if let Some(v)=p.fdc.dma.io(port,input,value){return Ok(Some(v));}
        if !input&&matches!(port,0x0a..=0x0f){p.fdc.dma.shared(port,value);return Ok(None);}
        if !matches!(port,0x3f2|0x3f4|0x3f5|0x3f7){return Ok(None);}
        Ok(Some(match(port,input){
            (0x3f2,false)=>{
                let was=p.fdc.dor;p.fdc.dor=value;
                // Leaving reset (bit 2 from 0 to 1) resets the controller:
                // four drives report a status change, and IRQ6 follows.
                if was&4==0&&value&4!=0{
                    let f=&mut p.fdc;f.command.clear();f.result.clear();f.sense.clear();
                    for d in 0..4{f.sense.push_back((0xc0|d,f.cylinder[usize::from(d)]));}
                    f.raise();
                }
                0
            },
            (0x3f2,true)=>p.fdc.dor,
            // Main status: ready for a byte (RQM), with result bytes to read
            // (DIO, busy) or command bytes to take.
            (0x3f4,true)=>if !p.fdc.result.is_empty(){0xd0}else if !p.fdc.command.is_empty(){0x90}else{0x80},
            (0x3f4,false)=>0, // data rate select: the image has no rate
            (0x3f5,true)=>{
                let v=p.fdc.result.pop_front().unwrap_or(0xff);v
            },
            (0x3f5,false)=>{
                if !p.fdc.result.is_empty(){return Ok(Some(0));}
                p.fdc.command.push(value);
                if p.fdc.command.len()>=length(p.fdc.command[0]){
                    let cmd=std::mem::take(&mut p.fdc.command);
                    execute(p,&cmd)?;
                }
                0
            },
            // Disk change line clear: the disk in the drive stays.
            (0x3f7,true)=>0x00,
            (0x3f7,false)=>0,
            _=>unreachable!(),
        }))
    }
    fn raise(&mut self){if self.dor&8!=0{self.irq=true;}}
}

fn execute(p:&mut Loaded,cmd:&[u8])->Result<(),String>{
    let op=cmd[0];
    let drive=cmd.get(1).map_or(0,|b|b&3);let head=cmd.get(1).map_or(0,|b|b>>2&1);
    match op&0x1f{
        0x03=>{}, // SPECIFY: step rate, head load and DMA mode
        0x04=>{
            // SENSE DRIVE STATUS: ST3 ready, two-sided, track 0.
            let present=p.disks.has_floppy(drive);
            let st3=drive|head<<2|if present{0x28}else{0}|if p.fdc.cylinder[usize::from(drive)]==0{0x10}else{0};
            p.fdc.result.push_back(st3);
        },
        0x07=>{
            // RECALIBRATE: to cylinder 0, seek end.
            p.fdc.cylinder[usize::from(drive)]=0;
            let st0=0x20|drive|if p.disks.has_floppy(drive){0}else{0x50};
            p.fdc.sense.push_back((st0,0));p.fdc.raise();
        },
        0x08=>{
            // SENSE INTERRUPT STATUS: the next pending status, else invalid.
            match p.fdc.sense.pop_front(){
                Some((st0,cyl))=>{p.fdc.result.push_back(st0);p.fdc.result.push_back(cyl);},
                None=>p.fdc.result.push_back(0x80),
            }
        },
        0x0f=>{
            p.fdc.cylinder[usize::from(drive)]=cmd[2];
            p.fdc.sense.push_back((0x20|head<<2|drive,cmd[2]));p.fdc.raise();
        },
        0x05|0x06=>transfer(p,cmd)?,
        0x0a=>{
            // READ ID: the next sector header under the head.
            let cyl=p.fdc.cylinder[usize::from(drive)];
            let st0=if p.disks.has_floppy(drive){head<<2|drive}else{0x48|head<<2|drive};
            p.fdc.result.extend([st0,0,0,cyl,head,1,2]);p.fdc.raise();
        },
        0x10=>p.fdc.result.push_back(0x80), // VERSION: an NEC uPD765A
        _=>p.fdc.result.push_back(0x80),     // invalid command: ST0 80h
    }
    Ok(())
}

/// READ DATA (06h) and WRITE DATA (05h), with MT (multi-track): sectors
/// from R to EOT on the head, then side 1 of the same cylinder with MT,
/// moved through DMA channel 2 until its count ends (terminal count) or
/// the track does. The result names the sector after the last one moved.
fn transfer(p:&mut Loaded,cmd:&[u8])->Result<(),String>{
    let (op,drive,head)=(cmd[0],cmd[1]&3,cmd[1]>>2&1);
    let (c,h,r,n,eot)=(cmd[2],cmd[3],cmd[4],cmd[5],cmd[6]);
    let write=op&0x1f==0x05;let multi=op&0x80!=0;
    let geometry=p.disks.floppy_geometry(drive);
    let fail=|p:&mut Loaded,st0:u8,st1:u8|{p.fdc.result.extend([st0|head<<2|drive,st1,0,c,h,r,n]);p.fdc.raise();};
    let Some((cylinders,heads,sectors))=geometry else{fail(p,0x48,0);return Ok(())};
    if n!=2||u32::from(c)>=cylinders||u32::from(h)>=heads||r==0||u32::from(r)>sectors{fail(p,0x40,0x04);return Ok(());}
    let (mut cur_h,mut cur_r)=(h,r);
    loop{
        if p.fdc.dma.bytes_left()<512||p.fdc.dma.masked{break;}
        let lba=(u32::from(c)*heads+u32::from(cur_h))*sectors+u32::from(cur_r)-1;
        // DMA addresses are physical: no page tables between disk and RAM.
        let dma=p.fdc.dma.clone();
        if write{
            let data=p.ram.physical(|ram|(0..512).map(|i|ram.load(dma.at(i),MemWidth::Byte).map(|b|b as u8)).collect::<Result<Vec<u8>,_>>())
                .map_err(|e|format!("diskette DMA read: {e:?}"))?;
            p.disks.write_sector(drive,lba,&data)?;
        }else{
            let data=p.disks.sector(drive,lba).ok_or("diskette sector outside the image")?.to_vec();
            p.ram.physical(|ram|data.iter().enumerate().try_for_each(|(i,b)|ram.store(dma.at(i),MemWidth::Byte,u32::from(*b))))
                .map_err(|e|format!("diskette DMA write: {e:?}"))?;
        }
        p.fdc.dma.advance(512);
        // Next sector: end of track goes to side 1 with MT, else ends.
        if cur_r==eot||u32::from(cur_r)==sectors{
            if multi&&cur_h==0&&heads>1{cur_h=1;cur_r=1;}else{cur_r=cur_r.wrapping_add(1);break;}
        }else{cur_r+=1;}
        if p.fdc.dma.terminal{break;}
    }
    // The ID after the last sector moved (uPD765 result ID table). Past
    // EOT the cylinder steps on: C+1 with H as it was without MT, C+1 and
    // H back to 0 after side 1 with MT; MT on a one-sided track goes to
    // side 1 of the same cylinder. A transfer that ran past the track's
    // end without terminal count ends abnormally (end of cylinder).
    let (rc,rh,rr)=if u32::from(cur_r)>u32::from(eot).min(sectors){
        if !multi{(c.wrapping_add(1),cur_h,1)}else if cur_h==1{(c.wrapping_add(1),0,1)}else{(c,1,1)}}else{(c,cur_h,cur_r)};
    // Terminal count ends the command normally; running out of track (or
    // of a masked or short DMA count) ends it abnormally.
    let (st0,st1)=if p.fdc.dma.terminal{(0,0)}else{(0x40,0x80)};
    p.fdc.result.extend([st0|cur_h<<2|drive,st1,0,rc,rh,rr,n]);
    p.fdc.raise();
    Ok(())
}

#[cfg(test)]
#[path="../tests/unit/dos_fdc.rs"]
mod tests;
