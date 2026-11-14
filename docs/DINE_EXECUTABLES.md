# Small, self-running Dine applications

A `Dine_<program>` launcher is a hybrid executable: a tiny loader plus the
original guest bytes, running on the **shared** Dine library. It contains no
copy of Dine or PIL (Tony, 13 November 2026: "as long as they contain not
dine or pil they are fine"). The library is the same for every program and
holds no generated code for one guest. When block compilation comes back it
will live inside that shared library, program-independent, so every
launcher gets it ([Block compiler](development/BLOCK_COMPILER.md)).

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
`--lib --features dine-library`, which never holds **application-specific blocks**. The launcher is then
compiled separately with the local Rust compiler and `opt-level=z` to favor
small output. Actual binary sizes depend on the compiler and target.

## DOS Turbo Pascal and long host filenames

The host native launcher translates a named source argument to the **same
8.3 alias that the existing Dine mapped-host drive provides**. This is
not a change to Turbo Pascal itself, and the original Pascal file is never
renamed or copied.

Build `Dine_TURBO.exe` from your existing DOS Turbo Pascal executable. Then
run it **from the folder holding the Pascal source**, so that folder is mapped
as Dine's DOS C: drive:

```powershell
python tools/build_dine.py path/to/TURBO.EXE
Set-Location path/to/pascal-sources
C:\path\to\PIL-Processor-Instruction-Language\programs\compiled\Dine_TURBO.exe this_is_my_long_name_pascal_code_in_x86_windows.pas
```

If the source exists, and Dine assigns it `THIS_I~1.PAS`, the legacy DOS
command tail contains `THIS_I~1.PAS`. Turbo's DOS open/find operations use
that alias to access the **original** host file. Dine itself retains native
UTF-8 host filenames in the directory listing. Actual aliases are allocated
by the directory listing, with collision suffixes, not hardcoded in the
launcher. Quoted filename arguments with spaces are supported too.

This bridge supports existing **files in the current directory** and maps
the caller's current directory as a writable host drive. It does not yet
translate native absolute paths, nested host path components or nonexisting
long filenames intended for creation. Do not launch an untrusted DOS program
from a directory containing sensitive files: mapped-drive DOS file operations
can modify host files there. For other directories, use a dedicated working
folder. DOS command tail length still limits the **translated** arguments.

A successful short-alias file-open test is not proof that the full Turbo
Pascal interactive editor, compiler, keyboard or DOS services work yet.
The initial Dine launcher still has only terminal input/preview support.

## Present runtime limitations

The initial `dine_run` ABI accepts the original guest bytes, DOS filename
and command tail. It runs all reached instructions through the existing PIL
interpreter plus the Dine DOS/BIOS services; there are **no native guest
compiled blocks yet**. A real `Stop::Exit` becomes the process exit status.
Budget stops continue automatically. The shared Rust console now handles host
input and output for this entry point as well as the CLI.

With terminal stdin/stdout it displays the shared DOS text page and sends real
BIOS keys, including typed CP437 text, arrows and function keys. On Windows,
graphics modes use the existing separate Win32/GDI window. The guest screen,
ANSI driver and VGA devices are the same ones used by the interpreter session.

With redirected stdin/stdout, or `DINE_TEXT=headless`, it uses UTF-8 transcript
output and bounded line input. EOF during a guest read is an explicit error;
it no longer discards the typed line and sends Enter alone. Transcript mode
escapes guest control sequences and does not present BIOS/direct-VRAM drawings.
`DINE_TEXT=terminal` requires terminal input and output; `auto` is the default.
The Dine entry point now uses `DINE_TEXT` for this choice rather than the older
`DINE_VIDEO` preview selection. `DINE_VIDEO` still applies to the separate JSON
session/native-video frontend.

Host keyboard polling leaves no detached stdin reader after `dine_run` returns,
and terminal modes are restored on exit or unwinding. Physical held-key state,
custom font rendering, blinking attributes, speaker playback and COM1-only
console routing remain limited. See [Rust DOS console](DOS_CONSOLE.md).

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
