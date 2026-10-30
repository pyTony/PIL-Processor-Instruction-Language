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

# Comparison strings and memory SETcc verified — 27 October 2026

Windows/Linux CI40562311100 passed on d7697f8994a41096070073d2ea64e7526c554cd0:
273 tests per host,118 full-state backend cases (6,577,356 trace bytes),existing
340,920 native overlap cases,full256x256 Mandelbrot and standalone105-file
runtime source/native-package checks. Eight new expected-state tests cover
comparison flags across widths,address sizes,DF,source/ES rules,zero count,
new-ZF termination,count exhaustion,entry-flag recovery/fault retry,all16 memory
SETcc conditions,readonly stores and bounded REP budgets. No new hardware-oracle
or ARM run is claimed.

CMPS/SCAS8/16/32 use shared subtraction flags and pointer effects. REPE/F3 and
REPNE/F2 evaluate new ZF after each bounded element. Generic checkpoint_flags
remembers entry EIP/EFLAGS/undefined metadata across partial resumes;execution
faults restore entry flags while retaining prior pointer/count progress. Full
retirement clears checkpoint state. It is included in machine traces and the
interpreter/generated wrappers share the same lifecycle. SETcc memory uses
existing select/store effects,including zero stores and fault checks.

Memory PUSH/POP needs post-increment ESP effective-address semantics and remains
separate;SMSW needs explicit machine-status state. Interrupt/exception delivery,
full prefix/mode audit and copied-code/CS-write model work remain. Version0.28
follow-up changes only versions/docs/evidence relative to the tested code.

# LEAVE and bounded DOS memory verified — 27 October 2026

Windows/Linux CI40536350302 passed on 4256a139aca4a21b997483a967de25de7860984c:
265 tests per host,103 full-state backend cases (6,089,907 trace bytes),existing
340,920 native overlap cases,full256x256 Mandelbrot and standalone103-file
runtime source/native-package checks. Thirteen new expected-state tests cover
frame sizes/base/limits/readonly reads and faults,MCB headers,PSP extent,first-fit
allocation,reuse/coalescing,zero/exact sizes,error results,failed growth and
all-or-none MCB permission checks. No new hardware-oracle or ARM run claimed.

LEAVE uses shared PIL effects;generated dispatch now considers named SP writes
when selecting alternative SS sizes,including LEAVE after a real-mode SS load.
ENTER remains separate. DOS48/49/4A use a bounded single-process first-fit MCB
arena with visible M/Z,owner/size fields and CF/AX/BX results. Resize failure
can grow to the maximum contiguous size,as documented for DOS2..6. Free keeps
bytes and delays coalescing until allocate/resize. Direct chain edits are
rejected rather than overwritten. Physical640KiB RAM and current PSP allocation
extent are distinct; instruction fetch windows remain bounded as before.
The source audit corrected original PSP:0002 retention and successful4A AX=ES
before the final verification run. These are checked against FreeDOS handler
commit d6791add2043c9d7b584d840a8ffaf8829fd2bdc,not assumed from field names.
No UMB,strategy,EXEC/TSR,environment allocator,interrupt delivery or full DOS
kernel is claimed. Version0.27 follow-up changes only versions/docs/evidence.

# Real-mode flags and counter branches verified — 27 October 2026

Windows/Linux CI40527735310 passed on 0e673f4f6db3b368bc37145803e53fca4f28f827:
252 tests per host,96 full-state backend cases (6,026,172 trace bytes),existing
340,920 native overlap cases,full256x256 Mandelbrot and standalone101-file
runtime source/native-package checks. New expected-state tests check visible
386 FLAGS bits,reserved/RF/VM handling,independent operand/SS sizes,precise stack
faults,mode rejection,JCXZ count/target sizes,limits and STI delay through REP
fault/retry. No new hardware oracle or ARM run is claimed.

FA/FB CLI/STI and9C/9D PUSHF/POPF16/32 are a real-mode subset;protected/V86
contexts are explicitly rejected. STI delay is separate from SS shadow and
included in full traces. IF-clear to IF-set creates delay;consecutive STI does
not extend it. Full PIL/monitor-I/O/DOS-bridge retirement advances shadows;
partial REP and faults preserve them. No interrupt/exception/NMI delivery or
legacy stack-shutdown behavior is claimed. JCXZ/JECXZ uses address size for the
counter and operand size for target wrapping,with flags/count untouched.
The0.26 follow-up changes only versions/docs/evidence after verification.

# Near CALL/RET and REP movement verified — 27 October 2026

Windows/Linux CI40511872914 passed on b0ab7c8159ac107895161cb0bfe331e5fef6d728:
243 tests per host,84 full-state backend cases (5,816,602 trace bytes),existing
340,920 native overlap cases,full256x256 Mandelbrot and standalone96-file runtime
source/native-package checks. These new instructions have independent expected-
state tests and interpreter/generated-Rust comparisons;no new hardware-oracle or
ARM run is claimed. The first run passed242 tests and failed only an old test
that expected CALL AX unsupported;it now checks the invalid far-register form.

