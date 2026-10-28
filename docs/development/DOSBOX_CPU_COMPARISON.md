# DOSBox-X / PIL same-binary CPU comparison

Measured on 29 October 2026. This is an original self-checking DOS COM smoke
test, not SingleStepTests, a hardware oracle or full 386/DOS conformance.

## Measured result

The exact same 36,340-byte CPUTEST.COM passed all 210 cases in both engines.
Both outputs contain every expected case ID in order, no failures, and the
completion marker. Captured output matches after CR/LF normalization. PIL
also reaches DOS exit code zero.

- DOSBox-X 2026.10.01, commit b6abbd5, Visual Studio SDL2 64-bit.
- DOSBox-X configuration: `core=normal`, `cputype=386`, fixed 10000 cycles.
- PIL executable reports 0.35.0-alpha.1. It came from the user's working tree;
  no exact source-commit provenance is asserted for that executable.
- COM SHA256: `e548b4c22f9f02137688a171d66edbc0ceaf143170b88d1f8db63d4469cee3ec`.
- PIL executable SHA256: `d2bf3c71db0e2062410f4d341de1414043397fb80634946d045b3699be9c78ad`.
- DOSBox-X executable SHA256: `e248c42984a7c2b6814474914f4161acc6a22eb65553e5a051e106bb86c39874`.

The negative control deliberately changes the first expected ADD result.
Both engines then report exactly one failed case (001), 209 passes, and PIL
exits with code one. The comparison script returns failure as required.
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
| Total | 210 |

All families exercise 8-, 16- and 32-bit operand sizes. Cases use fixed
accumulator/source registers and explicit boundary inputs. The COM checks
EAX, ECX, EDX, ESI and EDI, including untouched upper register bits. It captures
FLAGS before comparisons and checks independently computed defined/preserved
bits. Logical AF and all undefined DIV arithmetic flags are excluded. The
direction flag is checked as preserved clear. EBX holds the flags snapshot;
this is not an all-register or arbitrary-addressing test.

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
