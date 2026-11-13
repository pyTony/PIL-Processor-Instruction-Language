# TODO

- Block compiler inside `pil`, the same for every program: build-time Rust
  per PIL instruction form, blocks planned at run time from executed code,
  kept guards and interpreter fallback, both backends compared. See
  [docs/development/BLOCK_COMPILER.md](docs/development/BLOCK_COMPILER.md).
- No tool, build step or CI job may produce an engine, backend, Dine library
  or translator for one guest program (`tools/check_program_specific.py`).
  `Dine_<program>` launchers carry only a loader and the guest bytes.

The November 10 compiler TODO is kept at
[docs/history/precompiled/TODO_COMPILER_2026-11-10.md](docs/history/precompiled/TODO_COMPILER_2026-11-10.md).
