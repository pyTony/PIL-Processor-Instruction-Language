# TODO

Current completed baseline and recent deliveries are tracked in
[docs/development/STATUS.md](docs/development/STATUS.md). The November 10
compiler TODO is archived at
[docs/history/precompiled/TODO_COMPILER_2026-11-10.md](docs/history/precompiled/TODO_COMPILER_2026-11-10.md).

## Standing rules

- PIL and Dine are always program-independent libraries, used the same way
  everywhere. No tool, build step, or CI job may produce an engine, backend,
  Dine library, or translator for one guest program
  (`tools/check_program_specific.py`). `Dine_<program>` launchers carry only a
  loader and the guest bytes.
- Keep paths portable everywhere (`tools/check_portable_paths.py --self-test`)
  and keep the `386` CPU profile as the strict default.

## Active roadmap — 486DX, SVGA, and Protected-Mode Windows

See [docs/development/486DX_SVGA_WINDOWS_PLAN.md](docs/development/486DX_SVGA_WINDOWS_PLAN.md).

- **Milestone A — Optional 486DX CPU profile (`--cpu 386|486dx`):**
  - Stage 1 *(in progress)*: `CpuModel` (`I386`/`I486Dx`), `EFLAGS.AC`, and
    `--cpu` wiring across CLI, config, and session.
  - Stage 2 *(in progress)*: `I486Dx`-gated instructions (`BSWAP`, `XADD`, `CMPXCHG`,
    `INVLPG`, `INVD`, `WBINVD`).
  - Stage 3: 486 `CR0` (`NE`, `WP`, `AM`, `NW`, `CD`) / `CR3` (`PWT`/`PCD`),
    supervisor `CR0.WP` enforcement, and native `#MF` when `CR0.NE=1`.
  - Stage 4: Alignment Check (`CR0.AM` + `EFLAGS.AC` at CPL 3 -> `#AC` vector
    17) with compiled-block gating.
  - Stage 5: Regenerate `forms.rs` / `common_forms.rs` for 486 forms; optional
    SL-enhanced `EFLAGS.ID` and `CPUID` leaf 0/1.
  - Follow-up: add viewport scrolling in `src/host/setup.rs` to expose `cpu`,
    `ram`, and SVGA fields in `pil --pildos` Setup.
- **Milestone B — Hardware Mouse (PS/2 AUX + IRQ12 + BIOS `INT 15h`):**
  - 8042 AUX controller (`A7h`/`A8h`/`A9h`/`D4h`, `AUXB`, 3-byte packets,
    slave IRQ12), BIOS `INT 15h` `AH=C0h`/`AH=C2h`, and Win32/X11 `--window`
    routing to PS/2 AUX.
- **Milestone C — SVGA Graphics (VESA VBE 1.2/2.0 + Tseng ET4000AX):**
  - 1–2 MiB SVGA VRAM with banked `A0000h` aperture (plus optional VBE 2.0
    LFB), `INT 10h AH=4Fh` (`4F00h`–`4F09h`), Tseng ET4000AX registers (`3CDh`
    and extended CRTC/TS/ATC), and generalized `vga::Display` / `--window`
    rendering.
- **Milestone D — Remaining PC/BIOS pieces for Protected-Mode Windows:**
  - 8042 `FEh` / port `92h` soft reset wired through CMOS `0Fh` shutdown
    (`05h`/`0Ah`).
  - RTC periodic IRQ8 (CMOS status B `PIE` / status C read-to-clear) and PIC
    completeness (IRQ8/IRQ12, OCW3 special-mask mode).
  - 32-bit port I/O (`IN`/`OUT EAX, DX`, `INSD`/`OUTSD`) in `pil-pc`.
  - Configurable `--ram` (16–32 MiB) and `INT 15h AX=E820h` alongside
    `AH=88h` / `AX=E801h`.
  - Target boot ladder: FreeDOS 1.4 / DOS -> Windows 3.1 Standard Mode
    (`win /s`) -> Windows 3.1 / WfW 3.11 386 Enhanced Mode (`win /3`) ->
    Windows 95.

## Longer-term follow-ons

- **VGA & Linux guests** ([docs/development/LINUX_BOOT_PLAN.md](docs/development/LINUX_BOOT_PLAN.md)):
  VGA Mode X (`vgatest` 6–8), 2-colour modes, SVGA chipsets and X under Linux;
  later Linux 2.0/2.2 with initrd and serial console.
- **Block compiler** ([docs/development/BLOCK_COMPILER.md](docs/development/BLOCK_COMPILER.md)):
  side exits past branches/stores within a single compiled block.
- **Processor independence**: a second ("canary") ISA on top of the shared
  machine layer.
