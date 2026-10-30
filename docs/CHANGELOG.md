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

# PIL 0.35.0-alpha.1

## 0.35: Unsigned division (DIV) and foundational BIOS / DOS service shims

### Unsigned Division (DIV F6/F7 /6)
- Supported at all operand widths (8-bit, 16-bit, and 32-bit with 66h prefix) for both register and memory addressing modes.
- PIL tokens expanded with binary `div` and `rem` operations.
- Exact divide-by-zero and quotient-overflow bounds checking.
- Undefined arithmetic flags (CF, OF, SF, ZF, AF, PF) follow Intel SDM classification.

### Expanded DOS / BIOS probe services
- **BIOS Video (INT 10h):** Added AH=08h (read character and attribute) and AH=03h (read cursor position and scanlines).
- **BIOS Equipment & Memory (INT 11h, INT 12h):** Returns standard PC equipment list word with pointing device flag, and 640 KB conventional memory size.
- **BIOS System Timer (INT 1Ah):** Added AH=00h read timer ticks.
- **DOS File API (INT 21h):** Added bounded failure fallback for AH=3Dh (Open file, CF=1 / AX=0x0002) and AH=3Eh (Close file, CF=0).
- Probe report results: clean exits increased from 11 to 14, FreeCOM shells advance through initialization loops, and memory tools (MEMSTAT) successfully compute and display formatted decimal output.

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

# PIL 0.32.0-alpha.1

- Add real-mode far CALL: immediate 9A and memory FF /3, 16/32 operand sizes,
  independent 16/32 pointer addressing and SS stack sizes. Read the complete
  pointer before staging CS and return IP/EIP; keep loaded physical fetch bounds.
- Allow at most two ordered staged PIL RAM writes. Validate both before any
  commit; later memory, target or binding faults cancel the entire instruction.
  Loads after the first staged write remain rejected.
- Use a bounded contiguous return frame, matching the existing RETF profile.
  The wide CS slot is deterministically zero-padded, as with segment PUSH.
  No legacy hardware padding, split-frame wrap or shutdown behavior is claimed.
- Windows/Linux CI 40590192023: 312 tests per host and 175 full-state
  cases (7,147,554 matching trace bytes), standalone 110-file runtime, native
  packages and full Mandelbrot checks passed. Local Windows also passed.
  Nine new expected-state tests and fifteen backend cases cover this change.
- Actual FD14LITE.img at 100000 steps: all sixteen FreeCom FF /3 blockers reach
  DOS AH=25; GREP passes 9A and reaches memory XCHG (87). Imports remain 151/156,
  exits six; first missing-encoding programs fall from 21 to five.
- Protected/V86 calls, descriptor gates/tasks and exception delivery remain
  outside this increment. No new ARM or legacy real-mode hardware run claimed.

# PIL 0.31.0-alpha.1

- Add memory PUSH (FF /6) word/dword forms through shared PIL effects.
  Read the source with incoming registers; then decrement SP/ESP and store
  through SS. Operand, address and SS sizes are independent.
- Preserve flags and upper ESP bits on 16-bit stack updates; check full source
  and destination spans before committing the stack write or register effects.
- Add six expected-state tests and eleven full-state backend cases for mixed
  sizes, ESP/SIB addressing, segment overrides, overlap, read-only sources,
  bounds/protection faults and stack wrapping.
- Windows/Linux CI 40587290825: 303 tests per host, 160 full-state
  backend cases (6,951,410 matching trace bytes), standalone 109-file export,
  native package and full Mandelbrot checks passed. Local Windows also passed.
- Actual FD14LITE.img probe at 100000 steps: all sixteen FreeCom memory PUSH
  blockers advanced to FF /3 far CALL, twelve bytes later in each program.
  Summary remains 151/156 imported and six exited. FF totals alone do not
  identify the unsupported group extension.
- No new legacy real-mode hardware or ARM run is claimed.

# PIL 0.30.0-alpha.1

- Add memory POP (8F /0) with independent SS reads and destination addresses
  evaluated after SP/ESP advances. Operand, address and SS sizes are independent;
  faults commit no SP or store effects. The family-specific 16-bit SP wrap case
  with an ESP-based address is rejected pending legacy 386 hardware evidence.
