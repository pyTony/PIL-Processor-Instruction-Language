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

## Increment 2 status (first part)

PR 2 is split in two. The first part, on branch claude/project-thread-ssjwyx:
- MOV CR0 and LMSW can set PE; MOV CR0 can clear it. PG still stops.
- Segment loads (MOV, POP, LDS/LES/LSS/LFS/LGS) and far JMP/CALL/RETF read
  and check GDT descriptors at CPL 0, set the accessed bit, and fault with
  #GP, #NP or #SS and the selector as error code.
- A CS descriptor with the D bit selects 32-bit default operand and address
  size.
- Real-mode loads change only the base, so unreal mode works.
- test386 still stops at POST 08h, which sets PE and PG together.

## Increment 2 status (second part)

Merged first part: #116. The second part, on the same branch:
- LDTR and TR state; LLDT and LTR at CPL 0 check the GDT descriptor and LTR
  marks the TSS busy. SLDT and STR store the selectors.
- TI=1 selectors in segment loads read the LDT.
- SGDT and SIDT (any mode), SMSW r32.
- LAR, LSL, VERR, VERW and ARPL, protected mode only (#UD in real mode is
  the decoder's refusal).
- Not yet: #GP(0) for LLDT/LTR above CPL 0 (they stop, as the other
  privileged instructions do), task switches, gates. test386 still stops at
  POST 08h (PE and PG together), so paging (increment 3) is next.

## Increment 3 status

Merged increment 2: #116 and #117. Increment 3, on the same branch:
- MOV CR0 can set and clear PG (PG needs PE, or #GP(0)); LMSW keeps PG.
  MOV r32,CR0 reads PG back.
- Every RAM access, instruction fetch included, walks the 386 page tables
  while PG is set: the PDE at CR3, then the PTE. There is no TLB, so a
  changed entry takes effect at once (a 386 would need a CR3 reload).
- A successful walk sets the accessed bit in both entries and the dirty bit
  in the PTE on a write; a walk that faults sets neither.
- At CPL 3 a supervisor page, or a write to a read-only page, is a
  protection fault. CPL 0-2 may write read-only pages (the 386 has no WP).
- #PF stops the run as `PageFault{eip,address,error_code}` with the 386
  error code (P, W/R, U/S). CR2 and delivery through the IDT come with
  increment 4. A page table outside RAM is a bus fault, as before.
- test386 now passes POST 08h and 09h and stops at POST 20h, on an IRET in
  protected mode (increment 4). Its paging tests (POST 11h) run after that.
- Not yet: RAM above 1 MiB for Dine (increment 9).
