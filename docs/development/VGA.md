# VGA device module

`src/dos_vga.rs` owns the color-text CRTC register state. The DOS port adapter
routes byte accesses and index/data word accesses at 3B4h/3D4h to it. Port 3B5h/3D5h is
the data register. Word reads combine index in AL and data in AH; word writes
send AL to the index then AH to data. Failed word operations leave state intact.

Cursor shape, cursor address and display start are writable. The text frontend
reads the selected display origin with 32 KiB wrapping. BIOS console changes
synchronize the corresponding hardware registers. CRTC timing registers start
with color-text defaults; locked writes to 0..7 are ignored (overflow bit 4
remains writable). Unlocked timing/mode changes stop explicitly. Extended
register reads return FFh; unsupported extended writes stop.

The device does not yet implement planar graphics, the sequencer,
attribute/graphics controllers, monochrome text modes or hardware scan timing.
It does not yet advertise VGA BIOS capabilities. Existing status-port polling
remains a deterministic approximation. Landmark must be rerun to determine its
next requirement; a full successful benchmark is not claimed.

## CRTC address selection and adapter probes

The miscellaneous output register (write 3C2h, read 3CCh) starts at 67h.
Bit 0 selects the color (3D4h/3D5h, status 3DAh) or monochrome-address
(3B4h/3B5h, status 3BAh) bank of the same CRTC. Inactive bank writes are
ignored; reads return FFh, including both bytes of a word read. This is the
profile's unconnected-bus response, not an additional MDA device. Changes to
other miscellaneous-output bits remain unsupported, with no partial update.
Changing I/O selection alone does not select a framebuffer or enable mode 7.

The BIOS data area at 0040:0063 now reports 3D4h for the color-text profile.
Setting BIOS mode 3 restores that value and color I/O selection. Direct hardware
I/O selection does not rewrite the BIOS data area, matching their separate roles.
The inactive status port reads FFh; active status timing remains synthetic.

Reference: [FreeVGA external registers](https://www.scs.stanford.edu/10wi-cs140/pintos/specs/freevga/vga/extreg.htm), I/O Address Select.

The Windows launcher forwards terminal options, for example:

```powershell
wscript.exe .\PILdos.vbs --ignore-unassigned-out
```

This flag only ignores unassigned OUT ports; recognized video devices retain explicit unsupported-operation stops.


## Mode 13h Mandelbrot demonstration

BIOS INT 10h mode 13h exposes a 320x200 byte-per-pixel framebuffer at
A000:0000. The DOS terminal resolves its pixel indices through the emulated
256-entry VGA DAC. Ports 3C6h–3C9h provide the pixel mask, palette indices and RGB
components, so changing the palette recolours pixels already on screen.
This remains a bounded profile: planar VGA, attribute-controller palette
mapping and INT 10h palette services are not yet implemented. Setting mode
13h clears 64,000 bytes unless bit 7 requests preservation. Mode 3 returns to text display.

Build and create the 128x128 DOS demonstration once:

```powershell
cargo build --release --locked
.\target\release\pil.exe --write-dos-mandel .\driveD\MANDEL.COM
```

Run `D:\MANDEL.COM` inside PILdos. It calculates each pixel in the emulated
CPU and writes it directly to the A000 framebuffer. Press any key when the
image is complete to restore text mode and return to DOS. The program file is
created only if the path does not already exist; no image file is written.
The existing `--mandelbrot` serial/file benchmark remains available.

## CGA graphics modes and IBM BASIC

BIOS modes 04h and 06h provide 320x200 four-colour and 640x200
two-colour graphics. Their framebuffer is the CGA interleaved layout at
B800:0000: even scanlines start at offset 0000h and odd scanlines at 2000h,
80 bytes per scanline. Direct guest video-memory writes and BIOS pixel
read/write services (INT 10h AH=0Ch/0Dh) use this same framebuffer.
Mode 13h also supports those BIOS pixel services. AH=0Bh controls the CGA
palette and port 3D9h changes the displayed CGA colour register.

The supplied BASIC cube targets SCREEN 9 (640x350 EGA), which is not yet
implemented; SCREEN 2 needs centre Y=100, a smaller field of view, no COLOR
statement and LINE colour 1. `programs/dos/CUBE2.BAS` contains that adaptation.
GW-BASIC 3.23 drew part of the cube in both deterministic- and host-clock
tests. Its INT 1Ch timer hook now runs in a bounded real-mode BIOS wrapper.
This is not yet a complete interactive BASIC compatibility claim.
Graphics-mode text and scrolling are not yet complete, and a working
BASIC interpreter may need further DOS or CPU features. CGA modes do not
imply full EGA/VGA register or planar graphics support.

`programs/dos/MANDCYC.COM` is the separate full-screen palette-cycling
mode-13h demo. Run `python tools/dos_terminal.py programs/dos/MANDCYC.COM`;
the first frame takes time and a key exits to text mode. It draws 160x100
calculated points as 2x2 blocks.
