# Executable test program and monitor profile — v0.7

Use `sh tools/debug.sh` (Termux/Linux) or `./tools/debug.ps1` (Windows) to
build and load programs/selftest.bin. To choose another raw binary, supply its
path to the script. The CLI equivalent is `pil-proof --repl --load PATH`.
`demo selftest` loads the same built-in image without a file. `load PATH`
replaces the current image only after the entire file is read and validated.

```
list
step
serial rx 41
run 1000
check
mem 0x500 8
regs
serial tx
```

The selftest stops with DX=600D on success or DX=DEAD at an inline failed
check. It waits at IN for RX if none is queued. `check` verifies the success
stop, four RAM signatures and successful serial output history. It stays valid
after `serial tx` drains bytes. `pil-proof --selftest` injects 41, executes,
checks the result and exits nonzero on failure. It is a smoke test for the
implemented pipeline, not a replacement for unit/generated-code comparisons.

Successful TX is echoed automatically after step/run as hex and escaped text,
without consuming the transport queue. Bytes 00 and control characters are
escaped. Debugger command input is separate: use `serial rx` for guest input.
Reset clears output history and queues, restores the image and keeps breakpoints.

## Image environment

Raw bytes are loaded at offset 0x100 in 4096 bytes of RAM. CS=0, DS=0x10,
SS=0x20, FS=0x30, ES=GS=0; caches have normal 64-KiB limits. Default operand
and address sizes are 16 bits. This monitor supplies the already-loaded caches;
it does not execute segment loads, privilege changes or descriptor tables.
Code is fetched from RAM and decoded at each step, so poke can change code.
Image boundaries and unsupported instructions yield monitor diagnostics with
source EIP preserved; they are not synthesized guest #UD/#PF exceptions.

Supported encodings:

- B0..BF MOV immediate to register, including byte high-register views.
- 88..8B MOV register/register or register/memory, 8/16/32-bit operands.
- 00/01 ADD register/register and memory destination.
- 10/11 ADC, 18/19 SBB, 20/21 AND, 28/29 SUB, 38/39 CMP: register/register.
- 70..7F short Jcc; 0F 90..9F SETcc with register destination and /0.
- 66 operand size, 67 address size, all six segment override prefixes.
- 16-bit ModR/M and 32-bit ModR/M/SIB, disp8 sign extension and disp16/32.
- E4 E9 / E6 E9: IN AL / OUT AL on monitor mock port 0xe9.
- F4 HLT: monitor stop at the instruction, without interrupt/privilege semantics.

One prefix of each supported group is accepted. Repeated group prefixes,
LOCK/REP, near Jcc and all other encodings stop with an unsupported diagnostic.
The debug port uses nonblocking transport with wait/retry; it is not an 8250
UART, and real hardware IN behaviour is not modeled by that wait convention.
No instruction cache, paging, A20 masking, interrupts, BIOS or OS loader exists.

`programs/selftest.lst` gives exact instruction addresses and source intent.
`python3 tools/build_selftest.py --check` confirms reproducible fixture bytes.
The helper is an original fixed-image builder, not a general assembler.

## Earlier predecoded examples and command reference

# Interactive debugger and mock serial

Start from the existing project:

```sh
cargo run -- --repl
```

The current debugger executes explicitly predecoded PIL demos. It does not
load x86 executables, disk images or Windows yet. The eventual product is a
PC emulator on TTGO ESP32; this is its execution/debugging foundation.

## Counter example

```text
list
regs
break 0x102
run
flags
step
continue 100
breaks
delete 0x102
reset
run
quit
```

The counter begins at EIP=0x100 with EAX=3 and EBX=1, subtracts once per
iteration and loops with JNE. It ends at the harness stop at 0x108.
That stop is a test/debugger operation, not an emulated x86 HLT instruction.

Breakpoints stop before execution. Continuing after a breakpoint skips that
specific stop once; looping back to it stops again. An explicit step ignores
the breakpoint at the current EIP. Reset retains the current demo's
breakpoints and clears serial queues. Selecting a demo resets state and
clears breakpoints.

## Serial echo example

```text
demo echo
run
serial status
serial rx 41
run
serial tx
reset
serial rx 00
run
serial tx
quit
```

