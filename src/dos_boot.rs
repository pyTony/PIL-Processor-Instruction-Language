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
/// Vectors a BIOS points at a bare IRET: the CPU exceptions 00h-07h and
/// the slave PIC's IRQs 70h-77h.
fn dummy(n:u8)->bool{n<8||(0x70..=0x77).contains(&n)}

/// A machine that has booted `image` from BIOS drive `drive` (00h, the
/// first diskette, or 80h, the first fixed disk).
pub fn boot(image:&[u8],drive:u8)->Result<Loaded,String>{
    let disk=match drive{0|1=>bios_disk::Disk::floppy(image.to_vec())?,0x80=>bios_disk::Disk::fixed(image.to_vec())?,_=>return Err("boot drive must be 00h, 01h or 80h".into())};
    let mut p=load(&[0xf4],"BOOT.COM")?;
    // No XMS driver or upper memory: conventional memory to A000h, the
    // BIOS's own INT 15h AH=88h answer for the extended memory.
    p.select_xms(false)?;
    p.disks.attach(drive,disk)?;
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

/// The first 80x25 text page as lines, for a headless run.
pub(crate) fn screen_lines(p:&Loaded)->Vec<String>{
    (0..25u32).map(|row|{
        let line:String=(0..80u32).map(|col|{
            let c=p.ram.load(0xb8000+(row*80+col)*2,MemWidth::Byte).unwrap_or(0x20) as u8;
            if (0x20..0x7f).contains(&c){char::from(c)}else{' '}
        }).collect();
        line.trim_end().to_string()
    }).collect()
}

/// Run a booted image headless for `steps` instructions and report the
/// screen and why the run stopped.
pub fn probe(image:&[u8],drive:u8,steps:usize)->Result<String,String>{
    let mut p=boot(image,drive)?;
    // The host clock drives the timer, so BIOS time-outs (a boot loader's
    // prompt) pass as they would on the machine.
    p.clock_offset=Some(0);
    let mut out=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let (mut left,mut stop,mut done)=(steps,String::from("budget"),0usize);
    while left>0{
        let slice=left.min(MAX_STEPS);
        let r=run_with_backend(p,slice,&mut out,None)?;
        let ran=r.pil_steps.max(r.completed);done+=ran;
        p=r.loaded;
        if !matches!(r.stop,Stop::Budget){stop=format!("{:?}",r.stop);break;}
        left-=slice;
    }
    let mut report=String::new();
    for line in screen_lines(&p){report.push_str(&line);report.push('\n');}
    let cs=p.selectors[SegmentRegister::Cs.index()];
    writeln!(report,"-- stop: {stop}; CS:EIP={cs:04x}:{:08x}; {} mode; about {done} steps",p.cpu.eip,if p.image.real_mode{"real"}else{"protected"}).unwrap();
    Ok(report)
}
