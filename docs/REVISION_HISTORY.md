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

92 Rust tests supplied (27 new), including 153 segmented generated/interpreted
cases. Rust execution is pending hosted CI; no local Rust compiler exists.
Termux v0.7 and Windows launcher manual checks remain unreported.
