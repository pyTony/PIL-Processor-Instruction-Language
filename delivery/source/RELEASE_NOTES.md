# PIL 0.28.0-alpha.1

- Add CMPS/SCAS8/16/32 and F3 REPE/F2 REPNE with new-ZF termination,count-zero
  guard and bounded element steps. Keep operand/address sizes independent.
- Add generic entry-flag checkpoint/recovery across partial repeats;faults retain
  prior count/pointer progress and restore original EFLAGS/undefined metadata.
- Include checkpoint state in full traces;debugger/DOS paths retain fault updates.
- Add byte memory destinations to SETcc via existing shared select/store effects.
- Windows/Linux CI40562311100:273 tests,118 backend cases (6,577,356 trace bytes),
  existing340,920 native overlap cases,full Mandelbrot and105-file package checks.
- No new ARM/hardware-oracle run. Memory PUSH/POP,SMSW,interrupt delivery and
  full prefix/mode/encoding audit remain separate work.

# PIL 0.27.0-alpha.1

- Add LEAVE16/32 through shared PIL stack load/frame effects; keep SS and
  operand sizes independent,including stack rebinding after segment loads.
- Add bounded DOS INT21/AH48 allocate,49 free and4A resize with visible MCB
  headers,first-fit reuse/coalescing,CF/error results and ownership extent reporting.
- Separate physical640KiB RAM from initial/current program allocation; keep
  loaded-code fetch bounds. Precheck all MCB writes before service commit.
- Windows/Linux CI40536350302:265 tests,103 backend cases (6,089,907 trace bytes),
  existing340,920 native overlap cases,full Mandelbrot and103-file package checks.
- No new ARM/hardware-oracle run. ENTER,full DOS kernel,UMB/EXEC/TSR,interrupt
  delivery and copied-code fetch/real-mode CS-write model work remain separate.

# PIL 0.26.0-alpha.1

- Add real-mode CLI/STI and PUSHF/POPF16/32 with 386 visible/write masks.
- Track STI delay separately from SS; include it in full machine traces and
  retire it across PIL, monitor I/O and DOS calls while retaining REP progress.
- Add JCXZ/JECXZ: address-size count, operand-size target, signed rel8.
- Preserve state on stack/target faults; reject protected/V86 flag contexts.
- Windows/Linux CI40527735310:252 tests and96 backend cases (6,026,172 trace bytes),
  existing340,920 native overlap cases,full Mandelbrot and101-file package checks.
- No new ARM/hardware-oracle run; no interrupt delivery or stack shutdown model.

# PIL 0.25.0-alpha.1

- Add near relative/register/memory CALL and RET with optional immediate byte cleanup.
- Add MOVS/STOS/LODS8/16/32, independent address sizes, DF and segment rules.
- Add F3 REP with bounded element steps, zero-count guard and fault restart.
- Share per-access segment memory, guarded retirement and resume effects across
  the interpreter and generated Rust. Prior REP elements remain committed.
- DOS step budgets include REP elements; completed counts full instructions.
- Windows/Linux CI40511872914:243 tests,84 backend cases (5,816,602 bytes),existing
  340,920 native overlap cases,full Mandelbrot and96-file package checks.
- No new ARM/new-instruction hardware run. Far calls,CMPS/SCAS/F2,interrupt
  delivery and memory PUSH/POP remain separate work.

# PIL 0.24.0-alpha.1

- Add A0..A3 direct-offset accumulator MOV and C6/C7 /0 immediate destinations.
- Reuse shared PIL memory/register effects;keep operand/address sizes independent.
- Check high-byte destinations,SIB/SS defaults,segment overrides,limits,readonly
  memory and truncated encodings without partial state updates.
- Remove three normal-build warnings reported on Termux.
- Windows/Linux CI40500511068 passed235 tests and67 backend cases (5,417,290 bytes),
  existing340,920 native cases,full Mandelbrot and91-file package checks.
- Memory stack operands,CALL/RET and strings/REP remain separate work.

# PIL 0.23.0-alpha.1

