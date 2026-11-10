# 80386 protected mode plan — 7 November 2026 (status 8 November 2026)

Update: the 128 KiB POST 22h stop at #GP(00A8h) was the ROM's error path,
after its TS-check handler compared SMSW EAX and CR0 while paging was on.
The emulator now chooses full modeled CR0 for SMSW r32's undefined upper
half. It reaches the first actual task gate, INT 28h at 0053:E0A18; task
switching remains unimplemented. The 64 KiB ENTER failure remains open.

Tony set protected mode as the priority, ahead of the x87 FPU. The plan is
eight CPU increments and two Dine increments, each a PR against dev with
expected-state, fault and size-combination tests and green Windows/Linux CI.
Increments 1-6 are merged; increment 7 (task switching) is next. The section
"Current state and handoff" below is written so that another assistant can
pick the work up from this file alone.

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

| # | Increment | test386 target | Status |
| --- | --- | --- | --- |
| 1 | Real-mode exception delivery (#DE, #GP, #SS, #UD, INT n/INT3/INTO) | 00h-06h stay green; SST exception cases compared | done, #104 |
| 2 | Descriptors and the PE switch: CPL, GDT/LDT, selector checks, D bit, LLDT/LTR/SGDT/SIDT, ARPL/LAR/LSL/VERR/VERW, SMSW r32, CLTS, unreal mode | own tests and a DOSBox-X COM | done, #116 and #117 |
| 3 | Paging: CR3 walk, A/D bits, U/S and R/W, CR2, #PF, RAM above 1 MiB | 08h, 09h, 0Bh-10h | done, #122 (RAM above 1 MiB is increment 9) |
| 4 | Protected-mode exceptions and interrupts: IDT gates, error codes, #DF, triple fault as shutdown | 11h-1Ch | done, #123 |
| 5 | Privilege levels: call gates, conforming code, inter-level transfers, IOPL, I/O bitmap | 20h | done, #124 |
| 6 | V86 mode | 21h | done, #128; follow-ups #129 (ARPL `store_if`) and #131 (128 KB ROM) |
| 7 | Task switching | 22h (128 KB ROM) | next |
| 8 | Remaining test386 (E0h, EEh) | FFh | open; includes the ENTER fault below |
| 9 | Dine: memory above 1 MiB (INT 15h 88h/E801h/87h, CMOS) | — | open |
| 10 | Dine: XMS 3.0 driver (INT 2Fh 4300h/4310h) | — | open |

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

## Current state and handoff — 8 November 2026

Where test386 stops (local Linux, release build, `--steps 100000000`):
- 64 KB ROM: passes POST 00h-06h, 08h, 09h, 20h, 21h, 0Bh-19h and stops in
  POST 1Ah. That test runs ENTER 1,0 at ring 3 with ESP=1004h, the stack on
  a supervisor page (linear 21000h), and expects #PF. PIL's ENTER does not
  fault there; not yet diagnosed (check that ENTER's pushes and its frame
  pointer read translate at CPL 3, in isa/i386/enter.pil and Ram's user
  checks in src/memory.rs).
- 128 KB ROM: passes POST 00h-21h and stops in POST 22h with #GP(00A8h) at
  00C0:0012, in its first task-switch test (why it is #GP(00A8h) and not
  the InvalidContext stop for a TSS target is not yet traced).
- Test counts after #131: 654 Rust tests pass, 1 ignored; the generated-
  backend suite (`pil --compare`, src/native.rs) has 276 cases.
  SingleStepTests/80386 run 10: 1,621,228 of 1,758,700 real-mode cases
  pass, 0 generated-backend disagreements.

Increment 7, task switching, should cover:
- Far JMP and CALL to an available TSS descriptor (types 1 and 9) and
  through a task gate (type 5), in src/privilege.rs (the descriptor check
  near line 128, used by `far_jump` and `far_call`, refuses them today with
  InvalidContext) and src/address.rs (segment
  loads refuse them, line ~258).
- Interrupts and exceptions through a task gate in the IDT
  (src/address.rs `interrupt_target` refuses type 5; src/interrupt.rs
  delivers).
- IRET with NT=1: return to the task in the TSS back link
  (isa/i386/iret_protected.pil stops on NT=1 now).
- The switch itself: save the current state into the old TSS (386 and 286
  layouts), check and load the new TSS (limit at least 67h or 2Bh, #TS with
  the selector otherwise), busy bits (set on the new task; cleared on the
  old one for JMP and IRET, kept for CALL and interrupts), NT and the back
  link for CALL and interrupts, CR0.TS set, CR3 reloaded from a 386 TSS,
  LDTR and all segment registers loaded with the new task's CPL, the error
  code pushed on the new stack when the exception has one.
- A shared Rust module (for example src/task.rs) embedded in the generated
  backend like privilege.rs, so both backends switch tasks the same way;
  new tests in tests/unit (a task.rs file) and at least one generated case
  in src/native.rs.

Then: the ENTER fault above, the rest of test386 (increment 8), Dine's
memory above 1 MiB (9) and XMS (10). The x87 FPU comes after protected mode
(estimate: 9-12 PRs; src/fpu.rs has only the absent-FPU backend).

Key files:
- src/address.rs: SegmentState (caches, CPL, GDT/LDT/IDT/TR, CR0-CR3,
  paging), segment loads, `interrupt_target`.
- src/privilege.rs: far JMP/CALL/RETF/IRET, call gates, TSS stack switch,
  V86 entry. Shared by both backends.
- src/interrupt.rs: exception and interrupt delivery (real, protected, V86),
  #DF and shutdown.
- src/memory.rs: Ram, paging walk, A/D marks, user/supervisor checks.
- src/guest.rs: the x86 decoder, which binds operands and picks a PIL file.
- isa/i386/*.pil: instruction semantics; src/main.rs has the PIL tokens,
  the interpreter and the Rust emitter. docs/LANGUAGE.md lists every token.
- src/dos.rs `run_rom`: the test386 runner (`--run-rom`, 64 or 128 KB).

Conventions for each PR:
- Semantics live in PIL, or in Rust shared by the interpreter and the
  generated backend (embedded with include_str!), never in one backend.
- New behaviour gets its own expected-state and fault tests; both backends
  are compared in src/native.rs. Claim only what was run.
- Branch from dev, PR against dev; Tony merges. Never force-push. No
  Windows reserved device names (CON, NUL, COM1, ...) in branch names.
- Each delivery adds a top entry to CLAUDE.md and
  docs/development/STATUS.md, updates this plan and docs/LANGUAGE.md for
  new tokens, then regenerates docs/SHA256SUMS: after `git add -A`, hash
  every file in `git ls-files` except docs/SHA256SUMS itself, sorted, one
  line `"{sha256}  {path}"` each.
- test386 (GPL), FD14LITE.img and third-party programs stay out of the
  repository.

Running test386 (built in WSL; NASM is easier there than on Windows):
```sh
git clone https://github.com/barotto/test386.asm && cd test386.asm
git checkout cfd052d1
# 128 KB ROM: set ROM128 equ 1 in src/configuration.asm
nasm -i./src/ -f bin src/test386.asm -w-all -o test386-128.bin
cd /path/to/PIL-Processor-Instruction-Language
cargo run --release -- --run-rom ../test386.asm/test386-128.bin --steps 100000000
```
The runner prints each POST code written to port 190h and the stop reason.


## DOS games increment, 10 November 2026

Dine now supplies 16 MiB physical RAM, with the original 640 KiB DOS MCB
arena unchanged. AH=88h and CMOS 17h/18h/30h/31h report the same 15 MiB
extended area; A20 starts disabled. The executable system windows admit
extender-loaded code above 1 MiB. There is no XMS driver or external DPMI host.

Hardware IRQ0/IRQ1 follow the PIC vector into the protected IDT, retaining
IF, instruction-shadow, mask and EOI handling. Operand-size-prefixed I/O
uses the correct width. Mode 13h can switch between chain-4 and unchained
four-plane storage; 320x200 scanout interleaves plane bytes and uses the
CRTC start address for page flips. Other VGA timings stay unsupported.

Local release runs of the user's DOS Doom v1.9 render changing gameplay;
ESC opens the menu through IRQ1. This is not a completed timedemo result.
3DBench 1.0 renders its scene and shows a score (4.4 fps observed in one
capture; host-clock/runtime settings affect it). These files remain local.
The original AH3F stop was an invalid check against requested CX rather than
the actual returned byte count. Motor-off writes to 03F2h are accepted;
other floppy-controller operations remain unsupported.

ENTER now checks write access to its final stack-pointer byte even without
a store there. Paging/segment fault rollback, a successful retry and generated
Rust execution are tested. The locally decoded test386 HEX ROM still stops
at POST 1Ah; its ENTER diagnostic is not claimed fixed. Task switching remains
open. See DOS_GAMES.md for reproduction and limits.
