# Small, self-running Dine applications

This is the **interpreted-first** host-native executable path. It is distinct
from [pure compiled DOS standalone binaries](NATIVE_DOS_COMPILATION.md), which
contain no x86 decoder or interpreter, and from the deferred profile-guided
linked shadow executable.

The names reflect separate responsibilities:

- **PIL**: processor instruction language, compiler and interpreter.
- **Dine**: the DOS/BIOS and device runtime built using PIL's interpreter.
- **`Dine_MANDVGA.exe`**: a small host executable containing the original
  guest program and a loader that calls `dine.dll` dynamically.

## Build and run on Windows

From the repository root:

```powershell
python tools/build_dine.py programs/dos/MANDVGA.COM
.\programs\compiled\Dine_MANDVGA.exe
```

Expected published files in `programs/compiled/`:

| File | Contents |
| --- | --- |
| `Dine_MANDVGA.exe` | Small Rust loader and original MANDVGA.COM bytes, **no Dine interpreter** |
| `dine.dll` | Shared, **program-independent** DOS services, PC device model and PIL interpreter |
| `Dine_MANDVGA.exe.json` | Build identity, guest hash and runtime hash |

The launcher loads the runtime from **its own directory**. It does not invoke
`pil.exe`, Python or `dos_terminal.py`; nor does it open the original COM
file at run time. You may move the launcher and DLL to another directory
together. You cannot yet move only the EXE: **the shared library is required**.

To replace the same executable after changes:

```powershell
python tools/build_dine.py programs/dos/MANDVGA.COM --force
```

Builds for Linux or Termux emit `Dine_MANDVGA` plus `libdine.so`; for macOS,
`Dine_MANDVGA` plus `libdine.dylib`. The library is built using Cargo's
`--lib --features dine-library`, with `PIL_SHADOW_SOURCE` deliberately
unset to keep **all application-specific blocks out**. The launcher is then
compiled separately with the local Rust compiler and `opt-level=z` to favor
small output. Actual binary sizes depend on the compiler and target.

## Present runtime limitations

The initial `dine_run` ABI accepts the original guest bytes, DOS filename
and command tail. It runs all reached instructions through the existing PIL
interpreter plus the Dine DOS/BIOS services; there are **no native guest
compiled blocks yet**. A real `Stop::Exit` becomes the process exit status.
Budget stops continue automatically. At a DOS keyboard wait, the terminal
accepts Enter and enqueues its BIOS scan/ASCII key.

The Windows Dine DLL now provides a **live, host-native Win32/GDI VGA
window** for planar VGA modes 0Dh/0Eh/10h/12h and linear or unchained mode
13h. No third-party SDL/wgpu or Rust GUI crate is needed for this first
desktop display. The window samples the existing emulated VGA planes/DAC,
not a new graphics implementation. BIOS characters already drawn with the
software graphics font appear automatically because they are pixels in VGA
memory. Display updates are attempted at bounded interpreter slices with
an 80 ms minimum interval, and the last framebuffer is shown at input wait.
The image is stretched to a 4:3 desktop view, preserving intended VGA pixel
aspect ratios across 320x200 and 640x350 modes.

**Keyboard and text remain in the native terminal.** The non-activating
display window does not capture keystrokes, send game events or replace
DOS console I/O. At a BIOS input wait press Enter in the launching terminal.
Later interactive games will need a dedicated event-driven keyboard/mouse
frontend rather than terminal-line input. DOS text output continues through
the existing CP437 decoder. Text-mode VGA custom font plane contents are
still not presented as bitmapped glyphs in the desktop window.

The mode is selected at startup with `DINE_VIDEO`:

| `DINE_VIDEO` | Windows | Linux / Termux / other |
| --- | --- | --- |
| unset / `window` | Live native VGA window when graphics mode is active | Existing ANSI preview |
| `ansi` | 80x24 ANSI preview at input wait | Same |
| `off` | No graphical presentation, normal DOS text and terminal keyboard | Same |

Example on Windows (the original COM input is not needed at launch):

```powershell
$env:DINE_VIDEO = "window"
.\programs\compiled\Dine_MANDVGA.exe
Remove-Item Env:DINE_VIDEO
```

This GDI frontend is limited to **Windows desktop** today; it doesn't
pretend to be a cross-platform SDL implementation. CI and server hosts can
use `DINE_VIDEO=off` to avoid terminal escape sequences. The VGA sampler
does not emulate new graphics hardware or accelerate the DOS CPU.

The runtime C ABI is version 1 and currently exports
`dine_abi_version() -> u32` and
`dine_run(bytes, byte_count, name, tail) -> i32`.
The loader verifies the ABI before invoking guest code and reports a missing
or incompatible library rather than running without DOS services.

See [PILdos native CGA/VGA graphics](PILDOS_RUST_GRAPHICS.md) for the
opt-in Rust window with Python terminal and BIOS keyboard bridge.

## Refactoring direction

The next increment will move guest-specific compiled blocks into
`Dine_<program>.exe`, keeping `dine.dll` independent of the application.
That needs a versioned, opaque per-machine compiled-block callback ABI: a
native block receives Dine's CPU/memory context, validates instruction bytes
and segment state, reports consumed instructions, and falls back to PIL when
its guard misses. BIOS, DOS, FPU and VGA services stay in the shared runtime.
This change must keep instruction budgets, asynchronous interrupt boundaries,
code-modification guards and guest exception delivery consistent.

A separate future **native filesystem service** should support UTF-8 paths,
long host filenames, Unicode, directory enumeration, file metadata, 64-bit
offsets and opaque native handles. It should use permissioned host mount roots
and a common backend with the existing mapped DOS drives, but not misrepresent
this functionality as DOS 8.3, synthetic INT 21h or a completed Windows API.
Host path validation must prevent escaping configured roots, including via
symlinks or reparse points. The current version has only the existing Dine
DOS services; the new native filesystem ABI is not implemented.

## Verification

```powershell
cargo build --release --locked --lib --features dine-library
cargo test --locked
python -m unittest discover -s tests/gui -p test_build_dine.py
python tools/verify_dine_executable.py
python tools/build_dine.py programs/dos/MANDVGA.COM --force
.\programs\compiled\Dine_MANDVGA.exe
python tools/build_dine.py programs/dos/MANDANSI.COM --force
.\programs\compiled\Dine_MANDANSI.exe
```

Verify that the COM inputs can be renamed or removed **after** compilation,
the EXE still runs when the DLL is alongside it, and a missing DLL fails
clearly. Compare the Mandelbrot guest state and VGA planes to
`tools/verify_mandel_vga.py` and the linked-Dine reference before claiming
image fidelity. Full-window presentation and performance require manual Windows testing;
no run-time benchmark or graphics-window parity check has been executed
in this chat.
