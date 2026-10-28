# Debugger guide

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


Start with `sh tools/debug.sh` (Termux/Linux) or `./tools/debug.ps1` (Windows).
The script builds and loads the supplied selftest binary. `demo selftest` loads
the same program without a file. `load PATH` opens another raw binary.

## First session

```
list
step
serial rx 41
run 1000
check
mem 0x500 8
regs
serial tx
```

Success stops at HLT with DX=600D. A failed guest check stops with DX=DEAD.
`check` also verifies four RAM signatures and successful TX=41. The program
waits at IN if no input is queued. `--selftest` injects 41, runs and checks the
result automatically, returning nonzero on failure.

Successful guest output is shown after step/run as hex and escaped text:
`serial output: 41 | A`. This does not drain TX. `serial tx` retrieves queued
bytes; draining them does not invalidate `check`. Control bytes are escaped.

## Commands

| Command | Purpose |
|---|---|
| help | List commands |
| demo NAME | Load selftest, hello, factorials, loop, counter, echo, memory or mandelbrot (see table above) |
| load PATH | Load a raw binary, 1..3840 bytes; paths contain no spaces in the REPL |
| list [COUNT] / list ADDRESS COUNT | Show bytes, operands and branch targets; default16, max256; * marks EIP |
| step / s | Execute one instruction, ignoring its breakpoint |
| run [N] / continue [N] / c [N] | Run up to N completed steps; default 100, maximum 10,000 |
| regs / flags / eip | Inspect state; ? marks undefined flag results |
| set eax VALUE, etc. | Set a 32-bit general register, eip or eflags |
| break ADDRESS / delete ADDRESS / breaks | Set/remove/list breakpoints |
| reset | Restore initial CPU/RAM, clear serial queues/history; retain breakpoints |
| mem ADDRESS [COUNT] | Inspect 1..256 bytes, default 16 |
| mem8 / mem16 / mem32 ADDRESS [COUNT] [hex/dec] | Dump 8/16/32-bit values plus printable ASCII; count is values |
| poke ADDRESS HEX... | Write 1..256 bytes after checking the entire chunk |
| protect ADDRESS COUNT | Make a RAM interval read-only until reset |
| serial status | Show RX/TX queue state |
| serial rx HEX... | Queue guest input, for example 41 00 ff |
| serial tx | Drain output bytes as hex |
| serial clear | Clear RX and TX queues |
| quit / exit | Leave the debugger |

Numbers use decimal or 0x hexadecimal; serial/poke byte tokens are hexadecimal.
Register names for set are case-insensitive; commands and demo names are lowercase.
Continue skips the current breakpoint once, then stops if execution returns to
it. Binary breakpoints may use any offset within the loaded image; choose an
instruction boundary from the listing. Runs have a finite budget; Ctrl-C may
terminate the process rather than pause it.

The counter example subtracts EBX from EAX until zero. Echo waits for one byte
and returns it through mock serial. Memory stores/loads ECX at RAM 0x10;
`protect 0x10 4` demonstrates a restartable store fault. These three examples
use predecoded harness operations. The selftest fetches actual instruction bytes.

## Binary monitor profile

The image loads at offset 0x100 in 4096 bytes of RAM. CS=0, DS=0x10, SS=0x20,
FS=0x30, ES=GS=0, with ordinary 64-KiB segment limits. Default operand and
address sizes are 16 bits. Data accesses can use 32-bit overrides. Code is
fetched from RAM at each step, so poke can modify it. The included listing
`programs/selftest.lst` shows addresses and instruction intent.

Supported encodings:

- B0..BF MOV immediate to register, including high-byte register views.
- 88..8B MOV register/register or register/memory, with 8/16/32-bit data.
- 00..03 ADD, 08..0B OR, 10..13 ADC, 18..1B SBB, 20..23 AND,
  28..2B SUB, 30..33 XOR, 38..3B CMP: both register/memory directions.
- Accumulator-immediate forms 04/05 through 3C/3D for those eight ALU operations.
- 80/81/83 immediate ALU groups; 83 sign-extends its byte to operand width.
- 84/85 TEST r/m,register; A8/A9 accumulator TEST; F6/F7 /0 TEST immediate.
- F6/F7 /2 NOT and /3 NEG; FE/FF /0 INC and /1 DEC (register or memory).
- 40..47 INC and 48..4F DEC word/dword registers; 90 NOP.
- 70..7F short Jcc; 0F 90..9F SETcc with register destination and /0.
- 66 operand size, 67 address size, all six segment overrides.
- 16-bit ModR/M, 32-bit ModR/M/SIB, disp8 sign extension and disp16/32.
- E4 E9 / E6 E9: IN AL / OUT AL using mock serial on monitor port 0xe9.
- F4 HLT: monitor stop at the instruction.

One prefix per supported group is accepted. Repeated group prefixes, LOCK/REP,
other unsupported encodings stop with a diagnostic and unchanged source EIP.
The loaded-image boundary is a monitor boundary, not an architectural exception.
Segment bounds and permissions are checked before RAM accesses; CPU and the
single staged RAM store remain unchanged on a later fault. #SS(0)/#GP(0)
identify segment bounds. RAM bounds/read-only errors are emulator diagnostics,
not guest paging faults.

This is not a bootable PC emulator. Port 0xe9 is not an 8250 UART: empty RX or
full TX waits and retries the instruction. HLT does not model interrupts or
privilege checks. Loaded segment caches are supplied by the monitor; descriptor
loading, mode changes, paging, A20, BIOS, interrupts and OS boot are absent.
VGA, PS/2, SD and WLAN are not implemented in this release.


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