Near E8/FF2 CALL and C3/C2 RET use shared per-access segment effects,independent
operand/SS sizes,pre-write target snapshots and transactional stack faults.
MOVS/STOS/LODS8/16/32 support both address sizes,DF and source overrides/fixed ES.
F3 REP commits one element per execution step;zero count avoids data accesses,
fault retry retains prior progress,and SS shadow retires only at completion.
DOS budgets include element steps;completed instructions count full retirements.
No far calls,CMPS/SCAS/F2,external interrupt delivery or compiled changed-code
fallback is claimed. Memory PUSH/POP and broader mode/prefix audits remain.
The0.25 follow-up changes only versions/docs/evidence after verification.

# Additional MOV forms verified — 27 October 2026

Windows/Linux CI40500511068 passed ate74c95100ee5fff346b229a2c4cf45643ad25922:
235 tests per host,67 full-state backend cases (5,417,290 trace bytes),existing
340,920 native overlap cases,full256x256 Mandelbrot and standalone91-file source/
native-package checks. A0..A3 and C6/C7 use existing PIL load/store/constant/write
semantics;no instruction-specific backend operations were added. Operand/address
sizes,high bytes,SS defaults,SIB/overrides,readonly/boundary/fetch faults checked.
The first run's only failure was a test fixture using a>64KiB offset with a64KiB
cache;the fixture now supplies a larger cache and separately checks rejection.

Normal-build warnings fixed:remove unused mut;gate legacy segmented wrapper and
empty-state trace record helper as test-only. No new hardware oracle/ARM run is
claimed. The0.24 follow-up changes only versions/docs/evidence after verification.
Next remains independent per-access contexts,near CALL/RET and one-element
strings before restartable REP. Real-image evidence is recorded separately below.

# Real-mode segment transfers verified — 27 October 2026

Windows/Linux CI40498283191 passed at6b2f7d1ceceb57f73786f01d17b0e40d09cc740b:
231 tests per host,62 full-state backend cases (5,336,578 trace bytes),existing
340,920 native overlap cases,full256x256 Mandelbrot and standalone91-file source/
native-package checks. New trace sections compare selectors,caches,mode and SS
shadow after every instruction. No new segment hardware-oracle/ARM run claimed.

Local execution of the Linux CI artifact on unchangedFD14LITE.img imported151/156:
116 encoding stops,24 services,8 model faults,3 exits. JEMFBHLP and MOVEXBDA now
print their usage and exit0;VCHKBOX exits36. This proves usage paths,not drivers.
Counts:404MOV,82PUSH,31POP;segment MOV/POP first blockers are gone. The newly
reached SHARE CS-write fault is the same model limitation as the prior7 faults.
REP54,CALL18,moffs A0(17)/A3(3),STI16,PUSHF5,LODSB2,C7(1) remain first blockers.

PIL read_segment/write_segment and execute_machine share real-mode updates in
both backends. Segment POP uses old SS/stack width;SS load records boundary
inhibition until successful following retirement. External interrupt/NMI/debug
and protected loads remain unsupported;MOVr32,Sreg is rejected pending legacy
upper-half policy. Memory PUSH/POP and per-access contexts remain next alongside
moffs/CALL/string priorities. Runtime0.23 is a version/docs-only follow-up to
the tested implementation;user reviews/merges delivery PR.

# Real-mode segment transfers — hosted verification pending

Shared PIL selector reads/staged writes support MOV8C/8E and POP ES/SS/DS/FS/GS.
Real-mode cache rebuilding,SS-shadow state and runtime addressing are shared
between interpreter and generated code. Compiled runners select stack variants
from current SS;full traces include selectors,caches,mode and shadow. No protected
loads,legacy MOVr32,Sreg upper-half policy or interrupt delivery is claimed.
Eight new unit tests and six trace cases are awaiting Windows/Linux CI.

# Initial stack forms verified — 27 October 2026

Windows/Linux CI40487360431 passed at9189e64e7d18f9b0b87b55c6b2bba549c00a5236:
223 tests per host,56 backend cases (5,133,394 trace bytes),existing340,920 native
CPU overlap cases and full256x256 Mandelbrot checks. The stack checks include
independent full-state expectations for generated Rust,operand/SS-size matrices,
SP/ESP snapshot and POP alias ordering,precise RAM/SS/readonly faults and cache
identity. Native overlap counts do not include new legacy stack forms;no external
386 suite or new ARM run is claimed. Standalone90-file source/native package
checks passed. 0.22 delivery changes version/docs/evidence only after this run.

All general-register PUSH/POP (including FF/6 and8F/0 aliases),PUSH imm16/32/
sign-extended imm8 and PUSH ES/CS/SS/DS/FS/GS are admitted. Explicit visible
selectors live alongside caches in Image and are part of decode-cache identity.
The generic PIL foundation is snapshot read_gpr and ordered staged write_gpr;
it remains a precise RAM-only single-store model. Memory stack operands,
segment POP/MOV,CALL/RET,interrupt shadows and per-access contexts remain next.

# Initial stack foundation — hosted verification pending

Added snapshot read_gpr and ordered staged write_gpr effects in both backends.
Register PUSH/POP and immediate PUSH use independent SS stack-size state;
ES/CS/SS/DS/FS/GS pushes read explicit visible selectors, included in cache identity.
Tests cover register/size matrices, SP/ESP alias ordering, precision on SS/RAM/
readonly faults, independent emitted expectations and six new backend cases.
Memory PUSH/POP and segment POP are not implemented in this slice.
No Rust result is claimed before Windows/Linux CI. The implementation remains
RAM-only with one store; generalized multi-access/MMIO work remains P0.

