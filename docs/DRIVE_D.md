# Drive D: (your own DOS programs)

## Writable copies of FAT images

For a FAT12 or FAT16 floppy or hard-disk image, the terminal can copy the
files to a host folder and mount that folder as a writable drive:

```sh
python3 tools/dos_terminal.py --headless --drive-from-image A=images/disk.img images/FD14LITE.img::COMMAND.COM
```

The new folder is `drives/A/LABEL-IDENTIFIER`, relative to the current working
directory. The label comes from the image, or its filename when no label is
available. The identifier is a checksum of the image, distinguishing images
with the same label. The terminal reports the actual folder path. Repeat the
option with other letters to import multiple images. Each destination must be
new: an existing folder is never overwritten. For later sessions, mount the
reported folder directly, for example:

```sh
python3 tools/dos_terminal.py --headless --drive A=drives/A/LABEL-IDENTIFIER images/FD14LITE.img::COMMAND.COM
```

DOS changes persist in the host folder and do not alter the original image.
This is a file-level copy, not a writable image or a bootable disk: raw sector
access and disk capacity are not replicated. Empty directories and ZIP files
are retained, but FAT volume metadata, long filenames, file attributes and
timestamps are not preserved. Unsupported names cause the import to fail.
If an I/O error leaves an incomplete new folder, the error identifies it; remove
that incomplete folder before retrying. The `drives` folder is git-ignored.
ISO/CD-ROM images are not supported by this option.

`freedos.cmd` mounts a host folder as drive D: in the Dine session. By default
that is `driveD` in the checkout. Git ignores this folder, so its contents are
yours: GW-BASIC, Turbo Pascal, benchmarks, BASIC files and so on. Third-party
programs never go into the repository. The project's own DOS programs
(FACTOR, MANDEL, MANDBBC, MANDCGA, MANDVGA, TIMEIT) are in `programs/dos`.

Because the folder is ignored, `git stash -u`, `git checkout` between branches
that do not track it, and `git clean -fd` leave it alone. Only `git clean -x`
and `git stash -a` touch ignored files, so do not use those here.

To keep drive D: outside the checkout instead, set `PILDRIVE` before running
`freedos.cmd`:

    $env:PILDRIVE = "../PILdrive"      # PowerShell
    .\freedos.cmd

    PILDRIVE=../PILdrive                         # sh: pass it to --mount
    python3 tools/dos_terminal.py --mount "$PILDRIVE" images/FD14LITE.img::COMMAND.COM

## Updating from a version that still tracked driveD

Before November 2026, dev tracked some files under `driveD`. Pulling the
commit that stops tracking them makes git delete those files from your
folder. Copy the folder first and copy it back afterwards:

    Copy-Item driveD ..\driveD-backup -Recurse      # PowerShell
    git pull origin dev
    robocopy ..\driveD-backup driveD /E /XC /XN /XO

    cp -a driveD ../driveD-backup                    # sh
    git pull origin dev
    cp -an ../driveD-backup/. driveD/

Older branches still track `driveD`. Checking one out can overwrite or remove
files there, so make the same copy before switching to an old branch.

## Desktop shortcut

`python tools/makeshortcut.py` makes a "PIL DOS" shortcut that opens the
PILdos setup screen (`pil --pildos`, Rust): it edits `pildos.conf` (starting
from `pildos.conf.example` when there is none yet) and F10 starts the
configured program, for example FreeCOM with drive D: and `D:\AUTOEXEC.BAT`.
Build first (`cargo build --release --locked`); only the shortcut creator
itself uses Python.

- Windows: `PIL DOS.lnk` on the Desktop, starting `PILdos.vbs`, which runs
  `target\release\pil.exe --pildos` in the Dine window (Pillow is needed only
  to rebuild a missing `PILdos.ico` from `PILdos.png`).
- Linux: `pildos.desktop` in the application menu and on the Desktop folder,
  starting `PILdos.sh` in a terminal.
- Termux: also `~/.shortcuts/PIL DOS` for the Termux:Widget widget; the
  setup screen runs in a Termux session, so Termux-x11 is not needed.

`PILdos.vbs --python` and `sh PILdos.sh --python` still start the older
Python/Tk terminal (it needs Tk: `pkg install python-tkinter` in Termux,
`python3-tk` on Debian/Ubuntu), and `--rust-text` starts `pil --dos-text`.
