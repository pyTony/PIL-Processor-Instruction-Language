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

fn interpreted(guest: &[u8], name: &str, tail: &str) -> Result<i32, String> {
    // Current-directory mapping is intentional: Dine runs with the host
    // caller's folder as DOS C: so the aliases refer to real host files.
    // This is a compatibility mount, not unrestricted native filesystem I/O.
    let cwd=std::env::current_dir().map_err(|e|format!("Dine working directory: {e}"))?;
    let mut loaded = crate::dos::load_with_drives(
        guest, name, vec![crate::dos::Source::Host(cwd)], None)?;
    let translated=dos_tail(&loaded,tail)?;
    crate::dos::set_command_tail(&mut loaded,&translated)?;
    let mut session = crate::dos::session::Session::new(loaded);
    let options=crate::dos::host_console::Options{window:false,mouse:false,ansi:false,country:None,utc_offset:None};
    crate::dos_text_io::run(&mut session,&options,false)
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
        interpreted(guest, name, tail)
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
    fn null_guest_is_rejected() {
        let name = b"HELLO.COM\0";
        let tail = b"\0";
        let status = unsafe {
            super::dine_run(std::ptr::null(), 1, name.as_ptr().cast(), tail.as_ptr().cast())
        };
        assert_eq!(status, 1);
    }
}
