## Summary

## Checks
- [ ] No program-specific code in general files: nothing in src/, isa/, build.rs or tools/dos_terminal.py checks a program's name, file name, hash, PSP, command line or addresses (runtime JIT path, tests, docs and launchers exempt). `python tools/check_program_specific.py --self-test` passes.
- [ ] New forms have independent expected-state and fault tests; both backends agree.
- [ ] Windows and Linux CI are green.
- [ ] Results say where they were run (CI, local, or a report).
