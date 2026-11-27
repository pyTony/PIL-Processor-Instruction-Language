//! Disk images attached to the machine as drive media: the diskettes the
//! diskette controller reads and the fixed disks behind the AT controller.
//! The BIOS (INT 13h, bios/disk.rs) reads the same images.
#[derive(Clone,Debug)]
pub struct Disk{pub data:Vec<u8>,pub cylinders:u32,pub heads:u32,pub sectors:u32,
    /// The file a fixed disk came from: written sectors go back to it.
    pub file:Option<std::path::PathBuf>,
    /// That file, opened for writing at the first write and kept open (a
    /// copy of the machine shares it).
    pub handle:Option<std::sync::Arc<std::sync::Mutex<std::fs::File>>>}
impl Disk{
    /// A floppy image, its geometry from its size (the PC formats).
    pub fn floppy(data:Vec<u8>)->Result<Self,String>{
        let (cylinders,heads,sectors)=match data.len(){
            163_840=>(40,1,8),184_320=>(40,1,9),327_680=>(40,2,8),368_640=>(40,2,9),
            737_280=>(80,2,9),1_228_800=>(80,2,15),1_474_560=>(80,2,18),2_949_120=>(80,2,36),
            n=>return Err(format!("{n} bytes is not a PC floppy image size (160K-2.88M)")),
        };
        Ok(Self{data,cylinders,heads,sectors,file:None,handle:None})
    }
    /// A fixed-disk image: 16 heads and 63 sectors per track, the usual
    /// BIOS translation, and as many whole cylinders as the image holds.
    pub fn fixed(data:Vec<u8>)->Result<Self,String>{
        let cylinders=(data.len()/(16*63*512)) as u32;
        if cylinders==0||cylinders>1024{return Err(format!("{} bytes is not a fixed-disk image of 1-1024 cylinders (16 heads, 63 sectors)",data.len()));}
        Ok(Self{data,cylinders,heads:16,sectors:63,file:None,handle:None})
    }
    /// A fixed disk from a file, written back to it sector by sector.
    pub fn fixed_file(path:&std::path::Path)->Result<Self,String>{
        let data=std::fs::read(path).map_err(|e|format!("{}: {e}",path.display()))?;
        Ok(Self{file:Some(path.to_path_buf()),..Self::fixed(data)?})
    }
    /// Write whole sectors from `lba` on, in memory and to the file.
    pub fn write(&mut self,lba:u32,bytes:&[u8])->Result<(),String>{
        let at=lba as usize*512;
        self.data.get_mut(at..at+bytes.len()).ok_or("sector outside the image")?.copy_from_slice(bytes);
        if let Some(path)=&self.file{
            use std::io::{Seek,SeekFrom,Write};
            let fail=|e:std::io::Error|format!("{}: {e}",path.display());
            if self.handle.is_none(){
                let f=std::fs::OpenOptions::new().write(true).open(path).map_err(fail)?;
                self.handle=Some(std::sync::Arc::new(std::sync::Mutex::new(f)));
            }
            let mut f=self.handle.as_ref().unwrap().lock().map_err(|_|format!("{}: write lock poisoned",path.display()))?;
            f.seek(SeekFrom::Start(at as u64)).map_err(fail)?;f.write_all(bytes).map_err(fail)?;
        }
        Ok(())
    }
    pub fn geometry(&self)->(u32,u32,u32){(self.cylinders,self.heads,self.sectors)}
    /// The CMOS drive type of a floppy (register 10h): 1 360K, 2 1.2M,
    /// 3 720K, 4 1.44M, 5 2.88M.
    pub fn cmos_type(&self)->u8{
        match (self.cylinders,self.sectors){(40,_)=>1,(80,15)=>2,(80,9)=>3,(80,18)=>4,(80,36)=>5,_=>0}
    }
    pub fn lba(&self,cylinder:u32,head:u32,sector:u32)->Option<u32>{
        (cylinder<self.cylinders&&head<self.heads&&(1..=self.sectors).contains(&sector))
            .then(||(cylinder*self.heads+head)*self.sectors+sector-1)
    }
    pub fn total(&self)->u32{(self.data.len()/512) as u32}
}

/// The BIOS drives: floppies 00h and 01h, fixed disks 80h and 81h (the
/// master and slave on the AT controller).
#[derive(Clone,Debug,Default)]
pub struct Disks{pub floppy:[Option<Disk>;2],pub fixed:[Option<Disk>;2],
    /// Each diskette drive's change line: set when another disk goes in,
    /// cleared by a step of the head (a seek or recalibrate) or by the
    /// BIOS's change check.
    pub changed:[bool;2]}
