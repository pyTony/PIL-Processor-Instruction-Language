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
