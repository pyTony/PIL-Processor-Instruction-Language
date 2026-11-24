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
| Session state, COM/MZ loading, the execution loop | `src/dos.rs` (`Loaded` = machine fields plus `dos::Dos`) |
| PC devices: PIC, PIT, CMOS/RTC, speaker, 8042, UART, Sound Blaster, VGA registers, diskette and AT disk controllers | `src/pc/` |
| BIOS: INT 10h text and planar modes, INT 13h disks, INT 15h block move, the vector entries, booting a disk image, the folder disk | `src/bios/` |
| Dine's DOS: INT 21h/2Fh dispatch, files and LFN, memory blocks, EXEC, XMS, INT 33h driver, NLS, Ctrl-Break, ANSI | `src/dos/` |
| Host front ends: session protocol, console window and terminal, setup screen, configuration, audio, media catalog | `src/host/` |
| CPU, PIL execution and physical memory | `src/guest.rs`, `src/lib.rs`, `src/address.rs`, `src/memory.rs`, `isa/i386/` |

The folders are a first step of the split Tony asked for (24 November 2026):
files moved with their module names unchanged. Some files still mix layers
(`pc/serial.rs` holds INT 14h and DOS AUX next to the 8250; `bios/console.rs`
holds DOS console output; `dos/services.rs` dispatches BIOS and DOS calls;
`pc/keyboard.rs` holds the BIOS INT 09h handler). The next steps are to split
those, make `Loaded` a `Machine` plus `Dos` with a `Firmware` trait for INT n,
and then make `pc`, `bios` and `dine` separate crates in a Cargo workspace.

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
