# AH33 AL=05h boot drive — 30 October 2026

`DIR | MORE` stopped at INT 21h AX=3305h. FreeCOM asks for the boot drive
when no TEMP, TMP or current directory can take a temporary file, which is
always the case while every drive is read-only. Dine now reports its first
drive (C: with an image or a mounted folder, A: without). Pipes still cannot
work until some drive can be written: FreeCOM falls back to running the left
side to the screen and the right side reading the keyboard.

Verification: 412 Rust tests passed on Linux.

# IRET in real mode — 30 October 2026

`MORE` stopped at opcode CFh (IRET) in its own code before printing anything.
IRET and IRETD now run in real mode, in `isa/i386/iret.pil`: the whole IP, CS
and FLAGS frame is checked against SS first, then the far branch, SP and the
flags POPF can write (bits 1, 3, 5 and 15 stay fixed; RF and VM are kept) are
committed together. Protected-mode IRET still stops.

Verification: 411 Rust tests passed on Linux; 252 native verifier cases (5 new:
IRET and IRETD with 16- and 32-bit stacks, and a frame fault); DOSBox-X
2024.03.01 matched PIL on all 522 comparison cases (3 new IRET cases).

# INT 10h EGA/VGA functions on the CGA BIOS, and handle duplication — 30 October 2026

`SETLANG SU` in FD14LITE stopped at INT 10h AH=11h (character generator).
Dine's video BIOS is CGA class on an AT BIOS, which returns from AH=10h-12h
(reserved entries) and from functions past its table without changing
anything. AH=10h, 11h, 1Ah, 1Bh and 1Ch now do the same, as AH=12h and 4Fh
already did. Programs that check AX=1A00h or BL=10h see no VGA and no EGA.

`MORE SETLANG.BAT` stopped at INT 21h AH=45h. AH45 (duplicate a handle into
the lowest free one) and AH46 (force CX to refer to BX's file, closing CX
first) now work. Handles are resolved through their JFT entry, so duplicates
share one file position, and a file stays open until no handle in the running
program or its waiting parents refers to it. Writing to a file handle returns
access denied (5), since every file is opened read-only.

The terminal (`tools/dos_terminal.py`) now passes characters beyond ASCII
that the host keyboard layout produced, such as ä, ö and å, in code page 437,
with the scan code of their key in the Nordic layouts. Characters 437 lacks
(such as €) are still dropped. Since the host already applies the layout, no
DOS KEYB driver is needed. The screen now draws all 256 characters with their
IBM PC glyphs, including ☺ ♥ ♪ ► ↑ for 01h-1Fh and ⌂ for 7Fh, which Python's
cp437 codec leaves as control characters; § and ¶ can be typed as 15h/14h.

Verification: 410 Rust tests passed on Linux.

# EXEC: programs start from FreeCOM, plus FACTOR and MANDEL — 30 October 2026

Step 4 of Goal 1 (COMMAND.COM in the GUI, DIR, factorial, Mandelbrot). In
Tony's screenshots after #59, programs typed at the FreeCOM prompt stopped at
AH29 just before EXEC, and DATE/TIME stopped reading CON through AH3F.

- INT 21 AH4B AL=00h (load and execute), after FreeDOS task.c:
  - COM files and MZ EXE files, with relocations applied at the load segment.
    The program comes from any Dine drive. A missing file gives error 2 or 3,
    a directory error 5, and an unusable header error 11.
  - The child gets its own environment block (the given one or the parent's,
    then the count word and its full path) and program block. A COM file takes
    the largest free block; an EXE takes what fits between its minimum and
    maximum. Error 8 is returned when memory is short.
  - The PSP carries INT 20h, the block end, the parent's return address and
    INT 23h/24h vectors, the parent PSP, a copy of the parent's handles, the
    environment, the 20-entry handle table, the DOS version word, the INT 21h
    RETF stub, both FCBs and the command tail.
  - When the child ends (AH4C, INT 20h, AH31), its memory is freed (all
    but the resident block for AH31) and the files it opened are closed. The
    parent continues after its INT 21h with CF=0. Programs can nest 8 deep.
  - The PSP is no longer fixed at 1000h: AH51/62, AH48 ownership, the handle
    table and termination all follow the running program.
  - AL=01h and AL=03h (load without running, overlays) still stop the run.
- AH4D returns the last child's exit code and how it ended (AH=00h normal,
  03h resident) once, then 0.
- AH29 parses a file name into an FCB as FreeDOS FcbParseFname does: the four
  AL option bits, drive letters (FFh for a drive that does not exist), `*` and
  `?`, `.` and `..`, and DS:SI left after the name.
- AH3F on handles 0-2 while they are still CON reads a cooked line: the same
  editor as AH0A (127 characters), CR echoes LF, and the line with CR LF is
  handed out over as many reads as the program asks for.
- `programs/dos/` has two original demo programs with their NASM sources:
  `FACTOR.COM [n]` prints 0! to n! as exact decimals, and `MANDEL.COM` draws
  the Mandelbrot set in text characters with 8.8 fixed point.

From the FreeCOM 0.86 prompt this now works: running COM and EXE programs from
C: and D:, `IF ERRORLEVEL`, a nested `COMMAND` and `EXIT` back to the first
one, and typing a new date at the DATE prompt (setting it, AH2B, still stops).

  `python tools/dos_terminal.py --mount programs/dos images/FD14LITE.img::COMMAND.COM`

Verification: 408 Rust tests passed on Linux, including EXEC of a COM and a
relocated EXE with their return codes, AH29 cases from FreeDOS's rules, a CON
read split over two calls, and the output of both demo programs. FreeCOM 0.86
was driven through HELLO, an EXE, FACTOR 30, MANDEL, nested COMMAND and DATE
on a test FAT16 image plus a host folder. FD14LITE itself is not in CI.

# Drives C: and D:, DIR in FreeCOM, IDIV and country information — 30 October 2026

