# PIL / Dine runtime 0.38.0-alpha.1

PIL runs DOS COM/MZ programs through a shared CPU and DOS/BIOS implementation.
Dine exposes the same program-independent runtime as a shared library. This
is a bounded compatibility environment, not a complete bootable PC or DOS kernel.

## Dependencies

Build the source with a Rust toolchain and Cargo. Running the built Rust DOS
console needs no Python. The terminal adapter uses the locked Crossterm crate.
Windows provides a separate native graphics window; other hosts provide the
terminal console. Native packages run without a Rust toolchain.

Python remains optional for the legacy graphical debugger, Tk/COM1 frontend,
shortcut-generation helpers, and development/packaging utilities. The default
PILdos shortcut opens the Rust setup screen (`pil --pildos`); `--python`
selects the legacy Tk terminal and `--rust-text` selects `pil --dos-text`.

## Build and run DOS

From this directory:

```powershell
cargo build --release --locked
Copy-Item pildos.conf.example pildos.conf
.\target\release\pil.exe --dos-text
```

On Linux/Termux:

```sh
cargo build --release --locked
cp pildos.conf.example pildos.conf
./target/release/pil --dos-text
```

Edit the example configuration before running. It refers to your own local
DOS image and drive folder, which are not included. `PILDOS_CONFIG` or
`--config FILE` selects another configuration. Paths are relative to its folder.
With an explicit program:

```text
pil --dos-text --mount driveD --fpu 80387 images/FD14LITE.img::COMMAND.COM
```

`PILdos.vbs --rust-text` and `sh PILdos.sh --rust-text` are equivalent shortcuts;
`PILDOS_ENGINE` can select a different built executable. Windows terminal mode
opens the separate native graphics window as needed. `--dos-console --window`
selects the Windows text/graphics window frontend. Ctrl+] exits terminal mode.

`--ansi` loads the shared ANSI.SYS-style guest console driver. `--mouse` enables
the optional DOS mouse driver. Guest screen rendering and BIOS keys are shared
with the interpreter/backend session; no program-specific engine is generated.

Redirected input/output automatically selects a UTF-8 transcript with bounded
line input. `--plain` or `DINE_TEXT=headless` forces it. Characters are translated
to CP437, LF/CRLF becomes DOS Enter, and EOF during a guest read is an error.
Control sequences are escaped in transcript output. It captures DOS output,
not direct video-memory writes, BIOS drawings or graphics.

## Other runtime entry points

- `pil --pildos` opens the Rust configuration editor and launcher.
- `pil --dos-session ...` serves the existing JSON frontend protocol.
- `pil --dos-probe PROGRAM` reports a bounded diagnostic run.
- `pil --dos-list IMAGE` and `--dos-extract IMAGE NEW_DIRECTORY` inspect
  supported FAT16/ZIP containers.
- `pil --repl` opens the built-in text debugger without Python.
- `pil --selftest` runs the built-in diagnostic.
- `pil --help` lists commands and options.

Build the shared Dine library with:

```text
cargo build --release --locked --lib --features dine-library
```

The C ABI accepts guest bytes at runtime and uses the same Rust console adapter.
A Dine launcher is a loader plus guest bytes on that shared library, not a copy
of the CPU or DOS implementation. `DINE_TEXT=auto`, `terminal` or `headless`
selects its host I/O behavior.

## Documentation and provenance

- [Rust DOS console](docs/DOS_CONSOLE.md)
- [DOS configuration](docs/DOS_CONFIG.md)
- [Feature matrix](docs/FEATURE_MATRIX.md)
- [Release notes](docs/RELEASE_NOTES.md)
- [Debugger](docs/DEBUGGER.md)
- [DOS probe](docs/DOS_PROBE.md)
- [Containers](docs/CONTAINERS.md)
- [PIL language reference](docs/PIL_REFERENCE.md)
- [References and credits](docs/REFERENCES_AND_CREDITS.md)

`DELIVERY.json` records the exact source revision and version; `SHA256SUMS`
records exported file contents. Development, fixtures and CI stay on dev;
main receives only this runtime export through a separate reviewed PR.
