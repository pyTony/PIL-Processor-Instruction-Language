# Experimental DOS mouse support (INT 33h)

PILdos now has an **opt-in software Microsoft mouse API** for the regular
Python/Tk DOS terminal. This is deliberately separate from the PS/2 hardware
BIOS (`INT 15h/AH=C2h`) and from any mouse driver a DOS program installs.

## Try it

```powershell
cargo build --release --locked
python tools/dos_terminal.py --mouse --engine .\target\release\pil.exe .\programs\dos\MOUSETEST.COM
```

Build the demonstration COM file with NASM if it is not already present:

```powershell
nasm -f bin programs/dos/MOUSETEST.ASM -o programs/dos/MOUSETEST.COM
```

The test calls INT 33h reset and show, then polls buttons/coordinates.
Move the pointer and left-click inside the DOS window to exit the test.
In the interactive DOS text window the mouse pointer is rendered as a
highlighted character; graphics modes use a host-only bitmap arrow. Guest
B800/A000 video RAM is unchanged by the cursor overlay.

By default `--mouse` is **off**, retaining the existing "no mouse driver"
answer for FreeDOS EDIT, Turbo Pascal, and other programs that might otherwise
activate unsupported driver features. Only turn it on when testing
a DOS program's mouse interface.

Once the guest calls AX=0000h to reset the driver, left/right/middle button
events are forwarded as bits 0/1/2. Host right-click is delivered to the DOS
program while its mouse is active. Shift+right-click opens the host Drives
menu, and Shift+drag still selects host text for copying.

## Implemented INT 33h functions

Behaviour is checked against RBIL's INT 33h list and CuteMouse 2.1, the
FreeDOS mouse driver (its documentation and source, not copied).

| AX | Meaning |
| --- | --- |
| 0000h | Reset; AX=FFFFh, BX=3 buttons; hide and center cursor; re-enables after 001Fh |
| 0001h / 0002h | Show (also cancels a 0010h region) / hide (nested counter) |
| 0003h / 0004h | Read / set virtual cursor position and buttons |
| 0005h / 0006h | Read and clear per-button press / release counts and coordinates |
| 0007h / 0008h | Horizontal / vertical position limits |
| 0009h | Graphics cursor: hot spot BX/CX, 16 screen-mask + 16 cursor-mask rows at ES:DX |
| 000Ah | Software text cursor (BX=0): screen mask CX, cursor mask DX (default 77FFh/7700h) |
| 000Bh | Read and clear the horizontal/vertical motion counters (mickeys) |
| 000Ch | Install/clear the far event routine (CX call mask, low byte kept; ES:DX routine) |
| 000Dh / 000Eh | Light pen emulation on/off: accepted, no effect (as CuteMouse) |
| 000Fh | Mickey-to-8-pixel ratio used for the motion counters |
| 0010h | Update region (CX,DX)-(SI,DI): the cursor is hidden while inside it |
| 0012h | Large graphics cursor: not done, AX stays 0012h (as CuteMouse) |
| 0013h | Double-speed threshold (0 = 64); stored, no acceleration is modelled |
| 0014h | Exchange event routines: returns the previous CX mask and ES:DX |
| 0015h / 0016h / 0017h | State size (158 bytes) / save to ES:DX / restore from ES:DX; BX is ignored |
| 001Ah / 001Bh | Set / get sensitivity BX,CX and threshold DX (default 50, 50, 64) |
| 001Ch / 001Dh | Interrupt rate (InPort only) / display page: accepted, no effect (as CuteMouse) |
| 001Eh | Display page: the BIOS active page at 0040:0062 |
| 001Fh / 0020h | Disable (AX=001Fh, ES:BX = INT 33h vector before the driver) / enable |
| 0021h | Software reset: resets the driver state like 0000h, AX=FFFFh, BX=3 |
| 0022h / 0023h | Message language: ignored / BX=0 (English), as a US driver |
| 0024h | Version 7.05 in BX (CuteMouse's), CH=4 PS/2 type, CL=0 |
| 0026h | BX = disabled flag, CX/DX = maximum virtual coordinates |

Any other AX stops the session with "INT 33h mouse AX=....h is not
implemented" rather than returning a guess. Not implemented: 0011h (wheel),
0018h/0019h alternate handlers, 0025h, 0027h and later, and the hardware
text cursor (000Ah with BX<>0).

The virtual X range is 0..639 in supported text, CGA and VGA modes, including
320-pixel graphics modes. Text coordinates are quantized to 8-pixel cells.
Mode 10h uses Y=0..349 and mode 12h uses Y=0..479; most others use 0..199.
The host sends `mouse X Y BUTTONS` via the same JSON-line session protocol
as keyboard input. Snapshot replies expose `mouse_present`, `mouse_active`,
`mouse_visible`, `mouse_drawn`, `mouse_x`, `mouse_y`, `mouse_max_x` and
`mouse_max_y`, and after AX=0009h `mouse_hotspot` and `mouse_shape` (32
little-endian words).

## Cursor drawing

In text modes the session applies the software text cursor to the copy of
the screen it sends: the character/attribute word under the cursor is ANDed
with the screen mask and XORed with the cursor mask, and `mouse_drawn` is
true. The default masks swap foreground and background. In graphics modes
the terminal draws the program's AX=0009h shape (for each pixel: screen 0,
cursor 0 black; 0,1 white; 1,0 unchanged; 1,1 inverted), or its own arrow
until a program defines one. Neither changes B800h or A000h memory.

