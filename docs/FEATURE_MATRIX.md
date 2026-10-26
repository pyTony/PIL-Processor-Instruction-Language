# PIL 0.10.0-alpha.2 feature matrix

An alpha of the processor-language and host execution proof. It is not yet a
bootable PC emulator. Supported means implemented in the documented prototype;
partial means a deliberately limited subset. See DEBUGGER.md for exact encodings.

| Area | Status | What works / current limit |
|---|---|---|
| PIL data widths | Supported | Explicit 1..64-bit intermediate values; 8/16/32-bit guest operands |
| Arithmetic and flags | Supported subset | ADD, ADC, SUB, SBB, CMP, AND; flag preservation and undefined markers |
| Register views | Supported | 32-bit registers, low 16 bits, AL/AH-style byte views |
| Conditional flow | Supported subset | Sixteen predicates, short binary Jcc; supplied-context wider flow semantics |
| SETcc | Supported subset | Register byte destinations in decoder; sixteen conditions |
| Binary decoder | Partial | Documented MOV/arithmetic/Jcc/SETcc/IN/OUT/HLT monitor encodings only |
| Memory addressing | Supported subset | 16-bit ModR/M, 32-bit ModR/M/SIB, size and segment overrides |
| RAM and faults | Supported subset | 4-KiB monitor RAM, little-endian unaligned accesses, atomic single-store retirement |
| Segmentation | Partial | Supplied normalized caches, base/limit/permissions and expand-down checks; no descriptor loads |
| Token interpreter | Supported | Default debugger engine; cached and uncached modes |
| Decode/PIL cache | Supported | Byte/context validation, bounded capacity, statistics; code writes handled on reuse |
| Rust-source instruction emitter | Supported | Validated PIL emitted as Rust functions; generated/interpreted comparisons |
| Compiled Rust guest backend | Experimental | Offline rustc compilation of fixed guarded images; nine full-state trace cases and loop timings |
| Trace verifier | Supported proof | --verify-backends; exact CPU/RAM/serial/stop comparison gates timing |
| Direct PC CPU loop | x86-64 hosts | Equivalent ADD/SUB/JNZ arithmetic loop, checked result; no emulator overhead |
| Native engine in REPL / JIT | Planned | Debugger still interprets tokens; compiled backend is a separate proof runner |
| Correctness examples | Supported | Diagnostic selftest, Hello-world, 1!..12! table and counted loop |
| Numerical stress/timing | Supported | Fresh-run factorial checks, loop benchmark, warmup exclusion, host/profile reporting |
| Interactive debugger | Supported subset | Step/run/time, breakpoints, registers/flags, RAM edits/protection and binary listing |
| Serial | Mock only | Bounded RX/TX queues and console output; monitor port 0xe9, not a PC UART |
| Windows 11 / WSL / Termux | Host targets | Source build/scripts; Windows/Linux CI; phone interpreter examples measured |
| Cross-platform shared checkout | Supported setup | Separate CARGO_TARGET_DIR for Windows and WSL; use each environment's helpers |
| ESP32 / TTGO | Planned | Current std/host debugger is not an ESP32 firmware build |
| VGA / PS/2 / SD / WLAN | Planned | No device implementation bundled; retain intended WLAN-to-SD workflow later |
| BIOS / DOS / Windows 3.1x boot | Planned | No firmware or operating-system boot yet |
| Paging / A20 / interrupts | Planned | No page tables, A20 gate, privilege transitions or exception delivery |
| x87 / floating point / Mandelbrot | Planned | No x87; signed multiply/shift and image output workload still needed |
| ARM native target / Wine bindings | Planned | No Wine integration, ABI library or general x86-to-ARM runtime yet |

The compiled proof compares registers, EFLAGS, undefined-flag metadata, EIP,
all RAM bytes and RX/TX queues after every instruction for selftest, Hello-world,
factorials and a small loop. It also checks a read-only store fault, branch-limit
fault, changed-code stop, empty RX wait and full TX retry. Fault kinds/metadata
are compared; diagnostic strings are not part of the trace format.

The compiled runner refuses changed instruction bytes. It is not a replacement
for the interpreter's general code-write handling. It emits functions from the
same validated PIL descriptions rather than hand-writing a separate CPU.

Normal debugger binaries need no Rust compiler once built. --compare-backends
requires rustc locally and saves generated source, executable and trace files
under target/backend-comparison (or CARGO_TARGET_DIR). Compilation and process
startup are outside native sample timings. Native dispatch, code guards and
black_box per step remain inside; compare release builds on the same host.

Development tests, history, plans and packaging tools remain on dev. Main holds
only portable build/run source, helper scripts, examples and applicable user docs.
No project-wide licence has been selected; no third-party emulator or OS images
are bundled. An alpha label does not imply Windows/PC or ESP32 compatibility.
