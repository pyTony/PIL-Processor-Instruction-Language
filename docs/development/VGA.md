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

Planar graphics, the sequencer, graphics controller and attribute controller
are described in "EGA/VGA 16-colour planar modes" below. Monochrome text
modes and hardware scan timing are not implemented. Existing status-port polling
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
Graphics-mode text is described below. A working
BASIC interpreter may need further DOS or CPU features. CGA modes do not
imply full EGA/VGA register or planar graphics support.

`programs/dos/MANDCYC.COM` is the separate full-screen palette-cycling
mode-13h demo. Run `python tools/dos_terminal.py programs/dos/MANDCYC.COM`;
the first frame takes time and a key exits to text mode. It draws 160x100
calculated points as 2x2 blocks.

## Text in graphics modes

`src/dos_graphics_text.rs` draws BIOS and DOS text in modes 04h, 05h, 06h
and 13h as 8x8 character cells in the framebuffer (40x25, or 80x25 in mode
06h), as the IBM BIOS does. Glyphs 00h-7Fh come from the ROM font at
F000:FA6E; 80h-FFh from the table at the INT 1Fh vector. Dine's font is
original to this project: `src/dos_font8x8.txt` holds the 128 glyphs as
readable rows and is copied into the read-only BIOS area at start-up. INT 1Fh
starts at 0 (no GRAFTABL), and an upper-half character then stops the run
with that reason instead of drawing whatever the interrupt table holds.

- AH0Eh teletype: BL is the foreground colour, the rest of the cell is 0.
  BEL draws nothing; BS, CR and LF move the cursor; a line feed on row 24
  scrolls the whole screen up 8 scanlines and fills with colour 0. TAB keeps
  the text-mode policy (DOS expands it; BIOS AH0Eh refuses it).
- DOS console output (AH02h, AH09h, AH40h to CON) goes through the same
  teletype in colour 7, because FreeDOS INT 29h calls AH0Eh with BX=0007h.
  That is colour 3 in modes 4/5, 1 in mode 6 and 7 in mode 13h.
- AH09h/0Ah: CX copies of AL in colour BL from the cursor, which does not
  move. In the CGA modes bit 7 of BL XORs the glyph into the cell. A run past
  the last cell is refused before anything changes.
- AH08h: returns the character whose glyph matches the cell's non-zero
  pixels (searching 00h-7Fh, then the INT 1Fh table if installed), AL=0 if
  none, AH=0.
- AH06h/07h: the window is in character cells; vacated rows take BH.
- Only display page 0 exists in graphics modes. The BIOS data byte 0465h
  holds the CRT mode-control value (29h, 2Ah, 2Eh, 1Eh for modes 3-6).

Mode 05h is mode 04h with the CGA black-and-white bit. On an RGB monitor it
shows cyan, red and white, and the palette-select bit has no effect; the
terminal draws it that way. `programs/dos/MANDCGA.COM` uses mode 04h and
prints its caption through DOS, so it exercises the font, colour and cursor.

## EGA/VGA 16-colour planar modes and the VGA BIOS identity

Tony chose on 7 November 2026 to identify the video BIOS as a colour VGA
(it was CGA-class). INT 10h AX=1A00h returns AL=1Ah, BX=0008h; AH=12h
BL=10h returns BH=0 (colour), BL=3 (256 KiB), CX=0009h; the BIOS data area
holds 0485h=16 (9x16 text cells), 0487h=60h, 0488h=09h and 0489h=11h.

**Display memory.** `src/vga_planar.rs` holds four 64 KiB planes and is the
one memory-mapped device in `Ram` (`src/memory.rs`). While a planar mode is
set, guest bytes at A0000h-AFFFFh go to it; otherwise the window is ordinary
RAM, so mode 13h stays linear. Both execution backends embed the same code.
- Reads load the four latches, then return read mode 0 (Read Map Select) or
  read mode 1 (colour compare with Colour Don't Care).
- Writes use write modes 0-3, data rotate, the AND/OR/XOR function, the bit
  mask and the sequencer map mask.
- A latch load happens on the read itself, as on the hardware, even if the
  instruction later faults. Nothing else is staged.
- Chain-4, odd/even and shift-register modes are refused, with no change.
  Sequencer/graphics controller access outside the planar modes stops too.
- BIOS services reach the planes through a host-only alias at F0000000h,
  which real-mode guests cannot address.

**Ports.** 3C4h/3C5h (sequencer), 3CEh/3CFh (graphics controller), byte or
word OUT to the index port; 3C0h/3C1h (attribute controller, with the
index/data flip-flop that reading 3DAh resets). The 16 displayed colours go
through the attribute palette, colour plane enable, colour select and DAC.

**BIOS.** Modes 0Dh (320x200), 0Eh (640x200), 10h (640x350) and 12h
(640x480):
- Each mode set writes the BIOS data area, resets the registers and clears
  the planes, unless AL bit 7 asks to keep them.
- It loads the palette: EGA 64-colour DAC layout for 350/480 lines,
  CGA-compatible colours for 200 lines.
- AH=0Ch/0Dh draw and read pixels (bit 7 XORs). The text services draw 8-,
  14- or 16-line cells from the 8x8 font, repeating rows for the tall cells:
  14 lines show rows 0 and 7 once and the others twice.
- AH=10h handles palette registers (00h-03h, 07h-09h) and DAC entries and
  blocks (10h, 12h, 15h, 17h-19h). Colour paging and grey-scale summing stop.
- AX=1130h answers BH=0/1 (INT 1Fh/43h vectors) and BH=3 (the 8x8 ROM font).
  Other fonts and font loading stop, because no 8x14/8x16 tables exist.
- AH=1Bh/1Ch and VBE return unchanged.

`programs/dos/MANDVGA.COM` draws a mode 12h Mandelbrot through write mode 2
and the bit mask; `tools/verify_mandel_vga.py` checks all four planes.
