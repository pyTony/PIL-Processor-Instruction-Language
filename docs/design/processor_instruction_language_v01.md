# Processor Instruction Language (PIL) v0.1
Design proposal — 26 October 2026

This document preserves the proposed architecture and future language. It is
not the executable grammar or a current implementation-status report. See
[the implemented PIL reference](../LANGUAGE.md) for supported syntax and APIs.
Statements about the first proof/toolchain below describe the original milestone.

## Purpose and first target

Describe CPU instructions once, separating pure bit-vector computation from ordered architectural effects. First run the tokenized description in a Rust reference interpreter. Next generate Rust source from the same tokens. Add embedded optimizations and native-code backends later.

First architecture: Intel 80386-compatible single CPU in an AT-compatible PC. First system milestone: DOS, followed by Windows 3.1x standard mode. Next milestone: 386 enhanced mode and Windows for Workgroups 3.11. The exact edition meant by “Windows 3.12” remains unconfirmed; it is not used as a compatibility claim.

ESP32-S3 with PSRAM is the preferred embedded evaluation target. This is a design, not an implemented emulator or measured performance result. Running DOS/Windows inside an emulated PC is a separate integration from the earlier Wine goal.

## 1. Representation layers

1. ISA source: readable PIL declarations, encoding rules, source references and examples.
2. Validated semantic IR: typed SSA values plus an ordered effect graph, explicit control flow and restart points.
3. First execution target: a Rust interpreter executing validated typed tokens.
4. Second execution target: Rust source emitted from those exact same tokens, compiled and compared with the interpreter.
5. Optional desktop JIT: lower the same IR to an existing ARM64-capable backend.
6. Machine platform: devices, physical memory, firmware and event scheduling, outside arithmetic definitions.

Do not parse PIL on the MCU. Generate tables and specialized handlers on the development machine. Keep a simple interpreter as the oracle for optimized execution.

## 2. Values and widths

| Type | Meaning |
|---|---|
| bool | Truth value, distinct from a numeric bit vector |
| bv<N> | Exactly N bits; arithmetic wraps modulo 2^N |
| tuple<T...> | Multiple results |
| enum<E> | Finite architectural states |
| record<R> | Named fields, statically laid out |
| regref<N> | Typed reference to an architectural register view |
| memref<N> | Guest memory operand of N bits, including segment and offset |
| operand<N> | Register, immediate or memory operand selected by decoding |
| port16 | Guest I/O port number, not a host pointer |
| effect | Ordered state/bus transition token |
| result<T, Fault> | Success or an explicitly classified guest fault |

Widths are positive compile-time integers. The initial implementation supports bv<1..64>, including bv<33> for 32-bit carry and bv<64> for widened multiplication. The schema permits future wider types; an unsupported backend must reject them at build time. Host lowering can use limbs instead of assuming native integers exist.

Signedness belongs to operations: ult versus slt, udiv versus sdiv, zext versus sext. There are no implicit width changes or host-language overflow rules. Memory operations require byte-multiple widths. Layout of records and memory endianness are explicit.

Pure primitives:
- add, sub, mul, and, or, xor, not, eq, ult, slt, select.
- extract(value, low, width), concat, zext, sext, trunc.
- shl_total, lshr_total, ashr_total: specified for every count, including counts >= width.
- parity_even8, bit, popcount; division primitives return success or a pure arithmetic error.

ISA-specific count masking and exception conversion are separate from these primitives. Ordinary IR unsigned subtraction is not an implicit x86 flag update.

## 3. CPU state and register views

Example declarations:

~~~text
architecture i386 {
  endian little;
  register EAX : bv<32>;
  view AX : bv<16> = EAX[15:0]  write merge;
  view AL : bv<8>  = EAX[7:0]   write merge;
  view AH : bv<8>  = EAX[15:8]  write merge;

  register EFLAGS : bv<32>;
  flag CF = EFLAGS[0];
  flag PF = EFLAGS[2];
  flag AF = EFLAGS[4];
  flag ZF = EFLAGS[6];
  flag SF = EFLAGS[7];
  flag OF = EFLAGS[11];
}
~~~

A merged view write preserves all parent bits outside its slice. A future x64 profile can declare EAX as a view of RAX with a zero-upper write policy. That rule must not leak into the 386 profile.

