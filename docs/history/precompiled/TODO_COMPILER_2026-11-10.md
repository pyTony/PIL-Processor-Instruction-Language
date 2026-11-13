> **Historical (removed 13 November 2026).** This page describes tools that
> built an executable for one DOS program (shadow `pil-<program>`, standalone
> `<PROGRAM>.exe`, `Dine_<program>` launchers). They were removed: PIL and Dine
> are program-independent libraries, used the same everywhere. The commands
> below no longer exist. What was kept for later integration is listed in
> [BLOCK_COMPILER.md](../../development/BLOCK_COMPILER.md); measurements here
> remain as recorded.

# Project TODO

Updated 10 November 2026. This is the current project handoff and work order.
The compiler track is the focus of this chat; CPU and DOS compatibility remain
separate dependencies for broader application coverage.

## Current compiler increment — work from this chat

- [x] Compile the supported real-mode COM subset into self-contained Rust and
  native executables using the existing PIL instruction semantics.
- [x] Discover direct branches, loops and bounded near-call/return contexts;
  reject unsupported reachable code and overlapping instruction boundaries.
- [x] Summarize direct-call returns by entry/AH/operand width instead of
  expanding call strings, allowing recursion with the existing runtime stack.
- [x] Support ordinary RAM reads/stores/RMW, register/immediate PUSH, register
  POP, and near CALL/RET16/32 in the standalone subset.
- [x] Keep precise instruction faults, retired prefixes and return-site checks.
- [x] Protect translated bytes in standalone artifacts, which have no interpreter.
- [x] Verify linked `pil.exe` local fallback after code writes: changed sites use
  Dine; unchanged sites, including those on the same page, stay native eligible.
- [x] Extract common Site/Profile definitions, instruction-module/call emission,
  the 16-instruction block planner and block emitter into `src/dos_codegen.rs`.
- [x] Reuse that pipeline from both Dine and standalone runtime adapters.
- [x] Dispatch standalone production execution by blocks; keep single-instruction
  execution as a test reference. Standard Dine already has bounded block chaining.
- [x] Partition primary blocks and retain single-instruction alternate entries,
  replacing duplicated full suffixes. Profiled indirect targets become leaders.
- [x] Share the existing DOS output and PSP command-tail rules.
- [x] Add a separate Unicode/CP437 boundary, including Finnish command lines and
  byte-based PSP length validation. Default standalone stdout to UTF-8.
- [x] Add `PIL_DOS_RAW_OUTPUT=1` for raw CP437 stdout, suitable behind a Telnet
  server connected to a CP437 terminal. Transport framing stays with the server.
- [x] Add the original ANSI Mandelbrot, a black interior, a reproducible COM/NASM
  builder, and a Python/Dine/native comparison of all 1896 coloured cells.
- [x] Add reproducible LLVM/assembly flag probes and an in-process frame benchmark.
- [x] Rebuild `argument-demo.exe` with Finnish input and selectable output encoding;
  keep a separate `ansi-mandel-blocks.exe` and retain the earlier timing baselines.

Local checks before the entry-plan increment: 694 Rust tests passed, 2 ignored; 49 standalone
fixtures; 12 argument cases checked in UTF-8 and raw CP437 modes; generated
memory/stack/prefix-fault tests; linked self-modification fallback; ANSI output
matched against Dine and an independent arithmetic reference. These are local
Windows results. Hosted Windows/Linux CI for the combined increment is pending.

### Integration work still open for my changes

- [ ] Review and stage the intended compiler/runtime/example changes separately
  from other edits already in the working tree. Preserve the other work.
- [ ] Run the configured Windows/Linux CI for the final selected change set,
  including standalone generation, local fallback, encoding and ANSI Mandelbrot.
- [ ] Verify the standalone runtime-source export/package after introducing the
  shared compiler and encoding modules; update the selected-file checksums.
- [ ] Publish a coherent development increment on `dev`, with its test evidence.
  Prepare any runtime/main delivery separately; merges remain the user's action.
