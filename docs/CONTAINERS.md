# FAT16 disk images and ZIP packages

PIL can read FAT16 `.img`/`.ima` files and `.zip` archives directly. These are
read-only host import tools feeding the existing DOS executable probe; they do
not mount a guest drive, boot FreeDOS or implement DOS filesystem calls.

## Commands

Build and use the same profile. Default Termux/Linux build:

```sh
sh tools/build.sh
./target/debug/pil-proof --dos-list FD14LITE.img
./target/debug/pil-proof --dos-probe --steps 10000 FD14LITE.img > target/freedos-report.txt
./target/debug/pil-proof --dos-extract FD14LITE.img freedos-files
```

`freedos-files` must not already exist. The image is never modified. For release
use `sh tools/build.sh --release` and `target/release/pil-proof`. PowerShell
uses `.\tools\build.ps1` and `.\target\debug\pil-proof.exe` (or `-Release`
and the release path).

A ZIP package works with the same commands:

```sh
./target/debug/pil-proof --dos-list CHOICE.ZIP
./target/debug/pil-proof --dos-probe CHOICE.ZIP
./target/debug/pil-proof --dos-extract CHOICE.ZIP choice-files
```

The REPL's isolated `dosprobe PATH [steps]` also accepts these container paths.
Paths containing spaces should be quoted on the CLI; REPL arguments remain
whitespace-separated. Containers are selected by `.img`, `.ima` or `.zip` suffix.

## What is listed, extracted and probed

- `--dos-list` prints total file count, executable candidates and their sizes
  and original paths. It does not execute programs.
- `--dos-extract` extracts all regular files, retaining their relative paths.
  Empty directory records are not retained. Archive expansion places package
  contents under `PACKAGE.ZIP.contents/`; original ZIP bytes are retained too.
- One nested ZIP package level is expanded. A FAT16 installer image can thus
  expose both installed tools and programs inside its package ZIPs. A ZIP's
  embedded ZIP files get the same single additional level; no unbounded recursion.
- Unsupported/corrupt nested packages remain as original ZIP files and produce
  `ZIP package not expanded` notes. A corrupt top-level ZIP fails the command.
- `--dos-probe` selects `.COM`/`.EXE` candidates from containers and removes
  byte-identical container duplicates before execution. An internal hash indexes
  candidates, but equality is confirmed with their full bytes. Explicitly named
  individual DOS files retain their existing behavior, including repeated files.
- Each virtual filename is shown as `image.img::original/path`. MZ/COM import,
  reached instruction counts and first blockers use the existing DOS probe.
  Windows/LE or other incompatible executables are reported as import errors;
  an `.EXE` filename does not establish DOS compatibility.

At most256 resulting probe inputs are accepted. If there are more, list/extract
and choose a smaller corpus. Each program has its own step budget. Counts rank
first observed blockers and reached instructions, not whole-program coverage.

## Supported format subset and limits

| Input | Supported | Not currently supported |
|---|---|---|
| Disk image | First FAT16 MBR partition (types04/06/0E), or FAT16 superfloppy;512-byte sectors;8.3 ASCII names | FAT12,FAT32,ISO,GPT,extended/logical partitions,long filenames,legacy non-ASCII short names |
| ZIP | Stored(method0) and DEFLATE(method8);fixed/dynamic/stored DEFLATE blocks;central-directory sizes;data descriptors;ASCII or flagged UTF8 names | ZIP64,multi-disk,encryption,other compression methods,legacy non-ASCII filenames,symlinks |

The implementation uses Rust standard-library code without new crates or host
Python/unzip/mount dependencies. ZIP checks compare local/central headers,
declared sizes and CRC32. FAT checks validate volume/partition spans, mirrored
FAT agreement, file sizes and cluster/directory chains. Unsupported layouts are
reported explicitly; this is not a claim of complete FAT or ZIP compatibility.

Bounds:64MiB input container,16MiB per file/chain,128MiB total expanded bytes,
8,192 regular files,32 FAT directory levels,and1,024 bytes per path. Probe
input files still obey the DOS loader's own size/format limits.

Extraction rejects absolute paths,`..`,Windows device/alias names,case-duplicate
file paths and file/directory conflicts. It requires a new directory and creates
new files only. I/O failure may leave a partial new directory for inspection;
it does not overwrite existing destination files. The source remains read-only.

## FreeDOS image used during development

The supplied `FD14LITE.img` is a32MiB MBR image with its FAT16 partition atLBA63.
A separate Python extraction found688 original files and65 package ZIPs,
208 executable occurrences and156 distinct byte streams:58 COM,93 DOS MZ and
5 MZ/LE executables. All package ZIPs used stored/deflate compression and their
CRCs passed. That inventory is separate from claiming these programs run on PIL.

Source image SHA256:
`f539d456b792594bc3ca59d4e0f4c23d4f1fee73370c1390b2da245400718d36`.

The supplied image, FreeDOS binaries and extracted third-party source packages
are not committed to the PIL repository. Their original licenses remain in the
separate extraction bundle. Project tests use original synthetic images/data.

## Validation and references

`python tools/verify_containers.py` independently creates FAT16 and ZIP inputs
with Python struct/zipfile, including streaming ZIP data descriptors; it checks
listing, probe deduplication, byte-identical extraction, unchanged inputs and
unsafe/overwrite rejection. Rust tests include malformed inputs and independent
zlib-generated stored/fixed/dynamic/multi-block DEFLATE vectors. Source and native
package smoke checks exercise ZIP listing/probing/extraction.

Format sources: [PKWARE ZIP APPNOTE](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT),
[RFC1951](https://www.rfc-editor.org/rfc/rfc1951.html),
[Microsoft partition type documentation](https://learn.microsoft.com/windows/win32/fileio/disk-partition-types),
and the BPB/directory/FAT structures observed in the supplied image. No third-party
codec/filesystem implementation or specification text is bundled.

Current Rust/CLI/hosted execution results for this increment are pending CI.