Additional 386 state: the other seven general registers, EIP, six segment selectors and hidden descriptor caches, GDTR/IDTR, LDTR/TR with cached state, CR0/CR2/CR3, debug registers, halt/shutdown state, interrupt inhibition and pending events. Unsupported test-register behaviour must be tracked as a compatibility limitation.

Visible segment selectors alone are insufficient for snapshots. Define reset values and reserved-bit read/write policies per CPU model. CPL and execution mode should be derived or updated through checked transitions, not freely assigned by individual arithmetic instructions.

Decode context separates:
- operand width: 8, 16 or 32;
- effective-address width: 16 or 32;
- stack-address width: 16 or 32;
- code-segment attributes and real/protected/v8086 mode.

The CS default and 66h/67h prefixes select operand/address attributes. SS.B selects stack addressing independently [S2]. These values cannot be collapsed into one “CPU bitness” field.

## 4. Instruction declarations and encodings

Minimum language structure:

~~~text
instruction NAME<width_parameters>(operands) {
  availability { model; modes; }
  encoding { opcode; modrm; immediates; prefixes; }
  operation { pure expressions and values; }
  effects { ordered reads, writes, faults and commits; }
  flags { assignments; preserve; undef; }
  flow { fallthrough | branch | helper; }
  provenance { source_id; section; verification_status; }
}
~~~

The operation/effects split is semantic, not a requirement to put all computations before every load. Effectful loads return values used by subsequent pure SSA nodes.

Example encoding row, not the whole ADD family:

~~~text
encoding ADD_rm_r<W in {16,32}> {
  bytes = [0x01];
  modrm = required;
  dst = rm<W>;
  src = reg<W>;
  width W = context.operand_width;
  lock = allowed_only_if_memory(dst);
}
~~~

Each encoding row points to a shared semantic family. Decode tables must reject later-CPU instructions in a strict 386 profile. Prefix legality and undefined encodings are model-specific. Include segment overrides, default-segment selection, ModR/M, SIB, displacement extension and immediate extension. Fetch only instruction bytes actually needed; speculative decoding across a guest page must not introduce an architectural fetch fault.

## 5. Arithmetic and flags example

Complete arithmetic kernel for ADD; memory access and retirement are supplied by its operand wrapper.

~~~text
semantic add_value<W in {8,16,32}>(a:bv<W>, b:bv<W>) {
  operation {
    wide : bv<W+1> = add(zext<W+1>(a), zext<W+1>(b));
    r    : bv<W>   = trunc<W>(wide);
  }
  flags {
    CF = bit(wide, W);
    OF = bit(and(not(xor(a,b)), xor(a,r)), W-1);
    AF = bit(xor(xor(a,b),r), 4);
    SF = bit(r, W-1);
    ZF = eq(r, 0:W);
    PF = parity_even8(trunc<8>(r));
    preserve all_other_flags;
  }
  yield r;
}
~~~

Flag rules come from the instruction semantics and status-flag specification [S3, S4]. INC uses the same arithmetic result, OF/AF/SF/ZF/PF assignments, but preserves CF. CMP uses subtraction flags and performs no destination write. AND/OR/XOR clear CF and OF, derive SF/ZF/PF and mark AF undefined [S4].

An undefined flag is an architectural nondeterministic result, not a compiler “poison” value and not an assertion that the old flag is preserved. Provide two modes:
- architecture mode: comparisons mask undefined outputs;
- model compatibility mode: a documented deterministic policy matching a chosen CPU where evidence exists.

The implementation always stores a concrete bit; optional definedness masks support validation. A deterministic placeholder for an undefined bit does not justify optimizing arbitrary subsequent behaviour away.

Lazy flags are an optimization of these assignments. Track ownership per flag so INC can preserve an older CF while replacing the others. Materialize required bits for conditional operations, PUSHF, LAHF, ADC/SBB, exceptions, tracing and external state export. Do not use host flags as the guest architectural state.

## 6. Effects, faults and retirement

Effect primitives include:

