# DOS COM/EXE import and instruction-gap probe

This probe loads headerless COM or DOS MZ/ZM EXE files and runs the implemented
PIL instruction subset. It reports the first blocker, source file offset, raw
bytes, register context and reached instruction frequencies. It can rank the
first missing CPU encodings across a collection of programs.

## First run: original hello-world programs

After `sh tools/build.sh` on Termux/Linux, create the supplied examples:

```sh
./target/debug/pil-proof --write-dos-hello com target/dos-hello.com
./target/debug/pil-proof --write-dos-hello exe target/dos-hello.exe
./target/debug/pil-proof --dos-probe target/dos-hello.com target/dos-hello.exe
```

PowerShell, after `.\tools\build.ps1`:

```powershell
.\target\debug\pil-proof.exe --write-dos-hello com target/dos-hello.com
.\target\debug\pil-proof.exe --write-dos-hello exe target/dos-hello.exe
.\target\debug\pil-proof.exe --dos-probe target/dos-hello.com target/dos-hello.exe
```

Expected: both produce `OUTPUT (escaped bytes): Hello, DOS!\r\n`, stop with
`Exit(0)`, and report `SUMMARY imported=2/2 exited=2`. Each executes three PIL
instructions and two bridged DOS calls. These are original tiny DOS executables,
not the existing monitor-port hello.bin. Creation refuses to overwrite files.
The EXE deliberately addresses its string relative to the initial PSP DS so it
can run before segment-register move instructions are implemented.

## Build profile and executable path

The default build helpers create debug binaries: `sh tools/build.sh` builds
`target/debug/pil-proof`; `.\tools\build.ps1` builds
`target/debug/pil-proof.exe`. They do not refresh an existing release binary.
Run the path reported by the build helper, as in the examples above.

For an optimized build, use `sh tools/build.sh --release` or
`.\tools\build.ps1 -Release`, then replace `debug` with `release` in the
executable paths. If you configured `CARGO_TARGET_DIR`, use that directory
instead of `target`.

If a DOS option prints the old `AX=0x8000; ... typed tokens` demonstration,
you ran an older executable that does not recognize those options. Select the
newly built debug binary or explicitly rebuild release; no files are generated
by that old fallback demonstration.

## Import your programs

```sh
./target/debug/pil-proof --dos-probe --steps 10000 program.com program.exe
```

Use quoted paths for filenames containing spaces. The optional step budget must
precede the filenames and applies to each program. Default:10,000; maximum:
1,000,000. Up to256 input files are accepted per report, each at most1MiB.
The diagnostic command returns a report even for import/execution blockers;
inspect the `STOP` and `SUMMARY` lines rather than treating its host exit status
as a DOS program success code.

At the debugger prompt, `dosprobe file.com 10000` runs the same isolated probe
without replacing or modifying the current monitor program. For paths with
spaces use the CLI; the REPL uses whitespace-separated arguments. The existing
`load` command still loads raw monitor images; use the DOS probe for DOS files.

## Interpreting the report

| Result | Meaning |
|---|---|
| `Exit(code)` | Reached a supported DOS termination call |
| `MissingEncoding` | Decoder stopped on an unsupported instruction/form/prefix |
| `Service` | Unsupported DOS/BIOS call or a bounded DOS-string service failure |
| `Fault` | Shared execution or fetch/segment/RAM check failed |
| `Boundary` | Instruction fetch left the imported code window or truncated an instruction |
| `Budget` | Per-program execution limit reached |
| `Monitor` | Reached HLT or monitor-port I/O, which are not DOS termination/services |
| `IMPORT ERROR` | File/header/relocation/allocation checks rejected the input |

A typical relocated EXE startup can now load AX and execute `8E D8`
(`MOV DS,AX`) in real mode. The regression fixture then reaches an intentionally
unsupported `62` opcode at file offset37. That is a CPU gap; an
unimplemented `INT21/AH30` DOS-version query is a separate service gap.
Raw bytes are shown for context and are not claimed to be a decoded length.
`file_offset` is the position in the original COM file or MZ load module plus
header; it is absent for PSP or other addresses outside the module. Stop-state
registers and CS:IP show where execution reached, including after prior writes.

The missing-encoding histogram counts programs blocked by each first opcode,
not an estimate of every missing instruction in those programs. The reached
instruction mix counts fully retired instructions and lists bridged DOS calls
separately. The per-program step budget includes REP element steps (and bridged
services), so large repeats cannot bypass it. PIL execution counts include each
element; completed counts include the REP instruction only after full retirement.
Separate REP element mix entries count committed elements; zero-count REP
retires without a data access. A later fault preserves earlier element progress. Unknown encodings are never skipped; code/data bytes are not scanned
as if all were instructions. Fix a common blocker and rerun the same corpus to
reveal the next one. Dynamic paths, inputs and loops affect the frequencies.

