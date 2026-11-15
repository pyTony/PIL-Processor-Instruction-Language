# Project review and improvement plan — 15 November 2026

## Assessment

PIL/Dine has a useful shared execution architecture and substantial correctness
coverage. The next milestone should be a reproducible, dependable DOS workflow:
launch with normal drives, install across disks, run a program, and return cleanly.
Consolidate this before expanding the compiler or adding broad compatibility claims.

The program-independent architecture is the right constraint. Keep one PIL
engine and one Dine library, use guest-independent precompiled forms and guarded
blocks, and keep guest bytes in launchers. Preserve interpreter fallback.

## Scope and evidence

Reviewed local `dev` at `3ee34259ecce63ec12d146bbf72dc283dfda8fd4`, including
the existing uncommitted media/catalog changes. Those changes were preserved.
This is a targeted architecture, workflow and test review, not an exhaustive
instruction audit. No live GitHub issue/PR inventory or hosted CI result was
retrieved. Local remote-tracking refs do not prove current server state.

Checks performed on Windows:

| Check | Observed result |
| --- | --- |
| `cargo test --locked --quiet` | 864 passed, 2 ignored, 0 failed; 49.13 seconds |
| `python -m unittest discover -s tests/gui -q` | 90 run: 2 failures, 2 errors, 1 skipped |
| Three targeted frontend retries | All passed; configuration reader mocked to return empty defaults for two launch tests |
| `python tools/check_program_specific.py --self-test` | Passed, including project scan; 15 allow-list entries used |
| `python tools/check_portable_paths.py --self-test` | Self-test passed; scan reached cache write, then failed on read-only `.git/portable_paths_checked` |

The Rust build emitted an incremental-cache hard-link warning and copied files
instead. This was not a compilation failure. No fresh release benchmark,
interactive Windows media installation, Linux/Termux run, full external CPU
suite, generated-form freshness check or runtime-package verification was done.

## What is working well

- `src/precompiled/` and `src/block_compiler/engine.rs` implement shared native
  instruction forms and runtime block planning. Blocks validate context and code,
  stop at sensitive boundaries, and support guarded chaining. This is implemented,
  not merely a proposed integration.
- `tests/unit/precompiled.rs` checks interpreter equivalence, rewritten code,
  call/return chains, faults and graphics fixtures. Retain these as optimization gates.
- DOS services, memory, graphics and CPU semantics are shared with the Dine C ABI.
  CI checks both the CLI and shared library, generated forms, packaging and pixel output.
- Disk-set tests already cover read-only source images, retained CPU/RAM state,
  failed swaps, busy drives, manifest validation and session commands.
- The catalog configuration test checks preservation of C:, D:, E:, Z:, country
  and FPU settings. It passed as part of the Rust suite.
- Protected-mode and FPU work have explicit tests and documented limits. Avoid
  equating an application reaching its startup screen with full compatibility.

## Findings, in priority order

### P0 — Make frontend tests independent of the user's configuration

`tools/dos_terminal.py::main` merges `configured_frontend()` values before it
validates options. The local configuration enables `headless` and `shell`.
Consequently, `test_rust_graphics_option_requires_headless` starts a real session
instead of rejecting the arguments, and
`test_headless_shell_only_from_the_flag_not_the_name` receives `/K CTTY COM1`.
Both pass with `configured_frontend` mocked to return `{}`.

This demonstrates test contamination, not a demonstrated runtime regression.
Unit tests should explicitly supply their configuration and environment. Keep
separate integration tests for configuration precedence. Mock subprocess creation
in argument-validation tests so a failed assertion cannot launch a real engine.
Also document how a caller overrides an enabled boolean config option; current
positive-only flags do not express a false override.

The asynchronous frontend test requires submission in under 50 ms. It failed
at 85 ms while the Rust tests were running, then passed on retry. Replace this
host-speed threshold with synchronization proving that submission returns before
the worker is released, with a generous deadlock timeout. The TCP test failed
with Windows socket access denied in this sandbox; rerun on an unrestricted
host before diagnosing a serial/network defect.

### P0 — Validate the actual PR revision after automatic branch updates

`.github/workflows/update-prs.yml` merges dev into open same-repository PRs and
pushes with `GITHUB_TOKEN`. Its comments explicitly state these pushes do not
trigger Check; the updated result is tested after a later push or merge.

Require successful checks for the updated revision before merge. Select a
supported mechanism that explicitly checks that revision, or stop automatically
mutating PR branches and use a checked merge/update process. Verify branch
protection and current server settings before implementation; they were not
inspected here. Acceptance: an automatic update produces a recorded successful
Windows/Linux check for the new SHA before it can be merged.

### P1 — Make the portability cache optional

`tools/check_portable_paths.py::check` unconditionally writes a cache stamp into
`.git` after scanning. This makes an otherwise read-only validation fail in this
workspace. Treat cache-write failure as a cache miss for future runs: report the
scan result, optionally warn, and preserve nonzero exits for actual path findings.
Test writable-cache, unavailable-cache and invalid-path cases. This is a tooling
failure observed here, not evidence of a machine-specific path in project content.

### P1 — Finish the media workflow as a bounded deliverable

The current working tree includes new catalog, disk-set, native dialog and media
launch modules, launch scripts, tests and documentation. Do not mix their final
verification with unrelated CPU or compiler changes.

Use an original two-disk installer fixture to check the complete workflow:
launch from the root script; retain configured drives; write an installed file to
a writable destination; swap disks; verify the output; return to the catalog;
launch again. Exercise cancel, missing media, busy drive, guest mouse ownership,
keyboard shortcut and DOS exit versus host-window close. Verify the Windows UI
manually and the underlying session protocol on Linux/Termux.

