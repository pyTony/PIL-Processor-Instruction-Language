# Changes

## 0.7.0 (development)

- Add 16/32-bit effective addresses, segment overrides and cached-segment checks.
- Share segmented execution with generated Rust, preserving staged stores on faults.
- Add a small binary decoder and a reproducible executable 204-byte selftest.
- Echo successful guest serial output in the debugger without draining it.
- Add compile/debug helpers for Termux/Linux and Windows and native ZIP packaging.
- Use dev for development and main for reviewed deliveries; add host CI.
- Supply 92 Rust tests, including 153 segmented generated/interpreted cases.

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
