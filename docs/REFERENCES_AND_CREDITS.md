# References and credits

The Rust/PIL implementation, monitor decoder, scripts and included guest program
were authored for this project with AI assistance. No external Cargo crates,
third-party emulator code, manual text, BIOS, OS image or external test dataset
is included. A project-wide licence for this code has not yet been selected.

The implementation was informed by Intel's 80386 Programmer's Reference Manual
(1986), consulted through university-hosted copies:

- [Manual contents](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/toc.htm)
- [2.5: operand selection and effective addresses](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s02_05.htm)
- [6.3: segment permissions, limits and expand-down bounds](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s06_03.htm)
- [17.1: operand/address sizes](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s17_01.htm)
- [17.2: prefixes, ModR/M and SIB](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/s17_02.htm)
- [Appendix C: status flags](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/appc.htm)
- [Appendix D: condition predicates](https://www.read.seas.harvard.edu/~kohler/class/aosref/i386/appd.htm)

Thanks to Intel's authors and the university hosts for these references, and
to the Rust and Termux projects for the build/run environment. Their toolchains
and source are not bundled. Reference credit does not grant redistribution
rights to the referenced material.

The guest test program is an original fixed 204-byte image. Port 0xe9 RX/TX
and HLT stop behaviour are monitor conventions. This release does not import
Tiny386, QEMU, TinyEMU, SingleStepTests, SeaBIOS or FMOPL code/data.

Detailed development provenance is retained on the repository's dev branch.

The Hello-world image is original project code using the same documented MOV,
OUT and HLT monitor profile. No additional source material was imported.

The factorial and counted-loop programs and timing commands are original
project work using the same documented instructions. Factorial results use
integer arithmetic; host timings use the Rust standard library. No new
third-party implementation or dataset is bundled.

The bounded decoder/PIL cache is original project code using Rust's standard
library. It introduces no new external dependency, instruction semantics or
third-party source material.

The alpha compiled guest runner and binary trace format are original project
work generated from the same PIL definitions. Rustc and the standard library
provide compilation and timing. Operational references for the shared checkout:
[Cargo Book](https://doc.rust-lang.org/cargo/reference/build-cache.html) and
[Microsoft WSL filesystems](https://learn.microsoft.com/en-us/windows/wsl/filesystems).
No third-party source or documentation text is imported.

The direct x86-64 baseline uses original register-only inline assembly.
[Rust Reference](https://doc.rust-lang.org/reference/inline-assembly.html) and
[Microsoft KB896458](https://learn.microsoft.com/en-us/troubleshoot/windows-client/application-management/x64-windows-not-support-16-bit-programs)
were consulted for inline-assembly constraints and Windows 16-bit limitations.
