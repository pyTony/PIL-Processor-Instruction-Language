# Observed DOS code compiled through PIL and Rust

## Standalone ANSI Mandelbrot

The original `programs/dos/MANDANSI.COM` is an end-to-end standalone compiler
example: a 79x24 ANSI-coloured Mandelbrot, using signed 8.8 fixed-point
arithmetic, IMUL, nested loops, memory and near calls. It renders through DOS
AH09 and restores terminal colours on exit. No new PIL semantics or Rust
instruction-generation rules were needed.

```powershell
python tools/build_ansi_mandel.py --check
python tools/verify_ansi_mandel.py
```

The verifier generates standalone Rust and compiles it with rustc. It compares
all 1896 cell colours/glyphs and every ANSI byte with Dine and an independent
Python fixed-point reference. Raw ANSI output, a plain-text preview and a
result JSON are saved under `target/ansi-mandel-verified/`. Timing is one local
wall-clock sample with different frontend costs, not a controlled speedup
benchmark. Use an ANSI terminal for colour; DOS execution needs ANSI.SYS.
The generated application has no decoder/interpreter dependency. The NASM
source and a dependency-free byte builder are included in the repository.

## Standalone HELLO.COM milestone

Generate an original fixture, translate it, and compile the self-contained Rust
source directly (use new output paths):

```powershell
cargo build --locked
target/debug/pil.exe --write-dos-hello com target/standalone-hello.com
target/debug/pil.exe --emit-dos-standalone target/standalone-hello.com target/standalone-hello.rs
rustc --edition=2021 -Awarnings -O target/standalone-hello.rs -o target/standalone-hello.exe
target/standalone-hello.exe
python tools/verify_dos_standalone.py
```

On Linux/Termux use `target/debug/pil` and an executable name without `.exe`.
The application runs after removing both the original COM and generated source;
it needs neither Cargo, Dine nor an installed guest file at execution time.

The compiler accepts a graph of at most 4096 register, ordinary RAM and supported stack PIL instructions
and DOS INT21/AH02, AH09, AH4C and INT20. It discovers code from the entry and
follows direct control-flow edges, without running or profiling the program.
It emits native instruction functions using the existing Rust generator,
an exact sparse snapshot of initial RAM, and a small guest context/service
module. The initializer also preserves the loader's A20 setting. AH09 string validation is shared with Dine. Its AL return value, bounded
output and exit status are preserved. The decoder, token interpreter, loader
and fallback backend are absent from the generated source.

Both linked Dine and standalone compilation use `src/dos_blocks.rs` for
instruction profiles and the 16-instruction block plan, and `src/dos_codegen.rs`
for Rust instruction-module and block emission. Static discovery and loaded
state preparation live in `src/dos_translate.rs`; the carry idiom recognizer
returns typed metadata from `src/pil_opt.rs`. See
[compiler reuse boundaries](development/COMPILER_REUSE.md). Runtime adapters provide bindings, retirement,
fault reporting and standalone return-site metadata. Standalone execution
dispatches blocks; a test-only single-instruction path remains for comparison.
Blocks stop at stores, control transfers and machine-context changes. Instruction
helpers are inlined within blocks, so Rust can eliminate dead temporary/flag work
without changing PIL instruction definitions. A block fault retains its retired
prefix, verified against single-instruction execution.

The shared `BlockPlan` partitions eligible code into primary blocks. Direct
targets, continuations and observed Dine control-transfer targets are leaders.
Every interior site retains a one-instruction alternate entry, so entry in the
middle and small budgets remain supported without emitting a full suffix again.
The same plan determines emitted bodies, budget counts and code-byte guards.
For ANSI Mandelbrot this reduces emitted instruction bodies from 134 to 82
(22 primary blocks, 30 alternates); all 52 entry addresses remain available.

