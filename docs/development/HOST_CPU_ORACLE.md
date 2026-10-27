# Actual x86-64 host CPU oracle: initial ALU slice

Development-only; first N0/N1 implementation. This is separate from PC_NATIVE
loop timing and from the generated-Rust-versus-interpreter trace verifier.
It is not a64-bit guest implementation or a full386 hardware conformance claim.

On an x86-64 PC,from the dev branch:

```sh
python tools/verify_host_cpu.py
```

Or run directly without the helper:

```sh
cargo test --locked host_cpu_alu_matches_interpreter_and_generated_rust -- --nocapture
```

The Python helper exits UNAVAILABLE/nonzero on ARM and requires an actual
HOST_CPU success line. Normal cargo test ignores the hardware test on non-x86-64
and still runs the portable comparator corruption test. Forcing the ignored
hardware test on an unsupported target fails rather than claiming a pass.

## Admitted operations and independence

ADD,SUB,ADC,SBB,CMP,INC,DEC,NEG,NOT,AND,OR,XOR,TEST8/16/32 using fixed
accumulator/source registers. Sixteen
independent known-answer cases check native opcode/operand/carry/flag plumbing
before trusting the oracle. Boundary pairs plus512 deterministic pseudorandom
pairs per variant run with CF0 andCF1. They are original project cases,not
SingleStepTests vectors. This is not an exhaustive input-space claim.

Curated literal instruction bytes execute in an out-of-line Rust function's
assembly block. CMP AL,CL varies input arithmetic flags; BT seeds CF.
PUSHFQ/POP snapshots actual flags before and after the target,without compiler
instructions between them. Snapshots supply actual inputs independently of
diagnostic metadata. INC/DEC
preserve CF; NOT preserves all flags; NEG and the binary arithmetic forms
define all six outputs. Logical operations leave AF undefined. Unary cases
include known and undefined CF metadata; logical cases start with either no
undefined arithmetic flags or all six marked undefined.

| Operation | Written arithmetic flags | Preserved arithmetic flags | Metadata policy |
|---|---|---|---|
| ADD/SUB/ADC/SBB/CMP/NEG | CF/PF/AF/ZF/SF/OF | None | Clear all six undefined markers |
| INC/DEC | PF/AF/ZF/SF/OF | CF | Preserve incoming CF marker; clear the other five |
| NOT | None | CF/PF/AF/ZF/SF/OF | Preserve every marker |
| AND/OR/XOR/TEST | CF/PF/ZF/SF/OF | None | Set AF undefined; clear the other five markers |

Undefined metadata is a PIL diagnostic policy checked from this independent
specification; the hardware supplies concrete flag values,not metadata.

No privileged/reserved flag values are loaded and DF is never changed. Balanced
stack operations retain Rust's default assembly stack/memory/flag clobbers;
volatile registers are explicit so the compiler handles the platform ABI.

Hardware source/sentinel registers and flags outside the fixed architectural
mask0x8d5 must remain unchanged. LogicalAF is allowed to change but its actual
hardware value is excluded from comparison. No mask is inferred from DUT state. Full nativeRAX also checks byte/word merging,
32-bit zero-extension and CMP/TEST's non-writing behavior,including the upper half.
These native64-bit write-policy checks are separate from the386 low32 result.

The386 decoder consumes separately recorded guest bytes (16-bit defaults),
while native stubs execute long-mode bytes (32-bit defaults). The interpreter
executes that decoded PIL and the generated functions execute the same tokens;
both are checked independently against the hardware result. Each check covers
all8 modeled registers,nextEIP,defined arithmetic flags,preserved flags
and operation-specific undefined-marker preservation/clearing. Comparison never trusts the DUT's own
undefined mask to decide which bits to ignore.

Only the stated native registers/flags are captured; this is not arbitrary
full-register or system-state restoration testing. No memory operands,guest
fault delivery,BCD,privileged operations or386 timing are validated by this slice.

## Artifacts and CI

Under CARGO_TARGET_DIR (default target)/host-cpu-oracle/run-*/:
- cases.jsonl:case IDs,widths,carry,guest/native bytes,input/expected model state
  and nativeRAX upper halves;
- hardware.bin:schema2 records of28 little-endianu32 words (112 bytes),used
  by the generated runner; old runs used26 words/104 bytes;
- generated.rs/executable,build.stderr,generated.stdout and generated.stderr;
- report.txt:case count,mask/scope,CPU vendor/rawCPUID1,OS/architecture,Rust
  toolchain,source revision and a non-cryptographic FNV64 case fingerprint.

