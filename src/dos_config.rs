//! Shared startup configuration for interpreted and compiled DOS frontends.
use crate::dos::{self,Loaded};
use std::path::{Path,PathBuf};

#[derive(Debug,Default)]
struct Config{drives:Vec<(u8,PathBuf)>,country:Option<u16>,program:Option<String>,args:Option<String>,text:TextOptions,utc_offset:Option<String>}
#[derive(Debug,Default)]
pub(crate) struct TextOptions{pub ansi:bool,pub mouse:bool,pub fpu:Option<String>,pub utc_offset:Option<i64>}

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
    let mut config=Config::default();
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
        if matches!(key.to_ascii_lowercase().as_str(),"headless"|"rust_graphics"|"rust-graphics"|"mouse"|"shell"|"ansi"){
            if !matches!(value.to_ascii_lowercase().as_str(),"true"|"false"|"1"|"0"){return Err(format!("invalid boolean for {key} on config line {}",i+1));}
            let enabled=matches!(value.to_ascii_lowercase().as_str(),"true"|"1");
            match key.to_ascii_lowercase().as_str(){"ansi"=>config.text.ansi=enabled,"mouse"=>config.text.mouse=enabled,_=>{}}
            continue;
        }
        if key.eq_ignore_ascii_case("fpu"){
            if !matches!(value.to_ascii_lowercase().as_str(),"none"|"80387"){return Err(format!("invalid fpu on config line {}",i+1));}
            config.text.fpu=Some(value.to_ascii_lowercase());
            continue;
        }
        if matches!(key.to_ascii_lowercase().as_str(),"utc_offset"|"utc-offset"){
            if value.is_empty(){return Err("empty utc_offset".into());}
            config.utc_offset=Some(value.to_string());continue;
        }
        if matches!(key.to_ascii_lowercase().as_str(),"engine"|"com1"|"speaker_volume"|"speaker-volume"|"utc_offset"|"utc-offset"){
            if value.is_empty(){return Err(format!("empty {key} on config line {}",i+1));}
            continue;
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
/// Apply an explicit configuration, PILDOS_CONFIG, or optional ./pildos.conf.
/// `skip` protects a legacy explicit --mount from a configuration default.
fn load(explicit:Option<&str>)->Result<Option<Config>,String>{
    let selected=if let Some(path)=explicit{Some(host_path(path))}else{
        std::env::var_os("PILDOS_CONFIG").map(|p|p.to_str().map(host_path).unwrap_or_else(||PathBuf::from(&p)))
    };
    let required=selected.is_some();let path=selected.unwrap_or_else(||PathBuf::from("pildos.conf"));
    if !required&&!path.exists(){return Ok(None);}
    let meta=std::fs::metadata(&path).map_err(|e|format!("cannot read DOS config {}: {e}",path.display()))?;
    if meta.len()>32768{return Err("DOS config is larger than 32 KiB".into());}
    let text=std::fs::read_to_string(&path).map_err(|e|format!("cannot read DOS config {}: {e}",path.display()))?;
    Ok(Some(parse(&text,path.parent().unwrap_or_else(||Path::new(".")))?))
}
pub(crate) fn startup(explicit:Option<&str>)->Result<(String,Option<String>),String>{
    startup_optional(explicit)?.ok_or_else(||"DOS configuration needs #@program=...".to_string())
}
pub(crate) fn text_options(explicit:Option<&str>)->Result<TextOptions,String>{
    let Some(mut config)=load(explicit)? else{return Ok(TextOptions::default())};
    if let Some(value)=config.utc_offset{
        let offset=value.parse::<i64>().map_err(|_|"Rust utc_offset requires integer seconds")?;
        if !(-50400..=50400).contains(&offset){return Err("utc_offset must be -50400..50400".into());}
        config.text.utc_offset=Some(offset);
    }
    Ok(config.text)
}
pub(crate) fn startup_optional(explicit:Option<&str>)->Result<Option<(String,Option<String>)>,String>{
    let Some(config)=load(explicit)? else{return Ok(None)};
    Ok(config.program.map(|p|(p,config.args)))
}
pub(crate) fn apply(p:&mut Loaded,explicit:Option<&str>,skip:Option<u8>)->Result<(),String>{
    let Some(config)=load(explicit)? else{return Ok(())};
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

#[cfg(test)]
#[path="../tests/unit/dos_config.rs"]
mod tests;
