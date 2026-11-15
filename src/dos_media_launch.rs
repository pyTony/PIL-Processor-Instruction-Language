//! Native media selection before starting the same shared DOS runtime.
#[cfg(windows)]
pub(crate) fn run(shell:&str,media:Option<&str>,log_path:&str)->Result<(),String>{
    let media=match media{
        Some(path)=>crate::dos_config::host_path(path),
        None=>match super::media_dialog::file(false)?{Some(path)=>path,None=>return Ok(())},
    };
    let extension=media.extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase();
    let size=std::fs::metadata(&media).map_err(|e|e.to_string())?.len();
    if !matches!(extension.as_str(),"img"|"ima"|"lst"|"txt"){return Err("select an .img/.ima image or .lst disk-set manifest".into());}
    let mut session=super::catalog::open_program(shell,None,None)?;
    let text=if matches!(extension.as_str(),"lst"|"txt"){
        session.disk_set(b'A',&media)?
    }else if size<=2880*1024{
        session.floppy_media(b'A',&media)?
    }else{
        let path=if size>64*1024*1024{expand_hard_disk(&media)?}else{media.clone()};
        session.mount_media(b'E',path.to_str().ok_or("media path is not Unicode")?)?
    };
    println!("{text}\nD: is the writable install destination. Ctrl+Shift+M opens host drive controls.");
    let settings=crate::dos_config::console_settings(None)?;
    let options=super::Options{window:true,mouse:settings.mouse,ansi:settings.ansi,country:None,utc_offset:settings.utc_offset,host_pointer:settings.host_pointer.unwrap_or_default(),speaker_volume:settings.speaker_volume.unwrap_or(super::speaker::DEFAULT_VOLUME)};
    let result=super::run(&mut session,&options);
    session.log_stop(&mut crate::dos::stoplog::Logger::new(log_path,shell));
    println!("{}",result?);Ok(())
}

#[cfg(windows)]
fn expand_hard_disk(image:&std::path::Path)->Result<std::path::PathBuf,String>{
    use std::{path::PathBuf,os::windows::process::CommandExt};
    let program_files=std::env::var_os("ProgramFiles").ok_or("ProgramFiles is not set")?;
    let extractor=PathBuf::from(program_files).join("7-Zip/7z.exe");
    if !extractor.is_file(){return Err("images above 64 MiB require installed 7-Zip for host-side extraction".into());}
    let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
    let parent=PathBuf::from("drives/media");std::fs::create_dir_all(&parent).map_err(|e|e.to_string())?;
    let destination=parent.join(format!("import-{}-{nonce}",std::process::id()));
    std::fs::create_dir(&destination).map_err(|e|e.to_string())?;
    let output=std::process::Command::new(extractor).arg("x").arg(image).arg(format!("-o{}",destination.display())).arg("-aos")
        .creation_flags(0x08000000).output().map_err(|e|format!("image extraction: {e}; incomplete folder {}",destination.display()))?;
    if !output.status.success(){return Err(format!("image extraction failed; incomplete folder {}: {}",destination.display(),String::from_utf8_lossy(&output.stderr)));}
    println!("Hard-disk files are writable on E: and persist in {}",destination.display());Ok(destination)
}

#[cfg(not(windows))]
pub(crate) fn run(_shell:&str,_media:Option<&str>,_log_path:&str)->Result<(),String>{
    Err("the native media browser needs Windows; use diskset/disk in --dos-session on this platform".into())
}
