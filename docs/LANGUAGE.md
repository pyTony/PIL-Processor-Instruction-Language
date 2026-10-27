# Implemented PIL subset, v0.5

This document describes the running prototype's source grammar. The larger
processor_instruction_language_v01.md is a design proposal and includes
features not implemented yet.

## Values and compilation

One operation per line; whitespace separates fields; # begins a comment.
Values are named SSA bit vectors. Every value must be defined before use,
and a name may only be assigned once. Widths range from 1 to 64; arithmetic
wraps at the declared width. Binary operands require equal widths.
The destination binding uses the instruction operand width (8/16/32). The
source binding independently uses8/16/32 bits, fixed by its read statements;
all reads of src must agree on that width. If src is unread, its binding width
still defaults to operand width. Runtime View widths must match these compiled
widths. Generated APIs infer widths from the program and validate register/bit
ranges before effects. This supports MOVZX/MOVSX without decoder-side arithmetic.

$W, $EXT and $SIGN are text-template parameters: operand width, width+1 and
width-1. The prototype instantiates them before validation/tokenization.
conditions.pil and setcc.pil are combined before compilation; this is source
composition in the prototype, not a general include directive.

## Supported statements

| Statement | Meaning |
|---|---|
| read out dst WIDTH | Read destination binding from incoming CPU state |
| read out src WIDTH | Read source binding from incoming CPU state |
| read_flag out CF | Read incoming CF, PF, AF, ZF, SF or OF as bv<1> |
| const out WIDTH NUMBER | Define an unsigned decimal or hexadecimal constant |
| zext out input WIDTH | Zero-extend |
| sext out input WIDTH | Sign-extend a bit vector to an equal or wider width |
| trunc out input WIDTH | Truncate |
| add/sub/mul/xor/and/or out left right | Modular arithmetic or bitwise operation |
| shl/lshr/ashr out input count | Total shifts; oversized counts zero-fill or sign-fill; no implicit count masking |
| ult out left right | Unsigned comparison of equal-width values; bv<1> output |
| flag_if CF value defined active | If active, define value or mark undefined; otherwise preserve value and metadata; all three inputs bv<1> |
| not out input | Bitwise complement at the input width |
| bit out input INDEX | Extract a bv<1> |
| zero out input | bv<1> result of testing for zero |
| parity out input | bv<1> even parity of the low byte |
| select out predicate yes no | Select one of two equal-width values; predicate must be bv<1> |
| write dst input | Stage a destination write |
| write_if dst input predicate | Stage a destination write only for predicate=1 |
| write src input | Stage a source-binding write; inferred width must agree with source reads |
| write_if src input predicate | Stage a source-binding write only for predicate=1 |
| flag CF input | Stage a defined flag value; input must be bv<1> |
| preserve_flag CF | Explicitly preserve a flag value and its undefined-status bit |
| preserve_flags all | Preserve all six arithmetic flags |
| preserve_flags remaining | Preserve arithmetic flags not classified earlier |
| undef_flag AF | Declare an undefined flag output |
| require_width 8 | Reject other operand widths; used by SETcc |
| retire | Publish staged register, flag and diagnostic metadata writes |

Each of CF/PF/AF/ZF/SF/OF must be classified exactly once as defined,
preserved or undefined. Missing and duplicate classifications are errors.
Declare preserve_flags remaining after all explicit classifications.
Other EFLAGS bits are preserved automatically in this subset; privileged
system-flag changes are not implemented.

Only one write statement per binding (dst and src) is allowed per description,
whether conditional or unconditional. The predicate does not exempt it from static
validation. Reads always observe the incoming state, even after a staged
write; both interpreter and generated Rust use this rule.

select only selects already-computed values. It does not suppress evaluation
of either operand. write_if suppresses a register write, not earlier reads or
computations. No memory side effects exist yet; neither primitive should be
used to claim safe conditional memory access or general branching.

## Undefined flag policy

