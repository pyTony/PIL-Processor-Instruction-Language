//! Startup and redirected I/O for the shared Rust DOS console.
//! Interactive drawing and BIOS key mapping belong to dos::host_console.
use crate::dos::{self,session::Session,host_console::{self,Key,Mods}};
use std::{collections::VecDeque,io::{self,BufRead,Read,Write,IsTerminal}};

pub(crate) fn transcript(bytes:&[u8])->String{
    let mut out=String::new();
    for &b in bytes{
        if matches!(b,b'\r'|b'\n'|b'\t')||b>=32&&b!=127{out.push_str(&crate::dos_encoding::decode(&[b]));}
        else{use std::fmt::Write as _;let _=write!(out,"\\x{b:02x}");}
    }
    out
}
pub(crate) fn line_keys(line:&str)->Result<VecDeque<u16>,String>{
    let mut keys=VecDeque::new();let mut previous_cr=false;
    for c in line.chars(){
        if c=='\n'&&previous_cr{previous_cr=false;continue;}
        let key=match c{'\r'|'\n'=>Key::Enter,'\t'=>Key::Tab,'\u{8}'|'\u{7f}'=>Key::Backspace,'\u{1b}'=>Key::Escape,_=>Key::Char(c)};
        keys.push_back(host_console::bios_word(key,Mods::default()).ok_or_else(||format!("input character U+{:04X} is not a DOS key",c as u32))?);
        previous_cr=c=='\r';
    }
    Ok(keys)
}

