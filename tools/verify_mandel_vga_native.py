#!/usr/bin/env python3
"""Build original MANDVGA.COM standalone, compare all visible mode-12h pixels.

The reference is independent of PIL/Dine's implementation. A compiled binary
must reach the BIOS key wait, capture four planes and exit after its supplied
Enter key; no DOS interpreter or linked shadow engine may be used.
"""
import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
CAPTION = b"VGA 640x480 Mandelbrot  any key=exit"
BYTES_PER_PLANE = 80 * 480
STEP = 42


def colour(px, py):
    cx, cy = -9216 + STEP * px, -120 * STEP + STEP * py
    zr = zi = 0
    for k in range(30):
        if not -8192 < zr < 8192 or not -8192 < zi < 8192:
            return 1 + k % 15
        zr2, zi2 = (zr * zr) >> 12, (zi * zi) >> 12
        if zr2 + zi2 > 16384:
            return 1 + k % 15
        zr, zi = zr2 - zi2 + cx, ((zr * zi) >> 12) * 2 + cy
    return 0


def font():
    lines = [line for line in (ROOT / "src" / "dos_font8x8.txt").read_text(encoding="utf-8").splitlines()
             if not line.startswith(";")]
    return {int(lines[n * 9][:2], 16):
            [int(row.replace("#", "1").replace(".", "0"), 2)
             for row in lines[n * 9 + 1:n * 9 + 9]] for n in range(128)}


def verify(plane_bytes):
    assert len(plane_bytes) == 4 * BYTES_PER_PLANE, (
        f"native VGA capture has {len(plane_bytes)} bytes, expected {4 * BYTES_PER_PLANE}")
    planes = [plane_bytes[n * BYTES_PER_PLANE:(n + 1) * BYTES_PER_PLANE] for n in range(4)]

    def pixel(x, y):
        offset, bit = y * 80 + x // 8, 7 - x % 8
        return sum(((planes[n][offset] >> bit) & 1) << n for n in range(4))

    mismatches = []
    # The final row (lines 464-479) contains the caption, not the fractal.
    for py in range(232):
        row = [colour(px, py) for px in range(320)]
        for y in (2 * py, 2 * py + 1):
            for x in range(640):
                if pixel(x, y) != row[x // 2]:
                    mismatches.append((x, y, row[x // 2], pixel(x, y)))
                    if len(mismatches) == 5:
                        raise AssertionError(f"native VGA reference mismatch, first five {mismatches}")
    glyphs = font()
    for col, char in enumerate(CAPTION):
        for r in range(16):
            for bit in range(8):
                expected = 7 if glyphs[char][r // 2] & (0x80 >> bit) else 0
                actual = pixel(col * 8 + bit, 464 + r)
                assert actual == expected, (
                    f"caption at col {col}, row {r}, bit {bit}: "
                    f"native {actual}, expected {expected}")
    print(f"PASS: {640 * 464} reference pixels, four VGA planes, "
          f"{len(CAPTION)} caption glyphs")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    suffix = ".exe" if os.name == "nt" else ""
    parser.add_argument("--engine", type=Path,
                        default=ROOT / "target" / "release" / ("pil" + suffix))
    parser.add_argument("--guest", type=Path,
                        default=ROOT / "programs" / "dos" / "MANDVGA.COM")
    parser.add_argument("--timeout", type=float, default=120)
    args = parser.parse_args()
    engine = args.engine.resolve()
    guest = args.guest.resolve()
    if not engine.is_file():
        parser.error(f"build the PIL release engine first: {engine}")
    if not guest.is_file():
        parser.error(f"original MANDVGA.COM not found: {guest}")

    with tempfile.TemporaryDirectory(prefix="native-mandvga-") as temp:
        outdir = Path(temp)
        subprocess.run([sys.executable, str(ROOT / "tools" / "build_dos_native.py"),
                        str(guest), "--engine", str(engine), "--output-dir", str(outdir)],
                       cwd=ROOT, check=True)
        executable = outdir / (guest.name + (".exe" if os.name == "nt" else ".native"))
        capture = outdir / "vga-planes.bin"
        env = os.environ.copy()
        env["PIL_DOS_BIOS_KEY"] = "1C0D"
        env["PIL_DOS_VGA_CAPTURE"] = str(capture)
        env.pop("PIL_DOS_RAW_OUTPUT", None)
        started = time.perf_counter()
        result = subprocess.run([str(executable)], cwd=ROOT, env=env,
                                capture_output=True, timeout=args.timeout, check=False)
        elapsed = time.perf_counter() - started
        assert result.returncode == 0, (
            f"native MANDVGA exit {result.returncode}, stdout={result.stdout!r}, "
            f"stderr={result.stderr.decode(errors='replace')!r}")
        assert result.stdout == CAPTION, f"native caption output {result.stdout!r}"
        assert capture.is_file(), "native binary did not capture the VGA planes at the key wait"
        verify(capture.read_bytes())
        print(f"PASS: standalone native MANDVGA exit code 0, captured before Enter, "
              f"host execution {elapsed:.3f}s (not a Dine-comparable benchmark)")


if __name__ == "__main__":
    main()
