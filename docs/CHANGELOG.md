# PIL 0.30.0-alpha.1

- Add memory POP (8F /0) with independent SS reads and destination addresses
  evaluated after SP/ESP advances. Operand, address and SS sizes are independent;
  faults commit no SP or store effects. The family-specific 16-bit SP wrap case
  with an ESP-based address is rejected pending legacy 386 hardware evidence.
- Add SMSW (0F 01 /4) word register/memory forms and explicit traced MSW state.
  PE follows normalized mode. Initial ET=1 is a declared profile choice.
  Wide register forms and CR0/LMSW writes remain separate work.
- Add DOS INT 21/AH=30 with a declared 6.22 compatibility profile in PSP:0040,
  OEM FF or non-ROM flags, zero serial and preserved upper register halves/flags.
  Other unimplemented services still stop.
- Windows/Linux CI 40582461490: 297 tests and 149 full-state backend cases
  (6,870,394 matching trace bytes), standalone runtime and package checks.
  Twelve new expected-state tests and fourteen backend cases cover this change.
- No new full FreeDOS image, ARM or legacy real-mode hardware run is claimed.

# PIL 0.29.0-alpha.1

- Add real-mode CB/CA RETF16/32 with independent SS sizes and byte cleanup.
- Restore CS selector/cache,IP and SP together;check the complete frame and
  new code limit before commit. Protected/V86 returns remain unsupported.
- Anchor imported fetch windows to physical addresses across CS changes.
  Generated fixed-image dispatch uses physical addresses and live logical IP.
- Local Windows:285 tests passed;135 full-state backend cases include RETF,
  CS aliases,branching/data access after return and precise failure states.
- No new real-mode hardware/ARM/full FreeDOS image run is claimed.

# PIL 0.28.0-alpha.2

- Correct ordinary real-mode CS data permissions in monitor and COM/MZ loaders.
  CS overrides may write data/code; protected Code caches still reject stores.
- Preserve segment limits, RAM protection and loaded-image fetch boundaries.
  Cached modified instruction bytes are decoded again before reuse; the fixed
  compiled-image backend still stops at a changed-code guard.
- Add COM/MZ self-modifying execution, cached instruction invalidation and wider
  independent SETcc condition/addressing checks. Local Windows: 277 tests passed;
  121 interpreter/generated full-state cases matched 6,594,404 trace bytes.
- This increment does not claim a new full FreeDOS-image or ARM execution run.

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

## 0.16.0-alpha.1

- Add ROL/ROR8/16/32 with count1,CL,imm8 and register/memory destinations.
- Add CLC/STC; preserve untouched registers,flags and undefined markers.
- Express all semantics in PIL; retain zero/full-turn flag and precise RMW-fault rules.
- Expand compiled full-state proof to45 cases.
- RCL/RCR,CMC and double shifts remain outside this increment.

# Changes

## 0.15.0-alpha.3

- Accept16-to16 MOVSX/MOVZX register and memory sources using existing PIL conversions.
- Add demo extensions with a built-in checker and readable instruction listing.
- Preserve upper register halves, flags, word-source access checks and precise faults.
- Expand compiled proof to39 cases and independent extension/address checks to263,190.


## 0.15.0-alpha.2

- Fix unsupported-program check diagnostics; add all8 demo regression coverage.
- Explain runtime/dev test placement and clarify5 formatting hotspots.

## 0.15.0-alpha.1

- Add MOVZX/MOVSX byte-to16/32 and word-to32, register/memory sources.
- Add CBW/CWDE, CWD/CDQ and LEA with independent operand/address sizes.
- Preserve flags and source-width memory checks; LEA never accesses data memory.
- Permit independently typed source bindings in PIL and generated functions.
- Expand compiled per-instruction proof to34 cases and add exhaustive extensions.


## 0.14.0-alpha.2

- Compare full-size Mandelbrot generated Rust and cached PIL interpreter.
- Gate comparison with the existing 26-case per-instruction verifier.
- Check every pixel, full state at serial drain/halt boundaries and step/retry counts.
- Add -Compare / --compare to render helpers; retain byte guards and black_box.
- Report single-run timing scope and preserve pixel/state/source artifacts.


## 0.14.0-alpha.1

- Add Q12 guest Mandelbrot, reference checking, serial draining and BMP/PGM output.
- Add near Jcc/relative JMP and small interactive image demo.
- Add seven tests (155 total),26 compiled trace cases and independent image oracle.

## 0.13.0-alpha.3

- Show instruction operands, encoded bytes, branch targets and current EIP in list.
- Add list COUNT / list ADDRESS COUNT; make original numeric fixture listings readable.
- Add four listing regression tests (148 total including typed dumps).

## 0.13.0-alpha.2

- Add mem8/mem16/mem32 dumps, printable ASCII and optional unsigned decimal.
- Validate full ranges/counts before output; preserve legacy mem format.
- Add four debugger regression tests (144 total).

## 0.13.0-alpha.1

- Add total shifts/ult/flag_if PIL operations, interpreter and generated Rust.
- Add SHL/SAL, SHR, SAR register/memory/count forms at8/16/32 bits.
- Add six tests (140 total), independent shift oracle and23-case verifier.
- Put direct post-build debugger usage and a linked docs index at the top of README.
- List all seven demos consistently, including factorials/loop, in README and guide.

## 0.12.0-alpha.1

