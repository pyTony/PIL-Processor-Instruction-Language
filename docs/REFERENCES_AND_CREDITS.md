# References, provenance and credits

Maintained with the project. Last updated: 26 October 2026 (v0.15.0-alpha.3).

## What this release contains

The Rust/PIL/Python implementation in this prototype was authored for this
project during the conversation. No Tiny386, Intel manual, SingleStepTests,
QEMU, TinyEMU, SeaBIOS or FMOPL source/data is bundled in this release.
No external Cargo crates are used. Rust's standard library and toolchain
are used when building; their source is not included.

The design draws on the references below. Consulting a reference is not the
same as having permission to redistribute its contents. A reference/thanks
entry does not substitute for retaining an imported component's actual licence.
No project-wide distribution licence has yet been selected for our own code.

## Reference register

| ID | Material / author or project | How used | Current status |
|---|---|---|---|
| INTEL-386 | Intel, 80386 Programmer's Reference Manual, 1986 | Architectural background, widths, flags, instruction and system-mode requirements | Consulted; hosted manual copies, no manual text bundled |
| TEST386 | barotto/test386.asm and contributors | Later BIOS-replacement integration tests | README/COPYING inspected; GPLv3-or-later; no source/ROM imported or run |
| SST-386 | Daniel Balsom and SingleStepTests contributors, 80386 tests | Candidate hardware-derived tests and 80386.csv metadata | README,CSV,LICENSE inspected; software MIT,data public domain unless noted; no vectors imported/run |
| TINY386 | Chunhui He and Tiny386 contributors | ESP32 feasibility, device implementation and adapter planning | README, licence, instruction file and selected PC/VGA sources inspected; no code imported |
| ESP-S3 | Espressif Systems, ESP32-S3 product and ESP-IDF documentation | Host architecture and memory/execution constraints | Consulted; no code copied |
| ESP-32 | Espressif Systems, original ESP32 memory documentation | Distinguish original ESP32 memory limitations from S3 | Consulted; no code copied |
| MS-WFW | Microsoft KB Q126746, archived copy | WfW 3.1/3.11 hardware and configuration distinctions | Consulted; no article text bundled |
| TERMUX-RUST | Termux project, rust package/build setup | Rust installation on the phone | Consulted; no package source copied |
| CRANELIFT | Bytecode Alliance, Cranelift | Early backend feasibility research | Website consulted; not a dependency |
| REMILL | lifting-bits / Remill contributors | Early machine-instruction lifting research | README consulted; not a dependency |
| FEX | FEX-Emu contributors | Early Wine/ARM64 translation research | README consulted; not a dependency |
| BOX64 | ptitSeb and Box64 contributors | Early flag/runtime translation research | Documentation consulted; not a dependency |
| WINE | Wine contributors | Earlier Wine emulator-interface feasibility research | Release material consulted; not an integration in this release |

## Instruction-to-reference mapping

All formulae below are our descriptions, checked against an independent
arithmetic oracle. They are not extracted Intel pseudocode or a hardware test
suite. The rows below retain early verification milestones. The current monitor
implements the documented encoded operands and segmented/RAM checks; guest
exception delivery remains future work.

| Implementation | Semantics reference | Verification evidence |
|---|---|---|
| add.pil | Intel ADD and Appendix C | v0.1 Rust tests passed on user's Termux; v0.2 Python oracle |
| sub.pil | Intel SUB and Appendix C | v0.2 Python oracle; user reports all 14 v0.2 Rust tests passing |
| cmp.pil | Intel CMP and Appendix C | v0.2 Python oracle, including unchanged destination |
| adc.pil | Intel ADC and Appendix C | v0.2 Python oracle with incoming CF=0 and CF=1 |
| sbb.pil | Intel SBB operation and Appendix C | v0.2 Python oracle with incoming borrow=0 and borrow=1 |
| and.pil | Intel AND and Appendix C (AF undefined) | v0.3 Python oracle and Rust tests supplied |
| conditions.pil / setcc.pil | Intel Appendix D truth conditions; SETcc byte result and unchanged flags | v0.3 truth-table and CMP-sequence tests supplied |
| write_if_z.pil | Project-defined generic predication example | Not an x86 CMOV instruction; no external code used |
| AX/AL/AH views | Intel 386 register/operand model | v0.1 Termux register-view tests; expanded v0.2 tests supplied |

