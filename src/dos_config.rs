//! Shared startup configuration for interpreted and compiled DOS frontends.
use crate::dos::{self,Loaded};
use std::path::{Path,PathBuf};

#[derive(Debug,Default)]
struct Config{drives:Vec<(u8,PathBuf)>,country:Option<u16>,program:Option<String>,args:Option<String>,
    fpu:Option<bool>,mouse:Option<bool>,ansi:Option<bool>,utc_offset:Option<i64>,host_pointer:Option<HostPointer>,
    /// `own_programs`: Some(None) for none, Some(Some(letter)) for a drive.
    own_programs:Option<Option<u8>>,
    /// The folder of the configuration file.
    base:PathBuf}

/// Whether the console window shows the host's own mouse pointer over the
/// picture (`host_pointer` in the configuration, `--host-pointer`).
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub(crate) enum HostPointer{
    /// Hidden while the program's INT 33h driver shows its cursor, so only
    /// one arrow is seen; shown otherwise.
    #[default]
    Auto,
    /// Always shown, for programs that draw their own pointer without the
    /// driver's cursor, or when the host arrow is easier to follow.
    Show,
    /// Always hidden over the window's picture.
    Hide,
}
impl HostPointer{
    pub(crate) fn parse(text:&str)->Option<Self>{
        match text.trim().to_ascii_lowercase().as_str(){"auto"=>Some(Self::Auto),"show"=>Some(Self::Show),"hide"=>Some(Self::Hide),_=>None}
    }
    /// Whether to hide the host pointer, given whether the driver's cursor
    /// is on screen.
    #[cfg(any(windows,test))]
    pub(crate) fn hides(self,guest_cursor_visible:bool)->bool{
        match self{Self::Auto=>guest_cursor_visible,Self::Show=>false,Self::Hide=>true}
    }
}

/// The settings `pil --dos-console` takes from a configuration besides the
/// drives and country (which `apply` loads into the machine).
#[derive(Debug,Default,PartialEq,Eq)]
pub(crate) struct ConsoleSettings{pub(crate) fpu:Option<bool>,pub(crate) mouse:bool,pub(crate) ansi:bool,pub(crate) utc_offset:Option<i64>,pub(crate) host_pointer:Option<HostPointer>}

/// A UTC offset as the frontends write it: hours ("2", "-5", "5.5") or
/// "+HH:MM", in seconds; -12:00..+14:00.
pub(crate) fn parse_offset(text:&str)->Result<i64,String>{
    let text=text.trim();
    let bad=||format!("UTC offset {text} needs hours (2, -5, 5.5) or +HH:MM within -12:00..+14:00");
    let sign=if text.starts_with('-'){-1}else{1};
    let body=text.trim_start_matches(['+','-']);
    let seconds=if let Some((h,m))=body.split_once(':'){
        let (h,m)=(h.parse::<i64>().map_err(|_|bad())?,m.parse::<i64>().map_err(|_|bad())?);
        if !(0..60).contains(&m){return Err(bad());}
        h*3600+m*60
    }else{
        if body.is_empty()||!body.chars().all(|c|c.is_ascii_digit()||c=='.'){return Err(bad());}
        let hours=body.parse::<f64>().map_err(|_|bad())?;
        if !hours.is_finite()||hours>24.0{return Err(bad());}
        (hours*3600.0).round() as i64
    };
    let seconds=sign*seconds;
    if !(-12*3600..=14*3600).contains(&seconds){return Err(bad());}
    Ok(seconds)
}
fn boolean(key:&str,value:&str,line:usize)->Result<bool,String>{
    match value.to_ascii_lowercase().as_str(){"true"|"1"=>Ok(true),"false"|"0"=>Ok(false),_=>Err(format!("invalid boolean for {key} on config line {line}"))}
}