# CLD/STD verified — 27 October 2026

Windows/Linux CI40476098533 passed at5ec02f081aa618da0d44e541bb6d73da677428b4:
215 tests per host,50 backend comparisons (5,008,114 trace bytes),98,304 independent
Python direction cases and existing340,920 native overlap cases. The separate
ABI-safe CLD/STD hardware test ran on both hosts. Full256x256 Mandelbrot and
standalone88-file source/native package checks passed. No new ARM run claimed.

0.21alpha.1 delivery follows this tested implementation; follow-up changes only
version/documentation/evidence. Stack addressing and ordered effects remain P0
before PUSH/POP. User reviews/merges runtime PR; project uploads authorized.

# CLD/STD follow-up — implementation awaiting hosted verification

Added explicit DF to PIL flags, CLD/STD descriptions and FC/FD decoding. Other
flags, registers and undefined markers are retained; defining DF clears only
its undefined marker. Python independently passed 98,304 cases locally. Five
Rust tests add full-state, generated-source, decoder/prefix and x86-64 ABI-safe
native checks. One new trace case brings the expected backend count to 50.
Rust is unavailable locally; Windows/Linux CI has not yet confirmed this slice.
Updated stale ISA baseline to the verified 0.20/210-test result below.
Ignored local images/ in both dev and runtime exports.

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

# GitHub authorization — 27 October 2026

The user explicitly authorized all project source, tests, development documents
and runtime uploads to pyTony/pil_proof: "yes anything for there, go ahead".
Proceed with the CMC dev upload and CI; prepare a runtime delivery after checks.
Main remains a strict runtime projection and delivery PRs remain for user review.

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

# Two bound register effects and XCHG supplied —27 October2026

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

# Resume point after independent work —27 October2026

Stable dev:0.16.0-alpha.1,175 tests and321,784 native cases per Windows/Linux
host. Runtime-only PR19 is ready: https://github.com/pyTony/pil_proof/pull/19
Source21d65da,delivery e010681. Main remains0.15.0-alpha.3 until user merges.
The175 tests include six new rotate/control tests; do not infer full386 coverage.
No SingleStepTests/test386 vectors or ROMs were run/imported. No project license
has been selected. Do not silently introduce GPL source or an external dependency.

Next substantial work: named multiple-register effects and ordered memory commits
(P0) before stack/CALL/RET; separately start the pinned external386 adapter.
For next bit-operation slice,RCL/RCR need a clear incoming-CF metadata policy;
SHLD/SHRD need independent source/count bindings. CMC is also still planned.
Keep these semantics in PIL,not hand-coded in one execution backend. Retain
literal/native separate encodings,independent defined/preserved/undefined masks,
zero-count retirement/permission checks,alias snapshots and full fault state.

User can stay ondev and run python tools/verify_host_cpu.py onx86-64. Phone
may run cargo test --locked; its hardware test is ignored and must not be counted
as native verification. Main contains no dev test files or handoff. Generated
artifacts stay under target/dist; do not create extra home-version directories.

# 0.16 rotates and carry constants supplied —27 October2026

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
RCL/RCR,CMC and double shifts remain planned. Runtime PR: https://github.com/pyTony/pil_proof/pull/19
Keep dev separate from main; user reviews/merges the delivery PR. No external386 suite execution claimed.

# Independent overnight work: shifts and multiplication —27 October2026

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

# Logical hardware oracle expansion —27 October2026

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

# Unary hardware oracle expansion —27 October2026

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

# Initial actual-host CPU oracle —development only

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

# 386 completion and modern CPU/x86-64 TODO plan —27 October2026

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

# User Windows alpha.3 verified —recorded27 October2026

Runtime PR #18 is confirmed merged.
User supplied Windows/x86_64 release comparison10000/5:all39 cases matched
4,494,466 trace bytes. Generated median0.5976 ms (50,205,823 guest ins/s),
interpreter4.723 ms (6,352,530 ins/s),7.90x; generated compilation4.271 s
excluded. Cache29997 hits/7 misses/0 invalidations. Direct CPU baseline passed,
0.0022 ms/0.220 ns per iteration,narrower scope; not386 cycles.
Interactive extensions listing bytes/operands confirmed;15 completed instructions,
haltEIP12d,checkPASS. EAX/ECX=FFFFFF80,EDX=FFFFFFFF,EBX=80,ESP/EBP/EDI=FF80,
ESI=85. Debug build2.06 s/release2.27 s separately reported. No execution changes.
External hardware suites remain researched/planned,not yet imported or executed.

# Runtime PR #18 ready —0.15alpha.3

https://github.com/pyTony/pil_proof/pull/18; runtime head
a1babcdbee46d9c99a9e4ae7e62c2866478f211c,source dev
6b152b91e9d74e3bf3fc5c537351b4b6776ac9ec,base main
ecfdb9a673af6d833d4000814a3299f5ed0986bb.165 tests/39 trace cases passed
Windows/LinuxCI40408105698. Same-width conversions and15-step checked
extensions demo;72-file runtime-only export. After user merges/pulls:
demo extensions;list64;run1000;check;regs;mem16 0x180 1 hex.
External suites researched/licences pinned; plan ondev,none executed/imported.
User review pending; no tag/release. Next work: small pinned SingleStepTests
CPU adapter,then staged stack/control flow toward ROM-level testing.

