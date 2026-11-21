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
- **`pil --boot IMAGE`:** the boot sector at 0000:7C00, DL=00h/80h.
- **INT 13h on the image:** AH 00, 02, 03, 08, 15, 41/42/48 (LBA), with
  geometry for floppy sizes and a hard-disk translation; INT 19h; INT 12h;
  INT 15h AX=E820h, now needed.
- **First target:** boot the real FreeDOS kernel from FD14LITE.img (KERNEL.SYS,
  then FreeCOM). This proves the boot path without Linux and gives a real
  DOS kernel next to Dine's own DOS.

### 3. Hardware a protected-mode OS drives itself
- **IRQs:** every IRQ 0-15 through the PIC to the IDT, real and protected
  mode (today only IRQ0/1/5/7 and the mouse callback reach guests).
- **8042:** command byte, self-test, output port (A20, reset through
  port 92h and FEh), keyboard enable/disable, scan code set 1 translation.
- **16550 UART** for `console=ttyS0`, and VGA text memory as the second
  console.
- **Disks:** IDE and FDC report "none" correctly; a root file system comes
  from an initrd first (no disk controller needed).
- **HLT** in protected mode idles until the next IRQ.

### 4. Early Linux
- **Kernel:** a 386-capable Linux 2.0.x or 2.2.x kernel with an initrd
  (ext2 or minix) and `console=ttyS0`. Kernels from 3.8 on dropped the 386.
  Kernel and root image are user-supplied or downloaded with permission,
  and stay out of the repository.
- **Routes:**
  - (a) through milestone 2, booting a boot floppy (LILO/SYSLINUX);
  - (b) LOADLIN under Dine's DOS;
  - (c) a direct loader for the Linux boot protocol (setup at 9000:0000,
    kernel at 10000h or 100000h, initrd high, command line).

  (a) is the general one; (c) is the quickest to debug.
- **Done when:** the kernel reaches `init` from the initrd and a shell
  answers on the serial console.

### 5. Compiled protected-mode code
- **Forms:** enumerate the decoder for a 32-bit default code segment and
  protected-mode segment semantics as well (forms already take the segment
  state at run time; their enumeration fixes real mode).
- **Blocks with paging:** plan in protected mode and with paging. The guard
  adds CPL, CR0/CR3 and the code segment's descriptor; code watches use
  physical pages. A page-table write or CR3 load invalidates by version.
- **Checks:** generated-backend comparisons on pm_* cases; test386 with
  blocks against without; DOS/4GW Doom and the Linux boot as workloads.

## Principles
- **Program independence:** PIL and Dine stay program-independent. Every
  device follows its datasheet or the documented PC/AT behaviour, tested
  with original fixtures. No kernel- or distribution-specific code.
- **Host independence:** no architecture conditionals; Windows and
  Linux/Termux hosts as today.
- **Second ISA:** a "canary" ISA to test processor independence of the
  machine layer can follow milestone 1. It is not part of this plan.
