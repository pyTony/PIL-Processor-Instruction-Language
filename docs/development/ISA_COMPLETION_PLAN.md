# 80386 instruction completion TODO

Updated 27 October 2026. This is a development plan, not newly implemented CPU
support. Verified baseline: v0.20.0-alpha.1, 210 tests, 49 backend trace
cases and 340,920 narrow native overlap cases on Windows/Linux. External suites
remain unexecuted. CLD/STD follow-up passed215 tests and50 backend cases on both hosts (CI40476098533).

See [machine-readable backlog](ISA_BACKLOG.json), [parallel x86-64 plan](X86_64_VERIFICATION_PLAN.md),
[external-suite integration](EXTERNAL_TEST_SUITES.md), and [OS targets](OS_COMPATIBILITY_TARGETS.md).
Backlog rows describe families/forms, not a percentage of implemented opcodes.

## Scope and approach

Complete documented 80386 integer/system behavior, with separate compatibility
work for undocumented/model-specific instructions and optional 80387 support.
Do not silently add 486+ features to the 386 model. A decoded mnemonic does not
imply complete widths, operands, encodings, prefixes, modes or exception behavior.

Start independent checking early: pinned 386-family cases for legacy behavior,
and a modern-PC oracle for defined common operations. Extend shared PIL into an
explicit 64-bit profile in parallel with the longer 386 system work. Neither
interpreter/emitter agreement nor a modern CPU replaces the legacy oracle.

## Ordered TODO phases

| Phase | Work | Acceptance gate |
|---|---|---|
| P0 | Form inventory, independent adapters, named multi-register effects, ordered memory/commit rules, mode context | Precise case/effect contract; existing regressions retained |
| P1 | Missing MOV/ALU/unary/SETcc forms; XCHG; CLC/STC/CMC, LAHF/SAHF, CLD/STD; conversion/LEA audit | Operand, alias, memory and flag tests; first modern-PC comparisons |
| P2 | PUSH/POP, PUSHF/POPF, near CALL/RET, indirect JMP, LOOP/JCXZ family | Nested procedures, independent stack/operand sizes, alias and fault tests |
| P3 | PUSHA/POPA, ENTER/LEAVE, rotates, SHLD/SHRD, bit tests/scans | Count/index/zero cases and explicit undefined-destination behavior |
| P4 | Implicit MUL/IMUL, DIV/IDIV, XLAT, decimal/ASCII adjusts | Wide results, quotient/remainder and #DE; legacy adjust oracle |
| P5 | MOVS/STOS/LODS/CMPS/SCAS, REP/REPE/REPNE | Single element first; then count/ZF termination, partial progress and restart |
| P6 | Full IN/OUT and INS/OUTS; real-mode segment/far-pointer/far-control forms; BOUND; LOCK | Port bus, device ordering, segment changes and far transfers |
| P7 | Descriptor tables/selectors, LMSW/SMSW/CLTS, 386 control registers and protected-mode forms | Descriptor validation, privilege and fault precedence, actual mode changes |
| P8 | INT/IRET, CLI/STI, architectural HLT, debug/test registers, paging, task/gate transfers, WAIT/ESC interface | Fault/trap frames, interrupt shadows, page faults, task state and halt/wakeup |
| P9 | Encoding/prefix/model audit; reset/ROM/A20/bus integration; external ROM milestones | Quantified forms and independent evidence; unsupported cases not counted as passes |
| U/F | Undocumented compatibility; separate 80387 instruction project | Separate scope, model evidence, reference and licence records |

## P0 design work required by the remaining instructions

- [ ] Define instruction-form metadata: opcode/group, widths and defaults,
  modes, prefixes, implicit bindings, defined/preserved/undefined outputs,
  memory accesses and retirement class.
- [ ] Generalize the existing staged destination/source writes to named register
  effects and implicit-register inputs. Register XCHG and its two-binding snapshot
  ordering are implemented and host-verified; arbitrary named effects and stack
  alias/overlap ordering remain to be specified and implemented.
- [ ] Replace the single supplied-address assumption with per-access address
  expressions. Encode pre/post-update rules, including POP-memory addressing
  based on the updated stack pointer.
- [ ] Keep simple faults before commit. For multi-store/system instructions,
  describe ordered architectural commit points and observable earlier writes.
  Do not promise rollback for MMIO or whole REP execution.
