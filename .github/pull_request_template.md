## Summary

## Checks
- [ ] PIL and Dine stay program-independent libraries: nothing builds an engine, backend, Dine library or translator for one program (Dine_<program> launchers carry only a loader and guest bytes), and nothing in src/, isa/, Cargo.toml or the tools/ scripts checks a program's name, file name, hash, PSP, command line or addresses (tests, benchmarks, fixture builders, docs and launchers exempt; precompilation is not). `python tools/check_program_specific.py --self-test` passes.
- [ ] New forms have independent expected-state and fault tests; both backends agree.
- [ ] Windows and Linux CI are green.
- [ ] Results say where they were run (CI, local, or a report).
