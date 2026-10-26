# Modern CPU verification and an x86-64 PIL subset

Updated 27 October 2026. All milestones below are planned. Today's compiled
runner emits Rust for fixed 386 images. PC_NATIVE is a narrow ADD/SUB/JNZ
timing baseline, not a general hardware oracle or an x86-64 guest decoder.

## Objective and lessons to transfer

Reuse width-aware pure operations, explicit flag behavior, register views,
validated effects, memory/fault lessons, cache invalidation and first-divergence
traces. Share operation semantics where specifications agree; keep decoding,
register-write policy, mode checks and genuinely different semantics in explicit
profiles. Avoid a second hand-written CPU implementation as the generated backend.

Use the user's x86-64 Windows/WSL PC for actual instruction results. ARM64
Termux can run both backends and portable provenance-recorded fixtures, but
cannot directly execute x86 opcodes for fresh hardware observations.

## Parallel validation lanes

| Lane | Execution | What it establishes |
|---|---|---|
| Common 8/16/32 semantics | Selected modern long-mode instructions and 386 PIL operations with explicit operand mapping | Defined overlapping scalar/flag behavior |
| True x86-64 subset | Actual 64-bit instructions versus a new long-mode profile in both backends | Admitted 64-bit forms on the recorded host |
| Legacy/system behavior | Pinned 386-family hardware cases and machine tests | 386 behavior absent/different in long mode |

A 16-bit monitor image does not execute unchanged in long mode. Record separate
encodings, modes and mappings. Do not apply overlap normalization to original
external 386 vectors, whose input state and addresses must remain intact.

## Milestones

| ID | Deliverable | Acceptance gate |
|---|---|---|
| N0 | Development-only ABI-safe fixed assembly stubs and structured input/output | MOV/ADD capture registers, defined flags and untouched state on Windows/Linux x86-64; unavailable on ARM explicitly |
| N1 | Existing 8/16/32 register ALU, shifts and conversions | CPU/interpreter/generated comparisons, independent masks, boundary and carry cases, reproducible failures |
| N2 | Explicit 64-bit state, register writes and REX decoding | Sixteen GPRs, views/write policies, RIP/addresses; 386 regressions retained |
| N3 | Wide PIL values and initial 64-bit arithmetic | Widths through 128, total operations and correct 64-bit results/flags |
| N4 | Bounded memory, LEA and near control flow | Address-size/RIP-relative cases, guards, signed displacements and no excess writes |
| N5 | Managed stack, PUSH/POP/CALL/RET and selected strings | Host restoration, balanced control flow, alias/DF/stack checks and isolated faults |
| N6 | Versioned cross-host fixtures and shrinking | Seeds, masks and host provenance retained; Intel/AMD differences investigated; unsupported features explicit |

Start N0/N1 now, alongside 386 completion. N2/N3 need not wait for every
legacy/system instruction. Full-register hardware tests involving RSP/RBP
require the managed-context work, even when the core decoder already supports them.

## Initial admitted 64-bit subset

MOV, MOVZX/MOVSX/MOVSXD, LEA, ADD/ADC/SUB/SBB/CMP, AND/OR/XOR/TEST,
NOT/NEG/INC/DEC, SHL/SHR/SAR, selected rotates, truncated IMUL,
CBW/CWDE/CDQE, CWD/CDQ/CQO, NOP, SETcc and bounded near Jcc/JMP.
Admit each form separately: memory and stack/control tests are N4/N5 work.

Initially exclude guest syscall/ABI emulation, privileged operations,
SIMD/x87, far transfers and arbitrary executable loading. CPUID used by the
host harness discovers capabilities; it does not add CPUID to the 386 guest.
Wine bindings/general binary translation remain later API work.

## Mode-specific work that cannot be a simple u32-to-u64 substitution

1. Long-mode 32-bit GPR writes clear the upper 32 bits; byte/word writes merge.
   Test nonzero upper halves and 32-bit self-moves.
2. A sign-extended result written to a 32-bit destination is still a 32-bit
   write. For source byte 80, compare 00000000FFFFFF80 for a 32-bit destination
   with FFFFFFFFFFFFFF80 for a 64-bit destination.
3. Implement REX, extended registers and byte names explicitly. AH/CH/DH/BH
   are unavailable with REX. Keep legacy 40..4F INC/DEC separate from long-mode
   prefix decoding.
4. Keep operand/address/stack sizes independent. Add long-mode default sizes,
   64/32-bit address rules, RIP-related forms, sign-extended immediates and
   displacements. Legacy 16-bit addressing is not a long-mode address alternative.
5. Use a feature-gated host-profile value representation through 128 bits.
   ADD64 needs a 65-bit carry intermediate and full MUL/IMUL64 products need
   128 bits: existing bv1..64 templates cannot simply set W=64 everywhere.
   Extend literals, validation, interpreter values and emitted types together.
   Audit mask128, total shifts, casts, signed division and minimum-value/-1.
