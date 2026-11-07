# 80386 protected mode plan — 7 November 2026

Tony set protected mode as the priority, ahead of the x87 FPU. The plan is
eight CPU increments and two Dine increments, each a PR against dev with
expected-state, fault and size-combination tests and green Windows/Linux CI.
The full write-up (what dev has, DOS-side scope) is the project document
"80386 protected mode plan for PIL and Dine".

## Yardstick: test386.asm

barotto/test386.asm at cfd052d1 (GPLv3-or-later) is built outside the
repository and run with `pil --run-rom test386.bin`. The ROM reports its
progress on POST port 190h and stops with HLT on the first failing test. It
sets PE and PG in the same `MOV CR0` at POST 08h, so it cannot pass 08h until
descriptors and paging both work. On dev 1a82f59 it already passes POST
00h-06h and stops at 08h with `UnsupportedControl` on `MOV CR0,EAX`
(EAX=80000011h). That is a local Linux run of the 64 KB ROM built with
`nasm -i./src/ -f bin src/test386.asm -w-all` (NASM 2.16.01), SHA256
a53356b0c6073434c3deb8baeed5fbb5f0e61cd027d2923311f6d5be39ed3c8b.

SingleStepTests/80386 (revision 459d49fb) has only real-mode data; the
protected, unreal and V86 sets are announced but not published.

## Increments

| # | Increment | test386 target |
| --- | --- | --- |
| 1 | Real-mode exception delivery (#DE, #GP, #SS, #UD, INT n/INT3/INTO) | 00h-06h stay green; SST exception cases compared |
| 2 | Descriptors and the PE switch: CPL, GDT/LDT, selector checks, D bit, LLDT/LTR/SGDT/SIDT, ARPL/LAR/LSL/VERR/VERW, SMSW r32, CLTS, unreal mode | own tests and a DOSBox-X COM |
| 3 | Paging: CR3 walk, A/D bits, U/S and R/W, CR2, #PF, RAM above 1 MiB | 08h, 09h, 0Bh-10h |
| 4 | Protected-mode exceptions and interrupts: IDT gates, error codes, #DF, triple fault as shutdown | 11h-1Ch |
| 5 | Privilege levels: call gates, conforming code, inter-level transfers, IOPL, I/O bitmap | 20h |
| 6 | V86 mode | 21h |
| 7 | Task switching | 22h (128 KB ROM) |
| 8 | Remaining test386 (E0h, EEh) | FFh |
| 9 | Dine: memory above 1 MiB (INT 15h 88h/E801h/87h, CMOS) | — |
| 10 | Dine: XMS 3.0 driver (INT 2Fh 4300h/4310h) | — |

DPMI, VCPI and EMS are not Dine services at first: DOS extenders (CWSDPMI,
DOS/4GW, PMODE/W) bring their own DPMI host or run raw, and JEMM386 provides
VCPI/EMS once V86 mode and paging work. A raw-mode extender switches back to
real mode for INT 21h, so Dine's existing services keep answering.

## Increment 1 status

Done on branch claude/project-thread-ssjwyx; see STATUS.md, "Real-mode
exception delivery". The test386 runner (`--run-rom`) was already on dev.