Cpu.undefined_flags records which concrete flag outputs are architecturally
undefined. undef_flag chooses concrete zero and sets the marker. This is a
deterministic emulation policy, not a claim that physical 386 hardware clears
that flag. Defined writes clear the corresponding marker. Preservation retains
both the previous concrete bit and the marker.

Undefined bits may still be read and used; they are not Rust UB, compiler
poison, a guest exception or an automatic abort. This release does not
propagate a symbolic uncertainty/taint through all downstream values. It
executes one deterministic concrete choice, retaining diagnostic markers
for the declared flag outputs.

Generated execute_with_metadata takes a mutable undefined-flag mask and
preserves this state across instructions. The older execute wrapper remains
for compatibility, but initializes/discards the diagnostic mask and should
not be used to inspect undefined status across a sequence.

## Conditional examples

The sixteen canonical conditions are o/no/b/ae/e/ne/be/a/s/ns/p/np/l/ge/le/g.
The descriptions seto through setg combine conditions.pil with setcc.pil,
write a byte containing 0 or 1, and preserve all flags. Mnemonic synonyms
such as setz/setnz are not implemented as aliases yet.

write_if_z is a generic predicated-register-write example at 8/16/32 bits.
It is not an implementation of a 386 instruction named CMOV. Guest EIP and Jcc targets are added in v0.4 below; instruction fetching remains future work.

Example source emission:

```sh
cargo run -- --emit-rust 8 setl > setl.rs
rustc --edition=2021 --crate-type=lib setl.rs
cargo run -- --emit-rust 32 and > and32.rs
```

## v0.4: guest offsets and conditional jumps

New tokens:

| Statement | Meaning |
|---|---|
| read_ip out current WIDTH | Read the incoming guest EIP as a bit vector |
| read_ip out next WIDTH | Read the caller-supplied fall-through offset |
| read_disp out WIDTH | Read the decoded signed displacement at operand width (16 or 32) |
| branch_if target predicate | Stage a bv<32> target when a bv<1> predicate is true |
| require_width 16_or_32 | Reject byte operand widths |

Only one branch effect is allowed per description. Successful retirement
commits EIP along with registers/flags. All reads observe incoming state.
The normal non-branch retirement offset is FlowContext.next_eip.

The j<condition> names compose conditions.pil with jcc.pil. The signed
relative offset is added to the supplied following-instruction offset. A taken
target wraps at the selected 16/32-bit operand width and is checked against
the supplied inclusive protected-mode CS limit. A failed check returns
GeneralProtection with error_code=0 and the original EIP, without committing
staged effects. A not-taken Jcc leaves the supplied next offset untruncated
and does not validate its unused target. Instruction-fetch faults are separate.

FlowContext includes next_eip, displacement (i32), displacement_bits and
code_limit. Displacement bits must be 8, or equal to the 16/32-bit operand
width; the signed value must fit. This is caller/decoder input validation,
not an x86 #UD. Invalid bindings/context are distinct from the guest #GP result.

This models near protected-mode offset checks, not complete segment/mode
semantics, exception delivery, descriptor caches or instruction fetching.
There is no CS base translation, memory access, binary decoder or far jump.
Decoded displacement/length and valid fall-through offsets are caller duties.

Use interpreter execute_flow and generated execute_with_flow for these
operations. Legacy execute/execute_with_metadata wrappers preserve their old
interfaces for descriptions without flow, and reject flow descriptions that
lack context. The simple interpreter execute wrapper keeps EIP unchanged.

```sh
cargo run -- --demo-branch
cargo run -- --emit-rust 16 jne > jne16.rs
rustc --edition=2021 --crate-type=lib jne16.rs
python3 verify_flow.py
```

Tests stay in src/main.rs for this milestone. A tests/ directory is planned
later; the next feature is a serial mock and interactive REPL debugger.

## v0.5 debugger and layout

PIL files now live in isa/i386/. The language tokens are unchanged. The host
REPL and serial mock are described in DEBUGGER.md. MOCK_RX/TX and
HARNESS_HALT are debugger harness actions, not new x86 instructions.

