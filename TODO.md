# TODO

- Block compiler inside `pil`, the same for every program: build-time Rust
  per PIL instruction form, blocks planned at run time from executed code,
  kept guards and interpreter fallback, both backends compared. See
  [docs/development/BLOCK_COMPILER.md](docs/development/BLOCK_COMPILER.md).
- No tool, launcher, build step or CI job may produce an executable, backend
  or library for one guest program (`tools/check_program_specific.py`).

The November 10 compiler TODO is kept at
[docs/history/precompiled/TODO_COMPILER_2026-11-10.md](docs/history/precompiled/TODO_COMPILER_2026-11-10.md).
