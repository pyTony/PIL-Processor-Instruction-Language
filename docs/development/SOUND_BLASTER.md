# Sound Blaster digital audio (initial implementation)

PILdos emulates the **8-bit PCM DSP portion of a Sound Blaster 2.0**.
This is ISA hardware inside the shared DOS machine, independent of any guest
program. It is not an OPL2/AdLib/FM synthesizer or a full SB16 device.

## Guest-facing configuration

| Resource | Value |
| --- | --- |
| DSP base | 220h |
| DSP version | 2.01 (E1h command) |
| IRQ | 5 (mixer register 80h also allows IRQ7) |
| 8-bit DMA | channel 1, page register 83h |
| DOS environment | `BLASTER=A220 I5 D1 T3` |

The environment is inherited by EXEC children. IRQs use the master 8259,
including its masks, remapped vectors, in-service bits, and EOI. Only the DSP
latch is acknowledged through an IN from 22Eh. A DOS program must install an
IRQ handler, unmask its selected line and issue PIC EOI as on hardware.

The guest can reset the DSP by writing 1 then 0 to 226h; IN 22Ah receives
AAh, and IN 22Eh reports bit 7 while reply bytes are available. The write
status at 22Ch is ready. Supported DSP commands are: 10h direct DAC,
14h single-cycle 8-bit DMA, 1Ch auto-init DMA, 20h silent ADC sample,
40h time constant, 41h sample rate, 48h auto-init block length,
80h timed silence, D0h/D4h pause/resume, D1h/D3h speaker,
D8h speaker query, DAh exit auto-init, E0h identification, E1h version,
E4h/E8h test register, and F2h software IRQ.

The DMA model handles channel 1's address/count, page, mode, mask, byte
flip-flop and clear/reset ports, with 64 KiB page wrapping and terminal count.
The DMA source must be guest RAM, with an unmasked **memory-to-device**
channel-1 transfer mode. A DOS program can check explicit service-stop errors
for unsupported DSP commands or transfer types instead of silent corruption.
Auto-init mode reuses the programmed DMA count/address when the channel is
configured for automatic initialization.

The DSP queues at most eight PCM blocks. The Rust DOS console and PILdos
host window forward the blocks to the existing `dos_host_audio.rs` output
(Windows `winmm` waveOut, or `aplay`/`pacat` where installed).
Guest DMA/IRQs work even if no host output is available; extra host PCM
is dropped under sustained backlog rather than letting memory grow without
bound. `speaker_volume=0` mutes the host output. The default PC-speaker
volume is intentionally low, so the Sound Blaster sink applies a stronger
digital gain based on the same volume setting.

### Try it

For software supporting Sound Blaster 2.0, select **port 220h, IRQ 5,
DMA 1** in its sound setup. With Doom, select Sound Blaster for *sound effects*;
FM music is not implemented and should be disabled separately:

```powershell
cargo build --release --locked
.\\target\\release\\pil.exe --dos-console --window --speaker-volume 15 --mount games/doom --args '-nomusic' games/doom/DOOM.EXE
```

The DOS guest can also use `BLASTER` automatically. This is a compatibility
target, **not** a claim that Doom-with-sound has been validated in a real
game run. The example deliberately uses Rust's `--dos-console` for sound.
The separate Python `tools/dos_terminal.py` frontend currently runs a
session protocol that does not forward PCM to the host; it can exercise guest
DMA/IRQ logic but will not play it. The Rust-hosted `--pildos` path also
receives PCM.

## Current limits and next tests

- OPL2/OPL3, MIDI/UART, mixer volume, SB Pro stereo, 16-bit DMA,
  recording/ADC DMA and SB16 command sets are not modeled.
- DSP sampling rates are bounded to 4,000–44,100 Hz and 8-bit unsigned mono.
- The on-screen Python Tk/headless DOS terminal does not yet transport PCM
  from the Rust session protocol to a player. Rust's native host UI does.
- DMA completion is time-based using the session PIT/host clock; clockless
  probe runs use deterministic synthetic device ticks. Exact ISA bus cycles,
  DMA contention and audio/video synchronization are not modeled.
- Run `cargo test` on Windows/Linux, especially the device tests in
  `src/dos_soundblaster.rs`. Follow with a DOS program that performs
  reset/version, single-cycle DMA IRQ5/EOI, and auto-init DMA. Validate actual
  audio and continuous gameplay before describing any game as sound-enabled.

No copyrighted sound assets, BIOS ROMs or game programs are bundled here.