`src/dos_media_launch.rs` selects A: versus E: using image size and uses 7-Zip
for images above 64 MiB. Prefer validated container metadata or an explicit
choice over size alone. Verify extracted layout before announcing a usable
mount, and expose incomplete extraction clearly. Its unconditional message that
D: is writable is stronger than `catalog::open_program` guarantees: configured
D: is preserved and may be a read-only image. Derive destination guidance from
the mounted drive's capabilities. These are review findings; no failed real
installation is claimed.

### P1 — Reconcile current status and historical evidence

The README leads with 0.35 while Cargo declares 0.38.0-alpha.1.
`BLOCK_COMPILER.md` first says the kept modules are not wired into a command,
then documents the integrated implementation later in the same file.
`PROTECTED_MODE_PLAN.md` mixes older open/in-review status with later progress;
for example INT 15h AH=88h already reports extended RAM in `dos_services.rs`.
E801h/E820h are deliberately refused for the declared AT BIOS profile, so they
should not be treated as accidental omissions without a profile decision.

Add a concise current capability table with implemented subset, known limit,
test command and last verified revision. Move historical assertions under clearly
dated headings. Turn the very short TODO into links to a small ordered backlog.
Keep FPU phase records historical and derive the current subset from current code
and tests; older phase limitations need not describe today's implementation.

### P2 — Establish performance evidence before enlarging the compiler

Historical block-engine measurements are promising but were not reproduced in
this review. Record interpreter/default comparisons from the same revision,
release binary, inputs and frontend. Include output equality, guest instruction
counts, native coverage, elapsed time, startup cost and host/build metadata.
Keep paths portable and third-party binaries out of git.

Use short CPU fixtures, graphics, file I/O and interactive latency as separate
workloads. Report repeated samples and distinguish cold startup from execution.
Investigate fallback and dispatch costs before adding more precompiled forms.
Protected-mode blocks are a later increment: `Blocks::plan` currently refuses
paging and non-real-mode execution. Preserve that boundary until equivalent
fault, interrupt, paging and invalidation tests exist.

Add boundary-focused differential cases where coverage is missing: pending input
between chains, interrupt boundaries, cache-capacity eviction, interior branch
targets, context changes and code writes. First inspect existing coverage to
avoid duplicating tests. Do not assume these are known defects.

### P2 — Refactor along semantic boundaries, one small extraction at a time

`src/main.rs` is about 183 KB and serves both binary and library targets. It
contains parsing, execution, emitter helpers and CLI concerns. Large decoder,
DOS-service and filesystem modules also increase review cost.

Move the reusable core behind a normal library root and keep CLI dispatch thin,
then extract parser/IR, execution and emission separately. Preserve the Dine ABI
and shared semantics. Do not combine formatting sweeps or behavior changes with
these moves. Require unchanged backend comparisons and generated-form checks.
Split CI into a quick correctness tier and separately visible expensive oracle,
release and packaging jobs while retaining required coverage.

## Proposed implementation sequence

| Increment | Deliverable | Completion evidence |
| --- | --- | --- |
| 1 | Hermetic frontend tests and resilient portability checker | Tests pass with conflicting local defaults; validation works with read-only git metadata; real path violations still fail |
| 2 | Check automatically updated PR revisions | Windows/Linux results attached to the updated SHA before merge |
| 3 | Complete media/catalog work | Synthetic two-disk install, normal drives retained, source hashes unchanged, installed output verified, clean return and relaunch; Windows UI check |
| 4 | Current capability/status page | README, TODO and plans agree with tested code; historical results remain dated |
| 5 | Reproducible performance baseline and boundary tests | Interpreter/default outputs agree; repeated timings and fallback coverage recorded |
| 6 | One measured optimization or core-module extraction | Demonstrated gain or reduced coupling with unchanged correctness and ABI checks |

After these increments, rank compatibility work by reproducible stops in actual
workloads, then model the relevant DOS/BIOS/CPU behavior with original fixtures.
Do not add guest names, addresses or signatures to general code. XMS, additional
FPU semantics and protected-mode acceleration should each have a separate scope
and acceptance test, rather than becoming one broad 'run more programs' change.

## Recommended next action

Start with increment 1 while preserving the media work in progress. It is small,
has failures reproduced in this review, and makes subsequent results trustworthy.
Then finish the media workflow before starting another large runtime feature.

## Implementation progress — 15 November 2026

Increment 1 implemented locally:

- DOS frontend tests use a temporary empty configuration through `PILDOS_CONFIG`,
  leaving the user's configuration untouched. A separate test verifies explicit
  config precedence over the environment, inherited shell defaults and an explicit
  CLI FPU override. Argument validation verifies no subprocess is started.
- The asynchronous submission test uses events to prove the worker remains blocked
  when submission returns, replacing the 50 ms scheduling assumption.
- A portability-cache write error now reports an informational message and leaves
  successful validation successful. Three regression tests cover writable cache,
  unavailable cache and rejected host paths with either cache condition.
- The full frontend run after the initial fixes ran 93 tests: 91 passed, one
  skipped and one TCP error caused by sandbox socket permission denial. The
  subsequently added configuration-precedence test passed separately.
- The portability self-test and full 449-text-file scan passed despite read-only
  git metadata; the program-independence self-test and project scan passed.

No CPU/runtime semantics changed. Positive-only boolean CLI overrides remain a
separate interface decision. CI branch-update changes and media acceptance work
are still pending. These changes are local and uncommitted.
