# DOS console in Rust: `pil --dos-console`

For Android/Termux touch graphics without X11 or Tk, use the
[loopback browser interface](TERMUX_WEB.md) with the same DOS session backend.

## Native Windows held keys

The Rust native window forwards physical key-down and key-up independently.
Alphanumeric keys, arrows, function keys and modifiers stay down until released;
extended keys retain their E0 scan-code prefix. Host repeats produce additional
makes. The layout-translated character supplies a BIOS word on each make, so
guest IRQ1 handlers that chain to BIOS retain text entry. With no guest keyboard
handler, only translated down events enter the BIOS ring. Releases and modifier
edges do not type extra characters. Queue pressure retries events in order.
That direct BIOS-input path also updates the Shift/Ctrl/Alt bits at 0040:0017,
so INT 16h/AH=02h sees held modifiers even without a guest INT 09h handler.
Focus-loss releases clear those bits through the same shared BIOS logic.

Focus loss, window closure and opening the host menu release held guest keys.
Copy/paste, Shift+Insert, Shift+F10, the context-menu key, Ctrl+Shift+Enter and
Ctrl+Shift+F1..F4 keep their host functions. Paste remains a sequence of taps.
Windows translation still supplies AltGr and dead-key text; physical identity
does not change when a modifier is released first. Pause's E1 sequence and
Print Screen's multi-code hardware sequence are outside this input increment.
Terminal input still uses taps where the terminal supplies no reliable releases.

See the [Retro retest checklist](https://github.com/pyTony/PIL-Processor-Instruction-Language/blob/dev/docs/development/DOS_GAMES.md#native-windows-input-milestone-17-november-2026)
for interactive validation and the next diagnostic milestone.

## Startup, shortcuts and redirected input

`pil --dos-text` selects the Rust terminal console (`--dos-console --tty`).
With no program argument, both commands use startup from `pildos.conf`,
`PILDOS_CONFIG`, or `--config FILE`. They honor configured drives, country,
command tail, `fpu`, `mouse`, `ansi` and `utc_offset` (hours or +HH:MM, as in the setup editor).
Explicit command-line options override these defaults. Configured arguments
belong to configured startup; an explicitly named program starts with an empty
tail unless `--args` is given.

```powershell
.\target\release\pil.exe --dos-text --mouse --fpu 80387 --mount driveD images/FD14LITE.img::COMMAND.COM
```

On Windows, terminal mode opens the existing separate graphics window when
needed; `--rust-graphics` is accepted as an alias for `--tty`. No Python process
is involved. `PILdos.vbs --rust-text` and `sh PILdos.sh --rust-text` use this route,
forwarding remaining options. `PILDOS_ENGINE` selects another executable;
otherwise the shortcuts use `target/release/pil`. Ordinary shortcut launches
retain the existing Python/COM1 behavior. Existing `headless`, `com1`, `shell`,
`engine` and `rust_graphics` config flags belong to the Python
frontend; Rust text mode already supplies a terminal and native Windows graphics.

When the program ends or the session stops, the window (or the terminal's
alternate screen) closes and the console prints the program's last four
screen lines with text, then `DOS program ended: exit code N` (or the stop
reason), on standard output (so `> file` keeps it). So a program that ends at once, such as GW-BASIC
given a file it cannot find, still shows its message ("File not found").
Closing the window or Ctrl+] prints only the closing line.

When terminal-mode stdin or stdout is redirected, the Rust console uses a
UTF-8 transcript and line-buffered input. `--plain` or `DINE_TEXT=headless`
forces this mode. Typed text becomes CP437 BIOS keys, CRLF/LF becomes Enter,
and keys remain pending when the BIOS queue fills. Unrepresentable input, input
lines above 4096 UTF-8 bytes, and EOF during a guest read are explicit errors.
DOS output controls other than CR/LF/TAB are escaped, so guest escapes cannot
operate the host terminal. Transcript mode does not display direct VRAM or BIOS
drawings, graphics, or mouse input. Interactive mode still reads the shared screen.
`DINE_TEXT=terminal` requires terminal stdin/stdout; `auto` is the default.

