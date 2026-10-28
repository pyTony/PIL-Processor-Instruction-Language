# User-supplied FreeDOS v0.33 report — 28 October 2026

Source: attached freedos-report-33.txt, SHA256
`cd643067b99c2f26655de04de9a8fa1b8489443546e29c16799c71d5081b689a`.
This is analysis of the user's run, not a new local execution. The report names
FD14LITE.img but does not print the executable version, image checksum or step
budget. The user subsequently confirmed100000 steps per program. This matches
the recorded v0.32 run budget; the separate10000-step report remains distinct.
Its filename identifies v0.33; exact binary/image identity is not independently
established. No third-party disk image or executables are added to the repository.

## Observed outcome

208 container candidates deduplicate to156 distinct executable files.
151 import;5 are rejected as non-DOS formats. Among imported programs:

| First stop | Programs |
|---|---:|
| Unsupported DOS/BIOS service |78|
| Fetch outside loaded image |54|
| Missing encoding |13|
| DOS process exit |6|

At the user-confirmed100000-step budget, no first-stop Budget or Fault appears. Every one of the54 Boundary stops is
reported as fetch outside loaded image; the report alone does not establish
whether copied/unpacked code or an invalid control target caused each stop.
Imports151/156 and exits6 remain unchanged from the recorded v0.32 results.
Reaching additional blockers is not proof of whole-program correctness.

## Progress and newly exposed blockers

All16 FreeCom variants, including COMMAND.COM, now stop at INT21/AH37 with
AX=3700 (query switch character), beyond their former AH25 blocker.
COMMAND.COM reaches CS:IP1010:2bcf, file offset11455, after13152 completed
instructions and5 DOS calls. No program stops first at AH25 or AH35.

GREP passes memory XCHG and reaches CS:IP2051:31a1, file offset82865, bytes
D1 DB D1 EB: D1 /3 is RCR BX,1, followed by SHR BX,1. The report counts1573
retired XCHG instructions across the corpus, including register forms.

| Missing encoding | Programs | Interpretation |
|---|---:|---|
|C4|6|LES pointer load; all shown forms load ES:DI|
|0F BB|2|BTC register-index form in CWSDPMI/CWSDPR0|
|60|2|PUSHA in VCPI and LBACACHE|
|0F 01|1|Operand-size-prefixed SMSW EAX in CPUSTAT|
|2F|1|DAS in COMTEST|
|D1 /3|1|RCR BX,1 in GREP|

These13 first encoding stops expose deeper paths; the increase over v0.32's
five first encoding stops does not itself indicate a regression.

## Service priorities

| Service | Programs |
|---|---:|
|INT21/AH02 (character output)|20|
|INT10/AH08 (read character/attribute)|17|
|INT21/AH37 (switch character query)|16|
|INT21/AH3E (close handle)|4|
|INT2F/AH43|3|
|INT21/AH44|3|

Other service stops: INT2F/AH11 and INT21/AH31/AH52 each2; INT21/AH71/AH51/
AH1A/AH58/AH5F, INT12/AH10, INT33/AH00, INT10/AH12 and INT1A/AH00 each1.
AH on vectors such as INT12 is an observed register value, not necessarily a
service selector.

Suggested next increments: bounded AH02 output and AH37 query contracts;
a text-screen model for INT10/AH08; LES (six first blockers), then the newly
identified RCR form for GREP. Keep memory execution-window work separate and
investigate the54 boundary stops before changing fetch bounds. Implementing
these targets requires new tests and validation; this report adds evidence only.
