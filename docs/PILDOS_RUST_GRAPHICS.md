# PILdos native terminal with Rust CGA/VGA display

The DOS CPU, memory, VGA/CGA hardware emulation, palette state and
BIOS software graphics fonts already belong to Rust. The Python
\`dos_terminal.py\` frontend can now be used as a **terminal/COM1 bridge**
without receiving and unpacking the guest's graphics framebuffer.

This is a deliberately optional migration. The existing Tk PILdos
GUI and Tk headless graphics modes remain available and unchanged by
default.

## Windows trial

Build the regular (interpreted) PIL engine from this branch:

\`\`\`powershell
cargo build --release --locked
python tools/dos_terminal.py --headless --rust-graphics --engine .\target\release\pil.exe programs/dos/MANDVGA.COM
\`\`\`

The native Win32/GDI window belongs to \`pil.exe\`, not Python. It
presents graphics from the **same in-memory DOS machine** that executes
the program. The Python process remains a terminal for COM1 and a
session protocol client. The real-mode DOS program still executes
through the PIL interpreter unless you specifically built a shadow
engine. Rust has not yet been refactored into a separate shared core
crate for the CLI.

For a FreeDOS shell and host folder:

\`\`\`powershell
python tools/dos_terminal.py --headless --rust-graphics --engine .\target\release\pil.exe --mount .\DriveD images/FD14LITE.img::COMMAND.COM
\`\`\`

The session keeps \`--dos-session\` protocol version 1, DOS mounts,
country selection, command tails, serial input, and FPU options.

### Responsibilities

| Function | Owner with \`--headless --rust-graphics\` |
| --- | --- |
| DOS/PIL CPU, RAM, services, FPU | Rust \`pil.exe\` / Dine code |
| Graphics-mode BIOS text | Existing Rust \`dos_graphics_text.rs\` draws pixels in VGA/CGA memory |
| CGA 320x200 4-colour, 640x200 2-colour | Rust: B8000h even/odd banks, CGA 3D9h palette and mode-control |
| VGA mode 13h | Rust: DAC and 320x200 framebuffer |
| Planar VGA 0Dh/0Eh/10h/12h | Rust: four planes and palette |
| Desktop display | Rust Win32/GDI thread with 4:3 presentation; no SDL dependency yet |
| Terminal keyboard and DOS console | Python COM1 bridge, using the launching terminal |
| Interactive fullscreen keyboard/mouse | Not migrated; keep Tk GUI for programs requiring it |

The native window does not take focus. In this terminal mode, serial
console input reaches DOS via COM1. Programs that update only the
**text-mode VGA screen buffer B8000h**, rather than producing serial
console output, do **not** yet have a complete terminal-mode rendering
replacement. For editors/interactive full-screen text applications,
keep the normal PILdos Tk frontend until a text-cell-to-ANSI terminal
presenter and richer input mapping are available.

On Windows, the host display uses the source framebuffer and the live
CGA/VGA palette. The CGA/VGA bytes are no longer copied into session
JSON as multi-megabyte hex strings. The protocol provides the lightweight
\`native_graphics\` boolean instead. Text cells and other session
metadata remain available; existing non-native consumers still receive
their original graphics snapshot fields.

## Smoke tests

\`\`\`powershell
cargo test --locked desktop_scanout_tests --bin pil
python -m unittest discover -s tests/gui -p test_dos_terminal.py
python tools/dos_terminal.py --headless --rust-graphics --engine .\target\release\pil.exe programs/dos/MANDVGA.COM
\`\`\`

Verify that MANDVGA renders progressively and displays its software
font caption, Enter is received from the terminal, no Python Tk
window is created, and the program exits without a stuck GDI thread.
Compare CGA palette and mode 5 against normal PILdos; verify mode 6
monochrome and mode 13 DAC updates. Compare the elapsed time with and
without native video under the same CPU workload; do not conflate CPU
execution improvements with reduced Python framebuffer serialization.

The native renderer has been committed but has **not yet been compiled
or exercised on Windows from this conversation**. Keep the dependent
PR as draft until the full regression suite and Windows tests pass.