## v0.6: flat RAM effects

New tokens: `read_address out` returns the caller-supplied 32-bit flat address;
`load out address WIDTH` reads 8/16/32-bit little-endian data;
`store address value` stages one 8/16/32-bit RAM write. Address values must be
bv<32>; load widths and stored-value widths are checked independently of
address width. Unaligned accesses are supported. There may be multiple loads,
but at most one store per description, and no loads after a staged store.

`load_ram`, `store_ram` and `add_ram` are predecoded semantic helpers, not
binary instruction decoding or new x86 opcodes. Call interpreter
execute_memory or generated execute_with_memory with a mutable Ram and an
already calculated flat address. Existing arithmetic/flow APIs still work
for descriptions without memory and reject missing RAM context otherwise.

The RAM store is fully bounds/protection checked before mutation and is
published at retirement before the infallible CPU-state commit. A later branch
fault cancels it. Faults retain the instruction's original EIP and CPU flags,
registers and diagnostic markers. Reads occur in token order; stores are staged.
This is a restricted RAM-only model, not a universal rollback mechanism for
MMIO, I/O reads, multiple stores, page-table effects or REP instructions.

MemoryAccess errors report source EIP, address, byte count and read/write.
They are emulator RAM bounds/protection results, NOT architectural x86 #PF;
there is no paging/CR2 update or guest exception-handler delivery yet.
Protection here is a host-defined read-only RAM interval. Full segmentation,
page permission ordering, memory-mapped devices and instruction fetch remain
separate future work. RAM faults do not set arithmetic flags.

Generated Rust embeds the same RAM implementation while the description
interpreter and independent Python oracle exercise token semantics separately.
The supplied 65 Rust tests remain unexecuted in this authoring environment.


## v0.7: supplied address and segment context

The PIL vocabulary is unchanged. In flat execution, read_address/load/store
continue to use linear RAM addresses. With execute_segmented or generated
execute_with_segments, read_address supplies the resolved offset. Each load or
store translates its own offset using its actual token width and direction.

AddressContext carries a decoded EffectiveAddress (16-bit form or 32-bit
base/index/scale/displacement), optional segment override, and six normalized
SegmentCache values. Resolution snapshots the instruction's input registers.
Offsets wrap to address width. DS/SS selection follows base role; EBP as an
index alone does not select SS. Displacements are normalized by the caller.

Cache limits are inclusive expanded byte limits, not raw descriptor fields.
Expand-down lower bounds exclude the limit; upper bounds use the cache B bit.
Multi-byte offset spans do not wrap. Base+offset is a wrapping 32-bit linear
sum. Crossing the end of flat RAM, including the 4-GiB linear boundary, remains
a host RAM diagnostic; per-byte linear wrap is outside this prototype.

Segment bounds yield GeneralProtection(0) or StackSegment(0) for SS, at the
source EIP, before RAM access. Data/code permissions and unusable data caches
are checked. Invalid loaded SS caches are InvalidContext caller errors. The
caller must already have checked descriptor loads, presence and privilege;
there are no GDT/LDT loads, #NP delivery, CR2/page tables, or mode transitions.
All stores still commit only at retirement, including after translation.

## Signed IMUL description

Generic mul is modular multiplication at the common input width (1..64).
sext interprets the input high bit as sign and fills added high bits with it.
Neither operation directly changes flags. imul.pil widens two16/32-bit values
to32/64 bits, multiplies, truncates and compares the exact product with the
sign-extended low result. CF/OF indicate lost sign information; other arithmetic
flags are declared undefined. $DOUBLE expands to twice the operand width.

## Conditional flag effects and shifts

flag_if classifies its flag exactly once. With active0, it preserves both value
and diagnostic marker. With active1/defined1 it writes the supplied value and
clears the marker; active1/defined0 writes concrete zero and marks undefined.
All effects remain staged until retire. Invalid input types and duplicate flag
classification are rejected. Generic shifts take an unsigned count of any
bit-vector width. A count>=data width gives zero for shl/lshr and a repeated
sign bit for ashr; counts are never implicitly reduced modulo the host width.
The x86 descriptions explicitly AND with31 before invoking these operations.