Direct short/near JMP, all 16 short/near Jcc conditions, LOOP/LOOPE/LOOPNE,
and JCXZ/JECXZ are supported, including independent operand/address sizes.
Both conditional successors are discovered. A PIL-based analysis tracks AH
constants through branches, AL-only writes and instructions that preserve AH.
MOV AX/EAX or MOV AH with a literal can establish the service number; other
AH writes invalidate it. Every incoming path to INT21 must prove AH02, AH09 or AH4C.
All reachable paths must be supported and remain in the loaded COM/MZ module. Overlapping
instructions are rejected; service analysis is bounded to 65,536 states.
The emitted native dispatch loop executes the guest's register/flag semantics;
it contains no instruction decoder or token interpreter. A reachable exit is
required, but termination of every execution path is not proved.

Ordinary addressed reads use the existing PIL `read_address`/`load` effects
and the existing `execute_with_machine` Rust wrapper. This includes 8/16/32-bit
MOV loads, MOVZX/MOVSX and read-only memory operands of arithmetic/comparison
instructions. Address and operand sizes remain independent; moffs, 16-bit
effective-address wrapping, 32-bit SIB and segment overrides use the shared
address/segment/RAM implementation. A loaded AH is unknown to service analysis,
so an INT21 service number must be established again after such a load.

Ordinary addressed stores, conditional stores and read-modify-write instructions
also use the existing PIL store effects and Rust generator. Byte/word/dword MOV,
memory arithmetic and memory XCHG retain their existing staged-effect and fault
rules. The initializer copies the loader's read-only spans and protects the
exact translated instruction bytes. Data in the same module remains writable.
Writes touching translated code are runtime errors, including partial overlaps,
A20 aliases and explicit same-value writes. Existing PIL store checks are
preserved, including those of zero-count memory shifts. Self-modifying code is not supported by this standalone subset;
there is no stale-code execution or interpreter fallback after such a write.

Register/immediate PUSH, register POP, direct near CALL and near RET/RET imm16
are supported at 16/32-bit operand widths. They use the existing PIL stack,
store/load-segment and branch effects. Discovery follows call/return contexts
using function summaries keyed by entry, incoming AH and return width (at most
65,536 analysis states), including the callee's effect on the known DOS service
byte. Recursive base-case returns propagate to waiting callers until the
summaries stop changing; analysis does not unroll a host call stack or assume
that callees preserve AH. This accepts direct recursion, while indirect calls
remain separate work. Actual stack bounds and accesses are checked at runtime.
A separate return-site stack checks that each RET reaches its proven caller
with matching operand width; a rewritten return address is rejected before
committing return register/IP/SP effects. Guest stack faults keep their shared
PIL behavior, and stack writes cannot modify translated instruction bytes.

`RECURFIB.COM [n]` is an original recursive stack example (default 12, maximum
20). It prints F(0) through F(n), using two recursive calls and saving BX/CX on
the guest stack. `tools/build_recursive_fib.py` generates the COM and matching
NASM source; `tools/verify_recursive_native.py` compares native/Dine results
with an independent iterative reference, checks register/SP restoration on
recursive returns, and exercises a separate 65-frame chain.

The shared PIL-to-Rust emitter recognizes natural-width result/carry extraction
from widened add/sub-with-carry dataflow. Portable overflow operations expose
the result and carry separately to LLVM; unused flag calculations remain
removable, and x64 probes lower to ADC/SBB chains. Original PIL flag formulas
and instruction commit/fault boundaries remain in force. This is a narrow
dataflow optimization, with no opcode-specific assembly or host-FLAGS state.

Standalone executables accept host command-line arguments joined by a single
space between arguments. Before execution, the shared Dine PSP-tail writer
stores the DOS count, leading space, CP437 text and CR at PSP:80h. Host Unicode
is encoded at the boundary, with a limit of 125 DOS bytes; invalid input fails before
guest execution. No arguments preserves the loader's empty tail. Host-shell
quoting is consumed by the host shell; quotes are not reconstructed for DOS.

Output is decoded from CP437 to UTF-8 by default. For a CP437 terminal, including
one attached through a Telnet server, select raw DOS bytes instead:

```powershell
$env:PIL_DOS_RAW_OUTPUT = "1"  # raw CP437 for the terminal connection
.\argument-demo.exe tämä on demo
$env:PIL_DOS_RAW_OUTPUT = "0"  # restore UTF-8 output
```

