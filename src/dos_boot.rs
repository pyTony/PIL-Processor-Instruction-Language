//! Booting a disk image: the machine without Dine's DOS. The BIOS reads the
//! boot sector to 0000:7C00 and jumps there with DL holding the boot drive,
//! as the IBM AT BIOS does at the end of POST (INT 19h). From then on only
//! BIOS services answer INT n; whatever operating system the disk carries
//! brings its own DOS, or none.
use super::*;

/// Vectors the BIOS itself serves after POST: timer and keyboard IRQs,
/// video, equipment, memory size, disk, serial, system services, keyboard,
/// printer, bootstrap, clock, Ctrl-Break and the user timer hook.
pub(super) const BIOS:[u8;14]=[0x08,0x09,0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x19,0x1a,0x1b,0x1c];
/// Vectors a BIOS points at a bare IRET: the CPU exceptions 00h-07h, the
/// master PIC's IRQs 2-7 (0Ah-0Fh) and the slave PIC's IRQs 70h-77h.
fn dummy(n:u8)->bool{n<8||(0x0a..=0x0f).contains(&n)||(0x70..=0x77).contains(&n)}

/// A machine that has booted `image` from BIOS drive `drive` (00h, the
/// first diskette, or 80h, the first fixed disk).
pub fn boot(image:&[u8],drive:u8)->Result<Loaded,String>{
    let disk=match drive{0|1=>bios_disk::Disk::floppy(image.to_vec())?,0x80=>bios_disk::Disk::fixed(image.to_vec())?,_=>return Err("boot drive must be 00h, 01h or 80h".into())};
    if drive==0x80{start(None,Some(disk),drive)}else{start(Some((drive,disk)),None,drive)}
}
/// Boot with a fixed disk read from `hard_disk` as drive 80h (its writes go
/// back to the file), and optionally a diskette image as drive A:; start
/// from drive 00h or 80h.
pub fn boot_with_hard_disk(floppy:Option<&[u8]>,hard_disk:&std::path::Path,drive:u8)->Result<Loaded,String>{
    let floppy=floppy.map(|f|bios_disk::Disk::floppy(f.to_vec())).transpose()?.map(|d|(0,d));
    if drive==0&&floppy.is_none(){return Err("booting from drive A: needs a diskette image".into());}
    start(floppy,Some(bios_disk::Disk::fixed_file(hard_disk)?),drive)
}
fn start(floppy:Option<(u8,bios_disk::Disk)>,fixed:Option<bios_disk::Disk>,drive:u8)->Result<Loaded,String>{
    let mut p=load(&[0xf4],"BOOT")?;
    // No XMS driver or upper memory: conventional memory to A000h, the
    // BIOS's own INT 15h AH=88h answer for the extended memory.
    p.select_xms(false)?;
    if let Some((at,disk))=floppy{p.disks.attach(at,disk)?;}
    if let Some(disk)=fixed{p.disks.attach(0x80,disk)?;}
    p.booted=true;
    // Nothing of Dine's DOS stays in memory: its data segment, the memory
    // chain, the program's PSP and environment are DOS, not the machine.
    p.ram.write_bytes(0x500,&vec![0;0xa0000-0x500]).map_err(|e|format!("clearing conventional memory: {e:?}"))?;
    for n in 0..=255u8{
        let vector=if BIOS.contains(&n){vectors::entry(n)}else if dummy(n){vectors::entry(n)+2}else{
            // Data pointers the BIOS sets (video parameters 1Dh, graphics
            // font 1Fh, EGA font 43h) keep their values; INT 1Eh is set below.
            if matches!(n,0x1d|0x1f|0x43){continue;}
            0
        };
        p.ram.store(u32::from(n)*4,MemWidth::Dword,vector).map_err(|e|format!("INT {n:02x}h vector: {e:?}"))?;
    }
    bios_disk::install_table(&mut p)?;
    bios_disk::install_fixed_table(&mut p)?;
    // Equipment word: diskette drives present (bit 0) and their number less
    // one (bits 7-6).
    let floppies=p.disks.floppies();
    let equipment=p.ram.load(0x410,MemWidth::Word).map_err(|e|format!("BIOS equipment read: {e:?}"))? as u16;
    let equipment=(equipment&!0x00c1)|if floppies>0{1|((floppies-1)&3)<<6}else{0};
    p.ram.store(0x410,MemWidth::Word,u32::from(equipment)).map_err(|e|format!("BIOS equipment write: {e:?}"))?;
    // All of memory may hold code once an operating system is running.
    p.image.fetch_base=0;p.image.start=0;p.image.end=p.ram.bytes().len() as u32;
    bios_disk::load_boot_sector(&mut p,drive)?;
    Ok(p)
}