See [feature matrix](FEATURE_MATRIX.md) for the supported, partial and planned
areas in 0.14.0-alpha.1. This is an alpha of the host execution proof, not an OS
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


## Sharing a source folder between Windows 11 and WSL

A folder such as C:\Users\name\pil_proof is accessible in WSL as
/mnt/c/Users/name/pil_proof. Give Windows and WSL different build targets.

PowerShell, in the project folder:

```powershell
$env:CARGO_TARGET_DIR = "target/windows"
./tools/build.ps1
./tools/compare.ps1 -Iterations 10000 -Samples 5
```

WSL, in the same source folder:

```sh
export CARGO_TARGET_DIR=target/wsl
sh tools/build.sh
sh tools/compare.sh 10000 5
```

These settings last for the current terminal session. Use each environment's
Rust toolchain and scripts. Source and Cargo.lock are shared; Windows .exe and
Linux executables remain in separate build directories. Run Git operations in
one environment at a time. The repository uses LF line endings for scripts.

WSL builds on /mnt/c can be slower than builds within the Linux filesystem.
For primarily WSL work, a separate checkout under ~/pil_proof is an option;
keep it synchronized through GitHub. Compilation/file I/O times are separate
from the benchmark's inner guest-execution timer.

References: [Cargo build directories](https://doc.rust-lang.org/cargo/reference/build-cache.html)
and [Microsoft WSL filesystems](https://learn.microsoft.com/en-us/windows/wsl/filesystems).


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

Verifier mismatch reports include the case, step (starting at 1), instruction
EIP and field, such as EAX, EFLAGS, RAM[0x500] or TX[0x0]. RAM indices are
linear byte addresses; RX/TX indices are queue positions. A terminal halt,
fault or wait attempt also has a trace record. Truncated and malformed traces
fail verification. Both interpreter.trace and generated.trace are retained
under the reported artifacts path on a mismatch. No timings run after failure.

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

This verifies all 39 cases, then measures compiled Rust and cached interpreter.
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

Relative branches now include0F80..8F near Jcc rel16/32 and EB/E9 JMP rel8/16/32. Taken targets retain width wrapping and CS limit checks; flags are preserved. Indirect/far jumps remain unsupported.

## Full-image backend comparison from the shell

Use `pil-proof --compare-mandelbrot [SIZE] [ITERATIONS]`, default256/32;
Windows helper `./tools/mandelbrot.ps1 -Compare`, shell helper
`sh tools/mandelbrot.sh --compare 256 32`. This requires rustc and first runs the
39-case per-instruction verifier. The full image checks every pixel and CPU/RAM/
flags/serial/stop state at serial-drain/halt boundaries. One cold render per
engine includes draining and checkpoint capture, excluding setup/reference/files
and generated compilation. Comparison artifacts live in target/backend-comparison
(or CARGO_TARGET_DIR). The REPL engine remains the PIL token interpreter.

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

`check` supports selftest, factorials, loop, extensions and the small Mandelbrot demo.
Other programs report that no built-in check is available; a supported but
incomplete image retains its specific verification failure message.

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
work. Upper bits,flags and undefined markers are preserved. Memory XCHG86/87 now exchanges ordinary RAM with a register through shared PIL.
Explicit LOCK is accepted only on memory XCHG; flags and upper bits are preserved.
The single-executor RAM transaction is indivisible; MMIO, DMA and concurrent bus
masters are not modeled.

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

## DOS import/probe (0.19 alpha)

`dosprobe PATH [steps]` imports and runs an isolated DOS COM or MZ EXE and
reports its first blocker and reached instruction mix. The current debugger
program, CPU and RAM are preserved. CLI `--dos-probe [--steps N] PATH...`
accepts multiple paths and ranks first missing encodings across the corpus.
`--write-dos-hello com|exe PATH` creates new original DOS hello fixtures.
See [DOS probe guide](DOS_PROBE.md). A minimal print/exit shim is separate
from CPU interrupt execution and DOS/BIOS platform support. Raw `load`
continues to use the original monitor environment.

## Container inputs (0.20 alpha)

The isolated `dosprobe PATH [steps]` accepts FAT16 `.img`/`.ima` and `.zip`
containers too. It probes executable candidates and one ZIP package level,
removing byte-identical container copies. CLI `--dos-list INPUT` inspects;
`--dos-extract INPUT NEW_DIRECTORY` extracts. See [container guide](CONTAINERS.md).

## Memory PUSH (FF /6)

Word/dword memory PUSH supports 16/32 address sizes and ModRM/SIB forms.
The source uses incoming registers (including ESP), with ordinary default or
overridden segments. SS stack width is independent of both prefixes. Flags
are preserved; access faults preserve CPU, memory and retirement markers.
Other FF extensions, including far CALL /3, have separate support limits.

## Real-mode far CALL (9A and FF /3)

Immediate pointers encode an operand-sized offset followed by a 16-bit selector.
FF /3 accepts memory pointers with ModRM/SIB and independent 16/32 addressing;
register encodings are rejected. Source overrides select only the pointer read.
The call stages old CS and next IP/EIP in two operand-sized SS slots, then commits
CS, IP and SP together. Access/target faults preserve the full pre-call state.
RETF can return through CS aliases within the original physical image window.
Protected/V86 calls are rejected. Wide CS padding is zero in this profile; split
stack frames, gates, tasks and hardware exception/shutdown delivery are separate.
