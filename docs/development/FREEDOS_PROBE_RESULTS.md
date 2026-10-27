# First FreeDOS image probe — 27 October 2026

Source: user-supplied32MiB FD14LITE.img,
SHA256f539d456b792594bc3ca59d4e0f4c23d4f1fee73370c1390b2da245400718d36.
No image or third-party program binaries/source packages are committed here.

## Reader verification

The compiled Linux CI0.20 runtime at04914f4b listed/read the actual image:
FAT16atLBA63,688 original files plus1,459 files in65 expanded package ZIPs.
All2,147 extracted file contents matched an independent Python FAT16/zipfile
extraction byte-for-byte. All208 executable occurrences were found. The image
hash remained unchanged. Container reader code is unchanged inab63a6ec.

## Probe with corrected MZ entry relocation

The compiled Linux runtime atab63a6ec probed the image directly with a10,000
step limit per program. Byte-identical container copies were deduplicated:
156 distinct candidates,151 DOS COM/MZ imports and5 expected LE rejections.
Of the151 imports,138 stopped at missing encodings,5at unsupported services,
7at model access faults,and VCHKBOX.COM exited successfully.

| First encoding blocker | Program count | Decoding interpretation for next work |
|---|---:|---|
| 1E |30| PUSH DS |
| FD |22| STD |
| FC |21| CLD |
| 06 |18| PUSH ES |
| FB |16| STI |
| 0E |8| PUSH CS |
| 8E |8| MOV segment register from r/m16 |
| E8 |8| Near relative CALL |
| 8C |4| MOV segment register to r/m16 |
| 16 |1| PUSH SS |
| 54 |1| PUSH SP/ESP |
| C7 |1| Immediate-to-r/m MOV group |

These counts are first observed blockers per program,not total instruction
frequencies or missing-opcode coverage. Rerun after each family is added to
expose later blockers. Service/access failures need separate investigation.

The baseline0.19 importer had rejected13 mouse-driver MZ files using relative
CS=FFF0,IP=0100. Relocating16-bit CS/SS with wrapping before validating the
physical entry fixed all13 imports. A synthetic regression now covers this.

Measured priorities are direction-flag controls and segment/stack infrastructure.
PUSH/CALL need independent stack addressing and precise writes;segment moves
need explicit selector/cache state. STI needs interrupt-inhibition semantics;
a flag assignment alone must not be claimed as full STI support.

## Hosted validation

Windows/Linux CI40470152015 passed210 tests per host,independent synthetic
FAT16/streaming-ZIP CLI checks,standalone86-file source and unpacked native ZIP
list/probe/extract checks. Existing49 backend cases still match4,983,058 bytes;
340,920 existing native CPU cases and full256x256 Mandelbrot checks passed.
No ARM execution or complete FreeDOS/BIOS compatibility is claimed for0.20.
The actual uploaded-image run above was local Linux execution of the compiled
CI artifact;the uploaded image was not sent to GitHub CI.

## Initial stack forms: measured follow-up

The Linux CI runtime at9189e64e was run locally against the same unchanged image
with10,000 steps per program. It still imports151/156:125 first encoding blockers,
18 service stops,7 model access faults,and the placeholder VCHKBOX exit36.
Reached counts now include77 PUSH and2 POP instructions. First blockers:

| Opcode / instruction | Programs |
|---|---:|
| 8C / MOV from segment register |36|
| F3 / repeat prefix |24|
| 07 / POP ES |18|
| FB / STI |16|
| 8E / MOV to segment register |9|
| E8 / near CALL |9|
| 1F / POP DS |8|
| 9C / PUSHF |2|
| AC / LODSB |2|
| C7 / MOV immediate to r/m |1|

PUSH blockers have moved forward to segment reads/loads and POP. The unchanged
number of encoding stops does not mean no progress: these are new first blockers.
This supports prioritizing transactional segment-selector/cache updates next,
alongside independent per-access addressing. No complete DOS program or boot
milestone is claimed. The source image was not uploaded to GitHub CI.
