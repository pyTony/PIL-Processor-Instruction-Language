# DOS Doom and 3DBench, 10 November 2026

This increment uses the ordinary release interpreter, without shadow/native
program substitutions. The user's game executables and WAD/data files are
local inputs, not repository fixtures. No game assets or ROMs are distributed.

## Reproduction

Build with `cargo build --release --locked`. Run from the game directory or
mount that directory so the executable can find its data:

```powershell
python tools/dos_terminal.py --engine target/release/pil.exe --mount C:/games/doom --args '-nosound -nomusic -warp 1 1 -skill 3' C:/games/doom/DOOM.EXE
python tools/dos_terminal.py --engine target/release/pil.exe --mount C:/games/3dbench C:/games/3dbench/3DBENCH.EXE
```

The DOS terminal enables the host clock for PIT interrupts. In a scripted
`Session`, explicitly send `clock 0`. `tools/verify_dos_games.py` makes private
temporary copies, enables that clock and checks for changing graphics frames.
Its frame check is a smoke test, not a benchmark-score oracle.

## Observed local results

- Doom v1.9 shareware, DOS/4GW: initializes an 8 MiB zone and renders gameplay
  with `-timedemo demo3 -nosound -nomusic`. It ran 996 million guest steps
  without a service stop in a bounded run, with changing frame hashes.
  A separate `-warp 1 1 -skill 3 -nosound -nomusic` run opens the Doom menu
  after ESC, verifying guest keyboard IRQ1 handling. No completed timedemo
  time/FPS or sound output is claimed.
- Superscape 3DBench 1.0: renders its animated 3D scene and continuously displays
  a score; 4.4 frames/sec was visible in one capture. The bounded run continued
  for more than 840 million guest steps without a service stop. This score is
  emulator/host dependent and is not a physical 386 cycle estimate.
- 680 Rust tests passed, 2 ignored; the additional generated ENTER test passed
  separately. Full-state generated/interpreter comparisons remain in the suite.

## Fixes and limits

AH3F validates the returned short-read span rather than all of requested CX;
a genuinely crossing returned span fails with DOS error 5 without advancing
the file. Absent INT15/BF extender probes fail without resident signatures.
16 MiB physical RAM is separate from the 640 KiB DOS allocation arena; AH88
and CMOS agree on 15 MiB extended memory, with A20 initially disabled. No
XMS driver is supplied.

PIC timer/keyboard interrupts use the protected IDT after PE, keeping IF,
SS/STI inhibition, masks and EOI. Port instructions accept operand/address/
segment prefixes within 15 bytes and reject unimplemented dword device I/O.
VGA 320x200 scanout supports the chain-4 to unchained transition, plane masks
and CRTC page starts. Other VGA resolutions/timing changes remain unsupported.
Only motor-off floppy digital-output writes are accepted; disk commands are
not simulated.

ENTER probes write access at the final SP byte. Hand-computed paging/SS
expectations and a compiled generated-Rust test check rollback, retry and the
absence of a real store. The local test386 HEX ROM still stops at POST 1Ah;
this increment does not claim a full ROM pass or task switching.
