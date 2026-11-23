# Booting a disk image

`pil` can start the PC from a disk image, without Dine's own DOS: the BIOS
reads the boot sector to 0000:7C00 and runs it, and from then on only BIOS
services answer INT n. Whatever the disk carries (a DOS kernel, a boot loader
and Linux) runs as on a PC.

```powershell
pil --dos-console --boot images/mcc-interim/boot
```
```powershell
pil --boot-probe images/mcc-interim/boot --steps 30000000 --keys "\r"
```

- `--dos-console --boot IMAGE` (or `--boot-hd IMAGE` for a fixed-disk
  image) opens the usual console: the Dine window on Windows, the terminal
  with `--tty`.
- `--boot-probe IMAGE [--hd] [--steps N] [--keys TEXT] [--keys-after N]`
  runs headless and prints the text screen and why the run stopped. `--keys`
  is typed when the program first waits for a key, or after `--keys-after`
  steps (default 5,000,000) for a loader that polls the keyboard. `\r` is
  Enter.

Floppy images are recognised by size (160K-2.88M PC formats); a fixed-disk
image is read as 16 heads and 63 sectors per track. Writes stay in memory;
the image file is never changed. Images are local files and stay out of the
repository (`images/` is git-ignored).

The BIOS side: INT 13h 00h-04h, 08h, 15h-18h on drives 00h, 01h and 80h
(status at 0040:0041 and 0040:0074, the 64 KiB DMA limit on diskettes), the
diskette parameter table at F000:EFC7 behind INT 1Eh, CMOS register 10h with
the diskette types, the equipment word, INT 19h. XMS and upper memory belong
to Dine's DOS and are off; CPU exceptions and IRQ 8-15 vectors point at a
bare IRET, DOS vectors are 0000:0000.

Hardware an operating system drives itself, beyond the BIOS:
- **Diskette controller** (3F2h-3F7h, uPD765/82077 commands: specify,
  sense drive status, recalibrate, sense interrupt, seek, read/write data
  with multi-track, read ID, version) with **DMA channel 2** and **IRQ6**.
  Only a booted machine has it.
- **Keyboard commands** through port 60h (ACK FAh, reset FAh AAh, echo,
  identify, FEh for unknown commands), for Dine's DOS too.
- **Absent devices**: COM2-COM4 (and COM1 without a session) read FFh; in a
  booted machine every port no known device answers is an open bus (FFh),
  as ISA probes expect. Printer ports read 80h, so a printer probe still
  finds three ports (Linux: "lp0 exists").
- **Paging** with a TLB.

Status: docs/development/LINUX_BOOT_PLAN.md.