This changes text encoding only; the Telnet server must supply transport framing.
ANSI ASCII escape sequences are unchanged. Unrepresentable characters (for
example emoji or euro in CP437) produce an explicit error. `src/dos_encoding.rs`
is independent of instruction semantics and runtime state. The Dine JSON session
always remains valid Unicode JSON; raw mode applies to standalone stdout.

AH02 single-character output uses the same tab expansion, bounded-output check
and AL result as Dine's console output. AH09 string output was already supported.
Only the standalone headless console is supplied; redirected files and keyboard
break checks remain part of the full Dine runtime.

Scalar LODSB/W/D is admitted through the existing PIL load-segment, DF and
pointer effects (including address/segment overrides). Checked DIV/IDIV uses
the existing PIL divide predicates and emitted exception return; zero divisors
and quotient overflow stop standalone execution precisely, without installed
guest exception-handler delivery. REP strings remain separate.

The original `programs/dos/FACTOR.COM` is the next arithmetic workload after
Mandelbrot. It accepts 0..99 (default 25), stores decimal digits in guest RAM
and prints exact factorials. `python tools/verify_factor_native.py` compiles it
unchanged, compares results with Dine and Python, and checks precise scalar
load/division failures. Its carry is a numeric decimal-digit value, not an ADC
flag chain. No new x86 or PIL-to-Rust implementation was introduced.

Other implicit stack/string memory access, indirect calls/jumps, unpaired RET,
far calls/returns, MZ inputs and other services are compile errors;
no source file is created for rejected input. There are no files, keyboard,
screen rendering, interrupt handlers or EXEC in this subset.
The runtime is currently an embedded reusable module, not a separately packaged
library. General DOS programs continue to use the bundle path.

Local Windows validation: 49 independently authored fixtures match Dine's
output and exit code, including all 16 Jcc conditions taken and untaken,
loop arithmetic/output, JCXZ versus JECXZ, operand-size branches, memory widths,
sign/zero extension, upper-register preservation, moffs/SIB and wrapping addresses.
They compile directly with rustc and run after deleting the guest and source
files. The same verifier
is configured for Windows/Linux CI; no new Linux/Termux run is claimed here.
Eight read tests, seven store tests and five call/stack tests compiled directly into generated Rust additionally verify
precise segment/RAM faults, unchanged state on a failed read, live SS/FS caches,
successful reads at the segment end and from read-only RAM, and A20 wrapping.
Store tests also cover code/ROM protection, A20 aliases, partial overlap,
same-value writes, zero-count shifts and precise RMW/XCHG failure state.
No separate x86 or PIL-to-Rust instruction implementation was added.
Twelve additional command-line cases match Dine, including Finnish characters,
empty arguments, multiple spaces and the 125-DOS-byte limit, with UTF-8 and raw
CP437 output tested. Oversized/control/unrepresentable tails are rejected before
execution. Character-output cases check AL, tabs and loops.

## Code-write policy in pil.exe

The linked `pil.exe` engine allows self-modifying code and can fall back locally
to Dine. Its decoded-instruction cache assumes a watched physical code page is
unchanged until its first successful write, including a same-value write. That
page then permanently uses byte validation; faults that commit no write leave
the unchanged-code assumption valid. Paging and memory aliases retain their
existing conservative checks.

Generated native blocks end at stores and recheck instruction bytes and machine
context before the next block. Changed instruction sites fail their compiled
guards and use the interpreter. Unchanged sites, even on the same dirty page,
remain eligible for native execution. A code write therefore does not disable
compilation for the whole program. This is cache invalidation and local
fallback, not automatic recompilation of newly generated code.

`python tools/verify_dos_smc.py` builds an isolated linked backend for an original
fixture that changes INC AX to INC DX. It checks complete final CPU/image/RAM
state against Dine and confirms that the changed site falls back while another
site on the same page still executes through the compiled backend.

The standalone applications above have no linked interpreter. Their translated
instruction bytes remain protected and code writes stop explicitly.

## Build a bundle for a DOS drive

