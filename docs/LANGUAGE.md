# Implemented PIL subset, v0.5

This document describes the running prototype's source grammar. The larger
processor_instruction_language_v01.md is a design proposal and includes
features not implemented yet.

## Values and compilation

One operation per line; whitespace separates fields; # begins a comment.
Values are named SSA bit vectors. Every value must be defined before use,
and a name may only be assigned once. Widths range from 1 to 64; arithmetic
wraps at the declared width. Binary operands require equal widths.
The operand bindings currently support 8, 16 and 32 bits.

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
| trunc out input WIDTH | Truncate |
| add/sub/xor/and/or out left right | Modular arithmetic or bitwise operation |
| not out input | Bitwise complement at the input width |
| bit out input INDEX | Extract a bv<1> |
| zero out input | bv<1> result of testing for zero |
| parity out input | bv<1> even parity of the low byte |
| select out predicate yes no | Select one of two equal-width values; predicate must be bv<1> |
| write dst input | Stage a destination write |
| write_if dst input predicate | Stage a write only for predicate=1 |
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

Only one destination-write statement is allowed per description, whether
conditional or unconditional. The predicate does not exempt it from static
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
