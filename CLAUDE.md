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

# User PC factorial workload confirmed

Latest user transcript:336 steps,halt146,PASS all12 results incl479001600;
0.979 ms. Stress100 passed1200 results/33600 instructions in89.580 ms including
setup/verification.18 cache entries/319 hits/18 misses/0 invalidations. Surrounding
session0.11; transcript has no banner. This is not a0.13 user test.

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

# First Windows user interpreter run recorded

User v0.11.0-alpha.1 Windows/x86_64 release bench defaults passed:1000/3,
median0.438 ms,6,853,035 ins/s;2997 cache hits/7 misses/0 invalidations.
Full PC generated/native comparison still unreported. Phone used10000/5;
keep sample scopes separate. Current0.12 implementation remains host-tested
only. No implementation change in this evidence update.

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

# Current PR status

PR #2 is closed. PR #3 (delivery/v0.7.0 -> main) is the runtime-only delivery,
not a merge of dev. Its 36-file tree is exported from dev 0cb33f73 and verified
on Ubuntu/Windows in CI run 40320291757. The full 92 dev tests, isolated user
build/debug/serial/generated-Rust checks and unpacked ZIP checks passed.
Keep all future development on dev; refresh main only through runtime exports.

# Branch policy correction — runtime-only main

Keep development on dev. Never merge dev directly into main. Main contains
only portable run/build source, embedded ISA descriptions, compile/debug
scripts, sample program and user-facing documentation. Tests, oracles, CI,
export/package tools, history/design notes and this handoff remain on dev.

Export with python3 tools/export_runtime.py EMPTY_DEST --source-ref DEV_SHA.
CI adds --verify to compile/run the isolated export, exercise platform scripts
and compile generated Rust. Propose the exported tree on a delivery branch
based on main. Runtime source drops cfg(test) path modules requiring dev files.
Native archives also carry user docs/credits only. PR #2 (dev -> main) is
superseded by a runtime-only delivery PR. Preserve all development history here.

## Previous status (branch promotion policy above supersedes older notes)

# Current continuation status — v0.7 development

Develop on dev; main is the reviewed user delivery branch. The user approves
PRs in the browser and pulls in ~/pil_proof. Keep build output in target/ or
dist/, not extra directories in home. Use PRs instead of patch-only delivery.