# Same-width support/demo verified; external-suite plan recorded

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

# Same-width extension compatibility —0.15.0-alpha.3 development

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

# Claude release/ISA smoke results recorded —0.15alpha.2

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

# Runtime PR #17 ready — diagnostic follow-up

https://github.com/pyTony/pil_proof/pull/17 delivers0.15.0-alpha.2.
Runtime head9c40f41cec5971c740573734ad72d456691bbc91,source dev
0ced9f0e9263162f7ffe4259be9f2020d4f8c93a,base main
33377e984e4e7d54b99cf6db2f1e9c57b238b35c (merged ISA PR16).
164 tests/34 trace cases and runtime/native/full-image checks passed both hosts
in CI40405704118. Runtime72 files; tests remain ondev. User reviews/merges.
No new phone0.15 result; no tag/release. Next ISA work: staged stack/call/return.

# 0.15.0-alpha.2 diagnostic follow-up verified

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

# Diagnostic follow-up will be0.15.0-alpha.2

PR #16 merged original ISA head0d4da793e2159ff21ccd129d034d0738a1adfe1d.
Runtime check-message/README/formatting follow-up needs a separate alpha.2 PR.
Core patch54149d44cba6079d4ccf4f1b1716ac2f4511821a is being verified by
CI40405704118 (all164 tests already passed on both hosts; release checks pending).
Version/docs metadata bumped after core patch; execution logic unchanged.

# Claude runtime review follow-up

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

# 0.15 instruction expansion verified on Windows/Linux

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

# Instruction expansion —0.15.0-alpha.1 development

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

# User phone full-image comparison confirmed —0.14.0-alpha.2

User Termux android/aarch64 release --compare-mandelbrot256/32 passed the
26-case gate (4,273,138 exact per-instruction trace bytes),all65,536 reference
pixels and256 CPU/RAM/flags/queues/stop checkpoints (1,134,592 exact bytes).
Both engines completed12,940,344 guest instructions and255 TX retries.
Generated Rust151.897 ms vs cached interpreter2991.931 ms,19.70x in one cold
render each. Verifier compilation17.165 s and image compilation2.706 s excluded.
Checkpoint capture/draining included; setup/reference/files/startup excluded.
Not a median benchmark or386 cycle measurement. Runtime PR #15 is confirmed
merged. No execution changes.

# User phone alpha.1 comparison confirmed

Termux android/aarch64 release0.14.0-alpha.1,10000 loop iterations,5 measured
samples plus1 excluded warmup:all26 cases/4,273,138 trace bytes matched,
including mandelbrot2 (291 steps),near branch and jump faults. Generated median
0.700462 ms (42,833,159 guest ins/s),cached interpreter7.168 ms (4,185,439 ins/s),
10.23x; generated compilation16.910 s excluded. Cache29997 hits/7 misses/0
invalidations. PC_NATIVE unavailable on ARM64 as expected; emitted Rust does
execute natively on the phone. This is the counted-loop comparison,not a full
256x256 render or alpha.2 --compare-mandelbrot result. No execution changes.

# Runtime PR #15 ready for user review

https://github.com/pyTony/pil_proof/pull/15 delivers0.14.0-alpha.2.
Runtime head46053bcd2193060477efa43c0cbc6a8647b5e5c1; source dev
441844483574e5b55c7eb8691fdce7ff828a6a4c; base main
cf8cd1cbdc1843f4a92db9de44a820ace3dacf1c.66-file runtime-only projection.
Windows/Linux157 tests and full-image comparison passed CI40384865363.
After user merges:git pull --ff-only origin main; Windows
.\tools\mandelbrot.ps1 -Compare; shell sh tools/mandelbrot.sh --compare 256 32.
No user alpha.2 full-image comparison reported yet. Keep follow-up development
on dev; main only runtime exports. No release/tag published.

# Full-image backend comparison verified —0.14.0-alpha.2

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

# Full-image compiled comparison —0.14.0-alpha.2 development

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

# Mandelbrot verified — runtime PR #14

https://github.com/pyTony/pil_proof/pull/14 awaits user review. Core source
6c7297ea7c4075e73bb5738167ead7a496152f11 passed CI40378440616:155 tests,
Ubuntu13.60 s/Windows17.87 s;26 compiled cases/4,273,138 matching trace bytes.
Full256x256/32 rendered65,536 exact pixels in12,940,344 instructions,255 TX
backpressure retries:Ubuntu1599.244 ms/Windows1664.788 ms (single renders).
Independent Python PGM/BMP checks passed,rasterSHA256
3855106a06087a62fdfc6e322767988e441137f5e13071b473f43e2f20140619.
Standalone66-file source,helpers,emitted Rust and unpacked render passed.
Full image is interpreted; compiled proof includes only tiny2x2 image. Output
BMP/PGM/guest.bin/report stays under target/mandelbrot; no VGA/OS/firmware.
Delivery head daa9e99d095e7202006598401d1905290edd4acc based on main df69e8d622b42167b44ad6eded4f0826736e04bc.
No user0.14 run yet; no release/tag published. WSL0.13 comparison recorded.

