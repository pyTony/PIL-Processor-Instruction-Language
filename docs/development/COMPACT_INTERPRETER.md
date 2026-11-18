# Compact PIL interpreter experiment

This branch tests the size impact of omitting the precompiled PIL instruction-form table without changing generic instruction semantics.

## Build modes

- Default: `cargo build --locked` retains the existing generated native forms and runtime selection.
- Compact: `cargo build --locked --features compact-interpreter` does **not compile** `src/precompiled/forms.rs`. The resolver returns `None` and generic PIL execution handles the instruction.
- The mode is compatible with both real-mode and protected-mode PIL execution. DOS, FPU, native-source utilities, and desktop presentation are **not** yet independently feature-gated.

The `PIL_PRECOMPILED=off` environment variable disables native execution at runtime but does **not** shrink the binary; the new Cargo feature is needed for a smaller binary.

## Verification (Windows or a build-capable host)

```powershell
cargo check --locked --features compact-interpreter
cargo test --locked --features compact-interpreter
cargo build --release --locked --features compact-interpreter
Get-Item target/release/pil.exe | Select-Object Name,Length
```

For an apples-to-apples size comparison, build the default executable with the same toolchain/profile and record its size before replacing it. Consider using separate `CARGO_TARGET_DIR` values or copying the binaries. Run the DOS fixture and mode-transition tests on both variants.

On Termux, keep the single-job, low-memory script settings and use the new `--compact` flag:

```sh
sh tools/build_termux.sh --compact
# Optionally: sh tools/build_termux.sh --release --compact
ls -l target/debug/pil
```

To compare stripped sizes, make a copy with `cp target/debug/pil "$HOME/pil-compact-stripped"` and run `llvm-strip "$HOME/pil-compact-stripped"`. Compare byte counts with the prior default build, using matched profiles. **Do not run an unbounded direct Cargo release build.**

## Expected limitations

- The generated forms were a large fraction of the Termux debug binary's identified code symbols, but the new executable size must be measured.
- Source files remain coupled into the common crate. This is a first compile-time feature boundary, not complete library/embedded modularization.
- A missing generated form always falls back to the generic interpreter; block execution may still group interpreted instructions.
- No claim is made that an Android standalone executable, ESP32 firmware, or fully standalone DOS native program is produced.

## Acceptance criteria

1. Default build and tests remain unchanged.
2. Compact build passes tests, including the added no-forms and interpreter-fallback regression.
3. DOS real-mode and protected-mode transition tests remain green.
4. Binary size and runtime throughput are compared against the default using matched profiles.
5. Any regressions are documented before considering a merge.
