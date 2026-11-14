# DOS environment configuration

The Python-free Rust route is `pil --dos-text` (Windows:
`.\target\release\pil.exe --dos-text`), or `PILdos.vbs --rust-text` /
`sh PILdos.sh --rust-text`. It reads the same startup and drive configuration.
See [Rust DOS console](DOS_CONSOLE.md) for options and redirected I/O behavior.

## Build once, start without parameters

From the repository directory:

```powershell
cargo build --release --locked
Copy-Item pildos.conf.example pildos.conf
```

On Linux or Termux, use `cp pildos.conf.example pildos.conf`. Edit the paths
before starting. The example requires your local FreeDOS image, `driveD` folder
and `D:\AUTOEXEC.BAT`; the config file does not supply these files.

Start with `python tools/dos_terminal.py`, `PILdos.vbs` on Windows, or
`sh PILdos.sh` on Linux/Termux with a graphical display. `freedos.cmd` also
connects COM1 to its launching console. For a headless terminal, run
`python tools/dos_terminal.py --headless --shell` for a command shell such as
COMMAND.COM (it starts with `/K CTTY COM1` and keeps running after Esc), or
`--headless` alone for any other program (Esc ends the run). The terminal
never guesses this from the program's name.

Automatic engine discovery chooses the newest standard executable it finds,
including `target/release/pil` and `target/debug/pil` (`.exe` on Windows).
It no longer prefers `programs/compiled/pil-engine-v2.exe`. Use `--engine PATH`
to choose a particular build. Restart a running session to use a rebuilt engine.

## Setup screen: `pil --pildos`

`pil --pildos` edits the configuration on a setup screen and starts its
program. It is Rust only (no Python or Tk): the Dine window on Windows, the
terminal on Linux and Termux (`--tty` chooses the terminal on Windows too).

```powershell
.\target\release\pil.exe --pildos                 # .\pildos.conf, or PILDOS_CONFIG
.\target\release\pil.exe --pildos profiles\home.conf
.\target\release\pil.exe --pildos --tty
```

```sh
target/release/pil --pildos
target/release/pil --pildos profiles/home.conf
```

The file is chosen as for the frontend (CONFIG, then `PILDOS_CONFIG`, then
`pildos.conf`). When it does not exist yet, the screen starts from
`pildos.conf.example` in the same folder, and F2 writes the new file.

| Key | Effect |
| --- | --- |
| Up, Down, Tab | Select a setting |
| Enter, or typing | Edit a text setting; Enter accepts, Esc cancels |
| Enter, Space, Left, Right | Change a choice (country, fpu) or turn a switch on or off |
| Del | Remove the setting (its default applies) |
| F2 | Save |
| F10 | Save and start the program; a key after it ends returns to the screen |
| Esc | Quit (twice when there are unsaved changes) |

Saving keeps the file as written: comments, blank lines, the order of the
settings, how each key is spelled (`D:`, `#@program=`, `utc-offset`), the
spacing around `=`, the line ends and a leading BOM. A changed value
rewrites only its own line, a new setting goes at the end, and a removed
one loses its line. The text is checked by the same parser as every other
command before it is written; a value the parser would refuse (an offset
outside -12:00..+14:00, a volume above 100, non-ASCII DOS arguments) is
refused while editing. Settings marked `*` are read only by the Python
terminal; the screen keeps and edits them, but `pil --dos-console` does not
use them.

F10 runs the program in the same window or terminal as `pil --dos-console`
does, with the configuration's drives, country, fpu, mouse, ansi and
utc_offset. Ctrl+] ends the program and returns to the screen.

`pil --dos-console` without PROGRAM also starts the configured program and
args, and with or without it takes fpu, mouse, ansi and utc_offset from the
configuration; its own options override them.

## Example and settings

```ini
program=images/FD14LITE.img::COMMAND.COM
args=/K D:\AUTOEXEC.BAT
C=images/FD14LITE.img
D=driveD
country=358
```

