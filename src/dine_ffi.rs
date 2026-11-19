//! C ABI of the Dine shared library (dine.dll / libdine.so).
//!
//! The library is program-independent: the host passes the bytes of any DOS
//! program at run time, and the same PIL interpreter, DOS/BIOS services, VGA
//! device and memory model run them in-process. Nothing here, and no build
//! step, embeds a guest program. tools/verify_dine_library.py calls it with
//! ctypes.
use std::ffi::{c_char, CStr};

/// Convert a host command line to the legacy 8.3 DOS view.
/// File operands are resolved against the current host working directory,
/// mounted as C:. Quoted long host filenames are supported, but the legacy
/// guest receives its regular DOS command tail, not a new filesystem API.
/// DOS aliases come from the same listing implementation as INT 21h/AH3Dh.
fn dos_tail(loaded:&crate::dos::Loaded,command_line:&str)->Result<String,String>{
    let mut chars=command_line.chars().peekable();
    let mut arguments=Vec::new();
    while chars.peek().is_some(){
        while chars.peek().is_some_and(|c|c.is_whitespace()){chars.next();}
        if chars.peek().is_none(){break;}
        let mut value=String::new();
        let(mut quoted,mut had_quote)=(false,false);
        while let Some(&next)=chars.peek(){
            if !quoted&&next.is_whitespace(){break;}
            chars.next();
            if next=='"' {quoted=!quoted;had_quote=true;}
            else{value.push(next);}
        }
        if quoted{return Err("unterminated quotation mark in Dine command line".into());}
        if let Some(alias)=loaded.drive.host_argument_alias(&value)?{
            arguments.push(alias);
        }else if had_quote {
            // Retain quotes for ordinary guest arguments that aren't host
            // files. A DOS filename alias itself has no spaces or quotes.
            arguments.push(format!("\"{value}\""));
        }else{
            arguments.push(value);
        }
    }
    Ok(arguments.join(" "))
}

fn prepare(guest:&[u8],name:&str,tail:&str,config:Option<&str>)->Result<(crate::dos::session::Session,crate::dos::host_console::Options),String>{
    // Current-directory mapping is intentional: Dine runs with the host
    // caller's folder as DOS C: so the aliases refer to real host files.
    // This is a compatibility mount, not unrestricted native filesystem I/O.
    let cwd=std::env::current_dir().map_err(|e|format!("Dine working directory: {e}"))?;
    let defaults=crate::dos_config::console_settings(config)?;
    let loaded = crate::dos::load_with_drives(
        guest, name, vec![crate::dos::Source::Host(cwd),crate::dos::Source::Memory(crate::dos::MemDir::default())], None)?;
    let mut session=crate::dos::session::configured(loaded,None,config,None,defaults.fpu.map(|on|if on{"80387"}else{"none"}))?;
    let translated=dos_tail(session.machine().ok_or("session state lost")?,tail)?;
    crate::dos::set_command_tail(session.machine_mut().ok_or("session state lost")?,&translated)?;
    let options=crate::dos::host_console::Options{window:false,mouse:defaults.mouse,ansi:defaults.ansi,country:None,utc_offset:defaults.utc_offset,host_pointer:defaults.host_pointer.unwrap_or_default(),speaker_volume:defaults.speaker_volume.unwrap_or(crate::dos::host_console::speaker::DEFAULT_VOLUME)};
    Ok((session,options))
}
fn interpreted(guest: &[u8], name: &str, tail: &str,timed:bool) -> Result<i32, String> {
    let (mut session,options)=prepare(guest,name,tail,None)?;
    if timed{
        crate::dos::host_console::configure(&mut session,&options)?;
        let budget=std::env::var("DINE_BENCHMARK_STEPS").unwrap_or_else(|_|"1000000000".into())
            .parse::<usize>().ok().filter(|n|(1..=1_000_000_000).contains(n)).ok_or("DINE_BENCHMARK_STEPS must be 1..1000000000")?;
        let (elapsed,steps,boundary)=crate::dos_text_io::timed(&mut session,budget)?;
        println!("DINE_TIMING elapsed_ms={:.3} instructions={steps} boundary={boundary} presentation=off",elapsed.as_secs_f64()*1000.0);
        return Ok(i32::from(session.exit_code().unwrap_or(0)));
    }
    crate::dos_text_io::run(&mut session,&options,false)
}