## Real-mode CS data access

Ordinary COM/MZ CS caches permit data reads and writes, including CS overrides
used by startup and unpacking code. Segment limits and RAM write protection
still apply. Modified instruction bytes invalidate the interpreter decode cache
on reuse. The bounded loaded-image fetch window remains: this does not yet let
unpacked code execute outside that window. Explicit protected Code caches keep
their read/write restrictions. Fixed compiled images retain changed-code guards.

## Loader and service contract

- The probe places a minimal PSP at segment1000h in zero-filled conventional
  memory belowA0000h. It provides INT20 at PSP:0, the allocated-end segment,
  an empty command tail and no environment block or real DOS memory arena.
- COM starts at CS:IP=1000:0100 with CS/DS/ES/SS=PSP and SP=FFFEh. A zero
  stack word is installed there, including overwriting the last two bytes of
  a maximum-size65,280-byte image as part of that initial stack convention.
- MZ imports validate page counts, header/module sizes, relocation tables,
  relocation targets, initial entry and stack, min/max allocations and overlays.
  Relocations add the load segment1010h with16-bit wrapping, in table order.
  CS/SS are relocated with16-bit wrapping; DS/ES point to the PSP.
  Wrapped relative segments such as FFF0:0100 are accepted when the resulting
  physical entry lies inside the load module. Zero min=max high-load files,
  overlays and recognized NE/LE/LX/PE/W3/W4 containers are rejected. Trailing
  bytes beyond the declared DOS image are reported and not loaded. Checksum
  validation is not required; the MZ checksum field is ignored.
- Initial general registers are zero except SP, flags are0202h, and FS/GS use
  the PSP cache as a probe convention. These are not universal DOS entry-state
  promises. SS:SP is conservatively required to fit the assigned memory block.
- Execution uses the concrete PIL token interpreter and its existing decoder
  and access checks. DOS probing is not currently a generated-Rust runner.
  COM fetches are bounded to PSP plus imported bytes; MZ fetches to the loaded
  initial-CS code window. A20 wrap and executable code outside those windows
  are not modeled. Stack/segment/control instructions remain limited to what
  the current instruction profile implements.
- Only unprefixed `INT21/AH09` (DS:DX dollar-terminated output),
  `INT21/AH4C` (exit with AL) and `INT20` (exit with CS=PSP) are bridged.
  Output is capped at16KiB and a service checks the whole string before
  publishing output. AH09 returns AL=24h and preserves other modeled state.
- These calls are an explicitly labeled host shim, not CPU INT/IRET semantics:
  no interrupt frame, IVT dispatch, BIOS, files, keyboard, environment, memory
  allocation calls, Ctrl-Break handling or full DOS process lifetime exists.
  Captured bytes are displayed escaped; no CP437 screen rendering is implied.

## Sources and verification

