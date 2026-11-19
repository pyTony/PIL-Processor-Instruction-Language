#!/usr/bin/env python3
"""Build a small self-running Dine_<program> launcher and shared Dine runtime.

The launcher contains only original DOS bytes, a tiny C ABI loader and the
host command tail. The shared dine library contains no application-specific
blocks or program bytes. No Python, PIL executable or original DOS input file
is needed after building, but dine.dll/libdine.so must accompany the launcher.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]

# This is deliberately a dynamic-library loader, not a link of the entire
# interpreter into each executable. It uses only the Rust standard library
# and platform loader APIs, and verifies the stable C ABI before execution.
LAUNCHER = r'''
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};

const PROGRAM: &[u8] = include_bytes!(@@GUEST_PATH@@);
const GUEST_NAME: &str = @@GUEST_NAME@@;
const DINE_ABI: u32 = 1;

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn FreeLibrary(handle: *mut c_void) -> i32;
}
#[cfg(unix)]
#[cfg_attr(not(target_os = "macos"), link(name = "dl"))]
extern "C" {
    fn dlopen(name: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
}
#[cfg(not(any(windows, unix)))]
compile_error!("Dine launchers currently support Windows and Unix hosts");

struct Runtime { handle: *mut c_void }
impl Runtime {
    fn open(path: &Path) -> Result<Self, String> {
        #[cfg(windows)]
        let handle = {
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = path.as_os_str().encode_wide()
                .chain(std::iter::once(0)).collect();
            unsafe { LoadLibraryW(wide.as_ptr()) }
        };
        #[cfg(unix)]
        let handle = {
            use std::os::unix::ffi::OsStrExt;
            let path = CString::new(path.as_os_str().as_bytes())
                .map_err(|_| "invalid native library path")?;
            unsafe { dlopen(path.as_ptr(), 2) } // RTLD_NOW
        };
        if handle.is_null() {
            return Err(format!("cannot load Dine runtime {}: {}",
                path.display(), std::io::Error::last_os_error()));
        }
        Ok(Self { handle })
    }
    fn symbol(&self, name: &CStr) -> Result<*mut c_void, String> {
        #[cfg(windows)]
        let symbol = unsafe { GetProcAddress(self.handle, name.as_ptr()) };
        #[cfg(unix)]
        let symbol = unsafe { dlsym(self.handle, name.as_ptr()) };
        if symbol.is_null() {
            return Err(format!("Dine runtime does not export {}",
                name.to_string_lossy()));
        }
        Ok(symbol)
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe { FreeLibrary(self.handle); }
        #[cfg(unix)]
        unsafe { dlclose(self.handle); }
    }
}

fn runtime_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let folder = exe.parent().ok_or("launcher has no parent directory")?;
    #[cfg(windows)]
    let library = "dine.dll";
    #[cfg(target_os = "macos")]
    let library = "libdine.dylib";
    #[cfg(all(unix, not(target_os = "macos")))]
    let library = "libdine.so";
    Ok(folder.join(library))
}

fn launch() -> Result<i32, String> {
    let runtime = Runtime::open(&runtime_path()?)?;
    type Version = unsafe extern "C" fn() -> u32;
    type Run = unsafe extern "C" fn(*const u8, usize, *const c_char, *const c_char) -> i32;
    let version: Version = unsafe {
        std::mem::transmute(runtime.symbol(c"dine_abi_version")?)
    };
    let actual = unsafe { version() };
    if actual != DINE_ABI {
        return Err(format!("Dine DLL ABI {actual} is not supported; expected {DINE_ABI}"));
    }
    let run: Run = unsafe {
        std::mem::transmute(runtime.symbol(c"dine_run")?)
    };
    let name = CString::new(GUEST_NAME).map_err(|e| e.to_string())?;
    // Preserve native argument boundaries in the UTF-8 tail sent to Dine.
    // The Dine compatibility shim resolves quoted host long names to
    // legacy DOS aliases before constructing PSP:0080.
    let tail = std::env::args().skip(1).map(|arg| {
        if arg.chars().any(char::is_whitespace) {
            format!("\"{arg}\"")
        } else { arg }
    }).collect::<Vec<_>>().join(" ");
    let tail = CString::new(tail).map_err(|_| "NUL character in DOS command line")?;
    Ok(unsafe { run(PROGRAM.as_ptr(), PROGRAM.len(), name.as_ptr(), tail.as_ptr()) })
}

fn main() {
    match launch() {
        Ok(code) => std::process::exit(code),
        Err(e) => { eprintln!("Dine launcher: {e}"); std::process::exit(1); }
    }
}
'''


def rust_raw(text: str) -> str:
    """A source literal including backslashes and Unicode, without escaping."""
    hashes = "#"
    while f'"{hashes}' in text:
        hashes += "#"
    return f'r{hashes}"{text}"{hashes}'


def library_name() -> str:
    if os.name == "nt":
        return "dine.dll"
    if sys.platform == "darwin":
        return "libdine.dylib"
    return "libdine.so"


def read_program(program: Path) -> tuple[Path, bytes]:
    program = program.resolve(strict=True)
    if not program.is_file() or program.suffix.lower() not in (".com", ".exe"):
        raise ValueError("input must be an existing DOS .COM or .EXE file")
    with program.open("rb") as source:
        guest = source.read(1_048_577)
    if not 1 <= len(guest) <= 1_048_576:
        raise ValueError("DOS program must contain 1..1048576 bytes")
    return program, guest


def build_runtime() -> Path:
    """Build the same shared system implementation, independently of inputs."""
    env = os.environ.copy()
    env.pop("CARGO_BUILD_TARGET", None)
    env.pop("CARGO_TARGET_DIR", None)
    subprocess.run(
        ["cargo", "build", "--release", "--locked", "--lib",
         "--features", "dine-library"],
        cwd=ROOT, env=env, check=True,
    )
    library = ROOT / "target" / "release" / library_name()
    if not library.is_file():
        raise FileNotFoundError(f"expected shared Dine library {library}")
    return library


def launcher_name(program: Path) -> str:
    return f"Dine_{program.stem}{'.exe' if os.name == 'nt' else ''}"


def build(program: Path, output_dir: Path, force: bool = False, *, runtime: Path | None = None) -> Path:
    program, guest = read_program(program)
    output_dir = output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    launcher = output_dir / launcher_name(program)
    if launcher.exists() and not force:
        raise FileExistsError(f"{launcher} already exists (use --force to replace it)")

    # The shared Dine library is the same for every program: it never
    # contains generated Rust or compiled blocks for this guest.
    env = os.environ.copy()
    env.pop("CARGO_BUILD_TARGET", None)
    env.pop("CARGO_TARGET_DIR", None)
    library = runtime.resolve(strict=True) if runtime is not None else build_runtime()
    if not library.is_file():
        raise FileNotFoundError(f"expected shared Dine library {library}")

    # Compiled launcher source lives in target/, not beside the final exe.
    intermediate = ROOT / "target" / "dine-launchers" / program.stem
    intermediate.mkdir(parents=True, exist_ok=True)
    # Embed the bytes we hashed, using a relative source path. Subsequent
    # changes to the original input cannot change this build's payload.
    (intermediate / "guest.bin").write_bytes(guest)
    source = intermediate / "main.rs"
    source.write_text(
        LAUNCHER.replace("@@GUEST_PATH@@", rust_raw("guest.bin"))
                .replace("@@GUEST_NAME@@", rust_raw(program.name)),
        encoding="utf-8",
    )
    binary = intermediate / launcher.name
    subprocess.run(
        ["rustc", "--edition=2021", "-C", "opt-level=z",
         "-C", "panic=abort", "-C", "strip=symbols",
         str(source), "-o", str(binary)],
        cwd=ROOT, env=env, check=True,
    )
    if not binary.is_file():
        raise FileNotFoundError(f"launcher was not generated: {binary}")

    # Publish a complete executable/runtime pair only once both exist.
    pending = output_dir / (launcher.name + ".pending")
    shutil.copy2(binary, pending)
    shared_pending = output_dir / (library.name + ".pending")
    shutil.copy2(library, shared_pending)
    shared_pending.replace(output_dir / library.name)
    pending.replace(launcher)
    manifest = {
        "format": 1,
        "backend": "Shared PIL/Dine native forms with interpreter fallback",
        "application": launcher.name,
        "guest": program.name,
        "guest_sha256": hashlib.sha256(guest).hexdigest(),
        "runtime": library.name,
        "runtime_sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
        "interpreter_fallback": True,
        "compiled_guest_blocks": False,
        "original_guest_embedded": True,
        "external_dos_file_required": False,
    }
    (output_dir / (launcher.name + ".json")).write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )
    return launcher


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, epilog=(
        "Builds for this computer's native platform. The output uses shared PIL/Dine "
        "native forms with interpreter fallback; DOS compatibility is not guaranteed. "
        "Use --run -- ARGUMENTS to pass arguments to a single built program."))
    parser.add_argument("program", type=Path, nargs="*", help="DOS input files (quote paths containing spaces)")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "programs" / "compiled")
    parser.add_argument("--force", action="store_true", help="replace an existing launcher")
    parser.add_argument("--runtime", type=Path, help="reuse a prebuilt shared runtime (skips Cargo)")
    parser.add_argument("--run", action="store_true", help="run the single output after building")
    parser.add_argument("--working-dir", type=Path, help="run from this folder (default: input file's folder)")
    parser.add_argument("--doctor", action="store_true", help="check the required tools without building")
    argv = list(sys.argv[1:] if argv is None else argv)
    guest_args = []
    if "--" in argv:
        at = argv.index("--")
        argv, guest_args = argv[:at], argv[at + 1:]
    args = parser.parse_args(argv)
    if guest_args and not args.run:
        parser.error("arguments after -- require --run")
    if args.working_dir is not None and not args.run:
        parser.error("--working-dir requires --run")
    if args.run and len(args.program) > 1:
        parser.error("--run accepts one input program")
    try:
        runtime = args.runtime.resolve(strict=True) if args.runtime else None
        if runtime is not None and (not runtime.is_file() or runtime.name != library_name()):
            raise ValueError(f"--runtime must name a shared {library_name()} for this platform")
        required = ["rustc"] + ([] if runtime else ["cargo"])
        missing = [tool for tool in required if shutil.which(tool) is None]
        if missing:
            raise ValueError("missing " + ", ".join(missing) +
                             "; install the Rust toolchain, then run --doctor again")
        if args.doctor:
            for tool in required:
                subprocess.run([tool, "--version"], check=True)
            print("Ready to build for this computer's native platform.", flush=True)
            return 0
        if not args.program:
            if not sys.stdin.isatty():
                parser.error("provide at least one DOS input file (or use --doctor)")
            value = input("DOS program path: ").strip().strip('"')
            if not value:
                parser.error("a DOS input file is required")
            args.program = [Path(value)]
        # Preflight the complete batch before compiling or publishing anything.
        programs = [read_program(path)[0] for path in args.program]
        names = [launcher_name(path).casefold() for path in programs]
        if len(names) != len(set(names)):
            raise ValueError("inputs produce the same output name; build them into separate output folders")
        output = args.output_dir.resolve()
        for path in programs:
            destination = output / launcher_name(path)
            if destination.exists() and not args.force:
                raise FileExistsError(f"{destination} already exists (use --force to replace it)")
        working_dir = (args.working_dir or programs[0].parent).resolve(strict=True)
        if args.run and not working_dir.is_dir():
            raise ValueError("--working-dir must name a directory")
        if runtime is None:
            print("Building shared PIL/Dine runtime (first build can take several minutes)...", flush=True)
            runtime = build_runtime()
        results = []
        for index, program in enumerate(programs, 1):
            print(f"[{index}/{len(programs)}] Building native launcher: {program.name}", flush=True)
            result = build(program, output, args.force, runtime=runtime)
            results.append(result)
            print(f"Dine executable: {result}", flush=True)
        print(f"Shared runtime: {output / library_name()}", flush=True)
        print("Keep the shared runtime beside the executable(s).", flush=True)
        if args.run:
            print(f"Running from: {working_dir}", flush=True)
            return subprocess.run([str(results[0]), *guest_args], cwd=working_dir).returncode
        return 0
    except (OSError, ValueError, EOFError) as error:
        print(f"Build failed: {error}", file=sys.stderr)
        return 1
    except subprocess.CalledProcessError as error:
        print(f"Build failed: {error.cmd[0]} exited with status {error.returncode}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
