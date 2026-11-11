# Native DOS program compilation

This page describes **standalone, compiled DOS programs**. Block compilation is
the active compiler milestone on `dev`. The older profile-guided Dine shadow
engine is **deferred** and is not required for these steps.

A standalone binary is a host executable produced from the original DOS
`.COM` or MZ/`ZM` `.EXE` using PIL's real-mode decoder, validated
reachable instruction graph, shared PIL semantics and Rust code generation.
The program's blocks run directly as compiled Rust. **There is no x86
instruction interpreter or decoder in the generated executable.**

## Status and known test programs

The compiler and block planner are in `src/dos_translate.rs`,
`src/dos_blocks.rs`, `src/dos_codegen.rs` and
`src/dos_standalone.rs`. The block planner partitions reachable sites into
primary blocks (at most 16 instructions per block), retains a single-instruction
entry for interior addresses, and emits guarded guest semantics. This is already
present in the `dev` source tree; check the current remote commit and run the
tests below before calling any local outstanding optimizer change published.

| Program | Scope of native validation | Suggested check |
| --- | --- | --- |
| `programs/dos/FACTOR.COM` | DOS factorial output (0! through 99!), scalar LODS, DIV/IDIV, DOS command tail | `python tools/verify_factor_native.py --engine target/release/pil.exe` |
| `programs/dos/RECURFIB.COM` | recursive near calls/returns, runtime stack integrity, arguments 0..20 | `python tools/verify_recursive_native.py --engine target/release/pil.exe` |
| `programs/dos/MANDANSI.COM` | ANSI Mandelbrot output/blocks, compared against independent reference | `python tools/verify_dos_standalone.py --engine target/release/pil.exe` and `tools/audit_dos_codegen.py` |
| generated DOS MZ hello and argument example | MZ/ZM loading, relocations, entry CS:IP, Unicode/CP437 command-tail boundary | `python tools/verify_mz_native.py --engine target/release/pil.exe` |
| small COM control-flow and memory fixtures | Jcc/loop edges, RAM load/store/RMW, stack, division and precision faults | `python tools/verify_dos_standalone.py --engine target/release/pil.exe` |

These are the existing project regression targets and development claims.
A successful build depends on the **reachable instructions and services**, not
on filename or extension alone. Local timing and test numbers in
[`TODO.md`](../TODO.md) are historical observations, not fresh CI evidence.

## One-command native build (Windows PowerShell)

From the repository root, with Rust and Python available:

```powershell
git switch dev
git pull origin dev
cargo test --locked
python tools/build_dos_native.py programs/dos/FACTOR.COM
.\programs\compiled\FACTOR.COM.exe 10
python tools/build_dos_native.py programs/dos/RECURFIB.COM
.\programs\compiled\RECURFIB.COM.exe 12
```

The helper builds the PIL emitter if `--engine` is omitted. The output
directory defaults to `programs/compiled/`, with three files per guest:

- `FACTOR.COM.exe`: Windows host-native executable
- `FACTOR.COM.exe.rs`: inspectable self-contained Rust source
- `FACTOR.COM.exe.json`: source/guest/engine/compiler identity and SHA-256 values

Use `--output-dir PATH` to keep builds apart, `--engine PATH` to select
a known emitter, or `--force` to deliberately replace same-named artifacts.
For example:

```powershell
python tools/build_dos_native.py driveD/FACTOR.COM --engine target/release/pil.exe --output-dir target/native-factor
.\target\native-factor\FACTOR.COM.exe 99
```

Use the actual path containing the original guest file; the default examples
assume the tracked `programs/dos/` fixtures exist. No copied or modified
guest file is needed to *run* a compiled standalone executable.

On Linux/Termux:

```sh
cargo test --locked
python3 tools/build_dos_native.py programs/dos/FACTOR.COM
./programs/compiled/FACTOR.COM.native 10
```

This builds **for the current host**, not for DOS and not for another host
architecture. Build separately on Windows, Linux, or Android/Termux.

## Direct compiler invocation

The helper wraps these two essential stages (Windows example):

