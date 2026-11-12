# 80387 control-word handling: Turbo Pascal 7 compiler probe

## Reproduction

PILdos started with `--fpu 80387`; Turbo Pascal 7 IDE opens
`TP\PIL\MANDELFP.PAS` with `{$DEFINE USE387}` and `{$N+}`.
During **Compiling, Destination: Memory**, the interpreter stopped at:

```
80387 unsupported instruction
CS:IP 53AA:72FA
bytes=D9 3F
steps=5353353
```

`D9 3F` is `FNSTCW word ptr [BX]` (ModR/M register field /7,
memory address selected by BX, 16-bit real-mode address size).
This is inside Turbo Pascal's compile-time/runtime coprocessor control,
not a Mandelbrot calculation.

## Implemented

* `FNSTCW m16`, opcode `D9 /7`: store the 80387 16-bit control word
  to the resolved guest address without altering x87 register stack,
  control word or status.
* `FLDCW m16`, opcode `D9 /5`: read the guest 16-bit control word
  exactly, including precision, rounding and exception mask bits.
* Both use the existing 16/32-bit effective-address decoder, segment
  defaults and overrides, 2-byte segment/permission checking and the
  memory-protection model. Guest memory faults leave FPU state intact;
  stores preflight the full 2-byte destination.
* Real-mode DOS COM regressions test the exact `D9 3F` sequence,
  subsequent control load/store, BP default SS, ES override, protected
  destination, crossing DS limit, and a loaded precision mode that
  arithmetic still refuses explicitly.

## Scope and next verification

This adds the architectural control-word transfer, not full 80387
rounding/exception semantics. Existing finite arithmetic only supports
PC=11 (64-bit significand) and RC=00 (round-to-nearest/even).
`FLDCW` may legally load other settings; later arithmetic with an
unimplemented setting must still stop rather than pretend to succeed.

After CI passes, rebuild PILdos and retry **the same Turbo Pascal
Compile / Run procedure** using `--fpu 80387`. Report the next
`CS:IP`, instruction bytes and GUI state. A later stop should be
treated as the next missing FPU/DOS service, not automatically as
success running `MANDELFP.PAS`.

The software-Real Mandelbrot checksums `SUM=737`,
`WEIGHTED=97090` remain the baseline, not a phase-5 result.
