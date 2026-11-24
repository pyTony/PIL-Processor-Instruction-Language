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
    if drive==0x80{start(None,vec![disk],drive)}else{start(Some((drive,disk)),Vec::new(),drive)}
}
/// Boot with a fixed disk read from `hard_disk` as drive 80h (its writes go
/// back to the file), and optionally a diskette image as drive A:; start
/// from drive 00h or 80h.
#[cfg(test)]
pub fn boot_with_hard_disk(floppy:Option<&[u8]>,hard_disk:&std::path::Path,drive:u8)->Result<Loaded,String>{
    boot_with_hard_disks(floppy,&[hard_disk],drive)
}
/// Boot with one or two fixed disks read from files (80h, then 81h; their
/// writes go back to the files), and optionally a diskette image as drive
/// A:; start from drive 00h or 80h.
pub fn boot_with_hard_disks(floppy:Option<&[u8]>,hard_disks:&[&std::path::Path],drive:u8)->Result<Loaded,String>{
    let floppy=floppy.map(|f|bios_disk::Disk::floppy(f.to_vec())).transpose()?.map(|d|(0,d));
    if drive==0&&floppy.is_none(){return Err("booting from drive A: needs a diskette image".into());}
    if hard_disks.len()>2{return Err("the AT controller takes two fixed disks".into());}
    let fixed=hard_disks.iter().map(|d|bios_disk::Disk::fixed_file(d)).collect::<Result<Vec<_>,_>>()?;
    start(floppy,fixed,drive)
}
/// The machine the command line asks for: IMAGE booted from drive 00h (a
/// diskette) or 80h (a fixed disk read from its file), then the fixed
/// disks `--hd-image` and `--hd-folder` add, in that order (80h, 81h).
pub(crate) fn boot_command(image:&str,drive:u8,hard_disk:Option<&str>,folder:Option<&std::path::Path>)->Result<Loaded,String>{
    let mut fixed:Vec<&std::path::Path>=Vec::new();
    if drive==0x80{fixed.push(std::path::Path::new(image));}
    if let Some(disk)=hard_disk{fixed.push(std::path::Path::new(disk));}
    if let Some(folder)=folder{fixed.push(folder);}
    if fixed.len()>2{return Err("the AT controller takes two fixed disks (the boot disk, --hd-image, --hd-folder)".into());}
    if drive==0x80{return boot_with_hard_disks(None,&fixed,0x80);}
    let bytes=std::fs::read(image).map_err(|e|format!("{image}: {e}"))?;
    if fixed.is_empty(){boot(&bytes,0)}else{boot_with_hard_disks(Some(&bytes),&fixed,0)}
}
fn start(floppy:Option<(u8,bios_disk::Disk)>,fixed:Vec<bios_disk::Disk>,drive:u8)->Result<Loaded,String>{
    let mut p=load(&[0xf4],"BOOT")?;
    // No XMS driver or upper memory: conventional memory to A000h, the
    // BIOS's own INT 15h AH=88h answer for the extended memory.
    p.select_xms(false)?;
    if let Some((at,disk))=floppy{p.disks.attach(at,disk)?;}
    for(unit,disk)in fixed.into_iter().enumerate(){p.disks.attach(0x80+unit as u8,disk)?;}
    p.booted=true;
    // The operating system may program the VGA registers itself.
    p.vga.free=true;
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
/// Each `changes` (STEPS, IMAGE) puts another diskette into drive A: once
/// the run has made STEPS steps, as for a boot disk that asks for its root
/// disk.
pub fn probe(mut p:Loaded,steps:usize,keys:&str,keys_after:usize,typed_at:&[(usize,String)],changes:&[(usize,Vec<u8>)],rtc_year:Option<u32>,keymap:super::guest_keymap::Keymap)->Result<String,String>{
    // The host clock drives the timer, so BIOS time-outs (a boot loader's
    // prompt) pass as they would on the machine.
    p.clock_offset=Some(0);
    if let Some(year)=rtc_year{p.set_rtc_year(year)?;}
    let mut session=session::Session::new(p);
    session.keymap=keymap;
    let (mut left,mut done,mut typed)=(steps,0usize,keys.is_empty());
    enum Event<'a>{Type(&'a str),Floppy(&'a [u8])}
    let mut later:Vec<(usize,Event)>=typed_at.iter().map(|(at,text)|(*at,Event::Type(text)))
        .chain(changes.iter().map(|(at,image)|(*at,Event::Floppy(image)))).collect();
    later.sort_by_key(|t|t.0);
    let mut later=later.into_iter().peekable();
    let mut reason=String::from("budget");
    while left>0{
        let next=later.peek().map_or(usize::MAX,|t|t.0.saturating_sub(done).max(1));
        let slice=left.min(1_000_000).min(next).min(if typed{usize::MAX}else{keys_after.saturating_sub(done).max(1)});
        session.run(slice)?;done+=slice;left-=slice;
        while later.peek().is_some_and(|t|t.0<=done){
            match later.next().unwrap().1{
                Event::Type(text)=>{let ran=type_text(&mut session,text)?;done+=ran;left=left.saturating_sub(ran);},
                Event::Floppy(image)=>{
                    let disk=bios_disk::Disk::floppy(image.to_vec())?;
                    session.machine_mut().ok_or("session state lost")?.disks.change_floppy(0,disk)?;
                },
            }
        }
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

/// A diskette image of `size` bytes holding a ustar archive of `files`
/// (each stored under its own file name, mode 0644, owner root), as
/// `tar cf /dev/fd0` writes one; the rest of the diskette is zeros. A
/// system without a file system for the diskette reads it with
/// `cpio -i -H ustar </dev/fd0`.
pub fn tar_floppy(files:&[String],size:usize)->Result<Vec<u8>,String>{
    let mut out=Vec::new();
    for file in files{
        let data=std::fs::read(file).map_err(|e|format!("{file}: {e}"))?;
        let name=std::path::Path::new(file).file_name().and_then(|n|n.to_str()).ok_or_else(||format!("{file}: no file name"))?;
        if name.len()>99||!name.is_ascii(){return Err(format!("{name}: a ustar name holds up to 99 ASCII characters"));}
        let mut h=[0u8;512];
        let field=|h:&mut [u8;512],at:usize,text:&[u8]|h[at..at+text.len()].copy_from_slice(text);
        field(&mut h,0,name.as_bytes());
        field(&mut h,100,b"0000644\0");field(&mut h,108,b"0000000\0");field(&mut h,116,b"0000000\0");
        field(&mut h,124,format!("{:011o}\0",data.len()).as_bytes());
        // 1 January 1994, 00:00 UTC.
        field(&mut h,136,format!("{:011o}\0",757_382_400u32).as_bytes());
        field(&mut h,148,b"        ");h[156]=b'0';
        field(&mut h,257,b"ustar\0");field(&mut h,263,b"00");field(&mut h,265,b"root");field(&mut h,297,b"root");
        let sum:u32=h.iter().map(|&b|u32::from(b)).sum();
        field(&mut h,148,format!("{sum:06o}\0 ").as_bytes());
        out.extend_from_slice(&h);out.extend_from_slice(&data);
        out.resize(out.len().div_ceil(512)*512,0);
    }
    out.resize(out.len()+1024,0);
    if out.len()>size{return Err(format!("the archive takes {} bytes; the diskette holds {size}",out.len()));}
    out.resize(size,0);
    Ok(out)
}

/// One line of a `--script` file for the boot probe.
#[derive(Debug,Clone,PartialEq)]
pub enum ScriptStep{
    /// Run until the screen has changed since the last typed text and its
    /// last non-blank line ends with the text (a shell prompt).
    Prompt(String),
    /// Run until the text is anywhere on the screen.
    Screen(String),
    /// Type the text (US layout; `\r` is Enter, `\\` a backslash).
    Type(String),
    /// Put another diskette image of the same format into drive A:.
    Floppy(Vec<u8>),
    /// Run this many steps.
    Run(usize),
    /// Print the screen into the report.
    Show,
    /// Print the steps and host seconds since the last mark.
    Mark(String),
    /// Write the graphics screen to a binary PPM (P6) file.
    Shot(String),
}

/// Read a probe script: one step per line, `#` starts a comment line.
/// `prompt TEXT`, `screen TEXT`, `type TEXT`, `floppy IMAGE`, `run STEPS`,
/// `show`, `mark LABEL`, `shot FILE` (the graphics screen as a PPM
/// picture). Image paths are relative to the current folder.
pub fn parse_script(text:&str)->Result<Vec<ScriptStep>,String>{
    let unescape=|t:&str|->String{
        let mut out=String::new();let mut chars=t.chars();
        while let Some(c)=chars.next(){
            if c!='\\'{out.push(c);continue;}
            match chars.next(){Some('r'|'n')=>out.push('\r'),Some('s')=>out.push(' '),Some(o)=>out.push(o),None=>out.push('\\')}
        }
        out
    };
    let mut steps=Vec::new();
    for (n,line) in text.lines().enumerate(){
        let line=line.trim_end_matches('\r');
        if line.trim().is_empty()||line.trim_start().starts_with('#'){continue;}
        let (word,rest)=line.trim_start().split_once(' ').unwrap_or((line.trim(),""));
        let at=|e:String|format!("script line {}: {e}",n+1);
        steps.push(match word{
            "prompt"=>ScriptStep::Prompt(unescape(rest)),
            "screen"=>ScriptStep::Screen(unescape(rest)),
            "type"=>ScriptStep::Type(unescape(rest)),
            "floppy"=>ScriptStep::Floppy(std::fs::read(rest.trim()).map_err(|e|at(format!("{}: {e}",rest.trim())))?),
            "run"=>ScriptStep::Run(rest.trim().replace('_',"").parse().map_err(|_|at("run takes a step count".into()))?),
            "show"=>ScriptStep::Show,
            "mark"=>ScriptStep::Mark(rest.trim().to_string()),
            "shot" if !rest.trim().is_empty()=>ScriptStep::Shot(rest.trim().to_string()),
            "shot"=>return Err(at("shot takes a file name".into())),
            _=>return Err(at(format!("unknown step `{word}`"))),
        });
    }
    Ok(steps)
}

/// Run a booted machine through a probe script within `steps` instructions
/// in all; the report holds what `show` and `mark` print, then the final
/// screen and why the run stopped.
/// `progress`, when given, receives the report so far after each step.
pub fn script_probe(mut p:Loaded,steps:usize,script:&[ScriptStep],rtc_year:Option<u32>,keymap:super::guest_keymap::Keymap,mut progress:Option<&mut dyn FnMut(&str)>)->Result<String,String>{
    p.clock_offset=Some(0);
    if let Some(year)=rtc_year{p.set_rtc_year(year)?;}
    let mut session=session::Session::new(p);
    session.keymap=keymap;
    let mut report=String::new();
    let (mut done,mut mark_steps,mut mark_time)=(0usize,0usize,std::time::Instant::now());
    let mut typed_screen:Vec<String>=Vec::new();
    let mut reason=String::from("script done");
    let screen=|s:&session::Session|->Result<Vec<String>,String>{Ok(screen_lines(s.machine().ok_or("session state lost")?))};
    'script: for step in script{
        match step{
            ScriptStep::Prompt(_)|ScriptStep::Screen(_)|ScriptStep::Run(_)=>{
                let target=if let ScriptStep::Run(n)=step{done.saturating_add(*n)}else{usize::MAX};
                let mut stalled=false;
                loop{
                    let lines=screen(&session)?;
                    let met=match step{
                        ScriptStep::Prompt(t)=>lines!=typed_screen&&lines.iter().rev().find(|l|!l.trim().is_empty()).is_some_and(|l|l.trim_end().ends_with(t.trim_end())),
                        ScriptStep::Screen(t)=>lines.iter().any(|l|l.contains(t.as_str())),
                        _=>done>=target,
                    };
                    if met{break;}
                    if done>=steps{reason=format!("budget, waiting for {step:?}");break 'script;}
                    let slice=(steps-done).min(target.saturating_sub(done)).min(if matches!(step,ScriptStep::Run(_)){1_000_000}else{500_000});
                    session.run(slice)?;done+=slice;
                    let (status,detail)=session.status();
                    // A BIOS key wait goes on once the screen has been
                    // checked again; a second one in a row cannot change it.
                    if status=="waiting_input"&&!stalled{stalled=true;continue;}
                    stalled=false;
                    if status!="running"{reason=format!("{status} {detail}");break 'script;}
                }
            },
            ScriptStep::Type(text)=>{
                typed_screen=screen(&session)?;
                done+=type_text(&mut session,text)?;
            },
            ScriptStep::Floppy(image)=>{
                let disk=bios_disk::Disk::floppy(image.clone())?;
                session.machine_mut().ok_or("session state lost")?.disks.change_floppy(0,disk)?;
            },
            ScriptStep::Show=>{
                for line in screen(&session)?{report.push_str(&line);report.push('\n');}
                writeln!(report,"-- at {done} steps").unwrap();
            },
            ScriptStep::Mark(label)=>{
                let seconds=mark_time.elapsed().as_secs_f64();
                writeln!(report,"-- mark {label}: {} steps, {seconds:.3} s host time since the last mark ({done} steps in all)",done-mark_steps).unwrap();
                mark_steps=done;mark_time=std::time::Instant::now();
            },
            ScriptStep::Shot(file)=>{
                let p=session.machine().ok_or("session state lost")?;
                let display=p.vga.display;
                let Some((width,height,pixels))=super::desktop_vga_frame(p) else{
                    reason=format!("shot {file}: no graphics screen (display mode {:02x}h)",display.mode);break 'script;
                };
                let mut ppm=format!("P6\n{width} {height}\n255\n").into_bytes();
                for px in pixels{ppm.extend_from_slice(&[(px>>16)as u8,(px>>8)as u8,px as u8]);}
                std::fs::write(file,&ppm).map_err(|e|format!("shot {file}: {e}"))?;
                writeln!(report,"-- shot {file}: {width}x{height}, mode {:02x}h, at {done} steps",display.mode).unwrap();
            },
        }
        // A long script shows its progress as it goes.
        if let Some(out)=progress.as_mut(){out(&report);report.clear();}
    }
    let p=session.machine().ok_or("session state lost")?;
    for line in screen_lines(p){report.push_str(&line);report.push('\n');}
    let cs=p.selectors[SegmentRegister::Cs.index()];
    writeln!(report,"-- stop: {}; CS:EIP={cs:04x}:{:08x}; {} mode; {done} steps",reason.trim(),p.cpu.eip,if p.image.real_mode{"real"}else{"protected"}).unwrap();
    Ok(report)
}

/// Type text as key presses on the guest's layout (`session.keymap`, US
/// unless set): Shift and AltGr around the keys that need them; carriage
/// return or line feed is Enter. While the keyboard queue is full the
/// machine runs on in short slices; the steps run are returned.
fn type_text(session:&mut session::Session,text:&str)->Result<usize,String>{
    let mut ran=0usize;
    for (scan,extended,down,word) in super::guest_keymap::edges(session.keymap,text){
        let mut taken=false;
        for _ in 0..1000{
            if session.native_key(scan,extended,down,word)?{taken=true;break;}
            session.run(10_000)?;ran+=10_000;
        }
        if !taken{return Err("the keyboard queue stays full".into());}
    }
    Ok(ran)
}

#[cfg(test)]
#[path="../tests/unit/dos_boot.rs"]
mod tests;