/// The shown 80x25 text page as lines, for a headless run: from the CRTC
/// start address when a program set it (an OS console scrolls that way),
/// else from the BIOS page.
pub(crate) fn screen_lines(p:&Loaded)->Vec<String>{
    let origin=if p.vga.start_override{u32::from(p.vga.start())*2}else{0};
    (0..25u32).map(|row|{
        let line:String=(0..80u32).map(|col|{
            let c=p.ram.load(0xb8000+((origin+(row*80+col)*2)&0x7fff),MemWidth::Byte).unwrap_or(0x20) as u8;
            if (0x20..0x7f).contains(&c){char::from(c)}else{' '}
        }).collect();
        line.trim_end().to_string()
    }).collect()
}

/// Run a booted image headless for at most `steps` instructions and report
/// the screen and why the run stopped. `keys` are typed (ASCII; carriage
/// return is Enter) the first time the run waits for keyboard input or
/// after `keys_after` steps (a boot loader that polls the keyboard never
/// waits); a second wait for input ends the run. Each `typed` (STEPS, TEXT)
/// is typed once the run has made STEPS steps (an operating system that
/// idles in its own loop never waits as the BIOS does).
pub fn probe(mut p:Loaded,steps:usize,keys:&str,keys_after:usize,typed_at:&[(usize,String)],rtc_year:Option<u32>)->Result<String,String>{
    // The host clock drives the timer, so BIOS time-outs (a boot loader's
    // prompt) pass as they would on the machine.
    p.clock_offset=Some(0);
    if let Some(year)=rtc_year{p.set_rtc_year(year)?;}
    let mut session=session::Session::new(p);
    let (mut left,mut done,mut typed)=(steps,0usize,keys.is_empty());
    let mut later:Vec<&(usize,String)>=typed_at.iter().collect();later.sort_by_key(|t|t.0);
    let mut later=later.into_iter().peekable();
    let mut reason=String::from("budget");
    while left>0{
        let next=later.peek().map_or(usize::MAX,|t|t.0.saturating_sub(done).max(1));
        let slice=left.min(1_000_000).min(next).min(if typed{usize::MAX}else{keys_after.saturating_sub(done).max(1)});
        session.run(slice)?;done+=slice;left-=slice;
        while later.peek().is_some_and(|t|t.0<=done){let ran=type_text(&mut session,&later.next().unwrap().1)?;done+=ran;left=left.saturating_sub(ran);}
        let (status,detail)=session.status();
        let due=!typed&&(status=="waiting_input"||(status=="running"&&done>=keys_after));
        match status{
            _ if due=>{typed=true;let ran=type_text(&mut session,keys)?;done+=ran;left=left.saturating_sub(ran);},
            "running"=>{},
            _=>{reason=format!("{status} {detail}");break;},
        }
    }
    let p=session.machine().ok_or("session state lost")?;
    let mut report=String::new();
    for line in screen_lines(p){report.push_str(&line);report.push('\n');}
    let cs=p.selectors[SegmentRegister::Cs.index()];
    writeln!(report,"-- stop: {}; CS:EIP={cs:04x}:{:08x}; {} mode; at most {done} steps",reason.trim(),p.cpu.eip,if p.image.real_mode{"real"}else{"protected"}).unwrap();
    Ok(report)
}

/// Type ASCII text as US-layout key presses (Shift around capitals and
/// shifted symbols); carriage return or line feed is Enter. While the
/// keyboard queue is full the machine runs on in short slices; the steps
/// run are returned.
fn type_text(session:&mut session::Session,text:&str)->Result<usize,String>{
    let mut ran=0usize;
    let mut send=|session:&mut session::Session,code:u8,down:bool,word:Option<u16>,native:bool|->Result<(),String>{
        for _ in 0..1000{
            let taken=if native{session.native_key(code,false,down,word)?}else{let w=word.unwrap_or(0);session.key(w as u8,(w>>8) as u8)?};
            if taken{return Ok(());}
            session.run(10_000)?;ran+=10_000;
        }
        Err("the keyboard queue stays full".into())
    };
    for c in text.chars(){
        let (ascii,scan)=match c{'\r'|'\n'=>(13,0x1c),c if c.is_ascii()=>(c as u8,host_console::scan_of(c).unwrap_or(0)),_=>continue};
        let word=u16::from(scan)<<8|u16::from(ascii);
        let shift=c.is_ascii_uppercase()||host_console::SHIFTED.contains(c);
        if shift&&scan!=0{
            for (code,down,word) in [(0x2a,true,None),(scan,true,Some(word)),(scan,false,None),(0x2a,false,None)]{send(session,code,down,word,true)?;}
        }else{send(session,0,true,Some(word),false)?;}
    }
    Ok(ran)
}

#[cfg(test)]
#[path="../tests/unit/dos_boot.rs"]
mod tests;
