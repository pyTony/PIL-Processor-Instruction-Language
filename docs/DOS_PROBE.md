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

# DOS COM/EXE import and instruction-gap probe

This probe loads headerless COM or DOS MZ/ZM EXE files and runs the implemented
PIL instruction subset. It reports the first blocker, source file offset, raw
bytes, register context and reached instruction frequencies. It can rank the
first missing CPU encodings across a collection of programs.

## First run: original hello-world programs

After `sh tools/build.sh` on Termux/Linux, create the supplied examples:

```sh
./target/debug/pil-proof --write-dos-hello com target/dos-hello.com
./target/debug/pil-proof --write-dos-hello exe target/dos-hello.exe
./target/debug/pil-proof --dos-probe target/dos-hello.com target/dos-hello.exe
```

PowerShell, after `.\tools\build.ps1`:

```powershell
.\target\debug\pil-proof.exe --write-dos-hello com target/dos-hello.com
.\target\debug\pil-proof.exe --write-dos-hello exe target/dos-hello.exe
.\target\debug\pil-proof.exe --dos-probe target/dos-hello.com target/dos-hello.exe
```

Expected: both produce `OUTPUT (escaped bytes): Hello, DOS!\r\n`, stop with
`Exit(0)`, and report `SUMMARY imported=2/2 exited=2`. Each executes three PIL
instructions and two bridged DOS calls. These are original tiny DOS executables,
not the existing monitor-port hello.bin. Creation refuses to overwrite files.
The EXE deliberately addresses its string relative to the initial PSP DS so it
can run before segment-register move instructions are implemented.

## Build profile and executable path

The default build helpers create debug binaries: `sh tools/build.sh` builds
`target/debug/pil-proof`; `.\tools\build.ps1` builds
`target/debug/pil-proof.exe`. They do not refresh an existing release binary.
Run the path reported by the build helper, as in the examples above.

For an optimized build, use `sh tools/build.sh --release` or
`.\tools\build.ps1 -Release`, then replace `debug` with `release` in the
executable paths. If you configured `CARGO_TARGET_DIR`, use that directory
instead of `target`.

If a DOS option prints the old `AX=0x8000; ... typed tokens` demonstration,
you ran an older executable that does not recognize those options. Select the
newly built debug binary or explicitly rebuild release; no files are generated
by that old fallback demonstration.

## Import your programs

```sh
./target/debug/pil-proof --dos-probe --steps 10000 program.com program.exe
```

Use quoted paths for filenames containing spaces. The optional step budget must
precede the filenames and applies to each program. Default:10,000; maximum:
1,000,000. Up to256 input files are accepted per report, each at most1MiB.
The diagnostic command returns a report even for import/execution blockers;
inspect the `STOP` and `SUMMARY` lines rather than treating its host exit status
as a DOS program success code.

At the debugger prompt, `dosprobe file.com 10000` runs the same isolated probe
without replacing or modifying the current monitor program. For paths with
spaces use the CLI; the REPL uses whitespace-separated arguments. The existing
`load` command still loads raw monitor images; use the DOS probe for DOS files.

## Interpreting the report

| Result | Meaning |
|---|---|
| `Exit(code)` | Reached a supported DOS termination call |
| `MissingEncoding` | Decoder stopped on an unsupported instruction/form/prefix |
| `Service` | Unsupported DOS/BIOS call or a bounded DOS-string service failure |
| `Fault` | Shared execution or fetch/segment/RAM check failed |
| `Boundary` | Instruction fetch left the program's DOS memory block (PSP to load-time allocation end) or truncated an instruction |
| `Budget` | Per-program execution limit reached |
| `Monitor` | Reached HLT or monitor-port I/O, which are not DOS termination/services |
| `IMPORT ERROR` | File/header/relocation/allocation checks rejected the input |

A typical relocated EXE startup can now load AX and execute `8E D8`
(`MOV DS,AX`) in real mode. The regression fixture then reaches an intentionally
unsupported `62` opcode at file offset37. That is a CPU gap; an
unimplemented `INT21/AH30` DOS-version query is a separate service gap.
Raw bytes are shown for context and are not claimed to be a decoded length.
`file_offset` is the position in the original COM file or MZ load module plus
header; it is absent for PSP or other addresses outside the module. Stop-state
registers and CS:IP show where execution reached, including after prior writes.

The missing-encoding histogram counts programs blocked by each first opcode,
not an estimate of every missing instruction in those programs. The reached
instruction mix counts fully retired instructions and lists bridged DOS calls
separately. The per-program step budget includes REP element steps (and bridged
services), so large repeats cannot bypass it. PIL execution counts include each
element; completed counts include the REP instruction only after full retirement.
Separate REP element mix entries count committed elements; zero-count REP
retires without a data access. A later fault preserves earlier element progress. Unknown encodings are never skipped; code/data bytes are not scanned
as if all were instructions. Fix a common blocker and rerun the same corpus to
reveal the next one. Dynamic paths, inputs and loops affect the frequencies.