/// Optional shared settings editor. Existing ABI-1 loaders remain compatible.
#[no_mangle]
pub extern "C" fn dine_edit_settings()->i32{
    match std::panic::catch_unwind(||crate::dos::setup::edit(None,cfg!(windows))){
        Ok(Ok(line))=>{println!("{line}");0},
        Ok(Err(e))=>{eprintln!("Dine settings: {e}");1},
        Err(_)=>{eprintln!("Dine settings: runtime panicked");1},
    }
}

/// Stable ABI version for hosts. A change to the C contract increases it.
#[no_mangle]
pub extern "C" fn dine_abi_version() -> u32 { 1 }

/// Run DOS program bytes supplied by the host through the Dine interpreter.
///
/// `name` and `tail` must point to valid NUL-terminated UTF-8 strings.
/// `guest` must reference `guest_len` readable bytes (non-null).
/// Returns 0..255 for guest exit status, or 1 for a host/guest error.
/// Invalid foreign pointers (other than null) are outside this C ABI contract.
#[no_mangle]
pub unsafe extern "C" fn dine_run(
    guest: *const u8, guest_len: usize,
    name: *const c_char, tail: *const c_char,
) -> i32 {
    run_inner(guest,guest_len,name,tail,false)
}
/// Execution timing through the same shared runtime, without host presentation.
#[no_mangle]
pub unsafe extern "C" fn dine_run_timed(guest:*const u8,guest_len:usize,name:*const c_char,tail:*const c_char)->i32{
    run_inner(guest,guest_len,name,tail,true)
}
unsafe fn run_inner(guest:*const u8,guest_len:usize,name:*const c_char,tail:*const c_char,timed:bool)->i32{
    let outcome = std::panic::catch_unwind(|| -> Result<i32, String> {
        if guest.is_null() || name.is_null() || tail.is_null() {
            return Err("null argument to dine_run".into());
        }
        if guest_len == 0 || guest_len > crate::dos::MAX_FILE {
            return Err("DOS program must contain 1..1048576 bytes".into());
        }
        let guest = unsafe { std::slice::from_raw_parts(guest, guest_len) };
        let name = unsafe { CStr::from_ptr(name) }
            .to_str().map_err(|e| format!("DOS name is not UTF-8: {e}"))?;
        let tail = unsafe { CStr::from_ptr(tail) }
            .to_str().map_err(|e| format!("DOS command tail is not UTF-8: {e}"))?;
        interpreted(guest, name, tail,timed)
    });
    match outcome {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => {
            eprintln!("Dine: {error}");
            1
        }
        Err(_) => {
            eprintln!("Dine: runtime panicked");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_guests_use_shared_configuration_and_ansi_screen(){
        let root=std::env::temp_dir().join(format!("dine-config-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(root.join("files")).unwrap();
        std::fs::write(root.join("files/payload.dat"),b"data").unwrap();
        let config=root.join("pildos.conf");
        std::fs::write(&config,"ansi=true\nmouse=true\nfpu=80387\ncountry=358\nutc_offset=+02:00\nC=files\nown_programs=none\n").unwrap();
        let mut bytes=Vec::new();
        for &b in b"\x1b[92mX"{bytes.extend_from_slice(&[0xb2,b,0xb4,2,0xcd,0x21]);}
        bytes.extend_from_slice(&[0xb8,0,0x4c,0xcd,0x21]);
        let (mut session,options)=super::prepare(&bytes,"FIXTURE.COM","",config.to_str()).unwrap();
        assert!(options.ansi&&options.mouse);assert_eq!(options.utc_offset,Some(7200));
        let p=session.machine().unwrap();assert_eq!(p.country,358);
        assert_ne!(p.ram.load(0x410,crate::MemWidth::Word).unwrap()&2,0);
        assert!(p.drive.host_argument_alias("payload.dat").unwrap().is_some());
        crate::dos::host_console::configure(&mut session,&options).unwrap();
        crate::dos_text_io::stream(&mut session,&mut std::io::Cursor::new(""),&mut Vec::new()).unwrap();
        assert_eq!(&session.screen().unwrap().0[..2],&[b'X',10]);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn null_guest_is_rejected() {
        let name = b"HELLO.COM\0";
        let tail = b"\0";
        let status = unsafe {
            super::dine_run(std::ptr::null(), 1, name.as_ptr().cast(), tail.as_ptr().cast())
        };
        assert_eq!(status, 1);
    }
}
