# Modular CI

`Check` runs on every PR to dev and push to dev. There are no workflow-level
path exclusions. Configure branch protection to require **Required PR checks**:
this stable aggregate fails if selection or any selected job fails or is cancelled.
Retire required contexts belonging to the old monolithic Rust jobs when adopting
this policy; this change does not alter repository protection settings.

The always-running selection job checks portable paths, program independence,
runtime documentation links and the routing regression tests. The routing policy
is in `tools/ci_changes.py`:

| Changed inputs | Required checks |
| --- | --- |
| Markdown and checksum manifest only | Selection and documentation checks |
| Web page/server and its HTTP tests | Web tests on Linux and Windows; no Rust build |
| Other reviewed frontend files and GUI tests | Engine-backed GUI tests on Linux and Windows, sharing the Rust build job |
| Shared runtime startup/build drivers | Frontend, shared-library ABI and delivery checks; no instruction oracles, CPU/backend comparisons or performance benchmarks |
| Shared window rendering and container import | Focused DOS and container Rust tests, frontend, integration and library checks |
| Dedicated DOS source/tests and DOS fixtures | DOS Rust tests, frontend, DOS integration and Dine library checks on Linux/Windows |
| CPU/shared Rust, ISA, Cargo, other unit tests or fixtures | Full Rust tests and selftests on Linux/Windows; pure instruction oracles once on Linux; frontend, integration, library and CPU/backend modules |
| Export/package tools and their contract tests | Frontend tests and delivery on Linux/Windows |
| Benchmark/compare scripts | Core checks, pure oracles and performance on Linux/Windows |
| Other files, CI/routing changes, unavailable diff | All modules |

Dedicated `src/dos.rs`, `src/dos_*`, `tests/unit/dos*` and `programs/dos/`
changes do not select instruction oracles, the full CPU/backend suite or DOSBox
CPU comparisons. Shared window/display and container-import files have the same
explicit route. They select Rust tests with the filters `dos` and `containers::`,
plus DOS/emulator integration and Dine ABI checks. Compilation still builds the
shared crate; this distinction reduces test execution, not compiler scope.
CPU/shared-state changes retain the full suite.

`tools/runtime_startup.py`, `tools/build_dine.py` and the reviewed native-launcher
wrappers explicitly select frontend, library and delivery checks. Their startup
unit test is already run by the always-running selection job. Unclassified new
files and changes to CI/routing itself still select all modules. Consequently,
the PR that changes these rules receives a full validation once; later startup
or rendering PRs benefit from the narrower routes. Package smoke tests may still
run small selftests to verify that the delivered executable works.

Selections accumulate across all changed files. PRs compare merge base to head,
not just the last commit. Renames inspect both old and new names. New files are
conservative until explicitly classified. Documentation checks still run for
documentation-only changes. Expensive checks remain mandatory whenever selected.

`Full CI` separates broad DOS/emulator integration, generated/backend and Dine library
validation, standalone export/packages, and performance comparisons into selectable
modules. Rust-dependent modules share one job and target directory per OS, so
selecting more checks does not create separate cold builds. Cargo dependencies
and workspace outputs are cached with `Swatinem/rust-cache`; cache reuse across
runs is best-effort, not a promise of incremental recompilation. Debug, release
and library feature configurations still require their own compiler work.
Pure oracles and web-only tests run separately without waiting for compilation.
The full GUI suite launches the engine and runs after its shared debug build.
CPU/shared engine changes run integration, library and backend checks, while
standalone packaging and benchmarks normally run nightly. This trades immediate
package/performance detection for shorter engine PR checks; package and benchmark
tool changes still select those modules immediately.

The pure Python instruction/flag/memory oracles and deterministic fixture builders
run once on Linux. Rust execution, GUI behavior, shell/PowerShell launchers,
shared-library loading, package validation and performance retain Linux/Windows
coverage. All former Check commands are retained across these modules.

## Nightly and manual runs

Full CI runs daily at 01:23 UTC on the default branch (currently dev). GitHub
schedules only default-branch workflows, and may delay scheduled runs. It runs
all modules, including core checks, on both hosts where platform-sensitive.

In GitHub Actions select **Full CI**, **Run workflow**, then select dev or the PR
branch. Leave all module toggles enabled for complete validation. To check a PR
before merge, select its branch explicitly. Artifacts are uploaded by delivery.

With GitHub CLI:

```sh
gh workflow run full.yml --ref YOUR_BRANCH
gh run list --workflow full.yml --branch YOUR_BRANCH
gh run watch RUN_ID --exit-status
```

**Check** also supports manual full validation (the default). The PR refresh
workflow dispatches `Check` with `full=false` and `base_ref=dev` after its
GITHUB_TOKEN push, selecting checks from the entire refreshed branch diff.
Use these options only for affected checks; leave `full=true` for full validation.
Workflow files must exist on the default branch before GitHub exposes dispatch.
Neither workflow merges PRs. Superseded runs of the same workflow/ref are cancelled.
