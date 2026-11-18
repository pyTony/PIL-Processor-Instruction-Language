# Portable PIL native code generation benchmark

PIL's decoder, IR, optimizer and Rust emitter are independent of the native
compilation target. The same generated Rust source can be compiled on supported
Windows, Linux, Android/Termux and other Rust hosts. This benchmark compares
Rust/LLVM optimization levels **O0/O1/O2**, using a small fixed x86 counted-loop
proof generated through PIL's existing instruction code generator. It is not a
general DOS COM/EXE compiler.

## Generate and inspect portable Rust

Build the `pil` CLI using the platform's normal safe procedure. On
memory-constrained Termux, use:

```sh
sh tools/build_termux.sh --compact
```

On Windows/Linux with sufficient memory, `cargo build --locked --bin pil`
is another option. Then run:

```sh
python tools/bench_native_codegen.py --emit-only
```

The resulting Rust is at
`target/native-benchmarks/<host-triple>/loop.rs`. The source itself does not
depend on the selected target; directory separation prevents overwriting
artifacts from another host or cross-compile configuration.

## Benchmark on the host

```sh
python tools/bench_native_codegen.py --iterations 10000 --samples 5
```

The script compiles the **same source** at O0, O1 and O2, sequentially,
without LTO or debug info. On a native host each generated program executes
with assertions for registers, EFLAGS, undefined flags, guest memory, halt
status and instruction count. Results include compilation wall time,
unstripped executable bytes and native guest instruction throughput.

Outputs are stored under `target/native-benchmarks/<target-triple>/`:
`loop.rs`, `loop_O0[.exe]`, `loop_O1[.exe]`, `loop_O2[.exe]`,
`O*.compile.log`, `O*.run.log`, `report.csv`, `report.json`.

## Cross-compile or inspect assembly

```sh
python tools/bench_native_codegen.py --target aarch64-linux-android
python tools/bench_native_codegen.py --asm
```

The first command is an example: it requires the Rust target and appropriate
linker/system libraries installed. For any non-host target, the script
**only compiles**, reporting size/build time and leaving speed/verification
results null. The cross-compiled executable must be transferred and run on
its actual target. The optional `--asm` produces target assembly with an
additional compiler invocation at each optimization level.

The code generator is target-neutral, but the current generated proof uses
the Rust standard library and host-process facilities. It is **not yet**
`no_std` and is **not directly deployable on ESP32 firmware**. That will
require portable core/runtime separation, device-specific adapters, and an
appropriate embedded Rust target/toolchain. Do not fork PIL instruction
semantics per platform.

## Correctness and performance interpretation

These are fixed-image proof cases, not standalone DOS binaries. The
benchmark's internal assertions are useful correctness checks, but complete
step-by-step interpreter/native trace comparison remains in the existing
`--verify-backends` workflow.

Compare O0/O1/O2 only on the same hardware and with identical iteration
counts and sample counts. Guest instructions/s is a host throughput metric,
not an emulated 80386 clock frequency. Thermal throttling and noise can
affect small samples. Compiler speed and source-file size also matter.
