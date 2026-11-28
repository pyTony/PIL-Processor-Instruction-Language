# 486DX, SVGA Graphics, and Protected-Mode Windows Plan

This roadmap extends PIL from its current 386 + VGA + DOS/Linux baseline toward
an optional **486DX** CPU profile, **PS/2 mouse** hardware, **SVGA graphics**
(VESA VBE 1.2/2.0 + Tseng ET4000AX), and the PC/BIOS services needed to run
protected-mode Windows (Windows 3.1 Standard Mode, Windows for Workgroups 3.11
386 Enhanced Mode, and Windows 95).

## Current baseline

- **CPU:** Full 386 protected mode, two-level paging with TLB/MicroTlb, rings
  0–3, call gates, V86 mode, task switches, TSS I/O permission bitmap
  (`SegmentState::io_permitted`), IDT exception/IRQ delivery, `test386` POST
  complete, compiled protected-mode blocks (`5,369` precompiled forms),
  optional 80387 FPU with AT `IRQ13` / port `F0h` error reporting, and an
  opt-in `--cpu 486dx` profile (Stage 1 complete: `CpuModel`, `EFLAGS.AC`,
  integrated FPU).
- **PC/BIOS:** PIC, PIT, RTC/CMOS, 8042 keyboard, COM1 16450 UART with IRQ4
  and Microsoft 2-button serial mouse (`origin/feat/serial-mouse-com1-irq4-20261126`),
  FDC + DMA, ATA hard disks, CGA/EGA/VGA planar and mode 13h, FreeDOS 1.4
  kernel boot, MCC Interim Linux 1.0+ boot with `gcc 2.5.8` and `svgalib 1.1`.
- **Front ends:** `--window` (Win32 on Windows; X11 on Linux/Termux:X11 with
  `--features x11`) and `--tty`.

---

## Milestone A — 486DX CPU profile (`--cpu 386|486dx`)

Keep `386` as the default profile so `test386` and `SingleStepTests/80386`
validation remain strictly unchanged. Select `486dx` explicitly via `--cpu 486dx`
(CLI, config, or session command).

### Stage 1 — CPU profile & `EFLAGS.AC` (Done)
- Added `CpuModel` (`I386`, `I486Dx`) to `SegmentState`, `Image`, `same_code_context`,
  and state traces (`trace::record_machine`).
- Added `Flag::AC` (bit 18) in PIL and gated `PUSHFD`, `POPFD`, `IRETD`, and
  32-bit TSS task switches on `CpuModel::I486Dx` (masked to 0 on `I386`).
- Wired `--cpu 386|486dx` across `--dos-session`, `--dos-console`, `--dos-text`,
  `--boot-probe`, `pildos.conf`, and `Session` (`cpu` command + `cpu_model` in
  snapshots). Selecting `486dx` enables the integrated x87 FPU and rejects
  `fpu none`.
- *(Note: `pil --pildos` `Setup` currently displays 20 fixed rows; exposing
  `cpu`, `ram`, and SVGA fields in that TUI later will require viewport
  scrolling in `src/host/setup.rs`.)*

### Stage 2 — The 6 new 486 instructions (gated on `CpuModel::I486Dx`)
1. `BSWAP r32` (`0F C8..CF`) — 32-bit byte swap (plus defined 16-bit handling).
2. `XADD r/m, r` (`0F C0/C1`, 8/16/32-bit, `LOCK`-eligible on memory).
3. `CMPXCHG r/m, r` (`0F B0/B1`, 8/16/32-bit, `LOCK`-eligible on memory).
4. `INVLPG m` (`0F 01 /7`) — CPL 0 only (`#GP(0)` at CPL > 0 or V86, `#UD` on
   register operand); evicts the linear page from `Ram`'s `Tlb` and `MicroTlb`
   without faulting on unmapped pages.
5. `INVD` (`0F 08`) and `WBINVD` (`0F 09`) — CPL 0 only, architectural no-ops in
   PIL's coherent RAM model.

### Stage 3 — 486 control registers, `CR0.WP`, and native `#MF` (`CR0.NE`)
- Admit `CR0` bits `NE` (bit 5), `WP` (bit 16), `AM` (bit 18), `NW` (bit 29),
  `CD` (bit 30) with `#GP(0)` on `CD=0 && NW=1`, plus `CR3` `PWT`/`PCD` (bits
  3/4), when `CpuModel::I486Dx`.
- Enforce `CR0.WP` in `Ram::translate` (and `Tlb`/`MicroTlb`) so read-only pages
  fault on supervisor writes too.
- Honor `CR0.NE`: when `NE=1`, unmasked x87 exceptions latch `ES`/`B` and raise
  native `#MF` (vector 16) on the next waiting FPU instruction or `FWAIT`
  (bypassed by no-wait control forms); when `NE=0`, keep AT `IRQ13` / port `F0h`.

