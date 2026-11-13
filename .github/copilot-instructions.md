## PIL and Dine are program-independent libraries

PIL and Dine are always program-independent libraries, used the same way
everywhere. No tool, build step or CI job may generate an engine, backend,
Dine library or translator for one guest program; precompilation is not a
JIT, but it is not exempt. `Dine_<program>` launchers are fine: a loader
plus guest bytes on the shared dine library, no copy of Dine or PIL. See docs/development/BLOCK_COMPILER.md.

Do not put code for one guest program into PIL's general files (src/, isa/,
Cargo.toml and every script in tools/ except tests, benchmarks and fixture
builders): no program or file names, hashes, PSP or command-line checks, and
no addresses or values taken from one program's run. Model the documented
DOS/BIOS/CPU behaviour instead and test it with an original fixture. The
program's history goes in docs/ or a test. Only #[cfg(test)] items, tests,
fixtures, docs and launchers are exempt. `python
tools/check_program_specific.py --self-test` enforces this in CI; an
interface name may be listed with a reason in tools/program_specific_allow.txt.

<!-- mermaid-ai-skills:start -->
## Mermaid Diagrams

When the user asks to create, edit, or visualize a diagram, follow the
instructions in `.github/instructions/mermaid.instructions.md`.
<!-- mermaid-ai-skills:end -->