## Real-mode CS data access

Ordinary COM/MZ CS caches permit data reads and writes, including CS overrides
used by startup and unpacking code. Segment limits and RAM write protection
still apply. Modified instruction bytes invalidate the interpreter decode cache
on reuse. Fetch is allowed anywhere in the program's block (PSP through the
load-time allocation end), so unpacked or relocated code runs. Explicit protected Code caches keep
their read/write restrictions. Fixed compiled images retain changed-code guards.

## Loader and service contract

- The probe places a minimal PSP at segment1000h in zero-filled conventional
  memory belowA0000h. It provides INT20 at PSP:0, the allocated-end segment,
  an empty command tail and no environment block or real DOS memory arena.
- COM starts at CS:IP=1000:0100 with CS/DS/ES/SS=PSP and SP=FFFEh. A zero
  stack word is installed there, including overwriting the last two bytes of
  a maximum-size65,280-byte image as part of that initial stack convention.
- MZ imports validate page counts, header/module sizes, relocation tables,
  relocation targets, initial entry and stack, min/max allocations and overlays.
  Relocations add the load segment1010h with16-bit wrapping, in table order.
  CS/SS are relocated with16-bit wrapping; DS/ES point to the PSP.
  Wrapped relative segments such as FFF0:0100 are accepted when the resulting
  physical entry lies inside the load module. Zero min=max high-load files,
  overlays and recognized NE/LE/LX/PE/W3/W4 containers are rejected. Trailing
  bytes beyond the declared DOS image are reported and not loaded. Checksum
  validation is not required; the MZ checksum field is ignored.
- Initial general registers are zero except SP, flags are0202h, and FS/GS use
  the PSP cache as a probe convention. These are not universal DOS entry-state
  promises. SS:SP is conservatively required to fit the assigned memory block.
- Execution uses the concrete PIL token interpreter and its existing decoder
  and access checks. DOS probing is not currently a generated-Rust runner.
  COM fetches are bounded to PSP plus imported bytes; MZ fetches to the loaded
  initial-CS code window. A20 wrap and executable code outside those windows
  are not modeled. Stack/segment/control instructions remain limited to what
  the current instruction profile implements.
- Bridged services include:
  - **INT 10h:** AH=03h (read cursor position and size), AH=08h (read character and attribute).
  - **INT 11h:** BIOS equipment list word determination.
  - **INT 12h:** Conventional memory size query (640 KB).
  - **INT 1Ah:** AH=00h (read system-clock timer ticks).
  - **INT 20h:** CS=PSP exit.
  - **INT 21h:** AH=02h/AH=09h (output), AH=25h/AH=35h (set/get vector), AH=30h (version query), AH=37h/AL=00h (switch query), AH=3Dh (open file failure stub with CF=1, AX=0x0002), AH=3Eh (close file stub with CF=0), AH=48h/49h/4Ah (bounded memory), AH=4Ch (exit).
  Output is capped at 16KiB and a service checks the whole string before
  publishing output. AH09 returns AL=24h and preserves other modeled state.
- These calls are an explicitly labeled host shim, not CPU INT/IRET semantics:
  no interrupt frame, IVT dispatch, BIOS, files, keyboard, environment, memory
  allocation strategies, Ctrl-Break handling or full DOS process lifetime exists.
  Captured bytes are displayed escaped; no CP437 screen rendering is implied.

## Sources and verification