# Mandelbrot workload development —0.14.0-alpha.1

Original dynamically assembled Q12 guest renders powers-of-two2..256 with1..64
iterations. Default CLI256/32,debugger demo8/16. Serial mock streams pixels;
host drains TX on backpressure,checks every pixel against math reference before
writing BMP/PGM/guest.bin/report under target/mandelbrot. No larger guestRAM
or VGA. Runtime full image stays token-interpreted; tiny2x2 case joins compiled
proof (26 cases). NearJcc0F80..8F,relative JMP EB/E9 use PIL flow. Seven tests
added (155 total pending CI). Full256/32 release render and independent Python
PGM/BMP oracle added to CI on both hosts. Runtime66 files. CLI render validates
bounds before artifacts and limits instructions. Next: image performance/native
backend and additional device/machine-state support,without implying OS boot.

# Readable listing runtime PR #13 ready

https://github.com/pyTony/pil_proof/pull/13 contains the alpha.3 listing follow-up.
Delivery head6b193b23c11946855b87d4bf6221ddde74fe5a6c based on main7efaf780d0041b9c6c56260810f87940a4b4d175;
source dev d57ca83e80ec1ce247478a59be9616aa1610546a. Execution code matches
CI-tested0e2d8d495f84c27830dda4ae0cf31579247e9afd (148 tests,23 matching
cases); version/docs metadata updated separately. PR #12 already merged
memory dumps. No user alpha.3 listing result yet; no release/tag published.

# Readable listings verified — alpha.3 follow-up

User merged typed-dump PR #12 before the listing addition; main7efaf780d0041b9c6c56260810f87940a4b4d175
contains dumps,not readable listings. Prepare separate runtime alpha.3 PR based
on that main. Listing code0e2d8d495f84c27830dda4ae0cf31579247e9afd passed
CI40374526560:148 tests,Ubuntu8.32 s/Windows18.29 s;23 cases/3,044,833 bytes.
Standalone62-file source/emitted/native package checks passed. Version/docs
metadata changed toalpha.3 afterward; execution source unchanged. Preserve
main/runtime separation; no user listing run or release/tag claimed.

# Readable instruction listings added to pending dump delivery

User found factorial listing uninformative. Decoder labels now contain actual
registers,immediates,memory sizes/segments/EA expressions and branch targets.
list shows bytes and * atEIP; list [COUNT] and list ADDRESS COUNT accept1..256.
Default16; stop atHLT/end/error. Predecoded examples show only existing labels.
Formatting is derived from decoded operands,not a separate decoder. Four tests
added (148 total pending CI). Numeric fixture generator now names actual
registers instead of r0 etc.; guest bytes unchanged. Append to pending PR #12
if still open,or make a new runtime-only follow-up if user already merged it.

# Typed dumps verified — runtime PR #12

https://github.com/pyTony/pil_proof/pull/12 awaits user review. Dev code
381cac63ee71c552d248177ba0156e07ddce7b79 passed CI run40373417827:
144 tests,Ubuntu12.39 s/Windows16.80 s. Existing23 cases match3,044,833 bytes;
standalone62-file source, launchers, generated Rust and unpacked native checks
passed. Runtime head52834b3e0dbefbf724408b21dedb4cce19647568 is based on
main4e87b848590f85a842d03b84bd23bb6a3fcbbaa9 after merged PR #11.
New mem8/mem16/mem32 values/ASCII/decimal documented; no dump user run yet.
User Windows0.13 alpha.1 multiplication/shift proof passed all23 cases and
REPL startup. No release/tag published. Next: fixed-point Mandelbrot workload.

# Typed memory dumps —0.13.0-alpha.2

New mem8/mem16/mem32 ADDRESS [COUNT] [hex|dec]. Count is values1..256;
defaults16/8/4 values (16 bytes). Little-endian,unaligned,16-byte rows,
printable ASCII0x20..7e with dots for others. Decimal is unsigned.
Full-range validation before formatting; read-only operation, legacy mem unchanged.
Four tests added (144 total pending CI). Existing23 backend cases unchanged.
User merged PR #11 while dumps were under development. Prepare a separate
runtime-only dump PR based on that new main. Never merge dev into main.
No external sources/dependencies used for this debugger feature.

# Shifts and documentation verified — runtime PR #11

https://github.com/pyTony/pil_proof/pull/11 is ready for review; supersedes
unmerged PR #10 and includes its signed IMUL implementation. Core source
1a35fa53fcddd5a89e86813699736e5cc3c68832 passed CI run40371543845:
140 tests,Ubuntu12.43 s/Windows22.00 s. All23 cases matched3,044,833 bytes.
Standalone62-file source, scripts, emitted Rust and unpacked deliveries passed.
Python59,940 shifts+65,680 IMUL+1,184,058 arithmetic/logical+1,030 conditional
cases passed. Later docs-only changes have separate link/checksum checks.
Docs indexes, direct launch examples, all7 demos and default100-step behavior
are documented following user feedback. User PC remains on main; no new0.13
phone/PC run supplied. Next: actual fixed-point Mandelbrot image workload.
Keep dev/main separation; no tag/release published.

# Shifts and debugger discoverability — 0.13.0-alpha.1 development

