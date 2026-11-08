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
`--run-rom` also takes the 128 KB ROM (`ROM128 equ 1` in
src/configuration.asm), which holds the task-switching tests of POST 22h:
its lower half goes to E0000h, its upper half to F0000h as before. Built the
same way, SHA256 c4537dcdc514381b18eb6e58d4464efbc16cbf67910453d2adc2c73cac0c25fe.

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

## Increment 4 status

Merged increment 3: #122. Increment 4, on the same branch:
- src/interrupt.rs `deliver`: exceptions, INT n, INT3 and INTO in protected
  mode go through 286 and 386 interrupt and trap gates in the IDT, with the
  386 checks in order (IDT limit, gate type, gate DPL for INT n, present),
  then the target code segment (code, DPL at most CPL, present). Error
  codes for #DF, #NP, #SS, #GP and #PF are pushed; EXT is set unless INT n
  is being delivered. #PF writes CR2.
- A fault during delivery is delivered in turn, or as #DF (error code 0)
  for contributory after contributory and for #PF after #PF or a
  contributory fault; a fault during #DF is a shutdown (triple fault).
- New ExecError::SegmentNotPresent carries #NP's selector.
- IRET/IRETD in protected mode (isa/i386/iret_protected.pil) return at the
  same level. NT=1 (task return) and VM=1 at CPL 0 (return to V86 mode)
  stop.
- Not yet: the TSS stack switch to an inner level (a CPL 3 interrupt to a
  ring 0 handler), IRET to an outer level, task gates, IRQs in protected
  mode (Dine stops there).
- test386: the order of POST codes is 08h, 09h, 20h, 21h, 22h, 0Bh-0Fh,
  10h-1Ch, so increment 5 (ring 3) comes before anything after 09h. POST 20h
  starts with an IRETD to ring 3. A local build that skips POST 20h-22h and
  0Bh (not in the repo) runs 0Ch-10h and stops at POST 11h on the next
  switch to ring 3. The ROM runner now accepts up to 1,000,000,000 steps.

## Increment 5 status

Merged increment 4: #123. Increment 5, on the same branch:
- src/privilege.rs (shared by both backends and interrupt delivery): far
  JMP/CALL through 286 and 386 call gates, CALL to an inner level with the
  TSS stack switch and parameter copy, RETF/IRET to an outer level with
  SS:ESP popped and data segments nulled.
- Interrupts and exceptions from ring 3 to a ring 0 handler switch to the
  TSS stack. #TS is new (ExecError::InvalidTss).
- Privileged instructions above CPL 0 are #GP(0); CLI/STI check IOPL;
  POPF keeps IOPL and IF where it may not change them; IN/OUT/INS/OUTS
  (and Dine's own ports in protected mode) check IOPL and the TSS I/O
  bitmap.
- test386 passes POST 20h and stops at POST 21h (V86 mode, increment 6).
  POST 22h is task switching (increment 7). A local build that skips 21h
  and 22h runs 0Bh-17h; 17h fails on ARPL not writing back an unchanged
  memory selector, and with that removed 18h-1Ah run, with 1Ah failing on
  ENTER at ring 3 on a supervisor stack page.


## Increment 6 status

Merged increment 5: #124. Increment 6, on the same branch:
- Virtual-8086 mode is protected mode with a V86 CS cache (base
  selector*16, limit FFFFh, DPL 3; `SegmentState::v86`) and VM set. CPL is
  3 there, so paging checks it as a user.
- IRETD at CPL 0 with VM in the popped flags enters it, popping ESP, SS,
  ES, DS, FS and GS (privilege.rs `far_return`, new optional operand).
- In V86 mode segment loads, far JMP/CALL/RETF and IRET work as in real
  mode. CLI, STI, PUSHF, POPF, IRET, INT n, INT3 and INTO need IOPL 3, else
  #GP(0); POPF and IRET never change IOPL there. HLT and MOV CRn are
  #GP(0). I/O always goes through the TSS bitmap.
- Interrupts and exceptions from V86 mode go to a nonconforming ring 0
  handler only (#GP(selector) otherwise) on the TSS ring 0 stack, with GS,
  FS, DS and ES pushed and then nulled; IRETD returns.
- test386 passes POST 21h. POST 22h has no tests in the 64 KB ROM, so it
  goes on to 0Bh-16h and stops at POST 17h on the ARPL write-back to a
  read-only memory operand (known since increment 5). Next blockers there:
  that ARPL, then ENTER at ring 3 on a supervisor stack page (POST 1Ah).
- Follow-up: ARPL writes a memory destination only when the RPL changes
  (new PIL `store_if`). test386 now runs POST 17h-19h and stops in POST
  1Ah, the ENTER case above.
- Follow-up: `--run-rom` takes the 128 KB ROM. It passes POST 00h-21h as
  the 64 KB one does and stops in POST 22h, task switching (increment 7),
  on #GP(00A8h) at 00C0:0012.
- The fixed-image generated backend decodes each instruction once, at the
  starting mode: its V86 case runs 16-bit code on both sides of the IRETD
  and ends on MOV CR0 rather than HLT (HLT decoded at CPL 0 halts there).
