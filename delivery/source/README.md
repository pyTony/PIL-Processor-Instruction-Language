# PIL 386 debugger

A Rust interpreter for processor instruction descriptions, with a small 386
binary monitor and interactive debugger. This prototype runs its included guest
program; it does not yet boot DOS or Windows.

## Build and run

Install Rust, then run these commands from this directory.

Termux / Linux:

```sh
sh tools/build.sh
sh tools/debug.sh
```

Windows PowerShell:

```powershell
./tools/build.ps1
./tools/debug.ps1
```

The build writes the executable and `target/programs/selftest.bin`. The debug
script loads that program at CS:0100. Type one command per line:

```
list
step
serial rx 41
run 1000
check
regs
serial tx
```

Expected: `PASS`, `DX=600D`, and guest output `serial output: 41 | A`.
`serial rx` supplies guest input; ordinary debugger commands are separate.
For an automatic check, run `sh tools/debug.sh --check` or on Windows
`./tools/debug.ps1 -Check`.

## Other uses

- `sh tools/build.sh --release` builds an optimized executable.
- `sh tools/debug.sh PATH` opens another raw binary within the supported profile.
- `cargo run --locked -- --repl` opens the built-in predecoded counter demo.
- `cargo run --locked -- --selftest` runs the included program automatically.
- `cargo run --locked -- --emit-rust 32 add_ram` emits a Rust module from an
  embedded instruction description. Operand widths are 8, 16 or 32.

[Debugger commands and supported instructions](docs/DEBUGGER.md) explain the
monitor environment and limitations. See [release notes](docs/RELEASE_NOTES.md)
and [references and credits](docs/REFERENCES_AND_CREDITS.md).
`DELIVERY.json` identifies the source revision; `SHA256SUMS` covers this tree.

This branch contains the portable build/run source, descriptions embedded in
the binary, helper scripts, sample program and user documentation. Development
is maintained on [dev](https://github.com/pyTony/pil_proof/tree/dev).

No external Cargo dependencies are required. Host builds use Rust's standard
library. A Termux build runs on the phone; Linux/Windows desktop executables
are not Android or ESP32 binaries. No BIOS, OS image or third-party emulator
source is bundled. A project-wide licence has not yet been selected.
