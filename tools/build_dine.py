#!/usr/bin/env python3
"""Build a small self-running Dine_<program> launcher and shared Dine runtime.

The launcher contains only original DOS bytes, a tiny C ABI loader and the
host command tail. The shared dine library contains no application-specific
blocks or program bytes. No Python, PIL executable or original DOS input file
is needed after building, but dine.dll/libdine.so must accompany the launcher.
"""
import argparse
import importlib.util
import hashlib
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
_startup_spec = importlib.util.spec_from_file_location("pil_runtime_startup", Path(__file__).with_name("runtime_startup.py"))
runtime_startup = importlib.util.module_from_spec(_startup_spec)
_startup_spec.loader.exec_module(runtime_startup)

# This is deliberately a dynamic-library loader, not a link of the entire
# interpreter into each executable. It uses only the Rust standard library
# and platform loader APIs, and verifies the stable C ABI before execution.
LAUNCHER = r'''
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};

const PROGRAM: &[u8] = include_bytes!(@@GUEST_PATH@@);
const GUEST_NAME: &str = @@GUEST_NAME@@;
const DINE_ABI: u32 = 1;
const PREBUILT_RUNTIME: bool = @@PREBUILT_RUNTIME@@;

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
    // In a source checkout, use its one shared startup checker before loading
    // the DLL. An explicitly supplied library is loaded as deployed.
    if !PREBUILT_RUNTIME && std::env::var("PIL_STARTUP_CHECKED").as_deref() != Ok("1") {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        if let Some(root) = exe.ancestors().skip(1).find(|p| p.join("Cargo.toml").is_file() && p.join("tools/runtime_startup.py").is_file()) {
            let python = if cfg!(windows) { "python" } else { "python3" };
            let status = std::process::Command::new(python).arg(root.join("tools/runtime_startup.py"))
                .arg("--root").arg(root).arg("--kind").arg("library").arg("--engine").arg(runtime_path()?)
                .status().map_err(|e| format!("runtime startup check: {e}"))?;
            if !status.success() { return Err("runtime startup check failed".into()); }
        }
    }
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
    let mut arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.first().is_some_and(|arg| arg == "--dine-settings") {
        if arguments.len() != 1 { return Err("--dine-settings takes no guest arguments".into()); }
        type Edit = unsafe extern "C" fn() -> i32;
        let edit: Edit = unsafe { std::mem::transmute(runtime.symbol(c"dine_edit_settings")?) };
        return Ok(unsafe { edit() });
    }
    let mut timed = false;
    loop {
        match arguments.first().map(String::as_str) {
            Some("--dine-time") => { timed = true; },
            Some("--dine-interpreted") => { std::env::set_var("PIL_PRECOMPILED", "off"); },
            Some("--dine-native") => { std::env::set_var("PIL_PRECOMPILED", "on"); },
            _ => break,
        }
        arguments.remove(0);
    }
    let run: Run = unsafe {
        std::mem::transmute(runtime.symbol(if timed { c"dine_run_timed" } else { c"dine_run" })?)
    };
    let name = CString::new(GUEST_NAME).map_err(|e| e.to_string())?;
    // Preserve native argument boundaries in the UTF-8 tail sent to Dine.
    // The Dine compatibility shim resolves quoted host long names to
    // legacy DOS aliases before constructing PSP:0080.
    let tail = arguments.into_iter().map(|arg| {
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


def cached_runtime() -> Path | None:
    library = ROOT / "target" / "release" / library_name()
    return library if runtime_startup.current(ROOT, library, "library") else None


def build_runtime() -> Path:
    return runtime_startup.ensure(ROOT, ROOT / "target" / "release" / library_name(), "library")


def launcher_name(program: Path) -> str:
    return f"Dine_{program.stem}{'.exe' if os.name == 'nt' else ''}"


def reusable_launcher(program: Path, output: Path, runtime: Path | None, *, settings: bool = False, timing: bool = False) -> bool:
    """Reuse only a recorded build of these guest bytes and its shared runtime."""
    launcher = output / launcher_name(program)
    library = output / library_name()
    try:
        record = json.loads(launcher.with_name(launcher.name + ".json").read_text(encoding="utf-8"))
        if not isinstance(record, dict) or not launcher.is_file() or not library.is_file():
            return False
        _, guest = read_program(program)
        if (record.get("format") != 1 or record.get("application") != launcher.name
                or record.get("guest") != program.name
                or record.get("guest_sha256") != hashlib.sha256(guest).hexdigest()
                or record.get("runtime") != library.name
                or record.get("original_guest_embedded") is not True):
            return False
        if record.get("prebuilt_runtime", False) != (runtime is not None):
            return False
        runtime_hash = hashlib.sha256(library.read_bytes()).hexdigest()
        if settings and record.get("settings_editor") is not True:
            return False
        if timing and record.get("execution_timing") is not True:
            return False
        if record.get("runtime_sha256") != runtime_hash and not runtime_startup.current(ROOT, library, "library"):
            return False
        if runtime is not None and hashlib.sha256(runtime.read_bytes()).hexdigest() != runtime_hash:
            return False
        # Older manifests did not record a launcher hash. Keep them runnable;
        # new builds also detect an accidentally replaced native executable.
        expected = record.get("launcher_sha256")
        return expected is None or expected == hashlib.sha256(launcher.read_bytes()).hexdigest()
    except (OSError, ValueError):
        return False


def check_tools(runtime: Path | None) -> list[str]:
    required = ["rustc"] + ([] if runtime else ["cargo"])
    missing = [tool for tool in required if shutil.which(tool) is None]
    if missing:
        raise ValueError("missing " + ", ".join(missing) +
                         "; install the Rust toolchain, then run --doctor again")
    return required


def build(program: Path, output_dir: Path, force: bool = False, *,
          runtime: Path | None = None, prebuilt_runtime: bool = False) -> Path:
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
                .replace("@@GUEST_NAME@@", rust_raw(program.name))
                .replace("@@PREBUILT_RUNTIME@@", str(prebuilt_runtime).lower()),
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
    runtime_startup.record_copy(ROOT, library, output_dir / library.name)
    pending.replace(launcher)
    manifest = {
        "format": 1,
        "backend": "Shared PIL/Dine native forms with interpreter fallback",
        "application": launcher.name,
        "launcher_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "settings_editor": True,
        "execution_timing": True,
        "guest": program.name,
        "guest_sha256": hashlib.sha256(guest).hexdigest(),
        "runtime": library.name,
        "prebuilt_runtime": prebuilt_runtime,
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
    parser.add_argument("--runtime", type=Path, help="use a prebuilt shared runtime for build and direct launch (skips Cargo)")
    parser.add_argument("--run", action="store_true", help="run an unchanged existing output, or build and run it")
    parser.add_argument("--time", action="store_true", help="time execution to exit or first input wait, without host drawing")
    parser.add_argument("--interpreted", action="store_true", help="use the interpreter for this launch")
    parser.add_argument("--compare", action="store_true", help="time interpreted and native-form execution of the same output")
    parser.add_argument("--settings", action="store_true", help="open the shared PILdos settings editor before an optional run")
    parser.add_argument("--config", type=Path, help="DOS configuration to use for settings/run (default: PILDOS_CONFIG or pildos.conf)")
    parser.add_argument("--working-dir", type=Path, help="run from this folder (default: input file's folder)")
    parser.add_argument("--doctor", action="store_true", help="check the required tools without building")
    argv = list(sys.argv[1:] if argv is None else argv)
    guest_args = []
    if "--" in argv:
        at = argv.index("--")
        argv, guest_args = argv[:at], argv[at + 1:]
    args = parser.parse_args(argv)
    args.time |= args.compare
    args.run |= args.time or args.interpreted
    if args.compare and args.interpreted:
        parser.error("--compare selects both execution modes; omit --interpreted")
    if guest_args and not args.run:
        parser.error("arguments after -- require --run")
    if args.working_dir is not None and not (args.run or args.settings):
        parser.error("--working-dir requires --run or --settings")
    if (args.run or args.settings) and len(args.program) > 1:
        parser.error("--run/--settings accept one input program")
    if args.config is not None and not (args.run or args.settings):
        parser.error("--config requires --run, --time or --settings")
    try:
        runtime = args.runtime.resolve(strict=True) if args.runtime else None
        if runtime is not None and (not runtime.is_file() or runtime.name != library_name()):
            raise ValueError(f"--runtime must name a shared {library_name()} for this platform")
        if args.doctor:
            for tool in check_tools(runtime):
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
        runtime_startup.git_status(ROOT)
        # Preflight the complete batch before compiling or publishing anything.
        programs = [read_program(path)[0] for path in args.program]
        names = [launcher_name(path).casefold() for path in programs]
        if len(names) != len(set(names)):
            raise ValueError("inputs produce the same output name; build them into separate output folders")
        output = args.output_dir.resolve()
        reuse = None
        for path in programs:
            destination = output / launcher_name(path)
            if destination.exists() and not args.force:
                if (args.run or args.settings) and reusable_launcher(path, output, runtime,settings=args.settings,timing=args.time):
                    reuse = destination
                else:
                    raise FileExistsError(f"{destination} already exists (use --force to replace it; "
                                          "--run reuses only a matching recorded build)")
        working_dir = (args.working_dir or programs[0].parent).resolve(strict=True)
        if args.run and not working_dir.is_dir():
            raise ValueError("--working-dir must name a directory")
        if reuse is not None:
            selected = runtime if runtime is not None else build_runtime()
            deployed = output / library_name()
            digest = hashlib.sha256(selected.read_bytes()).hexdigest()
            if hashlib.sha256(deployed.read_bytes()).hexdigest() != digest:
                pending = deployed.with_name(deployed.name + ".pending")
                shutil.copy2(selected, pending)
                pending.replace(deployed)
                print("Updated shared runtime; launcher unchanged.", flush=True)
            runtime_startup.record_copy(ROOT, selected, deployed)
            metadata = reuse.with_name(reuse.name + ".json")
            record = json.loads(metadata.read_text(encoding="utf-8"))
            if record.get("runtime_sha256") != digest:
                record["runtime_sha256"] = digest
                metadata.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
            print(f"Using existing Dine executable: {reuse}", flush=True)
            return launch_output(reuse,working_dir,guest_args,args)
        if runtime is None:
            runtime = cached_runtime()
            if runtime is not None:
                print(f"Using compiled shared runtime: {runtime}", flush=True)
        check_tools(runtime)
        if runtime is None:
            runtime = build_runtime()
        results = []
        for index, program in enumerate(programs, 1):
            print(f"[{index}/{len(programs)}] Building native launcher: {program.name}", flush=True)
            result = build(program, output, args.force, runtime=runtime,
                           prebuilt_runtime=args.runtime is not None)
            results.append(result)
            print(f"Dine executable: {result}", flush=True)
        print(f"Shared runtime: {output / library_name()}", flush=True)
        print("Keep the shared runtime beside the executable(s).", flush=True)
        if args.run or args.settings:
            return launch_output(results[0],working_dir,guest_args,args)
        return 0
    except (OSError, ValueError, EOFError) as error:
        print(f"Build failed: {error}", file=sys.stderr)
        return 1
    except subprocess.CalledProcessError as error:
        print(f"Build failed: {error.cmd[0]} exited with status {error.returncode}", file=sys.stderr)
        return 1


def launch_output(launcher: Path, working_dir: Path, guest_args: list[str], args) -> int:
    env = os.environ.copy()
    env["PIL_STARTUP_CHECKED"] = "1"
    selected = args.config or env.get("PILDOS_CONFIG") or None
    if selected is None and Path("pildos.conf").is_file():
        selected = Path("pildos.conf")
    if selected is not None:
        env["PILDOS_CONFIG"] = str(Path(selected).resolve())
    if args.settings:
        status = subprocess.run([str(launcher), "--dine-settings"], cwd=working_dir, env=env).returncode
        if status or not args.run:
            return status
    if args.time:
        guest_args = ["--dine-time", *guest_args]
    if args.interpreted:
        env["PIL_PRECOMPILED"] = "off"
    print(f"Running from: {working_dir}", flush=True)
    if args.compare:
        times = []
        results = []
        for mode, label in (("off", "Interpreted"), ("on", "Native forms")):
            selected_env = dict(env, PIL_PRECOMPILED=mode)
            result = subprocess.run([str(launcher), *guest_args], cwd=working_dir, env=selected_env,
                                    capture_output=True, text=True)
            print(f"{label}: {result.stdout.strip()}", flush=True)
            if result.stderr:
                print(result.stderr, file=sys.stderr, end="")
            match = re.search(r"DINE_TIMING elapsed_ms=([0-9.]+) instructions=(\d+) boundary=(\w+)", result.stdout)
            if match is None:
                return result.returncode or 1
            times.append(float(match[1]))
            results.append((result.returncode, match[2], match[3]))
        if results[0] != results[1]:
            raise ValueError("execution modes disagree on exit status, instruction count or timing boundary")
        if times[1] > 0:
            print(f"Native-form speedup: {times[0] / times[1]:.2f}x (presentation off)", flush=True)
        return results[1][0]
    return subprocess.run([str(launcher), *guest_args], cwd=working_dir, env=env).returncode


if __name__ == "__main__":
    sys.exit(main())
