#!/usr/bin/env python3
"""Integration check of the program-independent Dine shared library.

Builds dine.dll / libdine.so (`cargo build --lib --features dine-library`),
loads it with ctypes in a child Python process and calls the C ABI
`dine_run(bytes, len, name, tail)` with DOS program bytes read at run time.
No executable is built for any program. Checks output, exit status, command
tails and the long-host-filename argument bridge (8.3 aliases).
"""
import math
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
FACTOR = ROOT / "programs" / "dos" / "FACTOR.COM"

# Runs in a child process so that the library's stdout is captured.
CHILD = r"""
import ctypes, sys
lib = ctypes.CDLL(sys.argv[1])
lib.dine_abi_version.restype = ctypes.c_uint32
assert lib.dine_abi_version() == 1
lib.dine_run.restype = ctypes.c_int32
lib.dine_run.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.c_char_p, ctypes.c_char_p]
if sys.argv[2] == "--null":
    sys.exit(lib.dine_run(None, 1, b"T.COM", b""))
guest = open(sys.argv[2], "rb").read()
status = lib.dine_run(guest, len(guest), sys.argv[3].encode(), sys.argv[4].encode("utf-8"))
sys.stdout.flush()
sys.exit(status)
"""


def library_path() -> Path:
    name = ("dine.dll" if os.name == "nt" else
            "libdine.dylib" if sys.platform == "darwin" else "libdine.so")
    return ROOT / "target" / "release" / name


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


def run(lib: Path, cwd: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run([sys.executable, "-I", "-c", CHILD, str(lib), *args],
                          cwd=cwd, capture_output=True, timeout=60)


def main() -> None:
    subprocess.run(["cargo", "build", "--release", "--locked", "--lib", "--features", "dine-library"],
                   cwd=ROOT, check=True, timeout=900)
    lib = library_path()
    assert lib.is_file(), lib
    with tempfile.TemporaryDirectory(prefix="verify-dine-") as temporary:
        root = Path(temporary)
        # The program bytes come from outside the working folder (mounted as C:).
        programs = root / "programs"
        work = root / "work"
        programs.mkdir()
        work.mkdir()
        factor = programs / "FACTOR.COM"
        factor.write_bytes(FACTOR.read_bytes())
        for argument, n in [("0", 0), ("5", 5), ("10", 10)]:
            result = run(lib, work, str(factor), "FACTOR.COM", argument)
            assert (result.returncode, result.stdout, result.stderr) == (0, expected(n), b""), (
                argument, result.returncode, result.stdout, result.stderr)
        result = run(lib, work, "--null")
        assert result.returncode == 1 and b"null argument" in result.stderr, result

        # A long host filename in the working folder reaches the guest as its
        # DOS 8.3 alias; the guest opens it with INT 21h AH=3Dh.
        opener = programs / "LONGOPEN.COM"
        opener.write_bytes(long_name_open_fixture())
        long_name = "this_is_my_long_name_pascal_code_in_x86_windows.pas"
        (work / long_name).write_text("program Hello; begin end.\n", encoding="utf-8")
        result = run(lib, work, str(opener), "LONGOPEN.COM", long_name)
        assert (result.returncode, result.stdout, result.stderr) == (0, b"FOUND", b""), result
        spaced = "My Pascal program with spaces.pas"
        (work / spaced).write_text("program Another; begin end.\n", encoding="utf-8")
        result = run(lib, work, str(opener), "LONGOPEN.COM", f'"{spaced}"')
        assert (result.returncode, result.stdout, result.stderr) == (0, b"FOUND", b""), result
        result = run(lib, work, str(opener), "LONGOPEN.COM", "NOSUCH.PAS")
        assert (result.returncode, result.stdout) == (1, b"MISSING"), result
    print(f"PASS: {lib.name} runs DOS program bytes through dine_run; "
          "output, exit status, command tails and long-name aliases verified")


if __name__ == "__main__":
    main()