- [ ] Label regenerated demos with their compiler/source identity so subsequent
  benchmarks compare known builds. Preserve old baselines intentionally.

## 1. Compiler optimization — next work

### 1.1 Reduce overlapping block generation

- [x] Identify actual block leaders, call/return continuations, service boundaries
  and required alternate entries instead of duplicating every full suffix.
- [x] Preserve entry at supported instruction starts and Dine's individual-site
  fallback for small budgets or unavailable block entries.
- [x] Retain code/context guards and boundaries at stores and machine changes.

Acceptance: existing differential/fault tests pass; report executable size,
generated function count, static code duplication and warm runtime together.

Completed locally 10 November: Mandelbrot now uses 22 primary blocks and 30
single-instruction alternate entries. All 52 entry addresses remain available;
emitted instruction bodies fell from 134 to 82. Executable size fell from
540672 to 517632 bytes (4.3%). The same in-process benchmark measured medians
of 8.632 versus 8.448 ms/frame; the small timing difference is within the sample
spread and is not presented as a significant speedup. Code-size reduction is
the demonstrated benefit. Generated Dine guards/budgets/local fallback, all 49
standalone fixtures, 12 UTF-8/raw CP437 input cases and the ANSI reference pass.
An interior-entry and prefix-fault test checks the new entry contract.
Final local Rust suite for this phase: 698 passed, 2 ignored. The observed
indirect-target regression also passes. Hosted CI remains pending.

### 1.2 Flag liveness and materialization

- [ ] Record which flags each PIL operation reads, defines, preserves or makes
  undefined; track undefined-flag metadata alongside the visible values.
- [ ] Determine liveness across block edges and eliminate computations whose
  results are overwritten before observation.
- [ ] Materialize exact state at faults, interrupts, services, debugger/trace
  checkpoints and other observation boundaries.
- [ ] Cover INC/DEC preserving CF, ADC/SBB carry dependencies, shifts/count zero,
  partial flag writes, and multiply/divide undefined-flag policy.
- [ ] Extend the LLVM probes with mixed operations, condition chains, memory-fault
  boundaries and representative hot blocks rather than only synthetic ADD chains.

Acceptance: independent flag expectations and interpreter/generated comparisons
remain valid at observation points; measure emitted code and dynamic workload
cost, rather than equating LLVM intrinsic counts with CPU cycles.

### Next goal program: FACTOR.COM

- [x] Compile the existing original FACTOR.COM without rewriting its algorithm.
- [x] Admit scalar LODSB/W/D and checked DIV/IDIV through their existing PIL
  effects; keep REP strings and standalone interrupt-handler delivery separate.
- [x] Compare default/boundary/invalid arguments and exact factorial output
  through 99! with Dine and Python. Test precise division/load faults.
- [x] Use its decimal-digit multiplication, quotient/remainder and formatting
  loops as another optimization workload. This is numeric digit carry; it does
  not replace dedicated ADC/SBB carry-flag chain tests.
- [ ] Export an inspectable program-level PIL/block intermediate with bindings
  and control-flow context. Current CLI output is Rust; validated PIL IR stays
  in memory. Define round-trip semantics before treating a listing as source.

Factor audit, 10 November: `tools/audit_dos_codegen.py` now accepts `--program`
and repeatable `--argument`, records source/program identities, and checks full
output equality before timing saved/current builds. `FACTOR.COM --argument 99`
produces 7634 bytes matching Python's 100 exact values. Local optimized build:
528896-byte executable, 87 block functions; 20 runs/sample, four samples,
first sample excluded, median 1.773 ms/run. Includes RAM/runtime initialization;
excludes process startup and terminal output. This is a baseline, not a speedup.
The added ADCx8/SBBx8 and ADC–INC–ADC/SBB–DEC–SBB probes retain only one
parity calculation each, but x64 assembly still uses widened carry arithmetic
and shifts. Inspect and improve that lowering before adding broad manual flag
liveness machinery. Existing ADD/ADC probes do not establish compact lowering
for longer carry chains.

