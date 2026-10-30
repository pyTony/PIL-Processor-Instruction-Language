# Runtime delivery policy

Development stays on dev. Main receives a strict run/build-only export via a
separate delivery branch; never merge the dev tree into main. The applicable
user documentation is maintained in delivery/source/. CI validates the isolated
export in addition to development tests. Full history and planning remain here.

## User direction: I/O service and debugger

Route device/service I/O through an explicit service boundary and improve the
debugger using the user's other projects as design references. See the
[concrete proposal](development/IO_SERVICE_AND_DEBUGGER_DESIGN.md), the
[IDE plan](development/IDE_PLAN.md) based on the lispy machine layout, and pinned
lispy3/z80-cpm review. The user clarified a DOS compatibility layer analogous
to Wine's Windows service provision; no external Dine project is involved.
This is planned architecture work; v0.34 runtime behavior is unchanged.

## Current increment —0.34,28 October2026

Implement bounded AH02 output, AX3700 fixed switch query and real-mode LES
from the user's100000-step v0.33 report. Windows/Linux CI40624671136 passed328 tests and199 backend cases,
plus standalone/native packages and full Mandelbrot comparisons.
Next remains a text-screen model for INT10/AH08, RCR for GREP, PUSHA and32-bit
SMSW. Investigate individual loaded-image boundary stops separately; no new
whole-image exit count is claimed.

## Previous position —0.33,28 October2026