- Add SMSW (0F 01 /4) word register/memory forms and explicit traced MSW state.
  PE follows normalized mode. Initial ET=1 is a declared profile choice.
  Wide register forms and CR0/LMSW writes remain separate work.
- Add DOS INT 21/AH=30 with a declared 6.22 compatibility profile in PSP:0040,
  OEM FF or non-ROM flags, zero serial and preserved upper register halves/flags.
  Other unimplemented services still stop.
- Windows/Linux CI 40582461490: 297 tests and 149 full-state backend cases
  (6,870,394 matching trace bytes), standalone runtime and package checks.
  Twelve new expected-state tests and fourteen backend cases cover this change.
- No new full FreeDOS image, ARM or legacy real-mode hardware run is claimed.

# PIL 0.29.0-alpha.1

- Add real-mode CB/CA RETF16/32 with independent SS sizes and byte cleanup.
- Restore CS selector/cache,IP and SP together;check the complete frame and
  new code limit before commit. Protected/V86 returns remain unsupported.
- Anchor imported fetch windows to physical addresses across CS changes.
  Generated fixed-image dispatch uses physical addresses and live logical IP.
- Local Windows:285 tests passed;135 full-state backend cases include RETF,
  CS aliases,branching/data access after return and precise failure states.
- No new real-mode hardware/ARM/full FreeDOS image run is claimed.

# PIL 0.28.0-alpha.2

- Correct ordinary real-mode CS data permissions in monitor and COM/MZ loaders.
  CS overrides may write data/code; protected Code caches still reject stores.
- Preserve segment limits, RAM protection and loaded-image fetch boundaries.
  Cached modified instruction bytes are decoded again before reuse; the fixed
  compiled-image backend still stops at a changed-code guard.
- Add COM/MZ self-modifying execution, cached instruction invalidation and wider
  independent SETcc condition/addressing checks. Local Windows: 277 tests passed;
  121 interpreter/generated full-state cases matched 6,594,404 trace bytes.
- This increment does not claim a new full FreeDOS-image or ARM execution run.

# PIL 0.20.0-alpha.1

- Read FAT16 images and ZIP stored/deflate packages without new runtime dependencies.
- List/extract files and directly probe executable candidates plus one package level.
- Deduplicate identical container programs;validate CRC,chains,paths and bounded expansion.
- Add synthetic FAT/ZIP,independent zlib vectors and CLI/package interoperability checks.
- Windows/Linux CI40470152015 passed210 tests and CLI/package checks.
- Actual supplied FreeDOS image extracted byte-identically;151 DOS programs imported.
- Correct wrapped MZ CS/SS relocation before physical entry/stack validation.

# PIL 0.19.0-alpha.1

- Add bounded COM and MZ EXE import with relocations and minimal PSP/entry state.
- Add DOS hello fixtures and explicit print/exit host service bridges.
- Report first CPU encoding versus DOS-service blockers and reached instruction mix.
- Add isolated REPL probing, malformed-input and independent CLI checks.
- Windows/Linux CI40460972486 passed200 tests,independent DOS CLI checks,
  both hello formats from standalone/native packages,and all existing regressions.

# PIL 0.18.0-alpha.1

- Add LAHF/SAHF through shared PIL descriptions and implicit AH bindings.
- Preserve untouched state and handle status-byte fixed bits and metadata.
- Extend independent, generated-Rust and CPUID-gated native CPU checks.
- Include the current PIL reference in runtime sources and native packages.
- Windows/Linux CI40446250754 passed188 tests and340,920 native CPU cases
  per host;49 full-state backend cases,full256x256 Mandelbrot and package checks passed.

# PIL 0.17.0-alpha.2

- Add CMC (F5) through shared PIL semantics and binary decoding.
- Preserve untouched state; document concrete incoming-CF metadata policy.
- Add prefix/fetch-fault, generated-Rust and native CPU oracle coverage.
- Windows/Linux CI40443065185 passed184 tests and332,728 native CPU cases per host;
  all48 backend cases and full256x256 Mandelbrot comparisons passed.

# PIL 0.17.0-alpha.1

- Add register XCHG8/16/32 and accumulator exchange forms.
- Extend PIL with staged write src and write_if src; reads remain snapshots.
- Reject conflicting overlapping writes before any memory or CPU commit.
- Expand compiled full-state proof to48 cases.
- Memory XCHG/LOCK,multiple implicit registers and multiple RAM stores remain planned.

