# Multi-disk installation media

Dine's persistent session can change installation disks without restarting the
installer. This uses the normal shared DOS filesystem, not a per-program engine.
Images are read-only; install to a mounted host folder such as drive D:.

## Launch from the native media browser

On Windows run **`PILmedia.cmd`** from the project root for the native software
catalog, with descriptions and program launchers. Select with Up/Down or a
mouse click, then press Enter. DOS starts only after a program is selected;
returning from it restores the catalog. Esc closes the catalog.
Closing the title-bar window also ends the catalog process. Windows allows
one catalog per login session; another launch focuses the existing browser.

Programs inherit the normal `pildos.conf` (or `PILDOS_CONFIG`) drives,
disk sets, country, FPU and console settings. Existing C:/D:/E: mappings
are preserved; `driveD` is supplied as D: only if D: is not configured.
Z: remains the session RAM drive. Selecting installation media explicitly
replaces only its source drive, A: or E:.

The root `.cmd` hands off to `PILmedia.vbs`, which hides the engine's
console so only the interactive browser window stays open. Double-click
`PILmedia.vbs` directly to avoid the temporary command window entirely.
The launcher runs an unchanged temporary copy of the generic engine and removes
that copy on exit, leaving Cargo's build executable free to be updated.
Use `PILbuild.cmd` at the project root to build with a launch lock. The
browser refuses new launches while that build runs. It also refuses a
runtime older than files under `src/`, `Cargo.toml` or `Cargo.lock`, including
after a failed manual Cargo build. Existing sessions continue using their
own engine copy and must be restarted to use an update. If a build is
interrupted, remove the empty `target/pil-build-in-progress` folder only
after confirming no build is running.

### Catalog file

`PILmedia.vbs` (and the "PIL Retro" desktop shortcut) reads the first of:

1. the file named by the `PIL_RETRO_CATALOG` environment variable;
2. `images/retro/catalog.tsv`;
3. `catalog.tsv` in a folder directly below `images/retro`, such as
   `images/retro/img/catalog.tsv` beside an extracted image.

Nothing in the repository writes this file: it lists your own software, so
you write it (images/ is git-ignored). A missing file, runtime or
`images/FD14LITE.img` is reported by name. UTF-8 text, one entry per line,
six fields separated by tabs (the sixth is reserved and may be left out); blank lines and lines starting with `#` are
skipped; at most 200 entries:

```text
# group	title	description	folder shown	DOS launcher	reserved (empty)
Tools	My editor	Full-screen text editor.	D:\APPS\EDIT	D:\MENUS\EDIT.BAT	
```

The launcher must be a `D:\...BAT` file; the browser runs it with
`COMMAND.COM /C`. To see a parse error with its line number, run the engine
in a terminal:

```powershell
target\release\pil.exe --dos-catalog images/FD14LITE.img::COMMAND.COM images/retro/catalog.tsv
```

```sh
target/release/pil --dos-catalog images/FD14LITE.img::COMMAND.COM images/retro/catalog.tsv
```

For installation images run **`PILimages.cmd`** from the project root. Its native file
chooser starts in `images/`. Select an image or an ordered `.lst` manifest;
the launcher starts the normal shared Rust runtime with a writable D:.
Selecting a numbered floppy detects same-prefix, same-width numbered images
beside it and inserts the first disk of that set. A single floppy uses A:.
Hard-disk media uses E:, leaving the shell image on C:.

The launcher uses `images/FD14LITE.img::COMMAND.COM` and the `driveD` target
folder. Pass media directly, or invoke the runtime with another shell image:

```powershell
.\PILimages.cmd images/compiler/disk01.img
target/release/pil.exe --dos-media images/shell.img::COMMAND.COM images/compiler/install.lst
```

The root launcher uses Rust directly, without Python or PowerShell.

Images up to 64 MiB use the runtime image reader. Larger hard-disk images
are extracted by installed 7-Zip into a new folder under `drives/media/`;
that folder is writable and persists after exit. The original image is
unchanged. This is a file-level host copy, not a boot or raw disk mount.
The launcher reports the folder for later direct mounting. It does not
automatically run installers, replace the DOS kernel or edit AUTOEXEC.BAT.

## Rust window controls