6. Parameterize today's eight u32 registers, u32 EIP/addresses, RAM accesses
   capped at 32 bits and 32-bit trace fields. Add sixteen u64 GPRs, qword memory,
   64-bit RIP/fault addresses and versioned traces without truncation. Check
   arithmetic before host-index conversion. Keep optional wide-value/native
   tooling out of the TTGO 386 build by default.
7. Flags and their definedness are case-dependent. Long-mode segment behavior
   is not the 386 segment model. Start with owned flat buffers; FS/GS and
   canonical-address faults need dedicated later state.
8. Explicitly parameterize 64-bit shift count masks and stack/call rules;
   do not infer guest rules from host pointer width.

BCD adjusts, BOUND and PUSHA/POPA illustrate instructions unavailable in
64-bit mode. Legacy segmentation, privilege, undefined outputs, model quirks
and cycles need separate evidence. An OS-caught native fault is not itself
proof of a 386 error code or exception frame.

## Native execution harness

- Start with curated out-of-line stubs compiled by the existing Rust toolchain.
  Record exact bytes and register roles; inspect emitted stubs so assembler or
  compiler lowering cannot silently change the tested operation. Arbitrary
  dynamic byte execution is not required initially.
- Use explicit input/output buffers. Save/restore nonvolatile registers and
  host stack. Implement Windows x64 shadow-space/alignment/unwind requirements
  and Linux SysV rules separately. Never load guest-arbitrary values into live
  host RSP. Use a managed scratch stack/context for later stack-pointer tests.
- Load inputs, seed only admitted writable arithmetic/DF bits, execute, capture
  results and flags before flag-changing cleanup, restore host state, return
  to Rust. Do not feed random privileged/reserved RFLAGS bits to POPFQ.
  Restore ABI-required DF before calling Rust/library code.
- Use owned buffers and sentinels; compare outputs, implicit writes and
  untouched regions, not only EAX. Exercise aliasing and both carry inputs.
- First exclude faults; later isolate them in a child process and retain the
  available OS exception context. Exit status is not a full CPU fault record.
  Branches stay inside known stubs and native CALL/RET remain balanced under
  enabled host control protections.
- Compare each backend independently against hardware. The DUT's undefined mask
  must never decide what to ignore. Use independent case-specific defined and
  preserved masks; keep undefined hardware observations for diagnosis.
- Record CPU vendor/family/model/stepping, features, OS/ABI, toolchain, mode,
  bytes, source revision, seed, masks, inputs and result hashes. Missing features
  or a non-x86 host are explicit unsupported outcomes, not passes.

## Case format, cross-mode comparison and triage

Versioned records contain case/semantic IDs, profile and widths, exact bytes,
initial registers/flags/memory, expected write set, independent masks,
hardware result/fault context, both backend results and host provenance.
Keep normalized overlap cases distinct from original-address 386 vectors and
true 64-bit architectural cases.

Every cross-mode address/operand mapping must be recorded. Compare only fields
justified by it; do not compare full native RAX with a 386 register while
ignoring different write policies. True 64-bit cases compare full 64-bit
outputs and upper halves.

Shrink mismatches by values, operands, prefixes and layout while preserving the
failure. Retain minimized original/native/backend states and add deterministic
regressions. Investigate defined Intel/AMD differences against the profiles;
do not make undefined observations universal rules.

## First implementation PR

N0 plus ADD/SUB/ADC/SBB/CMP 8/16/32 on safe fixed registers, boundary values,
carry inputs, complete defined arithmetic flags and unchanged-register checks.
Run hardware/interpreter/generated comparisons with exact IDs on Windows and
Linux x86-64. Keep it optional development tooling. Validate the harness before
expanding the operation list.

Correctness results make no performance claim. Separate later benchmarks can
use validated cases and explicit setup/compilation/output exclusions. Existing
PC_NATIVE timing retains its narrower name and scope.

## References and provenance

Consulted 27 October 2026:
- [Intel SDM portal](https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html), revision093 advertised; pin exact volumes/revisions for implementation.
- [Intel Volume1 253665-060US,September2016](https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-1-manual.pdf), register/operand environment; this older linked PDF is not claimed to be revision093.
- [AMD APM Volume3 listing,24594 revision3.38](https://docs.amd.com/v/u/en-US/24594_3.38_APM_Vol3_PUB), published21 July2026; listing inspected, not a full PDF audit.
- [Microsoft x64 instruction overview](https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/x64-instructions).
- [Windows x64 calling convention](https://learn.microsoft.com/en-us/cpp/build/x64-calling-convention?view=msvc-170).
- [x86-64 psABI project](https://gitlab.com/x86-psABIs/x86-64-ABI), located; pin/read the actual specification for N0.
- [Original 386 index](https://read.seas.harvard.edu/~kohler/class/aosref/i386/c17.htm) and [external-suite plan](EXTERNAL_TEST_SUITES.md).

Original project TODO/design work; no external implementation, manual prose or
vectors imported, and no new hardware-oracle passes claimed.