## 0.16.0-alpha.1

- Add ROL/ROR8/16/32 with count1,CL,imm8 and register/memory destinations.
- Add CLC/STC; preserve untouched registers,flags and undefined markers.
- Express all semantics in PIL; retain zero/full-turn flag and precise RMW-fault rules.
- Expand compiled full-state proof to45 cases.
- RCL/RCR,CMC and double shifts remain outside this increment.

# Changes

## 0.15.0-alpha.3

- Accept16-to16 MOVSX/MOVZX register and memory sources using existing PIL conversions.
- Add demo extensions with a built-in checker and readable instruction listing.
- Preserve upper register halves, flags, word-source access checks and precise faults.
- Expand compiled proof to39 cases and independent extension/address checks to263,190.


## 0.15.0-alpha.2

- Fix unsupported-program check diagnostics; add all8 demo regression coverage.
- Explain runtime/dev test placement and clarify5 formatting hotspots.

## 0.15.0-alpha.1

- Add MOVZX/MOVSX byte-to16/32 and word-to32, register/memory sources.
- Add CBW/CWDE, CWD/CDQ and LEA with independent operand/address sizes.
- Preserve flags and source-width memory checks; LEA never accesses data memory.
- Permit independently typed source bindings in PIL and generated functions.
- Expand compiled per-instruction proof to34 cases and add exhaustive extensions.


## 0.14.0-alpha.2

- Compare full-size Mandelbrot generated Rust and cached PIL interpreter.
- Gate comparison with the existing 26-case per-instruction verifier.
- Check every pixel, full state at serial drain/halt boundaries and step/retry counts.
- Add -Compare / --compare to render helpers; retain byte guards and black_box.
- Report single-run timing scope and preserve pixel/state/source artifacts.


## 0.14.0-alpha.1

- Add Q12 guest Mandelbrot, reference checking, serial draining and BMP/PGM output.
- Add near Jcc/relative JMP and small interactive image demo.
- Add seven tests (155 total),26 compiled trace cases and independent image oracle.

## 0.13.0-alpha.3

- Show instruction operands, encoded bytes, branch targets and current EIP in list.
- Add list COUNT / list ADDRESS COUNT; make original numeric fixture listings readable.
- Add four listing regression tests (148 total including typed dumps).

## 0.13.0-alpha.2

- Add mem8/mem16/mem32 dumps, printable ASCII and optional unsigned decimal.
- Validate full ranges/counts before output; preserve legacy mem format.
- Add four debugger regression tests (144 total).

## 0.13.0-alpha.1

- Add total shifts/ult/flag_if PIL operations, interpreter and generated Rust.
- Add SHL/SAL, SHR, SAR register/memory/count forms at8/16/32 bits.
- Add six tests (140 total), independent shift oracle and23-case verifier.
- Put direct post-build debugger usage and a linked docs index at the top of README.
- List all seven demos consistently, including factorials/loop, in README and guide.

## 0.12.0-alpha.1

- Add generic sext/mul tokens, interpreter and Rust emission.
- Add truncated signed IMUL16/32 register, memory and immediate encodings.
- Add six tests (134 total), independent signed oracle and 18-case verifier.
- Initialize verifier data before applying read-only protection.

## 0.11.0-alpha.1

- Expand binary ALU directions/immediates and eight instruction descriptions.
- Add six semantic/decoder tests, fourteen-case compiled verifier and logical oracles.
- Update exact supported encodings and references; no PC boot or device changes.

## 0.10.0-alpha.2

- Identify first verifier divergence by case, step, instruction EIP and state field.
- Reject malformed/truncated traces and retain both traces on mismatch.
- Add two verifier regression tests; record alpha.1 phone benchmarks.

## 0.10.0-alpha.1

- Generate and compile fixed-image Rust guest runners using shared PIL emission.
- Add nine-case byte-for-byte instruction traces including faults and serial waits.
- Compare generated Rust and cached interpreter with separate compilation timing.
- Add alpha feature matrix, comparison helpers and shared Windows/WSL setup.
- Add four tests (120 dev tests), retaining all prior emitted-code comparisons.

## 0.9.0