pub(crate) fn host_path(text:&str)->PathBuf{
    #[cfg(windows)]{
        let b=text.as_bytes();
        if b.len()>=7 && &b[..5]==b"/mnt/" && b[5].is_ascii_alphabetic() && b[6]==b'/'{
            return PathBuf::from(format!("{}:\\{}",char::from(b[5].to_ascii_uppercase()),text[7..].replace('/' ,"\\")));
        }
    }
    PathBuf::from(text)
}
fn parse(text:&str,parent:&Path)->Result<Config,String>{
    let mut config=Config{base:parent.to_path_buf(),..Config::default()};
    for (i,line) in text.trim_start_matches('\u{feff}').lines().enumerate(){
        let line=line.trim();
        let line=if let Some(setting)=line.strip_prefix("#@"){setting.trim()}else{
            if line.is_empty()||line.starts_with('#')||line.starts_with(';'){continue;}line
        };
        let (key,value)=line.split_once('=').ok_or_else(||format!("config line {} needs KEY=VALUE",i+1))?;
        let key=key.trim();let value=value.trim();
        if key.eq_ignore_ascii_case("program"){
            if value.is_empty()||config.program.is_some(){return Err(format!("invalid or duplicate program on config line {}",i+1));}
            let (outer,inner)=crate::containers::split_container_path(value).map_or((value,None),|(a,b)|(a,Some(b)));
            let path=host_path(outer);let path=if path.is_absolute(){path}else{parent.join(path)};
            let path=path.to_str().ok_or("DOS program path is not Unicode")?;
            config.program=Some(match inner{Some(part)=>format!("{path}::{part}"),None=>path.to_string()});
            continue;
        }
        if key.eq_ignore_ascii_case("args"){
            if config.args.replace(value.to_string()).is_some(){return Err(format!("duplicate args on config line {}",i+1));}
            continue;
        }
        if key.eq_ignore_ascii_case("country"){
            let country=value.parse::<u16>().map_err(|_|format!("invalid country on config line {}",i+1))?;
            if !matches!(country,1|358)||config.country.replace(country).is_some(){return Err(format!("invalid or duplicate country on config line {}",i+1));}
            continue;
        }
        let lower=key.to_ascii_lowercase().replace('-',"_");
        let once=|seen:bool|if seen{Err(format!("duplicate {key} on config line {}",i+1))}else{Ok(())};
        match lower.as_str(){
            "mouse"=>{once(config.mouse.is_some())?;config.mouse=Some(boolean(key,value,i+1)?);continue;},
            "ansi"=>{once(config.ansi.is_some())?;config.ansi=Some(boolean(key,value,i+1)?);continue;},
            "headless"|"rust_graphics"|"shell"=>{boolean(key,value,i+1)?;continue;},
            "fpu"=>{
                once(config.fpu.is_some())?;
                config.fpu=Some(match value.to_ascii_lowercase().as_str(){"none"=>false,"80387"=>true,_=>return Err(format!("invalid fpu on config line {}",i+1))});
                continue;
            },
            "utc_offset"=>{
                once(config.utc_offset.is_some())?;
                config.utc_offset=Some(parse_offset(value).map_err(|e|format!("{e} on config line {}",i+1))?);
                continue;
            },
            "host_pointer"=>{
                once(config.host_pointer.is_some())?;
                config.host_pointer=Some(HostPointer::parse(value).ok_or_else(||format!("host_pointer must be auto, show or hide on config line {}",i+1))?);
                continue;
            },
            "own_programs"=>{
                once(config.own_programs.is_some())?;
                let v=value.trim_end_matches(':').to_ascii_uppercase();
                config.own_programs=Some(match v.as_bytes(){
                    b"NONE"=>None,
                    [l] if (b'A'..=b'E').contains(l)=>Some(*l),
                    _=>return Err(format!("own_programs must be a drive letter A..E or none on config line {}",i+1)),
                });
                continue;
            },
            "speaker_volume"=>{
                if !value.parse::<u8>().is_ok_and(|v|v<=100){return Err(format!("speaker volume must be 0..100 on config line {}",i+1));}
                continue;
            },
            "engine"|"com1"=>{
                if value.is_empty(){return Err(format!("empty {key} on config line {}",i+1));}
                continue;
            },
            _=>{},
        }
        let drive=key.trim_end_matches(':').as_bytes();
        if drive.len()!=1||!(b'A'..=b'E').contains(&drive[0].to_ascii_uppercase())||value.is_empty(){return Err(format!("invalid drive on config line {}",i+1));}
        let letter=drive[0].to_ascii_uppercase();
        if config.drives.iter().any(|(d,_)|*d==letter){return Err(format!("duplicate drive on config line {}",i+1));}
        let path=host_path(value);let path=if path.is_absolute(){path}else{parent.join(path)};
        config.drives.push((letter,path));
    }
    Ok(config)
}
/// The configuration file to use: the explicit one, PILDOS_CONFIG, or
/// ./pildos.conf, and whether it was asked for (a default may be absent).
pub(crate) fn selected_path(explicit:Option<&str>)->(PathBuf,bool){
    let selected=if let Some(path)=explicit{Some(host_path(path))}else{
        std::env::var_os("PILDOS_CONFIG").filter(|p|!p.is_empty()).map(|p|p.to_str().map(host_path).unwrap_or_else(||PathBuf::from(&p)))
    };
    let required=selected.is_some();
    (selected.unwrap_or_else(||PathBuf::from("pildos.conf")),required)
}
/// Check configuration text as `load` would, with paths based on `parent`.
pub(crate) fn check(text:&str,parent:&Path)->Result<(),String>{parse(text,parent).map(|_|())}
/// Apply an explicit configuration, PILDOS_CONFIG, or optional ./pildos.conf.
/// `skip` protects a legacy explicit --mount from a configuration default.
fn load(explicit:Option<&str>)->Result<Option<Config>,String>{
    let (path,required)=selected_path(explicit);
    if !required&&!path.exists(){return Ok(None);}
    let meta=std::fs::metadata(&path).map_err(|e|format!("cannot read DOS config {}: {e}",path.display()))?;
    if meta.len()>32768{return Err("DOS config is larger than 32 KiB".into());}
    let text=std::fs::read_to_string(&path).map_err(|e|format!("cannot read DOS config {}: {e}",path.display()))?;
    Ok(Some(parse(&text,path.parent().unwrap_or_else(||Path::new(".")))?))
}
pub(crate) fn startup(explicit:Option<&str>)->Result<(String,Option<String>),String>{
    startup_optional(explicit)?.ok_or_else(||"DOS configuration needs #@program=...".to_string())
}
pub(crate) fn startup_optional(explicit:Option<&str>)->Result<Option<(String,Option<String>)>,String>{
    let Some(config)=load(explicit)? else{return Ok(None)};
    Ok(config.program.map(|p|(p,config.args)))
}
/// FPU, mouse, ANSI driver, clock offset and host pointer for `pil --dos-console`.
pub(crate) fn console_settings(explicit:Option<&str>)->Result<ConsoleSettings,String>{
    let Some(config)=load(explicit)? else{return Ok(ConsoleSettings::default())};
    Ok(ConsoleSettings{fpu:config.fpu,mouse:config.mouse==Some(true),ansi:config.ansi==Some(true),utc_offset:config.utc_offset,host_pointer:config.host_pointer})
}
/// The drive `own_programs` names (default D:) and the report of copying
/// programs/dos next to the configuration into its host folder.
fn refresh(config:&Config,skip:Option<u8>)->Option<(u8,crate::dos_own_programs::Report)>{
    let letter=config.own_programs.unwrap_or(Some(b'D'))?;
    if Some(letter)==skip{return None;}
    let (_,folder)=config.drives.iter().find(|(d,_)|*d==letter)?;
    if !folder.is_dir(){return None;}
    Some((letter,crate::dos_own_programs::refresh(&config.base.join("programs").join("dos"),folder)))
}
/// Bring the project's own programs on the `own_programs` drive up to date
/// now; a line saying what changed, if anything did.
pub(crate) fn refresh_own_programs(explicit:Option<&str>)->Result<Option<String>,String>{
    let Some(config)=load(explicit)? else{return Ok(None)};
    Ok(refresh(&config,None).and_then(|(letter,report)|report.summary(letter)))
}
pub(crate) fn apply(p:&mut Loaded,explicit:Option<&str>,skip:Option<u8>)->Result<(),String>{
    let Some(config)=load(explicit)? else{return Ok(())};
    refresh(&config,skip);
    let mut mounts=Vec::new();
    for (letter,path) in config.drives{
        if Some(letter)==skip{continue;}
        let path=path.to_str().ok_or("DOS config path is not Unicode")?;
        mounts.push((letter,dos::session::open_mount_source(path)?));
    }
    let choose_c=p.drive.letter()==b'A'&&mounts.iter().any(|(d,_)|*d==b'C');
    for (letter,source) in mounts{p.drive.mount(letter,source)?;}
    if choose_c{dos::select_startup_drive(p,b'C')?;}
    if let Some(country)=config.country{p.country=country;}
    Ok(())
}

/// Line-preserving editing of a configuration file, for `pil --pildos`.
#[path="dos_config_edit.rs"]
pub(crate) mod edit;

#[cfg(test)]
#[path="../tests/unit/dos_config.rs"]
mod tests;
