## No program-specific code in general files

Do not put code for one guest program into PIL's general files (src/, isa/,
build.rs, tools/dos_terminal.py): no program or file names, hashes, PSP or
command-line checks, and no addresses or values taken from one program's run.
Model the documented DOS/BIOS/CPU behaviour instead and test it with an
original fixture. The program's history goes in docs/ or a test. The runtime
JIT / native-block path, #[cfg(test)] items, tests, fixtures, docs and
launchers are exempt. `python tools/check_program_specific.py --self-test`
enforces this in CI; an interface name may be listed with a reason in
tools/program_specific_allow.txt.

<!-- mermaid-ai-skills:start -->
## Mermaid Diagrams

When the user asks to create, edit, or visualize a diagram, follow the
instructions in `.github/instructions/mermaid.instructions.md`.
<!-- mermaid-ai-skills:end -->
