# Booting a disk image

`pil` can start the PC from a disk image, without Dine's own DOS: the BIOS
reads the boot sector to 0000:7C00 and runs it, and from then on only BIOS
services answer INT n. Whatever the disk carries (a DOS kernel, a boot loader
and Linux) runs as on a PC.

```powershell
pil --dos-console --boot images/mcc-interim/boot
```
```powershell
pil --boot-probe images/mcc-interim/boot --steps 200000000 --keys "\r"
```

MCC Interim (Linux 0.99.10) reaches its install menu within 200 million
steps, about 11 s headless here.

- `--dos-console --boot IMAGE` (or `--boot-hd IMAGE` for a fixed-disk
  image) opens the usual console: the Dine window on Windows, the terminal
  with `--tty`. `--hd-image DISK` adds a fixed disk (drive 80h, `/dev/hda`)
  next to a `--boot` diskette.
- `--boot-probe IMAGE [--hd] [--hd-image DISK] [--steps N] [--keys TEXT]
  [--keys-after N] [--type STEPS:TEXT]...` runs headless and prints the
  text screen and why the run stopped. `--keys` is typed when the program
  first waits for a key, or after `--keys-after` steps (default 5,000,000)
  for a loader that polls the keyboard. Each `--type` is typed once the run
  has made STEPS steps (an operating system idles in its own loop, so the
  probe cannot see it wait). `\r` is Enter; capitals and shifted symbols
  are typed with Shift (US layout).
- `--boot-probe ... --change-floppy STEPS:IMAGE` (repeatable) puts another
  diskette of the same format into drive A: once the run has made STEPS
  steps. `--dos-console --boot IMAGE --floppy IMAGE2 [--floppy IMAGE3]...`
  gives the console the other diskettes: Ctrl+Shift+F1 changes to the next,
  Ctrl+Shift+F2 back to the previous. Either way the drive's change line
  goes active (FDC 3F7h bit 7 until the head steps; INT 13h AH=16h returns
  06h once), as when a hand changes the disk.
- `--rtc-year YEAR` (console and probe, 1980-2099) runs the real-time
  clock in that year from today's date and time. Linux 0.99 reads only the
  CMOS year's last two digits and takes `26` as 1926, a date before 1970,
  so its clock shows 23:59:59 of 31 December 1969; with `--rtc-year 1993`
  `date` gives today's date and time in 1993, as on a machine of its time.
- `pil --new-hd-image FILE MEGABYTES` creates an empty fixed-disk image of
  whole cylinders (16 heads, 63 sectors; 1-504 MB). It refuses an existing
  file.
- `--hd-folder DIR` (console and probe) adds a host folder as a fixed
  disk, the next one after the boot disk and `--hd-image`: drive 81h and
  `/dev/hdb` next to a hard disk, else 80h and `/dev/hda`. See "A host
  folder as a disk" below.

Floppy images are recognised by size (160K-2.88M PC formats); a fixed-disk
image is read as 16 heads and 63 sectors per track. A diskette's writes stay
in memory and its file is never changed. **A fixed disk's writes go back to
its file**, sector by sector, as a hard disk keeps them: keep a copy of an
image you want to return to. Images are local files and stay out of the
repository (`images/` is git-ignored).

Installing MCC Interim onto a hard disk (on the console; the same keys work
through `--boot-probe --type`):

```powershell
pil --new-hd-image images/mcc-interim/hd40.img 40
```
```powershell
pil --dos-console --boot images/mcc-interim/boot --hd-image images/mcc-interim/hd40.img
```

Enter at `boot:`, then in MCC's menu: `f` (fdisk on /dev/hda: `n`, `p`,
`1`, cylinders 1 to the last, `w`; answer `n` to the reboot question), `m`
(a Minix file system with long names on /dev/hda1), `i` (install Base onto
/dev/hda1, no swap, no other partition) and `y` to reboot. Booting the
diskette again with `rw root=301` at `boot:` mounts /dev/hda1 as `/`; `7`
skips the package floppies and `root` logs in. Add `--rtc-year 1993` to
the console command for a working clock.

MCC Interim 1.0+ (Linux 1.0.4, March 1994) comes as a boot disk and a
root disk, 1.2 MB each (`nocdboot` and `root` from the `images` folder of
the release, gunzipped; not the copies padded to 1.44 MB for a GoTek):

```powershell
pil --dos-console --boot images/mcc-1.0/nocdboot --floppy images/mcc-1.0/root --hd-image images/mcc-1.0/hd40.img --rtc-year 1994
```

Type `ramdisk` at `boot:`; when the kernel asks for the root disk, press
Ctrl+Shift+F1 and then Enter. The menu is the same as above; `m` offers
ext2. `i` installs Base onto /dev/hda1 and then asks about the keyboard
map, networking, LILO and a boot floppy (`n` to each) and the package
drive (`7`, none for now).