Original hello fixtures and tests are authored for this project; no external
DOS programs, emulator implementation or interrupt-list text is bundled.
Header/entry/relocation contracts were checked against Microsoft's published
[MS-DOS EXEC source](https://github.com/microsoft/MS-DOS/blob/main/v4.0/src/DOS/EXEC.ASM)
and [header definitions](https://github.com/microsoft/MS-DOS/blob/main/v2.0/source/DOSSYM.ASM),
plus the archived [MS-DOS3.3 Programmer's Reference](https://www.pcjs.org/documents/books/mspl13/msdos/dosref33/).
The small output/termination bridge follows the documented DOS API; AH09's
AL=24h behavior is recorded in [Ralf Brown's Interrupt List](https://delorie.com/djgpp/doc/rbinter/id/73/25.html).

Development checks include malformed and relocated MZ images, stack/allocation
bounds, hello execution, output limits, first-blocker reports, bounded loops,
and debugger isolation. `python tools/verify_dos_probe.py` additionally authors
its relocation fixture independently and checks actual CLI output. Runtime export
and native-package smoke checks run both COM and MZ hello files.

Verified on Windows/Linux CI40460972486 at source11dd1cae:200 tests per host,
independent DOS CLI fixtures and both COM/MZ hello examples in standalone source
and unpacked native packages passed. The existing49 backend/native CPU regression
suite also passed; it does not compare DOS-shim execution with generated Rust.

## Disk images and ZIP packages (0.20 alpha)

`--dos-probe` and REPL `dosprobe` also accept FAT16 `.img`/`.ima` and `.zip`
inputs. `--dos-list PATH` lists candidates; `--dos-extract PATH NEW_DIRECTORY`
extracts regular files and one package ZIP level without changing the source.
See [container guide](CONTAINERS.md) for FreeDOS commands, formats and bounds.
Container executables are deduplicated by full byte equality before probing.

## Bounded conventional-memory services (0.27 implementation)

INT21/AH48 allocates first-fit BX paragraphs and returns segment AX;AH49 frees
ES;AH4A resizes ES to BX paragraphs. CF is defined clear on success or set with
AX7 (corrupted MCB),8 (insufficient memory) or9 (invalid/unowned block). Insufficient-memory AH48
and insufficient-growth AH4A return the largest available size in BX. DOS2..6
resize failure still grows to the maximum contiguous size. Upper register
halves and other flags/markers are preserved. Free does not zero bytes;
adjacent free blocks coalesce during allocate/resize,not free. Zero paragraphs
are represented by an owned zero-data MCB. First-fit only; no UMB/strategy,
EXEC/TSR,environment allocator,shared processes or device-owned blocks.

The arena spans MCB0FFFh through conventional-memory end A000h. Each MCB uses
one paragraph with type M/Z,owner PSP (1000h) or0 and data size in paragraphs.
The loader writes a real initial chain and maintains trusted host metadata.
Before services,mirrored type/owner/size must match; direct guest chain edits
are rejected with DOS7 rather than silently overwritten. Names/reserved bytes
are not interpreted. A4096-block safety limit stops the service diagnostically.

Physical probe RAM is now always640KiB,independent of MZ initial allocation.
PSP:0002 retains the original load-time end,as in the FreeDOS handler;
allocation_end tracks the current owned PSP block separately. Successful4A
returns the block segment in AX. Allocation is DOS bookkeeping,not hardware memory protection:
freed/unallocated RAM remains addressable in real mode. Loaded instruction
fetch windows are unchanged,so allocating RAM does not make arbitrary data
executable. All MCB writes are prechecked before committing metadata or
register results; readonly header-write failures stop the bridge without partial state.

Reference audit: [FreeDOS INT21 handler](https://github.com/FDOS/kernel/blob/d6791add2043c9d7b584d840a8ffaf8829fd2bdc/kernel/inthndlr.c), especially4A PSP-size retention and AX=ES; [RBIL48](https://fd.lod.bz/rbil/interrup/dos_kernel/2148.html), [49](https://fd.lod.bz/rbil/interrup/dos_kernel/2149.html), [4A](https://fd.lod.bz/rbil/interrup/dos_kernel/214a.html) for the bounded DOS2..6 service profile.

## Real-mode far returns (0.29)

CB and CA imm16 now restore CS:IP and advance SP using shared PIL. Operand16/32
and SS size are independent;the selector uses the low16 bits of its stack slot.
A bad complete frame,new IP beyondFFFFh or RAM access fault commits no return
effects. Flags are preserved. Protected/V86 far returns and far calls/jumps are
not implemented. Fetch bounds now refer to the initial physical window across
CS changes;this permits aliases of imported code without permitting execution
of unpacked/copied code outside the original bounded window.

## DOS version compatibility profile (0.30)

The bounded host shim now accepts INT21/AH30. Its default declared compatibility
profile is6.22 (AX=1606h),initialized in the original PSP at offset0040h.
Guest edits to that word affect subsequent queries. AL=1 on entry selects
non-ROM version flags (BH=0);all other AL values select OEM FF. BL and CX are
zero;register high halves,other registers,EFLAGS and undefined markers are
preserved. A failed PSP read changes no return registers or output. This is
metadata for the shim,not the version of a booted kernel or a claim of complete
DOS6.22 functionality. Other unsupported DOS/BIOS services remain blockers.

The implementation was checked against the
[FreeDOS AH30 handler](https://github.com/FDOS/kernel/blob/d6791add2043c9d7b584d840a8ffaf8829fd2bdc/kernel/inthndlr.c)
for input AL,PSP version and serial fields. The implementation is original;
no kernel code is copied and the shim does not claim to be FreeDOS.

### Measured memory-PUSH progress (0.31)

The actual FD14LITE.img was probed locally on Windows with 100000 steps.
All sixteen FreeCom variants advanced from `FF 74 14` (memory PUSH /6) to
`FF 1E 1F 02` (far CALL /3), twelve bytes later. Imports remain 151/156
and exits remain six. The grouped `opcode ff` total is therefore unchanged;
compare per-program stop details, bytes and CS:IP to measure this progress.
The disk image and third-party programs are not bundled in the repository.

Measured image SHA256: `f539d456b792594bc3ca59d4e0f4c23d4f1fee73370c1390b2da245400718d36`.
