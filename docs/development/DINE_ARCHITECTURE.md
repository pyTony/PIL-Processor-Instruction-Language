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
| Session state, COM/MZ loading, the execution loop | `src/dos.rs` (`Loaded` = `pc::Machine<dos::Dos>`) |
| The machine (`pc::Machine<F>`: CPU, RAM, devices, disks, history) and the `Firmware` trait | `src/pc.rs` |
| PC devices: PIC, PIT, CMOS/RTC, speaker, 8042, UART, Sound Blaster, VGA registers, diskette and AT disk controllers, disk images | `src/pc/` |
| BIOS: INT 10h text and planar modes, INT 13h disks, INT 15h block move, the vector entries, booting a disk image, the folder disk | `src/bios/` |
| Dine's DOS: INT 21h/2Fh dispatch, files and LFN, memory blocks, EXEC, XMS, INT 33h driver, NLS, Ctrl-Break, ANSI | `src/dos/` |
| Host front ends: session protocol, console window and terminal, setup screen, configuration, audio, media catalog | `src/host/` |
| CPU core (crate `pil-cpu`): PIL parser, interpreter and emitter, decoder, memory and paging, segments, privilege and tasks, 80387, decode cache, precompiled forms, run-time blocks | `crates/pil-cpu/src/`, `isa/i386/` |

The folders are a first step of the split Tony asked for (24 November 2026):
files moved with their module names unchanged. The second step made the
machine its own module: `pc::Machine<F>` holds the CPU, memory and devices,
with `dos:F` for what runs on it, and `dos::Loaded` is `Machine<Dos>`. Code in
`src/pc.rs` and `src/pc/` does not name the DOS, BIOS or front-end modules
(`tools/check_layers.py`, run in CI). The three device events whose effect
depends on the firmware go through `pc::Firmware`, which `Dos` implements:
whether keys reach a program's IRQ1 handler, how an IRQ is delivered, and the
VGA controller ports (Dine's stricter BIOS-mode rules outside a booted
machine). INT 14h moved to `bios/int14.rs`, the AUX device to
`dos/aux_device.rs`, the BIOS INT 09h handler to `bios/int09.rs`.

The third step makes Cargo crates, one at a time. The CPU core came first:
`crates/pil-cpu` (library `pil_cpu`) is a workspace member that the `pil`
package (library `dine`, executable `pil`) depends on, and `src/lib.rs`
re-exports it as its crate root, so `crate::memory::Ram` and the like still
name the same items. The core names no PC, BIOS or DOS module; the one place
that needed Dine, the machine the precompiled forms are enumerated in, is
now a parameter (`precompiled::emit`, given by `form_machines` in
`src/lib.rs`). Its unit tests that load programs through Dine (the 80387,
precompiled forms, block guards and plans) stay in the `pil` package;
helpers only tests use are behind the `test-support` feature, which the
`pil` package enables as a dev-dependency. `cargo test` at the top runs
both packages.

Still mixed: `bios/console.rs` holds DOS console output and
`dos/services.rs` dispatches BIOS and DOS calls; the `bios` modules are
children of `dos` in the module tree. Next: the machine (`src/pc.rs`,
`src/pc/`) as a crate, then the BIOS apart from Dine's DOS.

Existing seams already support different hosts: `host_io::Output` abstracts
bounded DOS output; `serial::SerialTransport` defines nonblocking serial I/O;
`Loaded::enqueue_key` injects BIOS key words; and the session protocol allows
frontends to run the machine, supply input and obtain screen state. The
headless implementation can therefore be used independently of the Python
terminal. These interfaces solve different pieces of the problem; together
they are not yet a single complete replaceable machine interface.

The Dine boundary is still imperfect: the execution loop in `dos.rs` dispatches device and
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
