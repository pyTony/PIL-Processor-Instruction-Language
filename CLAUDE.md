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