Baseline: main 13ea986c53384cdfd9103cc346c9d32ebeed8881 (merged PR #1).
v0.6 passed 65 Rust tests on Termux in 9.47 s.

v0.7 adds address.rs (decoded 16/32-bit offsets and normalized loaded caches),
execute_segmented and generated execute_with_segments; checks occur per token
width/direction before RAM access. A later branch fault cancels a translated
staged store. StackSegment(0) distinguishes SS bounds. Descriptor loads and
privilege checks are caller responsibilities; paging/A20/interrupts are absent.

guest.rs decodes a small documented raw-binary monitor profile in 4-KiB RAM.
The original 204-byte programs/selftest.hex + listing are reproduced by
python3 tools/build_selftest.py --check. Rust writes the actual .bin during
build. Do not claim a general 386 decoder, UART, full HLT semantics or OS boot.

sh tools/build.sh; sh tools/debug.sh; serial rx 41; run 1000; check.
Automatic: sh tools/debug.sh --check. Windows: tools/build.ps1 / debug.ps1.
Successful TX appears automatically in console as hex/escaped text without
draining the queue. Check uses output history, so serial tx does not spoil it.
Selftest requires DX=600D, four RAM signatures and TX=41. DX=DEAD means failure.

92 Rust tests supplied, including 153 segmented generated/interpreted cases.
Core commit 56f194fd passed 92 tests on hosted Ubuntu (5.88 s) and Windows
(11.42 s); selftest passed 63 steps on both. Build/debug scripts and native ZIP
creation passed. Final packaging validation also checks the unpacked launcher
and serial transcript. No local Rust toolchain. Do not equate host CI with a
new Termux run. Native ZIPs are host-specific, not Android/ESP32.
See docs/development/STATUS.md and docs/DEBUGGER.md for current limitations.
No third-party code imported; keep provenance and revision logs/checksums.

## Historical handoff snapshots

# Current continuation status — v0.6 working patch

Repository: https://github.com/pyTony/pil_proof (private), main; inspected base
commit cfd4143512497c8afd28df9ef6739a96fc62b9f0. The user completed the first
push. Work remains in ~/pil_proof and updates remain patches.

v0.5 is verified on the phone: 46 tests passed in 4.27 s; REPL starts. The
user observed echo waiting, RX injection and two steps, but did not show the
final serial tx response. Older verification records are preserved below.

The v0.6 patch adds src/memory.rs, load_ram/store_ram/add_ram descriptions,
read_address/load/store tokens, execute_memory and generated
execute_with_memory, RAM fault metadata, and mem/poke/protect debugger commands.
Memory is flat, little endian, unaligned, with 8/16/32-bit data and 32-bit
addresses. At most one staged store and no loads after it are permitted.
All RAM/store checks precede mutation; a later branch fault cancels the staged
store and CPU effects. This is NOT a model for rollback of MMIO/device effects.
MemoryAccess is an emulator bounds/protection error, NOT x86 #PF; there are
no page tables, CR2 updates, segment translation or instruction fetch yet.

User-confirmed Termux result on 26 October 2026: 65 Rust tests passed,
0 failed, 0 ignored, 0 measured, 0 filtered out, in 9.47 s.
The authoring environment has no Rust compiler; Rust verification is from
the user's phone.
Run cargo test --locked, python3 tools/verify_memory.py, and the existing
Python oracles. Generated Rust embeds flow.rs and the non-test portion of
memory.rs. A generated-memory test compares 560 cases, including cancelled
stores and protection/bounds errors. Legacy arithmetic/flow tests remain.

Smoke test: cargo run -- --repl; demo memory; step; mem 0x10 4; step; regs.
Expected memory: 78 56 34 12; EAX after second step: 12345678.
After reset; protect 0x10 4; run, expect a RAM write fault at EIP=0x300
without changing CPU/RAM. New RAM is 256 bytes per debugger demo; reset restores
initial RAM and removes debugger protection. This is not a binary loader.

Priorities: add explicit effective-address/segment context
and controlled fetch/decode. Maintain source/attribution/revision logs. Do not
copy repository-backed source into an additional artifact store; use the Git
project and patches. No third-party emulator code has been imported. No own
project licence has yet been selected. Do not invent one or publish a remote
change without a task requiring it.

## Previous continuation notes (historical)

# PIL PC emulator — continuation guide

Updated 26 October 2026. Read this before changing the project.

## Goal and user requirements

Build a PC emulator, initially a 386 suitable eventually for Windows 3.1x.
The CPU is described in a width-aware language with pure operations and
explicit flag/state effects. Execute validated tokens in Rust first; generate
Rust source from the same tokens second. ARM/native backends can come later.

The eventual board is the user's TTGO with original ESP32, VGA, PS/2
keyboard/mouse, SD card and WLAN. Preserve the existing emulator's
Internet/WLAN-to-SD system/disk-image installation workflow. Exact upstream
installer/board revision is not yet identified. Tiny386 devices are candidates
for reuse behind an adapter; no third-party emulator code has been imported.
Windows edition called “3.12” by the user remains unconfirmed; use 3.1x as the
provisional scope, standard mode before enhanced mode.

Work in **~/pil_proof**. Do not create new version directories in the home
folder. Deliver guarded in-place patches. Preserve Cargo.lock, target/ and
unrelated user files. Maintain references, licences/attribution/thanks,
revision history and source checksums. Prepare the first GitHub push, but
do not invent a remote repository or a project licence.

## Confirmed baseline

The user installed Rust/Cargo 1.99.1 in Termux and reported:

| Version | Rust results | Reported test execution time |
|---|---|---|
| 0.1 | 7 passed, 0 failed | 0.67 s |
| 0.2 | 14 passed, 0 failed | 2.57 s |
| 0.3 | 20 passed, 0 failed | 3.30 s; then 3.39 s in ~/pil_proof |
| 0.4 | 28 passed, 0 failed | 4.30 s |
| 0.5 | 46 passed, 0 failed | 4.27 s |

v0.4 includes ADD/SUB/CMP/ADC/SBB/AND, 8/16/32-bit values and wider
intermediates, register views, complete arithmetic-flag classification,
undefined-output diagnostics, SETcc, select/write_if, EIP, Jcc, supplied
decoded displacements, protected-code limit checking and restartable fault
results. Interpreter and generated Rust are compared by tests.

## Current patch: v0.4 to v0.5

Adds bounded mock serial RX/TX and a host terminal REPL. Cleans the root for
GitHub and moves unit tests to tests/unit/. **The user verified all 46 Rust tests in Termux: 46 passed, 0 failed, 4.27 s.**
The user also compiled and launched cargo run -- --repl (2.67 s compilation),
with the pil> prompt displayed. Interactive command behavior has not yet been
reported by the user. The authoring environment still has no Rust compiler;
these are user-reported results, separate from local Python/migration checks.

Apply the supplied pil_v04_to_v05.py from inside the existing project.
It checks v0.4 implementation hashes, preserves/merges old documentation,
migrates owned files, and leaves generated/unrelated files alone.

Expected layout:

- src/main.rs: language validator, interpreter, Rust emitter, CLI entry.
- src/flow.rs: shared flow context/error types; also embedded in generated Rust.
- src/serial.rs: nonblocking SerialTransport and bounded MockSerial.
- src/debugger.rs: predecoded demos, breakpoints, commands, stdio REPL.
- isa/i386/: instruction and condition descriptions.
- tests/unit/{cpu,serial,debugger}.rs: unit-test modules.
- tools/verify_descriptions.py and tools/verify_flow.py: independent oracles.
- docs/: language, debugger, roadmap, reference/credits, history and Git notes.
- Cargo.toml, Cargo.lock, README.md, CLAUDE.md and Git rules at root.

## Immediate verification

```sh
cd ~/pil_proof
cargo test
python3 tools/verify_descriptions.py
python3 tools/verify_flow.py
cargo run -- --repl
```

The CPU tests remain unit tests with access to private parent items. They
are included using #[cfg(test)] #[path = "../tests/unit/..."] mod tests;
do not accidentally make them standalone integration crates.
Rust include_str! paths now point to ../isa/i386/. Python scripts locate that
directory relative to the project root. Preserve generated flow.rs embedding.

REPL smoke test:

```text
list
break 0x102
run
flags
step
continue 100
demo echo
run
serial rx 41
run
serial tx
quit
```

Counter uses EAX=3, EBX=1 and ends at harness EIP 0x108. Echo waits on RX,
transmits one byte and stops; serial tx should show 41. Console commands and
guest serial bytes are separate. Empty RX is distinct from byte zero.
Full TX waits without advancing EIP; injection is all-or-none.
Run is bounded to 1..10,000 steps. Continue skips the breakpoint just reported
once, then stops again if execution loops back. Reset retains breakpoints and
clears queues; loading a demo clears breakpoints.

## Important limits

No raw x86 binary loader, instruction fetch/memory, PC devices, guest exception
delivery, real UART, board build, network/SD installation or Windows boot yet.
MOCK_RX/TX and HARNESS_HALT are debugger harness operations, not implemented
x86 I/O/HLT instructions. The serial mock is not an 8250/16550 UART model.

Undefined flags have a concrete deterministic value plus a diagnostic mask;
they are not compiler poison or Rust UB. This is not full uncertainty taint
tracking. Faulting branches do not commit staged CPU writes. A supplied code
limit is not full segmentation.

The current code uses std, heap vectors/queues and host subprocess tests.
For TTGO compilation, separate host parsing/source generation/debugger from
an embedded core; precompile descriptions, bound/reuse buffers, then add board
drivers through Rust/C/C++ adapters. Espressif's Xtensa toolchain is the route
for original ESP32; cross-compiling this host CLI unchanged is not the goal.

## Next work

1. v0.5 now passes all tests on the user's phone; perform the REPL smoke test
   above and record the observed interactive results.
2. Review REPL/serial behavior and establish the embedded transport boundary.
3. Add memory and restartable effects, then binary decode/device integration.
4. Inspect the actual existing TTGO emulator/installer and reusable drivers.
5. Before first push, review staged files, Cargo.lock and attribution. See
   docs/GITHUB.md. No remote is configured by the patch and no project-wide
   distribution licence has been selected.

Keep changes incremental. Distinguish user-reported Rust results from local
Python/package checks. Do not claim Windows or hardware compatibility from
arithmetic/debugger tests.
