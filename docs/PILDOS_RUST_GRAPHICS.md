# PILdos native terminal with Rust CGA/VGA display

The DOS CPU, memory, VGA/CGA hardware emulation, palette state and
BIOS software graphics fonts already belong to Rust. The Python
`dos_terminal.py` frontend can now be used as a **terminal/COM1 bridge**
without receiving and unpacking the guest's graphics framebuffer.

This is a deliberately optional migration. The existing Tk PILdos
GUI and Tk headless graphics modes remain available and unchanged by
default.

## Diagnosing a missing Win32 window (opt-in, Windows)

If `MANDVGA.COM` renders when launched directly but **not** after starting
FreeCOM and typing `MANDVGA`, capture the native display lifecycle before
making changes to VGA or BIOS input semantics.

Set `DINE_VIDEO_LOG` in the same PowerShell window that starts PILdos:

```powershell
cargo build --release --locked
$env:DINE_VIDEO_LOG = (Join-Path (Get-Location) 'target\dine-video.log')
Remove-Item $env:DINE_VIDEO_LOG -ErrorAction SilentlyContinue
python tools/dos_terminal.py --headless --rust-graphics --mouse --fpu 80387 --engine .\target\release\pil.exe --mount DriveD images/FD14LITE.img::COMMAND.COM
```

At the FreeCOM prompt, enter `D:` and `MANDVGA`. **Do not press the
exit key until you have inspected the log.** In another PowerShell window,
run `Get-Content target\dine-video.log -Tail 50`, or inspect the file
after closing PILdos. To disable the diagnostic, run
`Remove-Item Env:DINE_VIDEO_LOG` before starting a fresh session.

The log records only transitions and one initial paint, not frame pixels,
DOS keystrokes, full command lines, or register dumps:

- `DOS session started`: whether `DINE_VIDEO=window` enabled the renderer.
- `BDA video mode changed`: whether the DOS machine entered mode 12h.
- `scanout absent` or `scanout ready`: whether the Rust VGA framebuffer
  converter supplied an RGB frame.
- `spawning Win32 video worker` and `Win32 video worker entered`:
  thread startup, or an explicit spawn failure.
- `RegisterClassW` and `CreateWindowExW`: success or Windows
  `GetLastError` codes on failure.
- `ShowWindow`: HWND visibility and window bounds. The current nonactivating
  window may appear behind the terminal. `previous_visible=0` is normal:
  it reports the *previous*, not the new, visibility state.
- `WM_PAINT first GDI result`: whether the graphics API was reached.
  A nonpositive result should be investigated, as should a window reported
  visible outside the desktop area.
- `WM_DESTROY` and `Win32 video owner dropped`: shutdown ordering.

The file is append-only across launches unless you delete it first.
It is written **directly by `pil.exe`**, not to the JSON session
stdout or to the pipe-captured stderr. This avoids losing Win32 errors
while the FreeCOM session stays alive. Logging is disabled completely
unless `DINE_VIDEO_LOG` is set. Compare a *direct MANDVGA* launch and
the *FreeCOM child launch* using the same release binary and log setting.

Related reports: [#177](https://github.com/pyTony/PIL-Processor-Instruction-Language/issues/177)
(native window missing after FreeCOM) and
[#178](https://github.com/pyTony/PIL-Processor-Instruction-Language/issues/178)
(COM1 arrow-key editing/history, separately tracked).

## Windows trial

Build the regular (interpreted) PIL engine from this branch:

```powershell
cargo build --release --locked
python tools/dos_terminal.py --headless --rust-graphics --engine .\target\release\pil.exe programs/dos/MANDVGA.COM
```

The native Win32/GDI window belongs to `pil.exe`, not Python. It
presents graphics from the **same in-memory DOS machine** that executes
the program. The Python process remains a terminal for COM1 and a
session protocol client. The real-mode DOS program still executes
through the PIL interpreter unless you specifically built a shadow
engine. Rust has not yet been refactored into a separate shared core
crate for the CLI.

For a FreeDOS shell and host folder:

```powershell
python tools/dos_terminal.py --headless --rust-graphics --engine .\target\release\pil.exe --mount .\DriveD images/FD14LITE.img::COMMAND.COM
```

The session keeps `--dos-session` protocol version 1, DOS mounts,
country selection, command tails, serial input, and FPU options.

### Responsibilities

| Function | Owner with `--headless --rust-graphics` |
| --- | --- |
| DOS/PIL CPU, RAM, services, FPU | Rust `pil.exe` / Dine code |
| Graphics-mode BIOS text | Existing Rust `dos_graphics_text.rs` draws pixels in VGA/CGA memory |
| CGA 320x200 4-colour, 640x200 2-colour | Rust: B8000h even/odd banks, CGA 3D9h palette and mode-control |
| VGA mode 13h | Rust: DAC and 320x200 framebuffer |
| Planar VGA 0Dh/0Eh/10h/12h | Rust: four planes and palette |
| Desktop display | Rust Win32/GDI thread with 4:3 presentation; no SDL dependency yet |
| Terminal keyboard and DOS console | Python COM1 bridge, using the launching terminal |
| Interactive fullscreen keyboard/mouse | Not migrated; keep Tk GUI for programs requiring it |

The native window does not take focus. In CGA/VGA graphics mode,
terminal keys reach BIOS INT 16h; other modes retain COM1 input. Programs that update only the
**text-mode VGA screen buffer B8000h**, rather than producing serial
console output, do **not** yet have a complete terminal-mode rendering
replacement. For editors/interactive full-screen text applications,
keep the normal PILdos Tk frontend until a text-cell-to-ANSI terminal
presenter and richer input mapping are available.

On Windows, the host display uses the source framebuffer and the live
CGA/VGA palette. The CGA/VGA bytes are no longer copied into session
JSON as multi-megabyte hex strings. The protocol provides the lightweight
`native_graphics` boolean instead. Text cells and other session
metadata remain available; existing non-native consumers still receive
their original graphics snapshot fields.

## Smoke tests

```powershell
cargo test --locked desktop_scanout_tests --bin pil
python -m unittest discover -s tests/gui -p test_dos_terminal.py
python tools/dos_terminal.py --headless --rust-graphics --engine .\target\release\pil.exe programs/dos/MANDVGA.COM
```

Verify progressive MANDVGA graphics and manual key input in the launching
terminal. Closing the native window ends the session without reporting a
false DOS program exit. No Python Tk window should appear.
Compare CGA palette and mode 5 against normal PILdos; verify mode 6
monochrome and mode 13 DAC updates. Compare the elapsed time with and
without native video under the same CPU workload; do not conflate CPU
execution improvements with reduced Python framebuffer serialization.

Original branch on 2026-11-13: 49 Python tests and four Rust scanout
tests passed; Windows release build succeeded; Mandelbrot rendered; automatic
Enter completed in 9.66 seconds. Verify the new manual keyboard and window
close paths on Windows after this merge. Keep the PR draft meanwhile.
