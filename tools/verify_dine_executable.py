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



def long_name_open_fixture() -> bytes:
    """COM program: turn PSP command-tail CR into NUL and DOS-open that name.

    Print FOUND or MISSING via INT21/AH09, then exit with status 0 or 1.
    This tests the actual DOS file interface against the native long-name
    argument bridge, rather than simply inspecting an emitted command line.
    """
    code = bytearray()
    code += bytes.fromhex("bf8100")     # mov di,0081h
    code += bytes.fromhex("31c9")       # xor cx,cx
    code += bytes.fromhex("8a0e8000")   # mov cl,[0080h]
    code += bytes.fromhex("01cf")       # add di,cx; locate command-tail CR
    code += bytes.fromhex("c60500")     # mov byte [di],0
    code += bytes.fromhex("ba8200")     # mov dx,0082h (argv[0])
    code += bytes.fromhex("b8003dcd21") # mov ax,3d00h; int 21h (open read only)
    jc = len(code)
    code += bytes.fromhex("7200")       # jc missing (patch forward jump)
    # Success: close handle, print FOUND, exit 0.
    code += bytes.fromhex("89c3")       # mov bx,ax (file handle)
    code += bytes.fromhex("b43ecd21")   # mov ah,3eh; int 21h
    code += bytes.fromhex("ba0000")     # mov dx,success_string (patch)
    good_ptr = len(code) - 2
    code += bytes.fromhex("b409cd21b8004ccd21")
    missing = len(code)
    code += bytes.fromhex("ba0000")     # mov dx,missing_string (patch)
    bad_ptr = len(code) - 2
    code += bytes.fromhex("b409cd21b8014ccd21")
    good = len(code)
    code += b"FOUND$"
    bad = len(code)
    code += b"MISSING$"
    delta = missing - (jc + 2)
    assert -128 <= delta <= 127
    code[jc + 1] = delta & 255
    code[good_ptr:good_ptr + 2] = (0x100 + good).to_bytes(2, "little")
    code[bad_ptr:bad_ptr + 2] = (0x100 + bad).to_bytes(2, "little")
    return bytes(code)


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

        # A long Unicode-safe host filename is visible through a DOS 8.3
        # alias, without copying/renaming the host file. The program itself
        # opens the alias using the original DOS AH3D service.
        long_name = "this_is_my_long_name_pascal_code_in_x86_windows.pas"
        long_path = root / long_name
        long_path.write_text("program Hello; begin writeln('Long file'); end.\\n",
                             encoding="utf-8")
        guest_open = root / "LONGOPEN.COM"
        guest_open.write_bytes(long_name_open_fixture())
        subprocess.run(
            [sys.executable, str(ROOT / "tools" / "build_dine.py"),
             str(guest_open), "--output-dir", str(out)],
            cwd=ROOT, check=True, timeout=240,
        )
        guest_open.unlink()
        open_exe = out / ("Dine_LONGOPEN.exe" if os.name == "nt" else "Dine_LONGOPEN")
        result = subprocess.run([str(open_exe), long_name], cwd=root,
                                capture_output=True, timeout=30)
        assert (result.returncode, result.stdout, result.stderr) == (0, b"FOUND", b""), (
            result.returncode, result.stdout, result.stderr
        )
        assert long_path.is_file() and long_path.name == long_name
        # Verify quoting reaches the same path mapping, even with spaces.
        spaced = root / "My Pascal program with spaces.pas"
        spaced.write_text("program Another; begin end.\\n", encoding="utf-8")
        result = subprocess.run([str(open_exe), spaced.name], cwd=root,
                                capture_output=True, timeout=30)
        assert (result.returncode, result.stdout, result.stderr) == (0, b"FOUND", b""), (
            result.returncode, result.stdout, result.stderr
        )

        print(f"PASS: Dine_FACTOR standalone launch through shared {lib.name}; "
              "DOS guest embedded, command tail and exit verified")


if __name__ == "__main__":
    main()
