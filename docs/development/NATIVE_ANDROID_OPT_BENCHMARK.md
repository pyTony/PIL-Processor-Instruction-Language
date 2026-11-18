# Native PIL optimization benchmark on Termux / Android ARM64

Compare Rust/LLVM optimization levels without compiling the full half-million-line
native verification suite. The fixed x86 counted loop is decoded and converted
to native Rust by PIL's existing compiler; each executable uses **the exact same
generated source**, with different `rustc -C opt-level` settings.

This is a **fixed-image correctness/performance proof**, not arbitrary DOS
COM/EXE compilation. It does not enable a new interpreter or guest JIT.

## Build

On `dev` with this experiment's changes, build PIL with the memory-limited
Termux command (no unrestricted Cargo release build):

```sh
sh tools/build_termux.sh --compact
python tools/bench_native_android.py --iterations 10000 --samples 5 --emit-only
```

Inspect `target/native-opt-levels/loop.rs` before compiling. This emitter
contains only the counted-loop guest image, rather than the entire native test
suite. The output still includes the shared PIL runtime support modules.

Run sequential native compilation and benchmarks:

```sh
python tools/bench_native_android.py --iterations 10000 --samples 5
```

The script compiles the same generated source at O0, O1 and O2, in order,
with no debug info, no LTO, no concurrent jobs. Each program runs with
`bench` mode, which asserts CPU registers, EFLAGS, undefined flags,
memory, exit status and retired instruction count for every timed sample.
It reports compile wall time, unstripped executable bytes and median
guest instructions/second, and keeps complete logs and CSV/JSON tables.

Outputs:

- `target/native-opt-levels/loop.rs`: inspectable generated Rust
- `target/native-opt-levels/loop_O0`, `loop_O1`, `loop_O2`: native host executables
- `target/native-opt-levels/O*.compile.log`: compiler warnings/errors
- `target/native-opt-levels/O*.run.log`: sample durations and median
- `target/native-opt-levels/report.csv`, `report.json`: results, retained
  after each successful level in case subsequent compilations fail

Optional `--asm` emits `loop_O0.s`, `loop_O1.s`, `loop_O2.s` with a
**second** rustc invocation per level. Avoid `--asm` on memory-constrained
phones until the base benchmark succeeds.

## Interpreting results

All builds must be checked on the **same Android device**, with the same
`--iterations` and `--samples`. Binary sizes include runtime support and
are **unstripped**, allowing a fair comparison across optimization levels.
The guest instruction rate is not an emulated 386 clock speed. Compilation
and process launch time are excluded from timed guest instruction samples.
These are short microbenchmarks; thermal throttling and timer noise may
affect measured ratios.

The generated program validates expected state independently, but this script
does **not** compare full native execution traces against the PIL interpreter.
The existing `--verify-backends` workflow supplies the stronger per-step
cross-backend correctness check.

Only add PIL-specific optimizer passes after identifying which duplicated
state/flag work LLVM fails to remove. Optimize basic blocks first, while
preserving precise fault order and state commits.
