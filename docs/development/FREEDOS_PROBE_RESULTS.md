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

## Real-mode segment transfers: measured follow-up

Compiled Linux CI artifact at6b2f7d1c was run locally on the unchanged image,
10,000 steps per program:151/156 imported,116 encoding stops,24 service stops,
8 model faults,3 exits. JEMFBHLP and MOVEXBDA print usage text and exit0;VCHKBOX
remains a placeholder exit36. No driver operation or boot is established.
Reached mix includes404 MOV,82 PUSH,31 POP. First segment MOV/POP blockers vanish:

| First opcode | Programs | Interpretation |
|---|---:|---|
| F3 |54| Repeat prefix |
| E8 |18| Near CALL |
| A0 |17| MOV AL,moffs8 |
| FB |16| STI |
| 9C |5| PUSHF |
| A3 |3| MOV moffs,AX/EAX |
| AC |2| LODSB |
| C7 |1| MOV immediate to r/m |

SHARE now reaches the same CS-write model limitation as the prior seven faulting
programs;this is a newly exposed path,not a new unsupported opcode. The source
image stayed local and is not bundled in GitHub tests or runtime delivery.

## Additional MOV forms: local Linux CI runtime probe

Linux CI artifact e74c9510 probed the unchanged FD14LITE.img locally with10,000
steps per program.151/156 imports;96 MissingEncoding, 44 Service, 3 Exit, 8 Fault.
A0/A3/C7 first blockers disappear;reached MOV count rises404->489. Three
exit paths are unchanged (two usage paths exit0 and the placeholder exits36).
An original COM fixture used C6 to store127,A0 to load AL,and exited127.
The source image was not uploaded to CI or committed. No new ARM/hardware run.

```text
SUMMARY imported=151/156 exited=3; first missing-encoding blockers (program count):
54 opcode f3
18 opcode e8
16 opcode fb
5 opcode 9c
3 opcode ac
Reached instruction mix (executed count; DOS bridges listed separately):
489 MOV
82 PUSH
67 ADD
56 JMP
52 STD
45 CMP
31 POP
31 TEST
27 CLD
22 JA
22 XOR
17 JG
7 SUB
5 JNE
4 DOS INT21/AH09
4 SHL
3 DOS INT21/AH4c
1 JBE
1 OR
1 SHR
Stops at the first blocker per program; no skipping unknown bytes or static scanning of data. This is not whole-program coverage or a full DOS/BIOS implementation.
```

## Near CALL/RET and REP movement: local Linux CI runtime probe

Linux CI artifact b0ab7c81 probed unchanged FD14LITE.img locally with10,000
execution steps per program. Stops: {'MissingEncoding': 27, 'Budget': 21, 'Boundary': 19, 'Service': 51, 'Fault': 27, 'Exit': 6}.
Step budgets include REP elements; completed counts full instruction retirement.
Previous0.24 results:96 encoding stops,44 service stops,8 faults,3 exits.
Image SHA256:f539d456b792594bc3ca59d4e0f4c23d4f1fee73370c1390b2da245400718d36. No image/programs uploaded to CI or committed.
No new ARM/new-instruction hardware run; these are first-blocker paths only.

```text
SUMMARY imported=151/156 exited=6; first missing-encoding blockers (program count):
16 opcode fb
8 opcode 9c
3 opcode e3
Reached instruction mix (retired instructions; REP elements and DOS bridges listed separately):
193431 REP MOVSW elements
116427 REP MOVSB elements
49996 REP STOSB elements
597 MOV
147 PUSH
105 ADD
76 JMP
69 CLD
63 CMP
53 CALL
52 STD
47 POP
38 SUB
34 JNE
33 JB
31 TEST
30 RET
27 JA
27 XOR
24 MOVSB
Budget counts execution steps (including REP elements);completed counts fully retired instructions.
Stops at the first blocker per program; no skipping unknown bytes or static scanning of data. This is not whole-program coverage or a full DOS/BIOS implementation.
```

Six termination paths: prior JEMFBHLP/MOVEXBDA usage exits0 and VCHKBOX placeholder
exit36, plus UMBM exit0, CTMOUSE/PROTOCOL exit0 and CALLVER exit255. These do not
establish full application functionality. New budget stops remain inside REP;
boundary examples fetch copied code beyond the imported window. Fault examples
reach writes through CS, which the current segment model rejects. These are
separate loader/model limitations exposed by further execution, not decoder gaps.