~~~text
guest_load<N>(segment, offset, access_context) -> value | fault
guest_store<N>(segment, offset, value, access_context) -> success | fault
reg_read(view) -> value
stage_reg_write(view, value)
stage_flag_write(flag, value)
port_read<N>(port) -> value | fault
port_write<N>(port, value) -> success | fault
invoke_checked(helper, args...) -> result
retire(next_eip)
checkpoint(restart_eip, progress_state)
raise_guest_fault(vector, error_code?, fault_address?)
~~~

Every faulting effect has an explicit successor and source-instruction mapping. Preserve fault EIP versus trap return EIP. Fault metadata includes CR2 when applicable, error code and guest-visible state required for delivery.

For a simple register ADD, compute and stage destination/flags, then retire. For a memory ADD, read the memory operand, compute, complete the checked write, then publish staged destination/flag changes. RAM writes crossing a page boundary require a defined access plan before publishing architectural updates.

This is NOT universal transactional rollback. Page-table accessed/dirty updates, device reads, complex instructions and other architecturally observable events need explicit policies. A memory helper's plan must encode check ordering, split accesses and fault behaviour for the chosen model. Never probe an MMIO register by actually reading it twice. Prevalidation of a write must not be moved ahead of a device read unless the architectural access ordering permits that.

LOCK and implicit locking are effect properties. Keep their boundary indivisible relative to guest-visible DMA/device events; a single guest CPU does not make device interleaving irrelevant. Interrupt delivery is allowed only at eligible boundaries. STI and SS-load inhibition require dedicated rules.

Complex helpers are allowed, but each declares reads, writes, possible faults, restart points and whether it ends a block. Initial candidates: far transfers, IRET, descriptor loads, task switching and interrupt delivery.

## 7. Restartable string instructions

REP MOVS is a loop with one architectural checkpoint per completed element, not a single giant memory transaction.

~~~text
repeat while counter(context.address_width) != 0 {
  value = guest_load<W>(source_segment, SI_or_ESI);
  guest_store<W>(ES, DI_or_EDI, value);
  checkpoint_same_instruction {
    SI_or_ESI += direction_step(W);
    DI_or_EDI += direction_step(W);
    CX_or_ECX -= 1;
  }
  poll_eligible_events();
}
retire(next_eip);
~~~

This is a structural sketch: helpers supply exact faults, wrapping, segment checks and event rules. Zero repetitions perform no data access. A fault in a later iteration retains completed progress. CMPS/SCAS additionally update flags and implement REPE/REPNE termination. INS/OUTS need separate definitions because I/O effects cannot be undone.

## 8. Windows compatibility profiles

| Profile | Scope | Acceptance gate |
|---|---|---|
| i386-real | Real mode, 16/32-size prefixes, integer instructions, A20, interrupts and I/O | Boot DOS and pass targeted CPU tests |
| i386-win-standard | 16-bit protected execution, segment checks, descriptor tables, gates, privilege transitions, 286-compatible facilities | Windows 3.1x standard mode; run chosen Win16 apps |
| i386-win-enhanced | Add complete required 32-bit system behaviour, paging, v8086 transitions, TSS/I/O permissions and VMM-sensitive exceptions | Enhanced mode plus a DOS session |
| i386-wfw311 | Enhanced profile and verified machine/device configuration | WfW 3.11 without networking first |
| optional-387 | Separately described x87 stack, formats, rounding and exception state | Dedicated numerical tests |

Microsoft's archived WfW history distinguishes the 286-capable WfW 3.1 standard configuration from the 386-required WfW 3.11 configuration [S7]. Do not assume every product called “3.11” has the same execution requirements.

For enhanced-mode correctness, paging includes two-level 4 KiB translation, CR3 changes, permissions, accessed/dirty state and page-fault reporting [S5]. V8086 is not just real-mode address calculation: interrupts leave through protected-mode machinery and return through defined state transitions [S6].

Implement a software translation cache whose invalidation follows the 386 model; do not accidentally require later INVLPG instructions or later CR0.WP semantics. The virtual machine should expose a coherent CPU identity and unsupported features. Test-register and debugging omissions remain explicit until validated.

No x87 may be advertised initially, but CR0.EM/TS, WAIT and coprocessor-unavailable behaviour still need correct handling. Applications requiring a coprocessor remain out of scope until emulation is added.

## 9. PC platform boundary