SHL/SAL,SHR,SAR8/16/32 supportcount1/CL/imm8 and register/memory. Generic
shl/lshr/ashr total shifts, ult comparison and flag_if conditional flag effects
keep x86 count masking/flags in PIL. Counts mask31; zero preserves flags and
markers; CF undefined for SHL/SHR count>=width, OF undefined ifcount>1,
AF undefined ifnonzero. Memory RMW validates writes even atzero count.
Six tests added (140 total pending CI);23 verifier cases;62 runtime files.
Independent Python shift oracle added. README now starts with direct launch
commands because user couldn't find them after building (guide was in docs).
Current main remains0.11; runtime projection also includes0.12 IMUL. Prepare
one runtime-only PR based on current main, superseding PR #10 if unmerged.
Next: actual fixed-point Mandelbrot image workload (shifts/multiply ready).

# User PC full comparison verified — 0.11.0-alpha.1

Windows/x86_64 user run10000/5 matched14 cases/2,460,193 bytes. Generated
median0.293500 ms,102,224,872 ins/s; interpreter4.080 ms,7,353,496 ins/s;
13.90x; compilation2.294 s excluded. Direct CPU arithmetic passed,0.002200 ms
median/0.220 ns per iteration; narrower scope, not guest386 cycles. Earlier
standalone10000/3 interpreter median3.216 ms/9,330,161 ins/s is a separate run.
Both phone and PC0.11 comparisons now confirmed.0.12 user verification remains
pending. No new tests or implementation changes.

# Signed multiplication verified — runtime PR #10

https://github.com/pyTony/pil_proof/pull/10 is ready for review. Core dev
c1a91bcaa82a555e2b7ec3d4682729d6619ff93a passed CI run40367901727:
134 tests, Ubuntu10.65 s / Windows18.35 s. Eighteen full-state cases matched
2,535,361 trace bytes. Standalone59-file source, emitted Rust, launchers and
unpacked native deliveries passed. Python65,680 IMUL cases passed in addition
to1,184,058 arithmetic/logical +1,030 conditional cases. Runtime delivery head
7cada78793347cc676a288cafd7b33608ac5c431 based on main37848d2753fa8a7724caf2208dd27959bf0b8c63.
No phone or user PC run yet for0.12. No tag/release published. Next: shifts
and a fixed-point image workload; keep development on dev and main runtime-only.

# Signed multiplication development — 0.12.0-alpha.1

Generic sext and modular mul tokens added; imul.pil uses double-width products,
truncation and sign restoration to compute CF/OF. PF/AF/ZF/SF undefined.
Decoder adds0F AF,69/6B for16/32 bits, memory sources and signed immediates;
no implicit AX/EDX:EAX form or byte multiply. Six new tests (134 total; host CI passed (see above));18 verifier cases;59 runtime files. Generic boundary tests through64 bits
compare emitted Rust with interpreter and independent expectations. Python
oracle65,680 signed IMUL cases in addition to existing arithmetic/conditions.
Fixture initialization patches RAM before applying read-only protection in both
runners. Windows PC comparison uses target/windows; WSL target/wsl.
Next: shifts for fixed-point Mandelbrot; then an actual image workload.

# User phone expanded ISA verified — 0.11.0-alpha.1

User Termux android/aarch64 run sh tools/compare.sh 10000 5 passed all14
cases and matched2,460,193 bytes. ALU8/16/32 each31 steps; read-only TEST
succeeded and NEG faulted before retirement. Generated median0.690346 ms,
43,460,815 guest instructions/s; cached interpreter7.091 ms,4,231,275 ins/s.
Observed10.27x; generated compilation10.230 s excluded. Five samples plus one
excluded warmup. User PC comparison remains unreported. This is a fixed-image
proof, not native REPL/JIT or 386 cycles.

# Instruction expansion verified — PR #9

Runtime-only PR https://github.com/pyTony/pil_proof/pull/9 is ready for review.
Core dev aab72b00d47f77aa31ad30abb404070472fcb49c passed CI run40364437807:
128 tests, Ubuntu7.50 s / Windows16.08 s. Fourteen full-state cases matched
2,460,193 bytes; isolated58-file runtime, emitted Rust, launchers and unpacked
deliveries passed. Python1,184,058 arithmetic/logical +1,030 conditional cases
passed. Runtime head b043980156a81306b1957e9d56047ef737b3ed82, based on
main3923d9f0020fc0858097c5e18bf4ab1ff1bb8465. No new phone run reported.
No release/tag published; preserve dev/runtime-main branch policy.

# Expanded instruction profile — 0.11.0-alpha.1 development

OR/XOR/TEST/NOT/NEG/INC/DEC/NOP added in PIL and monitor decoder. Binary
ALU supports register/memory both directions, accumulator immediates and
80/81/83 groups. F6/F7 /0 TEST,/2 NOT,/3 NEG; FE/FF /0 INC,/1 DEC;
40..4F INC/DEC;90 NOP. Unsupported-opcode tests now use62 instead of90.
Operand locations are bound into PIL before validation; no duplicated CPU flags.
Six new tests,128 total; host CI passed (see above); compiled proof14 cases (ALU8/16/32,
readonly TEST/NEG added). Runtime export58 files. No stack/multiply/shifts yet.

# User phone compiled backend verified — alpha.2

