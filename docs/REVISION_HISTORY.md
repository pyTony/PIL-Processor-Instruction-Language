# Revision history

Every delivered revision receives a version, feature/change summary, test
status, provenance update and source fingerprint. User-reported results are
recorded separately from checks executed in the authoring environment.
No Git commits have been created; these release numbers are not commit IDs.
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
https://github.com/pyTony/pil_proof/actions/runs/40320291757
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
https://github.com/pyTony/pil_proof/actions/runs/40324429616
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
https://github.com/pyTony/pil_proof/actions/runs/40331078455
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
https://github.com/pyTony/pil_proof/actions/runs/40336149129
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
