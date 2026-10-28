# DOS interrupt-vector services — 28 October 2026

Continuation from dev 979f6e932d4917c5dd8824afee2862691264ff05 (v0.32).
The user selected AH=25/35 as the next FreeCom blocker.

INT 21/AH=25 stores DS:DX in the physical RAM IVT entry selected by AL.
The offset occupies the first word and the selector the second. The complete
four-byte store is checked before RAM changes. Handler addresses are pointer
values and need not name mapped or executable memory.

INT 21/AH=35 reads the same entry into ES:BX, preserving the upper half of EBX
and updating both visible ES selectors and its ordinary real-mode cache.
Both services preserve flags, undefined-flag metadata and other registers.
RAM faults leave all service state unchanged. Normal bridge retirement occurs
only after success. Initial vectors are zero in this shim; no ROM or kernel
handlers are supplied. Setting a vector does not implement interrupt delivery:
the existing host service dispatcher still handles supported INT calls.

Original regression tests cover every vector, endpoint layout, unmapped handler
pointers, upper register halves, flags, all four protected store bytes, truncated
RAM, direct guest IVT writes and bridge retirement through the next stop/exit.

Validation: Hosted Windows/Linux CI40621030314 passed319 Rust tests,185 full-state cases
(7,224,306 matching trace bytes), standalone runtime/native-package checks and
full256x256 Mandelbrot comparisons on both hosts.
No new FreeDOS-image, ARM or legacy hardware run.

CI: https://github.com/pyTony/pil_proof/actions/runs/40621030314
Code97bbe894302b6135df28e716af44a377f3a77654. Local Rust is unavailable;
independent Python arithmetic, flow, RAM and exchange checks passed locally.
Keep development on dev; main receives a separate strict runtime projection.

Behavioral references: DOS INT 21h Set Interrupt Vector (AH=25h) and Get
Interrupt Vector (AH=35h), Microsoft MS-DOS Encyclopedia, and Ralf Brown's
Interrupt List (INT 21/25 and INT 21/35). No third-party implementation copied.
