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
