# Observed DOS code compiled through PIL and Rust

The experimental DOS compiler records instructions during an interpreted run,
then emits one Rust source file and builds it together with Dine. The native
service uses compiled blocks before looking up individual decoded instructions.
DOS/BIOS services, VGA controller accesses, keyboard input and the existing
graphics frontend remain shared with the interpreter.

This is a linked native service with interpreter fallback. It is not yet a
standalone translated application linked only to a separate DOS library, and
does not dynamically load a new binary into an already running process.

## Build and verify Mandelbrot

From the repository root, with Rust and Python installed:

```text
python tools/build_dos_shadow.py programs/dos/MANDVGA.COM --verify
```

The helper builds a profiling engine, runs the original COM file, writes the
observed Rust source, and builds it in one Cargo invocation. `--verify` runs
generated-code state and invalidation tests and then three complete Mandelbrot
comparisons. The source/output cache is:

```text
shadow/<target_os>-<target_cpu>/<guest_sha256>/
    backend.rs
    manifest.json
    pil-mandvga.exe     # Windows; pil-mandvga on Linux/Android
```

The manifest identifies the target OS, CPU, Rust target triple, compiler,
original guest hash and source/binary hashes. Rust source and binaries in
`shadow/` are local generated files excluded from Git. No DOS executable is
replaced. Compilation is native to the current host; cross-target builds are
not selected by this helper.

The profiling run has no host-folder mounts. It stops on exit, keyboard wait,
fault, unsupported service, or 120 one-million-step slices. It records only
actually executed PIL sites belonging to the requested executable, at most
65,536 distinct offsets. Data is not scanned as instructions. Code after an
unanswered key wait and branches not taken remain interpreter fallbacks. A
second encoding at an already recorded offset is not compiled in this version.

For an existing profiling engine, add `--engine target/release/pil.exe`
(or `target/release/pil` on Linux/Termux). This engine must have been rebuilt
from the same compiler sources. The generated source is a module for Dine;
it is compiled with the project, rather than by invoking rustc on it alone.

## Run with graphics

Use the exact native service path printed by the helper:

```text
python tools/dos_terminal.py --engine SHADOW_SERVICE --headless programs/dos/MANDVGA.COM
```

The existing frontend opens its graphics window when a graphics frame appears
and closes it on return to text mode. This change does not introduce a second
GUI implementation. Running through the JSON protocol verifies frame production
but does not itself visually inspect a desktop window.

To start COMMAND.COM instead, pass its image path and the usual mounts, then
run the matching program from DOS. The backend compares program identity when
EXEC changes the active executable; unrelated programs use the interpreter.
The direct Mandelbrot workflow is the verified path for this increment.

## Generated blocks and correctness boundaries

The writer joins consecutive observed instructions, currently at most 16 per
block, ending after a store or control-flow instruction. Instruction helper
functions are inlined so Rust can optimize across the operations in the block.
There is no interpreter token dispatch or decode-cache lookup inside a block.
Control returns to Dine at block boundaries. Native Rust loops spanning blocks
and subroutines are further work.

Entry validates the executable identity, segment/machine state, loaded fetch
windows and the entire block's code bytes. Entry at a later recorded offset
selects its corresponding sequence or falls back; it never jumps into the
middle of native machine code. A code write ends its block before another entry
can run. Changed bytes or machine context cause fallback. Paging, REP partial
execution, interrupt shadows and installed host-clock timer handlers retain
instruction-level execution in this version. Blocks fit wholly within the
remaining frontend step budget; faults retain the successfully retired prefix
and are delivered from the failing instruction.

The JSON service reports cumulative `native_blocks` and
`native_block_instructions`. These count multi-instruction blocks; generated
single-instruction fallbacks are not included in those counters.

## Reproduce the comparison

```text
python tools/verify_dos_shadow.py --native-engine SHADOW_SERVICE --samples 3
```

The checker compares all four VGA planes, palette, caption/screen, cursor,
reported CPU state, status and instruction count, then checks key exit and
text-mode restoration. The independent mathematical VGA oracle can also be run:
set `PIL_TEST_ENGINE` to the native service path and run
`python tools/verify_mandel_vga.py`.

Timing covers drawing to the keyboard wait and protocol frame transfer; process
startup, profiling, Rust compilation, and reference checking are excluded. It
includes the full DOS workload, rather than only the Mandelbrot arithmetic.
Do not compare it directly with the hand-written calculation-only reference or
run Cargo tests/builds concurrently with a performance measurement.

Results are written to `target/dos-shadow-comparison.json`. Performance depends
on the host and workload; correctness and actual block counts must be checked
before interpreting a speed ratio.