/// No detached stdin reader: a library caller can unload Dine after return.
pub(crate) fn stream<R:BufRead,W:Write>(session:&mut Session,input:&mut R,output:&mut W)->Result<i32,String>{
    let mut pending=VecDeque::new();
    loop{
        while let Some(&word)=pending.front(){
            if session.key(word as u8,(word>>8)as u8)?{pending.pop_front();}else{break;}
        }
        let bytes=session.run(10_000)?;
        output.write_all(transcript(&bytes).as_bytes()).and_then(|_|output.flush()).map_err(|e|e.to_string())?;
        match session.status().0{
            "running"=>{},
            "waiting_input"=>if pending.is_empty(){
                let mut line=Vec::new();
                let n=(&mut *input).take(4097).read_until(b'\n',&mut line).map_err(|e|e.to_string())?;
                if n==0{return Err("DOS guest is waiting for input, but host stdin reached EOF".into());}
                if n>4096{return Err("host input line exceeds 4096 UTF-8 bytes".into());}
                pending=line_keys(std::str::from_utf8(&line).map_err(|e|e.to_string())?)?;
            },
            "exit"=>return Ok(exit_code(session).unwrap_or(0)),
            _=>return Err(format!("DOS stopped: {}",session.status().1)),
        }
    }
}
fn exit_code(session:&Session)->Option<i32>{
    session.exit_code().map(i32::from)
}
/// Execution-only timing: no host rendering or human input time is included.
#[cfg_attr(not(any(feature="dine-library",test)),allow(dead_code))]
pub(crate) fn timed(session:&mut Session,budget:usize)->Result<(std::time::Duration,usize,&'static str),String>{
    let start=std::time::Instant::now();let initial=session.steps();let mut remaining=budget;
    while remaining>0{
        let slice=remaining.min(1_000_000);
        session.run(slice)?;
        let status=session.status().0;
        match status{
            "exit"|"waiting_input"=>return Ok((start.elapsed(),session.steps()-initial,status)),
            "running"=>remaining-=slice,
            _=>return Err(format!("DOS timing stopped: {}",session.status().1)),
        }
    }
    Err(format!("DOS timing exceeded its {budget}-step budget before exit or input wait"))
}
pub(crate) fn run(session:&mut Session,options:&host_console::Options,plain:bool)->Result<i32,String>{
    let mode=std::env::var("DINE_TEXT").unwrap_or_else(|_|"auto".into());
    if !matches!(mode.as_str(),"auto"|"terminal"|"headless"){return Err("DINE_TEXT must be auto, terminal or headless".into());}
    let tty=io::stdin().is_terminal()&&io::stdout().is_terminal();
    if !plain&&mode=="terminal"&&!tty{return Err("DINE_TEXT=terminal requires terminal stdin and stdout".into());}
    if plain||mode=="headless"||!options.window&&!tty{
        host_console::configure(session,options)?;
        stream(session,&mut io::stdin().lock(),&mut io::stdout().lock())
    }else{
        // The window or alternate screen has closed: say how the program ended.
        println!("{}",host_console::run(session,options)?);
        match session.status().0{"exit"=>Ok(exit_code(session).unwrap_or(0)),"running"|"waiting_input"=>Ok(0),_=>Err(format!("DOS stopped: {}",session.status().1))}
    }
}
pub(crate) fn cli(args:&[String],text_alias:bool,log_path:&str)->Result<i32,String>{
    let(mut config,mut mount,mut tail,mut fpu,mut program)=(None,None,None,None,None);
    let mut options=host_console::Options{window:cfg!(windows)&&!text_alias,mouse:false,ansi:false,country:None,utc_offset:None,host_pointer:Default::default(),speaker_volume:host_console::speaker::DEFAULT_VOLUME,rtc_year:None,guest_keymap:None};
    let mut plain=false;
    let mut boot:Option<(String,u8)>=None;
    let mut hard_disk:Option<String>=None;
    let mut folder:Option<String>=None;
    let mut floppies:Vec<std::path::PathBuf>=Vec::new();
    let mut host_pointer=None;
    let mut speaker_volume=None;
    let mut at=0;
    while at<args.len(){
        match args[at].as_str(){
            "--tty"|"--rust-graphics"=>options.window=false,"--window"=>options.window=true,"--plain"=>{plain=true;options.window=false;},
            "--ansi"=>options.ansi=true,"--mouse"=>options.mouse=true,
            "--host-pointer"=>{
                let value=args.get(at+1).ok_or("missing DOS console option value")?;
                host_pointer=Some(crate::dos_config::HostPointer::parse(value).ok_or("--host-pointer takes auto, show or hide")?);at+=1;
            },
            "--speaker-volume"=>{
                let value=args.get(at+1).ok_or("missing DOS console option value")?;
                speaker_volume=Some(value.parse::<u8>().ok().filter(|v|*v<=100).ok_or("--speaker-volume takes 0..100")?);at+=1;
            },
            "--floppy"=>{floppies.push(std::path::PathBuf::from(args.get(at+1).ok_or("--floppy takes a diskette image")?));at+=1;},
            "--hd-image"=>{hard_disk=Some(args.get(at+1).ok_or("--hd-image takes a fixed-disk image")?.clone());at+=1;},
            "--hd-folder"=>{folder=Some(args.get(at+1).ok_or("--hd-folder takes a host folder")?.clone());at+=1;},
            "--rtc-year"=>{
                let value=args.get(at+1).ok_or("--rtc-year takes a year")?;
                options.rtc_year=Some(value.parse::<u32>().ok().filter(|y|(1980..=2099).contains(y)).ok_or("--rtc-year takes a year 1980..2099")?);at+=1;
            },
            "--guest-keymap"=>{
                let value=args.get(at+1).ok_or("--guest-keymap takes us, uk or fi")?;
                options.guest_keymap=Some(crate::dos::guest_keymap::Keymap::parse(value)?);at+=1;
            },
            "--boot"|"--boot-hd"=>{
                let value=args.get(at+1).ok_or("--boot takes a disk image")?;
                boot=Some((value.clone(),if args[at]=="--boot-hd"{0x80}else{0}));at+=1;
            },
            "--config"|"--mount"|"--args"|"--fpu"|"--country"|"--utc-offset"=>{
                let value=args.get(at+1).ok_or("missing DOS console option value")?.as_str();
                match args[at].as_str(){"--config"=>config=Some(value),"--mount"=>mount=Some(value),"--args"=>tail=Some(value),"--fpu"=>fpu=Some(value),"--country"=>options.country=Some(value.parse().map_err(|_|"invalid country")?),_=>options.utc_offset=Some(value.parse().map_err(|_|"invalid UTC offset")?)}at+=1;
            },
            flag if flag.starts_with('-')=>return Err(format!("unknown DOS console option: {flag}")),
            path=>if program.replace(path.to_string()).is_some(){return Err("only one DOS program is allowed".into());},
        }at+=1;
    }
    let defaults=crate::dos_config::console_settings(config)?;
    options.ansi|=defaults.ansi;options.mouse|=defaults.mouse;
    options.utc_offset=options.utc_offset.or(defaults.utc_offset);
    options.host_pointer=host_pointer.or(defaults.host_pointer).unwrap_or_default();
    options.speaker_volume=speaker_volume.or(defaults.speaker_volume).unwrap_or(host_console::speaker::DEFAULT_VOLUME);
    if let Some((image,drive))=boot{
        // A booted disk: the machine without Dine's DOS (no program, drives
        // or configuration file of its own).
        // A fixed disk is written back to its file; a diskette is not.
        // A folder disk's changes go back to the folder when the run ends.
        let folder=folder.map(|dir|dos::FolderDisk::create(std::path::Path::new(&dir))).transpose()?;
        if let Some((_,notes))=&folder{for note in notes{eprintln!("--hd-folder: {note}");}}
        let result=(||{
            let mut machine=dos::boot_command(&image,drive,hard_disk.as_deref(),folder.as_ref().map(|f|f.0.image.as_path()))?;
            if fpu.or(defaults.fpu.map(|on|if on{"80387"}else{"none"}))==Some("80387"){machine.select_fpu(true)?;}
            let mut session=dos::session::Session::new(machine);
            if !floppies.is_empty(){
                if drive!=0{return Err("--floppy changes the diskette a --boot started from".into());}
                floppies.insert(0,std::path::PathBuf::from(&image));
                session.boot_diskettes(floppies)?;
            }
            let result=run(&mut session,&options,plain);
            session.log_stop(&mut dos::stoplog::Logger::new(log_path,&image));
            result
        })();
        // The session (and its open image file) is gone; copy back.
        if let Some((folder,_))=folder{
            match folder.finish(){Ok(report)=>println!("--hd-folder {report}"),Err(e)=>{eprintln!("--hd-folder {e}");if result.is_ok(){return Err(e);}}}
        }
        return result;
    }
    if hard_disk.is_some(){return Err("--hd-image needs --boot (a booted machine)".into());}
    if folder.is_some(){return Err("--hd-folder needs --boot (a booted machine); for DOS use --mount".into());}
    if !floppies.is_empty(){return Err("--floppy needs --boot (a booted machine)".into());}
    let startup=if program.is_none(){Some(crate::dos_config::startup(config)?)}else{None};
    let path=program.as_deref().or_else(||startup.as_ref().map(|s|s.0.as_str())).ok_or("DOS program required")?;
    let tail=tail.or_else(||startup.as_ref().and_then(|s|s.1.as_deref()));
    let mut session=dos::session::open(path,mount,tail,config,fpu.or(defaults.fpu.map(|on|if on{"80387"}else{"none"})))?;
    let result=run(&mut session,&options,plain);
    session.log_stop(&mut dos::stoplog::Logger::new(log_path,path));
    result
}

#[cfg(test)]
#[path="../../tests/unit/dos_text_io.rs"]
mod tests;
