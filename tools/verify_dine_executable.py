#!/usr/bin/env python3
"""Integration check: Dine shared library + tiny executable, no guest file at run.

Checks a real Rust build, DOS command tails, exit/output and missing-runtime
errors. It intentionally does not claim compiled-block or pixel parity.
"""
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
DOS = ROOT / "programs" / "dos" / "FACTOR.COM"


def expected(n: int) -> bytes:
    return "".join(f"{i:2d}! = {math.factorial(i)}\r\n" for i in range(n + 1)).encode("ascii")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="verify-dine-") as temporary:
        root = Path(temporary)
        guest = root / "FACTOR.COM"
        shutil.copyfile(DOS, guest)
        out = root / "bin"
        subprocess.run(
            [sys.executable, str(ROOT / "tools" / "build_dine.py"),
             str(guest), "--output-dir", str(out)],
            cwd=ROOT, check=True, timeout=240,
        )
        launcher = out / ("Dine_FACTOR.exe" if os.name == "nt" else "Dine_FACTOR")
        lib = out / ("dine.dll" if os.name == "nt" else
                     "libdine.dylib" if sys.platform == "darwin" else "libdine.so")
        assert launcher.is_file() and lib.is_file(), (launcher, lib)
        metadata = json.loads((out / (launcher.name + ".json")).read_text())
        assert metadata["guest"] == "FACTOR.COM"
        assert metadata["compiled_guest_blocks"] is False
        # Compiled guest bytes really are in the native application: do not
        # leave the original COM next to it and do not run from the output dir.
        guest.unlink()
        for argument, n in [("0", 0), ("5", 5), ("10", 10)]:
            result = subprocess.run([str(launcher), argument], cwd=root,
                                    capture_output=True, timeout=30)
            assert (result.returncode, result.stdout, result.stderr) == (0, expected(n), b""), (
                argument, result.returncode, result.stdout, result.stderr
            )
        # Runtime missing must fail clearly, not silently invoke another PIL.
        missing = lib.with_name(lib.name + ".hidden")
        lib.rename(missing)
        try:
            result = subprocess.run([str(launcher), "0"], cwd=root,
                                    capture_output=True, timeout=30)
            assert result.returncode != 0
            assert b"cannot load Dine runtime" in result.stderr, result.stderr
        finally:
            missing.rename(lib)
        print(f"PASS: Dine_FACTOR standalone launch through shared {lib.name}; "
              "DOS guest embedded, command tail and exit verified")


if __name__ == "__main__":
    main()