## A host folder as a disk

A booted system reads sectors, so `--hd-folder DIR` lays the folder out as
a real disk: a partition table with one primary FAT16 partition (type 04h;
06h when the folder needs more than 32 MB) holding the folder's files and
subdirectories, 8.3 names as Dine's `--mount` lists them (BASIS~N aliases
for long names, sorted by host name). The disk has 65 cylinders (32 MB)
or as many as the folder needs plus 16 MB free, up to 504 MB; at most
8,000 entries, 511 in the top folder, 16 MB a file and 128 MB in all.
Linux mounts it with its msdos file system:

```sh
mount -t msdos /dev/hdb1 /mnt
```

The disk is a temporary image file; the guest reads and writes it as any
other disk, and the host folder does not change while the machine runs.
**When the run ends** (the console closes, or the probe's budget is spent)
the files the guest created or changed are copied into the folder, and new
directories are made (new names in lower case, as Linux lists them).
Nothing in the folder is deleted or renamed: a file deleted on the disk
stays, a renamed one appears under its new name beside the old. A host file
that also changed on the host during the run is not overwritten; the run's
summary names it. Linux keeps writes in its buffers, so run `sync` or
`umount /mnt` before closing the console. If the FAT volume cannot be read
back, nothing is copied and the image file is kept and named.

```powershell
pil --dos-console --boot images/mcc-1.0/nocdboot --floppy images/mcc-1.0/root --hd-image images/mcc-1.0/hd40.img --hd-folder share --rtc-year 1994
```
```sh
pil --dos-console --boot images/mcc-1.0/nocdboot --floppy images/mcc-1.0/root --hd-image images/mcc-1.0/hd40.img --hd-folder share --rtc-year 1994
```

With a fixed disk 81h the BIOS also has its parameter table at F000:E411
behind INT 46h (Linux's setup copies it too), CMOS 12h low nibble Fh and
1Ah = 47, two fixed disks at 0040:0075 and in INT 13h AH=08h, and the AT
controller answers as the primary slave.

The BIOS side: INT 13h 00h-04h, 08h, 15h-18h on drives 00h, 01h and 80h
(status at 0040:0041 and 0040:0074, the 64 KiB DMA limit on diskettes), the
diskette parameter table at F000:EFC7 behind INT 1Eh, CMOS register 10h with
the diskette types, the equipment word, INT 19h. With a fixed disk: its
parameter table at F000:E401 behind INT 41h (cylinders, heads, sectors, as
Linux's setup copies it), CMOS 12h = F0h and 19h = 47 (drive C: present,
user-defined type), one fixed disk at 0040:0075. XMS and upper memory belong
to Dine's DOS and are off; CPU exceptions and IRQ 8-15 vectors point at a
bare IRET, DOS vectors are 0000:0000.

Hardware an operating system drives itself, beyond the BIOS:
- **Diskette controller** (3F2h-3F7h, uPD765/82077 commands: specify,
  sense drive status, recalibrate, sense interrupt, seek, read/write data
  with multi-track, read ID, version) with **DMA channel 2** and **IRQ6**.
  Only a booted machine has it.
- **Hard-disk controller** (1F0h-1F7h, 3F6h; WD1003/ATA-1 task file:
  recalibrate, read and write sectors, verify, seek, diagnostics,
  initialize drive parameters, identify; CHS or LBA; software reset and
  nIEN) with **IRQ14** through the slave PIC, for fixed disk 80h as the
  primary master and 81h as its slave. Only a booted machine with a fixed
  disk has it; the other ATA channels, and the slave without a disk 81h,
  stay empty. The two drives share one task file, status and error
  register (commands complete at once, so neither is busy while the other
  is addressed).
- **Keyboard commands** through port 60h (ACK FAh, reset FAh AAh, echo,
  identify, FEh for unknown commands), for Dine's DOS too. Controller
  commands ADh/AEh turn the keyboard interface off and on: while it is
  off, keys wait in the keyboard (Linux 1.0's keyboard interrupt does so).
- **Diskette change line** (3F7h bit 7, INT 13h AH=16h), see above.
- **Absent devices**: COM2-COM4 (and COM1 without a session) read FFh; in a
  booted machine every port no known device answers is an open bus (FFh,
  FFFFh for a word access), as ISA probes expect. So are the addresses the
  Sound Blaster does not decode (220h-223h, 227h, 22Bh, 22Dh), where
  Linux 1.0's network drivers probe. Printer ports read 80h, so a printer probe still
  finds three ports (Linux: "lp0 exists").
- **Paging** with a TLB. Protected-mode code, paged or not, runs in the
  same compiled blocks as real-mode code; a block's code guard follows the
  page mapping.

Status: docs/development/LINUX_BOOT_PLAN.md.