Rebuild the profiling engine first. The bundle builder scans `.COM`/`.EXE`
files, deduplicates identical executable bytes, profiles each in isolation and
compiles all successful profiles into one engine:

```powershell
cargo build --release --locked
python tools/build_dos_bundle.py driveD --recursive --engine target/release/pil.exe --steps 2000000 --sites 512
```

Use `--limit N` for a small trial, and `--output DIRECTORY` for a separate
bundle. The default result is `shadow/bundle/pil-shadow.exe` on Windows.
Launch the normal terminal with that engine, mounting the original DOS drive.
The bundle selects a backend by executable content whenever EXEC changes the
active program. Unknown executables, changed code and unobserved sites fall
back to the interpreter. Data stays on the original drives; no data mirror is
created. Real-mode block guards accept another ordinary load segment while
checking live segment semantics, loaded fetch windows and code bytes.

Startup profiling alone does not exercise interactive workloads. To train
GW-BASIC with the included arithmetic example:

```powershell
python tools/build_dos_bundle.py driveD/GWBASIC.EXE --engine target/release/pil.exe --train GWBASIC.EXE=programs/dos/training/GWBASIC.KEYS --steps 20000000 --sites 512 --output shadow/gwbasic
```

Training text is replayed as BIOS keystrokes after a startup interval, without
mounting host directories. It may need adapting for a program's startup flow.
The profile stops at exit, an unanswered blocking key wait, a fault or its
step limit. If more sites are observed than the limit, entry instructions and
the hottest observed sites are retained. The manifest records the stop reason,
coverage count, original executable hash and training input. Unsupported files
are reported individually and skipped. A directory build can be expensive;
larger profiles increase both generated code size and Rust compilation cost.

The Windows trial bundle in `shadow/gwbasic-mandel-bundle` contains GW-BASIC
(256 selected sites) and Mandelbrot. Three alternating arithmetic-workload
runs matched the sum 125250, screen, CPU state and exit: interpreter median
0.5365 s, bundle median 0.3068 s; 1,354,263 compiled instructions out of
1,804,127 guest steps. This is one trained workload, including protocol/input
overhead, not a general GW-BASIC speed guarantee. Reproduce with:

```text
python tools/verify_gwbasic_bundle.py --native-engine shadow/gwbasic-mandel-bundle/pil-shadow.exe
```

The same bundle was also checked with a host-clock DOS shell: GW-BASIC ran the
sum workload after EXEC and returned normally, and TIMEIT / MANDVGA used
9,621,157 compiled block entries and reported 1.81 s (no graphical frontend in
that test). An executable absent from the bundle matched interpreter output
and reported state with zero compiled blocks.

## Build and verify Mandelbrot

From the repository root, with Rust and Python installed:

```text
python tools/build_dos_shadow.py programs/dos/MANDVGA.COM --verify
```

The helper builds a profiling engine, runs the original COM file, writes the
observed Rust source, and builds it in one Cargo invocation. `--verify` runs
generated-code state and invalidation tests and then three complete Mandelbrot
comparisons. The source/output cache is:

```text
shadow/<target_os>-<target_cpu>/<guest_sha256>/
    backend.rs
    manifest.json
    pil-mandvga.exe     # Windows; pil-mandvga on Linux/Android
```

The manifest identifies the target OS, CPU, Rust target triple, compiler,
original guest hash and source/binary hashes. Rust source and binaries in
`shadow/` are local generated files excluded from Git. No DOS executable is
replaced. Compilation is native to the current host; cross-target builds are
not selected by this helper.

The profiling run has no host-folder mounts. It stops on exit, keyboard wait,
fault, unsupported service, or its explicit step budget. The single-program
helper defaults to 2,000,000 steps and 512 retained instruction sites, with
progress on stderr. Use `--steps N` (up to 120,000,000), `--sites N` (up to
65,536), and `--keys FILE` for a longer or trained profile. The retained sites
include the entry block followed by the hottest observed offsets. It records
only actually executed PIL sites belonging to the requested executable.
Data is not scanned as instructions. Code after an
unanswered key wait and branches not taken remain interpreter fallbacks. A
second encoding at an already recorded offset is not compiled in this version.