The Dine shared-library entry point uses this same I/O adapter, in terminal or
transcript mode. It no longer discards typed lines and sends only Enter. Host
keyboard polling uses Crossterm on the execution thread, with no detached stdin
reader left after the caller returns/unloads the library. Raw input mode, Windows
output mode and the alternate screen are restored on return and unwind. The
existing BIOS mapping, ANSI screen renderer and guest ANSI driver are reused;
no second guest ANSI parser or per-program runtime is generated. Both
interpreter and backend sessions use the same boundary.

Physical held-key/modifier state and Ctrl+Break delivery depend on what the host
terminal reports. Blink, custom fonts/palettes and COM1-only/TCP
console routing retain the limitations listed below. The Python serial frontend
remains available for COM1/CTTY COM1 applications.

`pil --dos-console` runs a DOS session in the `pil` process and is its
keyboard and screen. It needs no Python and no Tk. It uses the same session
as `--dos-session`: the same drives, `--config`, `--mount`, `--args`, `--fpu`
and stop log. Nothing in it depends on which program runs.

```text
pil --dos-console [--tty|--window] [--mouse] [--ansi] [--host-pointer auto|show|hide] [--speaker-volume 0..100] [--country 1|358]
                  [--utc-offset SECONDS] [--config FILE] [--mount HOST_FOLDER]
                  [--args COMMAND_LINE] [--fpu none|80387] [PROGRAM]
```

PROGRAM is a .COM or .EXE file, or `IMAGE.img::INNER_PATH`, as in
`--dos-session`. Without it, the configuration's `program` and `args` start
(docs/DOS_CONFIG.md). The configuration's `fpu`, `mouse`, `ansi`,
`utc_offset` and `host_pointer` apply in both cases unless an option overrides them. For a
setup screen that edits the configuration and starts it, see `pil --pildos`
in [DOS_CONFIG.md](DOS_CONFIG.md).

| Option | Effect |
|---|---|
| `--tty` | Use the host terminal. This is the default except on Windows. |
| `--window` | Use a Win32 window. This is the default on Windows, and it works only there. |
| `--mouse` | Install the INT 33h driver, like `mouse on` in a session. Without it no mouse is offered. |
| `--host-pointer` | The Windows pointer over the window: `auto` (default) hides it while the program's INT 33h cursor is shown, `show` never hides it, `hide` always does. It changes at the pointer's next move. |
| `--speaker-volume` | PC speaker volume `0..100`, `0` silent; default `speaker_volume` from the configuration, else 5. |
| `--ansi` | Load the ANSI.SYS-style CON driver. |
| `--country` | Set the DOS country: 1 or 358 (Finland). |
| `--utc-offset` | Set the DOS clock's offset from UTC, in seconds. The default is the host's time zone. |

## Keys

The console turns each host key into the word the IBM AT BIOS stores, with
the scan code in the high byte and ASCII in the low byte. Ctrl, Alt and
Shift give the BIOS variants of F1-F12, the arrows, the editing block, Tab,
Enter and Backspace. The word goes into the BIOS keyboard ring. When the
program owns INT 09h, the key arrives through IRQ1 instead, as it does with
`--dos-session`.

Line editing and history therefore belong to the program itself. FreeCOM's
arrow keys, command recall and Ctrl+C work as they do on a PC. The headless
Tk terminal's COM1 route (issue #178) dropped these keys; this route keeps
them.

Letters outside ASCII are entered as the host layout types them (for
example ä, ö and å) and stored in code page 437. Ä, Ö and Å take the scan
codes of the keys they sit on in the Nordic layouts. AltGr characters
(@, \, |, {, [, ], } and $ on a Finnish keyboard) type plainly in both front
ends; Windows reports AltGr as Ctrl+Alt with the character, and that is not
taken as a Ctrl or Alt combination, so AltGr+9 gives ] rather than ending a
terminal session.

| Front end | Ctrl+Break | Ending the console |
|---|---|---|
| Terminal | On Windows, Ctrl+Break. In a Unix terminal, Ctrl+C reaches DOS as 2E03h; there is no separate Break key. | Ctrl+] |
| Window | Ctrl+Break | Close the window. Alt+F4 also closes it. |

## Screen

