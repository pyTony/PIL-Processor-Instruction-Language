# Debugger guide

Start with `sh tools/debug.sh` (Termux/Linux) or `./tools/debug.ps1` (Windows).
The script builds and loads the supplied selftest binary. `demo selftest` loads
the same program without a file. `load PATH` opens another raw binary.

## First session

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

Success stops at HLT with DX=600D. A failed guest check stops with DX=DEAD.
`check` also verifies four RAM signatures and successful TX=41. The program
waits at IN if no input is queued. `--selftest` injects 41, runs and checks the
result automatically, returning nonzero on failure.

Successful guest output is shown after step/run as hex and escaped text:
`serial output: 41 | A`. This does not drain TX. `serial tx` retrieves queued
bytes; draining them does not invalidate `check`. Control bytes are escaped.

## Commands

| Command | Purpose |
|---|---|
| help | List commands |
| demo selftest / counter / echo / memory | Load a built-in program or predecoded example |
| load PATH | Load a raw binary, 1..3840 bytes; paths contain no spaces in the REPL |
| list | Decode up to 16 instructions from EIP, or list a predecoded example |
| step / s | Execute one instruction, ignoring its breakpoint |
| run [N] / continue [N] / c [N] | Run up to N completed steps; default 100, maximum 10,000 |
| regs / flags / eip | Inspect state; ? marks undefined flag results |
| set eax VALUE, etc. | Set a 32-bit general register, eip or eflags |
| break ADDRESS / delete ADDRESS / breaks | Set/remove/list breakpoints |
| reset | Restore initial CPU/RAM, clear serial queues/history; retain breakpoints |
| mem ADDRESS [COUNT] | Inspect 1..256 bytes, default 16 |
| poke ADDRESS HEX... | Write 1..256 bytes after checking the entire chunk |
| protect ADDRESS COUNT | Make a RAM interval read-only until reset |
| serial status | Show RX/TX queue state |
| serial rx HEX... | Queue guest input, for example 41 00 ff |
| serial tx | Drain output bytes as hex |
| serial clear | Clear RX and TX queues |
| quit / exit | Leave the debugger |

Numbers use decimal or 0x hexadecimal; serial/poke byte tokens are hexadecimal.
Register names for set are case-insensitive; commands and demo names are lowercase.
Continue skips the current breakpoint once, then stops if execution returns to
it. Binary breakpoints may use any offset within the loaded image; choose an
instruction boundary from the listing. Runs have a finite budget; Ctrl-C may
terminate the process rather than pause it.

The counter example subtracts EBX from EAX until zero. Echo waits for one byte
and returns it through mock serial. Memory stores/loads ECX at RAM 0x10;
`protect 0x10 4` demonstrates a restartable store fault. These three examples
use predecoded harness operations. The selftest fetches actual instruction bytes.

## Binary monitor profile

The image loads at offset 0x100 in 4096 bytes of RAM. CS=0, DS=0x10, SS=0x20,
FS=0x30, ES=GS=0, with ordinary 64-KiB segment limits. Default operand and
address sizes are 16 bits. Data accesses can use 32-bit overrides. Code is
fetched from RAM at each step, so poke can modify it. The included listing
`programs/selftest.lst` shows addresses and instruction intent.

Supported encodings:

- B0..BF MOV immediate to register, including high-byte register views.
- 88..8B MOV register/register or register/memory, with 8/16/32-bit data.
- 00/01 ADD register/register or memory destination.
- 10/11 ADC, 18/19 SBB, 20/21 AND, 28/29 SUB, 38/39 CMP: register/register.
- 70..7F short Jcc; 0F 90..9F SETcc with register destination and /0.
- 66 operand size, 67 address size, all six segment overrides.
- 16-bit ModR/M, 32-bit ModR/M/SIB, disp8 sign extension and disp16/32.
- E4 E9 / E6 E9: IN AL / OUT AL using mock serial on monitor port 0xe9.
- F4 HLT: monitor stop at the instruction.

One prefix per supported group is accepted. Repeated group prefixes, LOCK/REP,
near Jcc and other encodings stop with a diagnostic and unchanged source EIP.
The loaded-image boundary is a monitor boundary, not an architectural exception.
Segment bounds and permissions are checked before RAM accesses; CPU and the
single staged RAM store remain unchanged on a later fault. #SS(0)/#GP(0)
identify segment bounds. RAM bounds/read-only errors are emulator diagnostics,
not guest paging faults.

This is not a bootable PC emulator. Port 0xe9 is not an 8250 UART: empty RX or
full TX waits and retries the instruction. HLT does not model interrupts or
privilege checks. Loaded segment caches are supplied by the monitor; descriptor
loading, mode changes, paging, A20, BIOS, interrupts and OS boot are absent.
VGA, PS/2, SD and WLAN are not implemented in this release.


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
