# Block compiler: what was kept, and the road to integration

13 November 2026. Standing rule (Tony): **PIL and Dine are always
program-independent libraries, used the same way everywhere.** Nothing in the
repository may build, generate or ship an executable, a backend or a launcher
for one guest program. Precompilation is not a JIT, but it is not exempt.

## What was removed

| Removed | What it did |
|---|---|
| `build.rs` (`PIL_SHADOW_SOURCE`, cfg `pil_shadow_backend`), `src/dos_compile.rs`, `pil --emit-dos-backend` | Recorded one program and linked its observed blocks into a per-program `pil` engine |
| `src/dos_standalone.rs`, `src/dos_translate.rs`, `src/dos_runtime.rs`, `src/dine_native_video.rs`, `pil --emit-dos-standalone` | Translated one program into a standalone native executable with its own small DOS runtime |
| `tools/build_dos_shadow.py/.ps1`, `build_dos_native.py`, `build_dos_bundle.py`, `build_dine.py`, `audit_dos_codegen.py`, `pildos_raw.sh` | Build drivers for the above, and the `Dine_<program>` launchers that embedded one guest |
| `tools/verify_dos_standalone.py`, `verify_dos_smc.py`, `verify_*_native.py`, `verify_dos_shadow*.py`, `verify_lines_hybrid.py`, `verify_landmark_shadow.py`, `verify_gwbasic_bundle.py`, `verify_dine_executable.py` and three GUI tests | Checks of those per-program builds |
| `ProgramIdentity` and `Loaded::active_program` in `src/dos.rs`, saved and restored by EXEC | Existed only so a per-program backend knew which program it was compiled for |
| `scratch/pil-*before*.exe`, `scratch/*_backend.rs`, `scratch/mandvga_*.rs`, `scratch/shadow_guard_test.rs`, `scratch/compare_*.py`, `scratch/bench_lm_fallback.py` | Tracked host executables and one-off generated Rust from shadow experiments |

Their documents moved to `docs/history/precompiled/` with a "Historical"
banner; the November 10 compiler TODO is there as
`TODO_COMPILER_2026-11-10.md`. The git history keeps every line.

## What was kept, and where

Everything kept is program-independent and lives in `src/block_compiler/`.
It is compiled and unit-tested, but not wired into any command yet.

| Module | Origin | Contents |
|---|---|---|
| `sites.rs` | `src/dos_blocks.rs` | `Site` (decoded PIL program, bindings, flow, address and segment context, code bytes), `InstructionProfile` (register-only / memory / machine-context), `BlockPlan` (blocks of at most 16 instructions, alternate one-instruction entries for interior targets) |
| `codegen.rs` | `src/dos_codegen.rs` | Rust rendering of one instruction module and of a planned block through a `BlockAdapter` |
| `guards.rs` | entry checks of `dos_compile.rs` and the standalone runtime | `compatible` (segment state, real-mode relocation allowed, paging and unreal limits refused), `code_matches` (bytes unchanged, inside CS limit and an executable window), `MAX_CHAIN` = 256 |
| `recorder.rs` | recorder of `dos_compile.rs`, without program identity | An `InstructionBackend` that collects executed sites, block leaders and visit counts per code segment base; `record()` runs any COM/EXE in an isolated machine with a step and site budget |

Also kept in place:

- The hook in Dine: `dos::InstructionBackend`, `dos::run_with_backend`,
  `Session::with_backend` (tests in `tests/unit/dos_backend.rs`). With no
  backend the interpreter runs everything, as every command does today.
- `src/native.rs`: the fixed-image generated-Rust backend comparison
  (`--verify-backends`, `--compare-backends`) over the built-in demo images. It is a
  test of PIL's two backends, not a program build.
- The `dine` cdylib (`src/dine_ffi.rs`, `--features dine-library`): one
  program-independent C ABI (`dine_abi_version`, `dine_run`) that takes any
  guest at run time. `tools/verify_dine_library.py` checks it with ctypes.
- The VGA controller rules shared through `vga_planar::Planar::controller_io`.
- Small helpers that the removed runtime had and Dine needs:
  `dos::command_tail`, `dos_services::dollar_string`, `character_output`.

The CI guard `tools/check_program_specific.py` now also fails when anything
outside docs and comments uses the removed per-program build interfaces, or
when the repository tracks a host executable or shared library.

## Is the full blocks design on dev?

Yes. Checked on 13 November 2026 against `origin/dev` (8b5e280):

- Block PRs #133, #134, #140, #164 and #167 are merged.
- `docs/development/NATIVE_DOS_ARCHITECTURE.md` (the full "atomic native
  blocks" design, 1,080 lines), `COMPILER_REUSE.md` and
  `BLOCK_EXECUTION_UPGRADE.md` are on dev and stay as design references.
- Tony's local stashes from 10 November with compiler work: every file blob
  in them (dos_compile.rs, cache.rs, memory.rs, tests/unit/cache.rs,
  dos_backend.rs, build_dos_shadow.py) is in dev history; `DOS_NATIVE.md`
  differs only in wording (5-7 lines). Nothing of the design is missing.
- Not merged: `integration/pr141-conflict-resolution-20261111` has 18
  commits, but they are terminal and VGA work, not blocks.

## Integration route

The next compiler must be a library inside `pil`, the same for every
program:

1. A precompiled, program-independent table: Rust generated at build time
   from the PIL descriptions (per instruction form, not per program), so the
   one `pil` binary carries it.
2. At run time Dine plans blocks from what it executes (`recorder` logic
   inside the session, `BlockPlan`), keyed by code bytes and segment state,
   and runs them through the kept `guards`. Any guard failure falls back to
   the interpreter.
3. Both backends must agree: the existing trace comparison (`native.rs`,
   `--verify-backends`) extends to block execution.
4. No output file, cache directory or launcher belongs to one program.

Issue #148 (file I/O in the standalone runtime) is obsolete with this change.
