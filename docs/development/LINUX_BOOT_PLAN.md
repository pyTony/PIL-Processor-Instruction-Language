# Plan: a PC machine layer, booting early Linux, compiled protected-mode code

Tony, 21 November 2026: "Boot early linux and implement the compiled
protected code. First modularize OS layer to have common functionality
available for linux also."

## Where things stand

- **CPU:** protected-mode plan increments 1-10 are done (segmentation,
  paging, rings, call gates, V86, task switches, IDT delivery, XMS).
  test386 runs every POST.
- **Environment:** Dine is a DOS run in Rust, not a booted OS. Its BIOS and
  DOS services answer INT n in real mode only. A protected-mode program
  runs only when it brings its own extender (DOS/4GW, CWSDPMI).
- **Layout:** everything Dine models sits under `mod dos` (src/dos*.rs). One
  struct, `dos::Loaded`, holds both:
  - the hardware: CPU, RAM, segment state, FPU, PIC, PIT, keyboard
    controller, VGA, CMOS/RTC, speaker, Sound Blaster, COM1;
  - DOS: MCB arena, PSP, file handles and drives, EXEC frames, the INT 33h
    mouse driver, XMS, country and break settings.

  The run loop (`run_with_backend`) handles IRQs, ports, the FPU, blocks
  and the CPU, interleaved with DOS's INT n dispatch.
- **Speed:** precompiled forms and blocks cover real-mode 16-bit code only.
  Protected-mode and 32-bit code runs in the PIL interpreter.

A Linux kernel uses no DOS. After its real-mode setup it drives the
hardware itself: PIC, PIT, 8042, CMOS, VGA text memory, a UART, and disk
controllers or a RAM disk. So "common functionality" means a **PC machine**
both can run on: CPU, RAM, devices, firmware (BIOS) and the run loop. DOS
becomes one layer above it, and a booted OS (Linux, or the real FreeDOS
kernel) is another.

## Milestones (one PR each, every existing test passing throughout)

### 1. Machine layer (refactor, no behaviour change)
- **`pc::Machine`:** CPU, RAM, segment state/image, FPU, decode cache,
  blocks, and the devices: `pc::pic`, `pc::pit`, `pc::kbc` (8042),
  `pc::cmos`, `pc::vga` (text, planar, fonts), `pc::speaker`, `pc::uart`
  (from dos_serial), `pc::sb`.
- **`bios`:** INT 10h, 11h, 12h, 13h, 15h, 16h, 1Ah answered from Machine
  state alone, so a booted OS's real-mode code can call them too.
- **`dos::Dos`:** the rest: arena, PSP, files/drives, EXEC, XMS, INT 33h
  driver, NLS, config. `dos::Loaded` becomes `Machine` plus `Dos`.
- **Run loop:** `pc::run` steps the machine (IRQs, ports, blocks, CPU) and
  asks a `Firmware` trait about INT n and private entries. Dine's DOS
  implements it; plain BIOS firmware implements it for a booted OS.
- **Rename only:** module moves stay in `src/` paths that git can follow;
  public front ends (pil flags, Dine ABI, session protocol) stay as they
  are.

### 2. Booting a disk image through the BIOS
Done (22 November 2026, `feat/bios-boot-20261122`): MCC Interim's LILO
reaches `boot:`, loads the kernel, and Linux 0.99pl10 switches to protected
mode, decompresses and prints `Console: colour`. It then stops on OUT 60h
(a command to the keyboard itself), which is milestone 3. Usage:
docs/BOOT.md. The FreeDOS kernel boot was not tried yet.

- **`pil --boot IMAGE`:** the boot sector at 0000:7C00, DL=00h/80h.
- **INT 13h on the image:** AH 00, 02, 03, 08, 15, 41/42/48 (LBA), with
  geometry for floppy sizes and a hard-disk translation; INT 19h; INT 12h;
  INT 15h AX=E820h, now needed.
- **First target:** boot the real FreeDOS kernel from FD14LITE.img (KERNEL.SYS,
  then FreeCOM). This proves the boot path without Linux and gives a real
  DOS kernel next to Dine's own DOS.

### 3. Hardware a protected-mode OS drives itself
Partly done (22 November 2026, `feat/pc-devices-20261122`): keyboard
commands, absent UARTs, ISA open bus when booted, the diskette controller
with DMA channel 2 and IRQ6, and a TLB. Linux 0.99 initializes its drivers
and reads its root floppy. Not done: the 8042 controller commands beyond
A20/reset, the AT hard-disk controller.

- **IRQs:** every IRQ 0-15 through the PIC to the IDT, real and protected
  mode (today only IRQ0/1/5/7 and the mouse callback reach guests).
- **8042:** command byte, self-test, output port (A20, reset through
  port 92h and FEh), keyboard enable/disable, scan code set 1 translation.
- **Console:** VGA/EGA text memory and the 8042 keyboard. Kernels before
  2.1 have no serial console.
- **Floppy disks:** the floppy controller (8272/82077 at 3F0h-3F7h:
  specify, recalibrate, seek, read/write data, sense interrupt; IRQ6),
  8237 DMA channel 2 and the CMOS floppy types. The early kernels read
  their root floppy through it; they have no initrd.
