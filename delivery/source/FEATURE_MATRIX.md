# PIL / Dine 0.38.0-alpha.1 feature matrix

This runtime is a bounded DOS compatibility environment and CPU-language proof,
not a complete PC emulator or DOS kernel. Supported subsets retain explicit
faults/stops for behavior outside their implemented scope.

| Area | Current runtime |
|---|---|
| Rust execution | Shared interpreter and program-independent precompiled instruction forms with fallback/verification infrastructure; no per-program CPU/DOS library builds |
| CPU language | Typed PIL effects, register/flag/memory operations, precise validation and Rust emission |
| x86 | Broad 16/32-bit instruction support; real/protected/V86, segmentation, paging, interrupt and task subsets; not universal ISA or OS compatibility |
| x87 | Optional 80387 subset selected with `--fpu 80387`; unsupported operations stop explicitly |
| DOS import | Bounded COM and MZ loading, relocations, PSP and executable windows |
| DOS services | Shared console, file/drive, memory, process/EXEC and compatibility service subsets |
| Storage | Configurable host-folder/image/memory drives, FAT16 and bounded ZIP stored/deflate containers; third-party images/programs are user inputs |
| BIOS/device model | Shared video/keyboard, timer/PIT/PIC, serial, RTC and related implemented services; not a full hardware machine |
| Text/ANSI | Authoritative DOS video-memory text screen and optional ANSI.SYS-style CON driver |
| Rust terminal | CP437 BIOS keys, shared screen renderer, configured startup; no Python needed |
| Redirected I/O | Bounded UTF-8 line input to CP437 keys, explicit EOF errors, escaped DOS transcript; no direct-VRAM/BIOS drawing capture |
| Windows presentation | Native text/graphics window and terminal-mode graphics window; existing CGA/EGA/VGA scanout reused |
| Mouse | Optional DOS driver and native-window mouse events; no terminal mouse capture in this route |
| Graphics limits | Custom font/palette/blink fidelity varies by frontend; no new sound or complete hardware timing claim |
| Dine library | Program-independent C ABI taking any supported guest at runtime; shared Rust I/O adapter |
| Configuration | Same `pildos.conf` startup, drives, country and Rust-console options; Rust `--pildos` editor preserves comments, ordering and line endings |
| Optional Python | Legacy Tk debugger/console and COM1/TCP routing, helpers and development tools; not required by Rust DOS execution |
| Debugging/checks | Built-in text debugger, diagnostic/probe commands, backend comparisons and original regression fixtures on dev |
| Platforms | Windows native window; other hosts terminal console. Build for the target host; cross-platform completeness is not assumed |

See [Rust DOS console](DOS_CONSOLE.md), [configuration](DOS_CONFIG.md),
[release notes](RELEASE_NOTES.md) and [debugger](DEBUGGER.md) for details.
