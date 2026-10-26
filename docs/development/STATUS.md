# Expanded instruction profile — 0.11.0-alpha.1 development

OR/XOR/TEST/NOT/NEG/INC/DEC/NOP added in PIL and monitor decoder. Binary
ALU supports register/memory both directions, accumulator immediates and
80/81/83 groups. F6/F7 /0 TEST,/2 NOT,/3 NEG; FE/FF /0 INC,/1 DEC;
40..4F INC/DEC;90 NOP. Unsupported-opcode tests now use62 instead of90.
Operand locations are bound into PIL before validation; no duplicated CPU flags.
Six new tests,128 total pending CI; compiled proof14 cases (ALU8/16/32,
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
