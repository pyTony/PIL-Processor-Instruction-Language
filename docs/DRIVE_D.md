# Drive D: (your own DOS programs)

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

    $env:PILDRIVE = "C:\Users\admin\PILdrive"      # PowerShell
    .\freedos.cmd

    PILDRIVE=$HOME/PILdrive                         # sh: pass it to --mount
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

`python tools/makeshortcut.py` makes a "PIL DOS" shortcut that opens the DOS
terminal with drive D: and `D:\AUTOEXEC.BAT`:

- Windows: `PIL DOS.lnk` on the Desktop, starting `PILdos.vbs` (needs Pillow
  to rebuild `PILdos.ico` from `PILdos.png`).
- Linux and Android with Termux-x11: `pildos.desktop` in the application menu
  and on the Desktop folder, starting `PILdos.sh`. In Termux it also writes
  `~/.shortcuts/PIL DOS` for the Termux:Widget widget, which starts Termux-x11
  first. The terminal needs Tk (`pkg install python-tkinter` in Termux,
  `python3-tk` on Debian/Ubuntu).
