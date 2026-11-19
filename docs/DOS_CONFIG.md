# DOS environment configuration

For ordered floppy sets and the Rust window's **Ctrl+Shift+M** host drive
menu, see [multi-disk installation media](DOS_INSTALL_MEDIA.md).

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

`PILdos.vbs` on Windows and `sh PILdos.sh` on Linux/Termux open the Rust
PILdos start page below (`pil --pildos`): Enter starts the configured
program, S opens the setup screen. The
older Python/Tk terminal is `python tools/dos_terminal.py`, or
`PILdos.vbs --python` / `sh PILdos.sh --python` (Linux/Termux needs a
graphical display for it). `freedos.cmd` also uses the Python terminal and
connects COM1 to its launching console. For a headless terminal, run
`python tools/dos_terminal.py --headless --shell` for a command shell such as
COMMAND.COM (it starts with `/K CTTY COM1` and keeps running after Esc), or
`--headless` alone for any other program (Esc ends the run). The terminal
never guesses this from the program's name.

After Enter or F10, PILdos shows **Starting DOS...** while it loads the program
and drives. Image-backed programs use the bytes already read for the mounted
image. On Windows, setting `DINE_VIDEO_LOG` to a log filename also records the
program-and-drive loading time, which helps distinguish loading delays from
the DOS program's own startup.

When the desktop shortcut needs to rebuild the shared runtime, an **Updating
PIL DOS** progress console remains visible until the build finishes. Keep it
open; the PILdos start window opens after the update succeeds.

The `PILdos.vbs` and `PILdos.sh` desktop launchers prefer `target/release/pil`,
then `target/debug/pil`, and only use a root-level `pil` as a fallback (`.exe`
on Windows). Thus a stale root executable cannot shadow a newly built Cargo
release. `PILDOS_ENGINE` overrides this order when set. The Python debugger
and Python/Tk terminal use the most recently modified standard build, or an
explicit `--engine PATH`. Restart a running session to use a rebuilt engine.

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

When the file exists, is valid and names a `program`, a start page comes
first. It shows the program, its command line and the drives, and the
result of the last run.

| Key on the start page | Effect |
| --- | --- |
| Enter (or F10) | Start the program; a key after it ends returns to the start page |
| S, F2 or Tab | Open the setup screen |
| Esc | Quit |

Without a file or a program the setup screen opens directly.

| Key | Effect |
| --- | --- |
| Up, Down, Tab | Select a setting |
| Enter, or typing | Edit a text setting; Enter accepts, Esc cancels |
| Enter, Space, Left, Right | Change a choice (country, fpu, host_pointer, own_programs) or turn a switch on or off |
| Del | Remove the setting (its default applies) |
| F2 | Save |
| F10 | Save and start the program; a key after it ends returns to the start page (or this screen, when there is none) |
| Esc | Back to the start page, or quit when there is none (twice when there are unsaved changes; the second Esc drops them) |

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
does, with the configuration's drives, country, fpu, mouse, ansi,
utc_offset and host_pointer. Ctrl+] ends the program and returns to the screen.

`pil --dos-console` without PROGRAM also starts the configured program and
args, and with or without it takes fpu, mouse, ansi, utc_offset and
host_pointer from the
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
| `speaker_volume` / `speaker-volume` | PC speaker volume `0..100` (`0` mutes; frontend default `5`). |
| `host_pointer` / `host-pointer` | `auto` (default), `show` or `hide`: the Windows pointer over the `pil --dos-console`/`--pildos` window. `auto` hides it while the program's INT 33h cursor is on screen, so only one arrow shows; `show` keeps it for programs that draw their own pointer without the driver's; `hide` always hides it over the window. The Python terminal ignores it. |
| `own_programs` / `own-programs` | Drive letter `A`..`E` (default `D`) or `none`. At every start (`pil --pildos` screen, F10, `pil --dos-console`, `--dos-session`, the Python terminal with this file), when that drive is a host folder, the programs in `programs/dos` next to this file (COM, EXE, BAT and BAS files; no sources or notes) are copied into its `PIL` folder, which is created when missing: a missing program is added, and a file of the same name is replaced only when it is an earlier version of ours (its SHA-256 is in `programs/dos/versions.sha256`). Anything else is kept. Copies of ours that earlier starts left in the drive's root are removed. When the drive is mounted, the first program's environment has `PATH=D:\PIL` (that drive's letter). The setup screen's status line says what changed. See `docs/DRIVE_D.md` on dev. |
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
IEEE memory load/store, finite arithmetic and comparisons, and selected integer
conversions. `FISTP` supports signed 16-, 32- and 64-bit destinations through
the shared conversion implementation. Integer stores obey the control word's
rounding mode, check the complete destination before writing, and pop only
after a successful store. Unsupported exceptional conversions stop explicitly.
Arithmetic uses an integer-only 80-bit implementation; supported precision,
rounding and exceptional values depend on the operation. Masked inexact results
set PE, and C1 records upward rounding where applicable. Complete x87 exception
delivery is not implemented. Some hardware-floating-point programs run, but
support for an arbitrary Turbo Pascal or other 80387 application is not guaranteed.

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

