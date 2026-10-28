# PIL Interpreter & Lazy-Flag Optimization Report

The initial optimization measurements below were reported with 345 passing unit
tests, 207 full-state backend trace comparisons, and native host CPU oracle
checks. Review subsequently found generic PIL regressions outside those cases;
the corrections and additional verification are recorded below. Passing these
cases establishes equivalence for the tested inputs, not every valid program.
`Program::tokens` remains available for `pilstep` and `--emit-rust`.

---

## 1. Benchmark Summary (`release`, Windows/x86_64)

| Workload | Guest Instructions | Baseline | + PIL Super-Tokens & Cache | + KISS `LazyFlags` & In-Band Probe | Total Speedup |
| :--- | ---: | ---: | ---: | ---: | :--- |
| **`--bench 10000 5` (median)** | `30,003` | `4.389 ms` (`6.84M` ins/s) | `2.559 ms` (`11.73M` ins/s) | **`2.405 ms`** (**`12.47M` ins/s**) | **`1.83×` (+82.5%)** |
| **`--mandelbrot 64 32`** | `815,148` | `127.001 ms` (`6.42M` ins/s) | `87.242 ms` (`9.34M` ins/s) | **`75.284 ms`** (**`10.83M` ins/s**) | **`1.69×` (+68.7%)** |
| **Test Suite (`cargo test --locked`)** | 345 tests | 344 passed | 344 passed | **345 passed** | **0 diffs** |

---

## 2. Optimizations Implemented

1. **Zero-Allocation Stack Execution Frame (`src/main.rs`):**
   - Ordinary instructions use stack arrays (`[Value; 64]` and `[(View, Value); 4]`). Larger valid PIL programs use heap fallbacks sized at compile time, preserving the language's existing value and ordered register-effect support.
2. **Compile-Time PIL Super-Token Packing (`PackedOp` & `pack_tokens` in `src/main.rs`):**
   - Strips no-op `PreserveFlags` tokens and folds common multi-token sequences (`Conditions16`, `Jcc16`, `RelJmp`, `AddCore`/`AddLazy`, `SubCore`/`SubLazy`, `LogicCore`/`LogicLazy`, `ImulCore`, `ShiftSetup`/`ShiftFlags`, `Flags6`/`Flags5NoCf`/`LogicFlags`, `ReadDstSrc`/`ReadDstConst`) at `compile()` time while keeping `Program::tokens` intact.
3. **KISS Queued Lazy-Flag Evaluation (`LazyFlags` in `src/main.rs`):**
   - Queues arithmetic/logical flag descriptors (`None`, `Add`, `Sub`, `Logic`) across instructions during multi-step runs.
   - Full 6-flag ALU writers (`ADD`, `SUB`, `CMP`, `AND`, `OR`, `XOR`, `TEST`) overwrite and drop unconsumed previous flag operations without evaluating them.
   - 5-flag writers (`INC`, `DEC`) materialize only `CF` (`flush_cf_only`) from the previous queued operation and drop its `PF, AF, ZF, SF, OF` evaluations.
   - `Jcc16` evaluates common branch conditions (`JE`/`JNE`, `JB`/`JAE`/`JBE`/`JA`, `JS`/`JNS`, `JL`/`JGE`/`JLE`/`JG`) directly against the queued descriptor without flushing.
   - Automatically materializes (`LazyFlags::flush`) on flag-reading/modifying tokens (`flushes_lazy`) and at all step/breakpoint/halt/fault boundaries.
4. **Fast Cache Span Verification & Direct-Mapped Lookaside (`src/guest.rs`, `src/cache.rs`):**
   - Added a 256-entry direct-mapped lookaside table in `DecodeCache` and a single-slice span check fast path in `Image::fetch_signature` (falling back to byte-by-byte `Reader` on boundary/fault cases).
5. **In-Band Memory Access Tracking (`src/main.rs`, `src/debugger.rs`):**
   - Recorded `ProbedMemoryAccess` directly during `Load`, `Store`, `LoadSegment`, and `StoreSegment` execution inside `execute_state`, eliminating the redundant pre-execution `probe_machine_access` pass per step.

## 3. Review corrections (29 October 2026)

Corrections are based on `f4b516f7a95b077adaece51d84ea0f755b3cd17f`.
Source and tested binary hashes are recorded in
[review evidence](PR41_REVIEW_EVIDENCE.json).
The review reproduced an out-of-bounds panic with 80 PIL values and a flag
mismatch after replacing ADD's `flag CF cf` with the valid `flag CF zf`.
Regression tests now cover both, ordered register effects beyond four entries,
and packed arithmetic/shift intermediates consumed later in a description.

- Keep stack execution frames for ordinary instructions and allocate a fallback
  for larger value files or register-effect lists.
- Fuse calculation chunks only when omitted intermediate values are dead.
  Queue lazy flags only when the flag writes reference that calculation's
  actual output slots. Check the constant predicates required by shift packing.
- Ignore a container load result after its requesting engine session changes,
  its browser closes, or the application closes. Delete discarded temporary
  files and temporary files left by failed extraction.
- Preflight both BIOS timer BDA writes before committing RAM, timer or CPU state.
- Stop PIL preview/stepping at failed mode, segment-span and branch checks;
  report an execution stop when the instruction faults.

Local Windows/x86_64 verification of the corrected source:

- `cargo test --locked`: 350 passed, including the actual-host CPU oracle:
  340,920 cases, 64 encodings and 40 harness checks.
- Frontend unit tests: 11 passed. Debug-service and withdrawn Tk window smoke
  checks passed, including output rendering and Pause.
- Release `--verify-backends`: 207 cases, 7,407,658 matching trace bytes.
- Full `--compare-mandelbrot 256 32`: 65,536 reference pixels and 256
  CPU/RAM/flags/serial/stop checkpoints matched (1,157,120 checkpoint bytes);
  both engines completed 12,940,344 guest instructions with 255 TX retries.
- Isolated 116-file runtime export: build, launchers, emitted backend, serial
  transcript, debug service, DOS hello and ZIP listing/probe/extraction passed.
- Identical 36,340-byte COM program: 210/210 passed in both DOSBox-X
  (`normal`, `386`) and PIL; no changed binary hashes during the run.
  This is an original emulator comparison, not an external 386 hardware suite.

Alternating release benchmark runs on the same host compared the pre-optimization
PR head `9f89c41b1e55443ff6444ff5829dfd8409fa1ab5` with this corrected source.
For `--bench 10000 5`, baseline medians were 4.180/4.111 ms and corrected
medians were 2.524/2.533 ms: approximately 1.62–1.66x faster in these two runs.
Each invocation used five measured samples and one excluded warmup, with cold
cache/fresh CPU and RAM per sample. These measurements describe this workload
and host; they do not establish a general speedup or 386 cycle rate.
