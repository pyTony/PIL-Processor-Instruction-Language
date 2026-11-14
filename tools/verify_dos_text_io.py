#!/usr/bin/env python3
"""Run original DOS fixtures through the Rust console (no GUI or user image)."""
import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def verify(engine, library=None):
    with tempfile.TemporaryDirectory(prefix="pil-text-") as directory:
        root = Path(directory)
        guest = root / "ECHO.COM"
        # Read/echo until CR, then return a distinct DOS status.
        guest.write_bytes(bytes.fromhex("b4 01 cd 21 3c 0d 75 f8 b8 07 4c cd 21"))
        config = root / "pildos.conf"
        config.write_text("program=ECHO.COM\nfpu=80387\nansi=true\nmouse=true\ncountry=358\n", encoding="utf-8")
        env = dict(os.environ, PILDOS_CONFIG=str(config), DINE_VIDEO="off", DINE_TEXT="auto")

        def run(args, data=b""):
            return subprocess.run([str(engine), "--dos-text", *args], input=data,
                                  capture_output=True, cwd=root, env=env, timeout=30)

        result = run([], "äbc\n".encode())
        assert result.returncode == 7, result.stderr
        assert result.stdout == "äbc\r".encode(), result.stdout
        assert b"\x1b" not in result.stdout
        if library:
            child = """
import ctypes, sys
lib = ctypes.CDLL(sys.argv[1])
lib.dine_run.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.c_char_p, ctypes.c_char_p]
lib.dine_run.restype = ctypes.c_int
data = open(sys.argv[2], 'rb').read()
sys.exit(lib.dine_run(data, len(data), b'ECHO.COM', b''))
"""
            result = subprocess.run([sys.executable, "-I", "-c", child, str(library), str(guest)],
                                    input="äbc\n".encode(), capture_output=True, cwd=root, env=env, timeout=30)
            assert (result.returncode, result.stdout, result.stderr) == (7, "äbc\r".encode(), b""), result
        result = run([], b"")
        assert result.returncode != 0 and b"EOF" in result.stderr, result.stderr
        result = run(["--fpu", "invalid"])
        assert result.returncode != 0 and b"FPU" in result.stderr, result.stderr
        # Guest escape/control bytes must never operate the host terminal when piped.
        code = bytearray()
        for byte in b"\x1b[31mX":
            code.extend([0xb2, byte, 0xb4, 2, 0xcd, 0x21])
        code.extend(bytes.fromhex("b8 00 4c cd 21"))
        guest.write_bytes(code)
        result = run(["--plain"])
        assert result.returncode == 0, result.stderr
        assert result.stdout == b"\\x1b[31mX", result.stdout
        env["DINE_TEXT"] = "terminal"
        result = run([])
        assert result.returncode != 0 and b"requires terminal" in result.stderr
        print("Rust DOS text I/O: configuration, CP437 input, exit status, EOF and safe redirected output passed")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, default=ROOT / "target" / "debug" / ("pil.exe" if os.name == "nt" else "pil"))
    parser.add_argument("--library", type=Path, help="also check the shared Dine C ABI with real redirected input")
    args = parser.parse_args()
    verify(args.engine.resolve(), args.library.resolve() if args.library else None)
