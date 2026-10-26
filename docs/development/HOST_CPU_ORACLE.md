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

ADD,SUB,ADC,SBB,CMP8/16/32 using fixed accumulator/source registers. Six
independent known-answer cases check native opcode/operand/carry/flag plumbing
before trusting the oracle. Boundary pairs plus512 deterministic pseudorandom
pairs per variant run with CF0 andCF1. They are original project cases,not
SingleStepTests vectors. This is not an exhaustive input-space claim.

Curated literal instruction bytes execute in an out-of-line Rust function's
assembly block. BT seeds CF; PUSHFQ/POP snapshots actual flags before and after
the target,without compiler instructions between them. BT's other arithmetic
flag values are inputs only:every admitted target defines all six outputs.
No privileged/reserved flag values are loaded and DF is never changed. Balanced
stack operations retain Rust's default assembly stack/memory/flag clobbers;
volatile registers are explicit so the compiler handles the platform ABI.

Hardware source/sentinel registers and flags outside the fixed architectural
mask0x8d5 must remain unchanged. Full nativeRAX also checks byte/word merging,
32-bit zero-extension and CMP's non-writing behavior,including its upper half.
These native64-bit write-policy checks are separate from the386 low32 result.

The386 decoder consumes separately recorded guest bytes (16-bit defaults),
while native stubs execute long-mode bytes (32-bit defaults). The interpreter
executes that decoded PIL and the generated functions execute the same tokens;
both are checked independently against the hardware result. Each check covers
all8 modeled registers,nextEIP,defined arithmetic flags,preserved other flags
and expected undefined-marker clearing. Comparison never trusts the DUT's own
undefined mask to decide which bits to ignore.

Only the stated native registers/flags are captured; this is not arbitrary
full-register or system-state restoration testing. No memory operands,guest
fault delivery,BCD,privileged operations or386 timing are validated by this slice.

## Artifacts and CI

Under CARGO_TARGET_DIR (default target)/host-cpu-oracle/run-*/:
- cases.jsonl:case IDs,widths,carry,guest/native bytes,input/expected model state
  and nativeRAX upper halves;
- hardware.bin:version-specific26-word records used by the generated runner;
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