Local checkout fast-forwarded to origin/dev `29a78e8` on 10 November after
preserving tracked edits in a safety stash. Resolved the shared-codegen conflict
without losing the local implementation; other edits and untracked files remain
in place. This does not publish the outstanding compiler increment.

### 1.3 Make better use of host arithmetic flags

Completed narrow lowering increment, 10 November: `src/rust_codegen.rs`
recognizes the validated widened add/sub-with-carry dataflow and emits portable
natural-width overflow operations. It preserves all SSA values and the PIL flag
formulas, including undefined metadata and transactional commits. No opcode-name
special cases, inline assembly or separate flag-liveness pass were added.
LLVM now emits native ADC/SBB chains; final-branch-only eight-operation probes
have zero parity computations. Mixed INC/DEC chains retain incoming CF correctly.
This supersedes the widened-carry observation above. It demonstrates better
lowering and unused-flag elimination, not a measured general application speedup.
Factor has no ADC/SBB workload; its small timing variation is not evidence for
this change. Keep broad manual flag liveness deferred until workload evidence
justifies it.

Recursive goal completed: original `RECURFIB.COM` / `recurfib.exe`, arguments
0..20 (default 12), computes both recursive branches with BX/CX saved on the
guest stack. Native output matches iterative Python and Dine; tests check each
recursive return's BX/CX/SP, final stack balance, and a separate 65-frame call
chain. Static summary analysis retains the 65,536-state limit, exact return
width/service checks and runtime return-target protection. Local checks:
702 Rust tests passed, 2 ignored; 49 standalone fixtures and 12 encoding cases
pass; exported 222-file runtime source builds. Hosted CI remains pending.

- [ ] Check generated assembly before adding host-specific lowering: the current
  x64 probes already use ADD/ADC and host parity/condition instructions.
- [ ] Keep flag-producing operations and their consumers adjacent where possible.
  Avoid guest-FLAGS packing between operations when it is not observable.
- [ ] Evaluate optional x64 lowering for remaining expensive flag materialization,
  with the portable Rust/PIL path retained for other hosts.
- [ ] Match guest operand widths: host 8/16/32-bit forms on x64 can provide the
  corresponding arithmetic flags; 64-bit forms are not interchangeable with them.
- [ ] Treat host-call clobbers, guest control flags and instruction-specific flag
  behavior explicitly; never assume arbitrary Rust preserves host FLAGS.

Acceptance: byte-width exhaustive cases and wide boundary/randomized chains
cover defined, preserved and undefined state, including intervening clobbers.

### 1.4 Block connections and other optimizations

- [ ] Share more of the region-dispatch/chaining machinery between runtime
  adapters. Reuse Dine's existing bounded chaining rather than rebuilding it.
- [ ] Evaluate direct links for proven hot edges to reduce repeated guest-IP
  dispatch, preserving Dine's budgets, interrupt checks and invalidation rules.
- [ ] Assess constant propagation, dead temporaries, redundant bindings/address
  calculations and safe register-value reuse; first inspect what LLVM removes.

### Measured starting point

Local x64 Rust 1.99.1 with `-O -Ccodegen-units=1`:

| Observation | Result |
|---|---|
| Eight consecutive ADDs, final flags observable | One optimized parity calculation remains |
| ADD followed by JE/JG, only result/branch observable | No parity calculation remains |
| ADD followed by ADC / INC | Required carry dependencies remain |
| Mandelbrot instruction helpers / block functions | 47 helpers before; 0 helpers and 52 block functions after |
| Executable size, saved baseline / shared blocks | 506368 / 540672 bytes, about 6.8% growth |
| In-process Mandelbrot median, baseline / shared blocks | 12.094 / 8.604 ms per frame, about 1.41x |

The frame benchmark includes RAM initialization/runtime setup, excludes process
startup and terminal output, and uses 100 frames per sample with alternating
build order and warmup exclusion. It compares whole builds, not just flag code.
Static parity sites increased with duplicated block entries; this is not a
dynamic count. See `tools/audit_dos_codegen.py` and its JSON/IR/assembly output.

