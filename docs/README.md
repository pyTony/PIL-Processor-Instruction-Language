# PIL documentation overview

Start here to find the language specification, executable examples, user guides
and development plans. The complete documentation lives on the
[dev branch](https://github.com/pyTony/pil_proof/tree/dev).
The [main branch](https://github.com/pyTony/pil_proof/tree/main) is a separate
runtime delivery and contains a smaller set of user documentation.

## What PIL does

PIL describes an instruction as typed bit-vector calculations and explicit
register, flag, memory and control-flow effects. The prototype validates these
statements, interprets them, or emits Rust source for offline compilation.
Both backends share the instruction descriptions. The binary decoder supplies
operands and context; devices and the surrounding PC platform are separate.

## Start with these

| Your question | Read this |
|---|---|
| What syntax and behavior are implemented? | [PIL language reference](LANGUAGE.md) — current grammar, types, bindings, flags, memory, retirement and execution APIs |
| What does actual PIL look like? | [Executable instruction descriptions](../isa/i386/) — the `.pil` files embedded by the runtime |
| How do I build and run the project? | [Project README](../README.md) and [debugger guide](DEBUGGER.md) |
| Which x86 instructions and emulator features work? | [Delivery feature matrix](../delivery/source/FEATURE_MATRIX.md) and the instruction-profile sections of the [debugger guide](DEBUGGER.md) |
| What is the larger design? | [Original language design proposal](design/processor_instruction_language_v01.md) — architectural goals and proposed syntax, including features not implemented |

The **language reference** defines what the current parser accepts. The
**design proposal** describes a broader future system. The **feature matrix**
describes the delivered emulator subset. These documents answer different
questions; proposed syntax and plans are not implementation claims.

## Executable examples and implementation

| Source | What to inspect |
|---|---|
| [ADD](../isa/i386/add.pil) | Widened arithmetic and explicit flag calculations |
| [ADC](../isa/i386/adc.pil) | Reading incoming carry |
| [XCHG](../isa/i386/xchg.pil) | Snapshot reads and two staged register writes |
| [LAHF](../isa/i386/lahf.pil), [SAHF](../isa/i386/sahf.pil) | Packing status flags into AH and partial flag writes |
| [CMC](../isa/i386/cmc.pil) | Concrete carry input and a defined flag write |
| [SHL](../isa/i386/shl.pil) | Explicit count masking and conditional flag effects |
| [Memory ADD](../isa/i386/add_ram.pil) | Load, calculate and stage a store |
| [Conditions](../isa/i386/conditions.pil), [SETcc](../isa/i386/setcc.pil), [Jcc](../isa/i386/jcc.pil) | Composed predicates, conditional results and branches |
| [Parser, interpreter and Rust emitter](../src/main.rs) | Implemented token validation and both execution paths |
| [Binary decoder](../src/guest.rs) | Encodings, operands and decoded instruction context |
| [Address/segment support](../src/address.rs), [RAM](../src/memory.rs), [flow context](../src/flow.rs) | Shared access checks, memory model and error/context types |
| [Compiled backend verifier](../src/native.rs), [unit tests](../tests/unit/), [verification tools](../tools/) | Full-state comparisons and independent checks |

## Using and delivering the emulator

| Document | Purpose |
|---|---|
| [FAT16 and ZIP containers](CONTAINERS.md) | Open FreeDOS images,expand packages,list/extract files and feed the DOS probe |
| [DOS COM/EXE probe](DOS_PROBE.md) | Import DOS executables, run original hello examples and rank first instruction/service blockers |
| [Debugger guide](DEBUGGER.md) | Launchers, commands, demos, memory inspection, serial, comparisons and supported instruction forms |
| [Feature matrix](../delivery/source/FEATURE_MATRIX.md) | Supported, partial and planned delivery features |
| [Release notes](../delivery/source/RELEASE_NOTES.md) | User-facing changes by delivered version |
| [Native package guide](../delivery/README.md) | Running packaged binaries and their launchers |
| [Runtime-source README](../delivery/source/README.md) | Documentation prepared for the exported source delivery |
| [GitHub and delivery workflow](GITHUB.md) | dev/main separation, runtime exports and contribution procedures |

## Development plans and verification

| Document | Purpose |
|---|---|
| [Development status](development/STATUS.md) | Work log, verification evidence and continuation notes; newest entries first |
| [Roadmap](ROADMAP.md) | Emulator and portability goals, with historical planning notes |
| [386 instruction completion plan](development/ISA_COMPLETION_PLAN.md) | Ordered instruction-family work, dependencies and acceptance gates |
| [Machine-readable ISA backlog](development/ISA_BACKLOG.json) | Family/form inventory; not an opcode-completion percentage |
| [Host CPU oracle](development/HOST_CPU_ORACLE.md) | Native CPU comparisons, flag policies, provenance and artifacts |
| [External test suites](development/EXTERNAL_TEST_SUITES.md) | Pinned suites, licensing and adapter plans; distinguish plans from executed suites |
| [x86-64 verification plan](development/X86_64_VERIFICATION_PLAN.md) | Modern host verification and future true 64-bit guest support |
| [OS compatibility targets](development/OS_COMPATIBILITY_TARGETS.md) | Future Windows editions and machine requirements; not current boot support |
| [Tiny386 adapter plan](platform/tiny386-adapter.md) | Proposed device/platform boundary and reuse considerations |

## History, sources and handoff

| Document | Purpose |
|---|---|
| [Changelog](CHANGELOG.md) | Development changes by version |
| [Revision history](REVISION_HISTORY.md) | Detailed changes, verification evidence and user-reported results |
| [Machine-readable revision history](revision_history.json) | Structured releases, checks and measurements |
| [References and credits](REFERENCES_AND_CREDITS.md) | Manuals, attribution, third-party material and licensing findings |
| [Source checksums](SHA256SUMS) | Integrity manifest for tracked project sources and documentation |
| [Historical v0.4 README](history/v04_README.md) | Archived milestone snapshot; not the current reference |
| [Continuation handoff](../CLAUDE.md) | Working notes for resuming development; older entries retain their historical context |

Use the current language reference and executable sources for exact behavior.
Consult release-specific evidence for what was tested: a passing backend
comparison does not by itself establish full 386 or operating-system compatibility.