- [ ] Separate architectural #DE/#GP/#SS/#PF and delivery state from monitor
  diagnostics. Preserve fault versus trap return-IP rules and error metadata.
- [ ] Track operand, address and stack defaults independently, including CS
  default-size and SS stack-size attributes. Include new mode/cache state in
  decode-cache identity.
- [ ] Represent undefined register outputs where required, including BSF/BSR
  zero inputs. Concrete diagnostic values are not compiler poison.
- [ ] Build the independent one-step harness without rewriting vector addresses,
  state or bytes to fit the 4-KiB monitor. Test memory size is a host-harness
  choice, independent of runtime/embedded memory layout.

## Required edge cases

**Stack/control:** PUSH SP/ESP snapshots; POP SP/ESP; POP-memory address timing;
16/32-bit operands with 16/32-bit stacks; wrap, nesting and return-target
faults. Test actual 386 POPAD quirks against legacy evidence. Far transfers and
privilege-changing returns remain separate from near calls.

**Arithmetic:** carry/borrow inputs, sign boundaries, implicit high/low
products, divide-by-zero, quotient overflow, minimum signed dividend divided
by -1, and no partial result-register update on #DE.

**Bit/flag operations:** count 0/1/width/width+1 and masked counts, carry inputs,
signed memory bit indices, zero-source scans, and independently specified
defined/preserved masks. Do not turn modern undefined values into 386 rules.

**Strings/devices:** DF both directions, count zero, source overrides and ES
destination rules, stop-on-ZF behavior and faults after some elements succeed.
Keep completed progress; never replay irreversible device output on restart.

**System:** segment cache loads and SS/STI shadows, privilege/selector checks,
real/protected/V86 differences, page accessed/dirty state, task/gate transitions,
faults during delivery and reset hidden-CS state. Keep translation, physical
bus/A20 mapping and devices separate.

**Encoding audit:** register/memory and immediate variants; missing MOV C6/C7
and moffs; memory SETcc; segment PUSH/POP; 16:16/16:32 pointers; all group
extensions and aliases; prefix legality and selected-model length behavior.
Monitor rejection is not yet architectural exception delivery.

## First six reviewable implementation slices

1. Independent case/provenance/mask infrastructure. In parallel implement
   N0/N1 modern-PC register ALU stubs for the already supported subset.
2. Named multi-register effects and selected ordered memory/commit rules,
   compared in both backends before adding stack instructions.
3. Basic PUSH/POP register/immediate, then memory and SP/ESP alias cases.
4. Near CALL/RET/RET-immediate and indirect JMP, plus a checked debugger
   procedure/stack example.
5. Rotates, double shifts and bit operations in small independently checked groups.
6. Implicit MUL/IMUL and DIV/IDIV with #DE, followed by one-element strings.
   Do not bundle protected-mode changes into these arithmetic slices.

## Verification and machine milestones

Each admitted form needs independent expected results, backend agreement,
untouched-state and appropriate fault checks, readable disassembly and program
examples. Retain case IDs, masks and first-field failures. Separate unsupported
and revoked cases from passes. Timing remains separate from correctness.

The modern oracle verifies a defined overlap or the explicit 64-bit profile,
not 386 segmentation, decimal adjusts, privileged state, undefined outputs or
cycle timing. Use the legacy hardware lane and reviewed architectural rules
for those. See the separate x86-64 plan for the exact overlap contract.

Machine goals: reset plus a tiny real-mode diagnostic, applicable external ROM
tests, a DOS boot path, plain Windows 3.1x, WfW 3.11, then optional Windows 95
and NT 3.51 milestones. Each requires BIOS, interrupts, storage, display and
input support in addition to CPU coverage. See the edition-specific OS table;
no boot success is claimed. TTGO VGA/PS2/SD/WLAN and the existing WLAN-to-SD
installation workflow remain later board integration work.

Planning/tests/history stay on dev. Main receives reviewed runtime exports.
This plan changes no runtime behavior.

## References

