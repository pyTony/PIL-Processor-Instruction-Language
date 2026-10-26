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
