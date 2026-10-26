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
decoding, PIL description compilation and interpreter dispatch. Image setup,
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