| Front end | Text modes | Graphics modes |
|---|---|---|
| `--tty` | Each changed row is redrawn on the terminal's alternate screen, in the 16 CGA colours, with the IBM PC glyphs (CP437 and the symbols for 00h-1Fh). The cursor follows the BIOS cursor and hides when the BIOS cursor is off. | On Windows a window opens. It shows the same frames as `--window` and also takes keys. On Linux/Termux the existing CGA/EGA/VGA framebuffer is sampled into ANSI true-colour Unicode half-blocks (`▀`) in the terminal; keyboard input remains available there. This is a reduced-resolution preview, not a separate X11 window. |
| `--window` | The page is drawn as a VGA shows it: 9x16 cells, 720x400 for 80 columns (40 columns are drawn 360 wide and doubled), with Dine's own 8x16 font (src/dos_font8x16.txt). The ninth column is the background, except that C0h-DFh repeat their eighth column so lines join. The window opens at the largest whole-number size that fits the desktop (1440x800 on a 1920x1080 screen); `DINE_WINDOW_SCALE=1`..`8` chooses it. A resized window keeps whole-number scaling with black borders, unless that would use less than 85% of the room; then the picture fills it with its proportions kept (a 640x480 graphics mode in a 1440x800 window is drawn 1066x800). The BIOS cursor blinks at its shape; the BIOS default 6-7 becomes lines 13-14, as the VGA BIOS scales it. | Drawn from the same scanout as the GDI preview: CGA 4/5/6, EGA/VGA 0Dh/0Eh/10h/12h and 13h. |

Mouse motion and the three buttons scale to the program's virtual
coordinates over the picture.

## Copy and paste (window)

In the window a left drag selects character cells in reading order, shown
inverted, and Ctrl+Shift+C copies the selection (the whole screen without
one) to the Windows clipboard as Unicode text, lines ending in CR LF and
without trailing blanks. Ctrl+Shift+V or Shift+Insert types the clipboard:
line ends become Enter, characters outside code page 437 are left out.
A key clears the selection. Plain Ctrl+C and Ctrl+V still reach DOS.
Right-click opens the host menu when the guest mouse is inactive;
Shift+right-click or Ctrl+Shift+M opens it even with a DOS mouse driver.
The menu includes clipboard commands and drive/media controls; see
[multi-disk installation media](DOS_INSTALL_MEDIA.md).

### Live guest diagnostics (Windows host menu)

If a DOS program shows a black screen or stops responding but the Dine host
menu still opens, use **Diagnostics > Save live guest snapshot**. The menu is
available with right-click when the guest mouse is inactive, or with
Shift+right-click or Ctrl+Shift+M. It does not require the guest to exit or
raise an exception.

Each capture appends one single-line JSON object to
`PILdos-snapshots.jsonl` **in the emulator's current working directory**
(usually the repository root if PILdos.vbs was started there). The record
contains current CS:IP and instruction bytes, CPU registers, step count,
original executable name, best-effort active DOS executable path read
from the current PSP environment, PSP/EXEC nesting, VGA BIOS mode and
sequencer/graphics-controller registers, BIOS keyboard ring pointers, IRQ1
keyboard queue, and host keystrokes awaiting delivery. The initial
executable name may still be COMMAND.COM when it launched a DOS child;
active path lookup is bounded and may report null if guest data is absent.

The **first capture enables a 32-entry pre-dispatch history ring** if history
was disabled. Give the DOS program a moment to run, then capture again.
The subsequent record includes the 16 most recent pre-dispatch addresses
and their bytes; repeated addresses help identify a polling loop. An
existing `PIL_DOS_HISTORY` setting is preserved. While dispatch
history is enabled, native blocks are bypassed, so execution can slow down.
Use **Diagnostics > Disable instruction history** when finished.

From PowerShell after taking two samples:

```powershell
Get-Content .\PILdos-snapshots.jsonl -Tail 2
```

This is a separate file from `PILdos-stops.jsonl`; an unchanged
stop log does not mean the guest is idle. Snapshots are read-only with
respect to guest CPU, keyboard and VGA state, apart from enabling or
disabling the host-side tracing facility.