Step 3 of Goal 1 (COMMAND.COM in the GUI, DIR, factorial, Mandelbrot). FreeDOS
report 47 (dev after #56 and the DOS data area): 69 Exit, and COMMAND.COM got
past AH52 to AH43, the most common stop (16 programs).

- Dine has drives now, all read-only:
  - `--dos-session IMAGE.img::PROGRAM` makes the FAT16 image drive C:, with its
    volume label, attributes, dates and directory tree. COMSPEC and the program
    path point at C:.
  - `--mount DIR` adds a host folder as the next drive (D:, or C: without an
    image). Names that are not 8.3 get BASIS~N aliases per directory
    (`a-long-file-name.txt` is `A-LONG~1.TXT`); dates come from the host
    modification time in UTC. Symbolic links are not followed.
  - With neither, A: is an empty drive, as before.
- DOS services on those drives: AH4E/4F find (attribute filtering, `.` and `..`
  below the root, label-only searches), AH3B CD, AH47 current directory, AH0E
  and AH19 drive selection (LASTDRIVE E:), AH36 (an image's own geometry; no
  cluster is free), AH43 get attributes, and AH3D/3F/42/3E reading any file.
  Writes (AH39/3A/3C/41/56/5A/5B, AH43 set, AH3D for writing) return access
  denied. AH44/00 gives files their drive and CON bit 6 (not at end of input),
  as FreeDOS opens devices. AX=6900h and AH73 are invalid functions.
- Country 001, code page 437: AH38, AX=6501h, the 6502/04/05/06/07 table
  pointers, AH65 upper-case calls (20h-23h, A0h-A2h) and a far case-map
  routine in Dine's data segment that programs call.
- AH58 accepts the 4xh (upper memory only) strategies and allocates low, as
  FreeDOS does without UMBs. INT 2F AX=AE00h and B700h find no installable
  command and no APPEND.
- IDIV (F6/F7 /7): signed division through magnitudes, in both backends, with
  7 native verifier cases and 15 DOSBox-X comparison cases.

FreeCOM 0.86 now runs interactively: the `C:\>` prompt, DIR (volume label, 8.3
names, `<DIR>`, sizes with thousands separators, totals), CD, switching
between C: and D:, TYPE and VER.

  `python tools/dos_terminal.py --mount dosfiles images/FD14LITE.img::COMMAND.COM`

Verification: 404 Rust tests passed on Linux, 247 native verifier cases, and
DOSBox-X 2024.03.01 matched PIL on all 519 comparison cases. FreeCOM 0.86 was
driven through DIR, CD, C:/D: switching and TYPE on a test FAT16 image plus a
host folder. FD14LITE itself is not in CI.

# Interactive DOS session and text-mode terminal — 30 October 2026

Step 2 of Goal 1 (COMMAND.COM in the GUI, DIR, factorial, Mandelbrot):

- `pil-proof --dos-session PROGRAM` loads a .COM/.EXE, or IMAGE.img::INNER
  from a container, and serves one command per stdin line with one JSON reply
  per line:
  - `run N` executes up to N steps (1..1000000). The run pauses on the step
    budget and when the program waits for a key, and ends at exit or at the
    first unmodelled service, encoding, fault or boundary.
  - `key ASCII SCAN` puts one keystroke in the BIOS keyboard ring.
  - `snapshot` replies without running, and `quit` ends the session.
  - Every reply carries the status, the active B800 text page (character and
    attribute bytes as hex), its width, the BIOS cursor, CS:IP, the step
    count and the program's DOS output for that slice.
- tools/dos_terminal.py is a Tk window for the session. It draws the 80x25 page
  in the 16 CGA colours with the cursor, maps keys to BIOS ASCII and scan codes
  (US layout; arrows, F1-F10, Home/End, PgUp/PgDn, Ins/Del, Ctrl+letter), and
  shows the status, step count and CS:IP. It only displays: execution, Dine
  services and screen contents all come from the engine.

  `python tools/dos_terminal.py images/FD14LITE.img::COMMAND.COM`

Verification: 398 Rust tests passed on Linux (a new session test covers run,
the 15-key ring, exit and malformed commands). tests/gui/test_dos_terminal.py
covers key mapping, CP437 decoding and a live session that waits for keys,
echoes them and exits. The window was also driven under Xvfb with typed keys.

# DOS data area: list of lists and InDOS — 30 October 2026

FreeDOS report 46 (dev after #56, 100,000 steps): 67 Exit, 48 Budget, 33
Service, 3 MissingEncoding. FreeCOM now loads its strings through the
environment block and its own file. It then stops at INT 21h AH=52h, as do 20
other programs. This round:

- Dine's DOS data segment is 0070h. INT 21h AH=52h returns ES:BX = 0070:0026,
  the list of lists, with the first MCB segment in the word before it. That is
  the only field FreeCOM reads (lib/get1mcb.c), and the MCB chain it leads to
  is the real one: environment, program, end. The NUL device header ends the
  device chain and sectors are 512 bytes. Structures Dine does not model (DPB,
  SFT, CLOCK$, CON, buffers, CDS, FCBs) are FFFF:FFFF, and there are no block
  devices or drives yet.
- INT 21h AH=34h returns ES:BX = 0070:0001, the InDOS byte, which is 0. The
  critical-error byte before it is also 0.

Verification: 397 Rust tests passed on Linux. Not yet rerun on the FreeDOS
image.

# SHLD/SHRD, environment block, own-file reads and a growable handle table — 30 October 2026

FreeDOS report 45 (dev after #55, 100,000 steps): 65 Exit, 16 WaitingForInput,
45 Budget, 17 Service, 8 MissingEncoding. The BTC probe is gone. This round:

- SHLD and SHRD (0F A4/A5/AC/AD) by immediate or CL, on registers and memory,
  in isa/i386/shld.pil and shrd.pil. The count is masked to five bits and a
  zero count changes no flags. CF is the last bit shifted out, OF is defined
  only for a count of one, and AF is undefined. A 16-bit count above 16 leaves
  the result undefined on the 386, so it is rejected instead of guessed. All
  four CWSDPMI programs and FDNPKG stopped on `SHLD DX,AX,CL`.
- Environment block: PSP:2Ch points at a 16-paragraph block in its own MCB,
  owned by the program (so AH49 can free it), holding `COMSPEC=A:\COMMAND.COM`,
  the end-of-list NUL, the count word 1 and the program path `A:\NAME.EXT`.
- The program's own file is the one file the probe can open. AH3D with a path
  ending in the program's name opens it read-only on the lowest free handle;
  AH3F reads from it, AH42 seeks (origins 0, 1, 2; DX:AX is the new position)
  and AH3E closes it. Opening it for writing, a second open, and seeking before
  its start stop. Other paths still return file-not-found. Directories are not
  modelled. This was salvaged from the `feature/command-interactive` branch.
- INT 21h AH=67h follows FreeDOS SetJFTSize. PSP:32h/34h now hold the table
  size (20) and a far pointer to PSP:18h. A count that fits updates PSP:32h; a
  larger one allocates a block with AH48 rules, copies the entries, fills the
  rest with FFh and points PSP:34h at it. Without free memory it returns CF=1,
  AX=8. Handle opens and closes go through the pointer. SWSUBST asked for 50.
- DOSBox-X CPU suite is table-driven: 504 cases in a 31,169-byte COM (was
  448 cases in 64,398 bytes, near the FF00h limit). 16-bit SHLD-by-one OF is
  not compared, because DOSBox-X takes it from the source word.

Verification: 396 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 240 cases (new: double_shift with
CL and immediate counts on registers and memory; double_shift_reject for a
16-bit count of 17). DOSBox-X 2024.03.01 headless: CPU suite 504/504, the
negative control fails case 1 in both engines, console 9/9. The Python tool
checks and GUI tests pass. Not yet rerun on the FreeDOS image.

# BT family, true DOS version, default drive and handle count — 30 October 2026

FreeDOS report 44 (dev after #54, 100,000 steps): 63 programs exit and 16
reach a keyboard prompt. This round:

- BT, BTS, BTR and BTC (0F A3/AB/B3/BB) on registers, and the 0F BA /4-/7
  immediate forms on registers and memory, in isa/i386/bt.pil. The bit offset
  is taken modulo the operand width, CF gets the selected bit, and OF, SF, ZF,
  AF and PF are undefined. Register-offset memory forms (bit strings past the
  operand) are still a decode stop. Five programs stopped on `BTC EAX,ESI`, the
  EFLAGS.AC probe that tells a 386 from a 486.
- INT 21h AX=3306h (true version) returns BX=1606h (6.22, the same profile as
  AH30), DL=0 (revision) and DH=0 (not in ROM or the HMA), as FreeDOS
  inthndlr.c does with its own version. Other AH33 subfunctions still stop.
  DEVLOAD stopped here.
- INT 21h AH=19h returns AL=0, drive A:, matching the single floppy in the
  INT 11h equipment word. This is a profile choice; there is no filesystem.
  ASSIGN stopped here.
- INT 21h AH=67h (set handle count) succeeds for BX up to 20, the PSP handle
  table Dine already has. Larger requests stop, because Dine does not move the
  table. SWSUBST stopped here.
- INT 2Fh AX=0100h (PRINT) and AX=B710h (APPEND) installation checks reach
  the kernel default and return AX unchanged.

Verification: 393 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 238 cases (new: bit_test with the
report's BTC EAX,ESI, register and immediate-memory forms; bit_test_readonly
faults on a read-only BTS). DOSBox-X 2024.03.01 headless: CPU suite 448/448
(48 BT-family cases new, COM 64,398 bytes), the negative control fails case 1
in both engines as intended. Not yet rerun on the FreeDOS image.

# One-operand IMUL, XLAT, DOS time and allocation strategies — 30 October 2026

FreeDOS report 42 (dev after #53, 10,000 steps): 84 Budget, 47 Exit, 15 Service,
5 MissingEncoding, and no Fault or Monitor stops. VDELAY, VPAUSE, DISKCOPY and
REPLACE got past their report-41 stops. This round:

- One-operand IMUL (F6 /5, F7 /5) at 8/16/32 bits, register or memory: the
  signed product goes to AX, DX:AX or EDX:EAX, and CF=OF=1 when it does not
  fit the sign-extended low half (SF, ZF, AF, PF undefined). DISKCOPY stopped
  on `IMUL DX`.
- XLAT (D7) in isa/i386/xlat.pil: AL = seg:[(E)BX+AL], DS unless overridden,
  BX+AL wrapping at 16 bits. GREP stopped on it.
- INT 21h AH=2Ch reads the BIOS tick count (one INT 1Ah read, so the clock
  advances one tick) and scales it to hours, minutes, seconds and hundredths
  by 65536*5/59659, the scaling the FreeDOS clock driver uses.
- INT 21h AH=58h/01h now also accepts best fit (1) and last fit (2), with or
  without the 80h high-memory bit (no UMBs, so 8xh allocates low). AH=48h
  follows the strategy: best fit takes the smallest block that fits, last fit
  takes the top of the last block that fits. Upper-only strategies (4xh)
  still stop. SWSUBST stopped on last fit.
- INT 2Fh AX=0600h (ASSIGN installation check) returns AX unchanged.
- INT 15h AH=E8h (E801h/E820h memory maps) returns CF=1, AH=86h, like the
  other functions newer than this AT-class BIOS.
- DOSBox-X CPU suite: each case now calls one shared routine to print
  PASS/FAIL, so the COM shrank from 64,183 to 57,402 bytes with 400 cases.

Verification: 390 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 236 cases (new: imul1 with
8/16/32-bit register and memory forms, imul1 past the segment end, XLAT with
BX, EBX and ES). DOSBox-X 2024.03.01 headless: CPU suite 400/400 (6 one-operand
IMUL, 1 XLAT new), the negative control fails case 1 in both engines as
intended, and console 9/9. Not yet rerun on the FreeDOS image.

# Far JMP, TSR exit, date, BIOS identity bytes and HLT idle — 30 October 2026

FreeDOS report 41 (dev after #52, 10,000 steps): 81 Budget, 43 Exit, 19 Service,
4 MissingEncoding, 2 Fault, 2 Monitor. This round:

- Far JMP: EA ptr16:16/32 and FF /5 m16:16/32 (memory only) in
  isa/i386/jmp_far.pil, real mode, with the pointer read whole before the
  transfer. Report 41 stopped GREP on `JMP FAR [mem]`.
- INT 21h AH=31h ends the run as a TSR exit: the report shows Exit(AL) and a
  RESIDENT line with the paragraphs kept (at least 6, as in FreeDOS). No
  resident code runs afterwards.
- INT 21h AH=2Ah returns 1 January 1980, a Tuesday: the DOS default with no
  clock driver (FreeDOS systime.c computes the weekday as (days+2) mod 7).
- INT 21h AH=0Dh (disk reset) has no buffers to flush.
- INT 21h AH=58h: AL=00h returns the strategy; AL=01h accepts 00h and 80h
  (first fit; without UMBs 80h allocates low), rejects BL>82h or a fit above
  2 with error 1 as FreeDOS does, and stops on best/last fit, which the
  first-fit arena does not model. AL=02h returns link state 0; AL=03h fails
  with error 1 because there is no UMB chain.
- INT 21h AH=71h (long file names) returns AL=00h, CF=1 (AX=7100h), as
  FreeDOS does without an LFN API.
- INT 15h AH=88h returns AX=0: no memory above 1 MB.

- BIOS area (Tony's decision): C000:0000-F000:FFFF now reads as an empty bus
  (FFh, so no option-ROM 55AAh signature) and is read-only. The top 16 bytes
  identify a PC/AT-class BIOS: the reset jump, the date 01/10/84 at F000:FFF5
  and model byte FCh at F000:FFFE. No BIOS code is present. DISKCOPY and
  REPLACE faulted reading F000:FFEA, where they look for "COMPAQ"; they now
  read FFh. RAM grows from 768 KB to 1 MB to hold it.
- HLT (Tony's decision): with IF=1, as DOS starts programs, HLT idles until
  the next timer tick: one tick passes (seen by INT 1Ah) and execution
  continues. VDELAY and VPAUSE idle this way. With IF=0, HLT never resumes and
  stays a monitor stop. DOS unit fixtures that use HLT as an end marker now
  clear IF first.

Verification: 387 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 233 cases (new: far JMP immediate,
memory 16 and 32, pointer past the segment end, real-mode rejection). DOSBox-X
2024.03.01 headless: CPU suite 393/393 (2 far JMP new; the COM is 64,183 bytes,
close to the 64 KB limit) and console 9/9 in both engines. The date and DOS
memory answers are not compared with DOSBox-X, which uses the host clock and
has UMBs and XMS. Not yet rerun on the FreeDOS image.

# LDS, ENTER and absent-extension answers — 30 October 2026

FreeDOS report 40 (dev after #51, 10,000 steps): 79 Budget, 38 Exit, 28 Service,
6 MissingEncoding, down from 25 MissingEncoding in report 39. This round takes
the integer encodings and the services with a documented "not present" answer:

- LDS (C5) shares isa/i386/les.pil, which now takes the destination segment
  register. Report 40 stopped LBACACHE and RECOVER on it.
- ENTER size,0 (C8) in isa/i386/enter.pil: push BP/EBP, set BP to the pushed
  slot, reserve the frame. A non-zero nesting level is still a decode stop.
- INT 21h AH=40h writes CX bytes from DS:DX to the CON handles 0..2, with the
  same screen and capture path as AH=09h, and returns AX=CX. A closed handle
  returns error 6; AUX/PRN (3, 4) still stop as not modelled.
- INT 21h AH=51h returns the PSP in BX, like AH=62h (also added).
- INT 2Fh installation checks AX=1100h, 1600h, 1680h, 1687h, 1A00h and 4300h
  return AX unchanged, which is what the FreeDOS kernel's default multiplex
  handler does (kernel/int2f.asm, inthndlr.c) when no redirector, Windows,
  DPMI host, ANSI.SYS or XMS driver is loaded. AX=4310h (XMS entry point) is
  still a stop, because a caller should only ask after 4300h says installed.
- INT 10h AH=12h and AH=4Fh: the video BIOS is CGA-class, so these calls are
  ignored (BL=10h stays 10h: no EGA; AL is not 4Fh: no VBE).
- INT 15h AH=C0h returns CF=1, AH=86h (no configuration table), like AH=C2h.

Verification: 384 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 228 cases (new: lds16, lds32,
enter16, enter32, ENTER on a read-only stack slot). DOSBox-X 2024.03.01
headless: CPU suite 391/391 (2 LDS, 2 ENTER new) and console 9/9 (AH40 to
STDOUT, AH51/AH62) in both engines. INT 2Fh and INT 10h AH=12h are not compared
with DOSBox-X, which has its own XMS and a VGA BIOS. Not yet rerun on the
FreeDOS image.

# RCL/RCR, DAA/DAS, PUSHA/POPA and indirect JMP — 30 October 2026

The rest of the integer encodings that stop programs in FreeDOS reports 38 and 39:

- RCL and RCR (group 2 /2 and /3: D0-D3, C0/C1) at 8/16/32 bits, register or
  memory, in isa/i386/rcl.pil and rcr.pil. The count is masked to five bits
  and reduced modulo width+1 (CF is the extra ring bit). Flags change only for
  a non-zero masked count; OF only for a count of one (RCL: result MSB XOR new
  CF; RCR: input MSB XOR incoming CF), as the existing ROL/ROR do.
- DAA (27) and DAS (2F) from the Intel SDM pseudocode; OF is undefined.
- PUSHA/PUSHAD (60) and POPA/POPAD (61) on a 16- or 32-bit stack. PUSHA stores
  the original SP/ESP in the fifth slot; POPA skips that slot. PIL now allows
  up to eight staged stores per instruction (was two), so all eight PUSHA
  stores are checked before any commits: a protected or out-of-segment slot
  leaves registers and RAM unchanged.
- Near indirect JMP r/m16 and r/m32 (FF /4) from a register or memory, in
  isa/i386/jmp_indirect.pil. The new offset is zero-extended and checked
  against CS.limit. Report 39 stopped DISKCOPY, RECOVER, REPLACE, SHARE and
  UNFORMAT on `JMP [mem]` jump tables.

The DOSBox-X CPU suite puts identical strings in the COM once, so the 387 cases
still fit in one 64 KB COM.

Verification: 382 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 223 cases (new: RCL/RCR mix, RCL
on read-only memory, DAA/DAS chain, PUSHA/POPA 16 and 32, PUSHA on a read-only
stack slot, POPA at the top of the stack, indirect JMP register/memory/32-bit,
indirect JMP past CS.limit). DOSBox-X 2024.03.01 headless: CPU suite 387/387
(24 RCL/RCR, 10 DAA/DAS, 4 PUSHA/POPA, 3 indirect JMP new) and console 8/8 in
both engines. Not yet rerun on the FreeDOS image.

# LOOP family, unsigned MUL and INT15 AH=C2 — 30 October 2026

FreeDOS probe report 38 (current dev, 10,000 steps) stopped 16 programs on
missing encodings and 12 CTMOUSE builds on INT15 AH=C2. This adds:

- LOOP, LOOPE/LOOPZ and LOOPNE/LOOPNZ (E2, E1, E0) in isa/i386/loop.pil. The
  address size picks CX or ECX; the counter is decremented and written whether
  or not the branch is taken; flags are preserved. A taken target outside CS
  faults before the counter is written.
- Unsigned MUL F6/F7 /4 at 8/16/32 bits, register or memory: AX, DX:AX or
  EDX:EAX receive the product; CF=OF=1 when the upper half is non-zero;
  SF/ZF/AF/PF are undefined.
- INT15 AH=C2 (PS/2 pointing device) answers CF=1, AH=86h "function not
  supported", as a BIOS without PS/2 mouse support does (SeaBIOS behaviour).
  No mouse is modelled; Dine does not pretend one exists.

Verification: 380 Rust tests passed on Linux. The generated-Rust verifier
matches the interpreter byte for byte across 214 cases (seven new MUL/LOOP
cases, including a memory-operand fault and a taken-target fault). The DOSBox-X
CPU comparison grows to 346 cases (12 MUL, 6 LOOP) and passes in both engines.
Not yet rerun on the FreeDOS image.

# Code runs anywhere in the program's DOS block — 30 October 2026

The DOS loader's instruction fetch window was the bytes loaded from the file.
Self-unpacking COMs (UPX-style stubs) copy code above the file image and jump
there, so 36 small FreeDOS COMs stopped at Boundary. The window is now the
memory block DOS gives the program at load: PSP through the load-time
allocation end (all conventional memory for a COM; the MZ minimum/maximum
block otherwise). A fetch outside that block, such as a far call into ROM,
still stops as Boundary. The decode cache already re-checks instruction bytes
on reuse, so code written by the guest is decoded fresh.

Verification: 378 Rust tests passed on Linux (one new, two adjusted). The
DOSBox-X CPU (328) and console (8) comparisons still pass in both engines.
Not yet rerun on the FreeDOS image.

# DOS buffered line input (INT21 AH0A) — 30 October 2026

INT21 AH0A reads a line into the DS:DX buffer (size byte, count byte, text).
Keys come from the BIOS ring and are echoed to capture and the active page.
With no key the call waits without retiring and keeps the partial line, so a
line can be typed across several runs. A size byte of 0 returns at once; the
whole buffer must lie inside DS before any key is taken. Up to size-1
characters are kept; a key past that echoes BEL and is dropped. CR is stored
and echoed alone (no LF); the count byte excludes it. LF echoes CR LF (not as
the first key) and is not stored. BS and Left erase the last character on
screen: two columns for a ^X control character, back to the tab stop for TAB.
ESC echoes `\`, CR LF and indent spaces, then restarts the line. F6 stores
^Z. Control characters echo as ^X; ^F and other extended keys are ignored.
The DOS template keys (F1-F5, Right, Ins, Del) stop with "not modeled", and
Ctrl-C at the head of input stops as for the other break-checking reads.
AH0C/AL0A now flushes type-ahead once, then runs AH0A. Behaviour follows the
FreeDOS read_line (chario.c at d6791add); no kernel code is copied.

Verification: 377 Rust tests passed on Linux (five new). The console COM
fixture has eight cases, adding a stuffed-keys line edit (BS) and a full
buffer (BEL). In DOSBox-X 2024.03.01 (headless) all eight console cases pass
and the full output, including the echoed lines, is byte-identical to PIL;
the negative control fails exactly case 001 in both.

# DOS console input and screen-mirrored output — 30 October 2026

DOS stdin/stdout are CON. INT21 AH01 (echo), AH07 and AH08 read the BIOS
keyboard ring; with nothing typed they stop with WaitingForInput without
retiring and resume after Loaded::enqueue_key. An ASCII-0 key returns 00h,
then its scan code on the next read. AH01 echoes every byte it returns,
including that 00h, as the FreeDOS handler does. AH06 DL=FF polls: nothing
ready gives ZF=1/AL=0 without consuming; otherwise ZF=0 and the byte. Other
AH06 DL values output like AH02. AH0B returns AL=FF when input is ready, else
00. AH0C flushes type-ahead (and any owed scan code), then runs AL=01/06/07/08;
other AL values return 00h. A waiting AH0C read flushes once before its first
wait, so the key that ends the wait is kept. AH0C/AL0A is rejected until
buffered input (AH0A) exists.

Ctrl-C (03h) at the head of input stops AH01/02/08/09/0B before any effect,
because DOS break handling (INT23) is not modeled; AH06/AH07 return it as 03h.
Behaviour follows the FreeDOS kernel handlers (inthndlr.c, chario.c at
d6791add); no kernel code is copied.

DOS output from AH02, AH06 and AH09 now also teletypes onto the active text
page, as DOS console output does through the BIOS. The captured output is
unchanged. AH09 TABs expand to the next eight-column stop on the screen. The
screen write set is checked before capture, so a protected cell, bad cursor or
full provider changes neither capture nor RAM.

Verification: 372 Rust tests passed on Linux (six new). The console COM fixture
has seven cases, adding DOS status/read/extended-key/poll/flush; all pass in
PIL and the deliberate failure still fails once.

DOSBox-X comparison (Ubuntu dosbox-x 2024.03.01, headless SDL dummy driver):
all 328 CPU cases and all seven console cases pass in both engines; the
negative control fails exactly case 001 in both. Linux CI now installs
dosbox-x and runs both suites. The comparison found that BIOS AH0E writes to
the active page and ignores BH (as SeaBIOS also does); PIL now does the same.
BEL is not in the COM fixture: DOSBox-X 2024.03 advances the cursor on BEL,
while IBM and SeaBIOS BIOSes do not, and PIL keeps the IBM behaviour.

# BIOS teletype, scrolling, mode and page services — 30 October 2026

INT10 AH0E writes AL at the active page's cursor (BH is ignored, as in SeaBIOS
and DOSBox-X), keeps the cell attribute and handles
BEL (no write), BS (left, no erase, stops at column 0), CR and LF. Column 80
wraps; leaving row 24 scrolls that page up one line and fills the new line with
the attribute under the incoming cursor. TAB is rejected: IBM-derived and
DOSBox-derived BIOSes differ, so no policy is chosen silently.

AH06/AH07 scroll the inclusive CH,CL..DH,DL window on the active page up/down
by AL lines with fill attribute BH. AL=0 or more lines than the window clears
it; corners beyond 24,79 are clamped and an inverted window changes nothing.
AH05 selects active page 0..7 (BDA 0462, start offset 044E). AH0F returns
AH=80, AL=mode and BH=active page. AH00 accepts only mode 3: it resets the
mode-3 BDA fields, all eight cursors and the 0607 cursor shape, and clears all
pages unless AL bit 7 is set. Registers are preserved. Other modes are rejected.

Every service checks its full write set (cells and BDA) before mutating; faults,
bad pages and bad cursors change nothing. No CRTC/VGA ports are modeled. DOS
AH02/AH09 output still goes only to the capture provider, not to the screen.

Verification: 366 Rust tests passed on Linux (seven new service tests). The
console COM fixture grows from four to six cases (teletype controls/wrap/scroll,
active page query and window scroll); all six pass in PIL and the deliberate
readback failure still fails exactly once. Not yet rerun in DOSBox-X.

# DOS BIOS console foundation — 29 October 2026

INT10 AH08 now reads actual character/attribute data from B800:0000; AH09
writes repeated character+attribute and AH0A preserves attributes. The fixed
mode-3 profile has eight 4 KiB pages of 80x25 cells. Writes leave the cursor
unchanged and reject invalid pages, coordinates and visible-page overflow.
The whole write set is checked before mutation. Backing RAM extends to C0000;
the DOS MCB allocator and conventional memory size remain capped at A0000.

INT16 AH00/01/02/05 uses the classic BDA ring (15 usable scan/ASCII words).
Peek leaves the queue unchanged; empty peek sets ZF. An empty blocking read
returns WaitingForInput without retiring the INT. A host adapter can call
Loaded::enqueue_key and resume the returned Loaded state. Queue overflow,
invalid BDA pointers and protected memory have regression coverage. Enhanced
keyboard translation, IRQ1 and keyboard-controller ports are not implemented.

This is a backend foundation, not a visible interactive DOS console. DOS output
capture remains separate. Teletype, scrolling, mode switching, active-page
selection and Python display/input wiring are follow-ups. No ANSI.SYS behavior.

Verification: 359 Rust tests passed on Windows. Four original COM console groups
passed in both DOSBox-X (386/normal) and PIL; a deliberately wrong readback
produced exactly one failure in each. Existing 328 CPU COM cases also passed in
both engines. This does not establish physical 386 or full BIOS compatibility.

# PIL 0.34.0-alpha.1

- Add bounded DOS INT21/AH02 character output. Preserve flags and registers
  except AL; return the emitted byte, or20h for a tab. Expand tabs to the next
  eight-column stop derived from captured output. Check the entire expansion
  against the16KiB cap before changing output or AL. No keyboard/break checks,
  redirection, text-screen model or BIOS services are added.
- Add INT21/AX3700 switch-character query: AL=0, DL='/', preserving upper
  register bits and flags. This fixed DOS6.22 compatibility profile rejects
  unsupported AH37 subfunctions instead of claiming a mutable DOS setting.
- Add real-mode LES C4 r16,m16:16 and r32,m16:32 through shared PIL. Address
  and operand sizes are independent. Check the full pointer span and read both
  fields through the incoming source segment before committing the GPR and ES
  selector/cache. Read or segment faults preserve CPU, RAM and machine markers.
  Protected/V86 loads and LDS/LFS/LGS/LSS remain separate.
- Report headers now identify the runtime version and per-program step budget.
  Nine new expected-state tests and14 backend cases. Windows/Linux CI40624671136
  passed328 Rust tests,199 full-state cases (7,330,906 matching trace bytes),
  standalone runtime/native packages and full256x256 Mandelbrot comparisons.
- Targets the user's100000-step v0.33 report: AH02(20), AH37(16), LES(6).
  No new full FreeDOS-image, ARM or legacy real-mode hardware run is claimed.

# PIL 0.33.0-alpha.1

- Add DOS INT21/AH25 and AH35 using the physical RAM interrupt-vector table.
  AH25 stores DS:DX; AH35 returns ES:BX and updates the ES cache. Preserve flags
  and upper register halves. Check the complete four-byte access before effects.
  Initial vectors are zero; no kernel handlers or CPU interrupt delivery added.
- Add ordinary-RAM memory XCHG86/87 at8/16/32 bits through shared PIL tokens.
  Address and source use incoming registers, including overlapping BX/BH and
  ESP addressing. Segment/address sizes remain independent. Faults commit no
  RAM/register/retirement effects. Explicit LOCK is accepted for memory XCHG;
  register LOCK and other LOCK instructions remain rejected. The transaction
  is indivisible in the single-executor RAM model; no MMIO/DMA/bus simulation.
- Add seven expected-state tests and ten interpreter/generated backend cases.
  Hosted Windows/Linux CI40621030314 passed319 Rust tests,185 full-state cases
  (7,224,306 matching trace bytes), standalone runtime/native-package checks and
  full256x256 Mandelbrot comparisons on both hosts.
- This increment targets the reported AH35 blockers (26 programs), FreeCom AH25
  and GREP memory XCHG. No new full FreeDOS-image, ARM or legacy hardware run.
  The separate v0.32 10000-step report exhausts FreeCom's budget before far CALL;
  it must not be compared directly with the earlier100000-step measurements.

# Real-mode far CALL verified — 27 October 2026

Version 0.32 implements immediate 9A and memory FF /3 at 16/32 operand widths,
independent pointer addressing and SS sizes. Shared PIL now permits at most two
ordered staged RAM writes. Loads precede staging; both writes and all branch/
binding checks precede commit. Pointer aliases and second-slot faults are tested.

Local Windows: 312 tests passed; 175 full-state cases matched 7,147,554 trace
bytes. Standalone 110-file runtime built and passed debugger, generated-Rust,
DOS hello and ZIP checks. Hosted Windows/Linux CI 40590192023 passed on code
commit fd1441eb12c1c7f496fa3a742a2fa16c2a2ca23f, including native packages and full Mandelbrot checks.

Actual FD14LITE.img (SHA256 f539d456b792594bc3ca59d4e0f4c23d4f1fee73370c1390b2da245400718d36),
budget 100000: all 16 FreeCom FF /3 blockers reach DOS AH=25. GREP passes 9A and
reaches memory XCHG 87. Missing-encoding programs 21 -> 5: 2 0FBB, 1 0F01, 1 60,
1 87. Imports 151/156, exits 6. No third-party image/program files uploaded.

This is ordinary real-mode support: protected/V86 calls, gates/tasks and
exception/shutdown delivery remain separate. Wide CS padding is deterministically
zero, consistent with existing segment PUSH; no new legacy hardware padding
evidence is claimed. Frames are contiguous as required by the current RETF
profile; split-frame wrap is rejected. No new legacy real-mode/ARM execution.
Keep full development on dev and strict runtime/main delivery PRs. User
authorizes uploads and reviews/merges delivery PRs.

# Memory PUSH verified — 27 October 2026

Version 0.31.0-alpha.1 adds FF /6 m16/m32 through existing shared PIL effects.
Source EA sees old ESP; only the stack write uses SS after decrementing SP/ESP.
Operand, address and SS widths remain independent; faults commit no effects.

Local Windows: 303 tests passed; 160 full-state backend cases matched
6,951,410 trace bytes. The standalone 109-file export built and passed launch,
generated Rust, debugger, DOS hello and ZIP list/probe/extract checks.
Hosted Windows/Linux CI 40587290825 passed on code commit 5eb100818cc0b244f91dfccb6bf3046ae52bf143.
No new legacy real-mode hardware vectors or ARM execution are claimed.

Actual local FD14LITE.img, budget 100000: all sixteen FreeCom variants advanced
from FF 74 14 (PUSH /6) to FF 1E 1F 02 (far CALL /3), twelve bytes later.
PUSH retirements rose 796 -> 812. Imports remain 151/156 and exits remain six.
The first unsupported opcode counts remain 16 FF, 2 0FBB, 1 0F01, 1 60, 1 9A.
Read detailed group extensions and CS:IP; unchanged FF totals hide this progress.
The disk image and third-party program files stay outside the repository.

Keep development on dev and strict runtime delivery PRs against main. User
standing authorization covers project uploads; user reviews/merges deliveries.

# Memory POP, MSW and DOS version increment — 27 October 2026

Based on dev 28bf3088 (RETF) and the user's v0.29 corpus summary.
Adds 8F /0 memory POP with post-pop ESP address evaluation, explicit MSW/SMSW
word forms, and DOS AH=30 through PSP:0040 with a declared 6.22 shim profile.

Windows/Linux CI 40582461490 verifies code commit
a0d578c4347fa4f182e4e93c53f12d7405e70580: 297 Rust tests per host,
149 interpreter/generated full-state cases and 6,870,394 matching trace bytes.
Standalone runtime export, package and full Mandelbrot checks passed.
Local execution was unavailable because the host rejected its sandbox path;
these results come from hosted CI.

SP-wrap/ESP addressing is rejected where behavior is processor-family-specific.
SMSW r32 and CR0 writes remain separate work. No full FreeDOS image or new
legacy real-mode hardware/ARM execution is claimed. Existing modern-host native
overlap checks do not independently verify the newly added forms.

User standing authorization covers project uploads. Keep development on dev
and runtime deliveries as separate main PRs; merging remains the user's action.

# RETF real-mode increment — 27 October 2026

Based on dev1195a351 (0.28alpha.2,Windows/Linux CI40570140583).
Adds CB/CA RETF16/32 via shared PIL far_branch and check_segment effects.
CS/IP/SP commit together after both slots are checked/read;SS size is
independent,cleanup unsigned bytes,flags/markers preserved. New CS uses ordinary
real-mode caches and limitFFFFh. Protected/V86 returns are rejected.
Image fetch bounds retain their initial physical anchor across CS aliases;
generated dispatch/next-IP/limits use physical instruction identity and live
logical flow. Byte guards and bounded imported windows remain enforced.

Local Windows:285 tests passed,135 full-state backend cases (6,743,574 matching trace bytes). Original tests
cover sizes,selectors,padding,cleanup,wrap,readonly reads,SS/RAM/target faults,
mode rejection,CS-alias execution and original DOS process exit after RETF.
Standalone106-file runtime and unpacked Windows native-package checks passed.
Full256x256 Mandelbrot matched65,536 pixels and256 state checkpoints.
No new external real-mode hardware vectors/ARM execution/full FreeDOS rerun.
User 0.28alpha.2 report at100000 steps:151/156 imports,6 exits;31 CB blockers,
1SMSW group and1memory POP. This is user-reported evidence from the pasted
summary,not a locally analyzed whole report. RETF may expose further loaded-
window/service/encoding stops;do not claim31 new successful applications.

# Real-mode CS writes — 27 October 2026

Based on upstream dev15fb5b12 (0.28 comparison strings and memory SETcc).
Ordinary monitor and DOS CS caches now retain SegmentCache::real_mode data
permissions instead of being mislabeled protected Code caches. Generic protected
Code permission checks are unchanged. Segment limits, RAM write protection and
loaded-image fetch boundaries remain enforced. The interpreter invalidates
modified cached instructions on reuse; generated fixed images retain code guards.

Local Windows x86-64:277 Rust tests passed (16.64s),121 full-state backend cases
matched6,594,404 trace bytes. COM and MZ original fixtures modify a later INC AX
to INC CX and execute it; a separate cache fixture primes that instruction first.
Additional SETcc tests cover all16 predicates,mixed flags,BP/SS,FS,SIB,ignored66,
exact byte writes,readonly and truncated/invalid forms. Existing native hardware
oracle ran with the suite; it does not independently verify new CS semantics.
Standalone105-file runtime build, scripts, generated comparison, COM/MZ hello
and ZIP list/probe/extract checks passed locally. No new ARM run or full FD14LITE.img rerun:local image materialization was blocked
by network access. Prior corpus measurements remain upstream evidence only.

# Containers and actual FreeDOS image verified — 27 October 2026

Windows/Linux CI40470152015 atab63a6ec01b570bf29f5251e89f9f819b4fdba37
passed210 tests per host,independent FAT16/ZIP interoperability,standalone86-file
source/native ZIP list/probe/extract,existing49 backend cases(4,983,058 bytes),
340,920 native CPU cases and full256x256 Mandelbrot comparisons.

Compiled Linux artifact04914f4b read/extracted actualFD14LITE.img:2,147 files,
all688 original and1,459 package files byte-identical to independent Python.
Codec/container code is unchanged inab63a6ec. Correctedab63a6ec runtime directly
probed the actual image:208 occurrences->156 distinct;151 DOS imports,5LE
rejections;138encoding gaps,5services,7model faults,1successfulVCHKBOX exit.
Source image unchanged. The image was not uploaded toGitHub;only local runs.
Detailed measured blockers and next priorities:docs/development/FREEDOS_PROBE_RESULTS.md.

Prepare strict0.20alpha.1 runtime PR. This follow-up only records evidence/docs;
execution/export/package logic matches testedab63a6ec. No ARM run claimed.
User's extracted corpus and report are saved separately;do not add third-party
FreeDOS program/source packages to the git repo. All project uploads authorized;
user reviews/merges delivery PR. Chat placement in the PC project is unnecessary
for continuation:keep code/docs/handoff ondev and runtime projection onmain.

# Wrapped MZ entry correction from the FreeDOS corpus — 27 October 2026

The0.19 native CI runtime probed all156 unique extracted executables locally:
138 imported,125 missing encodings,5 service stops,7 faults,1 successful exit
(VCHKBOX),13 mouse-driver MZ entry rejections and5 expected LE rejections.
Inspection showed all13 mouse drivers use relativeCS=FFF0,IP=0100. Their
relocated physical entry is valid;MZCS/SS must wrap as16-bit additions before
entry/stack validation. Corrected this and the code-window start;added a
synthetic wrapped-segment hello regression. Expected210 tests now;CI pending.
The baseline first blockers were1E(30),FD(22),06(18),FB(16),0E/8E/E8/FC(8each),
8C(4),16/54/C7(1each);these are observed first blockers,not complete ISA usage.
Baseline was original0.19 code,not a pass of the new container reader.

# FAT16/ZIP container support — 27 October 2026

0.20alpha.1 adds original bounded FAT16 MBR/superfloppy reader and ZIP stored/
deflate reader with CRC32 and local/central checks. DEFLATE is original Rust
from RFC1951;no third-party codec/crate/runtime Python dependency. --dos-list
INPUT and --dos-extract INPUT NEW_DIRECTORY inspect/extract;--dos-probe and
REPLdosprobe expand executable candidates plus one ZIP package level and
remove byte-identical container duplicates. Inputs remain read-only;path,
chain,CRC,counts/size/depth and new-destination constraints are explicit.
Supported subset and omitted FAT12/FAT32/ISO/ZIP64/encryption are documented.

Expected209 Rust tests (9 new),existing49 backend/native cases unchanged.
New independent Python CLI script builds synthetic FAT16 plus streaming ZIPs
with data descriptors;tests verify source immutability,byte-equal extraction,
dedupe,overwrite/path rejection. Stored/fixed/dynamic/multiblock DEFLATE vectors
were independently generated with Pythonzlib.86 runtime-source files expected.
Local Python syntax/export/links checked;Rust/hosted execution pending CI.

User supplied32MiB FD14LITE.img was separately read/extracted with Python:
FAT16partitionLBA63;688 original files,65 package ZIPs;208 executable copies,
156 unique byte streams(58COM,93MZ,5LE). CRCs ofall65 archives passed.
ImageSHA256f539d456b792594bc3ca59d4e0f4c23d4f1fee73370c1390b2da245400718d36.
FreeDOS data and packages stay outside the git repo;only original synthetic
fixtures/source are committed. The extracted user bundle is separately saved.
No native Rust reading of the user's image is claimed before it is tested.

User authorizes all project uploads. Publish dev,complete Windows/Linux CI,
then strict runtime delivery PR frommain;user reviews/merges. Latest runtime
main includes0.19 plus the debug/release documentation correction.

# DOS probe verified — 27 October 2026

Windows/Linux CI40460972486 passed at11dd1caed36b786a447f4ff055be0bc073d8b700:
200 tests per host;independent CLI checks passed COM/MZ hello,relocated startup
blocker,bounded loop and overwrite refusal. Both standalone83-file source and
unpacked native delivery ran both original DOS hello examples to Exit0.
Existing49 backend cases still matched4,983,058 trace bytes;340,920 existing
native CPU cases passed per host. Full256x256 Mandelbrot comparison passed.
DOS probe is interpreter-only plus explicit host INT21/09,4C andINT20 bridges;
these results are not DOS/BIOS boot or general CPU interrupt verification.

InitialCI40460839517 failed only on a byte-literal cast syntax error in a new
test;fixed before the successful run. Execution code otherwise unchanged.
This follow-up changes evidence/docs only;tested loader/source matches11dd1cae.
Prepare separate0.19alpha.1 runtime PR frommain;user reviews/merges.
The report ranks first observed encoding blockers,not complete missing-ISA
percentages. Next useful family depends on the user's real COM/EXE corpus;
MOV segment registers,PUSH/POP,CALL/RET and further DOS services are likely
startup blockers but must be measured rather than assumed completed.

# DOS executable import/probe — 27 October 2026

0.19alpha.1 adds bounded COM and MZ/ZM EXE import,word relocations,min/max
allocation checks,minimal PSP,COM/MZ entry state,and a diagnostic interpreter
runner. --write-dos-hello com|exe PATH creates original fixtures;--dos-probe
[--steps N] PATH... reports first blockers and reached mix. REPL dosprobe
runs in isolation. DOS INT21/AH09,AH4C and INT20 are explicit host bridges,
not CPU INT/IRET or a DOS kernel. CPU gaps,service gaps,faults,boundaries and
budgets are separate;no unknown-byte skipping/static data scanning/coverage
percentage. An independent relocated-startup fixture stops on MOV DS,AX8E D8.

Expected200 Rust tests (12 new),existing49 backend cases and340,920 native
cases unchanged;DOS probing has no generated-Rust execution path yet.
Independent CLI script plus standalone/export/native-package hello checks added.
83 runtime files expected. Local Python syntax/export/Markdown checks passed;
Rust execution and hosted Windows/Linux checks are pending. No new native
DOS/386 hardware-oracle pass is claimed. No external executables/code imported.
Sources/limits/commands are in docs/DOS_PROBE.md.

PR21 merged:main3051f190 is0.18alpha.1. User phone Android/aarch64 reported
49 cases matching4,983,058 bytes includingbyte_flags7steps. Generated0.336038ms
(89,284,545guest ins/s),cached7.192ms(4,171,696/s),21.40x at10000/5.
Generated build23.544s excluded;nativeCPU baseline unavailable onARM as expected.
This is user-reported correctness/timing,not native386 validation orcycle data.

Publish dev and complete CI;prepare a strict runtime PR from currentmain.
User authorized project source/tests/docs/runtime uploads;user reviews/merges.

# LAHF/SAHF verified — 27 October 2026

Windows/Linux CI40446250754 passed atf2056e697e8d5dff5d5001ffb3f0176e7a8e64ab:
188 tests and340,920 native CPU cases per host,64 variants,40 known answers.
LAHF/SAHF hardware forms actually executed on both hosts. All49 backend
cases matched4,983,058 trace bytes. Python20,480 status-byte cases passed,
as did existing oracles,full256x256 Mandelbrot and standalone81-file source
builds/unpacked native packages. No ARM execution claimed for this increment.
Runtime now includes the current PIL reference and links the full docs overview.

Prepare a separate0.18alpha.1 delivery branch based on main;keep tests,CI and
history ondev. User reviews/merges. This follow-up only records evidence/docs;
execution source and export/package logic match testedf2056e69.
Next instruction candidates remain RCL/RCR (count/CF policy),direction flag
support and further P0 stack/ordered-effects work;do not claim them implemented.

# LAHF/SAHF increment — 27 October 2026

0.18.0-alpha.1 implements LAHF9F and SAHF9E with implicit AH bindings and
ordinary shared PIL tokens. LAHF forces byte bits1/3/5 to1/0/0 and preserves
flags/markers;SAHF writes five status flags,clears only their markers,and
preserves OF and registers. Prefix/fetch-fault and all-state emitted tests added.
Python20,480 independent cases passed locally. Expected188 Rust tests,49
backend cases,340,920 native cases/64 variants/40 harness checks when the
host CPUID LAHF/SAHF feature is available. Missing host feature skips and
reports only those native forms;guest support remains. CI execution pending.

Current reference docs/LANGUAGE.md and documentation map docs/README.md
were corrected/uploaded before this increment. Runtime export now includes
PIL_REFERENCE.md under docs;native packages include it at their root.
81-file runtime source expected. Source/native packages remain isolated.

PR20 merged;main4d6ca2b3 is0.17alpha.2. Prepare a separate0.18 runtime PR
only after Windows/Linux checks. All project uploads are user-authorized;
do not merge dev into main. User reviews/merges runtime delivery PRs.

User reported0.17alpha.2 source80162c46 passing332,728 hardware cases on
Windows/x86_64 AuthenticAMD CPUID1 EAX00a20f10,ECXfed8320b,EDX178bfbff,
rustc1.99.1. Fingerprint01164feba93c1f9c;selected test1 passed,183 filtered.
48 backend cases matched4,949,650 bytes. Generated median0.3087ms vs cached
interpreter3.537ms at10000 iterations/5 samples:11.46x. Generated compilation
5.686s excluded;correctness/timing evidence is user-reported,not386 cycles.

# CMC verified — 27 October 2026

Windows/Linux CI40443065185 passed at7bdfc1b0b27e87f9a897d9a3c66f86da3d2f3a2a:
184 tests and332,728 native CPU cases per host,62 encoding variants,36 known
answers. All48 backend cases matched4,949,650 trace bytes. Independent Python
rotate/carry327,728 and exchange131,216 cases passed. Full256x256 Mandelbrot,
standalone78-file runtime builds and unpacked packages passed on both hosts.
No ARM execution claimed for this increment. Incoming-CF diagnostic handling
is a concrete PIL policy,not symbolic uncertainty propagation.

Update existing runtime PR20 with0.17.0-alpha.2 (XCHG plus CMC),retaining a
strict runtime tree. Tests,CI and history remain ondev; user reviews/merges.
This follow-up changes evidence/docs only; execution source matches7bdfc1b0.
User has authorized all project source/tests/docs/runtime uploads to this repo.

# CMC increment — 27 October 2026

0.17.0-alpha.2 adds CMC(F5) using read_flag/not/flag in a shared PIL
file; no backend-specific semantics. Incoming concrete CF is complemented;
the defined write clears its diagnostic marker, consistent with ADC/SBB.
All other flags/markers, registers and RAM are preserved. Prefix/readonly,
fetch-fault and generated-state tests added; native harness gains CMC with
both carry values and two metadata states (expected 332,728 cases,62 variants,
36 known answers). 184 Rust tests expected; CI execution pending.
Python rotate/carry checker now includes49,152 CMC cases plus the previous
278,576 cases; Rust and native verification are separate.

Previous XCHG source e09b0c9bd7c47f16ba5c7a5e0c68a84b8acd0ea6 passed
Windows/Linux CI40427921270:182 tests,329,896 native cases/61 variants,
34 harness checks,48 backend cases/4,928,770 trace bytes,full256x256
Mandelbrot and standalone77-file runtime/packages on both hosts.
User explicitly authorized GitHub uploads after the earlier automated
approval rejection. Publish dev,verify CI,then prepare a strict runtime PR
from main; do not merge dev into main or merge the delivery PR for the user.

# Revision history

Every delivered revision receives a version, feature/change summary, test
status, provenance update and source fingerprint. User-reported results are
recorded separately from checks executed in the authoring environment.
Early revisions used archive snapshots; current revisions are tracked in Git.
Dates use the user's Europe/Helsinki calendar date.

| Revision | Date | Changes | Verification evidence |
|---|---|---|---|
| 0.1.0 | 2026-10-26 | ADD, typed tokens, Rust interpreter/emitter, register views | User's Termux: 7 passed, 0 failed, 0.67 s |
| 0.2.0 | 2026-10-26 | SUB/CMP/ADC/SBB, incoming carry, expanded tests, provenance register | User's Termux: 14 passed, 0 failed, 2.57 s; independent Python: 657,810 cases plus preservation checks |
| 0.3.0 | 2026-10-26 | Flags, AND, SETcc, conditional writes | User Termux: 20 passed twice, 3.30 s and 3.39 s; installed at ~/pil_proof |
| 0.4.0 | 2026-10-26 | EIP/Jcc, flow context and protected-limit result; in-place patch | 28 Rust tests supplied; Rust verification pending; 12,288 independent Jcc/context cases passed |

The durations above are reported test execution times, not throughput
benchmarks or total compilation times. An emulator performance claim cannot
be inferred from them.

## Fingerprints and evidence

revision_history.json records SHA-256 fingerprints of the original v0.1/v0.2
archives as retained in this workspace. SHA256SUMS records every file in this
release except SHA256SUMS itself, using relative paths. The package cannot
contain its own final archive checksum without a self-reference; earlier
archive hashes are recorded when preparing the next release.

User test messages are the evidence for the two Termux results. No device log
file, hardware CPU comparison or independent CI run is claimed. The Rust
version the user reported before testing was rustc/cargo 1.99.1.

## Update procedure

For each change, update CHANGELOG.md with what changed and why; update
REFERENCES_AND_CREDITS.md for new materials or imported code; update this
history and revision_history.json with exact verification status; generate
SHA256SUMS after final edits. Record later user test results against the
matching version without silently treating a previous release's pass as a
pass for new code.

Snapshots in the earlier archives remain identifiable by their recorded
hashes. Future source imports must also record upstream commit revisions
and local modifications; a release checksum is not a replacement for that.

## v0.4 patch delivery

Apply inside the existing ~/pil_proof. The installer checks implementation
hashes before writing, merges documentation/history and regenerates
SHA256SUMS. Cargo.lock, target/ and user files are not patch targets.
Tests remain inline; move them into a tests/ subdirectory later.
Serial mock and interactive CLI debugger are the next milestone for
the TTGO ESP32 PC emulator. New Rust tests remain unexecuted here.

## v0.5 serial/debugger and repository cleanup

Version 0.4 is user-verified: 28 Rust tests passed in Termux in 4.30 s.
Version 0.5 supplies 46 Rust tests (28 existing + 18 serial/debugger tests).
The new Rust has not been compiled in this authoring environment. Python
arithmetic/flow checks passed after relocation. Tests are now under tests/unit/.
This patch adds no Git remote and imports no third-party source.

## v0.6.0 — 26 October 2026

Baseline: GitHub main commit cfd4143512497c8afd28df9ef6739a96fc62b9f0.
User-confirmed v0.5 result: 46 passed, 0 failed, 4.27 s; REPL starts and the echo
wait/injection/execution sequence was observed. Final TX drain was not shown.

v0.6 adds flat RAM load/store/RMW, deferred single-store commit and diagnostic
access faults, memory debugger commands and 19 tests (65 total). New Rust has
not been compiled or executed in the authoring environment. Source checksums
and independent Python checks are separate from a Rust test result.

User-confirmed Termux verification, 26 October 2026: 65 passed, 0 failed,
0 ignored, 0 measured, 0 filtered out; finished in 9.47 s. This confirms
the Rust suite on the phone; the manual memory REPL smoke test is not yet reported.

## v0.7.0 — development, 26 October 2026

Based on merged PR #1, main commit 13ea986c53384cdfd9103cc346c9d32ebeed8881.
Adds decoded effective addresses and cache-based segment checking, a limited
binary monitor, executable selftest, serial console echo, build/debug scripts
and native delivery packaging. User requested dev as the development branch,
main for applicable user deliveries, and browser approval followed by pull.

92 Rust tests (27 new), including 153 segmented generated/interpreted cases.
Core commit 56f194fd031bed263b1b9f5ce5dc9b09f7abdb28 passed on hosted Ubuntu
(5.88 s) and Windows (11.42 s). Selftest passed 63 executed steps on both.
Build/debug scripts and native package creation passed in CI run 40318741977.
Final packaging additionally validates the unpacked launcher and serial console.
Termux v0.7 and interactive manual checks remain unreported. No local Rust
compiler exists in the authoring environment.

### v0.7 delivery policy correction

User clarified that development must remain on dev and main must contain only
run/build inputs and applicable user documentation. Replace direct dev-to-main
PR #2 with a strict runtime export based on main. Preserve the complete dev
tree, tests, revision/provenance logs, CI and handoff. Add standalone exported
source verification on both host platforms and remove development history from
native user archives. No CPU/decoder semantics changed by this correction.

Runtime split verified at dev 0cb33f73a629af01315f3562a3d0be46b58d4fba:
https://github.com/pyTony/PIL-Processor-Instruction-Language/actions/runs/40320291757
Both hosts passed the 92 dev tests (Ubuntu 4.90 s, Windows 7.57 s), the isolated
36-file runtime build/launch/serial/generated-Rust checks, and unpacked native
ZIP checks. Runtime cargo test reports zero tests because tests stay on dev.
PR #2 closed; runtime-only PR #3 targets main from delivery/v0.7.0, commit
fd97dca67d25a4d1e4f59ffdc9bbf53baabdb94b. Main remains unchanged until approval.

## v0.7.1 — Hello-world guest example

Add a 57-byte executable Hello-world guest, demo selection, binary generation
and build/archive/runtime-export integration. Two debugger tests check exact
output, no RX dependency, halted EIP and reset/replay. No decoder or ISA changes.
Development stays on dev; main receives a separate 38-file runtime export.

User confirmed v0.7 Termux executable selftest: 63 completed steps, halted at
EIP 0x1cb, PASS with DX=600D, RAM signatures, console output 41 | A and TX: 41.
This confirms the manual test as well as the previous automated host checks.

v0.7.1 host verification passed at dev c1eca1982401e179b5eaa893d25a0572490a28b8:
https://github.com/pyTony/PIL-Processor-Instruction-Language/actions/runs/40324429616
94 tests passed on Ubuntu (5.90 s) and Windows (10.60 s). Isolated 38-file
runtime builds, Hello-world serial transcripts, generated Rust and unpacked
native delivery checks passed on both. Runtime-only PR #4 targets main from
delivery/v0.7.1. Phone Hello-world execution remains unreported.

## v0.8.0 — numerical stress and loop timing

User requested numerical stress/tight-loop timing, preferably 256×256 Mandelbrot,
or factorials if the instruction additions were too large. Select factorials
and a counted integer loop using the existing decoder/ISA; Mandelbrot needs
additional arithmetic/framebuffer work. Add 71-byte factorial and 27-byte loop
programs, bounded time/bench/stress commands, result checks, release helper
scripts and runtime/archive integration. Seven tests added (101 dev tests).

User confirmed v0.7.1 Hello-world on Termux: 28 steps, EIP=0x138, console
Hello, world! plus newline and matching 14-byte TX output.
Host v0.8 verification pending. Development remains on dev; runtime-only main
delivery will contain 44 files. No new third-party material used.

v0.8 host verification at dev 2ff2bcb89bbc03ac46870413a6dabfb7cd5d90b9:
https://github.com/pyTony/PIL-Processor-Instruction-Language/actions/runs/40331078455
101 tests passed on Ubuntu (7.71 s) and Windows (13.29 s). Numeric CLI checks
verified all factorials and fresh-state loop/stress samples. Release benchmark
helpers, isolated 44-file runtime builds/scripts/generated Rust, and unpacked
native archive numeric transcripts passed on both platforms. Timing values are
host-dependent measurements, not pass thresholds. Runtime-only PR #5 targets
main from delivery/v0.8.0. Phone numerical results remain unreported.

## v0.9.0 — cached PIL interpreter

User asked to continue optimization and confirm the execution engine. The engine
is the PIL token interpreter; Rust-source emission is separate. Add a bounded
decoder/PIL cache with shared immutable descriptions, per-hit byte validation,
full image/segment-context matching and non-cached failure paths. Add cache
controls/stats, uncached benchmark mode, and 15 tests (116 dev tests total).
Cached-vs-uncached instruction states, self-modifying code, live addresses and
RAM protection, fetch context and 65,536-iteration execution are covered.

User-confirmed Termux v0.8 baseline (android/aarch64):
- Factorials: 336 instructions, PASS, debug elapsed 42.891 ms (7834 ins/s).
- Stress: 100 fresh runs, 1200 results, 33600 instructions, PASS, 3317.416 ms.
- Default loop: 65536 iterations, 196611 instructions, PASS including registers
  and flags; debug 22430.539 ms (8765 ins/s).
- Release benchmark: 10000 iterations, 30003 instructions/sample, five measured
  samples and one excluded warmup. Times [340.564, 340.618, 340.420, 340.299,
  340.141] ms; median 340.420 ms; 88135 ins/s. Decode/PIL compilation included;
  image setup/checking/output excluded. Rust build time was outside samples.

Cached host verification pending; no phone cache performance claim yet. Main
will receive a 45-file runtime-only export; dev retains all history and tests.

v0.9 host verification at dev bf9e681065cf26f976954ea9a5a997604d4d7582:
https://github.com/pyTony/PIL-Processor-Instruction-Language/actions/runs/40336149129
116 tests passed on Ubuntu (5.24 s) and Windows (10.81 s). Standalone 45-file
runtime, generated Rust, both cache modes and unpacked native deliveries passed.
Release 10000-iteration / 5-sample loop comparisons (30003 instructions/sample):
- Ubuntu cached median 2.812 ms (10670355 ins/s), uncached 157.096 ms
  (190985 ins/s), approximately 55.9x throughput.
- Windows cached median 4.019 ms (7465290 ins/s), uncached 216.011 ms
  (138896 ins/s), approximately 53.7x throughput.
Each sample starts cold; cached final sample: 29997 hits, 7 misses, no byte/context
invalidations. These are CI host results, not phone measurements or simulated
386 cycles. Runtime-only PR #6 targets main from delivery/v0.9.0.

## v0.10.0-alpha.1 — compiled guest proof and alpha candidate

User authorized the generated-Rust comparison, then an alpha with a feature
matrix. Add an offline fixed-image runner composed from the existing validated
PIL emitter. Nine cases compare full CPU/RAM/serial traces after every guest
instruction; loop timings separate compilation and startup from execution.
Generated code guards bytes and uses black_box per instruction. Interactive
debugger remains token-interpreted. Add feature matrix and shared Windows/WSL
source directory instructions. Four dev tests added (120 total).

User v0.9 Termux release results (10000 iterations, 30003 instructions/sample,
5 measured samples and 1 excluded warmup, android/aarch64):
- Uncached [351.561,351.858,351.526,351.396,351.307] ms; median351.526 ms,
  85351 ins/s.
- Cached [7.492,6.994,6.955,6.988,6.981] ms; median6.988 ms,
  4293668 ins/s; 29997 hits, 7 misses, no invalidations. About 50.3x gain.
No phone generated-backend result yet. User will also test the PC.
Alpha is a candidate PR, not a published GitHub release/tag. Runtime-only main
projection is 50 files; all development logs/tests/tools stay on dev.

User additionally requested direct PC-native comparison and a verifier. Add
--verify-backends as a verification-only command; timing starts only after
byte-exact traces match. Add --cpu-bench on x86-64 using a bounded, checked
32-bit-register ADD/SUB/JNZ loop in 64-bit mode. Other hosts report unavailable
for this x86-specific baseline. No original 386 MHz claim is made.

Alpha source dev ca195663533ad41a6f50952442bf6a38d1a92447 verified:
https://github.com/pyTony/PIL-Processor-Instruction-Language/actions/runs/40341288842
120 tests passed on Ubuntu (8.20 s) and Windows (14.03 s). Independent verifier
matched 2,046,769 trace bytes over nine cases; comparisons were gated on that
match. Verification-only command passed. Isolated 50-file runtime source and
unpacked alpha native deliveries passed including compiled backend and CPU
baseline. No tag or GitHub release has been published; alpha PR #7 is ready.

Release comparison, 10000 iterations / five samples, CI hosts:
- Ubuntu cached interpreter median3.077 ms; generated Rust0.380 ms (~8.10x);
  direct CPU ADD/SUB/JNZ median0.003391 ms. Native compile1.818 s excluded.
- Windows cached interpreter4.664 ms; generated Rust0.421 ms (~11.09x);
  direct CPU median0.003100 ms. Native compile3.346 s excluded.
PC_NATIVE is the equivalent arithmetic microbenchmark in 64-bit mode, excludes
PIL/virtual RAM/guards and is not the 16-bit image. Do not equate it with 386
cycles or compare its scope to a complete PC emulator. Phone native and user PC
measurements remain unreported. Full sample outputs are in the CI job logs.

## Alpha.1 user phone benchmarks; alpha.2 verifier diagnostics

User-reported Termux release run, android/aarch64, 10000 iterations and
30003 completed instructions/sample; five samples plus one excluded warmup.
Uncached samples368.701,368.558,368.477,377.750,385.848 ms; median368.701 ms,
81375 ins/s. Cached7.160,7.075,7.192,7.135,7.134 ms; median7.135 ms,
4205340 ins/s. Cache29997 hits,7 misses,0 invalidations; observed51.67x.
No 386 cycle rate or phone compiled-backend result is implied.
PR #7 merged as dc9e3f3351aca235aea2d20da48f88a966e328cc.

Alpha.2 improves verifier diagnostics and retention; adds two regression tests
(122 total pending hosted CI). Tests inject field changes on the second
instruction and malformed/truncated/extra records. No external code/materials
were used for this change; trace format and implementation are project originals.

Alpha.2 hosted verification at cdb5fc396617071a134fc65ce67807cf56dc68f3:
https://github.com/pyTony/PIL-Processor-Instruction-Language/actions/runs/40360385843 — 122 passed on
Ubuntu in8.35 s and Windows in12.67 s. Nine cases/2,046,769 trace bytes,
verification-only invocation, isolated runtime and unpacked native checks passed.
Runtime-only PR #8 awaits user review; no GitHub release/tag published.

## Alpha.2 — first user-confirmed Android generated-backend result

User console evidence, Termux android/aarch64, sh tools/compare.sh 10000 5:
all nine cases matched 2,046,769 trace bytes, including faults/serial waits.
Generated samples [0.493461,0.761154,0.767385,0.764269,0.771961] ms;
median0.764269 ms,39,257,120 guest instructions/s. Cached interpreter samples
[9.476,6.874,6.878,6.890,6.916] ms; median6.890 ms,4,354,547 ins/s.
30,003 completed instructions/sample; five measured samples and one excluded
warmup; cache29997 hits/7 misses/0 invalidations. Observed9.02x improvement.
Generated compilation5.201 s is excluded from timing samples.
The direct x86-64 CPU loop is unavailable on this aarch64 phone as expected;
the generated runner compiled and ran natively for aarch64. User PC results
remain pending. No new tests or implementation changes in this evidence update.

## 0.11.0-alpha.1 — instruction expansion

Eight additional mnemonics, expanded binary ALU directions, immediate groups
and accumulator operands. Six new Rust semantic/decoder tests (128 total)
pending hosted execution; fourteen compiled/interpreted verification cases.
Python independent logic oracle extended to OR/XOR/TEST. Runtime58 files.
Reference-only use of Intel 80386 manual recorded; no third-party code imported.

Verification completed at aab72b00d47f77aa31ad30abb404070472fcb49c:
https://github.com/pyTony/PIL-Processor-Instruction-Language/actions/runs/40364437807 — 128 tests passed
on Ubuntu7.50 s and Windows16.08 s. Fourteen traces matched2,460,193 bytes.
Python oracle1,184,058 arithmetic/logical +1,030 conditional cases passed.
Standalone58-file runtime and unpacked native deliveries passed. Runtime-only
PR #9 awaits user review; phone verification of this revision remains pending.

## 0.11.0-alpha.1 — user-confirmed phone verification

User console evidence, Termux android/aarch64, sh tools/compare.sh 10000 5:
all14 cases matched2,460,193 trace bytes, including ALU8/16/32 (31 steps
each), read-only TEST success and NEG fault before retirement.
Generated samples [0.690346,0.779307,0.688231,0.690385,0.689346] ms;
median0.690346 ms,43,460,815 guest instructions/s. Interpreter samples
[7.083,7.077,7.091,7.117,7.095] ms; median7.091 ms,4,231,275 ins/s.
30,003 instructions/sample, five measured samples plus one excluded warmup;
cache29997 hits/7 misses/0 invalidations. Observed ratio10.27x; generated
compilation10.230 s excluded. No controlled cross-version performance claim.
Direct x86-64 baseline unavailable on this aarch64 phone, as expected.
No implementation changes or new tests in this evidence update.

## 0.12.0-alpha.1 — truncated signed multiplication

Generic sext/mul plus IMUL16/32 register/memory/immediate decoding. Six tests
added (134 total pending hosted CI),18 compiled/interpreted cases,59 runtime
files. Independent Python65,680 signed products checked. Boundary primitive
tests cover widths1/8/16/32/63/64 and generated Rust. Credits updated for Intel
IMUL reference; no source imports. PC commands included in user docs.

Verification completed at c1a91bcaa82a555e2b7ec3d4682729d6619ff93a:
https://github.com/pyTony/PIL-Processor-Instruction-Language/actions/runs/40367901727 — 134 tests
passed on Ubuntu10.65 s and Windows18.35 s. Eighteen full-state cases matched
2,535,361 bytes. Standalone59-file source and unpacked native checks passed.
Runtime-only PR #10 awaits user review. Phone/PC user verification pending.

## First user Windows PC benchmark — 0.11.0-alpha.1

Windows/x86_64 release, tools/bench.ps1 defaults:1,000 iterations,3,003 guest
instructions/sample,three measured samples plus one excluded warmup. Samples
[0.518,0.438,0.410] ms; median0.438 ms,6,853,035 guest instructions/s.
Cache2997 hits,7 misses,0 invalidations. Cached interpreter only; no compiled
Rust or direct CPU measurement supplied yet. Different iteration/sample counts
from phone results; no controlled cross-device ratio inferred. User console
output is evidence; this documentation update changes no implementation.

## User Windows PC full comparison — 0.11.0-alpha.1

User PowerShell evidence, windows/x86_64 release,10000 iterations/30003
instructions per sample,5 samples plus1 excluded warmup. All14 cases matched
2,460,193 bytes. Generated samples [0.2936,0.2935,0.2934,0.2935,0.369] ms;
median0.293500 ms,102,224,872 guest instructions/s. Cached interpreter samples
[4.080,4.163,4.086,4.069,4.072] ms; median4.080 ms,7,353,496 ins/s.
Observed13.90x; compile2.294 s excluded. Cache29997 hits/7 misses/0 invalidations.
Direct CPU ADD/SUB/JNZ passed EAX10000/ECX0; samples [0.0022,0.0022,0.0021,
0.0021,0.0022] ms,median0.002200 ms,0.220 ns/iteration. This narrower64-bit
mode register loop excludes decoder/PIL/guards and includes call/timer overhead;
not execution of the16-bit image or a386 clock estimate.
Separate preceding standalone interpreter run10000/3: [3.225,3.216,3.172] ms,
median3.216 ms,9,330,161 ins/s; retain separately from comparison samples.
No implementation changes in this evidence update.0.12 user runs remain pending.

## 0.13.0-alpha.1 — shifts and direct debugger startup

SHL/SAL,SHR,SAR register/memory,count1/CL/imm8 plus generic shift/ult/flag_if
operations. Six new tests (140 total pending CI),23 verifier cases,62 runtime
files. README quick start added following user's difficulty finding debugger
usage after build. Intel PRM and SDM references logged; no external code imports.

## User Windows interactive debugger confirmation

User supplied a selftest transcript after running one step: next run completed
62 instructions (63 total), stopped EIP1cb and passed DX600D/RAM/TX41 checks.
RAM500 contained79 56 34 12 78 56 34 12. RAM5000 correctly reported out of
bounds for the4-KiB monitor. Register/TX output agreed. The transcript does
not contain a version banner; surrounding session was0.11, no0.13 run inferred.
User requested a README documentation index after finding launch instructions
in docs. Linked indexes added to development, runtime-source and native READMEs.

0.13 hosted verification completed at1a35fa53fcddd5a89e86813699736e5cc3c68832:
https://github.com/pyTony/PIL-Processor-Instruction-Language/actions/runs/40371543845 —140 passed on
Ubuntu12.43 s and Windows22.00 s;23 cases/3,044,833 exact trace bytes.
Standalone62-file runtime and unpacked native checks passed. Independent
Python59,940 shifts and65,680 signed products plus prior oracles passed.
Later docs-only commits add indexes/all7 demos/run-budget explanation; local
links/checksums checked without repeating unchanged Rust execution tests.
PR #11 supersedes unmerged signed-multiply PR #10; user main remains runtime-only.

User PC factorial debugger run:336 steps,halt146,0.979 ms,343172 ins/s;
cache18 entries,319 hits,18 misses,0 invalidations/evictions. Check passed
all12 results through12!=479001600,confirmed by48-byte RAM dump. Stress100
passed1200 results/33600 instructions in89.580 ms including setup/checking.
Transcript has no version banner; surrounding Windows session was0.11,not a
new0.13 verification. These timings differ in scope from the counted-loop bench.

## Typed memory dumps in pending0.13 alpha

User requested8/16/32-bit values and visible ASCII. Added mem8/mem16/mem32
with value counts, hex/unsigned-decimal selection, little-endian grouping and
safe ASCII rendering. Existing mem unchanged. Four debugger tests added
(144 total pending CI) for widths/endian/unaligned/ASCII, rows/defaults,
factorial decimal values and invalid ranges with state preservation.
Original implementation; no new reference materials or imports.

User merged PR #11 while typed dumps were under validation. Dump update gets
its own0.13.0-alpha.2 revision and runtime PR based on merged main.

User Windows0.13.0-alpha.1 comparison passed all23 cases/3,044,833 bytes.
Generated median0.297800 ms/100,748,825 ins/s from [0.2978,0.2978,0.2978,
0.3041,0.2978] ms. Interpreter median3.280 ms/9,146,141 ins/s from
[3.286,3.280,3.461,3.275,3.274] ms. Observed11.02x; build3.577 s excluded.
Cache29997 hits/7 misses/0 invalidations. Direct CPU baseline passed; samples
[0.0044,0.0043,0.0044,0.0043,0.0043] ms,median0.004300 ms/0.430 ns per
iteration. Narrower scope,not386 cycles.10000 iterations/30003 instructions
per sample,5 samples plus1 excluded warmup. REPL startup confirmed.

0.13.0-alpha.2 host verification at381cac63ee71c552d248177ba0156e07ddce7b79:
https://github.com/pyTony/PIL-Processor-Instruction-Language/actions/runs/40373417827 —144 passed,
Ubuntu12.39 s/Windows16.80 s. Existing23 cases/3,044,833 trace bytes match.
Standalone62-file source and unpacked native delivery checks passed. Runtime
PR #12 awaits review; typed dump user verification unreported.

## Readable disassembly in pending typed-dump delivery

User reported factorial list too terse. Add decoded registers/immediates/memory
operands/branch targets and encoded bytes,current EIP marker,bounded count/start
arguments. Fix generated numeric .lst files to name EAX etc.without changing
code bytes. Four tests added (148 total pending CI) for operand labels,targets,
full factorial listing,limits/live edits and preservation. No external code or
materials introduced; formatting uses the existing decoded operand metadata.

User merged PR #12 at its typed-dump head52834b3e0dbefbf724408b21dedb4cce19647568
before the listing follow-up was ready. Listing gets separate0.13.0-alpha.3.
Code source0e2d8d495f84c27830dda4ae0cf31579247e9afd passed148 tests on Ubuntu
8.32 s/Windows18.29 s in CI40374526560;23 cases matched3,044,833 bytes.
Only version/docs metadata changes after this test run; execution source unchanged.

Runtime-only alpha.3 PR #13 ready:head6b193b23c11946855b87d4bf6221ddde74fe5a6c,
exported from dev d57ca83e80ec1ce247478a59be9616aa1610546a on main after
merged typed-dump PR #12. No automatic merge or tag/release publication.

## 0.14.0-alpha.1 — guest Mandelbrot images

Q12 guest streams image pixels,host drains/checks/saves BMP/PGM. Add nearJcc
and relativeJMP. Seven tests supplied (155 total) pending hosted CI.26 full-state
compiled cases include tiny2x2 image and near-branch faults. Full256/32 renders
plus independent Python pixel/BMP oracle added to both host workflows. Runtime
66 files; references/credits updated,no imported code.

User WSL alpha.3 run on shared/mnt/c checkout passed23 cases/3,044,833 bytes.
Generated median0.345313 ms,86,886,390 ins/s; interpreter2.612 ms,11,484,806
ins/s;7.57x. Compilation3.077 s excluded; direct CPU baseline passed,median
0.002174 ms/0.217 ns per iteration.10000/5 with1 excluded warmup. Cache29997
hits/7 misses/0 invalidations. Separate target/wsl recommended for shared builds.
No controlled Windows-vs-WSL ratio inferred.

0.14 verification at6c7297ea7c4075e73bb5738167ead7a496152f11,CI40378440616:
155 passed on Ubuntu13.60 s/Windows17.87 s;26 cases/4,273,138 trace bytes.
Full256x256/32 exact65,536 pixels,12,940,344 guest instructions,255 TX retry
drains;single render1599.244/1664.788 ms respectively. Independent Python
PGM/BMP oracle passed,rasterSHA256=3855106a06087a62fdfc6e322767988e441137f5e13071b473f43e2f20140619.
Standalone66-file source and unpacked native render checks passed. Runtime-only
PR #14 awaits user review; no phone/PC user render reported or tag published.

## 0.14.0-alpha.2 — full-image compiled comparison

PR #14 is merged at main cf8cd1cbdc1843f4a92db9de44a820ace3dacf1c.
User Windows0.14alpha.1 full256x256/32 passed65,536 reference pixels,
12,940,344 steps in1484.024 ms,255 retries;43 cache entries,12,940,557 hits,
43 misses,no invalidations/evictions. This is one interpreted render.
New optional --compare-mandelbrot gates with26 per-instruction cases, then
compares full-image pixels plus CPU/RAM/flags/queues/stop at serial-drain/halt
checkpoints and exact step/retry counts. Single cold render each; capture/draining
included,setup/reference/files/compilation excluded. Existing interpreter image
render and REPL unchanged. Two tests added (157 total pending hosted CI).
Original code, no new external sources or dependencies. Main stays runtime-only.
Next: validate both hosts, export66 files and prepare separate runtime PR.

Earlier user reports now explicitly indexed: Windows0.13alpha.2 compared23
cases/3,044,833 bytes,generated0.2978 ms vs interpreter3.562 ms (11.96x),
compile3.613 s excluded. Its direct CPU baseline0.0022 ms has narrower scope.
User alpha.3 listing/current marker,100+236 factorial instructions and all12
decimal RAM values were also confirmed. These are user reports,not newCI runs.

## 0.14.0-alpha.2 hosted verification

Core dev a5db4e29542b4b62bdcc4389df17e741de8d094e passed CI40384865363:
157 tests on Ubuntu11.13 s/Windows26.02 s; existing26-case per-instruction
proof still matched4,273,138 bytes. Full256x256/32:65,536 pixels,12,940,344
instructions,255 retries each engine. All256 drain/halt CPU/RAM/flags/serial/
stop checkpoints matched1,134,592 bytes. Generated/interpreter single render:
Ubuntu110.936/1283.570 ms (11.57x),Windows232.995/2930.892 ms (12.58x).
Image compilation0.616/1.792 s excluded,as is the separate verifier build.
Includes checkpoint capture/draining; excludes setup/reference/files/startup.
Standalone66-file source/scripts and unpacked native delivery checks passed.
Follow-up files correct old docs and record evidence; execution source unchanged.
User Windows0.14alpha.1 normal render1484.024 ms is separately recorded.
Prepare delivery/v0.14.0-alpha.2 from main cf8cd1cbdc1843f4a92db9de44a820ace3dacf1c;
user reviews/merges in browser. No tag/release; development stays dev.
Next: user full-image comparison,then additional device/machine-state support.

Runtime-only PR #15: https://github.com/pyTony/PIL-Processor-Instruction-Language/pull/15;
head46053bcd2193060477efa43c0cbc6a8647b5e5c1,source441844483574e5b55c7eb8691fdce7ff828a6a4c.
User review pending; no tag/release published.

## User Termux0.14alpha.1 backend comparison

Termux android/aarch64 release0.14.0-alpha.1,10000 loop iterations,5 measured
samples plus1 excluded warmup:all26 cases/4,273,138 trace bytes matched,
including mandelbrot2 (291 steps),near branch and jump faults. Generated median
0.700462 ms (42,833,159 guest ins/s),cached interpreter7.168 ms (4,185,439 ins/s),
10.23x; generated compilation16.910 s excluded. Cache29997 hits/7 misses/0
invalidations. PC_NATIVE unavailable on ARM64 as expected; emitted Rust does
execute natively on the phone. This is the counted-loop comparison,not a full
256x256 render or alpha.2 --compare-mandelbrot result. No execution changes.

## User Termux0.14alpha.2 full-image comparison

User Termux android/aarch64 release --compare-mandelbrot256/32 passed the
26-case gate (4,273,138 exact per-instruction trace bytes),all65,536 reference
pixels and256 CPU/RAM/flags/queues/stop checkpoints (1,134,592 exact bytes).
Both engines completed12,940,344 guest instructions and255 TX retries.
Generated Rust151.897 ms vs cached interpreter2991.931 ms,19.70x in one cold
render each. Verifier compilation17.165 s and image compilation2.706 s excluded.
Checkpoint capture/draining included; setup/reference/files/startup excluded.
Not a median benchmark or386 cycle measurement. No execution changes.

## 0.15.0-alpha.1 instruction expansion

MOVZX/MOVSX byte-to16/32 and word-to32 (register/memory/high-byte),CBW/CWDE,
CWD/CDQ andLEA16/32 added with original PIL descriptions. Independently typed
source bindings (fixed8/16/32 per program); destination/read-dst still operand
width. Interpreter/emitter validate ranges before effects. LEA reuses effective
address resolution/read_address,never translates/accesses data RAM/segments;
CS fetch remains checked. Mixed operand/address sizes and overlap are tested.
All new instructions preserve flags/undefined metadata. Six new Rust tests
(163 total pending hosted CI),extended generated-function test and8 new full-state
cases (34 total). Independent Python132,118 extension/address cases passed.
Six new PIL files; runtime72 files. References/credits record consulted Intel386
manual entries. No external code/data/dependencies imported. Main stays runtime-only.
Next: Windows/Linux CI,then export and PR from main336610f56b46fd69a6aa5a2e2354dc64311800a5.

## 0.15.0-alpha.1 hosted verification

Core source6ba1ac3b8affe72702290a067a9c1c95f9626626 passed CI40404626380:
163 Rust tests,Ubuntu14.66 s/Windows31.44 s;34 per-instruction cases matched
4,394,242 trace bytes. Python132,118 independent extension/address cases passed.
Isolated72-file source/scripts/emitted Rust and unpacked native deliveries passed.
Full256x256 Mandelbrot regression matched65,536 pixels and256 checkpoints
(1,134,592 bytes) on both engines/hosts. No phone0.15 result yet. Execution code
unchanged by this evidence/docs follow-up. New instructions: MOVZX/MOVSX,
CBW/CWDE,CWD/CDQ,LEA; all flags/markers preserved. Source bindings independently
typed8/16/32. LEA resolves offset without data segment checks or data RAM access.
Prepare runtime-only delivery/v0.15.0-alpha.1 from current main; user merges.
Next useful ISA work: stack/call/return with explicitly staged register effects.
No release/tag published; preserve main/dev separation and reference records.

## Runtime review follow-up

Claude inspected runtime main0.14alpha.1 and correctly observed cargo test0,
all8 demos/checks passed,cleanbuild/manifest. No-tests conclusion reflects the
intentional export: full tests/CI ondev (163 passed Windows/LinuxCI40404626380).
Do not duplicate tests onto main. Runtime README now explains this distinction.
Valid finding fixed:unsupported check reports no built-in check and names the
4 supported images; incomplete supported programs retain specific failures.
One new regression test covers all8 demos/read-only state (164 pending CI).
Four EIP assignment spacings and separate REPL if lines address reported
ambiguous formatting; no wholesale rustfmt/style rewrite or clippy-clean claim.
Pending runtimePR16 (head0d4da793e2159ff21ccd129d034d0738a1adfe1d) is ready for
ISA expansion; append this tested follow-up only if still open,otherwise export
a separately versioned delivery. Keep development/tests/provenance ondev.

## 0.15.0-alpha.2 diagnostic delivery verification

Core patch54149d44cba6079d4ccf4f1b1716ac2f4511821a passed CI40405704118:
164 tests on Ubuntu15.29 s/Windows22.30 s;34-case per-instruction proof matched
4,394,242 bytes.72-file isolated source/helpers and unpacked native deliveries
passed; full Mandelbrot regression matched65,536 pixels/256 checkpoints.
Version metadata bumped to0.15alpha.2 after the core patch because ISA PR16 was
merged separately at main33377e984e4e7d54b99cf6db2f1e9c57b238b35c. Execution
logic unchanged by version/docs/evidence follow-up. Prepare separate runtime PR.
Unsupported check diagnostic fixed,8-demo regression added;5 formatting spots
clarified,not a full clippy/rustfmt rewrite. Runtime README explicitly says
cargo test0 is intentional; automated tests/CI live ondev. Claude's runtime
review passed8 demos; no-tests conclusion does not describe development branch.
No new external sources/code/dependencies. No release/tag; no phonealpha.2 result yet.

Runtime-only PR #17: https://github.com/pyTony/PIL-Processor-Instruction-Language/pull/17;
head9c40f41cec5971c740573734ad72d456691bbc91,source0ced9f0e9263162f7ffe4259be9f2020d4f8c93a.
User review pending. Tests/history remain ondev; main retains runtime-only policy.

## User-supplied Claude0.15alpha.2 release/ISA smoke report

PR17 is merged (API confirmed). User supplied Claude release report:manifest
complete/matching,DELIVERY/Cargo versions agree,warning-free build,selftest/
factorials/smallMandelbrot/loop PASS,hello correct. Custom binary confirmed
MOVZX EBX,AL=80;MOVSX ECX,AL=FFFFFF80;CWDE EAX=FFFFFF80;CDQ EDX=FFFFFFFF;
LEA SI,[BX+DI+5]=85. Host unspecified; this is reported evidence,not a local run.
Same-width MOVSX rejected with no changes,as current profile intends. Encoding
correction:0F BF F8 is DI,AX in16-bit operand size;DI,DI is0F BF FF.
Feature matrix template ondev now explicitly notes16-to16 MOVSX/MOVZX rejection;
this documentation will ship with the next runtime export. No execution changes.
Intel386 tables list widening forms only; SingleStepTests386 metadata consulted:
0FB7/0FBF rows advertise r16/32,r/m16; unprefixed real-mode MOO files exist.
No hardware vectors downloaded/imported/executed; do not claim those passed or
conclude from modern-host execution alone. Consider same-width support with
independent expected-state checks in a future compatibility increment.

## 0.15.0-alpha.3 same-width conversion compatibility

MOVSX/MOVZX16-to16 now accepted via existing equal-width sext/zext PIL tokens.
No special decoder arithmetic; word-source descriptions allow16/32 destinations.
Registers/aliases preserve upper halves and flags; memory forms still word-read
with normal checks. Original demo extensions runs15 instructions then check
verifies all8 regs,flags/markers,RAM180 andhalt. Nine demos now; unsupported
check diagnostic names extensions too. Existing boundary/source tests expanded;
one new demo regression (165 total pending hosted CI).39 compiled cases incl
read-only word sources,GP/SS/RAM faults. Python263,190 independent cases passed.
72 runtime files; isolated/native package smoke transcripts include the demo.
References/provenance updated; no hardware vectors imported/executed. Main/dev
separation retained. Next: hosted CI, runtime export and separate alpha.3 PR.

## 0.15.0-alpha.3 hosted verification and suite research

Core036ff225fcd30a16079fa48e1387fd29891d8a34 passed CI40408105698:
165 tests,Ubuntu15.27 s/Windows22.70 s;39 full-state cases matched4,494,466
trace bytes. Includes15-step extensions demo,read-only equal-width word reads
and preciseGP/SS/RAM faults. Python263,190 independent cases passed.72-file
isolated build/helpers and unpacked native archive exercise/check the new demo;
full256x256 Mandelbrot regression passed both hosts. No useralpha.3 run yet.
Follow-up changes: help strings list9 demos/5 checkers; docs/provenance/evidence
only, no execution logic change. Help text manually checked; core tests as above.
User suggested SingleStepTests andbarotto/test386.asm. README/licences reviewed
at pinned revisions; dev plan docs/development/EXTERNAL_TEST_SUITES.md records
MIT software/public-domain data forSST andGPLv3-or-later forbarotto. No external
vectors/code/ROM imported or executed. First external CPU adapter must preserve
original state/addresses and compare each backend to hardware; testROM islater.
Prepare separate0.15alpha.3 runtime PR from main; keep test/plan/history ondev.

Runtime-only PR #18: https://github.com/pyTony/PIL-Processor-Instruction-Language/pull/18;
heada1babcdbee46d9c99a9e4ae7e62c2866478f211c,source6b152b91e9d74e3bf3fc5c537351b4b6776ac9ec.
User review pending. Tests and external-suite plan remain ondev.

## User Windows0.15alpha.3 confirmation —27 October2026

User supplied Windows/x86_64 release comparison10000/5:all39 cases matched
4,494,466 trace bytes. Generated median0.5976 ms (50,205,823 guest ins/s),
interpreter4.723 ms (6,352,530 ins/s),7.90x; generated compilation4.271 s
excluded. Cache29997 hits/7 misses/0 invalidations. Direct CPU baseline passed,
0.0022 ms/0.220 ns per iteration,narrower scope; not386 cycles.
Interactive extensions listing bytes/operands confirmed;15 completed instructions,
haltEIP12d,checkPASS. EAX/ECX=FFFFFF80,EDX=FFFFFFFF,EBX=80,ESP/EBP/EDI=FF80,
ESI=85. Debug build2.06 s/release2.27 s separately reported. No execution changes.
External hardware suites remain researched/planned,not yet imported or executed.

## ISA completion/native64 planning —27 October2026

ISA_COMPLETION_PLAN.md orders P0..P9 for documented386 integer/system work,
with separate undocumented/80387 tiers. ISA_BACKLOG.json tracks55 family/form
planning items,not an opcode completion percentage. Baseline remains0.15alpha.3,
165 tests/39 original backend cases. External/native oracle passes remain zero.
X86_64_VERIFICATION_PLAN.md defines N0..N6: start a modern-PC oracle for existing
8/16/32 ADD/SUB/ADC/SBB/CMP now,then explicit64-bit state/REX/write policies,
wide values through128,memory and managed-stack/control tests. Keep shared PIL
semantics but separate mode-specific rules and independent flag masks.
In parallel build the pinned SingleStepTests one-step adapter. Named multiple
register effects and ordered memory/commit points precede stack/CALL/RET.
OS_COMPATIBILITY_TARGETS.md records the user's Windows milestones with edition
qualifiers: WfW3.11 is the3MB row (2MB without networking),Windows95 has a386DX/
4MB minimum,NT3.51 Workstation target12MB versus Server16MB. These are future
machine goals,not current boot support or guaranteed TTGO memory feasibility.
Plans linked from README/roadmap/external-suite notes; references and history
updated. Documentation-only; no runtime changes or new CPU verification claimed.
Keep this work ondev; no runtime delivery PR or Cargo version bump is needed.

## Actual-host ALU oracle development and old CI clarification —27 October2026

Adds optional x86-64 hardware comparison for ADD/SUB/ADC/SBB/CMP8/16/32,
CF0/1,boundary/random cases and fixed independent mask8d5. Literal native
bytes,CF/flags snapshots and source/sentinel/upper-half checks precede both
PIL comparisons; guest and long-mode encodings are separately recorded.
Six known-answer stub checks; original cases only,not external386 vectors.
Artifacts:target/host-cpu-oracle/run-*/ includingdata,source,CPU/toolchain/source
provenance and first-field failures. Helper python tools/verify_host_cpu.py.
CI40415384652 passed on Linux/Intel and Windows/AMD:167 tests per host,
20,280 native CPU cases per host checked against both PIL backends. Source
24d28740c8d63a0ce393cec05b56320b7833b5c4. ARM hardware test remains ignored;
no ARM run is claimed for this increment.
Source includes only a cfg(test) module; runtime export remains72 files.
Node24 action updates:checkoutv5,upload-artifactv6. User-linked old run40377893902
failed9632f84 due E0271 inMandelbrot argument closure; fixed historically. Its
Node20 warning was separate; Windows was canceled by matrix fail-fast.
No runtime/Cargo version change. Keep tests and tooling ondev,not main.

## Unary hardware oracle expansion —27 October2026

Extends the development-only host oracle with INC/DEC/NEG/NOT8/16/32.
CMP AL,CL varies incoming arithmetic flags before BT selects CF0/1. INC/DEC
must preserve CF value and metadata; NOT must preserve all flags/metadata.
Unary cases start both with known and undefined CF metadata. Independent
written masks,12 native known answers and decoded operand bindings are used.
CI40417668829 passed on Windows/Linux AMD hosts:52,728 native cases/27
variants against both backends;167 tests per host. Sourcef6ff393221cb9c771b12556fe76c05336f3559d4.
The initial DEC8 known-answer fixture used the wrong parity bit for7F;
corrected from814 to810 before acceptance. This was a harness fixture error.
Runtime semantics/version/export are unchanged; tooling stays ondev.

User WSL evidence for preceding source48556af:20,280 hardware cases passed
both backends on AuthenticAMD CPUID1 EAX00a20f10,linux/x86_64,rustc1.99.1.
Fingerprint8b8c01e8fdeff584; one selected test passed,166 filtered out.
Artifacts were isolated under target/wsl. This is user-reported correctness,
not a throughput measurement or a full-suite run.

## Logical hardware oracle expansion —27 October2026

Adds AND/OR/XOR/TEST8/16/32 to the development-only CPU oracle. Independent
flag policies partition every bit into defined,preserved,undefined. LogicalAF
is excluded from native value equality; concrete zero and AF diagnostic marker
are checked separately as PIL policy. TEST preserves the full nativeRAX.
CI40419168682 passed on Windows/Linux:168 tests and85,176 hardware cases
per host,39 encoding variants and16 known-answer harness checks. Source
342375e3a732dfe59f4b799358bf743d51cdd496. LinuxAMD CPUID1 EAX00b00f21;
WindowsAMD EAX00a00f11. Existing39 backend trace cases,full256x256 Mandelbrot
and isolated runtime/package checks passed. Record schema2 is28 little-endian
u32 words (112 bytes),with independent masks. No ARM execution claimed.
Runtime unchanged; no new guest instruction support claim.

User reported preceding sourced23ab45 passing52,728 hardware cases on native
Windows/x86_64 AuthenticAMD CPUID1 EAX00a20f10 (ECXfed8320b),rustc1.99.1.
Fingerprint0bc8895949b33008; one selected test passed,166 filtered out.
Artifacts under target/host-cpu-oracle. This is correctness,not a benchmark.

## Independent overnight work: shifts and multiplication —27 October2026

User authorized substantial independent work while away. First slice extends
host oracle to SHL/SHR/SAR8/16/32 (CL forms) and truncated two-operand IMUL16/32.
All256 count bytes cross boundary inputs; deterministic random pairs and both
carry/metadata states remain. Count-zero preserves flags/markers; active shift
undefined masks depend independently on masked count and width. IMUL compares
only CF/OF to hardware and checks its other four zero/undefined PIL outputs.
CI40422881682 passed on Windows/Linux:226,008 hardware cases/50 variants,
23 harness checks and169 tests per host. Source811af3596bdd5157cff8c3f3c9769de2b8f91b0a.
No runtime change in this verified first slice.
Next candidate: rotate descriptions/decoder/forms with independent oracles.
Keep new runtime work ondev; prepare a strict delivery PR only after checks.

## 0.16 rotates and carry constants supplied —27 October2026

Adds ROL/ROR8/16/32 register/memory,count1/CL/imm8,and CLC/STC via four PIL
descriptions. No new interpreter/emitter semantic tokens. Preserve other flags,
handle masked-zero/full-width turns distinctly,and retain RMW permission faults.
CI40423654988 passed on Windows/Linux at21d65da9a430d77ee3ad1e658afbeede907ae041:
175 tests,45 full-state backend cases (4,861,954 trace bytes),278,576 independent
Python description cases and321,784 native CPU cases/58 variants/31 harness
checks per host. Full256x256 Mandelbrot and isolated runtime/package checks passed.
Runtime export76 files; PR19 is open and mergeable,awaiting browser review.
Delivery head e010681736c87979f3bee77276e1ae27e4fa8f93; base main9653d8f.
No release/tag or main merge performed. No ARM execution claimed for this increment.
RCL/RCR,CMC and double shifts remain planned. Runtime PR: https://github.com/pyTony/PIL-Processor-Instruction-Language/pull/19
Keep dev separate from main; user reviews/merges the delivery PR. No external386 suite execution claimed.

## Two bound register effects and XCHG supplied —27 October2026

First P0 slice permits one staged write per dst/src binding,including
conditional source writes;reads always see incoming state. Active writes that
overlap must assign identical shared bits,or fail InvalidBinding before RAM
commit. Existing single-store and precise fault retirement are retained.
XCHG register86/87 and accumulator91..97 use the shared PIL description.
Memory XCHG/LOCK and arbitrary implicit-register/multi-store effects stay planned.
Seven new tests (182 expected),48 full-state backend cases. Independent Python
exchange checker passed131,216 cases locally. Modern hardware suite expected
329,896 cases/61 variants/34 harness checks;both exchanged native registers
and their upper-half policies are checked. Hosted execution pending.
Runtime export77 files. PR19 was confirmed merged before this increment.
Prepare a separate0.17 delivery PR after checks;main stays a strict runtime tree.

## PIL reference audit — 27 October 2026

Rewrote docs/LANGUAGE.md against the implemented parser, interpreter and emitter.
Corrected stale memory/fetch/test-layout claims, source-width inference from
reads and writes, complete template parameters, token restrictions, flag metadata,
staged overlap/retirement rules and execution-API differences. Marked the original
v0.1 proposal as a separate design and repaired its platform-adapter link.
Documentation only; executable source and instruction support are unchanged.
Validation: local Markdown destinations and source checksums verified; token/API
contracts checked directly against src/main.rs and src/flow.rs.

## Documentation overview — 27 October 2026

Added docs/README.md as the single documentation entry point, covering the
implemented language, original proposal, executable descriptions, runtime guides,
verification plans, architecture/platform goals, sources and history. Linked it
from the root README and both language documents. Clarified which documents
specify current behavior and which record proposals, plans or historical results.
Documentation only. Local Markdown destinations and source checksums checked.

## DOS guide build-profile correction — 27 October 2026

User's Termux log showed build.sh correctly building0.19 in target/debug while
the supplied commands invoked an older target/release executable. Corrected
DOS examples to the default debug paths on both platforms and documented
--release/-Release and CARGO_TARGET_DIR. Executable source is unchanged.
Validation: commands checked against both build helpers; Markdown/source
checksum checks passed. No execution-test rerun is needed for this docs-only fix.
