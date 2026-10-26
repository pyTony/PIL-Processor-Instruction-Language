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
