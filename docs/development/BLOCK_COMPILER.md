# Block compiler: what was kept, and the road to integration

13 November 2026. Standing rule (Tony): **PIL and Dine are always
program-independent libraries, used the same way everywhere.** Nothing in the
repository may build an engine, backend, Dine library or translator for one
guest program. `Dine_<program>` launchers stay: a tiny loader plus the guest
bytes on the shared dine library, with no copy of Dine or PIL inside
([DINE_EXECUTABLES.md](../DINE_EXECUTABLES.md)). Precompilation is not a JIT, but it is not exempt.

## What was removed

| Removed | What it did |
|---|---|
| `build.rs` (`PIL_SHADOW_SOURCE`, cfg `pil_shadow_backend`), `src/dos_compile.rs`, `pil --emit-dos-backend` | Recorded one program and linked its observed blocks into a per-program `pil` engine |
| `src/dos_standalone.rs`, `src/dos_translate.rs`, `src/dos_runtime.rs`, `src/dine_native_video.rs`, `pil --emit-dos-standalone` | Translated one program into a standalone native executable with its own small DOS runtime |
| `tools/build_dos_shadow.py/.ps1`, `build_dos_native.py`, `build_dos_bundle.py`, `audit_dos_codegen.py`, `pildos_raw.sh` | Build drivers for the above |
| `tools/verify_dos_standalone.py`, `verify_dos_smc.py`, `verify_*_native.py`, `verify_dos_shadow*.py`, `verify_lines_hybrid.py`, `verify_landmark_shadow.py`, `verify_gwbasic_bundle.py` and two GUI tests | Checks of those per-program builds |
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
4. No engine, library or cache belongs to one program; a `Dine_<program>`
   launcher only carries the guest bytes and loads the shared library.

Issue #148 (file I/O in the standalone runtime) is obsolete with this change.

## Integrated: precompiled forms and run-time blocks (13 November 2026)

Steps 1-3 of the route above are in, in the shared code every launcher,
session and the `dine` library use. Tracking issue #183.

**Precompiled forms** (`crates/pil-cpu/src/precompiled/`). A *form* is a decoded PIL
program with its immediate values taken out. `pil --emit-precompiled-forms
crates/pil-cpu/src/precompiled/forms.rs` enumerates the real-mode decoder (prefixes 66, 67,
the six segment overrides, F2/F3; one- and two-byte opcodes; all 256 ModRM
bytes; three immediate patterns) and finds 2,669 forms. For each it writes
`execute_state` and `execute_form`, rendered from the PIL tokens by the
`--emit-rust` emitter. A constant that differed between the enumerated
decodes becomes a run-time slot (`k[n]`); one that never did stays a
literal, and `precompiled::resolve` refuses the form when a program's value
there differs. The key is the program shape (tokens with constants zeroed,
widths and flags) hashed with two FNV-1a variants. A program the table does
not have runs in the interpreter, as before.

The generated file is committed (about 13.5 MB). CI regenerates it with
`--check` and fails when it is stale, so an emitter or decoder change must
come with a regenerated table. Real-mode code with a 16-bit stack only for
now: with 32-bit stacks and 32-bit protected-mode code the table was 4,319
forms (22 MB), which made the release build too slow for its gain today.

**Run-time blocks** (`crates/pil-cpu/src/block_compiler/engine.rs`). When the DOS loop
reaches an address, Dine plans a block there: straight-line PIL instructions
through the decode cache, at most 16, ending after any control transfer,
store, segment/control/table write, interrupt shadow or write of TF, IF,
IOPL, NT or VM. Instructions the loop answers itself never join a block
(port I/O, INS/OUTS, 80387 escapes and WAIT, INT n, HLT), nor does code
outside 0400h-9FFFFh. A block runs only with no interrupt shadow and TF
clear, when the segment state and fetch window equal the planned ones and
its bytes are unchanged (code stamp, else a byte compare); otherwise it
is planned again. The DOS loop runs one block per iteration, never past a
multiple of 256 steps (its timer check), and checks timer, keyboard, mouse
and Ctrl-C events between blocks as it does between instructions; nothing
inside a block can make one of them due. Dine's own low memory (0000h-0FFFh:
vectors, BIOS data, the DOS data area with its break stubs) is not covered.
Retired instructions are counted per mnemonic, so probe reports are
unchanged.

**Modes** (`PIL_PRECOMPILED`): default runs blocks and native forms; `off`
uses only the interpreter; `verify` runs each instruction's native form and
the interpreter on a copy of the state and stops with both states on any
difference (slow: it copies the 16 MiB RAM each step).