impl Disks{
    pub fn is_empty(&self)->bool{self.floppy.iter().all(Option::is_none)&&self.fixed.iter().all(Option::is_none)}
    pub fn attach(&mut self,drive:u8,disk:Disk)->Result<(),String>{
        match drive{
            0|1=>self.floppy[usize::from(drive)]=Some(disk),0x80=>self.fixed[0]=Some(disk),
            0x81 if self.fixed[0].is_some()=>self.fixed[1]=Some(disk),
            0x81=>return Err("fixed disk 81h needs a disk 80h".into()),
            _=>return Err(format!("BIOS drive {drive:02x}h: only 00h, 01h, 80h and 81h")),
        }
        Ok(())
    }
    pub fn get(&self,drive:u8)->Option<&Disk>{match drive{0|1=>self.floppy[usize::from(drive)].as_ref(),0x80|0x81=>self.fixed[usize::from(drive&1)].as_ref(),_=>None}}
    pub fn get_mut(&mut self,drive:u8)->Option<&mut Disk>{match drive{0|1=>self.floppy[usize::from(drive)].as_mut(),0x80|0x81=>self.fixed[usize::from(drive&1)].as_mut(),_=>None}}
    /// Put another diskette of the same format into drive 00h or 01h (the
    /// drive type in CMOS stays). Its change line goes active.
    pub fn change_floppy(&mut self,drive:u8,disk:Disk)->Result<(),String>{
        let slot=self.floppy.get_mut(usize::from(drive)).ok_or_else(||format!("BIOS drive {drive:02x}h is not a diskette drive"))?;
        let old=slot.as_ref().ok_or_else(||format!("there is no diskette drive {drive:02x}h"))?;
        if old.geometry()!=disk.geometry(){return Err(format!("the new diskette is {}K, the drive's is {}K",disk.data.len()/1024,old.data.len()/1024));}
        *slot=Some(disk);self.changed[usize::from(drive)]=true;Ok(())
    }
    /// The drive's change line (a diskette went in since the last step).
    pub fn changed(&self,drive:u8)->bool{drive<2&&self.changed[usize::from(drive)]}
    /// A step of the head with a diskette in the drive clears its change line.
    pub fn stepped(&mut self,drive:u8){if self.has_floppy(drive){self.changed[usize::from(drive)]=false;}}
    /// CMOS register 10h: drive A: type in the high nibble, B: in the low.
    pub fn cmos_floppy_types(&self)->u8{
        let t=|d:&Option<Disk>|d.as_ref().map_or(0,Disk::cmos_type);
        t(&self.floppy[0])<<4|t(&self.floppy[1])
    }
    /// The number of floppy drives, for the equipment word.
    pub fn floppies(&self)->u16{self.floppy.iter().filter(|d|d.is_some()).count() as u16}
    pub fn has_floppy(&self,drive:u8)->bool{drive<2&&self.floppy[usize::from(drive)].is_some()}
    /// Cylinders, heads and sectors per track of a diskette drive's image.
    pub fn floppy_geometry(&self,drive:u8)->Option<(u32,u32,u32)>{
        let d=if drive<2{self.floppy[usize::from(drive)].as_ref()}else{None}?;Some((d.cylinders,d.heads,d.sectors))
    }
    pub fn write_sector(&mut self,drive:u8,lba:u32,data:&[u8])->Result<(),String>{
        self.get_mut(drive).ok_or("no disk in the drive")?.write(lba,&data[..512])
    }
    /// The first fixed disk (80h), when one is attached.
    pub fn fixed(&self)->Option<&Disk>{self.fixed[0].as_ref()}
    /// Fixed disk 0 (80h, master) or 1 (81h, slave), when attached.
    pub fn fixed_unit(&self,unit:u8)->Option<&Disk>{self.fixed.get(usize::from(unit))?.as_ref()}
    /// The number of fixed disks, for 0040:0075 and INT 13h AH=08h.
    pub fn fixed_count(&self)->u8{self.fixed.iter().filter(|d|d.is_some()).count() as u8}
    pub fn sector(&self,drive:u8,lba:u32)->Option<&[u8]>{
        let d=self.get(drive)?;d.data.get(lba as usize*512..lba as usize*512+512)
    }
}