User ran sh tools/compare.sh 10000 5 on Termux android/aarch64.
All9 cases matched2,046,769 bytes. Generated median0.764269 ms,
39,257,120 guest instructions/s; cached interpreter median6.890 ms,
4,354,547 ins/s. Observed9.02x; compilation5.201 s excluded.
PC_NATIVE is explicitly unavailable on aarch64, as expected; generated Rust
compiled and executed natively for the phone. User PC comparison remains pending.
This is a guarded fixed-image proof, not a native REPL or 386 cycle estimate.

# Alpha.2 verification complete — runtime PR #8

https://github.com/pyTony/pil_proof/pull/8 awaits user review. Core dev source
cdb5fc396617071a134fc65ce67807cf56dc68f3 passed CI run40360385843:
122 tests on Ubuntu (8.35 s) and Windows (12.67 s). Nine cases matched
2,046,769 trace bytes; verification-only command, isolated 50-file runtime,
generated Rust and unpacked native deliveries passed. PR delivery head
f09ec201a353b7ee5c9d7bcf18b281ddbbfc74fc. No release/tag published.

# Verifier diagnostics — 0.10.0-alpha.2 development

Alpha.1 runtime PR #7 was merged as dc9e3f3351aca235aea2d20da48f88a966e328cc.
User Termux alpha.1 release loop 10000/5 passed: uncached median368.701 ms,
81375 ins/s; cached7.135 ms,4205340 ins/s,29997 hits/7 misses/0 invalidations.
Observed cache ratio51.67x. Generated-Rust phone and user PC runs remain pending.

Alpha.2 adds first-divergence case/step/instruction EIP/field diagnostics,
bounded trace parsing (including equal malformed inputs), and saves both
trace files before comparison. Two regression tests added,122 total; host CI passed (see verification above).
No new guest instructions. Develop on dev; export runtime-only delivery to main.

# Alpha verification complete — PR #7

Runtime-only alpha candidate: https://github.com/pyTony/pil_proof/pull/7
Core dev source ca195663533ad41a6f50952442bf6a38d1a92447, CI run40341288842:
120 tests on Ubuntu (8.20 s) / Windows (14.03 s); verifier matched 2,046,769
trace bytes across9 cases. Verification-only mode and isolated/unpacked native
comparison passed. Compiled Rust vs cached interpreter 10000/5 samples:
Ubuntu0.380 vs3.077 ms; Windows0.421 vs4.664 ms. Direct x86-64 CPU arithmetic
baseline0.003391/0.003100 ms is a different scope, not 386 MHz. All timings
exclude generated compilation (1.818 /3.346 s) and process startup.
User phone generated/native and PC results remain unreported. Alpha candidate
PR includes feature matrix/shared Windows-WSL build targets. No release/tag
published. Keep development on dev and runtime-only projection on main.

# v0.10.0-alpha.1 development

User authorized generated-Rust comparison and alpha feature matrix; alpha is a
candidate PR, not a published GitHub release/tag. New native.rs composes
fixed-image Rust runners from emit_rust_functions (shared with --emit-rust),
compiles with rustc -O and compares exact full-state instruction traces via
trace.rs. Nine cases: selftest/hello/factorials/loop17, store/branch faults,
changed-code stop and RX-wait/TX-full. Timed native loop guards code and uses
black_box per step; no JIT or native REPL. Compare helpers build release.
120 dev tests pending CI; runtime export50 files. Native proof needs local
rustc. Normal packaged debugger does not. User phone cached baseline now
6.988 ms / 4293668 ins/s vs uncached351.526 ms /85351 ins/s (~50.3x).
Phone native result unreported; user will also benchmark PC. Shared source
Windows/WSL uses separate CARGO_TARGET_DIR (target/windows vs target/wsl).
Feature matrix belongs in runtime docs; all tests/history remain on dev.

## Previous notes

# v0.9 cached interpreter verified on hosts

Runtime-only PR #6: https://github.com/pyTony/pil_proof/pull/6
Source dev bf9e681065cf26f976954ea9a5a997604d4d7582, CI run 40336149129:
116 tests passed on Ubuntu (5.24 s) and Windows (10.81 s), isolated 45-file
runtime, generated Rust and unpacked native deliveries in both cache modes.
Release loop 10000 iterations/5 samples: Ubuntu 2.812 ms cached vs 157.096 ms
uncached; Windows 4.019 ms vs 216.011 ms. About 54-56x on CI hosts; do not
extrapolate phone speed. Last cached sample 29997 hits, 7 misses, no invalidations.
Phone cache result is unreported; old v0.8 phone median was 340.420 ms / 88135
ins/s. Engine remains token interpreter. Future decoder mode/translation state
must participate in the image/cache identity and hit fetch checks. No MMIO,
paging or native/JIT execution model is introduced by this cache.

# v0.9 cached interpreter development

Engine remains PIL token interpreter. Generated Rust is separate. New cache.rs
stores up to 256 shared decoded/PIL descriptions, matches full Image/segment
context and re-fetches/checks every encoded byte on reuse. No RAM writer hooks
are needed; poke/guest writes invalidate on next use. Errors never cached.
Registers/flags/addresses/permissions are live at each instruction. Cache mode
persists on reset/load; entries/stats clear. Whole-cache flush at capacity.
REPL cache stats/on/off/clear; bench follows mode; shell --bench --uncached and
PowerShell -Uncached compare. Every sample starts cold; warmup is separate.
116 tests supplied; host CI pending. Main is a 45-file runtime-only export.
User v0.8 phone baseline: release 10000 iterations / 5 samples, median
340.420 ms, 88135 ins/s on android/aarch64. Full debug loop and stress passed.
See revision history JSON for all timings. No phone cache result yet. Preserve
all dev tooling/history here; never merge dev into main.

