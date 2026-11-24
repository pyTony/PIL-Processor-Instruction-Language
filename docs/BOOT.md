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

### Scripted runs: install, gcc 2.5.8 and a timed Mandelbrot

`pil --boot-probe IMAGE --script FILE` runs a headless machine through a
script, one step per line (`#` starts a comment):

| Step | Meaning |
|---|---|
| `run N` | run N steps (`_` separators allowed) |
| `type TEXT` | type TEXT (`\r` or `\n` is Enter, `\s` a space, `\\` a backslash) |
| `prompt TEXT` | wait until the screen changed after the last `type` and its last line ends with TEXT |
| `screen TEXT` | wait until TEXT is anywhere on the screen |
| `floppy IMAGE` | change the diskette (same format) |
| `show` | print the screen |
| `mark NAME` | print the steps and host seconds since the last mark |

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

`--fpu 80387` gives the booted machine the 80387 with its AT error path:
an unmasked exception sets the busy latch and raises IRQ13; FWAIT and the
next coprocessor instruction wait until OUT F0h clears it. Linux 1.0
checks this at boot ("Checking 386/387 coupling... Ok, fpu using irq13
error reporting."). Without it, Linux emulates the coprocessor itself.

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
  nIEN) with **IRQ14** through the slave PIC, for the fixed disk as the
  primary master. Only a booted machine with a fixed disk has it; the
  other ATA channels and the absent slave stay empty.
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
