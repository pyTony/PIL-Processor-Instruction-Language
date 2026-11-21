# BIOS boot path (Linux plan milestone 2): work in progress

Branch `feat/bios-boot-20261122`, built on #292 (`refactor/pc-machine-layer-20261121`).
Plan: docs/development/LINUX_BOOT_PLAN.md. First goal: MCC Interim 0.99.p10+
(`images/mcc-interim/boot`, 1.2 MB floppy, git-ignored) reaching LILO's `boot:`.

## Done in this branch
- src/dos_bios_disk.rs: raw images as BIOS drives (00h, 01h, 80h); INT 13h
  00-04h, 08h, 15h-18h and no-op 0Ch/0Dh/10h/11h/14h; 64 KiB DMA boundary
  error 09h on diskettes; status bytes 0040:0041/0074; diskette parameter
  table at F000:EFC7 (INT 1Eh); CMOS register 10h drive types.
- Loaded gains `disks` (bios_disk::Disks) and `booted`; INT 13h goes to
  bios_disk::int13 whenever a BIOS disk is attached.

## Still to do (in order)
1. `dos::boot(image, drive)` building a booted machine:
   - start from `load(&[0xf4], "BOOT.COM")`, then `select_xms(false)`;
   - attach the image (`Disk::floppy` by size; 80h for a hard-disk image);
   - set `booted = true` and clear 0500h-9FFFFh (Dine's DOS data, MCBs,
     PSP);
   - re-point the IVT: BIOS vectors only (08h-1Ch entries), zero for the
     DOS vectors (20h-2Fh, 33h), then `bios_disk::install_table`;
   - BDA equipment word: floppy count in bits 6-7, bit 0 set;
   - fetch window `fetch_base=0, start=0, end=RAM size`;
   - read sector 0 to 0000:7C00; CS=DS=ES=SS=0, IP=7C00h, SP=7C00h,
     DL=drive, EFLAGS=202h.
2. In boot mode, `vectors::answered` must require `vector == entry(n)`: a
   zero vector is a real 0000:0000. Skip `brk::returned`, `xms::trapped` and
   `mouse::returned` when `booted` (Linux 0.99 runs code at physical 0).
3. INT 19h: read sector 0 to 7C00h again and jump there.
4. CLI `pil --boot IMAGE [--hd]` on the console front end, plus a headless
   `--boot-steps N` that prints the text screen and the stop reason.
5. Run MCC until LILO prints `boot:`; fix each stop generically (documented
   BIOS behaviour, original fixtures in tests), as with DOS programs.
6. Unit tests: geometry by size, CHS->LBA, reads with the DMA boundary,
   AH=08h results, CMOS 10h; a tiny original boot sector that prints via
   INT 10h and reads sector 2.
