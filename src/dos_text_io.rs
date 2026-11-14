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
pub(crate) fn run(session:&mut Session,options:&host_console::Options,plain:bool)->Result<i32,String>{
    let mode=std::env::var("DINE_TEXT").unwrap_or_else(|_|"auto".into());
    if !matches!(mode.as_str(),"auto"|"terminal"|"headless"){return Err("DINE_TEXT must be auto, terminal or headless".into());}
    let tty=io::stdin().is_terminal()&&io::stdout().is_terminal();
    if !plain&&mode=="terminal"&&!tty{return Err("DINE_TEXT=terminal requires terminal stdin and stdout".into());}
    if plain||mode=="headless"||!options.window&&!tty{
        host_console::configure(session,options)?;
        stream(session,&mut io::stdin().lock(),&mut io::stdout().lock())
    }else{
        host_console::run(session,options)?;
        match session.status().0{"exit"=>Ok(exit_code(session).unwrap_or(0)),"running"|"waiting_input"=>Ok(0),_=>Err(format!("DOS stopped: {}",session.status().1))}
    }
}
pub(crate) fn cli(args:&[String],text_alias:bool,log_path:&str)->Result<i32,String>{
    let(mut config,mut mount,mut tail,mut fpu,mut program)=(None,None,None,None,None);
    let mut options=host_console::Options{window:cfg!(windows)&&!text_alias,mouse:false,ansi:false,country:None,utc_offset:None};
    let mut plain=false;
    let mut at=0;
    while at<args.len(){
        match args[at].as_str(){
            "--tty"|"--rust-graphics"=>options.window=false,"--window"=>options.window=true,"--plain"=>{plain=true;options.window=false;},
            "--ansi"=>options.ansi=true,"--mouse"=>options.mouse=true,
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
    let startup=if program.is_none(){Some(crate::dos_config::startup(config)?)}else{None};
    let path=program.as_deref().or_else(||startup.as_ref().map(|s|s.0.as_str())).ok_or("DOS program required")?;
    let tail=tail.or_else(||startup.as_ref().and_then(|s|s.1.as_deref()));
    let mut session=dos::session::open(path,mount,tail,config,fpu.or(defaults.fpu.map(|on|if on{"80387"}else{"none"})))?;
    let result=run(&mut session,&options,plain);
    session.log_stop(&mut dos::stoplog::Logger::new(log_path,path));
    result
}

#[cfg(test)]
#[path="../tests/unit/dos_text_io.rs"]
mod tests;
