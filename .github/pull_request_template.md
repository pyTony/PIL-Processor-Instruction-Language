## Summary

## Checks
- [ ] No program-specific code in general files: nothing in src/, isa/, build.rs, tools/dos_terminal.py or the precompiler build tools checks a program's name, file name, hash, PSP, command line or addresses (tests, docs and launchers exempt; the precompilers are not). `python tools/check_program_specific.py --self-test` passes.
- [ ] New forms have independent expected-state and fault tests; both backends agree.
- [ ] Windows and Linux CI are green.
- [ ] Results say where they were run (CI, local, or a report).