The echo demo copies exactly one RX byte into AL, queues AL for TX, then
stops. Empty RX waits without advancing EIP. A full TX queue also waits and
can be retried after draining it. AL writes preserve EAX's other bits.
MOCK_RX and MOCK_TX are harness operations, not x86 IN/OUT or 16550 UART
register accesses. An empty queue and a binary zero byte are distinct.

RX and TX are separate bounded queues of 256 bytes in the REPL. The mock
supports configurable capacities for tests. Injecting a chunk is all-or-none.
serial tx drains transmitted bytes as hexadecimal. Debugger command text is
never automatically placed in guest RX. Guest bytes are never parsed as
debugger commands. Nothing touches a real serial port, WLAN or SD card yet.

## Commands

| Command | Behavior |
|---|---|
| help | Show commands |
| demo counter / demo echo | Load/reset a built-in predecoded demo |
| list | List loaded instructions and current EIP |
| step / s | Execute one instruction or report a wait/fault/stop |
| run [N] / continue [N] / c [N] | Run up to N completed steps; default 100, maximum 10,000 |
| regs / flags / eip | Inspect CPU state; ? marks undefined flag outputs |
| set eax VALUE, etc. | Set a 32-bit debugger register value |
| set eip VALUE | Change the guest offset; missing instructions report a harness stop |
| set eflags VALUE | Set concrete flags and clear diagnostic undefined markers |
| break ADDRESS / delete ADDRESS / breaks | Manage breakpoints at loaded instructions |
| reset | Restore current demo; retain breakpoints, clear serial queues |
| serial status | Queue sizes/capacities/readiness |
| serial rx HEX... | Inject bytes, e.g. 41 00 ff |
| serial tx | Drain transmitted bytes |
| serial clear | Clear both serial queues |
| quit / exit | Leave the debugger |

Values/addresses use decimal or 0x-prefixed hexadecimal. Serial byte tokens
use hexadecimal. Register names for set are case-insensitive; other command
keywords and demo names are lowercase. Unknown or malformed commands report
an error without partially applying an RX chunk or register write.

Runs are bounded so loops return control after a finite budget; this release
does not implement an asynchronous pause command while a run is executing.
The OS's Ctrl-C behavior may terminate the process. A transport-level stop
request and remote REPL framing can be added later.

## Tests and portability

Unit tests live in tests/unit/ and are included as unit-test modules so they
can exercise private core/debugger interfaces without exposing a premature
public API. Cargo does not treat these nested files as integration-test crates.
No tests/unit/main.rs is present.

The SerialTransport trait is the intended seam for future UART or WLAN
transports. This implementation uses std/VecDeque and the debugger uses host
stdio. Embedded compilation still requires separating the core and replacing
host facilities with bounded storage and board I/O.

## v0.6 memory bring-up

The host debugger now has 256 bytes of flat RAM and a predecoded memory demo:

```text
demo memory
list
mem 0x10 4
step
mem 0x10 4
step
regs
reset
protect 0x10 4
run
```

The first step stores ECX=0x12345678 at 0x10; memory should show 78 56 34 12.
The second step loads that value into EAX. The demo stops at 0x304. After reset,
protecting that range makes the first store stop with a RAM write fault at
EIP=0x300; neither CPU nor RAM is committed. This is not x86 #PF delivery.

`mem ADDRESS [COUNT]` displays 1..256 bytes (default 16). `poke ADDRESS HEX...`
parses and checks the whole 1..256-byte chunk before writing. `protect ADDRESS
COUNT` marks an in-bounds interval read-only. Reset restores the demo's initial
RAM and removes debugger-added protection; loading another demo also resets RAM.
The old counter and echo demos continue to work. No instruction bytes are
fetched from this RAM yet.


## Hello, world!

In the debugger:

```
demo hello
run 1000
serial tx
```

The guest sends `Hello, world!` followed by a newline. The console displays
that newline as `\n`. No RX input is needed. This is a 57-byte real guest
program using MOV AL, OUT 0xe9 and HLT, with 28 completed steps. The build also
writes `target/programs/hello.bin`; use `load target/programs/hello.bin` to open
it, or consult `programs/hello.lst` for instruction addresses. `check` applies
to the diagnostic selftest; Hello-world can be inspected directly via serial tx.