- Cache decoded instructions/PIL with bounded storage and shared immutable programs.
- Revalidate encoded bytes and full image/segment context before every hit.
- Add cache controls/stats and uncached benchmark mode; remain a token interpreter.
- Add 15 regression tests (116 total), including cached/uncached step comparisons,
  guest code modification, runtime permission checks and the full 65,536 loop.

## 0.8.0

- Add guest factorial table and counted loop, with exact output/state checks.
- Add bounded time/bench/stress commands and optimized benchmark scripts.
- Include numeric fixtures in runtime exports and native archives.
- Add seven debugger regression tests (101 development tests total).

## 0.7.0 (development)

- Add 16/32-bit effective addresses, segment overrides and cached-segment checks.
- Share segmented execution with generated Rust, preserving staged stores on faults.
- Add a small binary decoder and a reproducible executable 204-byte selftest.
- Echo successful guest serial output in the debugger without draining it.
- Add compile/debug helpers for Termux/Linux and Windows and native ZIP packaging.
- Use dev for development and main for reviewed deliveries; add host CI.
- Pass 92 Rust tests on hosted Ubuntu/Windows, including 153 segmented comparisons.
- Pass the executable selftest in 63 steps and host build/debug/package checks.

## 0.6.0

- Add flat RAM with little-endian 8/16/32-bit reads, stores and read-only ranges.
- Add PIL read_address/load/store and load_ram/store_ram/add_ram descriptions.
- Defer the single supported RAM store until retirement; preserve CPU/RAM on faults.
- Add structured emulator MemoryAccess errors, distinct from x86 #PF.
- Add debugger memory demo and mem/poke/protect commands.
- Add 19 Rust tests (65 total), including 560 generated/interpreted memory cases.
- Add independent Python memory oracle; update source/verification history.
- New Rust build is not yet run in the authoring environment.


## 0.5.0

- Add bounded RX/TX mock serial with nonblocking reads/writes and atomic injection.
- Add interactive REPL, counter/echo demos, stepping, bounded runs, breakpoints,
  registers/flags/EIP, reset and serial inspection. Harness serial is not UART I/O.
- Add 18 tests (46 total); new Rust build awaits verification.
- Reorganize instructions, tests, tools and documentation; add Git rules and push notes.
- Preserve Cargo.lock/target and the existing ~/pil_proof project.
- Add CLAUDE.md continuation guide and keep provenance/history/checksums.


## 0.4.0

- Add guest EIP, explicit execution context and signed decoded displacements.
- Describe all sixteen Jcc predicates using the same condition definitions.
- Check taken targets against an inclusive supplied protected CS limit;
  return restartable #GP(0) metadata without publishing staged effects.
- Preserve legacy execution APIs for descriptions without flow.
- Add eight Rust tests (28 total), including 12,288 generated-flow comparisons.
- Add independent Python Jcc/context checks and a small decoded loop test.
- Deliver a guarded in-place patch for ~/pil_proof; leave Cargo.lock/target alone.
- Plan serial mock + REPL debugger next, tests/ directory later, original TTGO
  ESP32 compilation after host/embedded separation.
- Record both user-confirmed v0.3 test runs and continue reference tracking.


## 0.3.0

- Require a defined/preserved/undefined classification for every arithmetic flag.
- Track undefined outputs separately from concrete flag values.
- Add AND, sixteen byte-only SETcc predicates, select and write_if.
- Add six Rust tests; extend differential checks to metadata and new operations.
- Record v0.2's successful 14-test Termux result.
- Add revision history, JSON evidence records and source checksums.
- Preserve reference/attribution tracking; no third-party code imported.


## 0.2.0

- Added standalone SUB, CMP, ADC and SBB descriptions at 8/16/32 bits.
- Added generic modular subtraction and incoming-flag read tokens to the
  validator, interpreter and Rust emitter.
- CMP has no destination write; ADC/SBB use the incoming CF snapshot.
- Extended emitter CLI: --emit-rust WIDTH [add|sub|cmp|adc|sbb]. Existing
  --emit-rust WIDTH still selects ADD.
- Added seven Rust tests; expanded generated-source comparison to 15 variants.
- Added optional independent Python description checks.
- Added a maintained references/credits register distinguishing consulted
  material from imported code and tracking licence/attribution findings.
- No x86 decoder, memory, PC devices or ESP32 port added in this milestone.

## 0.1.0

Initial ADD proof. User reported all seven Rust tests passing in Termux.
