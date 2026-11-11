# PIL Usage & Quickstart Guide

This guide provides a central, quick reference for running, debugging, testing, and developing with PIL and its DOS environment (Dine).

---

## 1. Building PIL

Build the release binary (recommended for performance):
```powershell
cargo build --release
```
The executable is located at `target\release\pil.exe` (or `target/release/pil` on Linux).

---

## 2. Interactive DOS Sessions (GUI)

PIL runs full DOS sessions using FreeDOS `COMMAND.COM` and the Dine BIOS/DOS runtime.

### Quick Start
* **Windows shortcut script**:
  ```powershell
  .\freedos.cmd
  ```
* **Direct Python terminal**:
  ```powershell
  python tools/dos_terminal.py --mount driveD images/FD14LITE.img::COMMAND.COM
  ```
* **Create desktop shortcut**:
  ```powershell
  python tools/makeshortcut.py
  ```
  Creates a "PIL DOS" desktop shortcut (`PIL DOS.lnk` on Windows Desktop, `.desktop` on Linux).

### Mounting Host Folders as Drives
* `--mount PATH`: Mounts a writable host folder as the next available drive (typically `D:`).
  ```powershell
  python tools/dos_terminal.py --mount driveD images/FD14LITE.img::COMMAND.COM
  ```
  Files written by DOS programs to `D:\` are stored directly in your host `driveD\` directory.

### Mounting Multiple Drives (`--drive LETTER=PATH`)
Mount multiple drives (**A:** through **E:**) to host directories or disk images:
```powershell
# Mount programs/dos as A: and driveD as D:
python tools/dos_terminal.py --drive A=programs/dos --drive D=driveD images/FD14LITE.img::COMMAND.COM

# Mount floppy disk images to A: and B:, and a host folder to D:
python tools/dos_terminal.py --drive A=disk1.img --drive B=disk2.img --drive D=driveD images/FD14LITE.img::COMMAND.COM
```

### Auto-Running Batch Scripts at Boot (`--args`)
Pass startup parameters or batch files to `COMMAND.COM`:
```powershell
python tools/dos_terminal.py --mount driveD --args "/K D:\AUTOEXEC.BAT" images/FD14LITE.img::COMMAND.COM
```

### Terminal Customization Flags
* `--country 358`: Finnish dates, times, and decimal comma (`31.10.2026, 14.27.05`).
* `--ansi`: Loads ANSI.SYS driver for escape sequences and color prompts.
* `--speaker-volume 0..100`: Adjusts PC speaker volume (default 5, 0 mutes).
* `--utc-offset HOURS`: Set the DOS clock time zone (e.g. `2`, `-5`, `+02:00`).

---

## 3. Headless DOS Sessions (Console / CLI Only)

Run DOS command-line utilities and interactive shells directly in your current host console without opening the Tkinter text console.

When `--headless` is used:
* It runs directly in your host console (PowerShell, cmd.exe, bash, PuTTY), similar to `z80-cpm`.
* Automatically implies `--com1 console` and defaults `--args "/K CTTY COM1"`.
* If a custom `--args` batch file is given, FreeCOM automatically redirects to COM1 upon startup.
* Windows consoles pass `Ctrl+C` into DOS cleanly instead of terminating Python.
* **Graphics modes**: If a program switches to graphics (VGA Mode 13h, EGA/VGA planar, or CGA), a window pops up only for the graphics display and hides/closes when returning to text mode.

```powershell
# Interactive DOS shell right in your console terminal:
python tools/dos_terminal.py --headless --mount driveD images/FD14LITE.img::COMMAND.COM

# With startup drives and batch file:
python tools/dos_terminal.py --headless --drive d=driveD --args "/K D:\AUTOEXEC.BAT" images/FD14LITE.img::COMMAND.COM
```

---

## 4. Running DOS Programs Directly

You can launch DOS `.COM` or `.EXE` executables directly without going through `COMMAND.COM`:

* **VGA 640×480 16-Color Planar Mode**:
  ```powershell
  python tools/dos_terminal.py programs/dos/MANDVGA.COM
  ```
* **VGA 320×200 256-Color (Mode 13h)**:
  ```powershell
  python tools/dos_terminal.py programs/dos/MANDCYC.COM
  ```
* **Aspect-Corrected Mode 13h Mandelbrot**:
  ```powershell
  python tools/dos_terminal.py driveD/MANDASP.COM
  ```
* **CGA 4-Color Mode**:
  ```powershell
  python tools/dos_terminal.py programs/dos/MANDCGA.COM
  ```
* **Text Mode Calculations**:
  ```powershell
  python tools/dos_terminal.py programs/dos/FACTOR.COM
  ```

---

## 5. Interactive Debugger & REPL

Start the built-in monitor / debugger:
```powershell
target\release\pil.exe --repl
```

### Common REPL Commands
* `load <path>`: Load a binary into memory at `0x100` (e.g. `load programs/dos/MANDEL.COM`).
* `list [count]`: Disassemble the next instructions starting at current EIP.
* `step`: Execute a single instruction.
* `run [n]`: Execute up to `n` instructions.
* `regs`: Display CPU registers and flags.
* `demo <name>`: Load built-in demo (`hello`, `factorials`, `selftest`, `mandelbrot`, `loop`).
* `quit`: Exit the debugger.

---

## 6. Verification and Benchmarks

* **Run all Rust unit tests**:
  ```powershell
  cargo test --locked
  ```
* **Run GUI and terminal test suite**:
  ```powershell
  python -m unittest discover -s tests/gui -v
  ```
* **Compare interpreter and compiled Rust backends**:
  ```powershell
  target\release\pil.exe --verify-backends
  ```
* **Run full-suite benchmark comparison**:
  ```powershell
  target\release\pil.exe --compare-backends 10000 5
  ```
* **Render Mandelbrot benchmark**:
  ```powershell
  target\release\pil.exe --mandelbrot 256 32
  ```

---

## 7. DOS Compatibility Probe

Test a DOS executable against PIL's DOS and instruction support without human interaction:
```powershell
target\release\pil.exe --dos-probe programs/dos/*.COM
```
Produces a detailed report of reached instruction encodings, missing services, execution steps, and exit status.
