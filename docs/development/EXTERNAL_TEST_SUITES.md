# External 80386 validation suites

Reviewed26 October2026 following the user's suggestions. Since 31 October 2026
the SingleStepTests real-mode vectors are executed by a development-only adapter
(below); the data is fetched at a pinned revision and never committed. Existing
165-test/39-case results are original project tests, not external-suite passes.
The user's941-file/1.7-million-case figures have not been independently counted.

## Pinned references and licences

| Suite | Inspected revision | Source and licence | Intended role |
|---|---|---|---|
| [SingleStepTests/80386](https://github.com/SingleStepTests/80386) | 459d49fbe6280e9ed46fee887b58dacd9cb880ab | README,80386.csv,LICENSE; software MIT, data public domain unless otherwise noted | First independent CPU oracle |
| [barotto/test386.asm](https://github.com/barotto/test386.asm) | cfd052d1e64d5375dea5a681c1eadeed64ceda2c | README,COPYING; GPLv3-or-later per README | Later whole-machine ROM diagnostics |

Pinned licences:
https://github.com/SingleStepTests/80386/blob/459d49fbe6280e9ed46fee887b58dacd9cb880ab/LICENSE
https://github.com/barotto/test386.asm/blob/cfd052d1e64d5375dea5a681c1eadeed64ceda2c/COPYING

Retain notices for any reused MIT software. Preserve the actual suite licence
and provenance if redistributing third-party material. No project-wide licence
for PIL's own code is selected by this review. Keep the GPL test ROM as a
separate test artifact; do not copy its implementation into the core by default.

## SingleStepTests adapter: implemented 31 October 2026

`tests/unit/sst80386.rs` reads the real-mode MOO files (gzip through the
project's own DEFLATE reader), loads each case's registers, selectors and RAM
bytes into a 16 MiB host RAM, decodes one instruction with the ordinary guest
decoder and runs it through the PIL interpreter (REP repeats until retirement).
The final EIP is compared one past the result, because every capture ends after
the following HLT. Registers, selectors, CR0 MSW bits, EFLAGS (with the suite's
RM32 masks) and every listed RAM byte (written and unchanged) are compared.

Run it with `python tools/verify_sst80386.py [--files 00,6601] [--no-generated]`.
The script clones the suite at the pinned revision into
`target/external-tests/SingleStepTests-80386`; nothing is committed or run by
plain `cargo test`. The report goes to `target/external-tests/sst80386/report/`
(`report.md` per file, `failures.tsv` per case with hash and bytes).

Outcomes are kept apart, and only `Pass` is a pass:
- not decoded (family from the decoder message; repeated prefixes separately),
- expected exception where PIL faulted or rejected the opcode (no delivery yet),
- EFLAGS differing only in bits PIL marks undefined (`UndefinedFlagsOnly`),
- undefined SIB forms (index 100b, nonzero scale), which the suite README says
  to filter unless emulated,
- `HltFetchFault`: the instruction completed but the injected HLT lies past
  the CS limit, so hardware took #GP fetching it,
- failures, named by the first differing field.

Generated Rust: PIL folds immediates into each program, so all cases would be
about 165,000 programs (about 1 GB of source). The adapter compiles the first
eight PIL-decoded cases per file into one runner and compares its outcome with
the interpreter's and with the hardware. Each file starts from zeroed RAM;
bytes left outside the listed addresses are counted as stray writes.

First run (local cloud session, revision 459d49fb, 941 files, 1,758,700 cases):
1,188,711 pass; 122,682 expected exceptions without delivery; 333,287 not
decoded; 93,329 undefined-flag-only; 6,023 undefined SIB; 43 HLT fetch faults;
100 monitor port operations; 1 revoked; 14,524 failures. Generated Rust ran
6,736 sampled cases with no outcome disagreement. No stray writes.

## Original plan: SingleStepTests CPU adapter

The captures use an Intel80386EX KU80C386EX33 with a16-bit external data bus.
They are useful for register/memory correctness; their cycle traces are not a
386DX timing model. Published real-mode cases assume16MiB writable RAM and
arbitrary CPU/segment state. Files use compressed MOO, not the older JSON test
layout. Revocation hashes and variant-specific undefined cases need attention.

Our4-KiB monitor, fixed entry0x100, limited prefix rules and absent guest
exception delivery prevent directly loading the suite as a monitor program.
Build a development-only one-instruction adapter rather than relocating tests
or rewriting their inputs to fit the demos. Preserve initial CS:IP, registers,
flags, segment bases/limits, all supplied bytes and expected post-state.
Use a host test memory context sized for the dataset, independently of the
small runtime monitor and eventual embedded RAM layout.

Implementation order:
1. Inspect the pinned MOO specification and parser licence before selecting or
   writing a reader. Keep external downloads/cache under target/external-tests.
   Pin revisions, archive hashes and case IDs; no moving network dependencies
   during an ordinary cargo test.
2. Admit an explicit subset of single instructions and report unsupported
   encodings, unsupported execution state, revoked cases and exception cases
   separately. Skips are not passes. Start with MOVSX/MOVZX8/16-bit sources,
   both operand widths and same-width forms; then valid-SIB LEA and ADD/SUB/CMP.
3. Run the same original input through PIL interpreter and generated Rust.
   Compare each independently with hardware register/memory expectations,
   including unchanged state and actual memory write sets. Agreement between
   our two backends alone is not external validation.
4. Derive a case-specific defined-flag mask from architectural rules and
   cross-check dataset metadata. Do not let the device under test's undefined
   mask decide what the checker ignores. Count0/1 shift flags may need stricter
   masks than opcode-wide metadata. Keep captured undefined values for diagnosis;
   our deterministic values/diagnostic markers are not hardware flag values.
5. Fail with revision/archive/case hash, bytes, initial state and first divergent
   field. Keep pass/fail/unsupported/revoked totals per opcode/prefix/model.
   Add a small pinned, licence-recorded regression selection once the adapter
   is trustworthy; keep larger runs as an optional development check.

Initially exclude cases requiring guest exception stack frames, privileged
state we do not model, unsupported prefix combinations or known undefined SIB
forms. Do not claim support for those through relocation, silent normalisation
or broad masking. Expand supported groups deliberately as the CPU grows.

## Later: test386.asm machine integration

This NASM project is a BIOS replacement, not a one-instruction data oracle.
Its README describes64/128-KiB ROMs mapped at conventional BIOS addresses and
high aliases, reset at0xFFFFFFF0, and accesses within the first1MiB. It reports
through configurable POST/LPT/COM/custom output ports. It exercises startup,
stack/CALL/RET, segment changes, real/protected modes and later system features.
Those machine facilities are not provided by our current monitor.

Keep its ROM separate from emulator implementation. Once reset/ROM mapping,
stack/control flow and the required bus I/O exist, build a pinned test ROM with
NASM and record its configuration, tool version and checksum. Capture POST and
text diagnostics. POST0xFF alone is insufficient: the0xEE arithmetic output
must also be compared with the project's reference. The README explicitly
calls the suite incomplete and says it was not tested on real hardware, so it
complements rather than replaces the hardware-derived oracle.

Credits: Daniel Balsom/GloriousCow and SingleStepTests contributors; barotto and
test386.asm contributors; PCjs/IBMulator ancestry as documented by that project.

## Related plans

[386 completion](ISA_COMPLETION_PLAN.md) orders the CPU/effect/system work.
[Modern CPU/x86-64 verification](X86_64_VERIFICATION_PLAN.md) adds a parallel
oracle for defined overlapping behavior and later true64-bit operations.
It cannot replace the386-specific hardware lane. [OS targets](OS_COMPATIBILITY_TARGETS.md)
describe later machine milestones,not suite passes or current boot support.
