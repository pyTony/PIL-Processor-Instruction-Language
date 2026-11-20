# Small, self-running Dine applications

A `Dine_<program>` launcher is a hybrid executable: a tiny loader plus the
original guest bytes, running on the **shared** Dine library. It contains no
copy of Dine or PIL (Tony, 13 November 2026: "as long as they contain not
dine or pil they are fine"). The library is the same for every program and
holds no generated code for one guest. Native instruction forms and run-time
block planning live inside that shared library, program-independent, so every
launcher gets them ([Block compiler](development/BLOCK_COMPILER.md)).

The names reflect separate responsibilities:

- **PIL**: processor instruction language, compiler and interpreter.
- **Dine**: the shared DOS/BIOS and device runtime using PIL's executor.
- **`Dine_MANDVGA.exe`**: a small host executable containing the original
  guest program and a loader that calls `dine.dll` dynamically.

The standard `pil` command calls the same Rust `dine` library. Its entry
point is a thin wrapper around `dine::run_cli()`; all CPU, DOS and device
implementation lives in the library. Cargo produces a Rust `rlib` for the
command and a `cdylib` for C ABI launchers from that one library target.
The command links the Rust library directly and can run without a separate
DLL. Build the C ABI with `--lib --features dine-library` as before.

## One-command build workflow

Install Python 3.10 or newer and the Rust toolchain (`cargo`, `rustc`, and the
platform linker). Builds target the current computer: Windows, Linux, macOS
or Termux. Cross-compilation is not provided by this workflow.

From the checkout, check the tools and build your DOS input:

```powershell
.\compile-dos.cmd --doctor
.\compile-dos.cmd "path\to\program.exe" --run
```

On Linux, macOS or Termux:

```sh
sh compile-dos.sh --doctor
sh compile-dos.sh path/to/program.exe --run
```

The cross-platform command is `python tools/build_dine.py PROGRAM`.
Without a program argument, an interactive terminal prompts for the input
path. Inputs may be DOS `.COM` or MZ `.EXE` files within Dine's 1 MiB limit.
The builder checks file existence, extension and size; Dine validates the
executable format when loading it. A successful build does not establish
that every instruction or DOS service used by that input is supported.

The first build compiles the shared system runtime and may take several
minutes. The shared startup checker verifies system inputs and the compiled
artifact on every source-checkout launch. Unchanged builds skip Cargo;
changed inputs invoke Cargo before the program starts. It then compiles the small native
launcher. The guest bytes are embedded as input to shared native forms and
interpreter fallback. This workflow does not translate the entire guest to
an independent, fully ahead-of-time native executable.

Useful options (shown with the Windows entry point; the Python and shell
entry points accept the same arguments):

```powershell
# Build only; choose where the executable and shared runtime go.
.\compile-dos.cmd "path\to\program.exe" --output-dir dist/my-app

# Rebuild and pass arguments to the program after --.
.\compile-dos.cmd "path\to\program.exe" --force --run -- "input file.dat" /help

# Build several launchers using one shared runtime build.
.\compile-dos.cmd path/to/first.com path/to/second.exe --output-dir dist/apps

# Reuse a built runtime; only rustc and the platform linker are then needed.
.\compile-dos.cmd path/to/program.exe --runtime programs/compiled/dine.dll
```

`--run` accepts one input and returns the program's exit status. After a
successful build, repeat the command with `--run` to reuse it without
recompiling or requiring Rust tools when the system build is current. The input
name and bytes must still match; newer builds also check the launcher hash.
The runtime must match its recorded hash or a verified shared-system update.
Existing manifests from before launcher hashes were added remain usable.
Changed guest bytes, modified launchers and missing/unrecorded outputs require
`--force` to rebuild.
Use `--force --run` when you explicitly want a fresh build.
`--force` replaces the small launcher; it does not force recompilation of an
unchanged shared library. If shared system code changed, an ordinary `--run`
refreshes the library while keeping the matching launcher.

The same checker (`tools/runtime_startup.py`) is used by PILdos, the Retro
launcher, PILbuild and source-checkout native launchers. It reports Git branch,
changed/untracked counts and known upstream ahead/behind state. A branch other
than `dev` or `main` produces a warning (including a message box for desktop
shortcuts). It neither changes Git branches nor fetches or updates the repository.
The check honors the shared build-in-progress lock and never launches a known
stale runtime after a failed build. Desktop source-checkout launchers need
Python for this check; Cargo is needed only for missing or stale system builds.
Distributed native applications outside a checkout retain their loader plus
shared-library layout without Python, Cargo or Git dependencies.

It runs
from the original input's folder so relative data-file access uses that
folder as DOS C:. Override this with `--working-dir path/to/data`. Companion
files are not bundled: keep them available in the run folder. When starting
the generated executable directly, its **current working directory** becomes
DOS C:. The builder does not change the input or companion files; running a
guest may change files through Dine's mapped-drive services.

### DOS configuration and optional editor

Native launchers read the same DOS configuration as PILdos: `PILDOS_CONFIG`
or optional `pildos.conf` in the current working directory. Drive mounts,
country, FPU, ANSI, mouse, clock offset, host pointer, speaker volume and disk
sets use the shared implementations. Configured drives override the default
working-directory mount. The config's startup program does not replace the
embedded guest, and command-line arguments still come from the launcher call.

The build workflow accepts `--config path/to/pildos.conf`. Otherwise it pins
an existing config from the invoking directory before changing to the guest's
folder; relative paths inside the config remain relative to the config file.
If no invoking-directory config is selected, the runtime uses its working
directory's optional config.

```powershell
# Open the existing shared PILdos editor, then run the embedded program.
.\compile-dos.cmd path/to/program.com --settings --run --config pildos.conf

# Open settings directly from a newly built launcher.
.\programs\compiled\Dine_program.exe --dine-settings
```

F2 saves; F10 saves and closes this editor; Esc closes without starting the
configuration's startup program. The editor and DOS services live in the
shared library. ANSI output needs `ansi=true`; the shared driver also accepts
the bright foreground SGR extension (90-97). Redirected transcript output
continues to escape control bytes rather than drawing terminal graphics.

### Native execution timing

```powershell
# Refresh an older launcher/runtime once, then measure it.
.\compile-dos.cmd path/to/program.com --force --time
# Repeat using the matching existing build.
.\compile-dos.cmd path/to/program.com --time
# Or invoke timing directly from the generated executable.
.\programs\compiled\Dine_program.exe --dine-time
# Time both execution modes using that same executable and configuration.
.\compile-dos.cmd path/to/program.com --compare
# Select interpretation for an ordinary interactive run.
.\compile-dos.cmd path/to/program.com --interpreted
```

Timing runs the generated native executable through shared native forms and
interpreter fallback, without host drawing or keyboard input. It reports
`DINE_TIMING elapsed_ms=... instructions=... boundary=exit|waiting_input`.
The timed interval covers guest execution and device/service work after
loading and configuration. It ends at program exit or the first input wait,
excluding human response time. A guest that asks for input before doing its
work will therefore time only its startup. DOS children launched by the
embedded guest stay in that same measured session.

The default execution budget is one billion steps; set
`DINE_BENCHMARK_STEPS` to 1..1,000,000,000 to change it. Budget exhaustion or
a guest fault is an error, not a successful timing result. This measurement
does not include graphical presentation or prove whole-program native
translation. Older launchers require `--force` once to gain the new settings
and timing entry points; the shared runtime must also be current.

`--compare` runs two fresh processes with `PIL_PRECOMPILED=off` and `on`,
respectively. It checks that exit status, retired instruction count and timing
boundary agree before printing the ratio. A single comparison is a measurement,
not a correctness proof. Direct native launchers accept `--dine-interpreted`
and `--dine-native`, combined with `--dine-time` in either order.

Local Windows x86-64 release example (19 November 2026, Rust 1.99.1): the
repository's original MANDVGA input reached its keyboard wait after
150,784,761 instructions. Three fresh native-process runs took 2,888.798,
2,862.683 and 2,872.042 ms (median 2,872.042 ms), with the same instruction
count each time and presentation disabled. These are execution timings on
one host, not an interpreter speed comparison or a graphical-frame-rate
measurement.
One paired run of the same mapped-drive VGA input took 35,107.416 ms interpreted
and 2,860.439 ms with native forms (12.27x), both ending after 150,784,761
instructions at the same keyboard wait, with presentation off. These use the
current `Dine_` launcher in both modes, not older experimental executables.

Batch inputs must produce distinct output names. Existing launchers are
protected unless `--force` is supplied. For distribution, ship the launcher
and platform-specific shared runtime together, along with any required data
files; Python, Rust, the checkout and the original input file are not needed
on the destination computer. Use the same host OS and architecture for the
destination. Native OS runtime requirements still apply.

## Build and run example on Windows

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
$launcher = (Resolve-Path programs/compiled/Dine_TURBO.exe).Path
Set-Location path/to/pascal-sources
& $launcher this_is_my_long_name_pascal_code_in_x86_windows.pas
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

The `dine_run` ABI accepts the original guest bytes, DOS filename and command
tail. The shared PIL executor uses native instruction forms and run-time
blocks when enabled and applicable, with interpreter fallback. Dine supplies
the shared DOS/BIOS services. The launcher contains no compiled guest blocks
or system implementation. A real `Stop::Exit` becomes the process exit status.
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
It also provides the additive optional exports `dine_edit_settings() -> i32`
and `dine_run_timed(bytes, byte_count, name, tail) -> i32`. Existing ABI-1
loaders remain compatible; a new launcher requesting a missing optional
export reports that the shared runtime needs updating.
The loader verifies the ABI before invoking guest code and reports a missing
or incompatible library rather than running without DOS services.

See [PILdos native CGA/VGA graphics](PILDOS_RUST_GRAPHICS.md) for the
opt-in Rust window with Python terminal and BIOS keyboard bridge.

## Refactoring direction

Keep all system execution code in the shared PIL/Dine implementation:
instruction semantics, native forms, CPU and memory state handling, block
dispatch, flag formulas and lazy-flag materialization, plus BIOS, DOS, FPU
and VGA services. A launcher contains only its loader and guest bytes.
There is no per-program engine, copied runtime, or guest-specific build of
PIL or Dine. Run-time decoded instructions and block plans are per-machine
data processed by that same shared implementation.

Native lazy-flag optimization must also live in the shared executor and
emitter. Guest-specific files must not contain copies of flag helpers or
special rules for one application's flags. Preserve instruction budgets,
asynchronous interrupt boundaries, code-modification guards and guest
exception delivery when changing the shared executor.

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
image fidelity. The Windows workflow has been checked with a real release
runtime and native launcher, including factorial output, running after the
original DOS file is removed, long-name file access, a missing runtime, and
`--runtime --run` returning guest exit status 7 from an input folder with
spaces, and a build followed by `--run` reusing the existing output.
Build-helper tests cover batch runtime reuse, command-line handling,
matching-build reuse and launcher/system separation. Full-window presentation and
performance require separate validation; these workflow checks do not
establish graphics parity or compatibility with arbitrary DOS applications.

## Native COM entry ABI

The shared library exports `dine_run_compiled_v1` for a caller that already
has native code for a real-mode COM program. The caller supplies the native
entry point and one contiguous initialized data span. Dine reserves the COM
image in the process allocation starting at `PSP:0100h`. The data offset is
relative to that address and the rest of the reserved image starts zeroed.
The callback starts with standard COM registers, the DOS stack, and the
prepared command tail. The input filename is a DOS process name.

```c
#include <stddef.h>
#include <stdint.h>

typedef struct {
    uint32_t gpr[8];      /* AX, CX, DX, BX, SP, BP, SI, DI, including upper halves */
    uint32_t eflags;
    uint16_t ip;          /* Guest address of the current operation */
    uint16_t segments[6]; /* ES, CS, SS, DS, FS, GS */
} DineCompiledRegistersV1;

typedef struct {
    int32_t (*interrupt)(void *, DineCompiledRegistersV1 *, uint8_t vector);
    int32_t (*read)(void *, uint32_t linear, uint8_t bytes, uint32_t *value);
    int32_t (*write)(void *, uint32_t linear, uint8_t bytes, uint32_t value);
    int32_t (*port)(void *, uint16_t port, uint8_t bytes, uint8_t input, uint32_t *value);
} DineCompiledServicesV1;

typedef void (*DineCompiledEntryV1)(
    void *context, DineCompiledRegistersV1 *state, const DineCompiledServicesV1 *services);

int32_t dine_run_compiled_v1(
    const uint8_t *data, size_t data_len, size_t image_len, size_t data_offset,
    const char *name, const char *tail, DineCompiledEntryV1 entry);
```

The callback runs synchronously. It must stop after a service returns 1
(DOS exit) or -1 (runtime error). A service returns 0 after a normal call,
including a DOS error reported through guest carry and AX. The entry point
returns 0–255 on DOS exit and -1 on runtime error. Memory access widths
are 1, 2, or 4 bytes. Port direction is 1 for input and 0 for output.
Use the context and service table only on the callback thread and only
during the callback.

This first ABI supports synchronous DOS and BIOS calls, guest memory and
port I/O, and redirected text output. Compiled execution that requires
blocking input, guest interrupt handlers, child execution, or TSR
termination reports a runtime error. Graphics presentation, asynchronous
interrupt delivery, self-modifying guest code, and COM translation
are separate work.
