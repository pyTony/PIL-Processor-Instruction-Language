# DOS runtime debugger: pause and bounded dispatch history

This is the first runtime debugging milestone for DOS sessions. It works with
COM, EXE, and self-unpacking programs because it observes the **running DOS
machine**, not static executable file offsets.

## From a running PILdos window

Use the **host menu** (Shift+Enter or F10, depending on frontend) and open
Diagnostics:

- **Pause / resume guest execution** stops the DOS executor between bounded
  instruction slices. Windows keyboard shortcut: **Ctrl+Shift+F12**.
- **Enable 256-entry dispatch history** records the last 256 dispatched guest
  instructions, including runtime CS:IP, bytes, registers and flags.
- **Save live guest snapshot** appends JSON to `PILdos-snapshots.jsonl`
  and includes the last 16 recorded dispatches and VGA register state.
- **Disable instruction history** removes the ring and returns the normal
  uninstrumented dispatch path.

When paused, the host remains responsive and snapshots can still be captured;
queued guest keys are not delivered and no guest instructions are run. The
pause does not deliberately modify CPU registers, DOS state or interrupt flags.
The window and session can still be closed normally.

Start by enabling history, run Wolfenstein until the blank/white display,
pause, and save a snapshot. A compressed EXE such as LZEXE is supported by
watching the runtime instruction stream after its loader has unpacked code.
The reported CS:IP is a **runtime location**, not a stable disk-file offset.

## Limitations

The pre-dispatch trace shows the last 16 records in snapshots, from a
256-record ring. It is not a complete post-instruction trace or a reverse
debugger. It does not yet offer DOS CS:IP breakpoints, memory or I/O
watchpoints, symbolic addresses, persistent per-program `.dbg` configuration,
or full 256-record exports. Debug mode is opt-in to avoid slowing down native
block execution when disabled.

A host-side pause is not an x86 hardware interrupt, guest Ctrl+Break, or
an instruction-level breakpoint. It takes effect after the current bounded
execution slice ends.
