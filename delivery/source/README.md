# PIL 386 debugger

This is a runtime-only delivery. Automated tests and CI live on the
[dev branch](https://github.com/pyTony/pil_proof/tree/dev). `cargo test` here
intentionally runs zero tests; see [automated verification](#automated-verification).

## Documentation

[Complete documentation overview](https://github.com/pyTony/pil_proof/blob/dev/docs/README.md)
provides the map to all language, design, verification and planning documents.

| Read this | What it covers |
|---|---|
| [PIL language reference](docs/PIL_REFERENCE.md) | Executable syntax, types, effects and Rust emission |
| [DOS COM/EXE probe](docs/DOS_PROBE.md) | Load original DOS examples or your programs; report instruction/service blockers |
| [Debugger guide](docs/DEBUGGER.md) | Start the debugger, commands, demos, memory and serial; supported encodings |
| [Feature matrix](docs/FEATURE_MATRIX.md) | Implemented, partial and planned emulator features |
| [Release notes](docs/RELEASE_NOTES.md) | Changes in this delivery and earlier versions |
| [References and credits](docs/REFERENCES_AND_CREDITS.md) | Reference manuals, attribution and third-party material status |

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

## Start the debugger

After a release build, start it directly in PowerShell:

```powershell
.\target\release\pil-proof.exe --repl
```

If you built with `$env:CARGO_TARGET_DIR = "target/windows"`, use
`.\target\windows\release\pil-proof.exe --repl` instead. Linux/Termux:
`./target/release/pil-proof --repl`. For a debug build, replace `release` with
`debug`. These commands start the existing binary without rebuilding.

At the `pil>` prompt, enter one command per line:

```text
demo hello
run 1000
serial tx
demo factorials
time 1000
check
help
quit
```

`run` without a budget executes at most 100 instructions. Factorials needs 336
from a fresh start; `budget exhausted` means it paused. Use `run 1000` to finish
from the current position, then `check`. `list` defaults to 16 instructions; `list 64` shows the full factorial program.


Build and open the debugger together with `.\tools\debug.ps1` on Windows or
`sh tools/debug.sh` on Linux/Termux. These helpers load the diagnostic selftest;
enter `serial rx 41`, `run 1000`, then `check` to exercise it. Full command
reference: [Debugger guide](docs/DEBUGGER.md).


A Rust interpreter for processor instruction descriptions, with a small 386
binary monitor and interactive debugger. This prototype runs its included guest
program; it does not yet boot DOS or Windows.

## Build and run

Install Rust, then run these commands from this directory.

Termux / Linux:

```sh
sh tools/build.sh
sh tools/debug.sh
```

Windows PowerShell:

```powershell
./tools/build.ps1
./tools/debug.ps1
```

The build writes the executable and `target/programs/selftest.bin`. The debug
script loads that program at CS:0100. Type one command per line:

```
list
step
serial rx 41
run 1000
check
regs
serial tx
```

Expected: `PASS`, `DX=600D`, and guest output `serial output: 41 | A`.
`serial rx` supplies guest input; ordinary debugger commands are separate.
For an automatic check, run `sh tools/debug.sh --check` or on Windows
`./tools/debug.ps1 -Check`.

## Other uses

- `sh tools/build.sh --release` builds an optimized executable.
- `sh tools/debug.sh PATH` opens another raw binary within the supported profile.
- `cargo run --locked -- --repl` opens the built-in predecoded counter demo.
- `cargo run --locked -- --selftest` runs the included program automatically.
- `cargo run --locked -- --emit-rust 32 add_ram` emits a Rust module from an
  embedded instruction description. Operand widths are 8, 16 or 32.

[Debugger commands and supported instructions](docs/DEBUGGER.md) explain the
monitor environment and limitations. See [release notes](docs/RELEASE_NOTES.md)
and [references and credits](docs/REFERENCES_AND_CREDITS.md).
`DELIVERY.json` identifies the source revision; `SHA256SUMS` covers this tree.

This branch contains the portable build/run source, descriptions embedded in
the binary, helper scripts, sample program and user documentation. Development
is maintained on [dev](https://github.com/pyTony/pil_proof/tree/dev).

No external Cargo dependencies are required. Host builds use Rust's standard
library. A Termux build runs on the phone; Linux/Windows desktop executables
are not Android or ESP32 binaries. No BIOS, OS image or third-party emulator
source is bundled. A project-wide licence has not yet been selected.


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

See [feature matrix](docs/FEATURE_MATRIX.md) for the supported, partial and planned
areas in 0.26.0-alpha.1. This is an alpha of the host execution proof, not an OS
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

## Expanded arithmetic profile (0.11 alpha)

OR/XOR/TEST clear CF/OF, classify AF undefined and update SF/ZF/PF. TEST and
CMP do not write their operands; they can read protected RAM. NOT and NOP
preserve flags and undefined metadata. INC/DEC preserve CF and its metadata.
NEG computes zero minus its operand. New ALU forms use the same staged RAM
retirement and segment checks as existing instructions. Implicit-product MUL/IMUL, DIV, stack
operations, CALL/RET and general device I/O are still outside this profile.
The verifier adds ALU8/16/32 fixtures plus read-only TEST and NEG fault cases.

## Signed multiplication (0.12 alpha)

- 0F AF /r: IMUL r16/r32,r/m16/r/m32.
- 69 /r: IMUL r16/r32,r/m16/r/m32,imm16/imm32.
- 6B /r: IMUL r16/r32,r/m16/r/m32,sign-extended imm8.

The low operand-width product is written to the destination register. CF and
OF report whether the exact signed product fits that width. PF/AF/ZF/SF are
undefined (concrete zero plus undefined markers). Memory operands are sources;
read-only RAM is allowed. Faults preserve registers and flags. Implicit AX or
DX:AX/EDX:EAX forms F6/F7 /5 remain unsupported; no 8-bit IMUL form is added.
The compiled verifier adds 16/32-bit signed products and source-fault cases.

On Windows 11 PowerShell, after merging and pulling main:

```powershell
$env:CARGO_TARGET_DIR = "target/windows"
./tools/compare.ps1 -Iterations 10000 -Samples 5
```

This verifies all 48 cases, then measures compiled Rust and cached interpreter.
An x86-64 PC also reports the direct CPU arithmetic baseline. That baseline
still times ADD/SUB/JNZ; it is not an IMUL benchmark or an emulated 386 clock rate.
For WSL sharing the folder, use `export CARGO_TARGET_DIR=target/wsl` and
`sh tools/compare.sh 10000 5` in the WSL shell.

## Shifts (0.13 alpha)

SHL (SAL alias), SHR and SAR support byte/word/dword register or memory operands.
D0/D1 /4,/5,/7 use count1; D2/D3 use CL; C0/C1 use imm8. Counts are masked
with31 at every width. Rotates, /6, SHLD and SHRD remain unsupported.

A masked zero count preserves the operand, all flags and undefined markers.
Nonzero counts update SF/ZF/PF, leave AF undefined, and define OF only for count1.
CF is the last bit shifted out; SHL/SHR classify it undefined when count>=width.
SAR sign-fills, rounds negative values downward and keeps CF defined. Undefined
values follow the existing concrete-zero-plus-marker policy. This follows the
Intel SDM flag classification where it clarifies the older386 manual listing.

Memory forms use the existing read-modify-write permission checks even when the
masked count is zero. A failed read or write preserves CPU state and RAM.
The verifier includes shifts at all widths and protected zero/nonzero stores.


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

Build and render the original 256x256 fixed-point guest on Windows:

```powershell
.\tools\mandelbrot.ps1
Invoke-Item .\target\mandelbrot\256x256-i32\mandelbrot.bmp
```

Termux/Linux/WSL: `sh tools/mandelbrot.sh 256 32`. The helpers build release
and run `--mandelbrot SIZE ITERATIONS`. Use powers of two 2..256 and 1..64
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
rate. The normal renderer uses the interpreter; the optional comparison below
also runs the full image as generated Rust. No VGA, firmware or OS boot is implied.

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

## Compare the full Mandelbrot image with generated Rust

Windows: `./tools/mandelbrot.ps1 -Compare`.
Termux/Linux/WSL: `sh tools/mandelbrot.sh --compare 256 32`.
Or run the built executable with `--compare-mandelbrot [SIZE] [ITERATIONS]`.
This requires a local `rustc`; normal image rendering still needs only the
built executable. Size/iteration bounds are the same as the image renderer.

The command first runs the existing 39-case instruction-by-instruction verifier.
It then compiles the selected complete guest image from validated PIL into Rust
and runs both engines. Every pixel must match the independent Q12 reference;
CPU registers, flags, undefined flags, EIP, RAM, serial queues and stop status
must also match at every TX-full drain boundary and at halt. The full image is
checked at those boundaries, not traced after every instruction. Both engines
must complete the same instruction count and backpressure retry count.

The report compares one cold render per engine. Timing includes byte guards,
serial draining and checkpoint capture; setup, rustc compilation, child process
startup, reference checking and files are excluded. This is an observed ratio,
not a median benchmark or original 386 timing. The generated runner retains
black_box at every step. The interactive debugger remains token-interpreted.

Artifacts are saved under CARGO_TARGET_DIR (default target)/backend-comparison/
run-*/: the verification traces in verification/, and emitted source, executable,
both raw pixel streams, state checkpoints and report.txt in mandelbrot/.
The compare command produces comparison artifacts; use the normal render command
without Compare to save/view BMP and PGM images.

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

## Automated verification

This is a runtime-only source delivery. `cargo test` here intentionally runs no
development tests. The automated suite and Windows/Linux CI live on the
[dev branch](https://github.com/pyTony/pil_proof/tree/dev) and the
[Actions page](https://github.com/pyTony/pil_proof/actions). CI checks both the
full development suite and an isolated runtime export. DELIVERY.json identifies
the development source revision used for this delivery.

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

## Rotates and carry constants (0.16 alpha)

ROL/ROR8/16/32 accept register or memory destinations and count1,CL,imm8.
Count bytes are masked by31. Data rotates modulo operand width,while flag
classification uses the masked count:zero preserves all flags and metadata;
one defines CF/OF;larger counts define CF and mark OF undefined. Full-width
turns such as ROL AL,8 retain data but still update CF and leave OF undefined.
PF/AF/ZF/SF and their metadata are preserved. Undefined OF uses PIL's concrete
zero policy; it is not required to match an unspecified hardware value.

CLC(F8) and STC(F9) clear/set CF and its undefined marker; all registers,other
flags and other metadata remain unchanged. RCL/RCR and SHLD/SHRD remain
unimplemented. ROL/ROR memory destinations perform the same RMW permission
checks as shifts even for zero counts; faults preserve CPU/RAM state.

These instructions execute through descriptions and the existing generic PIL
operations in both backends. No new interpreter-only CPU semantics were added.

## Register exchange (0.17 alpha)

XCHG8/16/32 register forms86/87 and AX/EAX accumulator forms91..97 are
supported;90 remains NOP (equivalent self-exchange in the386 profile). Both
operands are read before retirement,so AL/AH and identical-register aliases
work. Upper bits,flags and undefined markers are preserved. Memory XCHG and
LOCK remain outside this profile pending a bus-atomic memory interface.

PIL now permits one staged write to each named binding,dst and src,including
conditional source writes. All reads see incoming state. Active overlapping
writes must agree on their shared bits;otherwise InvalidBinding is returned
before any RAM,register,flag,marker or EIP commit. Source binding widths
remain independently validated. This is a two-binding foundation,not arbitrary
implicit-register writes or ordered multi-store support.

## Complement carry (0.17 alpha.2)

CMC (F5) complements CF using the shared PIL description; registers, RAM and
all other flags and diagnostic markers are preserved. Operand/address-size
prefixes do not change its operation. LOCK is rejected by this decoder profile.
Like ADC/SBB, CMC consumes the concrete incoming CF even when its diagnostic
marker is set, then clears that marker on the defined CF write. This follows
the concrete-value policy; symbolic uncertainty is not propagated.

## Status-byte transfers (0.18 alpha)

LAHF (9F) packs SF/ZF/AF/PF/CF into AH bits7/6/4/2/0, forces AH bit1 to1
and bits3/5 to0, and preserves all flags and metadata. SAHF (9E) defines those
five flags from AH, ignores AH bits1/3/5, preserves OF and other EFLAGS bits,
and clears only the five written diagnostic markers. AL and the upper16 bits
of EAX are preserved; SAHF leaves every register unchanged. Operand/address
size and segment prefixes do not alter these byte operations. LOCK is rejected.

Both are shared PIL descriptions, not interpreter-only special cases. The
native x86-64 oracle checks CPUID.80000001H:ECX bit0 before executing them;
missing feature support is explicitly reported and does not remove guest386
support. Runtime source includes docs/PIL_REFERENCE.md; native archives include
PIL_REFERENCE.md. The complete documentation map is linked from their READMEs.

## DOS executable probe (0.19 alpha)

Use `--dos-probe [--steps N] file.com file.exe` for bounded DOS import and
execution diagnostics. `--write-dos-hello com|exe PATH` creates original hello
fixtures without overwriting files; `dosprobe PATH [steps]` runs an isolated
probe from the REPL. This is separate from raw monitor `load` and uses a small
host DOS-call bridge for print/exit, not a full DOS or CPU interrupt engine.

See the [DOS probe guide](docs/DOS_PROBE.md) for commands, loader limits and report meanings.

## Disk-image and ZIP imports (0.20 alpha)

`--dos-list IMAGE.img|PACKAGE.zip` lists programs. `--dos-probe` can run the
image/package candidates directly,including one nested package level,with
byte-identical duplicates removed. `--dos-extract INPUT NEW_DIRECTORY` extracts
files without overwriting an existing destination or modifying the source.
FAT16 and ZIP stored/deflate are supported;this is not a booted DOS filesystem.

See [container guide](docs/CONTAINERS.md) for commands and format limits.
