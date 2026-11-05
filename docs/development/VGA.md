# VGA device module

`src/dos_vga.rs` owns the color-text CRTC register state. The DOS port adapter
routes byte accesses and index/data word accesses at 3D4h to it. Port 3D5h is
the data register. Word reads combine index in AL and data in AH; word writes
send AL to the index then AH to data. Failed word operations leave state intact.

Cursor shape, cursor address and display start are writable. The text frontend
reads the selected display origin with 32 KiB wrapping. BIOS console changes
synchronize the corresponding hardware registers. CRTC timing registers start
with color-text defaults; locked writes to 0..7 are ignored (overflow bit 4
remains writable). Unlocked timing/mode changes stop explicitly. Extended
register reads return FFh; unsupported extended writes stop.

This increment does not implement planar graphics, VGA palettes, sequencer,
attribute/graphics controllers, monochrome decoding or hardware scan timing.
It does not yet advertise VGA BIOS capabilities. Existing status-port polling
remains a deterministic approximation. Landmark must be rerun to determine its
next requirement; a full successful benchmark is not claimed.
