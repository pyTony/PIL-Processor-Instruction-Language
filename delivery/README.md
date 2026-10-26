# PIL debugger delivery

Unpack this whole directory. No Rust or Python is needed for these native
binaries. Run the binary built for your operating system and CPU.

Linux: `sh debug.sh`
Windows PowerShell: `./debug.ps1`

The launcher opens programs/selftest.bin in the debugger. Type:

```
list
step
serial rx 41
run 1000
check
regs
serial tx
```

Success: `PASS`, `DX=600D`, RAM signatures and transmitted byte `41` (`A`).
Guest output also appears automatically as `serial output: 41 | A`.
`serial tx` drains the transport queue; `check` remains repeatable.

`reset` restarts the program and clears RAM changes and serial input/output.
`break 0x100`, `step`, `run`, `regs`, `flags`, `mem 0x500 8`, and `list` help
inspect execution. See DEBUGGER.md for commands and the monitor profile.

For an automatic check, run `./pil-proof --selftest` (Linux) or
`./pil-proof.exe --selftest` (Windows). Exit status zero means pass.

This is an early 386 monitor, not a bootable PC emulator. Only the documented
instruction subset is decoded. The IN/OUT debug port 0xe9 connects to a mock
serial queue; HLT is a monitor stop without interrupt or privilege semantics.
VGA, PS/2, SD, WLAN, BIOS, paging and OS loading are not present yet.

Built binaries target their build host. The Linux CI build is x86-64, not
Android/Termux or ESP32. Build from source in Termux for the phone.


## Hello, world!

In the debugger:

```
demo hello
run 1000
serial tx
```

The guest sends `Hello, world!` followed by a newline. The console displays
that newline as `\n`. No RX input is needed. This is a 57-byte real guest
program using MOV AL, OUT 0xe9 and HLT, with 28 completed steps. The archive
includes `programs/hello.bin`; use `load programs/hello.bin` to open it, or
consult `programs/hello.lst` for instruction addresses. `check` applies
to the diagnostic selftest; Hello-world can be inspected directly via serial tx.


## Numerical checks and loop timing

The factorial program uses only existing MOV/ADD/SUB/CMP/Jcc instructions.
It multiplies by repeated addition and stores all results 1!..12! as little-endian
dwords at linear RAM 0x500..0x52f. 12! is 479001600; 13! exceeds unsigned
32-bit range, so this example deliberately stops at 12. It executes 336 guest
instructions and halts at EIP=0x146.

```
demo factorials
time 1000
check
regs
mem 0x500 48
stress 100
```

`check` verifies all twelve RAM results, final registers and arithmetic flags.
`stress N` runs N fresh copies, checking every table; default 25, range 1..1000.
It leaves your current debugger state alone and reports total elapsed host time,
including setup and verification. A changed result makes the check fail.

For a tight loop:

```
demo loop
time 250000
check
bench 1000 3
```

The default guest counts 65,536 iterations with ADD/SUB/JNE: 196,611 completed
instructions including setup; halt at EIP=0x11a. `time N` runs the current
program for up to N steps (default 10,000; maximum 1,000,000), respects
breakpoints/waits/faults and reports elapsed milliseconds and instructions/s.
Use more `time` or `run` commands to continue a budget-limited run. HLT is a
monitor stop and is excluded from completed-instruction counts.

`bench ITERATIONS SAMPLES` runs a fresh counted-loop image per sample,
checks its answer, excludes one warmup and reports sample times, median and
instructions/s. Defaults are 1000 iterations and 3 samples; allowed ranges are
1..100000 and 1..9. Your loaded debugger program is preserved. Timing includes
decoding, PIL description compilation and interpreter dispatch. Image setup,
result checking and console output are outside the benchmark sample interval.
This is current interpreter throughput, not simulated 386 cycles or CPU MHz.

For representative timings, use an optimized build. From a source checkout:

```sh
sh tools/bench.sh 10000 5
# equivalent: cargo run --release --locked -- --bench 10000 5
```

Windows: `./tools/bench.ps1 -Iterations 10000 -Samples 5`.
The report identifies build profile, version and host OS/architecture. Compare
runs on the same host/profile; CPU scheduling and phone power state affect time.
`pil-proof --numeric-test` runs small automatic correctness checks.
`pil-proof --stress 100` runs the factorial stress test from the shell.

Source builds write factorials.bin and loop.bin in target/programs. Their
instruction listings are in programs/. Native archives include those binaries
under programs/ and can run the same debugger commands and --bench/--stress
options directly, without compiling.

A 256×256 Mandelbrot renderer is a later workload: signed fixed-point multiply,
shifts, framebuffer storage and image output are not implemented by this monitor.
