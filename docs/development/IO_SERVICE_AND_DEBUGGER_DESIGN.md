# DOS compatibility services and debugger interface — design proposal

28 October 2026. User direction: route services through an I/O processing
service and draw interface ideas from their other projects, including lispy3.
The user clarified that DOS I/O should be provided in the way Wine provides
Windows services: a compatibility layer backed by host services. There is no
separate Dine project to locate. This is an architectural analogy, not a claim
that Wine implements DOS services or that its code is being reused.
No runtime implementation changes accompany this document.

## Source review

- [lispy3, abc3d616](https://github.com/pyTony/lispy3/tree/abc3d616b4292753c349dfa5e690483a7d717954):
  `lispy_machine.pyw` separates editor, Listener and Inspector; evaluation runs
  away from Tk and UI calls are queued. Listener writes are batched.
  `lispy_support/lispy_terminal.py` provides separate program-output/input
  routing and a terminal viewer. `basic_io.py` injects input through a reader;
  `listener_output.py` incrementally filters ANSI for a plain-text view.
- [z80-cpm, 588c356b](https://github.com/pyTony/z80-cpm/tree/588c356bcf280d00585b26bda0a4c70ba5bdaa4c):
  `debug_gui.pyw` presents disassembly/breakpoints, changed registers, stack,
  memory, guest console and command entry together. Its console adapter and
  bounded execution/yield behavior are directly relevant to a CPU debugger.
  The repository carries an MIT licence, copyright2026 pyTony.

These are design references, not added dependencies. No source has been copied.
A root licence was not found in the reviewed lispy3 tree; preserve its existing
third-party provenance if code is later adapted. Any substantial z80-cpm reuse
must retain its MIT notice. No project-wide PIL licence is invented here.

## Proposed boundaries

1. **CPU/PIL execution** owns CPU, RAM, segments, precise effects and retirement.
   It does not call Tk, print guest characters or depend on a GUI timer.
2. **DOS compatibility services** decode supported DOS requests and apply their
   guest register/memory contracts using host-backed providers. CPU execution
   remains in PIL because Wine-style API provision does not execute x86 code on
   an ARM/ESP32 host. Hardware-facing BIOS services need an explicit machine
   model. Future CPU INT/IRET delivery remains distinct from the current host shim.
3. **I/O service** owns bounded guest input/output queues and device state.
   Console capture and a text-screen provider are separate interfaces; port I/O
   and serial devices can use the same machine boundary without treating every
   port as a DOS console operation. A DOS compatibility provider translates
   supported guest API requests into these host-backed operations. Its Rust
   interfaces and frontend transport remain implementation decisions.
4. **Debugger controller** owns run/step/pause, breakpoints and snapshots through
   the same machine instance used by the CLI. Host commands and guest keyboard
   bytes use distinct channels.
5. **Presentation** renders snapshots and guest-output events. A Tk frontend can
   follow lispy3/z80-cpm layout and event-routing patterns (see [IDE plan](IDE_PLAN.md)
   for the full editor/listener/inspector specification); CLI and headless
   execution remain supported. Do not create another CPU or DOS implementation
   in Python merely to provide the interface.

An output request must validate capacity/device access before the DOS bridge
changes AL or retires. A full queue means a defined wait/stop outcome, not
silently dropping bytes. Input-empty, EOF, disconnect and cancellation are
separate results. Tests and batch probes need a deterministic headless provider.

For INT10/AH08, introduce an actual emulated character/attribute cell model and
page/cursor state. The GUI displays that state; reading host terminal contents
or reconstructing cells from escaped output is not the BIOS implementation.
Choose the relationship with guest video RAM explicitly before adding a second
screen buffer. A presentation ANSI filter must never alter the raw guest trace.

## Debugger behavior

- Disassembly with current IP and breakpoints; registers/segments/flags with
  changes highlighted; stack/memory view; separate guest console and command
  Listener; Inspector for selected machine state and stop details.
- Run executes bounded quanta; pause is cooperative at an instruction or
  committed REP-element boundary. Preserve existing partial-REP/retirement
  semantics. Do not adapt lispy3's Python asynchronous thread-exception stop
  mechanism to interrupt a Rust CPU transaction.
- Queue UI updates onto Tk's owning thread, batch output and rate-limit display
  refresh. UI batching does not delay or reorder the authoritative machine
  effects. Keep controls responsive while execution or input waits.
- Keep binary guest output byte-exact. Character decoding/colour are view
  options. Commands, diagnostic errors, guest output and stop events stay
  distinguishable. Screenshots and UI text are not execution-oracle evidence.

## Implementation order and acceptance

Extract a narrow DOS compatibility-service interface around existing
capture/input behavior with
regressions for ordering, capacity, faults and waits; retain existing headless
DOS probe results. Build the screen provider and BIOS contracts on that layer.
Expose controller snapshots/events and cooperative execution, then implement
the optional desktop frontend using the reviewed projects as references.

Validate identical machine state and raw I/O for headless/CLI/frontend control
sequences, including partial REP, breakpoints, input wait, pause and disconnect.
Validate any CPU/backend changes with the existing full-state comparisons.
Keep DOS API contracts in the compatibility provider and GUI concerns in the
frontend. Validate each added DOS/BIOS service against its documented guest
contract. BIOS and CPU interrupt dispatch remain separate explicit capabilities.
