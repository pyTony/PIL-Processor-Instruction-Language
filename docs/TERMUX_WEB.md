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
  existing BIOS scan-code interface. Tap the DOS canvas (without the DOS
  mouse enabled) or the **⌨ Keyboard** button to focus the Android text
  input. Press the soft keyboard's **Enter** or **Type + ↵** to submit text
  followed by a DOS Return. **Type** sends text without Return. When the
  input field is empty, its Backspace key deletes the last DOS character
  provided the guest has not already accepted Enter. The on-screen Enter
  and Bksp buttons also work. Browser keystrokes are queued when the BIOS
  keyboard ring fills, rather than silently discarding the final Enter.
- **Layout:** navigation arrows sit in a directional pad; editing and
  command keys have their own group. Less frequently used F1-F10 and page
  keys are in an expandable section.
- **Viewing and rotation:** **Full screen** expands the DOS canvas and
  provides a compact in-view keyboard. **+**, **-** and **Fit** adjust screen
  magnification. Zoomed images scroll inside the viewer, not the DOS guest.
  Rotate between portrait and landscape at any time: the viewer refits to
  the available width and height, including Android browser toolbar/keyboard
  changes. Fullscreen retains the selected zoom factor after rotation.
  If the browser disallows the Fullscreen API, an in-page expanded view is
  used. This never changes emulated VGA resolution or guest execution.
- **Mouse:** --mouse opts into Dine's INT 33h driver. Touch sends its
  existing virtual coordinates and button state; without --mouse, taps
  do not modify guest input.
- **Ctrl+Break:** the browser reports whether the guest DOS session accepted
  or refused the interrupt. An accepted interrupt is not an unconditional
  process-kill request: DOS programs may not check for breaks until their next
  DOS call, and the BIOS does not automatically reset the VGA video mode.
  When the program is still displaying VGA, **Text view** shows its current
  text-page memory for diagnosis; **Graphics view** restores the picture.
  Switching views never changes the guest's actual video mode.
- **Media:** expand **Drives and ANSI.SYS** to mount an existing Termux
  folder or FAT disk image (.img/.ima) onto A: through E:, or unmount it.
  Paths are resolved on the *Termux host*, not through the browser's file
  picker. A relative path such as `driveD` is relative to the directory
  from which `tools/termux_web.py` was launched. The DOS engine rejects
  replacing media that has open DOS files and protects the boot drive from
  unmounting. After mounting, switch DOS to the new drive (e.g. `D:`).
- **ANSI.SYS:** **Enable ANSI.SYS** sends the existing `ansi on`
  session command; ANSI colours and cursor controls are rendered using
  normal DOS video RAM. No duplicate browser ANSI parser is used. This
  one-way installation lasts until the DOS session is restarted.
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
