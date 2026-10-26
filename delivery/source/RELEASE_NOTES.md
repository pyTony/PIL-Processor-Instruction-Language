# PIL 0.12.0-alpha.1

- Add generic signed extension and modular multiplication PIL operations.
- Add signed IMUL16/32, 0F AF plus 69/6B immediate forms and memory sources.
- Define CF/OF overflow and undefined PF/AF/ZF/SF in the description.
- Expand compiled/interpreted verification to eighteen full-state cases.
- Test generic operation widths through64 bits and signed product edge cases.
- Document Windows PC comparison and separate Windows/WSL target directories.

# PIL 0.11.0-alpha.1

- Add OR, XOR, TEST, NOT, NEG, INC, DEC and NOP descriptions and binary decoding.
- Support both register/memory directions for all eight binary ALU operations.
- Add accumulator immediates and 80/81/83 immediate groups with sign extension.
- Add unary memory/register and TEST immediate forms at 8/16/32 bits.
- Expand compiled/interpreted verification to fourteen full-state cases.
- Preserve precise flag metadata and fault-before-retirement behavior.

# PIL 0.10.0-alpha.2

- Verifier failures identify case, step, instruction EIP and the differing CPU,
  stop metadata, RAM or serial field.
- Truncated or malformed traces fail verification; both trace files are retained
  on a mismatch under the reported artifacts directory.
- The decoder and execution engines have the same guest instruction coverage.

# PIL 0.10.0-alpha.1

First alpha candidate of the PIL host execution proof. See FEATURE_MATRIX.md
for implemented subsets and planned work; this release does not boot a PC OS.

- Add an offline compiled Rust guest runner generated from the same validated PIL.
- Compare complete instruction traces across nine success/fault/wait cases.
- Gate timings with --verify-backends, then compare generated Rust/interpreter.
- Add a direct x86-64 PC arithmetic baseline via --cpu-bench.
- Add source/native artifact output and Linux/Windows comparison helper scripts.
- Document shared Windows/WSL source folders with separate build directories.
- Preserve the cached interpreter as the interactive debugger engine.

Native comparison requires rustc. Normal packaged debugger use does not.
Compilation and process startup are excluded from native sample timings; guards
and black_box per step are included. Native speed on the phone is not yet known.
No JIT, arbitrary compiled binary execution, OS boot or ESP32 firmware is added.

# PIL 0.9.0

- Cache decoded binary instructions and their compiled PIL descriptions.
- Revalidate bytes and segment context before reuse, including guest code writes.
- Add cache stats/on/off/clear commands and uncached benchmark comparisons.
- Preserve precise stops, current memory permissions and runtime register values.
- Keep execution in the token interpreter; generated Rust remains a separate API.

Cached samples start with an empty cache. No simulated cycle model or JIT is
introduced. See DEBUGGER.md for timing scope and cache controls.

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
