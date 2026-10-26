# PIL PC emulator prototype

A processor instruction language, Rust interpreter and Rust-source generator,
with an interactive debugger and a small executable 386 test program.
The eventual target is a TTGO ESP32 PC emulator with VGA, PS/2, SD and WLAN.
This version is a host prototype; it does not boot DOS or Windows.

## Build and debug

In Termux or Linux, with Rust installed:

```sh
sh tools/build.sh
sh tools/debug.sh
```

On Windows PowerShell:

```powershell
./tools/build.ps1
./tools/debug.ps1
```

These build the debugger and write `target/programs/selftest.bin`, then load
that real byte program at CS:0100. In the debugger, type one command per line:

```
list
step
serial rx 41
run 1000
check
regs
serial tx
```

Success reports `PASS` and `DX=600D`. Guest serial output appears directly in
the console: `serial output: 41 | A`. `serial tx` also retrieves queued bytes.
The program checks arithmetic, flags, conditional branches, register views,
16/32-bit addresses, DS/SS/FS memory and serial input/output. It tests the
implemented execution path, not the complete 386 instruction set.

For an automatic check: `sh tools/debug.sh --check`, or on Windows
`./tools/debug.ps1 -Check`. `cargo test --locked` runs the full Rust suite.

## Debugger and deliveries

`help`, `list`, `step`, `run`, `regs`, `flags`, `mem`, `poke`, `break` and
`reset` inspect and control execution. `demo counter`, `demo echo` and
`demo memory` retain the earlier predecoded examples. `load PATH` loads another
raw binary within the [supported monitor profile](docs/DEBUGGER.md).

Native build archives include the executable, launchers, selftest binary,
listing and documentation. See [delivery instructions](delivery/README.md).
CI archives are Linux x86-64 and Windows x86-64; they do not run on Android or
ESP32. Termux builds produce the phone's own binary. Releases remain prototypes.

## Project branches and documentation

`main` is the delivery branch. `dev` contains current development; changes are
tested there and proposed to `main` through a PR. Pull `main` after approving
a release PR. [Development status](docs/development/STATUS.md) records the
verified baseline and current work separately.

- [Debugger guide and instruction profile](docs/DEBUGGER.md)
- [Language](docs/LANGUAGE.md)
- [References and credits](docs/REFERENCES_AND_CREDITS.md)
- [Revision history](docs/REVISION_HISTORY.md)
- [Development roadmap](docs/ROADMAP.md)
- [Claude continuation notes](CLAUDE.md)

Sources: `src/`, descriptions: `isa/i386/`, tests: `tests/unit/`, reproducible
guest fixtures: `programs/`, helpers: `tools/`. No third-party emulator source,
BIOS or OS image is bundled. A project-wide licence has not yet been selected.
