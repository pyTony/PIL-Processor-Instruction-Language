# PIL development branch

This is the full development tree. The [main branch](https://github.com/pyTony/pil_proof/tree/main)
is a separate runtime-only delivery: portable build/run source, embedded PIL
descriptions, scripts, the sample guest program and user documentation.

Development tests, Python oracles, CI, packaging/export tools, design notes,
revision/provenance logs and Claude handoff stay here on dev. Do not merge dev
into main. Export the user tree with tools/export_runtime.py and propose that
projection in a delivery PR based on main. The exporter strips dev-only test
module declarations and checks the user documentation links. CI verifies that
the export builds and runs by itself on Linux and Windows.

## Development build and debugger


In Termux or Linux, with Rust installed:

```sh
sh tools/build.sh
sh tools/debug.sh
```

On Windows PowerShell:

```powershell
./tools/build.ps1
./tools/debug.ps1
```

These build the debugger and write `target/programs/selftest.bin`, then load
that real byte program at CS:0100. In the debugger, type one command per line:

```
list
step
serial rx 41
run 1000
check
regs
serial tx
```

Success reports `PASS` and `DX=600D`. Guest serial output appears directly in
the console: `serial output: 41 | A`. `serial tx` also retrieves queued bytes.
The program checks arithmetic, flags, conditional branches, register views,
16/32-bit addresses, DS/SS/FS memory and serial input/output. It tests the
implemented execution path, not the complete 386 instruction set.

For an automatic check: `sh tools/debug.sh --check`, or on Windows
`./tools/debug.ps1 -Check`. `cargo test --locked` runs the full Rust suite.

## Debugger and deliveries

`help`, `list`, `step`, `run`, `regs`, `flags`, `mem`, `poke`, `break` and
`reset` inspect and control execution. `demo counter`, `demo echo` and
`demo memory` retain the earlier predecoded examples. `load PATH` loads another
raw binary within the [supported monitor profile](docs/DEBUGGER.md).

Native build archives include the executable, launchers, selftest binary,
listing and documentation. See [delivery instructions](delivery/README.md).
CI archives are Linux x86-64 and Windows x86-64; they do not run on Android or
ESP32. Termux builds produce the phone's own binary. Releases remain prototypes.

## Project branches and documentation

`dev` keeps the complete development tree. A runtime-only export is proposed
to `main` through a separate delivery branch/PR. Pull `main` after approving
that delivery PR; do not merge dev directly into main. [Development status](docs/development/STATUS.md) records the
verified baseline and current work separately.

- [Debugger guide and instruction profile](docs/DEBUGGER.md)
- [Language](docs/LANGUAGE.md)
- [References and credits](docs/REFERENCES_AND_CREDITS.md)
- [Revision history](docs/REVISION_HISTORY.md)
- [Development roadmap](docs/ROADMAP.md)
- [Claude continuation notes](CLAUDE.md)

Sources: `src/`, descriptions: `isa/i386/`, tests: `tests/unit/`, reproducible
guest fixtures: `programs/`, helpers: `tools/`. No third-party emulator source,
BIOS or OS image is bundled. A project-wide licence has not yet been selected.


## Hello, world!

In the debugger:

```
demo hello
run 1000
serial tx
```

The guest sends `Hello, world!` followed by a newline. The console displays
that newline as `\n`. No RX input is needed. This is a 57-byte real guest
program using MOV AL, OUT 0xe9 and HLT, with 28 completed steps. The build also
writes `target/programs/hello.bin`; use `load target/programs/hello.bin` to open
it, or consult `programs/hello.lst` for instruction addresses. `check` applies
to the diagnostic selftest; Hello-world can be inspected directly via serial tx.


## Numerical checks and loop timing

The factorial program uses only existing MOV/ADD/SUB/CMP/Jcc instructions.
It multiplies by repeated addition and stores all results 1!..12! as little-endian
dwords at linear RAM 0x500..0x52f. 12! is 479001600; 13! exceeds unsigned
32-bit range, so this example deliberately stops at 12. It executes 336 guest
instructions and halts at EIP=0x146.

```
demo factorials
time 1000
check
regs
mem 0x500 48
stress 100
```

`check` verifies all twelve RAM results, final registers and arithmetic flags.
`stress N` runs N fresh copies, checking every table; default 25, range 1..1000.
It leaves your current debugger state alone and reports total elapsed host time,
including setup and verification. A changed result makes the check fail.

For a tight loop:

```
demo loop
time 250000
check
bench 1000 3
```

The default guest counts 65,536 iterations with ADD/SUB/JNE: 196,611 completed
instructions including setup; halt at EIP=0x11a. `time N` runs the current
program for up to N steps (default 10,000; maximum 1,000,000), respects
breakpoints/waits/faults and reports elapsed milliseconds and instructions/s.
Use more `time` or `run` commands to continue a budget-limited run. HLT is a
monitor stop and is excluded from completed-instruction counts.

`bench ITERATIONS SAMPLES` runs a fresh counted-loop image per sample,
checks its answer, excludes one warmup and reports sample times, median and
instructions/s. Defaults are 1000 iterations and 3 samples; allowed ranges are
1..100000 and 1..9. Your loaded debugger program is preserved. Timing includes
fetch validation, cache lookup, decoding/compilation on misses and token interpretation. Image setup,
result checking and console output are outside the benchmark sample interval.
This is current interpreter throughput, not simulated 386 cycles or CPU MHz.

For representative timings, use an optimized build. From a source checkout:

```sh
sh tools/bench.sh 10000 5
# equivalent: cargo run --release --locked -- --bench 10000 5
```

Windows: `./tools/bench.ps1 -Iterations 10000 -Samples 5`.
The report identifies build profile, version and host OS/architecture. Compare
runs on the same host/profile; CPU scheduling and phone power state affect time.
`pil-proof --numeric-test` runs small automatic correctness checks.
`pil-proof --stress 100` runs the factorial stress test from the shell.

The build also writes factorials.bin and loop.bin in target/programs. Their
instruction listings are in programs/. Native archives include those binaries
under programs/ and can run the same debugger commands and --bench/--stress
options directly, without compiling.

A 256×256 Mandelbrot renderer is a later workload: signed fixed-point multiply,
shifts, framebuffer storage and image output are not implemented by this monitor.


## Interpreter cache and timing comparisons

Execution uses the PIL token interpreter. The x86 decoder produces typed PIL
programs; the interpreter executes their tokens. `--emit-rust` generates Rust
source as a separate facility. The debugger does not execute generated native
code or a JIT.

The binary monitor now caches up to 256 decoded instructions and their compiled
PIL programs. Cache entries contain no saved register values or arithmetic
results. Addresses, flags, RAM protections and side effects are evaluated on
every execution. Every hit re-fetches/checks all encoded bytes; changed bytes,
image extent or segment context cause re-decoding. Debugger poke and guest
stores to code therefore take effect when that instruction executes again.
Errors are not cached. At capacity, the cache is cleared before inserting a new
entry. Reset/load/toggling clears entries and counters; the selected on/off mode
persists across reset and program loads. Predecoded demos do not use this cache.

```
cache stats
cache off
bench 10000 5
cache on
bench 10000 5
```

`cache clear` empties entries and zeroes counters. `cache stats` shows hits,
misses, byte/context invalidations and capacity evictions for the loaded image.
Bench uses a separate fresh debugger and the selected cache mode; its own final
sample counters appear in the benchmark report. Each sample starts with an
empty cache, so first-visit decoding/compilation is inside the timing interval.
The one excluded warmup is a separate run, not cache preloading for samples.

For a release-build comparison from a source checkout:

```sh
sh tools/bench.sh 10000 5 --uncached
sh tools/bench.sh 10000 5
```

Windows: add `-Uncached` to `tools/bench.ps1` for the uncached measurement.
Native binaries accept `--bench 10000 5 --uncached` or `--bench 10000 5`.
Both modes still interpret PIL. Cached timing includes byte validation, lookup,
cache-miss decoding/compilation and token dispatch; uncached mode decodes and
compiles on every visit. Compare the same iteration/sample counts, host and
build profile. These are host wall-time results, not 386 clock cycles.


## Alpha: compiled Rust backend proof

See [feature matrix](delivery/source/FEATURE_MATRIX.md) for the supported, partial and planned
areas in 0.10.0-alpha.1. This is an alpha of the host execution proof, not an OS
boot release or ESP32 firmware.

From a source checkout, build in release mode and compare the engines:

```sh
sh tools/compare.sh 10000 5
```

Windows PowerShell: `./tools/compare.ps1 -Iterations 10000 -Samples 5`.
The command emits Rust from the validated PIL of fixed guest images, compiles
it with rustc -O, and runs it as a separate executable. Rustc must be installed.
The ordinary debugger remains the cached PIL interpreter.

Before reporting speed, it compares complete instruction-by-instruction traces:
registers, EFLAGS, undefined flags, EIP, RAM contents and serial RX/TX queues.
Nine cases cover the diagnostic selftest, Hello-world, factorials, a short loop,
read-only store and branch-limit faults, a changed-code stop, empty RX and full
TX. Stop kinds/metadata are compared; descriptive strings are not trace fields.
Any mismatch fails the command rather than reporting a passing benchmark.

Both engines then time the same counted-loop image with the same iteration
count and sample count, fresh state per sample and one excluded warmup. The
cached interpreter samples start cold. Generated code includes fixed-image byte
guards, native dispatch and black_box per step to keep rustc from folding away
the known loop. Generation/compilation/process startup, trace recording, result
checking and output are outside native sample intervals. Compilation time is
reported separately. Do not compare a debug interpreter to release native code;
the compare helpers build the interpreter in release mode.

Artifacts stay under target/backend-comparison or CARGO_TARGET_DIR: emitted
Rust, native executable and the two matching binary traces. `--emit-suite-rust
10000 5` emits the standalone suite source without compiling. Compiled execution
is currently for fixed guarded images; changes to their code stop that runner.
It is not a JIT, a general binary translator, or a selectable native REPL engine.


## Verifier and direct PC baseline

`pil-proof --verify-backends` runs the nine-case compiled-Rust/interpreter
trace verifier without timing samples. `--compare-backends` must pass that
verifier before it starts either backend's measured comparison. A mismatch or
compiler error returns a nonzero exit status.

On an x86-64 PC the comparison also reports `PC_NATIVE`: an ADD/SUB/JNZ loop
executed directly by the processor in 64-bit mode, using 32-bit registers. It
checks the accumulator and final count. Run it alone with `--cpu-bench 10000 5`.
This excludes the decoder, PIL, code guards and virtual RAM; function-call and
timer overhead remain included. It is an arithmetic microbenchmark, not native
execution of the 16-bit monitor image or an original-386 cycle estimate.
On non-x86-64 hosts the x86-specific baseline is explicitly unavailable; the
compiled Rust backend still builds for the local host.
