# DOSBox-X / PIL same-binary CPU comparison

Measured on 29 October 2026. This is an original self-checking DOS COM smoke
test, not SingleStepTests, a hardware oracle or full 386/DOS conformance.

## Measured result

Update 30 October 2026: MUL, LOOP-family, RCL/RCR, DAA/DAS, PUSHA/POPA,
indirect JMP, LDS and ENTER cases bring the suite to 391. Ubuntu dosbox-x
2024.03.01 (headless, Linux CI) passes all 391 in both engines. The Windows measurement below is the earlier 328-case run.

The exact same 57,117-byte CPUTEST.COM passed all 328 cases in both engines.
Both outputs contain every expected case ID in order, no failures, and the
completion marker. Captured output matches after CR/LF normalization. PIL
also reaches DOS exit code zero.

- DOSBox-X 2026.10.01, commit b6abbd5, Visual Studio SDL2 64-bit.
- DOSBox-X configuration: `core=normal`, `cputype=386`, fixed 10000 cycles.
- PIL executable reports 0.35.0-alpha.1 (built from `dev` `8d012c5`).
- COM SHA256: `a8521fe22f60d0d886853756c785425b4ddfd76d622c415d5cb55e31e1a2d97f`.
- PIL executable SHA256: `4c5c807b437d160c46cd47ec1d80438c8f687cf68b5711d5c0ced0bd85fb6b5c`.
- DOSBox-X executable SHA256: `e248c42984a7c2b6814474914f4161acc6a22eb65553e5a051e106bb86c39874`.

The negative control deliberately changes the first expected ADD result.
Both engines then report exactly one failed case (001), 327 passes, and PIL
exits with code one (`com_sha256`: `cdb33e9c0457e3945a4e9e170630edee05fadadddcc68289c7cf59be523bdd67`). The comparison script returns failure as required.
The final normal run also verifies that neither executable nor the guest COM
changed between the start and end of the comparison.

## Coverage

| Forms | Cases |
|---|---:|
| ADD, SUB, CMP | 45 |
| ADC, SBB with carry/borrow zero and one | 60 |
| AND, OR, XOR, TEST | 36 |
| INC, DEC, NOT, NEG | 48 |
| MOV | 6 |
| Valid unsigned DIV | 15 |
| Shifts (SHL, SHR, SAR: CL=0, 1, CL=4, imm8=3) | 36 |
| Rotates (ROL, ROR: 1, CL=4, imm8=3) | 18 |
| XCHG (8/16/32-bit, checking both EAX and ECX) | 6 |
| Extensions & conversions (MOVZX, MOVSX, CBW/CWDE, CWD/CDQ) | 20 |
| Flag transfers & carry/direction (CLC, STC, CMC, separate STD & CLD, LAHF, SAHF) | 12 |
| Signed truncated IMUL (16/32-bit r,r/m and r,r/m,imm8) | 12 |
| Condition byte set (SETZ, SETNZ, SETB, SETL, SETG) & LEA (16/32-bit) | 14 |
| Unsigned MUL (8/16/32-bit, CF/OF from the upper half) | 12 |
| LOOP, LOOPE, LOOPNE (CX; LOOP with ECX via 67h) | 6 |
| RCL, RCR (1 with CF 0/1, CL=9, imm8=3) | 24 |
| DAA, DAS | 10 |
| PUSHA/POPA, PUSHAD/POPAD | 4 |
| Near indirect JMP (r16, SS:[BX], r32) | 3 |
| LDS (16/32-bit), ENTER n,0 with LEAVE (16/32-bit) | 4 |
| Total | 391 |

All operand-sized families exercise 8-, 16- and 32-bit widths (or 16/32-bit for
extensions, conversions, IMUL and LEA). Cases use fixed
accumulator/source registers and explicit boundary inputs. The COM checks
EAX, ECX, EDX, ESI and EDI, including untouched upper register bits and swapped
ECX on XCHG. It captures
FLAGS before comparisons (followed by `CLD` so `DF=1` cases never affect DOS
output) and checks independently computed defined/preserved
bits. `STD` (`DF: 0 -> 1`) and `CLD` (`DF: 1 -> 0`) are tested separately with
both cleared and set incoming arithmetic flags. Logical AF, multi-bit
shift/rotate OF, IMUL non-carry/overflow flags and
all undefined DIV arithmetic flags are excluded. EBX holds the flags snapshot;
this is not an all-register or arbitrary-addressing test.

## Interactive Debugger Case Manifest & Bisection

`tools/verify_dosbox_cpu.py` records each case's 1-based `index` and exact
guest addresses (`entry_eip`, `test_eip`, `check_eip`, `fail_eip`, `done_eip`)
in `cases.json`. Opening `cases.json` explicitly—or opening `CPUTEST.COM`
when its instruction bytes match the neighbouring `cases.json`—in
`tools/pil_debugger.py` populates the **Case** selector and synthesizes a
self-contained `<200`-byte monitor image for any selected case so you can jump
directly to `test_eip` (`BP @ Test`), step individual PIL tokens (`Step PIL F8`),
and inspect register/flag checks inside the 4 KB monitor. Unrelated binaries in
the same directory never trigger manifest discovery.

The program uses only INT21/AH09 for reporting and AH4C to exit. Test values
and expectations come from original Python reference calculations, not from
PIL descriptions or DOSBox output. Every case is executed even after a failure.

Excluded: divide-error delivery, undefined flag values, memory-operand forms,
protected/V86 state, BIOS/devices, cycle timing and whole applications. Agreement
between two emulators does not establish original 386 hardware correctness.

## Repeat

From the development repository, choose a new output directory for every run:

```powershell
python tools/verify_dosbox_cpu.py --engine target/release/pil-proof.exe --dosbox C:/DOSBox-X/dosbox-x.exe --out target/dosbox-cpu/run-001
```

The tool preserves the COM, case manifest, isolated DOSBox configuration,
both captures, executable hashes and result.json. It never changes an engine
binary or the installed DOSBox configuration. Existing output directories
are refused. No DOSBox download or installation is performed automatically.

For the negative control, add `--negative-control` and use another new directory.
It is expected to return a nonzero exit status and report case 001 as failed
in both engines.

References: [DOSBox-X CPU settings](https://dosbox-x.com/wiki/Guide%3ACPU-settings-in-DOSBox%E2%80%90X)
and [batch command-line options](https://dosbox-x.com/wiki/DOSBox%E2%80%90X%E2%80%99s-Command%E2%80%90Line-Options).
