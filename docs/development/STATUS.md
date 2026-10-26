# Development status

Development branch: dev. Delivery branch: main. Promote through reviewed PRs.
Current work: v0.7 effective addresses, segment contexts, binary monitor,
executable selftest, console serial echo and build/debug/delivery helpers.

Verified baseline v0.6: 65 Rust tests passed on Termux in 9.47 s. PR #1 merged
as 13ea986c53384cdfd9103cc346c9d32ebeed8881.

v0.7: 92 Rust tests supplied, hosted CI pending. Phone build/manual debugger
results have not been reported. No Rust toolchain in the authoring environment.
PowerShell scripts are included in Windows CI; interactive GUI/manual behaviour
is not implied by that.

Next: strengthen decoder coverage, general PC machine state and devices; keep
full paging, descriptor loading, exception delivery, interrupts, A20 and OS boot
explicitly outside the current implementation. The debug port is not a UART.