## Extending moves, accumulator conversion and LEA

The binary monitor supports MOVZX and MOVSX with byte sources and16/32-bit
register destinations (0F B6/BE), and word sources with16/32-bit destinations
(0F B7/BF). Sources can be registers, including AH/CH/DH/BH, or memory.
Memory reads check only the source width; read-only memory is permitted.
The16-to16 forms copy the word unchanged and preserve upper register halves.

Opcode98 executes CBW (AL to AX) or, with66, CWDE (AX to EAX). Opcode99
executes CWD (AX sign into DX) or, with66, CDQ (EAX sign into EDX). CWD/CDQ
leave EAX untouched. Word destinations preserve their upper register halves.
All of these instructions preserve flags and undefined-flag markers.

LEA8D calculates a memory-form effective offset into a16/32-bit register.
Operand size and address size are independent:16-bit addresses wrap then zero
extend for32-bit destinations;32-bit offsets truncate for16-bit destinations.
It performs no data load, adds no segment base and checks no data-segment
access/limit. Segment overrides do not change the result. Instruction fetch
still uses the normal CS checks. A register-source LEA is a monitor diagnostic.
Disassembly shows an address expression without a data size or segment.

Standalone descriptions are named movzx8/movsx8 (byte source) and
movzx16/movsx16 (word source); movzx/movsx alias the byte forms. Destination
width comes from --emit-rust WIDTH. lea and cwd use16/32-bit destination width;
cdq aliases cwd. The binary decoder binds98/99 implicit registers before PIL
validation. The interpreter and generated runner execute the same tokens.

## Extension instruction demo

```text
demo extensions
list 64
run 1000
check
regs
mem16 0x180 1 hex
```

This original15-instruction guest exercises MOVZX/MOVSX, CBW/CWDE/CDQ, LEA,
16-to16 register copies (including DI self-aliases), and word memory sources.
Expected final values: EAX=FFFFFF80, EBX=00000080, ECX=FFFFFF80,
EDX=FFFFFFFF, ESI=00000085, DI=BP=SP=FF80 and RAM word0x180=FF80.
The checker verifies every register, that RAM word, preserved flags and halt.

The decoder now accepts word-source MOVSX/MOVZX with16-bit as well as32-bit
destinations. At16-to16 both operations copy the word unchanged; the destination
register's upper16 bits and flags/undefined markers are preserved. Memory
sources still perform a real word read with normal segment/RAM checks, including
read-only sources. A same-register instruction is not skipped, so fetch and
memory checks retain their normal behavior. In16-bit operand size,0F BF F8
selects DI,AX;0F BF FF selects DI,DI.

## Rotate and carry-constant descriptions (0.16)

ROL/ROR are ordinary PIL descriptions:paired total shifts plus OR compute
data;flag_if classifies CF/OF by the masked count,preserving other flags and
metadata. A count masked tozero differs from a full-width turn. CLC/STC use
a one-bit constant,flag CF and preserve_flags remaining. Neither instruction
family adds a semantic opcode to the token interpreter or Rust emitter.

## Register exchange (0.17 alpha)

XCHG8/16/32 register forms86/87 and AX/EAX accumulator forms91..97 are
supported;90 remains NOP (equivalent self-exchange in the386 profile). Both
operands are read before retirement,so AL/AH and identical-register aliases
work. Upper bits,flags and undefined markers are preserved. Memory XCHG and
LOCK remain outside this profile pending a bus-atomic memory interface.

PIL now permits one staged write to each named binding,dst and src,including
conditional source writes. All reads see incoming state. Active overlapping
writes must agree on their shared bits;otherwise InvalidBinding is returned
before any RAM,register,flag,marker or EIP commit. Source binding widths
remain independently validated. This is a two-binding foundation,not arbitrary
implicit-register writes or ordered multi-store support.
