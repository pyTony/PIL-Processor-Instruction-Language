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

For a 16-colour planar VGA mode, the initial host-only display is a sampled
**80x24 truecolour terminal preview** shown at the first keyboard wait. This
is not a graphical 640x480 window, live frame renderer or SDL replacement.
The actual framebuffer continues to be the existing shared Dine VGA model,
including 0x12 and mapped planes. Visual output should be tested in an ANSI
truecolour terminal. Text output uses the existing CP437 decoder.

The runtime C ABI is version 1 and currently exports
`dine_abi_version() -> u32` and
`dine_run(bytes, byte_count, name, tail) -> i32`.
The loader verifies the ABI before invoking guest code and reports a missing
or incompatible library rather than running without DOS services.

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
image fidelity. No new runtime benchmarks, builds or graphics parity checks
have been executed in this chat.