**Checks** (tests/unit/precompiled.rs): forms against the interpreter on
full state, faults included, for more than 10,000 sampled encodings and
states; blocks against the interpreter after every slice (odd budgets) on
FACTOR, RECURFIB, MANDEL, MANDBBC and a self-modifying loop; verify mode in
lockstep; immediates sharing a form and a changed fixed constant refused.

**Local Linux release** (`PIL_PRECOMPILED=off` against the default):
`pil --dos-probe --steps 1000000`, whole process, reports identical:
MANDEL 143 -> 71 ms, MANDANSI 153 -> 74 ms, MANDBBC 166 -> 96 ms, MANDCGA
240 -> 58 ms. `tools/verify_mandel_vga.py` (a full MANDVGA session through
the terminal protocol, 296,960 pixels checked): 9.5 s -> 2.6 s.

**Chains through CALL, RET, jumps and loops** (14 November 2026). Blocks
still end after a control transfer or a store, but a block whose last
instruction is a near JMP, Jcc, LOOP, CALL, RET or a store no longer goes
back to the DOS loop: `Blocks::run` goes straight on to the block at the
new CS:IP, until the step budget (never past the 256-step timer check) is
used up or a block changes segments, control or descriptor-table
registers, the TSS, an interrupt shadow, a flag checkpoint, or IF, TF,
IOPL, NT or VM, or faults. Every block in a chain checks its own entry
guard; there is no prediction, so a RET to a different return address
simply finds (or plans) the block there. Each block keeps links to the
two blocks that last followed it: a link skips the table lookup and the
segment and window comparison (which a chain cannot change, and a link
is dropped when its block is planned again), never the code check. The
DOS loop chains only while it has no scan code or mouse event to look at
between instructions, so IRQ1, mouse callbacks, the timer and Ctrl-C come
at the same instruction boundaries as with the interpreter.

Code stamps (crates/pil-cpu/src/memory.rs `CodeWatch`, also used by the decode cache)
now count writes per 64-byte line instead of marking 4 KB pages dirty for
good. A write elsewhere in the page (data kept next to code, as small COM
programs do) keeps the code trusted; a write into its lines makes the next
entry compare bytes, and a match takes a new stamp.

Local Linux release, a full session through the terminal protocol to the
key wait (3 runs each, base = dev at 3919bd8): MANDCGA 1.91-2.06 s ->
0.87-0.91 s, MANDVGA 2.58-2.86 s -> 1.44-1.64 s, with the same 31.6M and
37.9M block instructions. Instructions per block are unchanged (3.8 and
3.9); instructions per return to the DOS loop are now 256 and 169 (one
block before). MANDBBC and MANDEL end in under 0.1 s either way (start-up
dominates). Checks: CALL/RET with a callee that rewrites the caller's next
instruction, #DE in a callee inside a chain (default INT 0 handler, exit
127), and MANDCGA and MANDVGA slice by slice against the interpreter.
`native_block_runs` in the session snapshot counts returns to the loop.

Not yet: instructions after a branch or store inside one block (side
exits), protected-mode and 32-bit-stack forms, a smaller emitter for the
table. Callgrind on 1M MANDCGA steps showed the remaining time in the
native forms and the per-block code check. The register-effect allocation
identified in CALL's form is addressed below.

## Scalar native register effects (19 November 2026)

The shared Rust emitter now captures each `write_gpr` value in a scalar
local and emits its fixed register and width directly at retirement.
Generated CALL, RET, PUSH, POP, multiply/divide and string forms no longer
allocate a vector to stage named register effects. Reads still see incoming
registers, repeated writes commit in token order, and a later fault leaves
all staged register effects uncommitted. The emitter handles any number of
validated register effects without a fixed staging-array limit.

The same change applies to standalone generated-Rust backend comparisons
and the program-independent native table shared by PIL and Dine. All 2,685
forms were regenerated; their keys and constant-slot layouts are unchanged.
The generated source shrank by 409,405 bytes. This does not extend native
instruction coverage or establish a measured execution-time improvement.

`generated_named_register_effects_are_ordered_transactional_and_allocation_free`
compiles and runs both generated APIs with allocation counting enabled only
during execution. It checks incoming-register snapshots, 27 ordered effects
with repeated 8/16/32-bit writes, preservation of upper register bits, and a
late branch-limit fault. Both successful and faulting execution allocate
zero times, including with debug code generation.

Local Windows validation: `cargo test --locked --bin pil` passed 972 tests
(2 ignored), including full-state form/block comparisons and the 280-case
interpreter/generated trace suite. The native-enabled binary builds and
passes `--selftest`; regeneration with `--check`, portable-path and
program-independence self-tests also pass. The full suite exposed an existing
text-watch fixture writing its initial 07h attribute while expecting a change;
the fixture now writes 1Fh so its five-event expectation checks five changed
bytes.
