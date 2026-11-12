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

| AX | Meaning |
| --- | --- |
| 0000h | Reset; AX=FFFFh, BX=3 buttons; hide and center cursor |
| 0001h / 0002h | Show / hide (nested visibility counter) |
| 0003h / 0004h | Read / set virtual cursor position and buttons |
| 0005h / 0006h | Read and clear per-button press / release counts and coordinates |
| 0007h / 0008h | Horizontal / vertical position limits |
| 000Bh | Read and clear signed relative movement (simplified mickeys) |
| 000Ch | Install/clear real-mode far mouse-event handler (CX event mask, ES:DX routine) |
| 000Fh | Set mickey-to-pixel ratio used for reported relative motion |
| 0021h | Report installed driver without resetting |
| 0026h | Get maximum virtual coordinates |

The virtual X range is 0..639 in supported text, CGA and VGA modes, including
320-pixel graphics modes. Text coordinates are quantized to 8-pixel cells.
Mode 10h uses Y=0..349 and mode 12h uses Y=0..479; most others use 0..199.
The host sends `mouse X Y BUTTONS` via the same JSON-line session protocol
as keyboard input. Snapshot replies expose `mouse_present`, `mouse_active`,
`mouse_visible`, `mouse_x`, `mouse_y`, `mouse_max_x` and `mouse_max_y`.

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
3/4 right press/release and 5/6 middle press/release. On entry, AX is
the event-condition mask, BX the current buttons, CX/DX virtual X/Y,
SI horizontal motion and DI vertical motion (simplified mickeys).
DS is the emulated driver's data segment; a handler must load its own
DS or address its own data through CS. The interrupted program's
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
- AX=0009h/000Ah guest-defined graphics/text cursor shapes and driver
  save/restore state.
- Physical PS/2 controller or IRQ12 input, mouse-wheel extensions, and
  faithful motion acceleration. Relative movements are approximated.
- Mouse input to the native Rust-only Dine desktop window or compiled
  standalone DOS executables. Those have separate frontend lifecycles.

These are reasons to keep this feature **experimental and opt-in** until
validated with the actual target programs.

## Regression checks

```powershell
cargo test --locked dos_mouse --bin pil
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
