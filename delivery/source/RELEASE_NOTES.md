# PIL 0.8.0

- Add real guest programs computing 1!..12! and a 65,536-iteration integer loop.
- Check every factorial RAM result and final loop registers/flags.
- Add time, bench and stress commands with bounded counts and fresh-state samples.
- Add release benchmark helper scripts for Termux/Linux and Windows.
- Include numeric binaries/listings in source builds and native deliveries.

Timing measures host interpreter throughput including decoding and PIL
compilation; it does not estimate original 386 clock cycles. Mandelbrot and
floating-point arithmetic are not included in this version.

# PIL 0.7.1

Add `demo hello`, a 57-byte guest Hello-world program with no input required.
Build scripts also write hello.bin. Successful serial output appears in the
debugger console and is available through serial tx.

# PIL 0.7.0

- Run an included 204-byte 386 guest program in the interactive debugger.
- Inspect registers, flags, RAM, instruction listings and breakpoints.
- See serial output directly in the console; inject input with serial rx.
- Use 16/32-bit effective addresses, segment overrides and segment bounds checks.
- Build/run with the supplied Termux/Linux and Windows scripts.
- Generate Rust source from embedded PIL descriptions with --emit-rust.

The program checks arithmetic/flags, conditional flow, register views, segmented
memory and mock serial. Expected success is DX=600D and TX=41. This verifies
the supported execution path; the debugger's instruction profile is limited.
See DEBUGGER.md for exact encodings and limitations.

This prototype does not boot an operating system or provide PC hardware yet.
