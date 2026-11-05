# DOS profiling

Enable host profiling with a leading engine switch:

```powershell
cargo run --release --locked -- --profile profile.json --dos-probe --steps 100000 images/FD14LITE.img::COMMAND.COM
python tools/dos_terminal.py --profile profile.json --mount driveD --args "/K D:\AUTOEXEC.BAT" images/FD14LITE.img::COMMAND.COM
```

For the windowless launcher, pass `--profile profile.json` if it forwards
arguments; otherwise use PILdos.cmd or tools/dos_terminal.py directly.
The Python Session constructor also accepts `profile="profile.json"`.
Close the terminal normally to finish the report. A forced process kill cannot
write the final report. Existing output files are replaced when profiling starts.

The JSON format contains elapsed host nanoseconds, inclusive phase timings and
call counts, and attempts at each of the first 4096 distinct CS:IP pairs.
Further attempts at untracked addresses are counted separately. These are
execution attempts, including REP elements and service bridges, not retired
instruction counts. Address counts accumulate across slices and EXEC children;
programs reusing a CS:IP share a counter. Phase timings overlap: dos_run contains
decode_cache, pil_execute and dos_bios_service. Do not sum them. Overall elapsed
time includes loading, terminal waits and report setup; dos_run excludes waits
between slices. Profiling adds clock and bookkeeping overhead, so use an ordinary
run for speed comparisons. Disabled profiling creates no report or timestamps.

## Stop diagnostics

The DOS terminal appends stopped runs to `PILdos-stops.jsonl`, with UTC time,
program path, CS:IP, up to 15 bytes at that address, CPU registers, selectors,
flags, MSW, steps and the reason. Repeated snapshots of the same stop are
suppressed. Use `--stop-log PATH` to choose a different file. The byte sequence
is an instruction window, not a decoded instruction length. Older engines may
omit the extra diagnostic fields; update and rebuild the engine as well.

## Unassigned I/O writes

`python tools/dos_terminal.py --ignore-unassigned-out --mount driveD images/FD14LITE.img::COMMAND.COM` enables compatibility for unassigned OUT
ports. The engine accepts the switch before its command, in either order with
`--profile PATH`. Each ignored port is reported once per loaded machine to
stderr, including width, value and CS:IP. IN behavior is unchanged. Word OUT
checks both adjacent ports. The conventional-PC assignment catalog in
`dos_ports.rs` keeps known devices strict: DMA, PIC, PIT, keyboard, CMOS/A20,
coprocessor control, ATA/floppy, game port, Sound Blaster, parallel/serial and
video. It is not an exhaustive inventory of expansion-card addresses. A port
outside this catalog may still belong to hardware on a different machine.
Known-device error messages name the device. Existing explicitly supported
port handlers retain their own behavior.