Original hello fixtures and tests are authored for this project; no external
DOS programs, emulator implementation or interrupt-list text is bundled.
Header/entry/relocation contracts were checked against Microsoft's published
[MS-DOS EXEC source](https://github.com/microsoft/MS-DOS/blob/main/v4.0/src/DOS/EXEC.ASM)
and [header definitions](https://github.com/microsoft/MS-DOS/blob/main/v2.0/source/DOSSYM.ASM),
plus the archived [MS-DOS3.3 Programmer's Reference](https://www.pcjs.org/documents/books/mspl13/msdos/dosref33/).
The small output/termination bridge follows the documented DOS API; AH09's
AL=24h behavior is recorded in [Ralf Brown's Interrupt List](https://delorie.com/djgpp/doc/rbinter/id/73/25.html).

Development checks include malformed and relocated MZ images, stack/allocation
bounds, hello execution, output limits, first-blocker reports, bounded loops,
and debugger isolation. `python tools/verify_dos_probe.py` additionally authors
its relocation fixture independently and checks actual CLI output. Runtime export
and native-package smoke checks run both COM and MZ hello files.

Verified on Windows/Linux CI40460972486 at source11dd1cae:200 tests per host,
independent DOS CLI fixtures and both COM/MZ hello examples in standalone source
and unpacked native packages passed. The existing49 backend/native CPU regression
suite also passed; it does not compare DOS-shim execution with generated Rust.

## Disk images and ZIP packages (0.20 alpha)

`--dos-probe` and REPL `dosprobe` also accept FAT16 `.img`/`.ima` and `.zip`
inputs. `--dos-list PATH` lists candidates; `--dos-extract PATH NEW_DIRECTORY`
extracts regular files and one package ZIP level without changing the source.
See [container guide](CONTAINERS.md) for FreeDOS commands, formats and bounds.
Container executables are deduplicated by full byte equality before probing.

## Bounded conventional-memory services (0.27 implementation)

INT21/AH48 allocates first-fit BX paragraphs and returns segment AX;AH49 frees
ES;AH4A resizes ES to BX paragraphs. CF is defined clear on success or set with
AX7 (corrupted MCB),8 (insufficient memory) or9 (invalid/unowned block). Insufficient-memory AH48
and insufficient-growth AH4A return the largest available size in BX. DOS2..6
resize failure still grows to the maximum contiguous size. Upper register
halves and other flags/markers are preserved. Free does not zero bytes;
adjacent free blocks coalesce during allocate/resize,not free. Zero paragraphs
are represented by an owned zero-data MCB. First-fit only; no UMB/strategy,
EXEC/TSR,environment allocator,shared processes or device-owned blocks.

The arena spans MCB0FFFh through conventional-memory end A000h. Each MCB uses
one paragraph with type M/Z,owner PSP (1000h) or0 and data size in paragraphs.
The loader writes a real initial chain and maintains trusted host metadata.
Before services,mirrored type/owner/size must match; direct guest chain edits
are rejected with DOS7 rather than silently overwritten. Names/reserved bytes
are not interpreted. A4096-block safety limit stops the service diagnostically.

Physical probe RAM is now always640KiB,independent of MZ initial allocation.
PSP:0002 retains the original load-time end,as in the FreeDOS handler;
allocation_end tracks the current owned PSP block separately. Successful4A
returns the block segment in AX. Allocation is DOS bookkeeping,not hardware memory protection:
freed/unallocated RAM remains addressable in real mode. The instruction
fetch window is the load-time program block; AH48 blocks above it are not
executable. All MCB writes are prechecked before committing metadata or
register results; readonly header-write failures stop the bridge without partial state.

Reference audit: [FreeDOS INT21 handler](https://github.com/FDOS/kernel/blob/d6791add2043c9d7b584d840a8ffaf8829fd2bdc/kernel/inthndlr.c), especially4A PSP-size retention and AX=ES; [RBIL48](https://fd.lod.bz/rbil/interrup/dos_kernel/2148.html), [49](https://fd.lod.bz/rbil/interrup/dos_kernel/2149.html), [4A](https://fd.lod.bz/rbil/interrup/dos_kernel/214a.html) for the bounded DOS2..6 service profile.

## Real-mode far returns (0.29)

CB and CA imm16 now restore CS:IP and advance SP using shared PIL. Operand16/32
and SS size are independent;the selector uses the low16 bits of its stack slot.
A bad complete frame,new IP beyondFFFFh or RAM access fault commits no return
effects. Flags are preserved. Protected/V86 far returns and far calls/jumps are
not implemented. Fetch bounds now refer to the initial physical window across
CS changes;this permits aliases of imported code without permitting execution
of unpacked/copied code outside the original bounded window.

## DOS version compatibility profile (0.30)

The bounded host shim now accepts INT21/AH30. Its default declared compatibility
profile is6.22 (AX=1606h),initialized in the original PSP at offset0040h.
Guest edits to that word affect subsequent queries. AL=1 on entry selects
non-ROM version flags (BH=0);all other AL values select OEM FF. BL and CX are
zero;register high halves,other registers,EFLAGS and undefined markers are
preserved. A failed PSP read changes no return registers or output. This is
metadata for the shim,not the version of a booted kernel or a claim of complete
DOS6.22 functionality. Other unsupported DOS/BIOS services remain blockers.

The implementation was checked against the
[FreeDOS AH30 handler](https://github.com/FDOS/kernel/blob/d6791add2043c9d7b584d840a8ffaf8829fd2bdc/kernel/inthndlr.c)
for input AL,PSP version and serial fields. The implementation is original;
no kernel code is copied and the shim does not claim to be FreeDOS.

### Measured memory-PUSH progress (0.31)

The actual FD14LITE.img was probed locally on Windows with 100000 steps.
All sixteen FreeCom variants advanced from `FF 74 14` (memory PUSH /6) to
`FF 1E 1F 02` (far CALL /3), twelve bytes later. Imports remain 151/156
and exits remain six. The grouped `opcode ff` total is therefore unchanged;
compare per-program stop details, bytes and CS:IP to measure this progress.
The disk image and third-party programs are not bundled in the repository.

Measured image SHA256: `f539d456b792594bc3ca59d4e0f4c23d4f1fee73370c1390b2da245400718d36`.

### Measured far-CALL progress (0.32)

With the same actual image and 100000-step budget, all sixteen FreeCom variants
pass FF /3 and stop at DOS INT 21/AH=25. GREP passes 9A and stops at memory XCHG
(`87 0E 7A 11`). Missing-encoding program counts are now two `0F BB`, one
`0F 01`, one `60`, and one `87` (five total, previously 21). Imports stay
151/156 and exits stay six. This is first-blocker progress, not completion of
these programs or a booted DOS kernel.


## Interrupt-vector services (0.33)

AH25 stores DS:DX at physical AL*4 as little-endian offset then selector.
AH35 reads that entry into ES:BX and rebuilds the ordinary real-mode ES cache.
Upper EBX and all flags are preserved. The four-byte RAM access is checked
before mutation; failed calls do not retire. Handler pointers are not executed
or dereferenced. Guest IVT writes are visible to AH35. Vectors initially contain
zero; no DOS/BIOS handlers are supplied and CPU interrupt dispatch remains
unimplemented. The existing host shim still intercepts supported INT calls.

Contracts follow Microsoft MS-DOS Encyclopedia INT21 Set Interrupt Vector
(AH25) and Get Interrupt Vector (AH35), and Ralf Brown's Interrupt List.
No third-party implementation copied.

## DOS output, switch query and LES (0.34)

INT21/AH02 reads DL, appends the byte to captured output and returns it in AL.
A tab expands to spaces up to the next eight-column boundary and returns AL=20h.
The column is derived from prior captured bytes: CR resets it, LF and BEL
preserve it, BS decrements toward zero, and tabs advance to the next tab stop.
Other bytes are captured unchanged and advance the column. Existing AH09 capture
remains unchanged. The whole AH02 expansion must fit the16KiB cap before output
or AL changes. Other registers and flags are preserved. Keyboard/break handling,
redirection and a screen/BIOS device are not implemented.

INT21/AX3700 returns AL=0 and DL='/' for the declared DOS6.22 compatibility
profile, preserving upper register bits, other registers and flags. AH37/AL01
and other subfunctions remain explicit unsupported-service stops.

LES C4 accepts real-mode memory16:16/16:32 pointers with independent16/32-bit
address sizes and ordinary segment overrides. Both fields use incoming address
registers and source caches, including an ES override; the full pointer must fit
the source segment and RAM. GPR and ES selector/cache commit together. Flags
are preserved. No protected/V86 load, descriptor lookup or other full-pointer
load instruction is added. Pointer segment-end wrap is rejected by this profile.

Report headers print the package version and selected per-program step budget,
including the default budget when --steps is omitted. Keep the supplied v0.33
100000-step report as historical user evidence until the new corpus is rerun.

## Unsigned division and BIOS / DOS service shims (0.35)

- Unsigned division `DIV` (`F6 /6`, `F7 /6`) is supported at 8, 16, and 32 bits (with 66h prefix) for both register and memory addressing modes. AX, DX:AX, and EDX:EAX dividends produce quotient and remainder in AL/AH, AX/DX, and EAX/EDX. Zero divisor and quotient overflow are checked via PIL `require`. Arithmetic flags CF, OF, SF, ZF, AF, PF are marked undefined.
- Foundational BIOS and DOS services added to the shim profile:
  - **INT 10h (AH=08h):** Read character and attribute at cursor; returns `AH=07h` (standard light gray attribute) and `AL=20h` (blank character).
  - **INT 10h (AH=03h):** Read cursor position and size; returns `CX=0607h` (standard CGA scanlines) and `DX=0000h` (row 0, column 0).
  - **INT 11h:** BIOS equipment list word; returns `AX=0025h` (pointing device installed, 80x25 color display, 1 floppy drive).
  - **INT 12h:** Conventional memory size query; returns `AX=0280h` (640 KB conventional RAM).
  - **INT 1Ah (AH=00h):** Read system-clock timer ticks; returns `AL=0` (midnight flag clear), `CX=0`, `DX=0` (tick count 0).
  - **INT 21h (AH=3Dh):** Open existing file; returns bounded failure fallback with `CF=1` and `AX=0x0002` (file not found).
  - **INT 21h (AH=3Eh):** Close file; returns success stub with `CF=0`.
- Probe report progress: clean exits increased from 11 to 14, FreeCOM shells advance through initialization loops without crashing on missing equipment/video queries, and memory utilities (such as MEMSTAT) calculate and display formatted decimal output using `DIV`.