## 2. Broader standalone compilation and DOS runtime coverage

- [x] Reuse the existing DOS MZ loader/relocations at compilation time; remove
  COM-specific address/entry assumptions from discovery and initialization.
- [ ] Discover and validate indirect jump/call targets and jump tables. Unknown
  targets must have an explicit policy; do not decode arbitrary data as code.
- [ ] Expand stack/segment/far-transfer forms with independent sizes and fault
  tests, keeping discovery/context identity correct.
- [ ] Extract/reuse existing Dine keyboard and buffered-line services for headless
  stdin, including input waits, EOF, echo and prompt flushing.
- [ ] Extract/reuse Dine file/handle/directory services, environment and process
  launching instead of implementing a second DOS service set.
- [ ] Define standalone interrupt/device synchronization boundaries before adding
  BIOS input, timer handlers, graphics or port-dependent applications.
- [ ] Specify overlays/generated-code coverage. Linked Dine can interpret changed
  code locally; pure standalone artifacts currently reject translated-code writes.
- [ ] Define runtime compatibility/versioning and artifact identity as reusable
  standalone services grow.

Acceptance: original fixtures plus authorized application paths; compare actual
results with Dine and independent references. Block compilation alone does not
establish support for arbitrary DOS programs.

## 3. CPU/protected-mode and full Dine compatibility

Protected-mode increments 1–6 are recorded as implemented: descriptors, paging,
IDT delivery, privilege transitions and V86. Remaining work is tracked in the
[protected-mode plan](docs/development/PROTECTED_MODE_PLAN.md).

- [ ] Implement task switching: TSS/task gates, saved/loaded state, busy bits,
  NT/backlink, CR3/TS and precise faults, including generated-backend coverage.
- [ ] Resolve the remaining ENTER fault and complete the remaining `test386`
  milestones; record ROM revision, configuration and measured POST results.
- [ ] Add Dine memory above 1 MiB and the planned BIOS/CMOS memory services.
- [ ] Implement the planned XMS driver; define/test extender compatibility paths.
- [ ] Keep DPMI/VCPI/EMS ownership explicit: these are not currently supplied
  as Dine services merely because parts of protected/V86 execution exist.
- [ ] Reconcile historical real-mode/ISA backlog entries with current code and
  rerun external suites before calling old blockers still open or completed.

CPU/runtime support and native compilation coverage must be reported separately.

## 4. Modularity and maintainability

- [x] Keep shared block generation free of loader/session dependencies.
- [x] Keep profiling discovery and static discovery in separate frontends.
- [x] Keep runtime policy in adapters and text encoding in its own module.
- [x] Separate static DOS translation (`dos_translate`) and target-neutral
  site/block planning (`dos_blocks`) from Rust rendering (`dos_codegen`).
- [x] Make carry recognition return typed dataflow metadata (`pil_opt`), with
  Rust rendering separate. Record guest-CPU/output-backend/runtime reuse limits
  in `docs/development/COMPILER_REUSE.md`.
- [ ] Plan extraction of the core PIL types/parser/executor/Rust emitter from
  the existing large `main.rs` into a library boundary, incrementally.
- [ ] Package a reusable DOS runtime once its ownership/service interfaces are
  established; the current embedded runtime module is an initial extraction.
- [ ] Bring older feature/status summaries into line with measured current
  capability; avoid marking existing features as new work.

## Working references

- [Native DOS usage and limits](docs/DOS_NATIVE.md)
- [Compiler reuse boundaries](docs/development/COMPILER_REUSE.md)
- [Protected-mode status and handoff](docs/development/PROTECTED_MODE_PLAN.md)
- [Historical ISA completion plan](docs/development/ISA_COMPLETION_PLAN.md)
- `tools/verify_dos_standalone.py`, `tools/verify_dos_smc.py`
- `tools/build_ansi_mandel.py`, `tools/verify_ansi_mandel.py`
- `tools/audit_dos_codegen.py`
