# DOS COM/EXE import and instruction-gap probe

This probe loads headerless COM or DOS MZ/ZM EXE files and runs the implemented
PIL instruction subset. It reports the first blocker, source file offset, raw
bytes, register context and reached instruction frequencies. It can rank the
first missing CPU encodings across a collection of programs.

## First run: original hello-world programs

After `sh tools/build.sh` on Termux/Linux, create the supplied examples:

```sh
./target/release/pil-proof --write-dos-hello com target/dos-hello.com
./target/release/pil-proof --write-dos-hello exe target/dos-hello.exe
./target/release/pil-proof --dos-probe target/dos-hello.com target/dos-hello.exe
```

PowerShell, after `.\tools\build.ps1`:

```powershell
.\target\release\pil-proof.exe --write-dos-hello com target/dos-hello.com
.\target\release\pil-proof.exe --write-dos-hello exe target/dos-hello.exe
.\target\release\pil-proof.exe --dos-probe target/dos-hello.com target/dos-hello.exe
```

Expected: both produce `OUTPUT (escaped bytes): Hello, DOS!\r\n`, stop with
`Exit(0)`, and report `SUMMARY imported=2/2 exited=2`. Each executes three PIL
instructions and two bridged DOS calls. These are original tiny DOS executables,
not the existing monitor-port hello.bin. Creation refuses to overwrite files.
The EXE deliberately addresses its string relative to the initial PSP DS so it
can run before segment-register move instructions are implemented.

## Import your programs

```sh
./target/release/pil-proof --dos-probe --steps 10000 program.com program.exe
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

A typical relocated EXE startup may load AX successfully and then stop on
`8E D8` (`MOV DS,AX`), which is not yet implemented. That is a CPU gap; an
unimplemented `INT21/AH30` DOS-version query is a separate service gap.
Raw bytes are shown for context and are not claimed to be a decoded length.
`file_offset` is the position in the original COM file or MZ load module plus
header; it is absent for PSP or other addresses outside the module. Stop-state
registers and CS:IP show where execution reached, including after prior writes.

The missing-encoding histogram counts programs blocked by each first opcode,
not an estimate of every missing instruction in those programs. The reached
instruction mix counts successful PIL instructions and lists bridged DOS calls
separately. Unknown encodings are never skipped; code/data bytes are not scanned
as if all were instructions. Fix a common blocker and rerun the same corpus to
reveal the next one. Dynamic paths, inputs and loops affect the frequencies.

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
  CS/SS are relocated; DS/ES point to the PSP. Zero min=max high-load files,
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
