# Opt-in generic DOS execution history

This feature is independent of guest programs and adds no new I/O or
instruction semantics. It works in any Dine DOS session, including the
Retro browser, the built-in PILdos setup frontend, and DOS benchmark suites.

On Windows PowerShell, launch PIL directly from the relevant checkout:

```powershell
$env:PIL_DOS_HISTORY = '64'
.\target\release\pil.exe --pildos
```

For the Retro browser, use `wscript.exe .\PILmedia.vbs` in the same
PowerShell process and checkout so that the environment variable is inherited.
A desktop shortcut launched from Explorer may **not** inherit variables set
in an already-open terminal.

`PIL_DOS_HISTORY` defaults to disabled and accepts counts 0..256 (higher
counts are capped at 256; invalid input disables the recorder). When enabled,
`PILdos-stops.jsonl` records up to N `recent_dispatches` snapshots on an
abnormal stop, including their sequence, CS:EIP, up to eight upcoming bytes,
general registers, selectors, EFLAGS and machine status word.

These are **pre-dispatch snapshots**, not proof that all entries retired.
An interrupt, fault, BIOS or DOS service may intervene. Diagnostic mode
bypasses multi-instruction native/code-cache blocks, so it is slower than
normal operation and must not be used for performance benchmarking. The
history persists across session slices without storing an unbounded trace.
With the variable unset, guest execution and log format remain unchanged.

The diagnostic deliberately does not accept, skip, or emulate the unexpected
`INSB` operation. It is intended to identify the true preceding control flow.