In the Rust DOS console, press **Ctrl+Shift+M** to open the host menu.
This shortcut is consumed by the host even when a DOS application owns the
mouse. **Shift+right-click** also opens it; plain right-click opens it when
the guest mouse is inactive. Plain mouse input still reaches mouse-enabled
DOS applications. The popup and native file/folder dialogs are host controls.

The Drive A:/B: submenus provide Mount image, Mount folder, Open disk set,
Next disk, Previous disk, Current disk and Unmount. D:/E: support mounting and
unmounting. Each drive menu also has **Mount clipboard path**, which accepts
a full host-folder or supported FAT .img/.ima file path. In Windows Explorer,
right-click the original file or folder and choose **Copy as path** (or use
Shift+right-click), then in the *running DOS window* select
**Ctrl+Shift+M → Drive E: → Mount clipboard path**. Quoted paths containing
spaces work. The native folder picker also has an editable path field.
This is the **Windows host path**, not the virtual DOS path `D:\RETRO\...`.

Clipboard Copy/Paste commands for DOS text are independent of mounting.
Media results and errors appear in the Rust window title. Dine pauses guest
execution while the host menu or a file dialog is open. Cancelling leaves
the inserted disk unchanged. The Retro catalog itself is not a running DOS
session and does not offer drive menus; configure `E=path/to/image.img`
in `pildos.conf` before starting the catalog if mounting must persist across
selected programs. Images are read-only and must be supported FAT media;
ZIP/ISO archives need extracting or another supported mounting mechanism.

**Images larger than 64 MiB:** the normal in-memory FAT reader refuses these to
bound RAM use. On Windows the host drive menu now automatically uses the
existing 7-Zip extraction route for oversized `.img`/`.ima` files. The image
remains unchanged; the extracted directory is placed under `drives/media/`
and mounted on the selected DOS drive. You need 7-Zip installed. The extraction
is not a raw disk mount and can take some time, depending on the image.

The `E=...` configuration entry **does not perform extraction**. For a
persistent E: mount, first extract the original image and then point the
configuration at its extracted host directory, e.g.:

```ini
E=drives/media/my-original-dos
```

You can also start the root `PILimages.cmd` launcher with the full original
image path. That launcher already extracts large images before mounting them
on E:, alongside the normal D: writable drive. Do not point the configuration
directly to a greater-than-64-MiB image.

With a loaded set, Ctrl+Shift+F1 advances A:, Ctrl+Shift+F2 advances B:;
Ctrl+Shift+F3/F4 select the previous disk on A:/B:. These shortcuts also work
in a host terminal that reports their modifiers. They never type a guest key.

The Python/Tk terminal offers the same disk-set operations under its Drives
menu, available with Shift+right-click or Ctrl+Shift+M.

## Define a disk set

Create a UTF-8 manifest, for example `media/compiler/install.lst`:

```text
# Disk order is explicit; paths are relative to this manifest.
disk01.img
disk02.img
disk03.img
```

Select it using Open disk set, or add this to a session configuration:

```ini
diskset_a=media/compiler/install.lst
D=driveD
```

The manifest path is relative to the configuration file. Do not also specify
an `A=` mount for the same drive. Disk one is inserted at startup. Blank
lines and lines starting with `#` or `;` are ignored. Sets contain 1-99
`.img`/`.ima` images, up to 64 KiB of manifest text. Spaces in paths are
supported, without quotation marks in the manifest.

Start a shell with that configuration, then run `A:\INSTALL` or the
installer supplied on disk one. Choose a writable destination on D: when
the installer asks. When it requests another disk, use the host menu and
then press the installer key to continue. Select Previous disk if it asks
for an earlier disk. End-of-set does not wrap around silently.

For a controller using `--dos-session`, the equivalent protocol commands are:

```text
diskset A media/compiler/install.lst
disk A
disk A next
disk A prev
disk A 3
```

Numbers are one-based. An ordinary successful `mount A ...` or `unmount A`
clears its disk set. Failed reads, invalid selections, missing media and
busy drives preserve the current disk and set position. Swaps are refused
while DOS has files open on that drive; close those files or let the
installer reach its disk prompt before swapping.

## Scope

This is file-level FAT12/FAT16 media access. It does not boot DOS, swap raw
BIOS sectors, emulate copy protection, or provide missing CPU/DOS services.
Some installers need additional runtime services before they can complete.
Read-only images cannot accept installer writes; use a host-folder copy if
a distribution requires writable source media. Installation, compiled output
and other writes to the target folder persist normally.