| Setting | Meaning |
| --- | --- |
| `program` | Host DOS COM/EXE file, or `image.img::PATH\PROGRAM.EXE` inside an image. Required when starting the frontend without a program argument. |
| `args` | DOS command tail. `/K D:\AUTOEXEC.BAT` runs that batch file in FreeCOM and keeps the shell open. Optional. |
| `A` through `E` | Mount a host folder or FAT IMG/IMA image. A colon is allowed in the key, such as `D:`. |
| `country` | `1` for USA or `358` for Finland. Optional; the engine defaults to USA. |
| `fpu` | `none` (default) or `80387` to enable the optional 80387 FPU backend. |
| `mouse` | Boolean (`true`/`false` or `1`/`0`). Enables the experimental INT 33h mouse driver. |
| `ansi` | Boolean (`true`/`false` or `1`/`0`). Loads an ANSI.SYS-style console driver. |
| `headless` | Boolean (`true`/`false` or `1`/`0`). Runs the frontend without a window, interacting through the terminal (opens a window only when graphics output is active). |
| `rust_graphics` / `rust-graphics` | Boolean (`true`/`false` or `1`/`0`). Enables native Windows CGA/VGA graphics with `headless` and terminal BIOS keys. |
| `shell` | Boolean (`true`/`false` or `1`/`0`). Marks the startup program as a command shell for `headless` mode (`/K CTTY COM1`, keeps running after Esc). |
| `com1` | `console` or `tcp:PORT`. Connects COM1 to the console or a TCP port. |
| `speaker_volume` / `speaker-volume` | PC speaker click volume `0..100` (`0` mutes; frontend default `5`). |
| `utc_offset` / `utc-offset` | Time zone of the DOS clock: hours such as `2`, `-5`, `5.5`, or `+02:00`, within -12:00..+14:00. (The engine's own `--utc-offset` and session `clock` take seconds.) |
| `engine` | Path to the `pil` executable (relative to the config file, or absolute). |

Keys are case-insensitive and may appear once. Files are UTF-8, optionally with
a leading BOM, and at most 32 KiB. Blank lines and lines starting with `#` or
`;` are ignored. The older `#@program=...` and `#@args=...` syntax remains
supported as settings, rather than comments.

Values are literal: backslashes need no escaping, and paths with spaces need no
quotes. Environment variables, `~`, inline comments and quoted-path syntax are
not expanded. DOS arguments must be printable ASCII and at most 125 characters.

## Paths, multiple drives and write access

Relative host paths are based on the config file's directory, including the
image part of `program`. A config at `profiles/home.conf` uses
`../images/FD14LITE.img` to reach the repository's images folder. Absolute paths
are accepted. Windows targets can translate WSL drive-mount paths to their Windows
equivalents; use paths accessible to the host running the engine.

Images are **read-only**. Folders are **writable**: DOS file creation, changes,
renames and deletion affect that host folder. Programs can save configuration
there. There is currently no per-folder read-only option in this format.

For a config inside a subdirectory such as `profiles`:

```ini
program=../images/FD14LITE.img::COMMAND.COM
A=../images/floppy1.img
B=../images/floppy2.ima
C=../images/FD14LITE.img
D=../driveD
E=../workfiles
country=358
```

Z is reserved for the session's RAM-backed temporary drive, with `TEMP=Z:\`.
Its contents disappear when the session ends. If a standalone loaded program
starts on the empty A drive and configuration supplies C, startup selects C and
updates its initial COMSPEC and program path.

## Experimental 80387 selection

The default DOS session has **no numerical coprocessor**. To opt into the
partial 80387 implementation, use one of these startup switches:

```powershell
python tools/dos_terminal.py --fpu 80387 programs/dos/HELLO.COM
target/release/pil.exe --dos-session --fpu 80387 programs/dos/HELLO.COM
```

The session protocol also accepts `fpu 80387` or `fpu none` **before its first
`run` command**. A later change is refused so the guest never sees a reset
coprocessor halfway through execution. Snapshots report `fpu_present`, and
BIOS INT 11h plus the BDA equipment word reflect the selected device.
Without the switch, existing no-coprocessor behavior is unchanged.

**This backend is incomplete.** It handles selected x87 register operations,
IEEE memory load/store, and finite normalized *register* FADD, FADDP, FMUL,
FMULP, FSUB, FSUBP, FSUBR and FSUBRP. Selected m32/m64 memory operands for
FADD, FMUL, FSUB and FSUBR are supported too. Arithmetic uses an integer-only
80-bit calculation with round-to-nearest-even at 64-bit significand precision.
Masked inexact results set PE and C1 indicates upward rounding. Unmasked PE,
unsupported precision/rounding modes, denormals, infinities, NaNs, exponent
underflow/overflow, division, comparisons and complete x87
exception delivery still stop explicitly. In particular, Turbo Pascal hardware
floating-point (`{$N+}`) programs are **not yet supported**. This opt-in mode
is intended for targeted tests, not full 80387 compatibility.

## Select a configuration

Selection order is:

1. Frontend `--config FILE`.
2. The `PILDOS_CONFIG` environment variable.
3. `pildos.conf` in the working directory.

These files are not combined. A missing explicit or environment-selected file
is an error. An absent default file is allowed when the program is supplied on
the command line. Starting the Python frontend without either a program argument
or a configured program produces an error. The launchers first change to the
repository directory and retain their legacy defaults when no config is selected.

```powershell
python tools/dos_terminal.py --config profiles\home.conf

# Select the same profile through the Windows launchers:
$env:PILDOS_CONFIG = 'profiles\home.conf'
.\freedos.cmd
```

```sh
PILDOS_CONFIG=profiles/home.conf python tools/dos_terminal.py --headless
```

## Command-line overrides

An explicit frontend program argument replaces the configured startup program;
its arguments then come from `--args`, if supplied. When the program comes from
configuration, `--args` overrides the configured args.

Frontend `--country`, `--drive LETTER=PATH`, `--fpu`, `--speaker-volume`,
`--engine`, `--com1`, and `--utc-offset` are applied after configuration and
override those settings. Boolean flags (`--headless`, `--rust-graphics`,
`--mouse`, `--shell`, `--ansi`) enable the corresponding option even if it is
`false` or omitted in the config file. A legacy `--mount PATH` keeps the drive it supplies
instead of that config mount: normally D after loading from an image, or C for
a standalone host program. Paths supplied directly on the command line are
relative to the working directory.

```sh
python tools/dos_terminal.py --config profiles/home.conf --country 1 --drive E=workfiles
```

## Diagnostics and limits

Inspect the configured startup without executing DOS code:

```powershell
.\target\release\pil.exe --pildos-startup profiles\home.conf
```

This prints JSON containing the resolved program and argument string. On Linux
and Termux, the executable is `target/release/pil`. The lower-level
`pil --dos-session --config FILE PROGRAM` applies configured drives and country
but still requires its program argument.

Unknown keys, duplicate settings, unsupported countries and malformed lines
produce errors. If a mount fails, check that its path is relative to the config
file and accessible to the engine.

This module provides the shared configuration foundation for the interpreter
and later compiled DOS runners. Existing experimental standalone executables
contain their old embedded runtime: rebuilding PIL does not update them.
The DOS-to-native generators still need integration into dev before an ordinary
build can recreate them.

