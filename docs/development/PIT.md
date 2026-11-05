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
GATE transitions and live count rewrites without a new control word stop.
Only periodic output edges are modeled; control-word-induced OUT transitions
and a complete PIC pending/in-service/masking model are not implemented.
The existing BIOS timer shortcut catches up elapsed ticks in one update;
it does not reproduce individual IRQ delivery while interrupts were masked.
User INT 08h/1Ch handlers still stop rather than being silently skipped.
Clock offset should be selected before execution, as the terminal does.

TOPBENCH 0.38a was run directly with -l and a host clock. It reached its
continuous-score startup, then stopped at the explicit custom INT 08h IRQ0
handler boundary. No benchmark score or complete TOPBENCH pass is claimed.
E90Bh reload itself is verified through an original guest I/O fixture and
exact-clock device tests.

References inspected (implementation written independently):
- [Intel 8254 datasheet, modes 2/3, latch and programming rules](https://www.scs.stanford.edu/10wi-cs140/pintos/specs/8254.pdf)
- [TOPBENCH published source](https://github.com/MobyGamer/TOPBENCH), including its ZTIMER11 timing routines.
