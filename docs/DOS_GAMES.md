# DOS Doom and 3DBench, 10 November 2026

## Native Windows input milestone, 17 November 2026

Issue [#238](https://github.com/pyTony/PIL-Processor-Instruction-Language/issues/238)
reports visible gameplay but no held movement in the Rust native window. Its
old route synthesized a make and immediate break for every translated key.
The earlier GUI edge work below applied to the Python frontend/session protocol;
it did not wire the Rust native window's Windows releases.

The native route now joins physical set-1 identity with Windows text translation
in a host-independent state machine. IRQ1 receives separate make/break codes,
including E0 prefixes and repeats; chained BIOS handlers receive one translated
word per make. BIOS-only programs keep down-only text input. Pending input and
paste share one FIFO, with all-or-none extended-code enqueue. Focus loss and
host menus release tracked keys, including modifiers.

Deterministic Rust fixtures cover held/repeated/simultaneous keys, releases,
focus loss, layout changes, AltGr text, controls, host shortcuts, BIOS versus IRQ1
dispatch, and full queues. These are synthetic tests: no new DOOM, EDIT, Turbo
Pascal or other private DOS application pass is claimed by this increment.

### Windows retest

1. Build `cargo build --release --locked`; launch the Rust native console with
   `target/release/pil.exe --dos-console --window --config pildos.conf` (or the
   same configured Retro entry previously used). Verify the executable is this
   build and use the same game installation and options for comparisons.
2. At FreeCOM, type text, hold Backspace, recall commands with arrows, and try
   Ctrl+C, AltGr characters and paste. In EDIT and Turbo Pascal, check typing,
   held navigation, Shift selection, Ctrl/Alt shortcuts, function keys, menus,
   clipboard host shortcuts and normal exit back to the shell.
3. Launch DOOM through the same Retro entry as the original report. Hold each
   configured movement key for several frames; release and verify movement
   stops. Hold two movement keys together, release one, then the other. Repeat
   with Ctrl/Alt/Shift actions. Use the game's configured bindings rather than
   assuming WASD. Repeat from the D: installation that previously stopped at
   word IN 224h; that is a separate reproduction, fixed by merged PR #239.
4. While holding movement, Alt+Tab away, release outside the window and return.
   Verify no stuck movement/modifiers. Repeat with the host right-click menu,
   Shift+F10 and Ctrl+Shift+Enter. Check ordinary shell input after game exit.
5. If movement still fails, use Diagnostics > Save live guest snapshot twice
   while holding and twice after release. Compare CS:IP, PIC/IRQ1 state and host
   pending input. Disable instruction history afterwards to restore speed.
   Record build SHA, launch entry, bindings, exact action and observed result.

### Next smallest Retro milestone

After the native input retest, use merged PR #237 snapshots to diagnose the
reported Wolfenstein black-screen/polling loop before choosing a CPU, VGA or
service fix. Require a reproducible trace and an original regression fixture.
Issue #216 still tracks digital audio validation; word mixer I/O #239 does not
prove full audio compatibility. OPL music remains separate. Turbo C++ 3 issue
#234 needs a bounded allocation/DPMI-probe trace before implementing services;
the IDE's memory message alone is insufficient evidence for a DPMI fix.
CI modularization is in separate PR #243; this change does not alter workflows
or the recently refreshed README status.

## Original 10 November increment

This increment uses the ordinary release interpreter, without shadow/native
program substitutions. The user's game executables and WAD/data files are
local inputs, not repository fixtures. No game assets or ROMs are distributed.

## Reproduction

Build with `cargo build --release --locked`. Run from the game directory or
mount that directory so the executable can find its data:

```powershell
python tools/dos_terminal.py --engine target/release/pil.exe --mount games/doom --args '-nosound -nomusic -warp 1 1 -skill 3' games/doom/DOOM.EXE
python tools/dos_terminal.py --engine target/release/pil.exe --mount games/3dbench games/3dbench/3DBENCH.EXE
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


## Exit fixes, 0.37.0-alpha.2

Both local games now return to a FreeCOM `D:\>` prompt with no service stop.
The parent session reaches `waiting_input` after the child finishes.
3DBench exits on ESC; Doom exits via ESC, Q, Enter, Y.

Doom needed BIOS INT1A/AH02 (BCD time); AH04 (BCD date) uses the same
session RTC. Each call samples the clock once, preserves other registers
and flag bits, defines/clears CF, and does not mutate the clock or BDA.
The daylight-saving return byte is 0; the configured clock offset supplies
local time. RTC set/alarm calls remain outside this increment.

3DBench's INT09 handler samples a pressed-key table. An atomic make/break
pair was cleared before the main loop could observe the key. GUI key-down
and key-up are now separate IRQ1 events, including in the headless graphics
window. Focus loss releases held keys. The BIOS console and older engines
keep the legacy `key ASCII SCAN` path. New engines advertise `key_edges`
and the currently active `raw_keyboard` path; scripts may use
`keydown ASCII SCAN` and `keyup ASCII SCAN`. A synthetic guest reproduces
the original missed-ESC bug and exits through the new session protocol.

Validation: 685 Rust tests pass (2 ignored); 37 frontend tests pass.
Private game runs reach Exit(0) directly and return to FreeCOM under EXEC.
Game data and the diagnostic binaries remain local and are not fixtures.
