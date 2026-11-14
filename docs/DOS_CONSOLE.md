# DOS console in Rust: `pil --dos-console`

`pil --dos-console` runs a DOS session in the `pil` process and is its
keyboard and screen. It needs no Python and no Tk. It uses the same session
as `--dos-session`: the same drives, `--config`, `--mount`, `--args`, `--fpu`
and stop log. Nothing in it depends on which program runs.

```text
pil --dos-console [--tty|--window] [--mouse] [--ansi] [--country 1|358]
                  [--utc-offset SECONDS] [--config FILE] [--mount HOST_FOLDER]
                  [--args COMMAND_LINE] [--fpu none|80387] [PROGRAM]
```

PROGRAM is a .COM or .EXE file, or `IMAGE.img::INNER_PATH`, as in
`--dos-session`. Without it, the configuration's `program` and `args` start
(docs/DOS_CONFIG.md). The configuration's `fpu`, `mouse`, `ansi` and
`utc_offset` apply in both cases unless an option overrides them. For a
setup screen that edits the configuration and starts it, see `pil --pildos`
in [DOS_CONFIG.md](DOS_CONFIG.md).

| Option | Effect |
|---|---|
| `--tty` | Use the host terminal. This is the default except on Windows. |
| `--window` | Use a Win32 window. This is the default on Windows, and it works only there. |
| `--mouse` | Install the INT 33h driver, like `mouse on` in a session. Without it no mouse is offered. |
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
codes of the keys they sit on in the Nordic layouts.

| Front end | Ctrl+Break | Ending the console |
|---|---|---|
| Terminal | On Windows, Ctrl+Break. In a Unix terminal, Ctrl+C reaches DOS as 2E03h; there is no separate Break key. | Ctrl+] |
| Window | Ctrl+Break | Close the window. Alt+F4 also closes it. |

## Screen

| Front end | Text modes | Graphics modes |
|---|---|---|
| `--tty` | Each changed row is redrawn on the terminal's alternate screen, in the 16 CGA colours, with the IBM PC glyphs (CP437 and the symbols for 00h-1Fh). The cursor follows the BIOS cursor and hides when the BIOS cursor is off. | On Windows a window opens. It shows the same frames as `--window` and also takes keys. Elsewhere the terminal shows a one-line note; keys still reach the program. |
| `--window` | The page is drawn with the 8x8 ROM font, each line twice (640x400 for 80 columns), scaled to the window. The BIOS cursor blinks at its shape. | Drawn from the same scanout as the GDI preview: CGA 4/5/6, EGA/VGA 0Dh/0Eh/10h/12h and 13h. |

Mouse motion and the three buttons scale to the program's virtual
coordinates over the whole client area.

## Not done yet

- No PC speaker sound. The Tk terminal plays it.
- No drive menu, copy and paste, or key log.
- A program's own font is not used: plane 2, INT 1Fh tables and AX=1100h
  are ignored. The console draws with the ROM font.
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
