# Development status

Development branch: dev. Delivery branch: main. Promote through reviewed PRs.
Current work: v0.7 effective addresses, segment contexts, binary monitor,
executable selftest, console serial echo and build/debug/delivery helpers.

Verified baseline v0.6: 65 Rust tests passed on Termux in 9.47 s. PR #1 merged
as 13ea986c53384cdfd9103cc346c9d32ebeed8881.

v0.7 core commit 56f194fd031bed263b1b9f5ce5dc9b09f7abdb28 passed all 92
Rust tests on hosted Ubuntu (5.88 s) and Windows (11.42 s). The executable
selftest passed in 63 completed steps on both, as did the platform build/debug
scripts and native ZIP creation. CI run:
https://github.com/pyTony/pil_proof/actions/runs/40318741977

The final packaging step also verifies an unpacked archive's native selftest
and debugger launcher/serial transcript. Phone build/manual debugger results
have not been reported. No Rust toolchain in the authoring environment.
PowerShell scripts are included in Windows CI; interactive GUI/manual behaviour
is not implied by that.

Next: strengthen decoder coverage, general PC machine state and devices; keep
full paging, descriptor loading, exception delivery, interrupts, A20 and OS boot
explicitly outside the current implementation. The debug port is not a UART.