- Add shared PIL selector snapshots and staged real-mode segment-cache loads.
- Add MOV8C/8E register/word-memory and POP ES/SS/DS/FS/GS; POP uses old SS state.
- Record SS inhibition state across successful retirements,including DOS bridges;
  no external interrupt/NMI/debug delivery or protected segment loads is claimed.
- Compiled runners use runtime caches/selectors and choose the current SS-size variant.
- Full traces include segment state;debugger reset restores the initial image context.
- Reject MOVr32,Sreg pending legacy undefined-upper-half policy.
- Windows/Linux CI40498283191 passed231 tests,62 backend cases (5,336,578 bytes),
  existing340,920 native overlap cases,full Mandelbrot and91-file package checks.
- Actual FreeDOS image:segment MOV/POP blockers gone;JEMFBHLP and MOVEXBDA usage
  paths print and exit0. These are not driver/device execution successes.

# PIL 0.22.0-alpha.1

- Add snapshot GPR reads and ordered staged GPR writes to PIL and both backends.
- Add register PUSH/POP (50..5F,FF/6,8F/0),immediate PUSH (68,6A),and all six
  segment PUSH forms,using explicit visible selectors and SS stack-size state.
- Preserve flags,handle SP/ESP aliases,and retain CPU/RAM state on access faults.
- Cache identity includes selectors and SS attributes;implicit stack accesses use SS.
- Windows/Linux CI40487360431 passed223 tests and56 full-state backend cases,
  5,133,394 trace bytes,full Mandelbrot and standalone/native-package checks.
- Existing340,920 native CPU cases remain green;these are not new stack hardware cases.
- Memory stack forms,segment POP/MOV,CALL/RET and interrupts remain separate work.

# PIL 0.21.0-alpha.1

- Add CLD/STD through shared PIL descriptions and explicit DF (EFLAGS bit 10).
- Preserve all other flags, registers and markers; define DF and clear its marker.
- Add independent full-state, generated Rust, decoder and ABI-safe hardware tests.
- Windows/Linux CI40476098533 passed215 tests,50 backend cases (5,008,114 trace bytes),
  98,304 independent Python direction cases and existing340,920 native overlap cases.
- Full Mandelbrot, standalone source and native-package checks passed on both hosts.
- Ignore local images/ in source deliveries; refresh the ISA TODO baseline.
- No string instructions, stack support or interrupt control added by this slice.

# PIL 0.20.0-alpha.1

- Read FAT16 images and ZIP stored/deflate packages without new runtime dependencies.
- List/extract files and directly probe executable candidates plus one package level.
- Deduplicate identical container programs;validate CRC,chains,paths and bounded expansion.
- Add synthetic FAT/ZIP,independent zlib vectors and CLI/package interoperability checks.
- Windows/Linux CI40470152015 passed210 tests and CLI/package checks.
- Actual supplied FreeDOS image extracted byte-identically;151 DOS programs imported.
- Correct wrapped MZ CS/SS relocation before physical entry/stack validation.

# PIL 0.19.0-alpha.1

- Add bounded COM and MZ EXE import with relocations and minimal PSP/entry state.
- Add DOS hello fixtures and explicit print/exit host service bridges.
- Report first CPU encoding versus DOS-service blockers and reached instruction mix.
- Add isolated REPL probing, malformed-input and independent CLI checks.
- Windows/Linux CI40460972486 passed200 tests,independent DOS CLI checks,
  both hello formats from standalone/native packages,and all existing regressions.

# PIL 0.18.0-alpha.1

- Add LAHF/SAHF through shared PIL descriptions and implicit AH bindings.
- Preserve untouched state and handle status-byte fixed bits and metadata.
- Extend independent, generated-Rust and CPUID-gated native CPU checks.
- Include the current PIL reference in runtime sources and native packages.
- Windows/Linux CI40446250754 passed188 tests and340,920 native CPU cases
  per host;49 full-state backend cases,full256x256 Mandelbrot and package checks passed.

# PIL 0.17.0-alpha.2

