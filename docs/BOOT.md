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
  [--keys-after N] [--type STEPS:TEXT]... [--guest-keymap us|uk|fi]` runs
  headless and prints the text screen and why the run stopped. `--keys` is typed when the program
  first waits for a key, or after `--keys-after` steps (default 5,000,000)
  for a loader that polls the keyboard. Each `--type` is typed once the run
  has made STEPS steps (an operating system idles in its own loop, so the
  probe cannot see it wait). `\r` is Enter; text is typed on the keys of
  the `--guest-keymap` layout (US unless given), with Shift and AltGr
  where it needs them.
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

## Installing MCC Interim 1.0+ (Linux 1.0.4) step by step

MCC Interim 1.0+ (Linux 1.0.4, March 1994) is the MCC release in the
oldlinux repository (github.com/youngd24/oldlinux, folder
`mcc-linux/MCC-1.0`). It comes as a boot disk and a root disk, 1.2 MB each:
`nocdboot.gz` and `root.gz` in its `images` folder, gunzipped. (The copies
in `mcc-linux/images/bootroot` are padded to 1.44 MB for a GoTek; use the
1.2 MB ones.) The images stay local, in the git-ignored `images` folder.

**1. Get the two diskettes.** From the checkout's folder, in PowerShell:

```powershell
New-Item -ItemType Directory -Force images\mcc-1.0 | Out-Null
$base = "https://github.com/youngd24/oldlinux/raw/main/mcc-linux/MCC-1.0/images"
foreach ($n in "nocdboot", "root") {
  Invoke-WebRequest "$base/$n.gz" -OutFile "images\mcc-1.0\$n.gz"
  $in = New-Object IO.Compression.GZipStream([IO.File]::OpenRead((Join-Path $PWD "images\mcc-1.0\$n.gz")), [IO.Compression.CompressionMode]::Decompress)
  $out = [IO.File]::Create((Join-Path $PWD "images\mcc-1.0\$n"))
  $in.CopyTo($out); $out.Close(); $in.Close()
}
Get-FileHash images\mcc-1.0\nocdboot, images\mcc-1.0\root
```

In sh:

```sh
mkdir -p images/mcc-1.0 && cd images/mcc-1.0
base=https://github.com/youngd24/oldlinux/raw/main/mcc-linux/MCC-1.0/images
curl -LO $base/nocdboot.gz && curl -LO $base/root.gz
gunzip -k nocdboot.gz root.gz && sha256sum nocdboot root && cd ../..
```

Both files are 1,228,800 bytes. SHA256: `nocdboot` 9ae65a42...b59048c,
`root` 0959510f...a3f9f.

**2. Make an empty 40 MB hard disk** (it refuses a file that exists):

```powershell
pil --new-hd-image images/mcc-1.0/hd40.img 40
```

**3. Start the boot disk with the root disk and the hard disk** (the same
command in sh; on Windows it opens the Dine window):

```powershell
pil --dos-console --boot images/mcc-1.0/nocdboot --floppy images/mcc-1.0/root --hd-image images/mcc-1.0/hd40.img --rtc-year 1994
```

`--rtc-year 1994` gives Linux 1.0 a clock it can read (without it the
prompt shows 23:59:59).

**4. Boot the installer.** At LILO's `boot:` type `ramdisk` and Enter. The
kernel loads the boot disk into a RAM disk and then asks for the root
disk: press **Ctrl+Shift+F1** (drive A: gets the next diskette, `root`),
then Enter. "VFS: Disk change detected" and the install menu follow
(`f`, `s`, `a`, `m`, `i`, `c`, `e`, `r`, `q` and a `?` prompt).

**5. Partition the disk: `f`.** The answers, each followed by Enter:

| Question | Answer |
|---|---|
| Run fdisk on (block device; default = /dev/hda) | Enter |
| Command (m for help) | `n` (new partition) |
| e extended / p primary | `p` |
| Partition number (1-4) | `1` |
| First cylinder | `1` |
| Last cylinder | `81` (the last one of a 40 MB image) |
| Command (m for help) | `w` (write and leave) |

The kernel prints `hda: hda1` and the menu comes back.

**6. Make the file system: `m`.**

| Question | Answer |
|---|---|
| WARNING: This will destroy any files on this partition. Do you want to continue | `y` |
| Run mkfs on (block device; default = /dev/hda1) | Enter |
| Create an ext2 file system | `y` |

**7. Install Base: `i`.**

| Question | Answer |
|---|---|
| This file system is on (block device; default = /dev/hda1) | Enter |
| Does this device contain an ext2 file system | `y` |
| Do you wish to have a swap device | `n` (see below) |
| Is there another partition you wish to mount? ... type 'n' | `n` |

Base is copied from the root disk and its CRCs checked ("Verifying
Base.crc", "Verifying base.crc"); this takes a few minutes. Then:

| Question | Answer |
|---|---|
| Do you wish to change the default keyboard map | `n` keeps MCC's UK map; `y` lets you pick one (Finnish is `fi-latin1`) |
| Do you wish to install networking software | `n` |
| Would you like LILO to install a boot program now? | `n` |
| Which drive will you use to install Linux packages | `7` (none for now) |
| Reboot the system now | close the window instead |

Answer `n` to the swap question: the installer's default swap device is
/dev/hda5, which this disk does not have, so `y` writes a swap line for a
partition that does not exist into /etc/fstab. Every boot then prints
"harddisk I/O error, dev 0305, sector 0" and "Unable to find swap-space
signature" twice (/etc/bcheckrc and /etc/rc both run `swapon -a`). Linux
refuses a read past a partition's end before it reaches the disk
controller, so a PC of the time printed the same; it does no harm. To
remove it, at the root prompt: `grep -v hda5 /etc/fstab > /tmp/f; cp
/tmp/f /etc/fstab`. A real swap partition would need a second partition
from fdisk (type 82, `t`) and the menu's `s` before `i`.

**8. Boot Linux from the hard disk.** The installed system is on
/dev/hda1, but there is no LILO on the hard disk, so the boot disk loads
the kernel:

```powershell
pil --dos-console --boot images/mcc-1.0/nocdboot --hd-image images/mcc-1.0/hd40.img --rtc-year 1994
```

At `boot:` type `ro root=301` and Enter (301 is /dev/hda1; the kernel
mounts it read-only, /etc/bcheckrc checks it and mounts it read-write).
At `linux login:` type `root` (no password). `uname -a`, `df`, `ls /`,
`free` and `date` answer.

The hard disk image keeps every write, as a disk does: copy
`hd40.img` after a step you want to come back to.

**Typing and pasting.** Keys typed in the window reach Linux as the
physical keys pressed, so Linux's own keymap decides the character, as on
a PC with that keyboard. A paste (right-click menu, Ctrl+Shift+V,
Shift+Insert) has only characters, so in a booted machine it is typed on
the keys of the layout given by `--guest-keymap us|uk|fi` (default `us`):
give the layout of the keymap Linux has loaded (`fi` after choosing
`fi-latin1`; `uk` for MCC's default map; for plain letters, digits and
`-` `/` `.` `,` `=` `;` US and UK agree). `--boot-probe ... --type` types
the same way.

### Scripted runs: install, gcc 2.5.8 and a timed Mandelbrot

`pil --boot-probe IMAGE --script FILE` runs a headless machine through a
script, one step per line (`#` starts a comment):

