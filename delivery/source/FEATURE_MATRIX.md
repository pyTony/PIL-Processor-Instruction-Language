# PIL 0.28.0-alpha.1 feature matrix

An alpha of the processor-language and host execution proof. It is not yet a
bootable PC emulator. Supported means implemented in the documented prototype;
partial means a deliberately limited subset. See DEBUGGER.md for exact encodings.

| Area | Status | What works / current limit |
|---|---|---|
| PIL data widths | Supported | Explicit 1..64-bit intermediate values; 8/16/32-bit guest operands |
| MOV forms | Supported subset | Register/immediate, ModRM/SIB memory, A0..A3 moffs and C6/C7 /0; independent16/32 address and8/16/32 data sizes; segment-transfer limits apply |
| Arithmetic and flags | Supported subset | ADD, ADC, SUB, SBB, CMP, AND, OR, XOR, TEST, NOT, NEG, INC, DEC; truncated signed IMUL16/32; flag preservation and undefined markers |
| Multiplication | Supported subset | 0F AF and 69/6B truncated signed IMUL16/32, register/memory sources; no implicit double-width forms |
| Shifts | Supported subset | SHL/SAL, SHR, SAR8/16/32; count1, CL, imm8; register/memory; no double shifts |
| Image/archive imports | Bounded read-only subset | FAT16 MBR/superfloppy,ZIP stored/deflate,listing/new-directory extraction and probe deduplication;not FAT12/FAT32/ISO/ZIP64 |
| DOS executable probe | Bounded diagnostic subset | COM/MZ relocation import, original hello, bounded print/exit/memory host shim and first-blocker corpus reports; not full DOS |
| Status-byte transfers | Supported subset | LAHF/SAHF with implicit AH; five status bits and fixed byte bits; OF preserved by SAHF |
| Real-mode control flags | Supported subset | CLI/STI, PUSHF/POPF and PUSHFD/POPFD; distinct traced STI delay; no protected/V86 privilege rules or interrupt delivery |
| Frame teardown | Supported subset | LEAVE16/32 with independent SS size, precise read faults and dynamic SS rebinding; ENTER remains planned |
| DOS memory services | Bounded subset | INT21/AH48/49/4A first-fit single-process conventional MCB arena, visible headers, CF/error codes and PSP extent; no UMB/EXEC/TSR or strategy selection |
| Stack | Supported subset | Register PUSH/POP, immediate PUSH and segment PUSH; independent operand/SS sizes, precise RAM faults; memory stack forms remain planned; real-mode segment POP is supported |
| Segment transfers | Real-mode subset | MOV8C/8E register/word-memory, POP ES/SS/DS/FS/GS; transactional selector/cache updates and SS-shadow state; no protected loads or MOVr32,Sreg |
| Direction flag | Supported subset | CLD/STD via shared PIL; DF bit10 defined, all other flags preserved; MOVS/STOS/LODS use DF |
| Rotates / carry constants | Supported subset | ROL/ROR8/16/32,register/memory,count1/CL/imm8; CLC/STC/CMC. RCL/RCR remain planned |
| Data conversion / addresses | Supported subset | MOVZX/MOVSX8-to16/32 and16-to16/32 (register/memory, including aliases). CBW/CWDE/CWD/CDQ and LEA16/32 preserve flags |
| Register exchange | Supported subset | XCHG8/16/32 register forms86/87 and accumulator91..97; snapshots and aliases; memory/LOCK remain planned |
| Register effects | Supported subset | Snapshot reads; ordered named GPR effects or bound writes; one staged segment-cache change and one RAM store |
| Register views | Supported | 32-bit registers, low 16 bits, AL/AH-style byte views |
| Calls and returns | Near subset | E8 relative and FF /2 register/memory CALL, C3/C2 RET; independent operand/SS sizes and byte cleanup; no far transfers |
| Comparison strings | Supported subset | CMPS/SCAS8/16/32, REPE/REPNE, DF, independent address sizes, source override/fixed ES and entry-flag fault recovery; no interrupt delivery |
| Movement strings | Supported subset | MOVS/STOS/LODS8/16/32, address16/32, DF, source overrides and fixed ES; F3 REP with per-element progress/fault retry; F2 movement prefixes and interrupt delivery remain unsupported |
| Conditional flow | Supported subset | Sixteen predicates, short/near binary Jcc and relative JMP and JCXZ/JECXZ; no indirect/far jumps |
| SETcc | Supported subset | Register and ModRM/SIB memory byte destinations; sixteen conditions; /0 profile restriction |
| Binary decoder | Partial | Documented MOV/arithmetic/Jcc/SETcc/IN/OUT/HLT monitor encodings only |
| Memory addressing | Supported subset | 16-bit ModR/M, 32-bit ModR/M/SIB, size and segment overrides |
| RAM and faults | Supported subset | 4-KiB monitor RAM, little-endian unaligned accesses, atomic single-store retirement |
| Segmentation | Partial | Supplied normalized caches, base/limit/permissions and expand-down checks; no descriptor loads |
| Token interpreter | Supported | Default debugger engine; cached and uncached modes |
| Decode/PIL cache | Supported | Byte/context validation, bounded capacity, statistics; code writes handled on reuse |
| Rust-source instruction emitter | Supported | Validated PIL emitted as Rust functions; generated/interpreted comparisons |
| Compiled Rust guest backend | Experimental | Offline rustc compilation of fixed guarded images; 118 full-state trace cases and loop timings |
| External hardware-derived suites | Planned | SingleStepTests CPU adapter and later test386.asm ROM; no external vectors/ROM executed yet |
| Trace verifier | Supported proof | --verify-backends; exact CPU/RAM/serial/stop comparison gates timing |
| Direct PC CPU loop | x86-64 hosts | Equivalent ADD/SUB/JNZ arithmetic loop, checked result; no emulator overhead |
| Native engine in REPL / JIT | Planned | Debugger still interprets tokens; compiled backend is a separate proof runner |
| Correctness examples | Supported | Diagnostic selftest, Hello-world, 1!..12! table, counted loop and conversion/address demo |
| Numerical stress/timing | Supported | Fresh-run factorial checks, loop benchmark, warmup exclusion, host/profile reporting |
| Interactive debugger | Supported subset | Step/run/time, breakpoints, registers/flags, RAM edits/protection and binary listing |
| Serial | Mock only | Bounded RX/TX queues and console output; monitor port 0xe9, not a PC UART |
| Windows 11 / WSL / Termux | Host targets | Source build/scripts; Windows/Linux CI; phone interpreter and compiled-Rust proof measured |
| Cross-platform shared checkout | Supported setup | Separate CARGO_TARGET_DIR for Windows and WSL; use each environment's helpers |
| ESP32 / TTGO | Planned | Current std/host debugger is not an ESP32 firmware build |
| VGA / PS/2 / SD / WLAN | Planned | No device implementation bundled; retain intended WLAN-to-SD workflow later |
| BIOS / DOS / Windows 3.1x boot | Planned | No firmware or operating-system boot yet |
| Paging / A20 / interrupts | Planned | No page tables, A20 gate, privilege transitions or exception delivery |
| Mandelbrot image workload | Supported proof | Q12 guest, serial-streamed pixels, reference checking, BMP/PGM output through256x256; optional full-image generated-Rust comparison with drain/halt state checks |
| x87 / floating point | Planned | No x87; Mandelbrot uses integer fixed-point |
| ARM native target / Wine bindings | Planned | Fixed proof runs natively on ARM64; no Wine/ABI library or general translation runtime |

The compiled proof compares registers, EFLAGS, undefined-flag metadata, EIP,
all RAM bytes and RX/TX queues after every execution step for selftest, Hello-world,
factorials and a small loop. It also checks a read-only store fault, branch-limit
fault, changed-code stop, empty RX wait and full TX retry. Fault kinds/metadata
are compared; diagnostic strings are not part of the trace format.

The compiled runner refuses changed instruction bytes. It is not a replacement
for the interpreter's general code-write handling. It emits functions from the
same validated PIL descriptions rather than hand-writing a separate CPU.

Normal debugger binaries need no Rust compiler once built. --compare-backends
and --compare-mandelbrot require rustc locally and saves generated source, executable and trace files
under target/backend-comparison (or CARGO_TARGET_DIR). Compilation and process
startup are outside native sample timings. Native dispatch, code guards and
black_box per step remain inside; compare release builds on the same host.

Development tests, history, plans and packaging tools remain on dev. Main holds
only portable build/run source, helper scripts, examples and applicable user docs.
No project-wide licence has been selected; no third-party emulator or OS images
are bundled. An alpha label does not imply Windows/PC or ESP32 compatibility.
