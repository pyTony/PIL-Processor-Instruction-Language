# 80387 compatibility: legacy 8087/80287 control instructions

A protected-mode DOS application launched from the Retro Software Browser
stopped at `23DC:0243` on `DB E4`, with CW=037F, SW=0000, ST(0)=empty.
This is `FNSETPM`, the 80287 "set protected mode" instruction.

Intel's *80387 Programmer's Reference Manual* (order 231917-001),
section 6.1.2 and appendices A/C, specifies that the 80387 ignores this
instruction. The same manual marks the older 8087 FENI and FDISI as FNOP.

| Encoding | Historical mnemonic | 80387 behavior |
| --- | --- | --- |
| `DB E0` | `FNENI` / `FENI` | No operation |
| `DB E1` | `FNDISI` / `FDISI` | No operation |
| `DB E4` | `FNSETPM` / `FSETPM` | No operation |

Only these three exact encodings get the `LegacyControlNop` classification;
other unsupported ESC encodings continue to fail explicitly. The CPU's
existing EM/TS check (#NM) precedes dispatch. Successful execution changes
neither CPU flags/registers, x87 status/control/register stack nor memory,
and works with an empty x87 stack.

Tests cover all three encodings, exact state preservation, and real-mode DOS
COM execution with an initially empty x87 stack.

Reference: https://www.dosdays.co.uk/media/intel/231917-001_80387_Programmers_Reference_Manual_1987.pdf

Integration acceptance: rebuild `pil`, run the same DOS application with
`--fpu 80387` and verify that it passes `DB E4`. This does not claim the
rest of Doom or Quake works yet.
