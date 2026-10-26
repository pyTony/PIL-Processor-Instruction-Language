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