### Stage 4 — Alignment Check (`CR0.AM` + `EFLAGS.AC` → `#AC` vector 17)
- When `I486Dx && CR0.AM && EFLAGS.AC && CPL == 3`, check natural alignment on
  user data accesses before commit and raise `#AC` (vector 17, error code `0`).
- Gate compiled blocks on `ac_active` so normal execution remains fast.

### Stage 5 — Precompiled forms & optional `CPUID` (SL-enhanced 486DX)
- Regenerate `forms.rs` / `common_forms.rs` with the new 486 forms (`BSWAP`,
  `XADD`, `CMPXCHG` inside blocks; block termination on `INVLPG`/`INVD`/`WBINVD`).
- Optional SL-enhanced `EFLAGS.ID` (bit 21) and `CPUID` (`0F A2`) leaf 0/1
  (`GenuineIntel`, family 4 DX/DX2 signature + FPU feature bit).

---

## Milestone B — Low priority / optional: PS/2 AUX mouse & extra serial mouse protocols

Because `origin/feat/serial-mouse-com1-irq4-20261126` already provides a
16450 COM1 UART with IRQ4 and a Microsoft 2-button serial mouse (supported out
of the box by Windows 3.1, WfW 3.11, Windows 95, DOS `CTMOUSE`, and Linux),
PS/2 mouse hardware is low priority. (`INT 15h AH=C0h` stays in Milestone D.)
- **Optional 8042 AUX controller** (`pil-pc`): commands `A7h` (disable AUX),
  `A8h` (enable AUX), `A9h` (interface test), `D4h` (write AUX device), port
  `64h` status bit 5 (`AUXB`), standard 3-byte PS/2 mouse packets, slave
  **IRQ12** (`pic.rs`), and BIOS `INT 15h AH=C2h` (PS/2 pointing device).
- **Optional serial mouse protocols** (`crates/pil-pc/src/serial.rs`): if
  Logitech 3-button (`'M3'`) or Mouse Systems 5-byte MSC support is desired
  later, extract a small `SerialMouse` / `MouseProtocol` type inside `pil-pc`
  so `Serial` remains pure 16450 UART logic.

---

## Milestone C — SVGA Graphics (VESA VBE 1.2/2.0 + Tseng ET4000AX)

- **Video RAM & aperture:** expand SVGA video memory to 1–2 MiB behind the
  64 KiB banked `A0000h` window (plus optional VBE 2.0 Linear Framebuffer for
  32-bit OSes / `VBEMP`).
- **VESA VBE (`INT 10h AH=4Fh`):** controller info (`4F00h`), mode info (`4F01h`),
  set/get mode (`4F02h`/`4F03h`), bank window control (`4F05h`), scanline length
  (`4F06h`), display start (`4F07h`), DAC palette (`4F08h`/`4F09h`), covering
  `640x480`, `800x600`, `1024x768` at 4/8/16/24 bpp.
- **Tseng ET4000AX registers:** segment select port `3CDh` plus extended CRTC /
  TS / ATC overflow and width registers, supported in-box by Windows 3.1,
  WfW 3.11, Windows 95, and MCC Linux `svgalib 1.1` / XFree86.
- **Display front ends:** generalize `vga::Display` and `--window` (Win32/X11)
  to render arbitrary SVGA resolutions and pixel formats (4-bit planar, 8-bit
  DAC-indexed, 15/16/24/32-bit RGB).

---

## Milestone D — Remaining PC/BIOS pieces for Protected-Mode Windows

- **8042/port `92h` soft reset & CMOS `0Fh` shutdown:** wire `FEh` / port `92h`
  reset vector through `p.cmos_shutdown` (`05h`/`0Ah`) so `HIMEM.SYS` and
  Windows 3.1 Standard Mode (`DOSX`) can return from protected mode to real mode.
- **RTC periodic IRQ8 & PIC completeness:** CMOS status B `PIE` (bit 6) and
  status C read-to-clear → slave **IRQ8**; slave IRQ12, serial IRQ3/4, and
  OCW3 special-mask mode in `pil-pc`.
- **32-bit port I/O:** support dword `IN`/`OUT` (`EAX, DX`) and `INSD`/`OUTSD`
  in `pil-pc`.
- **Configurable RAM & memory map:** `--ram` sizing (e.g. 16–32 MiB) and
  `INT 15h AX=E820h` alongside `AH=88h` / `AX=E801h`.
- **Target boot ladder:** FreeDOS 1.4 / DOS → Windows 3.1 Standard Mode
  (`win /s`) → Windows 3.1 / WfW 3.11 386 Enhanced Mode (`win /3`) → Windows 95.

---

## Principles

- **Program independence:** model documented CPU/BIOS/hardware contracts only;
  no guest-specific hacks or checks anywhere in `src/`, `crates/`, `isa/`, or
  `tools/`.
- **Portable paths & host independence:** no host-specific paths; Windows and
  Linux/Termux hosts remain supported equally.
- **Strict 386 baseline preserved:** `CpuModel::I386` remains the default.
