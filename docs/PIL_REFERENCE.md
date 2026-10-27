# PIL reference — implemented language

[All PIL documentation](https://github.com/pyTony/pil_proof/blob/dev/docs/README.md)

This is the reference for the executable PIL grammar in the current repository,
checked against the parser, interpreter and Rust emitter in
[src/main.rs](https://github.com/pyTony/pil_proof/blob/dev/src/main.rs).
The [original v0.1 proposal](https://github.com/pyTony/pil_proof/blob/dev/docs/design/processor_instruction_language_v01.md)
is a separate design document. Its architecture declarations, encoding DSL,
effect graphs, helpers and restart checkpoints are not executable syntax.

## Source, values and bindings

There is one statement per line. Whitespace separates fields; `#` starts a
comment. Names are case-sensitive SSA values: define each name once, before
use. Statement names are lowercase; flag names are uppercase. Numeric constants
are unsigned decimal or lowercase `0x` hexadecimal and must fit their widths.

Values have widths from 1 through 64 bits. Predicates are `bv<1>` values, not a
separate Boolean type. Arithmetic wraps at the result width. Binary arithmetic
and bitwise inputs must have equal widths; there are no implicit conversions.
Instruction operand width is independently restricted to 8, 16 or 32 bits.

The `dst` binding always has the instruction operand width. The `src` binding
has one independently inferred width, 8/16/32, shared by **all source reads and
writes**, including conditional writes. If no statement reads or writes `src`,
its width defaults to the instruction operand width. Even unused or inactive
bindings must be valid at execution time.

Interpreter bindings are register views `(register, low, width)`: register index
0..7, the compiled width, and a slice fitting in 32 bits. A write merges its
slice and preserves the other bits. Generated APIs take `(register, low)` and
use widths fixed at compilation. Decoder-selected x86 aliases are a separate
layer; the language does not declare architectural registers by name.

Instruction templates are expanded before compilation: `$W` is operand width,
`$EXT` is width+1, `$SIGN` is width-1 and `$DOUBLE` is twice operand width.
They are textual substitutions, not grammar expressions. Conditions are composed
from `conditions.pil` and `setcc.pil`/`jcc.pil`; no general `include` directive exists.

## Pure values

| Statement | Type and behavior |
|---|---|
| `const out WIDTH NUMBER` | Define a fitting unsigned constant, width 1..64 |
| `read out dst WIDTH` | Snapshot of incoming destination; WIDTH must equal operand width |
| `read out src WIDTH` | Snapshot of incoming source; infers/validates source width |
| `read_flag out FLAG` | Incoming concrete CF/PF/AF/ZF/SF/DF/OF as `bv<1>` |
| `zext out input WIDTH` | Equal-width copy or zero extension |
| `sext out input WIDTH` | Equal-width copy or sign extension |
| `trunc out input WIDTH` | Equal-width copy or narrowing |
| `add/sub/mul/xor/and/or out left right` | Equal-width operands; result has that width |
| `not out input` | Bitwise complement at input width |
| `shl/lshr/ashr out input count` | Total shift; count can have any valid bit-vector width |
| `ult out left right` | Unsigned comparison of equal-width values; `bv<1>` result |
| `bit out input INDEX` | Extract one bit; unsigned index must be below input width |
| `zero out input` | `bv<1>` result of equality to zero |
| `parity out input` | Even parity of low byte; input must be at least 8 bits |
| `select out predicate yes no` | `bv<1>` predicate and equal-width arms |

A shift by a count at least as large as the input width returns zero for
`shl`/`lshr`, or repeated sign bits for `ashr`. Counts are not implicitly masked
or reduced modulo the host word width. x86 count rules belong in descriptions.

All reads of registers, flags and EIP see incoming state, even after staged
writes. `select` selects already-computed values; it does not suppress an
earlier load, fault or calculation. `write_if` only suppresses its register write.

## Register and flag effects

| Statement | Effect |
|---|---|
| `write dst input` | Stage destination-width register write |
| `write_if dst input predicate` | Stage write only when `bv<1>` predicate is one |
| `write src input` | Stage source write; input infers/validates source width |
| `write_if src input predicate` | Conditional source write; same static width rule |
| `flag FLAG input` | Stage a defined flag bit; input must be `bv<1>` |
| `flag_if FLAG value defined active` | Three `bv<1>` inputs; behavior below |
| `preserve_flag FLAG` | Preserve concrete bit and diagnostic marker |
| `preserve_flags all` | Classify all six arithmetic flags as preserved |
| `preserve_flags remaining` | Preserve arithmetic flags not yet classified |
| `undef_flag FLAG` | Stage concrete zero and set diagnostic undefined marker |
| `require_width WIDTH` | Compile-time operand-width restriction; numeric width or `16_or_32` |
| `retire` | Commit staged effects and the next EIP |

Each CF/PF/AF/ZF/SF/OF must be classified exactly once. Missing or duplicate
classifications fail compilation. Put `preserve_flags remaining` after explicit
classifications. DF (bit 10) may additionally be read, defined, preserved or
marked undefined explicitly; duplicate classifications are rejected. The `all`
and `remaining` masks still cover the six arithmetic flags only. Unmentioned DF
and other EFLAGS bits are automatically preserved. IF and privileged system
flags cannot currently be modified. CLD/STD define DF as zero/one using ordinary
PIL flag writes and clear only its diagnostic undefined marker.

| `active` | `defined` | `flag_if` behavior |
|---|---|---|
| 0 | Either | Preserve incoming flag and marker |
| 1 | 1 | Write `value`; clear its marker |
| 1 | 0 | Write zero; set its marker |

At most one write statement per binding is allowed, even with a false
predicate. Both bindings can target the same physical register. At retirement,
active overlapping writes must agree on their shared bits or execution returns
`InvalidBinding` before committing CPU or RAM changes. Disjoint slices such as
AL and AH and agreeing self-aliases are supported. This is two binding writes,
not arbitrary implicit-register effects.

Exactly one final `retire` is required. No statements may follow it; comments
and blank lines are allowed. Compiler statement diagnostics include one-based
source line numbers; final missing-retire/flag-coverage checks are program-wide.

## Undefined-flag policy

`Cpu.undefined_flags` is diagnostic state. Undefined outputs choose deterministic
zero and set a marker; defined writes clear the corresponding marker; preserved
flags retain both bit and marker. Comparisons with hardware exclude only the
architecturally undefined outputs, using independent masks.

`read_flag` reads the concrete bit even if marked undefined. The language does
not propagate symbolic uncertainty through computations or register values.
For example ADC/SBB and CMC consume the concrete carry; a defined output clears
its marker. Undefined flags are not Rust UB, compiler poison, guest exceptions
or reasons to discard subsequent instructions.

## Flow context

| Statement | Type and behavior |
|---|---|
| `read_ip out current WIDTH` | Incoming EIP truncated/extended as an unsigned bit vector, width 1..64 |
| `read_ip out next WIDTH` | Caller-supplied fall-through EIP at width 1..64 |
| `read_disp out WIDTH` | Signed displacement bit pattern at the 16/32-bit operand width |
| `branch_if target predicate` | Stage `bv<32>` target when `bv<1>` predicate is one |

`FlowContext` supplies `next_eip`, signed `displacement`, `displacement_bits`
and inclusive `code_limit`. A program reading displacement requires bits=8 or
the operand width and a value fitting that signed width. Without `read_disp`,
displacement fields are not validated or used. `branch_if` is restricted to
16/32-bit instruction operand widths and occurs at most once.

A taken branch checks its supplied 32-bit target against `code_limit`, returning
`GeneralProtection { eip: original_eip, error_code: 0 }` on failure. Truncation
and relative-address arithmetic are explicit in Jcc/JMP descriptions, not an
implicit behavior of `branch_if`. A false predicate skips target validation.
Ordinary retirement uses `next_eip`; it does not itself validate instruction
fetch or the fall-through limit. The monitor performs CS fetch checks separately.

## RAM and segmentation

| Statement | Type and behavior |
|---|---|
| `read_address out` | Caller-supplied address/offset as `bv<32>` |
| `load out address WIDTH` | 8/16/32-bit little-endian RAM read; address must be `bv<32>` |
| `store address input` | Stage one 8/16/32-bit RAM store, width inferred from input |

Any of these statements, including `read_address` alone, requires a RAM-capable
execution API. Multiple loads are allowed, but only one store and no loads
after that store. Checks occur in token order. A store checks permission before
being staged and is published at retirement before infallible CPU-state commits.
Later faults and overlap conflicts cancel the staged store and CPU effects.
Unaligned RAM accesses are supported.

Flat execution uses linear RAM addresses. Segmented execution resolves a
supplied `AddressContext` from incoming registers, then translates each load or
store with its actual width and direction. `read_address` returns the resolved
effective offset. Address-size wrapping, default DS/SS choice and segment
overrides are handled by the shared address module. Limits are inclusive expanded
byte limits; expand-down segments exclude the limit and use B for the upper bound.
Multi-byte spans do not wrap at the offset limit; base+offset wraps to 32 bits.

Segmentation failures report `GeneralProtection(0)` or `StackSegment(0)` at the
source EIP. Invalid supplied bindings/cache context are caller errors.
`MemoryAccess` reports EIP, address, byte count and direction for RAM bounds or
host-defined read-only protection. It is not x86 page-fault delivery: no paging,
CR2, descriptor loads or exception handlers are implemented. Device/MMIO side
effects, multiple stores, LOCK and REP checkpoints need a future effect contract;
this staging model is not universal transactional rollback.

## Execution APIs and emitted Rust

| Interpreter | Generated API | Context |
|---|---|---|
| `execute` | `execute_with_metadata` | Register/flag programs; rejects explicit flow or RAM tokens |
| `execute_flow` | `execute_with_flow` | Supplied flow context; no RAM tokens |
| `execute_memory` | `execute_with_memory` | Flow plus flat RAM and supplied address |
| `execute_segmented` | `execute_with_segments` | Flow plus RAM and decoded address/segment context |

The interpreter `execute` preserves incoming EIP. Generated
`execute_with_metadata` has no EIP parameter and uses a private zero-initialized
EIP. Generated compatibility `execute` also initializes/discards the undefined
mask; use metadata/stateful APIs to inspect flags across instructions.

The monitor has binary fetching/decoding, normalized segment caches, a decoded
instruction cache, mock serial I/O and a debugger. Those are separate from PIL
syntax. `--emit-rust` emits standalone Rust source, compiled offline with rustc;
it is not a JIT. Tests and independent oracles live on `dev`; runtime `main`
contains buildable source and embedded descriptions without development tests.

```sh
cargo run -- --emit-rust 8 setl > setl.rs
rustc --edition=2021 --crate-type=lib setl.rs
cargo run -- --emit-rust 32 and > and32.rs
# Development checkout:
python tools/verify_descriptions.py
python tools/verify_flow.py
python tools/verify_host_cpu.py
```

## Example: exchanging two incoming bindings

```text
read a dst $W
read b src $W
write dst b
write src a
preserve_flags all
retire
```

Instantiated for 8/16/32 bits, this is `xchg.pil`. The x86 decoder supplies
register aliases and legal encodings. Memory XCHG is not implemented. Other
instruction-family details and demos are in the debugger guide and feature
matrix; grammar support alone does not imply complete 386 instruction coverage.

## LAHF and SAHF descriptions (0.18 alpha)

`lahf` and `sahf` require 8-bit bindings. The decoder binds AH for opcodes
9F/9E, independently of operand/address-size prefixes. LAHF uses read_flag,
zero extension, shifts and OR to write `SF:ZF:0:AF:0:PF:1:CF` into AH, while
preserving all flags and markers. SAHF reads AH and defines CF/PF/AF/ZF/SF;
it clears their markers and preserves OF and all other flags/markers.
LAHF copies concrete input bits without propagating diagnostic taint to AH.
SAHF ignores AH bits1/3/5 and does not normalize unrelated supplied EFLAGS bits.
Neither description adds a new semantic token or backend-specific handler.

## Named general-register effects and initial stack forms

`read_gpr out INDEX WIDTH` reads an incoming GPR (0..7 in EAX..EDI order)
as bv16/bv32. `write_gpr INDEX value` stages a low16/full32 write. Reads always
see incoming state; named writes commit in source order, so a later overlapping
write overrides the earlier one. Mixing named writes with bound dst/src writes
is rejected. All memory checks and the single staged RAM store precede register
commit. This is a RAM-only precise-commit contract, not ordered MMIO/multi-store.

PUSH/POP register and PUSH imm16/32/sign-extended imm8 use these effects. Stack
address width comes from SS.big, separately from operand width (66). Address-size
and segment overrides do not redirect the implicit SS access. PUSH SP/ESP reads
the original register; POP SP/ESP increments the stack pointer before writing
the popped operand. A multi-byte access crossing the segment end faults rather
than wrapping each byte. Initial segment PUSH ES/CS/SS/DS/FS/GS reads explicit visible
selectors supplied by the loader, not inferred hidden-cache bases. Cache identity
includes those selectors and the SS attributes. Memory stack operands and
CALL/RET remain follow-up work. Real-mode segment POP and SS inhibition state
are implemented through the mutable segment-state API below; interrupt delivery
remains separate.

The `push`/`pop` example templates use a word stack and EAX/AX destination by
default with --emit-rust. The decoder substitutes the actual SS size/register.

Register aliases FF /6 and 8F /0 share the same PUSH/POP descriptions.

## Mutable real-mode segment state

`read_segment out INDEX` snapshots a visible selector (ES,CS,SS,DS,FS,GS =0..5)
as bv16. `write_segment INDEX value` stages one bv16 non-CS selector load,
rebuilds its ordinary real-mode cache and commits with GPR/memory effects.
Use execute_machine / emitted execute_with_machine with SegmentState; contextless
wrappers reject these tokens. Protected-mode loads are explicitly unsupported.
MOV8C/8E supports register and word-memory transfers; operand-prefix66 does not
widen memory accesses. MOV r32,Sreg is rejected pending legacy upper-half policy.
POP ES/SS/DS/FS/GS reads through old SS using the original stack-address width,
then commits pointer and segment state together. SS loads set ss_shadow=1;only
successful following retirement clears it. This records the boundary inhibition
state;external interrupt/NMI/debug delivery is still not implemented.

Backend trace records now include a fourth, length-prefixed segment-state section
with selectors,cache attributes,mode and SS shadow. Compiled fixed-image runners
use runtime caches and choose the stack-size variant at each stack instruction.
References: Intel386 [MOV](https://pdos.csail.mit.edu/6.828/2012/readings/i386/MOV.htm),
[POP](https://pdos.csail.mit.edu/6.828/2017/readings/i386/POP.htm),and
[SS inhibition](https://pdos.csail.mit.edu/6.828/2004/readings/i386/s09_02.htm).

## Direct-offset and immediate MOV forms (0.24)

A0/A1 load AL/AX/EAX from a direct offset;A2/A3 store the accumulator. Address
width (67) controls only encoded offset size,independently of operand width (66).
DS is the default and segment overrides apply;loaded-cache limits still apply
when the offset is32 bits. C6/C7 /0 writes an immediate to register or memory,
including high-byte registers and existing16/32-bit ModRM/SIB forms. Other group
extensions are rejected. These decode into existing PIL constant/load/store/write
operations;flags and markers remain intact and stores commit after checks.

## Near procedures and restartable string elements

`load_segment out SEG offset WIDTH` and `store_segment SEG offset value` use
explicit loaded caches per access (0..5 =ES,CS,SS,DS,FS,GS). Offsets are bv32;
widths8/16/32;one staged store is retained. Source reads precede the store and
all effects of a faulting element remain uncommitted. Requires machine context.
Named GPR reads/writes now also admit low8 values for AL string operations.

Near CALL E8/FF2 and RET C3/C2 use shared descriptions and independent operand/
SS sizes. Indirect targets snapshot the old register/memory value;RET cleanup
is an unsigned imm16 byte count. Far/privilege transfers are not admitted.

MOVS/STOS/LODS support8/16/32-bit elements and independent16/32-bit addresses.
DS (or source override) and fixed ES destination are separate accesses;DF controls
pointer direction. F3 REP commits one element per execution step. `retire_if bv1`
provides an effect-free early retirement guard;the compiler rejects earlier
writes/branches before it. `resume_if bv1` commits partial progress while keeping
the original EIP;it cannot be combined with another branch/resume effect.
SS shadow is retained during partial progress and retired only on completion.
Fault retry uses updated pointers/count without replaying committed elements.
REP with zero count touches no data memory. F2/CMPS/SCAS/port strings remain out.

Debugger steps and backend checkpoints now include REP element boundaries.
DOS `--steps` counts bounded execution steps;`completed` counts fully retired
instructions,while `PIL` includes element steps. Mix reports list REP element
counts separately. These counts are not whole-program instruction coverage.
The fixed-image generated runner retains its byte guards;self-modifying-code
fallback and interrupt delivery remain separate work. `--emit-rust` accepts
call,ret,movs,stos,lods,rep_movs,rep_stos,rep_lods with default word stack/address
profiles;the decoder binds the actual profiles and source segment.

## Real-mode flags and counter branches (0.26 implementation)

`read_flags out` snapshots bv32 EFLAGS; `require_real_mode` requires supplied
machine state with real_mode=true and VM clear, otherwise InvalidContext before
commit. `interrupt_shadow_if bv1` stages one STI-delay effect, set after full
retirement when true. It cannot precede an early retire guard. Shadows are
separate SS/STI state, included in full traces; partial REP and faults retain
both, and full PIL, monitor-I/O and DOS-bridge retirements advance both.

FA/FB implement real-mode CLI/STI. IF becomes defined while other flags/markers
stay unchanged. STI requests delay only if incoming IF was clear, so consecutive
STI does not extend it. This is bookkeeping, not interrupt/exception/NMI delivery
or protected/V86 privilege modeling. These modes are explicitly rejected.

9C/9D implement real-mode PUSHF/POPF and 66 PUSHFD/POPFD. SS size controls stack
offset and operand size controls slot bytes. PUSH images include documented
386 bits0,2,4,6..14 and fixed bit1; RF/VM and reserved bits are excluded. POP
restores mask0x7fd5 and defines those bits; RF/VM and internal reserved bits /
markers stay unchanged. POP does not create STI delay. Stack failures preserve
all pending CPU/RAM/machine effects; legacy shutdown/double-fault delivery is
not modeled and existing normalized stack errors remain.

E3 JCXZ/JECXZ reads CX/ECX according to address size (67). Operand size (66)
controls target wrapping; signed displacement is always8 bits. Counter,flags
and markers are preserved. Only taken branches validate target against CS.limit.

## LEAVE (0.27 implementation)

C9 copies BP/EBP into SP/ESP according to SS stack-address size,then loads the
saved BP/EBP according to operand size. Address prefix67 and segment overrides
do not redirect the stack. The shared PIL snapshot/load/named-write effects
commit the frame teardown only after a valid read; faults preserve all pending
register/flag/memory/shadow state. Flags and undefined markers are preserved.
The generated runner now dispatches alternative SS sizes for named SP writes
as well as reads,covering LEAVE after a segment load. ENTER remains unsupported.

## Comparison strings and memory SETcc (0.28 implementation)

CMPS/SCAS8/16/32 use independent address16/32,DF,source segment override for
CMPS and fixed ES destination. CMP subtraction effects define six arithmetic
flags; pointers advance only on success. F3 REPE and F2 REPNE test the newly
computed ZF after each element,not incoming ZF. Count zero avoids data access.
Each element consumes a budget step;full retirement occurs on count exhaustion
or failed ZF repeat condition. Movement-string F2 remains rejected.

`checkpoint_flags` is a first-operation generic machine effect. Machine-backed
execution remembers entry EIP/EFLAGS/undefined metadata across partial resumes.
Execution faults restore that original flag state,clear the checkpoint and
retain completed element count/pointer progress. Full retirement clears it.
The checkpoint is included in traces;debugger and DOS runner retain fault-state
updates. Normal single-element CMPS/SCAS faults retain incoming state. This
models memory/execution fault recovery,not interrupt delivery or exception frames.

SETcc may now store one byte to ordinary ModRM/SIB memory,using existing PIL
conditional selection/store effects. A false predicate still stores zero and
checks permissions. Flags/markers/registers are unchanged; /0 profile restriction
and existing normalized segment/memory checks remain.

Reference: [Intel REP instruction semantics](https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-software-developer-vol-2b-manual.pdf), including entry-EFLAGS recovery for faulting REPE/REPNE CMPS/SCAS; legacy386 REP pseudocode has inverted stop-condition typos,so use its detailed step6 description for ZF termination.

## Real-mode far return and checked frame spans (0.29)

`check_segment SEG OFFSET BYTES` checks a positive literal byte span through
an incoming segment cache without reading memory. OFFSET is bv64 to reject
wraparound before narrowing to a32-bit address. SS violations use the existing
StackSegment model error; it does not deliver a guest exception frame.
`far_branch SELECTOR OFFSET` stages a real-mode CS cache/selector and EIP update.
SELECTOR is bv16,OFFSET bv32; the new ordinary real-mode cache has limitFFFFh.
The effect rejects protected/V86 contexts and offsets beyond that new limit.
It cannot mix with another segment update or branch. Changes commit at retire.

Original retf.pil uses these effects plus ordered SS loads and named SP writes.
CB and CA imm16 accept operand16/32 independently of SS size;the second slot
has the operand width and only its low16 selector bits are used. Cleanup is an
unsigned byte count. Both complete slots are range-checked before reading.
Flags/undefined markers stay intact;all new CPU/CS/SP effects cancel on a fault.

Image fetch bounds are anchored to the initial physical code window,not the
current logical offsets. CS aliases inside that window work after RETF;copied
code outside it remains rejected. Fixed generated images dispatch by physical
instruction address,with live logical next-IP/code limits and byte guards.
They do not gain dynamic compilation of unknown or changed instruction bytes.
Reference: [Intel80386 RET semantics](https://pdos.csail.mit.edu/6.828/2018/readings/i386/RET.htm).
