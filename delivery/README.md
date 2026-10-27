# PIL debugger delivery
## Documentation

[Complete documentation overview](https://github.com/pyTony/pil_proof/blob/dev/docs/README.md)
provides the map to all language, design, verification and planning documents.

| Read this | What it covers |
|---|---|
| [PIL language reference](PIL_REFERENCE.md) | Executable syntax, types, effects and Rust emission |
| [Debugger guide](DEBUGGER.md) | Start the debugger, commands, demos, memory and serial; supported encodings |
| [Feature matrix](FEATURE_MATRIX.md) | Implemented, partial and planned emulator features |
| [Release notes](RELEASE_NOTES.md) | Changes in this delivery and earlier versions |
| [References and credits](REFERENCES_AND_CREDITS.md) | Reference manuals, attribution and third-party material status |


## Built-in demos

| Command | Purpose |
|---|---|
| `demo selftest` | Binary diagnostic; queue `serial rx 41`, then `run 1000` and `check` |
| `demo hello` | Binary Hello-world output through mock serial |
| `demo factorials` | Binary 1!..12! table; `time 1000` then `check`; 12!=479001600 |
| `demo extensions` | Conversion/address instructions, same-width moves and flags; `run 1000` then `check` |
| `demo mandelbrot` | Small 8x8 fixed-point image; `time 1000000` then `check`; serial pixel bytes |
| `demo loop` | Binary 65,536-iteration counted loop; `time 250000` then `check` |
| `demo counter` | Small predecoded subtract-and-branch example |
| `demo echo` | Predecoded one-byte serial echo; waits for RX input |
| `demo memory` | Predecoded RAM store/load and protection example |

Unpack this whole directory. No Rust or Python is needed for these native
binaries. Run the binary built for your operating system and CPU.

Linux: `sh debug.sh`
Windows PowerShell: `./debug.ps1`

The launcher opens programs/selftest.bin in the debugger. Type:

```
list
step
serial rx 41
run 1000
check
regs
serial tx
```

Success: `PASS`, `DX=600D`, RAM signatures and transmitted byte `41` (`A`).
Guest output also appears automatically as `serial output: 41 | A`.
`serial tx` drains the transport queue; `check` remains repeatable.

`reset` restarts the program and clears RAM changes and serial input/output.
`break 0x100`, `step`, `run`, `regs`, `flags`, `mem 0x500 8`, and `list` help
inspect execution. See DEBUGGER.md for commands and the monitor profile.

For an automatic check, run `./pil-proof --selftest` (Linux) or
`./pil-proof.exe --selftest` (Windows). Exit status zero means pass.

This is an early 386 monitor, not a bootable PC emulator. Only the documented
instruction subset is decoded. The IN/OUT debug port 0xe9 connects to a mock
serial queue; HLT is a monitor stop without interrupt or privilege semantics.
VGA, PS/2, SD, WLAN, BIOS, paging and OS loading are not present yet.

Built binaries target their build host. The Linux CI build is x86-64, not
Android/Termux or ESP32. Build from source in Termux for the phone.


## Hello, world!

In the debugger:

```
demo hello
run 1000
serial tx
```

The guest sends `Hello, world!` followed by a newline. The console displays
that newline as `\n`. No RX input is needed. This is a 57-byte real guest
program using MOV AL, OUT 0xe9 and HLT, with 28 completed steps. The archive
includes `programs/hello.bin`; use `load programs/hello.bin` to open it, or
consult `programs/hello.lst` for instruction addresses. `check` applies
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

`run` without a budget executes at most 100 instructions. Factorials needs 336
from a fresh start; `budget exhausted` means it paused. Use `run 1000` to finish
from the current position, then `check`. `list` defaults to 16 instructions; `list 64` shows the full factorial program.


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

Source builds write factorials.bin and loop.bin in target/programs. Their
instruction listings are in programs/. Native archives include those binaries
under programs/ and can run the same debugger commands and --bench/--stress
options directly, without compiling.

The Mandelbrot guest uses signed fixed-point multiply and shifts, streams
pixels through serial, and saves a 256×256 image on the host. It needs no guest
framebuffer or VGA emulation. See the image workload section below.


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

See [feature matrix](FEATURE_MATRIX.md) for the supported, partial and planned
areas in 0.10.0-alpha.1. This is an alpha of the host execution proof, not an OS
boot release or ESP32 firmware.

Native delivery: run `./pil-proof --compare-backends 10000 5` on Linux or
`./pil-proof.exe --compare-backends 10000 5` on Windows.
The command emits Rust from the validated PIL of fixed guest images, compiles
it with rustc -O, and runs it as a separate executable. Rustc must be installed.
The ordinary debugger remains the cached PIL interpreter.

Before reporting speed, it compares complete instruction-by-instruction traces:
registers, EFLAGS, undefined flags, EIP, RAM contents and serial RX/TX queues.
Twenty-six cases cover the diagnostic selftest, Hello-world, factorials, a short loop,
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

`pil-proof --verify-backends` runs the 39-case compiled-Rust/interpreter
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


## Typed memory dumps

```text
mem8 0x500 16
mem16 0x500 8
mem32 0x500 4
mem32 0x500 12 dec
```

`mem8`, `mem16` and `mem32` display unsigned 8-,16- and 32-bit values with an
ASCII column. Counts are numbers of values, not bytes: the first three examples
each inspect 16 bytes. `hex` is the default; optional `dec` displays unsigned
decimal values. After `demo factorials`, `run 1000`, `check`, the last command
shows the 12 results as 1,2,6,...,479001600.

Values use little-endian byte order and unaligned addresses are allowed. The
ASCII column follows the underlying bytes in address order: space through `~`
are displayed, with `.` for control bytes, DEL and bytes above 0x7e. Nothing is
sent as terminal control codes. Rows contain at most 16 bytes, with padding for
a partial final row. Addresses are linear monitor RAM addresses.

Omitting the count displays 16 bytes (16/8/4 values). Counts must be 1..256
values; the entire byte range must fit RAM. Invalid requests produce an error
without partial output or changes to CPU, RAM, serial queues or cache state.
The existing `mem ADDRESS [BYTE_COUNT]` command keeps its previous output.


## Instruction listings

```text
demo factorials
list 64
list 0x100 64
```

Binary listings show the code offset, encoded bytes and decoded operands. `*`
marks the current EIP. Register names follow operand width (AL/AH, AX or EAX).
Memory operands include size, segment and effective-address expression; branch
operands show the computed target. For example, the factorial loop includes
`ADD EAX,EDX`, `SUB ECX,EBX`, `JNE 0x00000130` and
`MOV dword DS:[DI],EAX`.

`list` defaults to 16 instructions from current EIP. `list COUNT` accepts 1..256;
`list ADDRESS COUNT` starts at the specified code offset, useful after execution.
The full factorial program has 18 instructions, including HLT. Listing stops at
HLT, the end of the loaded image or a decode error. It reads current bytes, so
edits made with `poke` are reflected. It does not execute or change CPU/RAM/cache.
Choose instruction boundaries when supplying an address. Predecoded demos show
harness labels, without invented instruction bytes.

Factorials uses repeated addition: EDX holds the previous factorial, ECX counts
the additions for the current n in ESI, EAX accumulates the result, and DI selects
the output slot starting at linear RAM 0x500. This demo intentionally retains
its original ADD/SUB algorithm even though IMUL is now available.


## Mandelbrot image workload

Render the original 256x256 fixed-point guest using the packaged binary:

```powershell
.\pil-proof.exe --mandelbrot 256 32
Invoke-Item .\target\mandelbrot\256x256-i32\mandelbrot.bmp
```

Linux: `./pil-proof --mandelbrot 256 32`. Rendering needs no Rust compiler.
The binary accepts `--mandelbrot SIZE ITERATIONS`. Use powers of two 2..256 and 1..64
iterations; default 256/32. Smaller example: `--mandelbrot 64 16`. Output stays
under CARGO_TARGET_DIR (default target)/mandelbrot/SIZExSIZE-iITERATIONS/:
mandelbrot.bmp, mandelbrot.pgm, guest.bin and report.txt. Custom output roots
require adjusting the image path in the open command above.

The actual guest executes IMUL, SAR/SHL, arithmetic and relative branches. It
streams one byte per pixel through mock port 0xe9; the host drains full TX queues
and checks every byte against a separate mathematical reference before saving
images. No guest framebuffer larger than the existing 4-KiB RAM is needed. BMP
is 24-bit uncompressed grayscale (interior black, escaped values255-value
for visibility); PGM stores raw escape-iteration values as an
intensity visualization, not calibrated luminance.

Coordinates are Q12 over [-2,1) x [-1.5,1.5), starting z=0. Each iteration checks
x*x+y*y>4 in Q24, then computes the next x/y with arithmetic shifts (floor
rounding). Escaped pixels use 4*iteration; points not escaping within the limit
are 0. Values are bounded to fit the guest's signed 32-bit products.

Timing measures the cached token interpreter including serial-drain handling;
it excludes setup, reference checking and file output. It is not a 386 cycle
rate. Normal rendering is interpreted; --compare-mandelbrot also executes the
full image as generated Rust. The 39-case verifier includes a tiny image traced
after every instruction; the full-image comparison checks state at serial-drain
boundaries and halt. No VGA, firmware or OS boot is implied.

Small interactive example (64 output bytes, no queue draining needed):

```text
demo mandelbrot
list 64
time 1000000
check
serial tx
```

This demo is 8x8 at 16 iterations. Manual `load` of a larger generated guest can
pause when TX is full; use `serial tx` to drain then continue, or use the image
helper to handle draining automatically.

## Full-image generated Rust comparison

With rustc installed, run `./pil-proof --compare-mandelbrot 256 32`
(Windows: `.\pil-proof.exe --compare-mandelbrot 256 32`). The command first gates
with the 39-case per-instruction verifier, then checks every image pixel against
the mathematical reference and full state at each serial drain/halt boundary.
Instruction counts and TX retry counts must also agree. Artifacts go under
target/backend-comparison (or CARGO_TARGET_DIR), including both pixel streams
and state checkpoints. Comparison does not write a BMP; use --mandelbrot to
render the image. Each engine has one cold render timed, including checkpoint
capture and serial draining, excluding setup, compilation, child process startup,
reference checking and files. This is not a median benchmark or 386 cycles.

## Extending moves, accumulator conversion and LEA

The binary monitor supports MOVZX and MOVSX with byte sources and16/32-bit
register destinations (0F B6/BE), and word sources with16/32-bit destinations
(0F B7/BF). Sources can be registers, including AH/CH/DH/BH, or memory.
Memory reads check only the source width; read-only memory is permitted.
The16-to16 forms copy the word unchanged and preserve the upper register half.

Opcode98 executes CBW (AL to AX) or, with66, CWDE (AX to EAX). Opcode99
executes CWD (AX sign into DX) or, with66, CDQ (EAX sign into EDX). CWD/CDQ
leave EAX untouched. Word destinations preserve their upper register halves.
All of these instructions preserve flags and undefined-flag markers.

LEA8D calculates a memory-form effective offset into a16/32-bit register.
Operand size and address size are independent:16-bit addresses wrap then zero
extend for32-bit destinations;32-bit offsets truncate for16-bit destinations.
It performs no data load, adds no segment base and checks no data-segment
access/limit. Segment overrides do not change the result. Instruction fetch
still uses the normal CS checks. A register-source LEA is a monitor diagnostic.
Disassembly shows an address expression without a data size or segment.

Standalone descriptions are named movzx8/movsx8 (byte source) and
movzx16/movsx16 (word source); movzx/movsx alias the byte forms. Destination
width comes from --emit-rust WIDTH. lea and cwd use16/32-bit destination width;
cdq aliases cwd. The binary decoder binds98/99 implicit registers before PIL
validation. The interpreter and generated runner execute the same tokens.

## Extension instruction demo

```text
demo extensions
list 64
run 1000
check
regs
mem16 0x180 1 hex
```

This original15-instruction guest exercises MOVZX/MOVSX, CBW/CWDE/CDQ, LEA,
16-to16 register copies (including DI self-aliases), and word memory sources.
Expected final values: EAX=FFFFFF80, EBX=00000080, ECX=FFFFFF80,
EDX=FFFFFFFF, ESI=00000085, DI=BP=SP=FF80 and RAM word0x180=FF80.
The checker verifies every register, that RAM word, preserved flags and halt.

The decoder now accepts word-source MOVSX/MOVZX with16-bit as well as32-bit
destinations. At16-to16 both operations copy the word unchanged; the destination
register's upper16 bits and flags/undefined markers are preserved. Memory
sources still perform a real word read with normal segment/RAM checks, including
read-only sources. A same-register instruction is not skipped, so fetch and
memory checks retain their normal behavior. In16-bit operand size,0F BF F8
selects DI,AX;0F BF FF selects DI,DI.
