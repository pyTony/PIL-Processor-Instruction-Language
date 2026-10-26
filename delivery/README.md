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
