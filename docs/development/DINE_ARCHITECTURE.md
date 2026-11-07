# PIL and Dine: modular structure and current boundaries

PIL already has a useful separation between guest instruction semantics and
the host running them. The i386 descriptions in `isa/i386/` feed the Rust
execution machinery; the interpreter and the offline generated-Rust backend
do not need the host CPU to be an i386. They depend on Rust's supported target
and the surrounding host facilities. The generated backend is currently an
offline proof for fixed images, not a general DOS JIT. Host-specific oracle and
platform integration are separate concerns; portability to every Rust target
or identical support for every backend is not implied.

Dine is PIL's host-implemented DOS/BIOS compatibility environment. It loads
DOS programs and services the calls they make; it is not an executed DOS
kernel. The name describes a responsibility, not a separate crate or an
independently replaceable component today.

| Responsibility | Current code |
| --- | --- |
| Session state, COM/MZ loading, DOS execution loop | `src/dos.rs` |
| DOS and BIOS interrupt dispatch | `src/dos_services.rs`, `src/dos_vectors.rs` |
| DOS files and drives, memory blocks, EXEC and overlays | `src/dos_files.rs`, `src/dos_memory.rs`, `src/dos_exec.rs` |
| DOS console, ANSI, country and Ctrl-Break | `src/dos_console.rs`, `src/dos_color_text.rs`, `src/dos_ansi.rs`, `src/dos_nls.rs`, `src/dos_break.rs` |
| Emulated machine devices | `src/dos_ports.rs`, `src/dos_pic.rs`, `src/dos_pit.rs`, `src/dos_rtc.rs`, `src/dos_serial.rs`, `src/dos_vga.rs` |
| Host-facing session protocol | `src/dos_session.rs`; terminal frontend in `tools/dos_terminal.py` |
| CPU, PIL execution and physical memory | `src/guest.rs`, `src/main.rs`, `src/address.rs`, `src/memory.rs`, `isa/i386/` |

Existing seams already support different hosts: `host_io::Output` abstracts
bounded DOS output; `serial::SerialTransport` defines nonblocking serial I/O;
`Loaded::enqueue_key` injects BIOS key words; and the session protocol allows
frontends to run the machine, supply input and obtain screen state. The
headless implementation can therefore be used independently of the Python
terminal. These interfaces solve different pieces of the problem; together
they are not yet a single complete replaceable machine interface.

The Dine boundary is still imperfect: `Loaded` in `dos.rs` combines CPU, memory,
devices and DOS process state, and its execution loop dispatches device and
BIOS calls alongside CPU execution. Some BIOS behavior lives in the DOS
service dispatcher. The session frontend controls this combined machine;
it is not Dine itself.

If Dine becomes replaceable, separate the machine (CPU, memory and hardware
devices) from a DOS/BIOS service provider with an explicit interface. Keep
the current service and instruction tests when moving code, and permit a
different provider, such as a real booted DOS, to use the same machine. This
is a design direction, not a completed refactor or a claim that a real DOS
kernel can currently boot. A later Rust frontend using SDL could use the
session-facing interface and remove the Python dependency for interactive
use. It should remain a frontend option rather than a requirement of the CPU,
Dine services, headless use, or other frontends; this is a future direction,
not a change to current packaging or a new SDL dependency.
