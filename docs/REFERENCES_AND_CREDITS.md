# References and credits

The Rust/PIL implementation, monitor decoder, scripts and included guest program
were authored for this project with AI assistance. No external Cargo crates,
third-party emulator code, manual text, BIOS, OS image or external test dataset
is included. A project-wide licence for this code has not yet been selected.

The implementation was informed by Intel's 80386 Programmer's Reference Manual
(1986), consulted through university-hosted copies:

- [Manual contents](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/toc.htm)
- [2.5: operand selection and effective addresses](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s02_05.htm)
- [6.3: segment permissions, limits and expand-down bounds](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s06_03.htm)
- [17.1: operand/address sizes](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s17_01.htm)
- [17.2: prefixes, ModR/M and SIB](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s17_02.htm)
- [Appendix C: status flags](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/appc.htm)
- [Appendix D: condition predicates](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/appd.htm)

Thanks to Intel's authors and the university hosts for these references, and
to the Rust and Termux projects for the build/run environment. Their toolchains
and source are not bundled. Reference credit does not grant redistribution
rights to the referenced material.

The guest test program is an original fixed 204-byte image. Port 0xe9 RX/TX
and HLT stop behaviour are monitor conventions. This release does not import
Tiny386, QEMU, TinyEMU, SingleStepTests, SeaBIOS or FMOPL code/data.

Detailed development provenance is retained on the repository's dev branch.

The Hello-world image is original project code using the same documented MOV,
OUT and HLT monitor profile. No additional source material was imported.

The factorial and counted-loop programs and timing commands are original
project work using the same documented instructions. Factorial results use
integer arithmetic; host timings use the Rust standard library. No new
third-party implementation or dataset is bundled.

The bounded decoder/PIL cache is original project code using Rust's standard
library. It introduces no new external dependency, instruction semantics or
third-party source material.