| Step | Meaning |
|---|---|
| `run N` | run N steps (`_` separators allowed) |
| `type TEXT` | type TEXT (`\r` or `\n` is Enter, `\s` a space, `\b` Backspace, `\e` Esc, `\\` a backslash) |
| `prompt TEXT` | wait until the screen changed after the last `type` and its last line ends with TEXT |
| `screen TEXT` | wait until TEXT is anywhere on the screen |
| `floppy IMAGE` | change the diskette (same format) |
| `show` | print the screen |
| `mark NAME` | print the steps and host seconds since the last mark |
| `shot FILE` | write the graphics screen as a P6 PPM (a text screen stops the script) |

The output streams as the script runs. A script ends at its last step,
at `--steps` or when the guest waits for a key no step will give.
`--script` replaces `--keys`, `--type` and `--change-floppy`.

`pil --new-tar-floppy IMAGE [--size 360|720|1200|1440|2880] FILE...`
writes a diskette image that holds one ustar archive with the files (by
their names, without folders). Linux reads it with
`cpio -imV -H ustar </dev/fd0`; that is how packages reach MCC before a
network or a host folder. The MCC 1.0+ packages are in
`mcc-linux/MCC-1.0/packages` of github.com/youngd24/oldlinux:
gcca.tgz (SHA256 caf7ac51...c7eb) and gccb.tgz (445e35d6...dffdf),
554,454 and 825,727 bytes, so each fits one 1.2 MB diskette.

From the checkout's root folder, with `nocdboot`, `root`, `gcca.tgz` and
`gccb.tgz` in images/mcc-1.0 (git-ignored), PowerShell:

```powershell
cargo build --release --locked
$pil = ".\target\release\pil.exe"
& $pil --new-tar-floppy images/mcc-1.0/gcca.flp images/mcc-1.0/gcca.tgz programs/linux/mandel.c
& $pil --new-tar-floppy images/mcc-1.0/gccb.flp images/mcc-1.0/gccb.tgz
& $pil --new-hd-image images/mcc-1.0/hd40.img 40
& $pil --boot-probe images/mcc-1.0/nocdboot --hd-image images/mcc-1.0/hd40.img --script tools/mcc/install.txt --steps 4000000000 --rtc-year 1994
& $pil --boot-probe images/mcc-1.0/nocdboot --hd-image images/mcc-1.0/hd40.img --script tools/mcc/gcc.txt --steps 4000000000 --rtc-year 1994
& $pil --boot-probe images/mcc-1.0/nocdboot --hd-image images/mcc-1.0/hd40.img --script tools/mcc/mandel.txt --fpu 80387 --steps 9000000000 --rtc-year 1994
```