- Add generic sext/mul tokens, interpreter and Rust emission.
- Add truncated signed IMUL16/32 register, memory and immediate encodings.
- Add six tests (134 total), independent signed oracle and 18-case verifier.
- Initialize verifier data before applying read-only protection.

## 0.11.0-alpha.1

- Expand binary ALU directions/immediates and eight instruction descriptions.
- Add six semantic/decoder tests, fourteen-case compiled verifier and logical oracles.
- Update exact supported encodings and references; no PC boot or device changes.

## 0.10.0-alpha.2

- Identify first verifier divergence by case, step, instruction EIP and state field.
- Reject malformed/truncated traces and retain both traces on mismatch.
- Add two verifier regression tests; record alpha.1 phone benchmarks.

## 0.10.0-alpha.1

- Generate and compile fixed-image Rust guest runners using shared PIL emission.
- Add nine-case byte-for-byte instruction traces including faults and serial waits.
- Compare generated Rust and cached interpreter with separate compilation timing.
- Add alpha feature matrix, comparison helpers and shared Windows/WSL setup.
- Add four tests (120 dev tests), retaining all prior emitted-code comparisons.

## 0.9.0

- Cache decoded instructions/PIL with bounded storage and shared immutable programs.
- Revalidate encoded bytes and full image/segment context before every hit.
- Add cache controls/stats and uncached benchmark mode; remain a token interpreter.
- Add 15 regression tests (116 total), including cached/uncached step comparisons,
  guest code modification, runtime permission checks and the full 65,536 loop.

## 0.8.0

- Add guest factorial table and counted loop, with exact output/state checks.
- Add bounded time/bench/stress commands and optimized benchmark scripts.
- Include numeric fixtures in runtime exports and native archives.
- Add seven debugger regression tests (101 development tests total).

## 0.7.0 (development)

- Add 16/32-bit effective addresses, segment overrides and cached-segment checks.
- Share segmented execution with generated Rust, preserving staged stores on faults.
- Add a small binary decoder and a reproducible executable 204-byte selftest.
- Echo successful guest serial output in the debugger without draining it.
- Add compile/debug helpers for Termux/Linux and Windows and native ZIP packaging.
- Use dev for development and main for reviewed deliveries; add host CI.
- Pass 92 Rust tests on hosted Ubuntu/Windows, including 153 segmented comparisons.
- Pass the executable selftest in 63 steps and host build/debug/package checks.

## 0.6.0

- Add flat RAM with little-endian 8/16/32-bit reads, stores and read-only ranges.
- Add PIL read_address/load/store and load_ram/store_ram/add_ram descriptions.
- Defer the single supported RAM store until retirement; preserve CPU/RAM on faults.
- Add structured emulator MemoryAccess errors, distinct from x86 #PF.
- Add debugger memory demo and mem/poke/protect commands.
- Add 19 Rust tests (65 total), including 560 generated/interpreted memory cases.
- Add independent Python memory oracle; update source/verification history.
- New Rust build is not yet run in the authoring environment.


## 0.5.0

- Add bounded RX/TX mock serial with nonblocking reads/writes and atomic injection.
- Add interactive REPL, counter/echo demos, stepping, bounded runs, breakpoints,
  registers/flags/EIP, reset and serial inspection. Harness serial is not UART I/O.
- Add 18 tests (46 total); new Rust build awaits verification.
- Reorganize instructions, tests, tools and documentation; add Git rules and push notes.
- Preserve Cargo.lock/target and the existing ~/pil_proof project.
- Add CLAUDE.md continuation guide and keep provenance/history/checksums.


## 0.4.0

- Add guest EIP, explicit execution context and signed decoded displacements.
- Describe all sixteen Jcc predicates using the same condition definitions.
- Check taken targets against an inclusive supplied protected CS limit;
  return restartable #GP(0) metadata without publishing staged effects.
- Preserve legacy execution APIs for descriptions without flow.
- Add eight Rust tests (28 total), including 12,288 generated-flow comparisons.
- Add independent Python Jcc/context checks and a small decoded loop test.
- Deliver a guarded in-place patch for ~/pil_proof; leave Cargo.lock/target alone.
- Plan serial mock + REPL debugger next, tests/ directory later, original TTGO
  ESP32 compilation after host/embedded separation.
- Record both user-confirmed v0.3 test runs and continue reference tracking.


## 0.3.0

- Require a defined/preserved/undefined classification for every arithmetic flag.
- Track undefined outputs separately from concrete flag values.
- Add AND, sixteen byte-only SETcc predicates, select and write_if.
- Add six Rust tests; extend differential checks to metadata and new operations.
- Record v0.2's successful 14-test Termux result.
- Add revision history, JSON evidence records and source checksums.
- Preserve reference/attribution tracking; no third-party code imported.


## 0.2.0

- Added standalone SUB, CMP, ADC and SBB descriptions at 8/16/32 bits.
- Added generic modular subtraction and incoming-flag read tokens to the
  validator, interpreter and Rust emitter.
- CMP has no destination write; ADC/SBB use the incoming CF snapshot.
- Extended emitter CLI: --emit-rust WIDTH [add|sub|cmp|adc|sbb]. Existing
  --emit-rust WIDTH still selects ADD.
- Added seven Rust tests; expanded generated-source comparison to 15 variants.
- Added optional independent Python description checks.
- Added a maintained references/credits register distinguishing consulted
  material from imported code and tracking licence/attribution findings.
- No x86 decoder, memory, PC devices or ESP32 port added in this milestone.

## 0.1.0

Initial ADD proof. User reported all seven Rust tests passing in Termux.
