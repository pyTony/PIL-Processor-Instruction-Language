# PIL reference — implemented language

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
| `read_flag out FLAG` | Incoming concrete CF/PF/AF/ZF/SF/OF as `bv<1>` |
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
classifications. Other EFLAGS bits are automatically preserved; the language
cannot currently modify DF, IF or privileged system flags.

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