CPU correctness alone does not boot Windows. Supply an AT-compatible platform with firmware, PIC, PIT, CMOS/RTC, keyboard controller and A20/reset controls, VGA, storage and a mouse path supported by the guest. Start with standard VGA, keyboard, a serial mouse if convenient and one disk controller. Sound and networking are later milestones.

Device callbacks receive guest physical addresses or port numbers. Never expose raw host pointers in PIL. Machine state serialization includes device and event state. Disk image mounting must preserve explicit geometry/controller assumptions.

Selected Tiny386 devices are now the preferred platform starting point, behind a separate C ABI. See [the platform adapter plan](../platform/tiny386-adapter.md) for the proposed boundary, CPU coupling and checked licence notes. No upstream sources have been imported yet.

## 10. ESP32 execution strategy

ESP32-S3 uses Xtensa LX7 rather than ARM [S8]. The agreed implementation order is a Rust token interpreter, then generated Rust handlers. Prove desktop correctness first; establish the specific Xtensa Rust toolchain and ESP-IDF or no_std integration separately. A C backend remains optional rather than the first target.

Use:
- internal RAM for hot CPU state, small TLB, device queues and dispatcher metadata;
- PSRAM for guest RAM, video backing and bounded decoded blocks;
- flash for generated handlers, firmware and constant tables;
- SD storage for disk images, not per-instruction memory swapping.

Illustrative budget for an 8 MiB PSRAM board, to be measured:
| Allocation | Planning budget |
|---|---:|
| Guest RAM | 4 MiB |
| VGA backing | 256 KiB |
| Decoded instruction cache | 128–256 KiB |
| Disk/block buffers | 128–256 KiB |
| Remaining PSRAM | Runtime, display, optional network and headroom |

These are allocations, not a guarantee that any display/Wi-Fi combination fits. Framebuffer needs depend on output hardware. Bound code expansion and evict blocks instead of storing IR for all guest code.

Original ESP32 memory/executable-region restrictions differ from ESP32-S3 [S9, S10]. Do not assume writable PSRAM is a ready-made JIT code buffer. Start without native JIT; optimize common decoded handlers and lazy flags after correctness measurements. More guest instruction support does not itself imply more speed.

Block keys must include relevant code-segment/decode state and mapping generations. Physical code writes, DMA, bank changes and address aliases must invalidate affected decoded blocks. Mapping changes require revalidation. Keep guest memory and decoding caches coherent.

## 11. Suitable source data

| Source | Use | Limits |
|---|---|---|
| Intel 80386 Programmer's Reference Manual [S1] | Primary semantics, encodings, flags, faults and system modes | Human-readable; requires curated conversion |
| SingleStepTests/80386, including 80386.csv [S11] | Machine-readable instruction metadata and hardware-generated comparison tests | 386EX source CPU; currently real-mode coverage, not complete protected/v8086 validation |
| Tiny386 instruction definitions and PC implementation [S12, S13] | Compact implementation reference and practical ESP32 comparison | Known missing CPU facilities; not an architectural oracle |

The SingleStepTests repository documents 80386.csv, including undefined-flag metadata, and MOO test files with initial/final state. It also documents pending protected/v8086 suites and a revocation list [S11]. Import those as external inputs with pinned revisions and checksums. Check licensing and attribution before redistributing any data; do not assume a public manual or repository permits copying all its text into a new specification.

Tiny386 already demonstrates 386-class PC emulation on ESP32-S3, so “first 386 on ESP32” is not a valid project goal. Its README identifies missing debugging, hardware tasking and permission checks [S12]. The useful differentiator here is one reusable, testable semantic description and stronger verified compatibility.

An opcode database cannot automatically supply complete ordered execution semantics. Curate each family against Intel's descriptions and independent tests.

Recommended normalized import record:

~~~text
instruction_id, encoding_id, opcode_map, opcode_bytes,
modrm_constraint, operand_schema, width_rule,
minimum_cpu, allowed_modes, prefix_rules,
flags_read, flags_defined, flags_undefined, flags_preserved,
semantic_family, fault_rules, source_revision,
manual_section, validation_status
~~~

All missing data stays unknown and fails a completeness check; it must not silently become “no faults” or “preserve every flag.”

## 12. Validation and build outputs

