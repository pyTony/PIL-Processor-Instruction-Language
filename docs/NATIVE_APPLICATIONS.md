# Prepared native applications

The application compiler prepares native CPU calls and completes initial DOS
relocation ahead of time. Its output imports the shared Dine library. The
compiler and library are the same for every program; generated applications
contain no CPU, flag, DOS, BIOS or device implementation.

```powershell
python tools/compile_application.py path/to/application.exe
```

The default output is `programs/compiled/Native_application.exe` on Windows
(no `.exe` suffix on Unix) plus a `.native.json` description pointing to the
shared Dine runtime directory (`shared_runtime_dir`), so the shared library does
not need to be copied into every output folder. When `dine.latest.json` is
present in that directory, the launcher loads the newest build recorded there
without rebuilding or copying the application. Pass `--copy-runtime` if you want
a copy of the library placed beside the native executable for standalone use, or
`--force` to replace an existing native output. Rust and Cargo are needed to
build; the original DOS executable is not an external dependency when running
the result.

## Where the program finds its files

A native application (and a `Dine_<program>` launcher) sees its files as it
would when DOS runs it from its own folder:

- The folder you start it from is the current drive and directory. If a
  configured host drive (such as `D=driveD`) contains that folder, the
  program starts there, cd'ed into it; otherwise the folder goes on a free
  letter (E: down to A:) and every configured drive stays.
- The program's own path in its environment, which DOS 3+ programs read to
  find support files beside themselves, names **the native executable's
  folder**: through a configured drive that contains it (for example
  `D:\TP\BIN\TURBO.EXE`), else that folder on a free letter (E: down to A:).

So keep a native application in the same folder as the support files its
original expects beside itself. For Turbo Pascal, compile into the folder
that holds TURBO.TPL, TURBO.TPH and TPC.CFG:

```powershell
python tools/compile_application.py driveD/TP/BIN/TPC.EXE --output-dir driveD/TP/BIN
```

`Native_TPC.exe` then finds TURBO.TPL from any working folder, and the files
you name on its command line are taken from the folder you start it in.

Native applications use the shared DOS configuration: `PILDOS_CONFIG` takes
precedence, followed by `pildos.conf` in the launch directory. If neither is
present there, the native launcher searches its executable's parent folders
for `pildos.conf`, so an application in a nested output directory can still
find the checkout configuration. On Windows the native application uses the
DOS window frontend; `mouse=true` enables the guest mouse driver and host
pointer input. With `--copy-runtime`, keep the copied library beside the
executable when rebuilding the shared runtime; otherwise `.native.json`
resolves the shared runtime directory (and `dine.latest.json`) automatically,
and `DINE_RUNTIME` overrides the library or runtime directory when needed.

For a local compiler/runtime build, pass `--engine` and `--runtime`. Use
`--output-dir` for a separate destination and `--force` to overwrite an
existing native output in that destination. Compiler intermediates live under
`target/native-applications/` and are not runtime dependencies. Paths in
generated Rust and `.native.json` stay relative when possible.

## What executes

The compiler enumerates every possible byte entry in the relocated initial
image and the executable resource, including unvisited functions and indirect
targets. Available native forms become calls with constant arguments and saved
bindings/address metadata. A compact native jump table selects those calls.
There is no runtime x86 decoding or PIL-token interpretation on that path.
Control flow, CPU effects and instruction retirement use shared native forms;
DOS/BIOS, devices, interrupts and presentation remain shared Dine services.

The prepared startup has zero relocation entries. The usual shared DOS
initialization still creates the PSP, environment, memory arena, devices and
configured drives for the current launch. Initial CS:IP and SS:SP and the
already relocated memory image come from compilation.

The complete original input is retained **as a read-only executable resource**
for DOS self-reads and overlays; relocated initial memory is retained as data
and for execution guards. This is not a package without original instruction
bytes. Keeping these bytes preserves file-read behavior and permits the user-
authorized reference path for changed code. Runtime execution does not require
the original input file on disk.

When DOS reads an executable resource into RAM, shared memory origin hints let
unchanged loaded instructions use their ahead-of-time calls at the new address.
Hints do not authorize execution: the bytes and current execution context must
still match. Changed instructions, generated code, unavailable native forms and
unsupported compilation contexts use the shared interpreter separately. No
observed trace is treated as a complete inventory of the original program.

This version prepares 16-bit-default real-mode code with 16-bit stacks. Other
execution contexts retain the reference path. It does not replace Pascal's
DOS-target compiler with a host-target Pascal compiler: generated DOS programs
can themselves be passed to `compile_application.py`.

## Verification and diagnostics

`tools/verify_native_application.py --engine PATH --runtime PATH` builds original
COM/MZ fixtures, deletes their temporary source executable files, then runs the
native outputs against Dine. It verifies expected output and exit status, zero
interpreted instructions for these fixtures, and command-tail behavior. The
Rust tests compare CPU, segment, memory and output state after small slices,
including initial relocation, calls, REP progress and file-loaded code.

`DINE_PROTOCOL=1` selects Dine's existing JSON session protocol for diagnostics
without consuming an original application's command-line option. Snapshots
report native and interpreted retirements separately. Successfully compiling
an application does not assert complete native coverage or prove equivalence
of every function of an arbitrary closed binary.

`tools/bench_native_application.py` times a native application against its
original on Dine: both run the same command line in a fresh folder with the
same files, through the same session protocol, to the program's exit. Start
up, window and key waits are left out on both sides. It reports the median
run-loop time, guest instructions, and how many retired natively.

```powershell
python tools/bench_native_application.py --original path/to/TPC.EXE --native path/to/Native_TPC.exe --support path/to/TURBO.TPL --source BIG.PAS --args BIG.PAS --runs 5
```

The local Pascal IDE acceptance test takes user-supplied binaries and libraries:

```powershell
python tools/verify_turbo_pascal.py --tp-bin path/to/compiler/bin --native-executable path/to/Native_TURBO.exe
```

It creates a fresh folder containing support libraries and an original Pascal
source, with no original IDE executable. It edits and saves that source, performs
two compile/run/return cycles, checks `5050`, and requires a clean exit. Evidence
and scratch files remain under `target/pascal-acceptance/`. Third-party programs
and resources are not included in the repository or CI.