While the program has the mouse driver active (it called INT 33h AX=0000h
with `--mouse`), the mouse belongs to the program; hold Shift to select.
With `--host-pointer auto` the Windows arrow disappears over the window
while the driver's cursor is shown (AX=0001h), so the program's arrow is
the only one; a program that hides the driver's cursor and draws its own
needs `--host-pointer hide` for the same effect.

Graphics modes work too: each cell is matched against the glyphs the BIOS
draws it with (the 8x8 font, or the 8x14/8x16 font of modes 10h/12h), so
text the program wrote through the BIOS copies as text. Picture cells and
cells of one colour copy as spaces. In a terminal (`--tty`) the terminal's
own selection does this.

## Termux graphics preview

Build the Rust frontend and start it in a Termux terminal with true-colour Unicode support:

```sh
cargo build --release --locked
./target/release/pil --dos-text
```

When a DOS program changes to a supported graphics mode (CGA 4/5/6, EGA/VGA planar, or VGA 13h), the framebuffer appears as coloured half-block characters fitted to the current terminal dimensions. The final terminal column and row are reserved to avoid wrapping. The renderer does not need Termux:X11 and does not modify guest framebuffer contents. When the program restores text mode the DOS text page is drawn again. Ctrl+] still quits. `DISPLAY`/Termux:X11 by themselves do not open a PILdos graphics window; a separate Linux X11 frontend is not implemented by this change.

For a clearer picture, use Termux's landscape orientation and reduce terminal font size. This is a preview: scaling, aspect ratio, and 24-bit ANSI colour support depend on the host terminal.

## PC speaker

The console plays what the program sends to the speaker: the tone of PIT
channel 2 for as long as port 61h bits 0 and 1 let it through, so `SOUND`,
`PLAY` and `BEEP` in GW-BASIC, game music and benchmark ticks sound at
their own pitch and length. The speaker signal (port 61h bit 1 AND channel
2's output; square wave in mode 3, a one-clock pulse in mode 2, low until
the count runs out in mode 0, high with the gate low) is recorded with the
PIT clock of each change and synthesised at 22,050 Hz about 50 ms behind
the program, averaged over each sample and through a high-pass filter as a
speaker cone moves: a lone switch-on is a click, a square wave a tone. A
sound shorter than 1 ms after at least 10 ms of silence is held for 1 ms,
so ticks that a fast host makes a microsecond long are still heard.

BEL (07h) printed to the console (DOS output, INT 29h, BIOS teletype)
beeps as the IBM AT BIOS does: 896 Hz for 31/64 s; several BELs follow one
another (at most 2 s waiting).

Windows plays the sound through winmm waveOut. Linux and Termux pipe raw
samples to `aplay`, else `pacat` (PulseAudio), whichever is on PATH;
without either nothing sounds. The output opens at the first sound and
closes after a second of silence. `speaker_volume` in pildos.conf (the
setup screen's "Speaker volume") or `--speaker-volume` sets the level
(0 starts no sound at all). The Tk terminal (`--python`) still plays one
click per switch-on.

## Not done yet

- No Sound Blaster or AdLib (ports 220h-22Fh and 388h stop as not
  modelled); PC speaker only.
- No drive menu or key log.
- A program's own text-mode font is not used: plane 2 and AX=1100h are
  ignored, and the console draws text modes with Dine's 8x16 font.
  Graphics modes draw what the program's INT 1Fh/43h tables hold.
- The blink attribute bit is not shown as blinking.
- Bytes the program sends to COM1 are dropped.

## Checked

Local Linux:

- `pil --dos-console --tty` in a pseudo-terminal, with FreeCOM from
  FD14LITE.img and programs/dos mounted as D:.
  - Up recalled earlier commands, and Left and an insert edited one.
  - MANDVGA switched to mode 12h, a key returned to the D:\> prompt, and
    Ctrl+] ended the console.
- tests/unit/host_console.rs has 7 tests:
  - the BIOS word tables;
  - terminal byte sequences;
  - ANSI redraw;
  - the font raster and cursor;
  - INT 16h receiving console keys;
  - an EXEC child's mode 12h frame (issue #177) and the parent's return to
    text mode.

The Windows front ends type-check for x86_64-pc-windows-gnu. Neither has
been run on Windows yet.
