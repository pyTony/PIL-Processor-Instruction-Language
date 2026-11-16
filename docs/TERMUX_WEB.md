# PILdos on Termux: low-memory builds and no-X11 graphics

PILdos uses the **same general 80386 interpreter, DOS/BIOS services and VGA
state** on Windows, Linux and Android. Only the frontend changes. On Android
there is no need to install X11, VNC, Tk or a desktop environment.

## 1. Build with lower RAM requirements

From the cloned repository in Termux's **home directory**, not on Android
shared storage (/sdcard), run:

~~~sh
pkg install rust python
sh tools/build_termux.sh
./target/debug/pil --help
~~~

This build helper uses one Cargo compiler job, disables incremental builds and
debug symbols, reduces Rust code-generation parallelism, and builds **only**
the pil executable. No shared Dine library or optional graphical host target
is requested. It avoids the duplicate heavy build that a bare cargo run
can trigger.

For an optimized executable on a phone with sufficient free memory:

~~~sh
sh tools/build_termux.sh --release
./target/release/pil --help
~~~

For diagnosing an actual **runtime** crash, run the already-built binary
directly. This avoids confusing a failed rebuild with a program failure:

~~~sh
set -o pipefail
RUST_BACKTRACE=1 ./target/debug/pil --help 2>&1 | tee pil-termux.log
echo "exit status: $?"
~~~

The final two lines above use Bash (Termux's usual interactive shell).
An Android low-memory kill might show only "Killed" with exit status 137;
a Rust panic normally prints a panic location and backtrace. Neither outcome
should be mistaken for a guest DOS CPU exception.

## 2. Start the browser interface

The frontend requires only Python's standard library. It launches the existing
pil --dos-session subprocess; it does **not** compile Rust. It uses a
loopback-only HTTP server and your phone's normal Android browser for text,
CGA, EGA and VGA modes, with optional touch mouse and an on-screen DOS keyboard.

~~~sh
python tools/termux_web.py --engine target/debug/pil --mouse \
  --open images/FD14LITE.img::COMMAND.COM
~~~

With a release build, select --engine target/release/pil instead. If
termux-open-url is not available, open the printed address manually in
Android Chrome or Firefox:

    http://127.0.0.1:8765/

The --open option is optional. The browser and PIL remain on the **same
Android device**; nothing is exposed on the Wi-Fi network.

To mount a host folder (e.g. driveD) for DOS file access:

~~~sh
python tools/termux_web.py --engine target/debug/pil \
  --mount driveD --fpu 80387 --mouse --open \
  images/FD14LITE.img::COMMAND.COM
~~~

When no program path is specified, the frontend reads the configured DOS
startup through the same pildos.conf logic as the normal terminal frontend.
--config FILE selects a different configuration and --args TEXT passes
a DOS command line.

## Input and output

- **Display:** renders 80-column CP437 text, mode 13h 320×200, CGA 4/5/6,
  and planar EGA/VGA 0Dh/0Eh/10h/12h from the existing session snapshots.
  Scaling uses the display's 4:3 aspect ratio. This is host-only rendering.
- **Keyboard:** hardware keys, F1–F10, arrows and Esc work through the
  existing BIOS scan-code interface. The on-screen key strip and the text
  field work on touchscreens. Submit short text chunks to avoid filling a
  guest's BIOS keyboard buffer.
- **Mouse:** --mouse opts into Dine's INT 33h driver. Touch sends its
  existing virtual coordinates and button state; without --mouse, taps
  do not modify guest input.
- **CPU:** the web page sends bounded run N buffered commands and fetches
  snapshots. Pause/resume changes only how frequently the host runs the guest.
- **Stop:** Ctrl+C in the Termux process closes the server and guest engine.

The server binds only 127.0.0.1; write requests require JSON and matching
browser origin. Do not reverse-proxy this interface onto the public network.

## Current limitations

This is a first browser frontend, not native Android OpenGL. Audio currently
uses the existing engine state but does not yet play PCM or Sound Blaster audio
in the browser. DOS mouse callbacks and unusual keyboard layouts may need
additional shared input-protocol support. Browser background-tab throttling can
slow guest execution. High-frequency 640×480 graphics consume more memory and
CPU than text modes; lower the CPU slice or pause when debugging.

Termux:GUI is an alternative future native-controls frontend. It can consume
the same DOS-session protocol without changing the emulator.
