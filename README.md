# PIL PC emulator prototype

A 386 CPU description language, Rust token interpreter and Rust-source backend.
The eventual target is a PC emulator on TTGO ESP32 with VGA, PS/2, SD and WLAN.
This is an early CPU/debugger prototype; it does not boot an OS or load x86 binaries.

## Build and run

```sh
cargo test
cargo run -- --repl
python3 tools/verify_descriptions.py
python3 tools/verify_flow.py
```

Version 0.4 passed 28 Rust tests on the user's Termux phone. Version 0.5 adds
serial mocks and the REPL, with 46 Rust tests awaiting compilation/verification.
The authoring environment has no Rust compiler. See [continuation notes](CLAUDE.md).

Try `help`, `list`, `step`, `run`, `regs`, `flags`, and `break 0x102` in the REPL.
For serial: `demo echo`, `run`, `serial rx 41`, `run`, `serial tx`.

## Source layout

| Directory | Contents |
|---|---|
| src/ | Interpreter, code generator, flow model, mock serial and debugger |
| isa/i386/ | Instruction/condition descriptions |
| tests/unit/ | CPU, transport and debugger unit tests |
| tools/ | Independent Python description/flow checks |
| docs/ | Design, language, debugger, roadmap, provenance and history |

- [Debugger guide](docs/DEBUGGER.md)
- [Language](docs/LANGUAGE.md)
- [Roadmap](docs/ROADMAP.md)
- [References and credits](docs/REFERENCES_AND_CREDITS.md)
- [Revision history](docs/REVISION_HISTORY.md)
- [First GitHub push](docs/GITHUB.md)

Continue in `~/pil_proof`; updates are guarded in-place patches.
No third-party emulator source or disk images are bundled. A distribution
licence for the project's own code has not yet been selected.
