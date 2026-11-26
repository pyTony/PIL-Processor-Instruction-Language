# Compact and common-forms PIL builds

These build modes trade precompiled native-form table size and compile memory against interpreter fallback without changing generic instruction semantics.

## Build modes

- **Default:** `cargo build --release --locked` compiles the full `crates/pil-cpu/src/precompiled/forms.rs` table (`5,369` forms, `10,128,523` source bytes) covering 16-bit real mode, 32-bit protected mode with a 32-bit stack, 16-bit ring-3 protected mode, and real mode with a 32-bit stack.
- **Common forms (`--common` / `common-forms`):** `cargo build --release --locked --features common-forms` compiles `crates/pil-cpu/src/precompiled/common_forms.rs` (`506` forms, `944,371` source bytes): the most frequently used 16-bit real-mode and 32-bit protected-mode ALU, move, shift, stack, control-flow, string, and flag instructions. Rarer and privileged/system instructions return `None` from the resolver and run in the generic PIL interpreter.
- **Compact (`--compact` / `compact-interpreter`):** `cargo build --release --locked --features compact-interpreter` does **not compile** any generated form table (`0` forms); the resolver returns `None` and generic PIL execution handles every instruction. (`compact-interpreter` takes precedence if both features are enabled.)

The `PIL_PRECOMPILED=off` environment variable disables native execution at runtime but does **not** shrink the binary; a Cargo feature is needed for a smaller binary.

### Measured Windows x86-64 release sizes (`cargo build --release --locked`)

| Mode | Feature flags | Compiled forms | Source table bytes | Release `pil.exe` bytes |
|---|---|---|---|---|
| Default | *(none)* | 5,369 | 10,128,523 | 13,628,928 |
| Common forms | `--features common-forms` | 506 | 944,371 | 4,476,928 |
| Compact | `--features compact-interpreter` | 0 | 0 | 3,879,936 |

## Verification (Windows or a build-capable host)

```powershell
cargo check --locked --features common-forms
cargo test --release --locked --features common-forms --lib precompiled_tests
cargo build --release --locked --features common-forms

cargo check --locked --features compact-interpreter
cargo test --release --locked --features compact-interpreter --lib precompiled_tests
cargo build --release --locked --features compact-interpreter
```

Both generated tables are checked in CI:
```sh
pil --emit-precompiled-forms crates/pil-cpu/src/precompiled/forms.rs --check
pil --emit-precompiled-forms crates/pil-cpu/src/precompiled/common_forms.rs --common --check
```

On Termux, keep the single-job, low-memory script settings and pass either `--common` (most common native forms ready) or `--compact` (no generated forms):

```sh
sh tools/build_termux.sh --common
sh tools/build_termux.sh --release --common
sh tools/build_termux.sh --compact
# Optionally: sh tools/build_termux.sh --release --compact
```

**Do not run an unbounded direct Cargo release build on memory-constrained Termux hosts.**

## Acceptance criteria

1. Default build and tests remain unchanged.
2. Both `common-forms` and `compact-interpreter` builds pass tests with zero warnings, including fallback to the generic PIL interpreter.
3. All feature combinations (including `--all-features`) compile cleanly.
4. Both `forms.rs` and `common_forms.rs` pass `--emit-precompiled-forms ... --check`.