- Add CMC (F5) through shared PIL semantics and binary decoding.
- Preserve untouched state; document concrete incoming-CF metadata policy.
- Add prefix/fetch-fault, generated-Rust and native CPU oracle coverage.
- Windows/Linux CI40443065185 passed184 tests and332,728 native CPU cases per host;
  all48 backend cases and full256x256 Mandelbrot comparisons passed.

# PIL 0.17.0-alpha.1

- Add register XCHG8/16/32 and accumulator exchange forms.
- Extend PIL with staged write src and write_if src; reads remain snapshots.
- Reject conflicting overlapping writes before any memory or CPU commit.
- Expand compiled full-state proof to48 cases.
- Memory XCHG/LOCK,multiple implicit registers and multiple RAM stores remain planned.

# PIL 0.16.0-alpha.1

- Add ROL/ROR8/16/32 with count1,CL,imm8 and register/memory destinations.
- Add CLC/STC; preserve untouched registers,flags and undefined markers.
- Express all semantics in PIL; retain zero/full-turn flag and precise RMW-fault rules.
- Expand compiled full-state proof to45 cases.
- RCL/RCR,CMC and double shifts remain outside this increment.

# PIL 0.15.0-alpha.3

- Accept16-to16 MOVSX/MOVZX register and memory sources using existing PIL conversions.
- Add demo extensions with a built-in checker and readable instruction listing.
- Preserve upper register halves, flags, word-source access checks and precise faults.
- Expand compiled proof to39 cases and independent extension/address checks to263,190.

# PIL 0.15.0-alpha.2

- Report when the current program has no built-in check, with supported images.
- Keep specific verification failures for supported but unfinished programs.
- Explain runtime/dev automated-test placement in the runtime README.
- Clarify assignment/conditional formatting in the debugger.

# PIL 0.15.0-alpha.1


- Add MOVZX/MOVSX byte-to16/32 and word-to32, register/memory sources.
- Add CBW/CWDE, CWD/CDQ and LEA with independent operand/address sizes.
- Preserve flags and source-width memory checks; LEA never accesses data memory.
- Permit independently typed source bindings in PIL and generated functions.
- Expand compiled per-instruction proof to34 cases and add exhaustive extensions.

# PIL 0.14.0-alpha.2

- Compare full-size Mandelbrot generated Rust and cached PIL interpreter.
- Gate comparison with the existing 26-case per-instruction verifier.
- Check every pixel, full state at serial drain/halt boundaries and step/retry counts.
- Add -Compare / --compare to render helpers; retain byte guards and black_box.
- Report single-run timing scope and preserve pixel/state/source artifacts.

# PIL 0.14.0-alpha.1

- Add original Q12 Mandelbrot guest and independent reference checking.
- Stream pixels through mock serial; save BMP/PGM, guest binary and report.
- Add release render helpers; support256x256 and small debugger demo.
- Decode near Jcc and short/near relative JMP using existing PIL flow effects.
- Expand compiled full-state proof to26 cases including tiny image and faults.
- Add independent Python raster/BMP checks and full-size hosted render checks.

# PIL 0.13.0-alpha.3

- Show encoded bytes, registers/immediates, memory operands and branch targets.
- Add bounded list COUNT / list ADDRESS COUNT; mark current EIP.
- Make original numeric fixture listings use real register names.
- Preserve live-byte decoding and read-only inspection behavior.

# PIL 0.13.0-alpha.2

- Add mem8/mem16/mem32 typed debugger dumps with printable ASCII.
- Support little-endian grouping, unaligned reads and optional unsigned decimal.
- Validate full ranges/counts and retain the original mem command format.

# PIL 0.13.0-alpha.1

- Add generic total shifts, unsigned comparison and conditional flag effects.
- Decode SHL/SAL, SHR, SAR8/16/32: count1, CL and imm8, register/memory.
- Preserve flags and undefined markers at masked zero; classify undefined flags.
- Expand full-state verification to23 cases, including protected zero-count RMW.
- Include signed IMUL from0.12 and a prominent post-build debugger quick start.

# PIL 0.12.0-alpha.1