Generated instruction modules use an `ins_` prefix: hexadecimal offset 0Fh
must produce `ins_f`, never the Rust keyword `if`. Small block bodies are
optimized individually, without inlining hundreds of overlapping blocks into
the dispatcher. Region chaining returns to Dine after any segment/context change,
so subsequent byte guards use the updated CS cache. With `--verify`, an engine
is published only after the requested checks pass. A profile ending at an unsupported service is partial;
`--verify` checks generated state/code guards, not a complete interactive
application run. In particular, Turbo Pascal's isolated startup profile still
stops at a second open of its executable; the built shadow uses interpreter
fallback for unobserved paths.

An original synthetic MZ regression covers the offset-0Fh identifier and a
far call/return between segments with the same instruction offset. It builds
the shadow, runs its guard/state tests, and compares its exit, registers,
flags, selectors and screen with the interpreter:

```text
python tools/verify_dos_shadow_edges.py --engine target/release/pil.exe
```

For an existing profiling engine, add `--engine target/release/pil.exe`
(or `target/release/pil` on Linux/Termux). This engine must have been rebuilt
from the same compiler sources. The generated source is a module for Dine;
it is compiled with the project, rather than by invoking rustc on it alone.

## Run with graphics

Use the exact native service path printed by the helper:

```text
python tools/dos_terminal.py --engine SHADOW_SERVICE --headless programs/dos/MANDVGA.COM
```

The existing frontend opens its graphics window when a graphics frame appears
and closes it on return to text mode. This change does not introduce a second
GUI implementation. Running through the JSON protocol verifies frame production
but does not itself visually inspect a desktop window.

The frontend caps execution-frame transfers at 120 Hz. Engines advertising
`buffered_run` accept `run N buffered`, returning execution status and console
output without serializing the framebuffer. A `snapshot` transfers the latest
frame at redraw time; intermediate frames are retained only as current device
state in the engine, without a growing frame queue. Older engines retain the
full-reply path. Execution and rendering are still synchronous, and 120 Hz is
a cap rather than a guaranteed display rate.

To start COMMAND.COM instead, pass its image path and the usual mounts, then
run the matching program from DOS. The backend compares program identity when
EXEC changes the active executable; unrelated programs use the interpreter.
The direct Mandelbrot workflow and a COMMAND.COM / TIMEIT / MANDVGA launch
have both been checked on Windows. Real-mode relocation guards now permit
compiled blocks after EXEC; earlier builds were tied to the profiling load
address and reported zero blocks through the shell.

For DOS `TIMEIT`, launch the frontend with `--auto-graphics-key` and run
`TIMEIT MANDVGA.COM` at the DOS prompt. The frontend supplies Enter when a
graphics program reaches a blocking key wait, so the result excludes a human
key press. This option applies to every graphics key wait while enabled; use
it only when the key is meant to end the test. A program that polls for keys
without entering a blocking wait is not covered.

## Generated blocks and correctness boundaries

The writer joins consecutive observed instructions, currently at most 16 per
block, ending after a store or control-flow instruction. Instruction helper
functions are inlined so Rust can optimize across the operations in the block.
There is no interpreter token dispatch or decode-cache lookup inside a block.
An internal compiled dispatch loop follows observed branches, calls and returns
across these blocks, returning to Dine after at most 256 guest instructions, a
guard miss, a fault, an interrupt shadow or a paging change. The same compiled
region is reused for every pixel; pixel coordinates are runtime data. This is
bounded block chaining, rather than a reconstruction of guest loops as Rust
arithmetic loops.

Entry validates the executable identity, segment/machine state, loaded fetch
windows and the entire block's code bytes. Entry at a later recorded offset
selects its corresponding sequence or falls back; it never jumps into the
middle of native machine code. A code write ends its block before another entry
can run. Changed bytes or machine context cause fallback. Paging, REP partial
execution, interrupt shadows and installed host-clock timer handlers retain
instruction-level execution in this version. Blocks fit wholly within the
remaining frontend step budget; faults retain the successfully retired prefix
and are delivered from the failing instruction.

