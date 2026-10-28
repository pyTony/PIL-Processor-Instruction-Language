# PIL Interpreter & Lazy-Flag Optimization Report

All optimizations preserve **100% bit-exact architectural and trace equivalence** across all 345 unit tests, 207 full-state backend trace comparisons, and native host CPU oracle checks, while leaving `Program::tokens` untouched for `Step PIL F8` (`pilstep`) and `--emit-rust`.

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
   - Replaced per-instruction heap allocations (`values: Vec<Value>` and `pending_registers: Vec<(View, Value)>`) in `execute_state` with fixed-size stack arrays (`[Value; 64]` and `[(View, Value); 4]`).
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
