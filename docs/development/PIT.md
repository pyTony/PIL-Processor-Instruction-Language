# PIT channel 0 profile

Channel 0 now retains its programmed binary divisor and mode. Control words
select mode 2 (rate generator) or 3 (square wave), including aliases 6/7,
and low-only, high-only or low/high byte accesses. Count zero means 65,536;
count one is rejected for these modes. New counts load on the next input
clock. Mode 2 counts down by one, mode 3 by two with the documented odd-count
high/low half-cycle behavior. Input clock frequency is 1,193,182 Hz.

Counter latches retain a snapshot until the selected read sequence completes;
another latch request does not overwrite an unread snapshot. A new control
word holds the previous count and IRQ total until a complete count is written
(the bounded 8253-compatible programming profile used by legacy timing code).
Reads during programming therefore observe that held count. Byte read and
write phases are independent. Unsupported commands and invalid reload values
fail before device-state mutation.

A host-clock session samples local time once, then advances its PIT source
with monotonic host time. Counter phase and periodic IRQ edge totals share that
source. BIOS tick updates and INT 1Ah reads use those edges, so programming a
new divisor changes the BIOS tick rate too. IF, STI and SS interrupt shadows
prevent BIOS tick delivery. Without a selected host clock, the deterministic
session behavior remains: PIT input time is zero and BIOS reads advance whole
ticks. This is not a cycle-accurate CPU or PIC model.

Limits remain explicit: BCD, modes 0/1/4/5, channel 1, read-back/status commands,
GATE transitions and live count rewrites to a different divisor without a
new control word stop. A low/high rewrite of the currently active divisor
is accepted without changing its phase; GW-BASIC 3.23 uses this at startup.
Only periodic output edges are modeled; control-word-induced OUT transitions
and a complete PIC pending/in-service/masking model are not implemented.
The existing BIOS timer shortcut catches up elapsed ticks in one update;
it does not reproduce individual IRQ delivery while interrupts were masked.
A custom real-mode INT 1Ch handler runs through a synthetic BIOS IRQ0 wrapper
that saves EAX and DS, calls the hook, and returns with IRET. The handler can
chain to the default no-op INT 1Ch entry. The shortcut invokes the hook once
per delivered poll even if multiple edges elapsed; it is not a full PIC or
individual-edge model. Clock reads with a hooked INT 1Ch return the most
recently delivered tick instead of entering the hook mid-service. Custom
INT 08h handlers and timer hooks outside real mode still stop explicitly. Clock offset should be selected before execution,
as the terminal does.

With no game adapter, port 201h reads FFh and ignores writes. BIOS joystick
service INT 15h AH=84h reports unsupported (AH=86h, CF=1), as documented by
an AT-compatible BIOS without a joystick port. This is absence, not joystick
emulation.

TOPBENCH 0.38a was run directly with -l and a host clock. It reached its
continuous-score startup, then stopped at the explicit custom INT 08h IRQ0
handler boundary. No benchmark score or complete TOPBENCH pass is claimed.
E90Bh reload itself is verified through an original guest I/O fixture and
exact-clock device tests.

References inspected (implementation written independently):
- [Intel 8254 datasheet, modes 2/3, latch and programming rules](https://www.scs.stanford.edu/10wi-cs140/pintos/specs/8254.pdf)
- [TOPBENCH published source](https://github.com/MobyGamer/TOPBENCH), including its ZTIMER11 timing routines.

References for this increment:
- [Epson BIOS joystick service, no-port behavior](https://download.epson.de/pub/electronics-de/card-pc/general/dbios-e.pdf).
- [Bochs BIOS timer interrupt code](https://android.googlesource.com/platform/external/qemu-pc-bios/+/80e81e424e98b6646f3c7f3420f7bad4dc308a08/bochs/bios/rombios.c), including the registers it saves around INT 1Ch.