A mismatch names the first field and keeps its artifacts. CPU snapshots supply
expected data before either PIL backend is accepted. The comparator corruption
test verifies that wrong flags cannot be hidden by marking them undefined.

Windows/Linux CI executes this as part of cargo test with successful output
shown and uploads reports/case records. Runtime export removes its cfg(test)
module; tests,helper and hardware artifacts are not added to main. No Cargo
version bump or runtime delivery is required for this development tool.

## References/provenance

Original test code/stubs/data. Rust reference for explicit operands,clobbers
and assembly stack rules; current CPUID intrinsic signature inspected:
https://doc.rust-lang.org/reference/inline-assembly.html
https://doc.rust-lang.org/core/arch/x86_64/fn.__cpuid.html

Instruction/flag references and ABI work are listed in the
[x86-64 plan](X86_64_VERIFICATION_PLAN.md). No external tests or implementation
code imported. Next:extend admitted operations and add the separate pinned386
hardware-vector adapter.

## Verified development result —27 October2026

[CI40415384652](https://github.com/pyTony/pil_proof/actions/runs/40415384652)
passed for source24d28740c8d63a0ce393cec05b56320b7833b5c4. Both hosts passed
167 tests and20,280 hardware cases against both PIL backends,including the
six native harness checks. Ubuntu reported GenuineIntel; Windows reported
AuthenticAMD. Existing39-case backend verification,full256x256 Mandelbrot
comparison and isolated runtime/package checks also passed on both hosts.
The checkoutv5/upload-artifactv6 Node24 actions completed successfully.
This is hosted CI evidence; no local ARM execution is claimed.

## Unary expansion verified —27 October2026

The new default has27 variants and52,728 cases. Unary forms use FE/FF /0
(INC),FE/FF /1 (DEC),F6/F7 /3 (NEG),F6/F7 /2 (NOT); long-mode single-byte
40..4F encodings are deliberately avoided because they are REX prefixes.
Metadata has independent known-CF/undefined-CF inputs; raw native flags still
provide concrete values in both cases. JSON records now include comparison
and written masks plus input/expected undefined metadata. Binary records keep
the existing104-byte layout. Prior20,280-case results below remain historical.

User reported the preceding source48556af passing20,280 cases under WSL on
AuthenticAMD (CPUID1 EAX00a20f10),rustc1.99.1,with fingerprint8b8c01e8fdeff584.
The helper ran one test with166 filtered out; artifacts used target/wsl.

[CI40417668829](https://github.com/pyTony/pil_proof/actions/runs/40417668829)
passed for sourcef6ff393221cb9c771b12556fe76c05336f3559d4 on both Windows/Linux
AuthenticAMD hosts (CPUID1 EAX00a00f11). Each passed167 tests and52,728 native
cases against both backends. Existing39 trace cases,full256x256 Mandelbrot
comparisons and isolated source/package tests passed. Tests were enhanced,
not added,so the test count stays167. No ARM execution is claimed.

The first new DEC8 harness answer incorrectly included PF for7F (seven set
bits); run40417508356 rejected it. The expected written flags were corrected
from814 to810. All12 harness constants were also checked independently with
a small Python calculation. No DUT semantics were changed to obtain the pass.
Runtime export remains72 files with unchanged contents apart from source
provenance/checksums. This development work needs no runtime delivery PR.

## Logical expansion verified —27 October2026

The new default has39 variants and85,176 hardware cases,plus16 known-answer
harness checks. AND/OR/XOR/TEST use register forms20/21,08/09,30/31,84/85.
TEST never writes its operand,including nativeRAX's upper32 bits. No new
guest instruction is implemented by this verification increment.

Independent per-operation masks partition all32 flag bits into defined,
preserved,undefined sets. The comparator never uses DUT metadata to choose
which values count. For logical forms,AF's actual hardware value is recorded
but ignored; PIL's concrete-zero behavior and undefined marker are checked
separately. CF/OF must clear; SF/ZF/PF follow hardware. Preservation covers
all remaining bits. Logical cases start both with zero and all-arithmetic
undefined metadata to exercise new AF marking and defined-marker clearing.

Schema2 keeps words0..24 from earlier records. Word25 is now the defined mask;
26 is preserved and27 undefined. The generated runner reads all three masks,
checks concrete zero only as PIL policy and checks the exact diagnostic marker.
JSON records carry record_schema=2 and all masks; flags_expected retains raw
hardware bits even where a mask excludes them. Old artifacts are historical
and should be run with their own recorded generated runner.

A portable corruption test permits nativeAF=1 with PILAF=0,then rejects a wrong
PIL zero policy,missing AF marker,and changes to each defined flag or preserved
IF. Claiming all flags undefined cannot hide a wrong defined result.

User Windows baseline atd23ab45 passed52,728 cases,one selected test with166
filtered out,on AuthenticAMD EAX00a20f10/ECXfed8320b,rustc1.99.1,fingerprint
0bc8895949b33008. This is user-reported correctness evidence,not throughput.

[CI40419168682](https://github.com/pyTony/pil_proof/actions/runs/40419168682)
passed for source342375e3a732dfe59f4b799358bf743d51cdd496. Both hosts passed
168 tests and85,176 hardware cases against the interpreter and generated Rust.
Linux reported AuthenticAMD EAX00b00f21; Windows reported AuthenticAMD
EAX00a00f11. All16 native harness checks,39 existing backend trace cases,
full256x256 Mandelbrot and isolated runtime/package checks passed. No ARM run
is claimed. The72-file runtime export is unchanged except source provenance
and its checksum; no delivery PR or Cargo version bump is needed.

## Shift and truncated-IMUL expansion (execution pending)

SHL/SHR/SAR CL forms at8/16/32 bits and two-operand IMUL16/32 add11 native
encoding variants. Shift counts0..255 cross all boundary values;512 random
pairs per width/form remain. New total226,008 cases/50 variants,plus23 native
harness answers. One portable count/mask regression raises Rust tests to169.

Shift counts are masked by31. Zero preserves flags/metadata. Active shifts
define SF/ZF/PF,leave AF undefined and define OF only for masked count1.
SHL/SHR CF is excluded at or above operand width; SAR CF stays defined.
IMUL defines CF/OF and leaves PF/AF/ZF/SF undefined. All undefined native
values remain recorded but excluded from hardware equality; separate PIL
concrete-zero and exact metadata policies still apply. Schema2 is unchanged.
Modern results cover the admitted common semantics,not386 timing/system
behavior or undocumented count rules. Runtime instructions remain unchanged.

## Shift/IMUL result and rotate/CF-control increment

CI40422881682 passed226,008 native cases and169 tests per Windows/Linux host
at811af3596bdd5157cff8c3f3c9769de2b8f91b0a. The subsequent0.16 increment adds
ROL/ROR CL8/16/32 and CLC/STC,bringing the verified suite to321,784 cases,58
encoding variants and31 native harness answers (CI40423654988).

Rotate data uses count modulo width; policy uses count&31. Masked-zero
preserves all flags;otherwise CF is defined,OF defined only at masked count1
and undefined at larger counts. PF/AF/ZF/SF are preserved,including markers.
CLC/STC define CF and preserve every other bit/marker and full nativeRAX.

## 0.16 verified delivery evidence

[CI40423654988](https://github.com/pyTony/pil_proof/actions/runs/40423654988)
passed175 tests and321,784 hardware cases per Windows/Linux host for source
21d65da9a430d77ee3ad1e658afbeede907ae041. Both backends matched the admitted
hardware outputs and independent metadata policy. Independent Python checked
278,576 rotate/CF-control cases. Existing/new full-state backend coverage is45
cases (4,861,954 trace bytes);full256x256 Mandelbrot and source/package checks
also passed. Six new Rust tests cover rotations,aliases,CF control and faults.
No ARM or external386 suite run is claimed. Runtime PR19 exports76 run/build
files;native tooling and CI remain ondev.

## Register-XCHG/two-binding expansion (execution pending)

XCHG8/16/32 adds3 native forms:329,896 cases/61 variants/34 harness checks.
Unlike previous forms,it changes both RAX and RCX. The native harness checks
low-width exchange,byte/word high-bit preservation and32-bit upper clearing
on both registers. Expected model records already contain all8 registers.
All flags/markers are preserved. Generic alias/conditional/late-fault tests
exercise the two-write PIL contract in both interpreter and emitted Rust.

## CMC expansion (execution pending)

CMC adds one native F5 form, two known answers and2,832 cases: expected
332,728 cases/62 variants/36 harness checks. The oracle checks preserved
registers and flags, both incoming carry values and concrete metadata policy.
No hardware pass is claimed before the new CI run completes.