## Earlier notes

# v0.8 host verification complete

Runtime-only PR #5: https://github.com/pyTony/pil_proof/pull/5
Dev source 2ff2bcb89bbc03ac46870413a6dabfb7cd5d90b9 passed 101 tests on
Ubuntu (7.71 s) and Windows (13.29 s), numeric CLI, release benchmark helpers,
standalone 44-file runtime and unpacked native delivery numerical checks.
CI: https://github.com/pyTony/pil_proof/actions/runs/40331078455
Phone numerical results remain unreported. Time includes current decode/PIL
compile overhead; do not claim 386 cycles or hardware MHz. Mandelbrot remains
future work. Development stays on dev, main uses the separate runtime export.

# v0.8 numerical workload development

User confirmed v0.7.1 Hello-world on Termux: 28 steps, halt 0x138, correct
14-byte console/TX text including newline. New work: factorials 1!..12! guest
(71 bytes, 336 instructions, halt 0x146) and counted loop (27 bytes, default
65536 iterations / 196611 instructions, halt 0x11a). No new ISA or decoder ops.
Timing includes decoder and PIL compilation; benchmark excludes image setup,
result checking and console I/O. One excluded warmup; fresh state per sample.
Time respects debugger stops and 1M-step bound; stress verifies every table.
101 dev tests supplied; hosted CI pending. Mandelbrot postponed until signed
multiply/shift and image output exist. Runtime-only export is 44 files. Keep
all tests, logs, tools and this handoff on dev; never merge dev into main.

## Earlier notes

# v0.7.1 Hello-world verified on hosts

PR #4: https://github.com/pyTony/pil_proof/pull/4 (runtime-only main delivery).
Dev source c1eca198 passed 94 tests on Ubuntu (5.90 s) and Windows (10.60 s)
in CI run 40324429616. Isolated runtime and unpacked archive Hello-world serial
transcripts passed. User has not yet reported phone Hello-world execution.
Keep full tests/history here on dev; never merge this branch into main.

# Current v0.7.1 work — Hello-world

User confirmed the v0.7 diagnostic selftest on Termux: 63 steps, PASS, EIP=1cb,
DX=600D, RAM signatures and console/TX byte 41. Add demo hello (57 guest bytes,
28 steps, output Hello, world! plus newline), --write-hello and build/package
integration. 94 dev tests supplied; host CI pending. Keep tests/history here
on dev and export a 38-file run/build/user-doc tree for a separate main PR.

## Earlier notes

# Runtime-only delivery verified

PR #3: https://github.com/pyTony/pil_proof/pull/3
Head delivery/v0.7.0 (fd97dca67d25a4d1e4f59ffdc9bbf53baabdb94b), exported
from dev 0cb33f73a629af01315f3562a3d0be46b58d4fba. Direct dev-to-main PR #2
is closed. Main awaits user approval of the delivery-only tree.

https://github.com/pyTony/pil_proof/actions/runs/40320291757
passed on Ubuntu and Windows: 92 development tests, isolated 36-file runtime
build/scripts/serial/generated Rust, and unpacked native deliveries. Runtime
cargo test intentionally has zero tests; development tests remain here on dev.
Phone v0.7 verification is still unreported.

## Branch policy and prior verification

# Branch layout policy

Development stays on dev; main receives a generated runtime-only tree through
a delivery branch/PR. Never merge dev into main. Run tools/export_runtime.py
with an empty output directory; --verify checks the isolated build, launchers,
serial transcript and generated Rust. User documentation templates live in
delivery/source/. Development logs, tests, CI and tooling stay on dev.

## Feature verification

# Development status

Development branch: dev. Delivery branch: main. Promote through reviewed PRs.
Current work: v0.7 effective addresses, segment contexts, binary monitor,
executable selftest, console serial echo and build/debug/delivery helpers.

Verified baseline v0.6: 65 Rust tests passed on Termux in 9.47 s. PR #1 merged
as 13ea986c53384cdfd9103cc346c9d32ebeed8881.

v0.7 core commit 56f194fd031bed263b1b9f5ce5dc9b09f7abdb28 passed all 92
Rust tests on hosted Ubuntu (5.88 s) and Windows (11.42 s). The executable
selftest passed in 63 completed steps on both, as did the platform build/debug
scripts and native ZIP creation. CI run:
https://github.com/pyTony/pil_proof/actions/runs/40318741977

The final packaging step also verifies an unpacked archive's native selftest
and debugger launcher/serial transcript. Phone build/manual debugger results
have not been reported. No Rust toolchain in the authoring environment.
PowerShell scripts are included in Windows CI; interactive GUI/manual behaviour
is not implied by that.

Next: strengthen decoder coverage, general PC machine state and devices; keep
full paging, descriptor loading, exception delivery, interrupts, A20 and OS boot
explicitly outside the current implementation. The debug port is not a UART.

