# LINES.EXE hybrid acceptance

This is a **manual, program-supplied** acceptance test, not a bundled DOS
fixture. The original LINES.EXE supplied for the experiment is a 60,659-byte
DOS MZ program, with an `LZ91` marker for **LZEXE 0.91**. Its compressed
payload contains Borland BGI/EGA-VGA graphics strings. That is suggestive of
graphics dependencies, not proof of any particular video path.

### Why run interpreted first?

The LZEXE startup is a decompressor: first the DOS engine executes the packed
stub, then the guest transfers to runtime-unpacked memory. The old executable
bytes are still the program identity, but observed instruction bytes may come
from **newly written guest memory**. For hybrid compilation, such sites must
be guarded by live memory signatures; compiled blocks must fall back when
bytes or machine state differ. Fully static standalone DOS compilation is
**not** the right first test for this binary.

### Inspect the user-supplied binary

```powershell
python tools/verify_lines_hybrid.py .\DriveD\LINES.EXE --scan-only
```

Replace `DriveD\LINES.EXE` with the actual path. Nothing is uploaded to
GitHub and the `--scan-only` mode does not execute any DOS code.

### Profile, compile and compare

```powershell
cargo build --release --locked
python tools/verify_lines_hybrid.py .\DriveD\LINES.EXE `
  --engine .\target\release\pil.exe `
  --profile-steps 2000000 --sites 512 `
  --run-steps 100000 --slices 6 `
  --output-dir .\target\lines-hybrid
```

The helper runs the plain interpreter, builds the existing compiled-block
backend using `tools/build_dos_shadow.py --verify`, then runs that backend
through the same JSON DOS session. It compares CPU registers, flags, CS:IP,
screen, graphics buffers, palette and DOS output at the end of each bounded
slice. The JSON report stores hashes of large graphics/text buffers instead
of printing a flood of pixels:

`target/lines-hybrid/lines-comparison.json`

A clean comparison with `native_block_instructions > 0` is an initial hybrid
milestone, **not a claim that all LINES graphics work**. A clean comparison
with zero compiled instructions means interpretation fallback only. If both
engines stop at an unsupported DOS/BIOS or graphics service, the tool returns
exit code 2, records the reason, and we fix that **shared** service first.
Mismatched checkpoints return exit code 1.

For interactive programs, the bounded default run may stop at a BIOS key
wait. To profile and replay a small set of keys, provide a binary file of
ASCII bytes with `--keys file`. Use Enter (`0D`) when a keypress ends
a splash screen. The replay helper injects keys only at blocking BIOS waits;
polling-only input loops may require a different training scenario.

### Continue without another Rust compilation

Once the first acceptance run has published `target/lines-hybrid/manifest.json`,
reuse the **same** compiled backend for a longer identical interpreter/hybrid
checkpoint comparison. The helper validates the guest SHA-256, target host,
compiled engine SHA-256, name and previous guard-test success; it refuses
stale or mismatched artifacts.

```powershell
git pull --ff-only
python tools/verify_lines_hybrid.py .\\DriveD\\LINES\\LINES.EXE `
  --engine .\\target\\release\\pil.exe --reuse `
  --run-steps 500000 --slices 20
```

This tests up to 10 million additional step-budget units **without re-profiling
or rebuilding**. It may be computationally expensive, so inspect each
checkpoint before raising the budget again. The report includes
`graphics_observed` and the native block counters. The initial roughly
569-thousand-step run stayed in text mode without output; its matching native
blocks therefore prove startup/unpacking correctness only, not BGI display.

### Visual run after comparison

The hybrid executable is generated at:

`target/lines-hybrid/pil-lines.exe` (Windows)

Then run with the existing PILdos frontend:

```powershell
python tools/dos_terminal.py `
  --engine .\target\lines-hybrid\pil-lines.exe `
  --headless .\DriveD\LINES.EXE
```

`--headless` uses the terminal frontend, not the standalone native graphics
window. Remove `--headless` to use the Python/Tk graphics window if desired.
Both run the original guest program through the **hybrid block backend**,
not the separately compiled standalone binary. When adding mounts for
external BGI files, use a copy or a disposable directory because host-folder
DOS mounts can permit guest writes.

### Limitations

The executable may use BGI, custom video mode setup or direct hardware ports
not currently represented by the DOS services. An LZEXE unpacker may also
change code bytes or jump into a dynamically materialized region. This test
intentionally exposes these stops instead of silently claiming that a profile
success means full runtime correctness. Guest clock-dependent draws may
produce snapshot mismatches that need investigation.

No LINES.EXE binary or vendor library is committed with this test.


### Performance baseline and clock-aware training (experimental)

The LINES.EXE title sequence was manually timed on Windows 11 to the first
appearing game balls (approximately the same visible point):

| Runtime | Approximate elapsed time |
| --- | ---: |
| DOSBox-X, configured for 3000 cycles | 10.00 s |
| PILdos interpreter | 28.94 s |
| Existing early-profiled shadow | 28.78–28.96 s |

The old shadow reported 767,490 compiled **block** instructions in 95,456,824
guest steps, but did not report compiled single-instruction dispatches. Its
0.8% is *block coverage only*, not total native coverage. Manual exit timing
and DOSBox-X's cycle setting prevent treating this as a precise CPU benchmark.

The earlier shadow training kept the deterministic clock disabled. LINES waits
in a loop at `18F0:0AE4` for a memory byte to change and does not enter its
intro until the normal host-clock DOS service is enabled. Extend training
through the animated title and board, with a **disposable copy** of any
external game files so the DOS guest cannot modify the original folder:

```powershell
python tools/build_dos_shadow.py .\DriveD\Lines\LINES.EXE `
  --engine .\target\release\pil.exe `
  --profile-clock 7200 `
  --train-mount .\DriveD\Lines `
  --steps 120000000 --sites 2048 `
  --output-dir .\target\lines-hybrid-clock --verify
```

`--profile-clock` is the UTC offset **in seconds**; 7200 is UTC+02:00.
It is opt-in to preserve existing deterministic shadow training. The profile
uses the host wall clock, including guest IRQ0 interrupts. It is consequently
**not deterministic** across runs, and training for a fixed step budget is
not equivalent to playback to a fixed wall-clock timestamp. The training
copy is removed after profiling; do not point a manually invoked
`pil --emit-dos-backend --mount` at valuable original directories, because
host-mounted DOS files are writable. The LINES executable and vendor files
are not committed. This is a performance experiment, not a verified speedup.

After compilation, run the same original program and mount with the revised
backend and print native counters on exit:

```powershell
python tools/dos_terminal.py `
  --engine .\target\lines-hybrid-clock\pil-lines.exe `
  --mount .\DriveD\Lines --native-stats `
  .\DriveD\Lines\LINES.EXE
```

`native_block_instructions` measures compiled basic-block dispatch;
`native_single_instructions` measures compiled single-instruction dispatch.
The number of other steps also includes DOS/BIOS services and interrupt work,
so it must not be labelled pure interpreter fallback. Compare both engines
at the same visible game point and test five-ball gameplay, timer animation,
keyboard responsiveness, and exit before accepting a speed claim. A later
optimization can examine timer-safe native batching, but it must retain
instruction-level IRQ0 semantics or provide an equivalent scheduling guard.