Generator checks:
- widths, explicit extensions, valid bit slices and compatible branch results;
- exhaustive flag classification where specified;
- encoding collisions and CPU-feature gating;
- effect-token ordering, complete fault exits and retirement on successful paths;
- no invented purity for device accesses or system helpers.

Tests:
1. Exhaust all 8-bit ADD/SUB operands; include carry input for ADC/SBB.
2. Directed 16/32-bit overflow, carry, borrow, AF, parity and partial-register cases.
3. All operand/address-size combinations and independent stack widths.
4. Hardware-derived real-mode instruction tests, respecting revoked cases and undefined flags.
5. Custom protected-mode tests for segment limits, privilege faults, page crossings and exception frames.
6. REP fault after N successful iterations; verify exactly N committed elements and correct restart state.
7. Self-modifying code, CR3 reloads, aliases and interrupt inhibition.
8. DOS boot, Windows standard mode, then enhanced mode with interactive DOS session.

Run interpreter versus optimized handler comparison, but also compare independent hardware/reference results: two backends generated from the same wrong definition can agree.

Suggested future source layout:
- pil-spec/: grammar, type rules and effect contracts.
- isa/i386/: register/mode descriptions, encodings and semantic families.
- pil-gen/: Rust parser, checker and generators.
- runtime/: portable memory/device/exception interfaces.
- platform/esp32s3/: board integration and memory configuration.
- tests/: imported adapters, directed tests and system test recipes.

Library interface sketch: opaque CPU handle; create/reset/run with an execution budget; inject events; read/write architectural state; memory and I/O callbacks; trace/fault exits. Batch execution across the ABI. Keep CPU execution independent of display and disk ownership.

## 13. Implementation milestones and estimates

Estimates are engineering judgement, not source-derived measurements. One experienced full-time developer, reusing platform components:

1. 2–4 weeks: concrete grammar, typed AST, validator, register views and arithmetic families; generated host interpreter.
2. 4–8 additional weeks: broad integer decode, strings and hardware-test adapter; DOS-oriented platform integration.
3. 1–3 additional months: system-instruction semantics, protected-mode faults and Windows standard-mode debugging.
4. 2–6 additional months: paging/v8086 correctness and enhanced-mode debugging.
5. 1–3 months, partly overlapping: ESP32-S3 port, resource measurement, display/input and performance work.

Plan roughly 6–12+ months for a useful tested design and selected Windows workloads with reuse. A full PC platform written from scratch or a requirement to outperform existing emulators can extend this substantially. These milestones are gates, not boot-date promises.

First proof is ADD at 8/16/32 bits, with flag expressions supplied by the description, plus an INC-style CF-preservation variant. The accompanying pil_proof source provides a tokenizer/validator, Rust interpreter, Rust source emitter and tests. It has not been compiled in this environment because no Rust toolchain is installed. Next expand to ADC/SUB/SBB/CMP/DEC/logic and then memory-fault tests, before attempting Windows. Tokenization here produces in-memory typed enums; a persistent binary token format is not yet implemented.

## References

[S1] Intel, 80386 Programmer's Reference Manual (1986), hosted copy:
https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/toc.htm

[S2] Intel §17.1, operand/address and stack attributes:
https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s17_01.htm

[S3] Intel ADD instruction:
https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/ADD.htm

[S4] Intel Appendix C, status flags:
https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/appc.htm

[S5] Intel §5.2, paging:
https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s05_02.htm

[S6] Intel §15.3, v8086 transitions:
https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s15_03.htm

[S7] Microsoft KB Q126746, archived WfW version history:
https://ftp.zx.net.nz/pub/archive/ftp.microsoft.com/MISC/KB/en-us/126/746.HTM

[S8] Espressif, ESP32-S3:
https://www.espressif.com/products/socs/esp32-s3

[S9] Espressif, original ESP32 memory types:
https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-guides/memory-types.html

[S10] Espressif, ESP32-S3 external RAM:
https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/api-guides/external-ram.html

[S11] SingleStepTests/80386:
https://github.com/SingleStepTests/80386

[S12] Tiny386:
https://github.com/hchunhui/tiny386

[S13] Tiny386 instruction definitions:
https://github.com/hchunhui/tiny386/blob/master/i386ins.def