The alpha compiled guest runner and binary trace format are original project
work generated from the same PIL definitions. Rustc and the standard library
provide compilation and timing. Operational references for the shared checkout:
[Cargo Book](https://doc.rust-lang.org/cargo/reference/build-cache.html) and
[Microsoft WSL filesystems](https://learn.microsoft.com/en-us/windows/wsl/filesystems).
No third-party source or documentation text is imported.

The direct x86-64 baseline uses original register-only inline assembly.
[Rust Reference](https://doc.rust-lang.org/reference/inline-assembly.html) and
[Microsoft KB896458](https://learn.microsoft.com/en-us/troubleshoot/windows-client/application-management/x64-windows-not-support-16-bit-programs)
were consulted for inline-assembly constraints and Windows 16-bit limitations.

## 0.11 instruction expansion references (consulted 2026-10-26)

Intel 80386 Programmer's Reference Manual, chapter17 opcode definitions and
AppendixA group tables; MIT/Ardent Tool mirrors, used for encodings and flags:
- https://pdos.csail.mit.edu/6.828/2018/readings/i386.pdf
- https://pdos.csail.mit.edu/6.828/2018/readings/i386/OR.htm
- https://pdos.csail.mit.edu/6.828/2018/readings/i386/TEST.htm
- https://pdos.csail.mit.edu/6.828/2018/readings/i386/NOT.htm
- https://pdos.csail.mit.edu/6.828/2018/readings/i386/DEC.htm
- https://www.ardent-tool.com/CPU/docs/Intel/386/manuals/prref386/INC.htm
- https://www.ardent-tool.com/CPU/docs/Intel/386/manuals/prref386/NEG.htm

Reference-only use; no manual text, source code or data tables imported.
Descriptions, operand binding, fixtures and tests are original project code.
The old INC listing has an FF /6 dword typo; AppendixA group5 identifies
/0 as INC at both widths. /6 PUSH remains unsupported.

## 0.12 signed multiplication reference (consulted 2026-10-26)

Intel 80386 Programmer's Reference Manual, chapter17 IMUL entry:
https://pdos.csail.mit.edu/6.828/2018/readings/i386/IMUL.htm
Used for0F AF,69/6B encodings, signed fit and flag classification. Reference
only; no text/code imported. Generic sext/mul implementation, IMUL description,
fixtures and tests are project originals. Thanks to Intel and the MIT mirror.

## 0.13 shifts and conditional effects (consulted 2026-10-26)

Intel80386 PRM SAL/SAR/SHL/SHR opcode entry, count mask and encodings:
https://pdos.csail.mit.edu/6.828/2018/readings/i386/SAL.htm
Intel SDM Vol2B SAL/SAR/SHL/SHR flags clarification (zero-count preservation,
undefined AF and SHL/SHR CF at count>=width):
https://cdrdv2-public.intel.com/782151/253667-sdm-vol-2b.pdf
Reference only; no text/code imported. Original descriptions, generic PIL
operations, decoder bindings and regression fixtures. No new dependencies.

## 0.14 Mandelbrot/control-flow/image references (consulted2026-10-26)

- Intel80386 PRM Jcc: https://pdos.csail.mit.edu/6.828/2018/readings/i386/Jcc.htm
- Intel80386 PRM JMP: https://pdos.csail.mit.edu/6.828/2018/readings/i386/JMP.htm
- Netpbm PGM specification: https://netpbm.sourceforge.net/doc/pgm.html
- Microsoft bitmap storage: https://learn.microsoft.com/en-us/windows/win32/gdi/bitmap-storage

Used for relative-branch encodings and image container layout only. Original
assembler, guest algorithm, mathematical references, image writers and tests;
no external source code/assets imported. Thanks to the documentation authors
and maintainers. Escape-iteration PGM is an intensity visualization, not a
calibrated luminance image. No new dependencies or firmware reuse.

## Full-image backend comparison

Original extension of the project PIL Rust emitter, trace parser and Mandelbrot
workload. No additional external code, dependencies or reference material used.
Thanks to Tony for the Windows full-image reference-checked render report.

## Extension/address instructions —0.15

Intel80386 Programmer's Reference Manual1986, Chapter17 entries, consulted
26 October2026 through the Harvard HTML mirror:
- MOVSX: https://read.seas.harvard.edu/~kohler/class/aosref/i386/MOVSX.htm
- MOVZX: https://read.seas.harvard.edu/~kohler/class/aosref/i386/MOVZX.htm
- LEA: https://read.seas.harvard.edu/~kohler/class/aosref/i386/LEA.htm
- CBW/CWDE: https://read.seas.harvard.edu/~kohler/class/aosref/i386/CBW.htm
- CWD/CDQ: https://read.seas.harvard.edu/~kohler/class/aosref/i386/CWD.htm

Used for encodings, source/destination sizes, preserved flags and LEA offset-only
semantics. Original PIL/Rust/Python implementation and tests; no manual text,
external implementation, test vectors or dependencies imported. Same-width
MOVSX/MOVZX forms are outside the documented monitor profile. Generic source
binding validation and original mathematical oracles are project work.

## Same-width extension coverage follow-up

User-supplied Claude release/ISA smoke report recorded separately from hosted CI.
Its16-to16 compatibility claim prompted another review of Intel386 MOVSX/MOVZX
entries and SingleStepTests/80386 metadata/listing:
https://github.com/SingleStepTests/80386/blob/main/80386.csv
https://github.com/SingleStepTests/80386/tree/main/v1_ex_real_mode
The CSV advertises16/32-bit destinations with16-bit sources for0FB7/0FBF;
unprefixed hardware-derived test archives are listed. This is metadata review,
not an execution/validation of those vectors. No test data or external code is
imported. Thanks to Daniel Balsom and contributors for the hardware-test work.
The0.15alpha.2 monitor deliberately rejected16-to16 forms. Feature matrix clarification
is queued for the next runtime export; execution semantics are unchanged.

Intel386 Section17.2 ModR/M tables also consulted to correct the reported
encoding:0F BF F8 selects DI,AX;DI,DI would require0F BF FF with16-bit operands.
https://read.seas.harvard.edu/~kohler/class/aosref/i386/s17_02.htm

## Same-width MOVSX/MOVZX support —0.15alpha.3

The earlier coverage restriction is now removed, using the existing equal-width
sext/zext behavior rather than a special decoder-side copy. Source references:
Intel386 MOVSX/MOVZX entries and ModR/M tables; SingleStepTests/80386 CSV rows
0FB7/0FBF (r16/32 with r/m16) and archive inventory, as recorded above. The
original1986 instruction tables list widening forms only; broader encoding
metadata motivates this compatibility increment. Raw hardware vectors have not
been imported or executed. Register, alias, read-only-memory and fault checks
are original project tests; demo bytes are independently authored for this project.

## External validation suites reviewed26 October2026

SingleStepTests/80386 revision459d49fbe6280e9ed46fee887b58dacd9cb880ab:
README,80386.csv and LICENSE consulted. Software is MIT; data public domain
unless otherwise noted. Captures use80386EX,not a386DX cycle model.
https://github.com/SingleStepTests/80386/blob/459d49fbe6280e9ed46fee887b58dacd9cb880ab/LICENSE

barotto/test386.asm revisioncfd052d1e64d5375dea5a681c1eadeed64ceda2c:
README and COPYING consulted. README specifies GPLv3-or-later. Intended as a
separate external BIOS-test ROM after machine-level prerequisites exist.
https://github.com/barotto/test386.asm/blob/cfd052d1e64d5375dea5a681c1eadeed64ceda2c/README.md
https://github.com/barotto/test386.asm/blob/cfd052d1e64d5375dea5a681c1eadeed64ceda2c/COPYING

No third-party tests, code or ROMs imported/executed. Thanks to Daniel Balsom,
SingleStepTests contributors and barotto for making these validation resources
available,and to Tony for suggesting them. Detailed integration work stays ondev.
