# References, provenance and credits

Maintained with the project. Last updated: 26 October 2026 (v0.7.0).

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
| SST-386 | Daniel Balsom and SingleStepTests contributors, 80386 tests | Candidate hardware-derived tests and 80386.csv metadata | README inspected; tests/data not imported or run |
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
suite. Memory exceptions and encoded operands are not implemented yet.

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
| SingleStepTests data/metadata | Redistribution terms not audited in this work | Inspect applicable licences and provenance before import |
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