- **Hard disks:** later, the AT hard-disk controller (1F0h-1F7h, IRQ14).
- **HLT** in protected mode idles until the next IRQ.

### 4. Early Linux: MCC Interim
Reached (22 November 2026, `feat/pc-devices-20261122`): Linux 0.99.10
mounts the root floppy and MCC's install menu waits at its prompt, after
about 1.5 billion guest steps (10.5 minutes headless).

- **Target (Tony's choice):** MCC Interim Linux (Manchester Computing
  Centre, 1992-93), Linux 0.97-0.99 on a boot floppy and a root floppy, 386
  only, minix file system.
- **Boot:** the kernel's own boot sector loads it through BIOS INT 13h
  (milestone 2). The kernel then switches to protected mode and reads the
  root floppy through the floppy controller (milestone 3), asking for a
  disk swap on the console.
- **Media:** user-supplied, or downloaded from a Linux history archive with
  permission. Never in the repository.
- **Chosen image (21 November 2026):** MCC Interim 0.99.p10+ `boot.gz`
  (878,864 bytes, 9 July 1993) from the Manchester mirror,
  `debian.man.ac.uk/non-debian/mcc-interim/old/0.99.p10+/images/`, kept
  locally in the git-ignored `images/mcc-interim/`.
  - SHA256 of `boot.gz`:
    74610555a78d0ff7dcea2da121d2653c5443dc83637990a449c24efbfa9084e8.
  - Unpacked, it is a 1,228,800-byte 1.2 MB 5.25" floppy (80 cylinders,
    2 heads, 15 sectors), so the CMOS reports drive A: as type 2.
  - LILO is in its boot sector. LILO reads the kernel through BIOS INT 13h
    and prints `boot:` through INT 10h, then waits on INT 16h. Enter boots
    the floppy as a read-only `/` with MCC's install menu.
  - The kernel then reads the same floppy through the floppy controller
    (/dev/fd0); no disk swap is needed.
- **Done when:** the root floppy mounts and a shell answers on the console.
- **Done (23 November 2026), hard disk:** the AT hard-disk controller
  (1F0h-1F7h, 3F6h, IRQ14), the INT 41h table and CMOS types. MCC's menu
  partitions, formats (Minix) and installs Base onto a 40 MB image; booting
  the diskette with `rw root=301` runs Linux from /dev/hda1 to a root shell.
  Packages beyond Base need MCC's other disks.
- **Done (23 November 2026), MCC Interim 1.0+ (Linux 1.0.4):** from the
  oldlinux repository (github.com/youngd24/oldlinux, mcc-linux/MCC-1.0, the
  only MCC release there; no svgalib or X server in it). Boot and root
  disks: the diskette change line and a disk change in the probe and the
  console; 8042 ADh/AEh; open-bus word probes and unused Sound Blaster
  addresses. Installed onto an ext2 /dev/hda1 and booted with `ro
  root=301` to a root login.
- **Next:** MCC's package floppies (raw tar streams on the GoTek images,
  read from drive B: or after a reboot), then svgalib (Tony's choice over
  X, 23 November 2026), which needs a Linux with svgalib from elsewhere.
- **Later:** Linux 2.0/2.2 with an initrd and a serial console. Linux 3.8
  and later dropped the 386.

### 5. Compiled protected-mode code
- **Done (23 November 2026), blocks with paging:** blocks are planned in
  protected mode and with paging, keyed by linear address as before. Under
  paging a block stays in one page. Code stamps (blocks and the decode
  cache) record the bus address the code was found at; each check
  translates the linear address again with the current CR3 and CPL (a TLB
  hit, or a walk that sets the accessed bits a fetch sets) and compares it,
  then the 64-byte line counters of that bus range. A changed mapping, a
  fault, or a write to the code (through any alias) replans the block. In
  real and V86 mode the old linear exclusions stay; elsewhere video memory
  and the ROM/BIOS area (by bus address) never join a block.
  MCC Interim to its menu: about 80 s -> 11 s (200 million steps).
- **Not done, forms:** the precompiled table still enumerates the real-mode
  decoder. A 32-bit protected-mode instruction runs its native form when
  its PIL program is one of those forms (they take the segment state at
  run time), else the interpreter inside the block. Enumerating a 32-bit
  default code segment is the next step.
- **Checks:** a paged 32-bit boot-sector fixture against the interpreter
  per slice, with a remapped page and code rewritten through an alias page
  (tests/unit/precompiled.rs); DOS/4GW Doom and the Linux boot as
  workloads. test386 with blocks against without is not done.

## Principles
- **Program independence:** PIL and Dine stay program-independent. Every
  device follows its datasheet or the documented PC/AT behaviour, tested
  with original fixtures. No kernel- or distribution-specific code.
- **Host independence:** no architecture conditionals; Windows and
  Linux/Termux hosts as today.
- **Second ISA:** a "canary" ISA to test processor independence of the
  machine layer can follow milestone 1. It is not part of this plan.