- Add generic signed extension and modular multiplication PIL operations.
- Add signed IMUL16/32, 0F AF plus 69/6B immediate forms and memory sources.
- Define CF/OF overflow and undefined PF/AF/ZF/SF in the description.
- Expand compiled/interpreted verification to eighteen full-state cases.
- Test generic operation widths through64 bits and signed product edge cases.
- Document Windows PC comparison and separate Windows/WSL target directories.

# PIL 0.11.0-alpha.1

- Add OR, XOR, TEST, NOT, NEG, INC, DEC and NOP descriptions and binary decoding.
- Support both register/memory directions for all eight binary ALU operations.
- Add accumulator immediates and 80/81/83 immediate groups with sign extension.
- Add unary memory/register and TEST immediate forms at 8/16/32 bits.
- Expand compiled/interpreted verification to fourteen full-state cases.
- Preserve precise flag metadata and fault-before-retirement behavior.

# PIL 0.10.0-alpha.2

- Verifier failures identify case, step, instruction EIP and the differing CPU,
  stop metadata, RAM or serial field.
- Truncated or malformed traces fail verification; both trace files are retained
  on a mismatch under the reported artifacts directory.
- The decoder and execution engines have the same guest instruction coverage.

# PIL 0.10.0-alpha.1

First alpha candidate of the PIL host execution proof. See FEATURE_MATRIX.md
for implemented subsets and planned work; this release does not boot a PC OS.

- Add an offline compiled Rust guest runner generated from the same validated PIL.
- Compare complete instruction traces across nine success/fault/wait cases.
- Gate timings with --verify-backends, then compare generated Rust/interpreter.
- Add a direct x86-64 PC arithmetic baseline via --cpu-bench.
- Add source/native artifact output and Linux/Windows comparison helper scripts.
- Document shared Windows/WSL source folders with separate build directories.
- Preserve the cached interpreter as the interactive debugger engine.

Native comparison requires rustc. Normal packaged debugger use does not.
Compilation and process startup are excluded from native sample timings; guards
and black_box per step are included. Native speed on the phone is not yet known.
No JIT, arbitrary compiled binary execution, OS boot or ESP32 firmware is added.

# PIL 0.9.0

- Cache decoded binary instructions and their compiled PIL descriptions.
- Revalidate bytes and segment context before reuse, including guest code writes.
- Add cache stats/on/off/clear commands and uncached benchmark comparisons.
- Preserve precise stops, current memory permissions and runtime register values.
- Keep execution in the token interpreter; generated Rust remains a separate API.

Cached samples start with an empty cache. No simulated cycle model or JIT is
introduced. See DEBUGGER.md for timing scope and cache controls.

# PIL 0.8.0

- Add real guest programs computing 1!..12! and a 65,536-iteration integer loop.
- Check every factorial RAM result and final loop registers/flags.
- Add time, bench and stress commands with bounded counts and fresh-state samples.
- Add release benchmark helper scripts for Termux/Linux and Windows.
- Include numeric binaries/listings in source builds and native deliveries.

Timing measures host interpreter throughput including decoding and PIL
compilation; it does not estimate original 386 clock cycles. Mandelbrot and
floating-point arithmetic are not included in this version.

# PIL 0.7.1

Add `demo hello`, a 57-byte guest Hello-world program with no input required.
Build scripts also write hello.bin. Successful serial output appears in the
debugger console and is available through serial tx.

# PIL 0.7.0

- Run an included 204-byte 386 guest program in the interactive debugger.
- Inspect registers, flags, RAM, instruction listings and breakpoints.
- See serial output directly in the console; inject input with serial rx.
- Use 16/32-bit effective addresses, segment overrides and segment bounds checks.
- Build/run with the supplied Termux/Linux and Windows scripts.
- Generate Rust source from embedded PIL descriptions with --emit-rust.

The program checks arithmetic/flags, conditional flow, register views, segmented
memory and mock serial. Expected success is DX=600D and TX=41. This verifies
the supported execution path; the debugger's instruction profile is limited.
See DEBUGGER.md for exact encodings and limitations.

This prototype does not boot an operating system or provide PC hardware yet.