Version0.33 adds AH25/35 RAM interrupt-vector services and ordinary-RAM XCHG.
Windows/Linux CI40621030314 passed319 tests and185 backend cases, plus full
Mandelbrot and standalone-package checks. [PR38](https://github.com/pyTony/PIL-Processor-Instruction-Language/pull/38)
is the separate runtime delivery. Existing far CALL/RETF and memory PUSH/POP
remain part of the supported bounded profile.

Next reported service priority is INT10/AH08 backed by a text screen
(17 programs in the separate10000-step report). PUSHA and32-bit SMSW remain
individual instruction blockers. No new whole-image rerun establishes changed
exit counts yet. CPU interrupt delivery, ROM/kernel handlers, broader LOCK RMW,
MMIO/concurrent buses and full mode/prefix validation remain separate work.

## Historical plan —27 October2026

The active TODO is the [386 completion plan](development/ISA_COMPLETION_PLAN.md),
with [family/form backlog](development/ISA_BACKLOG.json). The parallel
[x86-64 verification plan](development/X86_64_VERIFICATION_PLAN.md) starts a
modern-PC instruction oracle early,then an explicit64-bit profile. The
[OS targets](development/OS_COMPATIBILITY_TARGETS.md) distinguish Windows3.1x
editions and optional Windows95/NT3.51 goals from current CPU coverage.

First slices: independent case/oracle infrastructure; named multiple register
effects and ordered memory accesses; basic stack; near calls/returns; rotates/
bit operations; implicit wide multiplication/division. Baseline165 tests/39
original trace cases before the new host oracle. The [host CPU oracle](development/HOST_CPU_ORACLE.md)
now passes321,784 hardware cases over arithmetic/logical/unary operations,
shifts,IMUL,ROL/ROR and CLC/STC,with175 tests per Windows/Linux host and
independent defined/preserved/undefined flag masks.0.16 runtime PR19 is ready;
RCL/RCR,CMC and double shifts remain planned. The pinned
SingleStepTests adapter remains planned. Older
roadmap sections below are retained as history.

## Independent validation next

See [external test suites](development/EXTERNAL_TEST_SUITES.md). First build a
pinned SingleStepTests one-instruction adapter that preserves original machine
state and checks both backends against hardware expectations. A separate
barotto test386.asm ROM becomes useful after reset/ROM mapping,stack/CALL/RET
and bus I/O exist. No external-suite passes are claimed yet.

## Feature roadmap and historical notes

# Roadmap — v0.7 development

- Baseline v0.6 passed all 65 Rust tests on Termux (9.47 s).
- Current: segmented addressing, a limited executable guest selftest, debugger
  serial console output, compile/debug scripts and user ZIP deliveries.
- Development happens on dev; main receives reviewed delivery PRs.
- Next: decoder coverage and a real PC bus/UART boundary, then system state,
  descriptor loading and interrupt/exception delivery. Paging and OS boot remain
  later milestones. Preserve the intended TTGO WLAN/SD installation workflow.
- RAM staging is not a general MMIO/device rollback mechanism.

## Historical roadmap notes

# Roadmap update — v0.6

- v0.5 is user-verified: 46 Rust tests passed in 4.27 s; REPL startup succeeded.
- Echo waiting, RX injection and two executed steps were observed by the user;
  the final TX-drain response was not shown. Do not assume additional manual checks.
- Current: flat RAM token effects, safe single-store retirement, bounds/protection
  faults and memory inspection. New Rust tests are pending execution.
- Next: explicit effective-address/segment context and controlled instruction
  fetch/decode, while maintaining precise errors. Add general checkpoints for
  multi-store/string/device instructions rather than extending RAM rollback to MMIO.
- GitHub repository now exists privately at pyTony/PIL-Processor-Instruction-Language on main. Continue
  supplying patches inside ~/pil_proof; no more version folders in home.

## Earlier roadmap detail

# Working roadmap — PC emulator

## Verified baseline: v0.4 execution patch

- Guest EIP, supplied fall-through offset and decoded relative displacement.
- All sixteen Jcc predicates at 16/32-bit operand widths.
- Short/near signed displacement validation and taken-target wrapping.
- Supplied protected-code-segment limit check and restartable #GP(0) result.
- Interpreter/generated-Rust comparisons and small predecoded loop.
- Patch the existing ~/pil_proof; retain tests in src/main.rs for this step.

## Current v0.5: serial mock and interactive CLI debugger

Implementation supplied in v0.5; Rust compilation and verification pending:

1. Add a byte-oriented serial transport interface with in-memory RX/TX
   queues, empty/ready states and deterministic scripted input/output.
2. Keep guest serial traffic separate from debugger command input and output.
3. Bring up a terminal REPL with help, step, bounded run/continue, regs,
   flags (including undefined markers), EIP, breakpoints and reset.
4. Begin with explicitly loaded/predecoded instruction descriptions; expose
   that limitation. Do not label it a debugger for arbitrary x86 binaries.
5. Use the mock for repeatable REPL and serial tests before real ports.
6. Add a physical serial backend later. A transport mock alone is not an
   8250/16550 UART device or a PC I/O-port model.

Run must have a step budget and an interrupt/stop path so a guest loop cannot
hang the debugger. Breakpoints use guest offsets and stop before execution.
Report faults with the instruction's starting EIP and preserve state.

## Subsequent foundation

- Memory effects and restartable faults, with CPU/device interfaces.
- Binary x86 decoder, segment attributes and instruction fetch.
- Tiny386 hardware adapter, then DOS and Windows compatibility milestones.
- v0.5 moves unit tests into tests/unit/ while retaining private unit-module access.

Keep reference/attribution and revision histories current for every patch.
Do not create versioned project folders under the home directory.

## TTGO/original ESP32 portability goal

The original TTGO ESP32 remains a target, alongside the larger ESP32-S3.
Espressif's espup-managed Xtensa toolchain provides the compiler route;
this patch does not yet provide a firmware build or prove a board memory fit.

Separate host-only parsing, source generation and rustc-spawning tests from
an embedded execution core. Precompile PIL to tokens or Rust handlers on the
host. Replace per-step allocations with bounded/reused storage before moving
to the MCU. Keep serial transports behind the same interface on host and UART.
Choose ESP-IDF integration or no_std only after the core boundary is stable.

For full PC guests, measure available guest memory after display, devices,
queues and runtime allocations. PSRAM availability depends on the exact TTGO
module. Firmware compilability does not guarantee Windows usability.

## Explicit end goal and board integration

The end product is a complete PC emulator on the user's TTGO ESP32, not
merely a debugger or a standalone instruction evaluator. The CPU description,
interpreter and generated handlers are its execution core. The REPL is a
bring-up and diagnostic interface for developing that emulator.

The user confirms VGA, PS/2, SD card and WLAN on the target setup. Connect
these through a board layer, separate from guest PC devices:

| Physical TTGO facility | Planned emulator connection |
|---|---|
| VGA output | Render the emulated PC VGA state/display |
| PS/2 keyboard/mouse | Feed emulated keyboard/mouse controllers |
| SD card | Store guest disk images behind an emulated disk controller |
| WLAN | Optional debugger/console transport, file transfer, and later a backend for an emulated PC NIC |
| UART/serial | Bring-up console and debugger transport; guest serial is a separate emulated device |

Reuse compatible Tiny386 device components where their exact licences permit;
keep them behind the planned adapter. Reuse board drivers through appropriate
C/C++/Rust boundaries as needed. The board's physical VGA and PS/2 support
is distinct from emulating all corresponding PC registers and behaviours.
WLAN does not automatically become a guest-compatible network card.

Measure firmware fit, display timing and PSRAM before claiming full-system
performance or Windows compatibility. This patch implements CPU flow only;
it does not connect these peripherals yet.

## Preserve the existing network-to-SD installation workflow

The user reports that the current TTGO PC emulator installs directly from
the Internet to SD. Retain this user-facing provisioning model: WLAN-backed
catalog/download selection writes supported system/disk images to SD; the
emulator boots the selected image. Inspect the exact existing installer and
image configuration before choosing an adapter; its repository/revision is
not yet identified here. Reuse it where compatible instead of assuming disk
images must be copied from a PC. Keep downloads/storage in the board layer,
independent of instruction semantics and guest network-card emulation.

## Continuation handoff

See root CLAUDE.md for verified state, current unverified work and continuation steps.


## Previous roadmap notes

# Working roadmap — PC emulator

## Current: v0.4 execution patch

- Guest EIP, supplied fall-through offset and decoded relative displacement.
- All sixteen Jcc predicates at 16/32-bit operand widths.
- Short/near signed displacement validation and taken-target wrapping.
- Supplied protected-code-segment limit check and restartable #GP(0) result.
- Interpreter/generated-Rust comparisons and small predecoded loop.
- Patch the existing ~/pil_proof; retain tests in src/main.rs for this step.

## Next: serial mock and interactive CLI debugger

After the basic execution patch is verified:

1. Add a byte-oriented serial transport interface with in-memory RX/TX
   queues, empty/ready states and deterministic scripted input/output.
2. Keep guest serial traffic separate from debugger command input and output.
3. Bring up a terminal REPL with help, step, bounded run/continue, regs,
   flags (including undefined markers), EIP, breakpoints and reset.
4. Begin with explicitly loaded/predecoded instruction descriptions; expose
   that limitation. Do not label it a debugger for arbitrary x86 binaries.
5. Use the mock for repeatable REPL and serial tests before real ports.
6. Add a physical serial backend later. A transport mock alone is not an
   8250/16550 UART device or a PC I/O-port model.

Run must have a step budget and an interrupt/stop path so a guest loop cannot
hang the debugger. Breakpoints use guest offsets and stop before execution.
Report faults with the instruction's starting EIP and preserve state.

## Subsequent foundation

- Memory effects and restartable faults, with CPU/device interfaces.
- Binary x86 decoder, segment attributes and instruction fetch.
- Tiny386 hardware adapter, then DOS and Windows compatibility milestones.
- Move tests into a dedicated tests/ subdirectory once interfaces settle.

Keep reference/attribution and revision histories current for every patch.
Do not create versioned project folders under the home directory.

## TTGO/original ESP32 portability goal

The original TTGO ESP32 remains a target, alongside the larger ESP32-S3.
Espressif's espup-managed Xtensa toolchain provides the compiler route;
this patch does not yet provide a firmware build or prove a board memory fit.

Separate host-only parsing, source generation and rustc-spawning tests from
an embedded execution core. Precompile PIL to tokens or Rust handlers on the
host. Replace per-step allocations with bounded/reused storage before moving
to the MCU. Keep serial transports behind the same interface on host and UART.
Choose ESP-IDF integration or no_std only after the core boundary is stable.

For full PC guests, measure available guest memory after display, devices,
queues and runtime allocations. PSRAM availability depends on the exact TTGO
module. Firmware compilability does not guarantee Windows usability.

## Explicit end goal and board integration

The end product is a complete PC emulator on the user's TTGO ESP32, not
merely a debugger or a standalone instruction evaluator. The CPU description,
interpreter and generated handlers are its execution core. The REPL is a
bring-up and diagnostic interface for developing that emulator.

The user confirms VGA, PS/2, SD card and WLAN on the target setup. Connect
these through a board layer, separate from guest PC devices:

| Physical TTGO facility | Planned emulator connection |
|---|---|
| VGA output | Render the emulated PC VGA state/display |
| PS/2 keyboard/mouse | Feed emulated keyboard/mouse controllers |
| SD card | Store guest disk images behind an emulated disk controller |
| WLAN | Optional debugger/console transport, file transfer, and later a backend for an emulated PC NIC |
| UART/serial | Bring-up console and debugger transport; guest serial is a separate emulated device |

Reuse compatible Tiny386 device components where their exact licences permit;
keep them behind the planned adapter. Reuse board drivers through appropriate
C/C++/Rust boundaries as needed. The board's physical VGA and PS/2 support
is distinct from emulating all corresponding PC registers and behaviours.
WLAN does not automatically become a guest-compatible network card.

Measure firmware fit, display timing and PSRAM before claiming full-system
performance or Windows compatibility. This patch implements CPU flow only;
it does not connect these peripherals yet.

## Preserve the existing network-to-SD installation workflow

The user reports that the current TTGO PC emulator installs directly from
the Internet to SD. Retain this user-facing provisioning model: WLAN-backed
catalog/download selection writes supported system/disk images to SD; the
emulator boots the selected image. Inspect the exact existing installer and
image configuration before choosing an adapter; its repository/revision is
not yet identified here. Reuse it where compatible instead of assuming disk
images must be copied from a PC. Keep downloads/storage in the board layer,
independent of instruction semantics and guest network-card emulation.