Inside an unpaged block, register-only operations from a conservative PIL
whitelist skip per-instruction RAM paging synchronization. Memory accesses and
other operations use the regular generated wrapper; instructions that can
change the address space end the block. The entry guard also refuses the fast
path if RAM is still configured for paging. VGA accesses continue through the
existing controller, including its latches and write modes.

The JSON service reports cumulative `native_blocks` and
`native_block_instructions`. These count guarded block entries within regions,
including single-instruction entries used to follow calls and returns; the
separate generated instruction fallback is not included. Block counts from
before region chaining therefore have a different scope.

## Reproduce the comparison

```text
python tools/verify_dos_shadow.py --native-engine SHADOW_SERVICE --samples 3
```

The checker compares all four VGA planes, palette, caption/screen, cursor,
reported CPU state, status and instruction count, then checks key exit and
text-mode restoration. The independent mathematical VGA oracle can also be run:
set `PIL_TEST_ENGINE` to the native service path and run
`python tools/verify_mandel_vga.py`.

Timing covers drawing to the keyboard wait and protocol frame transfer; process
startup, profiling, Rust compilation, and reference checking are excluded. It
includes the full DOS workload, rather than only the Mandelbrot arithmetic.
Do not compare it directly with the hand-written calculation-only reference or
run Cargo tests/builds concurrently with a performance measurement.

Results are written to `target/dos-shadow-comparison.json`. Performance depends
on the host and workload; correctness and actual block counts must be checked
before interpreting a speed ratio.

## Landmark compatibility check

Landmark 6.0 now reaches its welcome screen and continuous CPU benchmark with
the VGA identity. The earlier working screenshot identifies CGA, which may
explain why this vendor-detection path was not previously encountered; a
revision-by-revision regression cause has not been established. Missing
Everex (7000h), Video7 (6F00h) and Chips/Realtek (5F00h)
queries with BX=0 return without advertising their extension signatures.
Absent indexed extensions and Tseng/Oak ports read FFh and ignore writes;
CRTC index 33h no longer aliases standard register 13h. Real timing changes
and unrelated device operations keep their existing checks.

The BIOS user-font load AX=1100h supports block zero at the current text cell
height, checking the complete source span before replacing glyphs in plane 2.
As with existing guest font-plane writes, Tk still displays its host font;
custom glyph definitions are stored/readable but their bitmaps are not yet
used to draw the text window. Paging and memory permissions are unchanged.

The regular engine was checked both directly and through COMMAND.COM with
`LM60 /b /q /2`: the benchmark screen appeared and timed exit returned code 0.
Landmark is not included in the current two-program shadow bundle. A separate
bounded startup shadow can be built, but a profile covering its timed workload
and a controlled comparison are needed before claiming benchmark acceleration.
Its continuously changing MHz-AT figures are emulation scores, not a claim
of cycle-accurate 386 timing.

F9 recalculation was also checked through COMMAND.COM: a key was sent after
the first CPU score, the clock-calculation screen appeared, scores returned,
and the timed run exited with code 0. No-driver INT 33h calls other than reset
now preserve registers, flags and RAM, matching an absent-driver IRET stub.
This covers Landmark's text-cursor setup (AX=000Ah) and driver-state calls;
it does not install a mouse or invent cursor/button data. A guest-installed
INT 33h handler retains precedence; a regression test installs and exercises
one. Reset continues to report no installed driver.