For the hosted SBB page, use the mathematical Operation section: the prose
Description appears to interchange operand names. Our SBB definition computes
destination minus source minus incoming CF. Do not mechanically import
human-readable instruction descriptions without reviewing inconsistencies.
Some hosted SETcc table entries also have apparent AND/OR or synonym typos; our condition formulas follow Appendix D, with signed/unsigned comparisons as independent tests.

v0.2 is now user-verified in Termux: all 14 Rust tests passed in a reported 2.57 s. v0.3 Rust tests remain unexecuted in this authoring environment. See REVISION_HISTORY.md for version-specific evidence.

## Primary and project links

### Intel manual (Intel publication, university-hosted copies)

- [Contents](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/toc.htm)
- [Operand/address/stack widths, section 17.1](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s17_01.htm)
- [ADD](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/ADD.htm)
- [SUB](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/SUB.htm)
- [CMP](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/CMP.htm)
- [ADC](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/ADC.htm)
- [SBB](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/SBB.htm)
- [Condition truth tables, Appendix D](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/appd.htm)
- [SETcc (Intel manual hosted by MIT)](https://pdos.csail.mit.edu/6.828/2004/readings/i386/SETcc.htm)
- [AND](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/AND.htm)
- [Status flags, Appendix C](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/appc.htm)
- [Paging, section 5.2](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s05_02.htm)
- [Virtual 8086 transitions, section 15.3](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s15_03.htm)

### Hardware tests and candidate PC devices

- [SingleStepTests/80386](https://github.com/SingleStepTests/80386)
- [Tiny386](https://github.com/hchunhui/tiny386)
- [Tiny386 licence](https://github.com/hchunhui/tiny386/blob/master/LICENSE)
- [Tiny386 instruction definitions](https://github.com/hchunhui/tiny386/blob/master/i386ins.def)
- [Tiny386 PC header](https://github.com/hchunhui/tiny386/blob/master/pc.h)
- [Tiny386 PC integration](https://github.com/hchunhui/tiny386/blob/master/pc.c)
- [Tiny386 VGA implementation and MIT notice](https://github.com/hchunhui/tiny386/blob/master/vga.c)

### Platform and historical software

- [Espressif ESP32-S3](https://www.espressif.com/products/socs/esp32-s3)
- [Original ESP32 memory types](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-guides/memory-types.html)
- [ESP32 external RAM](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-guides/external-ram.html)
- [ESP32-S3 external RAM](https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/api-guides/external-ram.html)
- [Archived Microsoft KB Q126746](https://ftp.zx.net.nz/pub/archive/ftp.microsoft.com/MISC/KB/en-us/126/746.HTM)
- [Termux Rust package](https://github.com/termux/termux-packages/blob/master/packages/rust/build.sh)
- [Termux Rust setup instructions](https://github.com/termux/termux-packages/blob/master/scripts/build/setup/termux_setup_rust.sh)

### Earlier translator/backend research

- [Cranelift](https://cranelift.dev/)
- [Remill](https://github.com/lifting-bits/remill)
- [McSema and its limitations](https://github.com/lifting-bits/mcsema)
- [FEX](https://github.com/FEX-Emu/FEX)
- [Box64 usage and deferred flags](https://github.com/ptitSeb/box64/blob/main/docs/USAGE.md)
- [Wine release announcements](https://list.winehq.org/hyperkitty/list/wine-announce%40list.winehq.org/)

These links are reference locations, not pinned source revisions. No upstream
commit hashes are claimed. References were researched on 26 October 2026;
some individual instruction pages are related references rather than a claim
that every line has been audited.

## Third-party licence findings for possible future reuse

| Component | Observed licence information | Before inclusion |
|---|---|---|
| Tiny386 original project code | BSD-3-Clause, top-level LICENSE | Pin revision; preserve copyright, conditions and disclaimer |
| Inspected Tiny386 vga.c | MIT notice naming Fabrice Bellard | Preserve full applicable notice and any additional file notices |
| Other TinyEMU/QEMU-derived files | Tiny386 README says some parts are MIT | Inspect each selected file; do not infer all QEMU code is MIT |
| SeaBIOS | Tiny386 README identifies LGPL-3 | Track firmware separately and record actual source/build/licence details |
| Optional FMOPL | Tiny386 README identifies LGPL | Inspect exact licence/version and distribution obligations if enabled |
| SingleStepTests software/data | LICENSE reviewed: software MIT,data public domain unless otherwise noted | Retain software notices and check any per-file exceptions before importing |
| barotto/test386.asm | GPLv3-or-later per README; COPYING inspected | Keep as separate external ROM; retain applicable notices when redistributing |
| Intel manual content | Copyrighted reference; redistribution terms not established | Link/reference; do not bulk-copy prose or tables without establishing permission |

None of these proposed components is currently bundled, so no third-party
licence texts have been copied into this release as though it contained them.
Once code/data is imported, include the exact upstream notices under
LICENSES/ and a file-level provenance record.

## Import record required for every future borrowed component

Record component, author/copyright holder, upstream URL, exact revision,
upstream path, local path, licence/SPDX identifier where established,
local modifications, bundled notice path, attribution location, date and
verification method. For generated files, also record the source description
and generator revision. Use “unknown/not yet checked” instead of guessing.

## Thanks

- Intel for the architectural documentation.
- Daniel Balsom and SingleStepTests contributors for hardware-derived tests.
- Chunhui He and Tiny386 contributors for a compact PC-emulation reference.
- Fabrice Bellard and TinyEMU/QEMU contributors for reusable device-emulation work.
- Espressif, Rust and Termux contributors for the target and development tools.
- Tony for defining the project direction and compiling/testing the first
  interpreter and generated-source proof on his phone.

Acknowledgement does not imply endorsement or involvement by these projects.

## v0.4 references and next-target research

- Intel 80386 manual, Jcc: https://read.seas.harvard.edu/~kohler/class/aosref/i386/Jcc.htm
  Consulted for relative target calculation, operand-width wrapping, preserved
  flags and protected-mode code-limit fault conditions. No source/prose imported.
- Espressif, Rust toolchain installation: https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html
  Consulted for the espup-managed Xtensa compiler route for original ESP32.
- Espressif, Rust hardware overview: https://docs.espressif.com/projects/rust/book/introduction/hardware-overview.html
  Consulted to distinguish ESP32 Xtensa from ESP32-C RISC-V targets.
- Espressif, original ESP32 external RAM: https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-guides/external-ram.html
  Consulted for directly mapped PSRAM constraints; exact TTGO module unconfirmed.

Checked 26 October 2026. These are reference locations, not pinned source
revisions. No third-party source/data is included in this patch. UART mock,
REPL commands and patch installer are project-defined design/implementation.
The user reports v0.3 tests passing twice: 20/0 in 3.30 s, then 20/0 in
3.39 s after installation under ~/pil_proof. v0.4 Rust verification is pending.

## v0.5 provenance and handoff

SerialTransport, MockSerial, debugger commands, demos and tests were authored
for this project; no third-party source was imported. Existing materials and
licence findings remain above. Paths were reorganized into src/, isa/i386/,
tests/unit/, tools/ and docs/. See root CLAUDE.md for continuation context.
User confirmed v0.4: 28 passed, 0 failed, 4.30 s. v0.5 Rust tests are pending.

## v0.6 memory provenance

The RAM implementation, token effects, debugger commands and tests were authored
for this project. No third-party source or data was imported. Existing Intel
ADD/status-flag references apply to add_ram's arithmetic, while the restricted
RAM transaction/protection contract is a project design, not an extracted x86
MMU specification. Do not cite these flat RAM tests as validation of page-table,
segmentation, MMIO or hardware exception behaviour.

Upstream baseline inspected: pyTony/pil_proof, commit
cfd4143512497c8afd28df9ef6739a96fc62b9f0 (26 October 2026).
GitHub: https://github.com/pyTony/pil_proof
The complete materialized source tree was checked against its Git blob hashes.


## v0.7 addressing, monitor and delivery provenance

Consulted Intel's 80386 Programmer's Reference Manual (1986), university-hosted
copies, on 26 October 2026:

- [2.5, operand selection and effective addresses](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s02_05.htm)
- [6.3, segment type/limit checks and expand-down bounds](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s06_03.htm)
- [17.1, independent address/operand size and prefixes](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s17_01.htm)
- [17.2, encodings, ModR/M/SIB and displacement rules](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s17_02.htm)

address.rs, the limited monitor decoder, fixture builder and the 204-byte guest
test program are original project code. No manual text, third-party assembler,
emulator implementation or external test corpus was copied. Port 0xe9 is a
project monitor convention; RX waiting and HLT stopping are explicit harness
behaviour, not claims of complete PC device or privileged-instruction semantics.

GitHub Actions uses actions/checkout and actions/upload-artifact on hosted
Ubuntu/Windows runners; these are build services, not bundled source dependencies.
Generated native ZIPs carry this attribution register and source history.

The runtime export and user documentation templates are original project work.
Main and native archives carry a release-specific attribution summary; the
complete provenance register and revision records remain on dev.

Hello-world (v0.7.1): original MOV AL / OUT 0xe9 / HLT byte fixture and listing.
Uses the already-documented instruction/monitor references; no new materials
or third-party code imported.

Numerical workloads (v0.8.0) are original byte fixtures using existing documented
MOV, ADD, SUB, CMP and Jcc semantics. Expected factorials are ordinary integer
mathematics, independently cross-checked by Python math.factorial in the dev
builder. Timing uses Rust standard-library Instant, not a hardware-cycle model.
No new third-party code/data imported or external dependencies added.

Instruction cache (v0.9): original project implementation using Rust's standard
BTreeMap and Rc. No external implementation or library was imported. Encoded
byte/segment-context validation follows this monitor's existing fetch rules.
No new instruction semantics or reference materials are introduced.

Alpha compiled guest runner: original project code composed from the existing
validated PIL emitter and monitor fixture definitions. The shared trace format
uses exact bytes, not a third-party emulator oracle. The compiler and emitted
runner use Rust's standard library; no new Cargo crate or external source was
imported. Existing Intel instruction references still apply.

Additional operational documentation consulted on 26 October 2026:
- Rust project, Cargo Book, Build Cache: https://doc.rust-lang.org/cargo/reference/build-cache.html
  (CARGO_TARGET_DIR and profile-specific artifacts).
- Microsoft Learn, Working across file systems: https://learn.microsoft.com/en-us/windows/wsl/filesystems
  (shared Windows/WSL paths and cross-filesystem performance).
No text or source from these pages is bundled; user instructions are paraphrased.

Direct PC baseline: original bounded register-only ADD/SUB/JNZ loop using
Rust inline asm on x86-64. Rust Reference inline assembly documentation
(https://doc.rust-lang.org/reference/inline-assembly.html) informs operand and
clobber handling. Microsoft KB896458 documents lack of native 16-bit support
on Windows x64; the baseline therefore uses equivalent arithmetic in 64-bit
mode, rather than executing the monitor image directly. No source imported.

## Alpha.2 verifier diagnostics

Original trace parser, state-field diagnostics and regression tests. No new
external references, copied code or third-party dependencies. Phone timing
evidence is the user's supplied alpha.1 Termux console output.

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

## Typed debugger memory dumps

Original implementation and tests; no new external reference materials,
third-party code or dependencies. ASCII rendering is byte-based (0x20..0x7e).

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

## Instruction completion,64-bit verification and OS planning —27 October2026

Original development plans/TODO inventory; no implementation or hardware passes
added. References consulted:
- Intel386 instruction index and encoding chapter,Harvard mirror:
  https://read.seas.harvard.edu/~kohler/class/aosref/i386/c17.htm
  https://read.seas.harvard.edu/~kohler/class/aosref/i386/s17_02.htm
- Intel SDM portal,revision093 advertised at review:
  https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html
- Intel Volume1,253665-060US,September2016,register/operand environment:
  https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-1-manual.pdf
- AMD APM Volume3 listing24594 revision3.38 (listing only):
  https://docs.amd.com/v/u/en-US/24594_3.38_APM_Vol3_PUB
- Microsoft x64 instruction and WindowsABI references:
  https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/x64-instructions
  https://learn.microsoft.com/en-us/cpp/build/x64-calling-convention?view=msvc-170
- x86-64 psABI project located; specification to pin/review at harness implementation:
  https://gitlab.com/x86-psABIs/x86-64-ABI
- Microsoft archived KB126746 (WfW),KB138349 (Windows95),KB182751 (Windows98),
  KB126690 (NT4); exact archive URLs and NT3.51 vendor-announcement evidence/
  retrieval caveats in docs/development/OS_COMPATIBILITY_TARGETS.md.

No manual prose,OS media,implementation or test vectors imported. Modern-CPU
evidence is explicitly distinct from386-specific behavior and machine boot tests.
