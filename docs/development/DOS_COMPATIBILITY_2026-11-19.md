# Shared DOS compatibility corrections — 19 November 2026

Three independent user-reported stops are addressed in the shared runtime.
No application detection, captured addresses, saved output or per-program
engine is used.

## FPU word integer stores

The SuperCalc log stopped on `DF /3`, `FISTP m16int`, with a zero in ST(0).
The generic x87 decoder now admits that form. It reuses the signed integer
conversion and the control word's rounding mode, validates the signed 16-bit
range and entire destination, stores exactly two bytes, then pops ST(0).
CPU retirement remains owned by Dine. Invalid/unimplemented conversions and
unmasked precision exceptions retain the existing explicit-stop policy;
complete x87 exception handling is not added.

Original fixtures cover signed limits, rounding ties and all four rounding
modes, C1/PE, 16/32-bit addressing, segment overrides, operand prefixes,
read-only second bytes, segment faults and empty stacks. These tests do not
establish that the whole spreadsheet now works.

## Sound Blaster FM aliases

Prince of Persia reached an OUT at 228h after the earlier 22Ah read-only
DSP-latch correction. The Sound Blaster 2.0 FM address/data aliases at
228h/229h now use the existing absent-OPL profile: reads float high, writes
are ignored. Consecutive word cycles are handled transactionally like the
mixer's index/data pair. DSP replies, DMA and digital audio remain separate.
No FM music is synthesized. The user reports that selecting PC speaker in
the game's setup also allows it to start.

## Guest-visible DOS memory chains

Volkov Commander split/relinked a valid MCB chain while arranging resident
memory. Dine's private block tracker still held the previous shape, so later
memory calls rejected it and the program and parent shell printed misleading
out-of-memory messages.

Memory services now reconcile changed metadata by walking the visible MCB
chain from its fixed arena start. Every header must have an M/Z type, advance
monotonically, remain below the conventional-memory limit, and terminate
within the block-count bound. Malformed chains still return DOS error 7;
they are never repaired by overwriting guest RAM. Termination also refreshes
the chain before releasing owned blocks and preflights all owner writes.
Normal unchanged chains retain the existing validation path.

A local diagnostic run using the user's unchanged executable and installation
reproduced the original allocation failure. With this change it reached the
file panels and remained running through 40 million guest execution steps.
The temporary diagnostic harness and third-party files are not committed.
Synthetic fixtures cover guest splits, allocation, releasing newly linked
owned blocks, malformed and oversized chains, block-count limits and
read-only release rollback. No extra conventional memory, EMS or XMS is added.

The separately reported Prince setup exit was reproduced as a missing
`SETUP.CFG`; the user confirmed deleting that installation file. No runtime
workaround or replacement configuration is supplied for that case.