sh (Linux, Termux): the same lines with `pil=target/release/pil` and
`$pil` in place of `& $pil`. The hard disk image keeps what each script
wrote, so copy it between stages to repeat one.

- tools/mcc/install.txt: fdisk (one Linux partition), mkfs (ext2 on
  /dev/hda1), install Base (CRC checked), no swap, LILO or packages.
- tools/mcc/gcc.txt: boots `ro root=301`, logs in as root, unpacks each
  diskette at `/` and runs `mccinstall gcca.tgz` and `gccb.tgz` (CRCs
  checked), then `gcc -v` (gcc version 2.5.8).
- tools/mcc/mandel.txt: copies programs/linux/mandel.c into /tmp,
  `time gcc -O2 -o mandel mandel.c`, then `./mandel i 1` (the picture),
  `time ./mandel i 500` (fixed point, as MANDEL.COM) and
  `time ./mandel f 100` (double precision).

### svgalib 1.1: vgatest and a Mandelbrot in 256 and 16 colours

MCC 1.0 has no svgalib, so it is built from source in the guest with
gcc 2.5.8: svgalib111.tar.gz (svgalib 1.1, May 1994; SHA256 ea7b2157...
df86, 305,979 bytes) from `slackware/slackware-2.0.0/source/d/svgalib` of
github.com/youngd24/oldlinux. svgalib sets modes through the VGA registers
itself (see the register list below), with `chipset VGA` in
/usr/local/lib/libvga.config. After gcc.txt, PowerShell:

```powershell
python tools/mcc/ustar_repack.py images/mcc-1.0/svgalib111.tar.gz images/mcc-1.0/svga.tgz
& $pil --new-tar-floppy images/mcc-1.0/svga.flp images/mcc-1.0/svga.tgz tools/mcc/svgabld.sh programs/linux/vgamand.c
& $pil --boot-probe images/mcc-1.0/nocdboot --hd-image images/mcc-1.0/hd40.img --script tools/mcc/svgalib.txt --fpu 80387 --steps 4000000000 --rtc-year 1994
& $pil --boot-probe images/mcc-1.0/nocdboot --hd-image images/mcc-1.0/hd40.img --script tools/mcc/vgatest.txt --fpu 80387 --steps 3000000000 --rtc-year 1994
& $pil --boot-probe images/mcc-1.0/nocdboot --hd-image images/mcc-1.0/hd40.img --script tools/mcc/vgamand.txt --fpu 80387 --steps 3000000000 --rtc-year 1994
```

sh: `python3` and `$pil` as above. ustar_repack.py rewrites the archive
as ustar owned by root, which MCC's cpio reads.

- tools/mcc/svgalib.txt runs tools/mcc/svgabld.sh: libvga.a into
  /usr/lib with its headers in /usr/include, vgatest into /usr/local/bin,
  vgamand into /tmp (7 minutes in the guest, 440 s host time).
- tools/mcc/vgatest.txt: vgatest mode 5 (320x200x256) and 4 (640x480x16),
  a screenshot of each (vgatest-5.ppm, vgatest-4.ppm), back to text.
- tools/mcc/vgamand.txt: programs/linux/vgamand.c in both modes, with
  screenshots; it prints the times() ticks and the sum of the iteration
  counts after returning to text (1104757 and 5294314, as on the host).

`--fpu 80387` gives the booted machine the 80387 with its AT error path:
an unmasked exception sets the busy latch and raises IRQ13; FWAIT and the
next coprocessor instruction wait until OUT F0h clears it. Linux 1.0
checks this at boot ("Checking 386/387 coupling... Ok, fpu using irq13
error reporting."). Without it, Linux emulates the coprocessor itself.

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
- **VGA registers** (src/dos_vga_regs.rs): the miscellaneous output,
  sequencer 0-4, CRTC 0-18h (11h bit 7 protects 0-7) and graphics
  controller 0-8 take any value. CPU access to A0000h follows the live
  sequencer and graphics controller (planes with map mask, read map, write
  modes, latches, chain-4; ordinary RAM with odd/even or the B0000h/B8000h
  maps), and the shown mode is read from the registers: 320x200x256
  chained is mode 13h, 16-colour planar 320x200, 640x200, 640x350 and
  640x480 are 0Dh, 0Eh, 10h and 12h, text has the CRTC's columns and rows.
  Other layouts (mode X, other sizes) are shown as nothing. Not modelled:
  odd/even graphics addressing, split screens, panning.
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