[Original 386 instruction index](https://read.seas.harvard.edu/~kohler/class/aosref/i386/c17.htm)
and [encoding format](https://read.seas.harvard.edu/~kohler/class/aosref/i386/s17_02.htm);
[reference register](../REFERENCES_AND_CREDITS.md). This is an original planning
inventory, not imported manual prose or vectors. Pin detailed references and
hardware cases before implementing each instruction's exception/commit rules.


## FreeDOS-driven next slices

CLD/STD descriptions, DF state and decoder forms are implemented in the current
follow-up, with independent Python, generated-code and native-ABI tests. Windows/Linux
Rust and native DF checks passed (CI40476098533). Next complete P0 independent stack-size/address and
commit rules before admitting PUSH/POP. Segment PUSH/POP also needs visible
selector state; existing hidden segment caches alone do not supply selectors.
STI stays in P8 with interrupt shadows and delivery; do not emulate it as a bit
write. The first-blocker corpus is prioritization evidence, not whole-program
coverage. Keep CS-write model faults separate from missing instructions.

## Initial stack implementation (0.22)

Snapshot named GPR inputs and source-ordered named writes now support precise
register/immediate PUSH,register POP and all segment PUSH forms. Operand and
SS stack sizes are independent;SP/ESP alias ordering is tested in both backends.
Explicit visible selectors are stored alongside caches. CI40487360431 passed
223 tests/56 backend cases on Windows/Linux. This is partial P0/P2 work: memory
operands require independent access contexts (POP memory uses updated ESP),and
segment POP/MOV requires transactional selector/cache state and SS shadows.
The single-store RAM model is not yet a general ordered MMIO/system model.

## Real-mode segment follow-up (0.23)

PIL now snapshots selectors and stages a real-mode selector/cache update at
retirement. MOV8C/8E and POP ES/SS/DS/FS/GS are implemented;SS-shadow state is
recorded across successful instruction/DOS-bridge retirements. Both backends
use runtime segment contexts and full traces include caches/selectors/shadow.
Windows/Linux CI40498283191 passed231 tests/62 cases. Descriptor-table loads,
interrupt delivery and MOVr32,Sreg legacy upper-half policy remain unimplemented.
Memory PUSH/POP still requires independent per-access contexts. Measured FreeDOS
blockers now favor REP,moffs,CALL and PUSHF;no whole-program coverage claimed.

## Additional MOV forms (0.24)

A0..A3 moffs and C6/C7 /0 register/memory immediate MOV now use existing PIL
operations. Address/operand sizes remain independent. CI40500511068 passed235 tests/
67 backend cases on Windows/Linux. P1 encoding audit remains partial;segment
MOV limitations are unchanged. Next establish per-access addressing for stack
memory/near calls and one-element strings,before restartable REP.

## Near calls and restartable movement (0.25)

Shared per-access segment memory now supports near relative/indirect CALL,
RET with byte cleanup,and MOVS/STOS/LODS with F3 REP. Each REP element is a
bounded execution step; earlier elements survive faults and retry resumes at
the first unfinished element. Address size controls count/pointers, operand
size controls data and SS size controls stack addressing. CI40511872914 passed243
tests and84 full-state backend cases on Windows/Linux. These are partial P2/P5
results,not full family completion. Next use measured first blockers to choose
among flags,remaining strings,indirect flow and DOS service work; preserve
independent fault/retirement tests. Interrupt delivery and far transfers remain.

## Real-mode flags and zero-count branches (0.26)

CLI/STI,PUSHF/POPF16/32 and JCXZ/JECXZ now use shared PIL effects. Distinct STI
shadow is traced with segment state; it survives partial REP and faults until
full retirement. Independent tests cover flags masks,stack sizes/faults and
branch count/target-size separation. CI40527735310:252 tests,96 full-state cases on
Windows/Linux. P2/P8 remain partial:protected/V86 privilege checks,interrupt
and exception delivery,stack shutdown and full mode/prefix/hardware audit remain.
Choose next work from the updated actual-image first blockers and loader/model
limits rather than declaring whole instruction families complete.

## LEAVE and DOS memory (0.27)

LEAVE16/32 covers independent operand/SS sizes and named SP write dispatch after
SS loads. ENTER remains in P3. A bounded DOS shim now supports48/49/4A via one
visible MCB chain with error results,first-fit and precise commit checks and original PSP-size retention;this is
host DOS behavior,not a CPU instruction implementation. CI40536350302:265 tests and
103 backend cases Windows/Linux. Initial/current allocation is distinct from
physical RAM; fixed instruction windows and real-mode CS-write limitations
remain. Use updated first-blocker evidence for further ISA/service/model work.
