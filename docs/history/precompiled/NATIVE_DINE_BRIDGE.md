> **Historical (removed 13 November 2026).** This page describes tools that
> built an executable for one DOS program (shadow `pil-<program>`, standalone
> `<PROGRAM>.exe`, `Dine_<program>` launchers). They were removed: PIL and Dine
> are program-independent libraries, used the same everywhere. The commands
> below no longer exist. What was kept for later integration is listed in
> [BLOCK_COMPILER.md](../../development/BLOCK_COMPILER.md); measurements here
> remain as recorded.

# Native MANDVGA + the Dine graphics presenter (experimental)

This is the first **compiled execution ↔ reusable Dine video** connection,
stacked on standalone native MANDVGA PR #153. The compiled executable executes
generated Rust blocks with **no guest x86 decoder or PIL interpreter**. The same
guest-independent Win32/GDI window presenter used by Dine now receives host
RGB scanout derived directly from its compiled runtime's existing four VGA
planes. The only shared UI code is the window thread, not a second CPU.

The implementation extracts `src/dine_window.rs` from the existing Dine
presenter and embeds that source into native builds alongside a small
`src/dine_native_video.rs` adapter. This is **source sharing, not a finished
stable Dine DLL service ABI**. Files, general DOS services, DOS EXEC selection,
live VGA DAC programming and automatic interpreter fallback in the standalone
binary remain future work.

## Run the first live-native graphics test on Windows

From this feature branch:

```powershell
cargo build --release --locked
python tools/build_dos_native.py programs/dos/MANDVGA.COM `
  --engine .\target\release\pil.exe `
  --output-dir .\target\native-video-test --force

# Do NOT inject a BIOS key: terminal Enter is delivered at the guest wait.
Remove-Item Env:PIL_DOS_BIOS_KEY -ErrorAction SilentlyContinue
$env:DINE_VIDEO = "window"
.\target\native-video-test\MANDVGA.COM.exe
Remove-Item Env:DINE_VIDEO
```

Expected: a **separate non-activating** 640×480 VGA window showing the
Mandelbrot image progressively, with the DOS caption drawn using the same
ROM font and planar memory semantics as PR #153. At the BIOS `INT 16h`
wait, the native program prompts in the terminal for Enter. Press Enter to
restore text mode and exit; the VGA window closes before executable exit.
The host terminal keeps keyboard focus.

The native snapshot is at most once per 50 ms between generated blocks,
and once immediately before the BIOS key wait. It is a *read-only* scanout:
no guest pixel, latch, machine or CPU state is mutated by presentation.
The native program uses the default EGA mode-12h palette and admits no live
DAC updates. Rendering modes beyond 12h is not yet claimed.

### Preserve the deterministic, headless reference

```powershell
$env:DINE_VIDEO = "off"
$env:PIL_DOS_BIOS_KEY = "1C0D"
python tools/verify_mandel_vga_native.py --engine .\target\release\pil.exe
Remove-Item Env:DINE_VIDEO
Remove-Item Env:PIL_DOS_BIOS_KEY
```

This path must still verify all 296,960 reference pixels and the caption.
The graphics window is **opt-in**, so CI never opens a desktop window by
accident. On non-Windows hosts, requesting `DINE_VIDEO=window` currently
returns an explicit unsupported-host error; headless compilation and
verification remain supported.

## Try LINES using existing hybrid compiled blocks

LINES is not part of the currently checked-in `programs/dos` fixtures.
If you have a DOS LINES.COM or LINES.EXE, use the **hybrid** engine rather
than the strictly static standalone compiler until its whole control-flow
and DOS services have been proven:

```powershell
python tools/build_dos_shadow.py .\DriveD\LINES.EXE `
  --engine .\target\release\pil.exe `
  --steps 2000000 --sites 512 --verify `
  --output-dir .\target\hybrid-lines

python tools/dos_terminal.py `
  --engine .\target\hybrid-lines\pil-lines.exe `
  --mount .\DriveD .\DriveD\LINES.EXE
```

Replace `LINES.EXE` with the actual guest filename, including extension.
The shadow profile produces a **program-hash-matched, guarded native block
backend with Dine interpreter fallback** for unobserved branches, changed
code, or unsupported instruction sites. Running via the Python frontend
still provides keyboard, DOS sessions and graphical display. This example
is a suggested smoke test, **not a verified LINES result**. If profiling
cannot reach some graphics paths, supply a representative `--keys` file or
expand profiling steps; do not mark program support as proven without a
complete execution.

The later work is a stable service ABI for compiled blocks, verified
automatic native-artifact selection at DOS `EXEC`, and unified file/handle
semantics. Neither this PR nor the shadow compiler automatically replaces
programs launched by FreeCOM.

## Regression commands

```powershell
cargo test --locked
cargo build --release --locked
cargo build --release --lib --features dine-library --locked
python tools/verify_dos_standalone.py --engine .\target\release\pil.exe
python tools/verify_mandel_vga_native.py --engine .\target\release\pil.exe
```

The Windows display remains a manual acceptance gate because CI cannot
verify that GDI visibly paints a window without focus theft. Until the
Windows executable and both CI jobs have passed, keep this PR in draft.