```powershell
cargo build --release --locked
.\target\release\pil.exe --emit-dos-standalone programs/dos/FACTOR.COM target/factor-native.rs
rustc --edition=2021 -O -Ccodegen-units=1 target/factor-native.rs -o target/factor-native.exe
.\target\factor-native.exe 10
```

The `--emit-dos-standalone` command refuses to overwrite its output. Delete
or choose a new generated-source path before rerunning it. The one-command
helper creates new temporary paths and publishes after a successful compile.

The generated application maps host command-line arguments to the DOS PSP
command tail. Output is UTF-8 by default. Set `PIL_DOS_RAW_OUTPUT=1` if a
CP437-aware terminal/transport needs raw bytes.

## What the standalone runtime currently supports

- DOS COM and DOS MZ/ZM EXE loading, including relocations and initial
  segment/stack setup; it does not mean every `.EXE` is compilable.
- Statically proven reachable direct branches, loops, near calls/returns
  (including bounded recursive discovery), eligible arithmetic/PIL operations,
  ordinary RAM reads/stores, register/immediate stack operations, scalar
  LODS and checked division. Guest code bytes are protected against writes.
- A minimal **headless** DOS service boundary: `INT 20h` termination,
  `INT 21h/AH=02h` character output, `AH=09h` dollar-terminated string
  output, and `AH=4Ch` process exit.
- Command-tail translation through CP437 and output transcoding to UTF-8.

There is **no file API, keyboard input, graphics screen, EXEC child process,
installed interrupt handler, REP string support or arbitrary indirect
control-flow discovery** in standalone mode today. Unsupported reachable
code is rejected during translation; generated executables cannot fall back
to Dine. Standalone services and Dine services must not be conflated.

### Running from a DOS drive is a different milestone

Building `FACTOR.COM.exe` does **not** automatically replace
`D:\FACTOR.COM` when FreeCOM executes it. The standalone binary also does
not consume `pildos.conf`, DOS drive mounts or Dine file services. The
existing DOS drive continues to run the original program through Dine.

The desired next integration is **native executable lookup and DOS-service
reuse**, not another shadow-code profiler:

1. Identify the DOS guest by its actual bytes/hash and resolved DOS path.
   Keep explicit target host architecture/OS metadata and separate artifacts
   per guest identity. Never choose a compiled binary merely by stem.
2. At DOS `EXEC`, check for a compatible native artifact and an **explicit
   service/ABI capability contract**. When incompatible, run the original
   guest under Dine. Preserve command tails, environment, drive mappings,
   process exit codes, and host-file isolation.
3. Extract/reuse Dine's input and file/handle/directory services instead
   of duplicating a second DOS implementation. Test partial I/O, EOF,
   permissions and errors against Dine, then validate `FACTOR`,
   `RECURFIB`, GREP and a real file-reading application.

Do not switch to compiled programs by default until the service bridge and
parent/child execution semantics have differential tests. Turbo Pascal,
FreeCOM, DOOM and GW-BASIC require substantially broader runtime support.

## Tests and performance

```powershell
cargo test --locked
python tools/verify_dos_standalone.py --engine target/release/pil.exe
python tools/verify_factor_native.py --engine target/release/pil.exe
python tools/verify_recursive_native.py --engine target/release/pil.exe
python tools/verify_mz_native.py --engine target/release/pil.exe
python tools/audit_dos_codegen.py --engine target/release/pil.exe --program programs/dos/FACTOR.COM --argument 99 --benchmark
```

For Linux/Termux, change `pil.exe` to `pil`. Measure original and generated
outputs **byte for byte** before comparing performance. Runtime benchmarks
must distinguish process startup, generated RAM initialization, terminal
output and guest instruction work. See [compiler plan](../TODO.md) for the
published experiment definitions and [DOS usage](USAGE.md) for ordinary Dine.

### Related but deferred work

`tools/build_dos_shadow.py` and `tools/build_dos_bundle.py` generate
profile-guided *linked Dine engines* with interpreter fallback. They remain
available for historical work but are **not** the active standalone compiler
workflow. Their artifacts are not equivalent to `FACTOR.COM.exe`.