## Callback behavior and manual test

Build and run the event-driven demo in addition to the polling demo:

```powershell
nasm -f bin programs/dos/MOUSECALL.ASM -o programs/dos/MOUSECALL.COM
python tools/dos_terminal.py --mouse --engine .\target\release\pil.exe programs/dos/MOUSECALL.COM
```

`MOUSECALL.COM` installs a callback, waits without polling the button,
increments a word in guest memory when the left button goes down, then
unregisters and exits. The callback must execute `RETF`, **not `IRET`**.

Callback CX mask bit 0 receives movement, bits 1/2 left press/release,
3/4 right press/release and 5/6 middle press/release; bits 7-15 are unused
(CX=FFFFh is accepted). On entry, AX holds the conditions that occurred and
are in the call mask, BX the current buttons, CX/DX virtual X/Y, and SI/DI
the horizontal/vertical motion counters as AX=000Bh would read them at that
event (CuteMouse passes the same counters). Interrupts are enabled, as
CuteMouse does STI before the call. DS is the emulated driver's data
segment; a handler must load its own DS or address its own data through CS. The interrupted program's
registers, segment caches and flags are restored on callback return.
Guest RAM writes made by the callback remain visible.

Events are queued with a bounded 256-event buffer; adjacent
movement-only samples coalesce while button changes remain separate.
Delivery is postponed when interrupts are disabled, during the x86
interrupt shadow, or while a previous callback is active. A guest
handler must be reachable in the active DOS executable or system code
window. A private BIOS continuation catches the callback's `RETF`,
checks its stack balance, and resumes the interrupted instruction.

## Not implemented yet

- The functions listed above as not implemented.
- Physical PS/2 controller or IRQ12 input, and motion acceleration: the
  pointer follows the host pointer, so sensitivity and threshold are only
  stored and reported. Motion counters come from the host position change
  times the 000Fh ratio.
- Mouse input to the native Rust-only Dine desktop window or compiled
  standalone DOS executables. Those have separate frontend lifecycles.

These are reasons to keep this feature **experimental and opt-in** until
validated with the actual target programs.

## Regression checks

```powershell
cargo test --locked mouse::tests --bin pil
python -m unittest discover -s tests/gui -p test_dos_terminal.py
cargo test --locked
```

Also verify a session **without** `--mouse`: AX=0000h still reports AX=0,
BX=0, and FreeDOS EDIT/Turbo Pascal continue working with the keyboard.
Test left/right/middle press/release and repeated clicks, guest AX=0004h
cursor relocation, text/320-wide/640-wide mode coordinate mapping, and
normal host copy/selection.

Manual testing of `MOUSECALL.COM` and real event-driven applications is the
next compatibility milestone for Turbo Vision and DOS mouse programs.
