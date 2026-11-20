# Prepared native applications

The application compiler prepares native CPU calls and completes initial DOS
relocation ahead of time. Its output imports the shared Dine library. The
compiler and library are the same for every program; generated applications
contain no CPU, flag, DOS, BIOS or device implementation.

```powershell
python tools/compile_application.py path/to/application.exe
```

The default output is `programs/compiled/Native_application.exe` on Windows
(no `.exe` suffix on Unix), plus the shared Dine library and a `.native.json`
description. Keep the library beside the native executable. Multiple native
applications in the same output folder use that one library. Rust and Cargo
are needed to build; the original DOS executable is not an external dependency
when running the result.

For a local compiler/runtime build, pass `--engine` and `--runtime`. Use
`--output-dir` for a separate destination. The helper refuses existing native
application outputs and a destination containing a different shared runtime.
Compiler intermediates live under `target/native-applications/` and are not
runtime dependencies. Paths in generated Rust are relative to those files.

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

The local Pascal IDE acceptance test takes user-supplied binaries and libraries:

```powershell
python tools/verify_turbo_pascal.py --tp-bin path/to/compiler/bin --native-executable path/to/Native_TURBO.exe
```

It creates a fresh folder containing support libraries and an original Pascal
source, with no original IDE executable. It edits and saves that source, performs
two compile/run/return cycles, checks `5050`, and requires a clean exit. Evidence
and scratch files remain under `target/pascal-acceptance/`. Third-party programs
and resources are not included in the repository or CI.
