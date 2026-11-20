# Native applications and the Pascal environment

20 November 2026. User-directed acceptance target: the Pascal IDE and compiler,
with an edit, compile, run and return cycle, delivered as native application
code calling the shared Dine library. The original DOS executable must not be
an external runtime dependency. Preserve the original program's functions;
successful startup alone is insufficient evidence.

## Execution boundary

- Relocate the initial image during compilation and emit its prepared register,
  segment and initialized-data state. Do not repeat guest relocation at launch.
- Compile ordinary guest code as application code. CPU semantics, flag rules,
  memory, devices and DOS services remain in the program-independent libraries.
- Keep code that needs modification or is generated at runtime separate. The
  user permits interpretation of such code. Do not describe this exception as
  complete static native coverage.
- Retain resources needed by the original application. A source executable may
  carry overlays as well as its initial load image. Any retained instruction
  bytes and their execution path must be disclosed explicitly.
- Neither a loader embedding guest bytes nor the current runtime form/block
  engine alone meets this ahead-of-time application target.

## Milestones

1. Measure native and interpreted execution correctly and provide a mode that
   refuses interpreter fallback. Compare normal and reference execution.
2. Prepare relocated startup state and an application-to-library interface that
   uses shared native CPU forms. Validate original synthetic COM and MZ programs
   before translating a third-party application.
3. Translate the IDE's ordinary code, preserve its overlay/resource behavior,
   and account separately for runtime-generated or modified regions.
4. Verify source editing and saving, repeated compilation and execution,
   expected output, return to the IDE, and a clean exit. Repeat through a DOS
   shell and check that the shell remains usable.
5. Run the repository policy checks, appropriate regression/backend checks and
   hosted Windows/Linux CI; publish development PRs against `dev`. The user
   reviews and merges. Runtime-only `main` delivery stays separate.

The local third-party compiler, libraries and executables remain outside Git.
Use original fixtures for shared implementation regressions. An observed
workflow proves that workflow, not equivalence of every function of a closed
binary.

## Native execution accounting

`PIL_PRECOMPILED=required` (alias `strict`) runs the shared native forms and
blocks and stops before a PIL instruction without a native form. DOS/BIOS,
device, coprocessor and interrupt handling remain the ordinary shared Rust
services. This mode still decodes guest bytes and is not the final static
application compiler.

Session snapshots distinguish `native_block_instructions`,
`native_single_instructions` and `interpreter_instructions`. These count
successfully retired PIL instructions: a faulting instruction does not count,
and a partially executed REP counts only when it retires. Block entries can
contain interpreted operations in normal mode; those do not count as native.
`native_total_instructions` adds the two native retirement counters. Other
session steps include DOS/BIOS services and interrupt handling.

Local Windows validation of this accounting increment: 1,015 Rust tests pass,
2 ignored. The existing native-form, block and generated-backend comparisons
are included. A local IDE cycle compiled an original summation program,
printed `PIL native Pascal: 5050`, and returned to the editor with strict
native forms enabled. It does not yet establish a standalone translated IDE,
repeat editing/saving, clean exit, or complete program equivalence.
