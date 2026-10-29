# FreeDOS service increment — 29 October 2026

Base: dev 8d012c5d69f8b1309e146f21ec1b4f6ea07f5d5b.

Adds INT21 AH1A/2F DTA set/get (initial PSP:0080), AX4400 device information
for the five initially open standard handles, and INT10 AH01/02 cursor shape
and per-page position setters. AH03 now reads the BIOS data area instead of
returning constants. BDA cursor shape starts at 0607. Pages 0..7 have separate
positions. Invalid page numbers stop explicitly. Writes validate the full word
before mutation; getters read all required memory before changing registers.
DTA setters retain the far pointer without touching the caller's buffer.

IOCTL reports a minimal console profile 0083 for handles 0..2 and character
profile 0080 for AUX/PRN. Closed/invalid handles return CF=1, AX=6. Other IOCTL
subfunctions still stop explicitly. This is a headless compatibility profile,
not a full DOS device driver, redirection model or display adapter. No file
search, graphical cursor rendering, TSR or mouse implementation is added.

Contracts: Ralf Brown's Interrupt List:
- https://fd.lod.bz/rbil/interrup/dos_kernel/211a.html
- https://fd.lod.bz/rbil/interrup/dos_kernel/212f.html
- https://fd.lod.bz/rbil/interrup/dos_kernel/214400.html
- https://delorie.com/djgpp/doc/rbinter/id/91/0.html
- https://delorie.com/djgpp/doc/rbinter/id/93/0.html

Validation on Windows:
- Full Rust suite: 353 passed before adding AH02; final DOS suite: 32 passed.
- Release build passes.
- Same FD14LITE.img, 100000 steps per program, before/after:

| Stop category | Before | After |
|---|---:|---:|
| Imported | 151/156 | 151/156 |
| Exit | 16 | 20 |
| Service | 55 | 42 |
| Missing encoding | 26 | 35 |
| Image boundary | 54 | 54 |

Exit codes before: 0 x11, 1 x3, 36 x1, 255 x1.
After: 0 x14, 1 x4, 36 x1, 255 x1. Termination is not proof of application
success. Increased missing-encoding stops mean programs progressed past service
calls and encountered later instruction gaps. No budget stops in this run.
All 16 FF first blockers are FF E0 (near indirect JMP AX). This is the next
high-impact ISA target, separate from this service increment. Existing boundary
stops remain unresolved; whole-program compatibility is not claimed.

Reproduce from repository root:

```powershell
.\tools\build.ps1 -Release
.\target\release\pil-proof.exe --dos-probe --steps 100000 .\images\FD14LITE.img > .\target\freedos-services.txt
cargo test --locked dos::tests
```
