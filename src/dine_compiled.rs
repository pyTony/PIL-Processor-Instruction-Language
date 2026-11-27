//! Version 1 entry point for native real-mode COM code.
use crate::{dos, host_io::CaptureOutput, MemWidth};
use std::ffi::{c_char, c_void, CStr};
use std::io::Write;

#[repr(C)]
pub struct Registers {
    pub gpr: [u32; 8],
    pub eflags: u32,
    pub ip: u16,
    pub segments: [u16; 6],
}

#[repr(C)]
pub struct Services {
    pub interrupt: unsafe extern "C" fn(*mut c_void, *mut Registers, u8) -> i32,
    pub read: unsafe extern "C" fn(*mut c_void, u32, u8, *mut u32) -> i32,
    pub write: unsafe extern "C" fn(*mut c_void, u32, u8, u32) -> i32,
    pub port: unsafe extern "C" fn(*mut c_void, u16, u8, u8, *mut u32) -> i32,
}

type Entry = unsafe extern "C" fn(*mut c_void, *mut Registers, *const Services);

struct Process<'a> {
    guest: &'a mut dos::Loaded,
    output: CaptureOutput,
    exit: Option<u8>,
    error: Option<String>,
}

fn call(
    context: *mut c_void,
    operation: impl FnOnce(&mut Process<'_>) -> Result<i32, String>,
) -> i32 {
    let Some(process) = (unsafe { context.cast::<Process<'_>>().as_mut() }) else {
        return -1;
    };
    if process.error.is_some() {
        return -1;
    }
    if process.exit.is_some() {
        return 1;
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(process))) {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => {
            process.error = Some(error);
            -1
        }
        Err(_) => {
            process.error = Some("compiled service panicked".into());
            -1
        }
    }
}

unsafe extern "C" fn interrupt(context: *mut c_void, state: *mut Registers, vector: u8) -> i32 {
    call(context, |process| {
        let state = unsafe { state.as_mut() }.ok_or("null guest registers")?;
        let guest = &mut *process.guest;
        guest.cpu.regs = state.gpr;
        guest.cpu.eflags = state.eflags;
        guest.cpu.eip = u32::from(state.ip);
        for (i, &segment) in state.segments.iter().enumerate() {
            if segment != guest.selectors[i] {
                guest.image.segments[i] = guest.image.segments[i].real_mode_reload(segment);
            }
        }
        guest.selectors = state.segments;
        guest.image.selectors = state.segments;
        process.exit = dos::compiled_interrupt(guest, vector, &mut process.output)?;
        state.gpr = guest.cpu.regs;
        state.eflags = guest.cpu.eflags;
        state.segments = guest.selectors;
        Ok(i32::from(process.exit.is_some()))
    })
}

fn width(bytes: u8) -> Result<MemWidth, String> {
    match bytes {
        1 => Ok(MemWidth::Byte),
        2 => Ok(MemWidth::Word),
        4 => Ok(MemWidth::Dword),
        _ => Err("compiled memory access width must be 1, 2, or 4 bytes".into()),
    }
}

unsafe extern "C" fn read(context: *mut c_void, address: u32, bytes: u8, value: *mut u32) -> i32 {
    call(context, |process| {
        let result = unsafe { value.as_mut() }.ok_or("null memory result")?;
        *result = process
            .guest
            .ram
            .load(address, width(bytes)?)
            .map_err(|e| format!("{e:?}"))?;
        Ok(0)
    })
}

unsafe extern "C" fn write(context: *mut c_void, address: u32, bytes: u8, value: u32) -> i32 {
    call(context, |process| {
        process
            .guest
            .ram
            .store(address, width(bytes)?, value)
            .map_err(|e| format!("{e:?}"))?;
        Ok(0)
    })
}

unsafe extern "C" fn port(
    context: *mut c_void,
    number: u16,
    bytes: u8,
    input: u8,
    value: *mut u32,
) -> i32 {
    call(context, |process| {
        if input > 1 {
            return Err("invalid compiled I/O direction".into());
        }
        let width = width(bytes)?;
        let value = unsafe { value.as_mut() }.ok_or("null port value")?;
        let result = dos::compiled_port(
            process.guest,
            number,
            width.bytes(),
            input == 1,
            if input == 1 { 0 } else { *value },
        )?;
        if input == 1 {
            *value = result;
        }
        Ok(0)
    })
}

static SERVICES: Services = Services {
    port,
    interrupt,
    read,
    write,
};

/// Call a native entry point with COM startup state and a reserved image at 0100h.
/// Returns the DOS exit code, or -1 on a runtime error.
/// Foreign pointers and the callback must remain valid for this call.
#[no_mangle]
pub unsafe extern "C" fn dine_run_compiled_v1(
    data: *const u8,
    len: usize,
    image_len: usize,
    data_offset: usize,
    name: *const c_char,
    tail: *const c_char,
    entry: Option<Entry>,
) -> i32 {
    let run = || -> Result<i32, String> {
        let entry = entry.ok_or("null compiled entry point")?;
        if name.is_null() || tail.is_null() || (len != 0 && data.is_null()) || len > 0xff00 {
            return Err("invalid compiled COM data, name, or command tail".into());
        }
        let data = if len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(data, len) }
        };
        let name = unsafe { CStr::from_ptr(name) }
            .to_str()
            .map_err(|e| e.to_string())?;
        let tail = unsafe { CStr::from_ptr(tail) }
            .to_str()
            .map_err(|e| e.to_string())?;
        let (mut session, _) =
            super::prepare_image(data, name, tail, None, Some((image_len, data_offset)))?;
        let guest = session.machine_mut().ok_or("session state lost")?;
        let mut state = Registers {
            gpr: guest.cpu.regs,
            eflags: guest.cpu.eflags,
            ip: guest.cpu.eip as u16,
            segments: guest.selectors,
        };
        let mut process = Process {
            guest,
            output: CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT),
            exit: None,
            error: None,
        };
        unsafe {
            entry(
                (&mut process as *mut Process<'_>).cast(),
                &mut state,
                &SERVICES,
            )
        };
        let transcript = crate::dos_text_io::transcript(&process.output.into_bytes());
        std::io::stdout()
            .lock()
            .write_all(transcript.as_bytes())
            .map_err(|e| e.to_string())?;
        if let Some(error) = process.error {
            return Err(error);
        }
        process
            .exit
            .map(i32::from)
            .ok_or_else(|| "compiled entry returned before DOS exit".into())
    };
    match std::panic::catch_unwind(run) {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => {
            eprintln!("Dine compiled: {error}");
            -1
        }
        Err(_) => {
            eprintln!("Dine compiled: runtime panicked");
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn native_entry(
        context: *mut c_void,
        state: *mut Registers,
        api: *const Services,
    ) {
        let (state, api) = unsafe { (&mut *state, &*api) };
        let ds = u32::from(state.segments[3]) << 4;
        let mut value = 0;
        if state.ip != 0x100
            || state.gpr[4] != 0xfffe
            || unsafe { (api.read)(context, ds, 2, &mut value) } != 0
            || value != 0x20cd
            || unsafe { (api.read)(context, ds + 0x100, 2, &mut value) } != 0
            || value != 0
            || unsafe { (api.read)(context, ds + 0x140, 2, &mut value) } != 0
            || value != 0x5a4d
            || unsafe { (api.read)(context, ds + 0x80, 1, &mut value) } != 0
            || value != 2
            || unsafe { (api.write)(context, ds + 0x142, 1, u32::from(b'!')) } != 0
        {
            return;
        }
        state.gpr[0] = 0x0900;
        state.gpr[2] = 0x140;
        if unsafe { (api.interrupt)(context, state, 0x21) } != 0 {
            return;
        }
        state.gpr[0] = 0x3e00;
        state.gpr[3] = 0xffff;
        if unsafe { (api.interrupt)(context, state, 0x21) } != 0
            || state.eflags & 1 == 0
            || state.gpr[0] as u16 != 6
        {
            return;
        }
        state.gpr[0] = 0x4c2a;
        unsafe { (api.interrupt)(context, state, 0x21) };
    }

    #[test]
    fn compiled_com_enters_native_code_with_only_data_and_returns_dos_exit() {
        let status = unsafe {
            dine_run_compiled_v1(
                b"MZ?$".as_ptr(),
                4,
                0x80,
                0x40,
                b"FIXTURE.COM\0".as_ptr().cast(),
                b"X\0".as_ptr().cast(),
                Some(native_entry),
            )
        };
        assert_eq!(status, 42);
    }
}