References: [BIOS font-load ABI](https://fd.lod.bz/rbil/interrup/video/101100.html),
[video extension queries](https://fd.lod.bz/rbil/interrup/video/index.html),
[DOSBox-X CRTC index handling](https://dosbox-x.com/doxygen/html/vga__crtc_8cpp_source.html).

### CGA mode control and Turbo Pascal

Port 03D8h retains the full written byte in the VGA compatibility state.
The original CGA register is output-only: IN returns the profile's FFh
unconnected-bus value, while BIOS 0040:0065 remains a software shadow.
Direct OUT does not change BIOS RAM or clear framebuffer memory.

Bit 3 blanks/restores text and graphics presentation. Bit 5 selects text
blink versus bright backgrounds; Tk blinks text on a half-second cadence.
BIOS AX=1003h and attribute-controller blink writes share this state.
Bit 2 selects the existing mode-4/mode-5 RGB palette behavior in 320x200
CGA graphics; composite analog output is not simulated. Width, graphics/text
and high-resolution layout changes (bits 0, 1, 4) remain explicit errors,
before any state change. BIOS mode sets restore the standard latch value.

Regression tests exercise OUT DX,AL to 03D8h, output-only readback, unchanged
BIOS shadow/pixels, rejected layout changes, BIOS reset, palette selection
and frontend blink/blanking. The unchanged local TEST286.PAS was compiled
with Turbo Pascal 7 TPC and TEST286.EXE exited with code 0 through COMMAND.COM.
The IDE opens that source, but Ctrl+F9 currently stops at the separate
unimplemented DOS INT 21h/AH=55h child-PSP service; IDE execution is not
claimed to pass.
Third-party source and executables remain outside the repository.

## Partial-profile dispatch overhead

A native miss used to materialize pending lazy flags before discovering that
the current program/site was not compiled. A partial shadow could therefore
slow down the interpreter fallback, including the DOS shell that launched it.
Backends now provide conservative eligibility checks before flags are flushed.
The generated checks use program identity and live code bytes, so an offset
from another segment is not treated as a compiled hit. Full byte/context guards
remain in the executor. Bundle selection delegates the checks to its selected
program; programs absent from the bundle continue directly in the lazy interpreter.
Existing shadow binaries need rebuilding to use these checks.

On local Windows, three alternating fixed-step LM60 startup/calibration runs
gave old-shadow median 2.1215 s and corrected-shadow median 1.8590 s (12.4% less
time). Each run requested 5,000,000 execution steps, reported 2,392,073 retired
instructions and 819,590 compiled block instructions, and matched CPU state,
selectors and flags. A separate three-run reference comparison gave interpreter
median 1.9669 s and corrected-shadow median 1.8842 s. These runs use no Tk and
no host clock: they measure this fixed workload, not Landmark's displayed MHz,
nor complete coverage of its continuously running CPU benchmark.

Reproduce the fixed-work comparison with local LM60.EXE:

```text
python tools/verify_landmark_shadow.py --engine PATH_TO_REFERENCE_ENGINE --native-engine shadow/landmark-fallback-fix/pil-lm60.exe
```

The experimental engine was built with `build_dos_shadow.py --output-dir
shadow/landmark-fallback-fix`; the existing canonical shadow was preserved.

### Older Mandelbrot shadow and DOS EXEC

The older `aa6c44.../pil-mandvga.exe` built on 9 November at 16:54 used exact
recorded segment-state equality for block entry. Direct COM loading matched;
DOS EXEC loaded the same bytes at different segments and lost block execution.
With that same engine and no Tk, a local host-clock comparison reached the
graphics key wait in 1.2404 s directly (37,863,543 native block instructions),
versus 11.9871 s through COMMAND.COM (zero native blocks). Both exited with
code 0 after Enter. The two COM files had identical SHA256 hashes.

Consequently, starting a default DOS shell with `--engine PATH` and starting
`programs/dos/MANDVGA.COM` directly are different benchmark paths. A GUI/headless
comparison must supply the same program and key-wait behavior in both commands.
Current generated real-mode guards allow ordinary EXEC relocation, while
retaining live code-byte, segment-limit and non-paged-context checks.

The separate rebuilt `shadow/mandelbrot-relocation-fix/pil-mandvga.exe` passed
four generated guard/state tests and three full VGA comparisons against the
interpreter. A subsequent host-clock, no-Tk direct/EXEC comparison took
1.2395 s directly (37,863,534 compiled block instructions) and 1.5617 s
through COMMAND.COM (37,863,513 compiled block instructions). The VGA planes,
palette and dimensions matched across launch paths; both exited with code 0.
These timings exclude the final key wait and do not establish a Tk render time.
