# PIL proof prototype — v0.4.0

## Current v0.4 patch — PC emulator foundation

This patch adds EIP and all 16 conditional jump predicates with decoded
relative offsets. It supplies 28 Rust tests. Rust is not installed in the
authoring environment, so compilation and the new Rust tests await verification
on your machine. v0.3's 20-test pass does not verify these new sources.
Independent Python checks passed 789,372 arithmetic + 1,030 conditional cases,
and 12,288 Jcc/context cases plus a staged-fault check.

```sh
cargo test
cargo run -- --demo-branch
python3 verify_flow.py
```

Both generated and interpreted execution retain the arithmetic compatibility
APIs; branch descriptions require explicit flow context. A separate generated
Rust test covers 12,288 flow cases. There is still no binary decoder, guest
memory or exception-handler delivery; see LANGUAGE.md for the exact contract.

Use this existing ~/pil_proof project. Future updates are patches, not new
versioned directories. Leave tests inline for now; a tests/ directory comes
later. ROADMAP.md records the next serial mock and terminal REPL debugger,
followed by an embedded core suitable for TTGO ESP32 cross-compilation.

End goal: a PC emulator on the TTGO ESP32 using VGA, PS/2, SD and WLAN.
The debugger is a development interface for that emulator.


A width-aware CPU instruction description runs as validated tokens in Rust,
or emits Rust source for compilation. No external Cargo dependencies.

The previous v0.2 release passed all 14 tests on the user's Termux phone.
This release supplies 20 Rust tests and awaits Rust verification: the authoring
environment has no Rust compiler. Independent Python checks run separately.

## Earlier v0.3 changes

- Explicit, complete classification of CF/PF/AF/ZF/SF/OF: defined, preserved
  or undefined. Duplicate and missing classifications are rejected.
- Undefined output mask with deterministic concrete zero, distinct from
  preserving the previous value; AND demonstrates undefined AF.
- All 16 canonical SETcc condition predicates, with byte-only destinations.
- Generic select and conditional register write tokens; no guest jumps yet.
- Revision history, source checksums and updated references/credits.

## Build and test

Requires a recent stable Rust toolchain (edition 2021).

```sh
cargo test
cargo run
cargo run -- --emit-rust 8 setl > setl.rs
rustc --edition=2021 --crate-type=lib setl.rs
cargo run -- --emit-rust 32 sbb > sbb32.rs
rustc --edition=2021 --crate-type=lib sbb32.rs
```

The differential test invokes rustc (from PATH or RUSTC) and compares 37
operation/width variants, including undefined-status masks. Its planned
526,390 records include exhaustive 8-bit arithmetic/logic and every SETcc
truth-table combination, plus directed wider and predicated-write cases.
The test removes its temporary files after success. Generated code may
produce unused-value warnings; those are not test failures.

On Windows PowerShell, use Set-Content -Encoding utf8 instead of legacy
output redirection when saving generated source if required by that shell.

Optional independent description check, using Python 3.10 or newer:

```sh
python3 verify_descriptions.py
```

This runner reads the actual PIL files and compares results with arithmetic
and condition oracles; it does not compile or execute Rust. The original
v0.1/v0.2 Rust passes do not establish v0.3 correctness.

## Files

- LANGUAGE.md: precise supported token syntax and undefined-flag policy.
- src/main.rs: validator, token interpreter, Rust emitter and Rust tests.
- add/sub/cmp/adc/sbb/and.pil: operation and flag descriptions.
- conditions.pil + setcc.pil: the sixteen canonical SETcc descriptions.
- write_if_z.pil: generic predicated write demonstration, not a 386 CMOV.
- verify_descriptions.py: independent Python oracle runner.
- REFERENCES_AND_CREDITS.md: reference/provenance/licence/thanks register.
- CHANGELOG.md: concise feature changes by release.
- REVISION_HISTORY.md and revision_history.json: version and test evidence.
- SHA256SUMS: content fingerprints for this release.
- tiny386-adapter.md: planned PC-device boundary, not a linked module.
- ../processor_instruction_language_v01.md: larger architecture proposal.

Generated execute_with_metadata preserves a diagnostic undefined-flag mask
across instructions. The compatibility execute wrapper discards that metadata.
See LANGUAGE.md for limits: the mask is not complete symbolic taint tracking.

There is no binary x86 decoder, memory, guest exception/EIP model, Jcc,
PC hardware, no_std port or ESP32 integration yet. Descriptions tokenize to
in-memory enums; no persistent bytecode format is implemented. The next
milestone is guest control flow, followed by memory and restartable faults.
