#!/usr/bin/env python3
"""Check that the machine layer does not reach into the layers above it.

src/pc.rs and src/pc/ hold the PC (CPU, memory, devices). Code there may use
the CPU, memory and other machine modules of the crate, but never Dine's DOS
(crate::dos), the BIOS (crate::bios) or the front ends (crate::host): what
the firmware decides reaches the machine through the Firmware trait in
src/pc.rs. Comments and string literals are not checked.

--self-test runs the rule on built-in samples first.
"""
import argparse
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN = re.compile(
    r"\bcrate::(?:dos|bios|host)\b"
    r"|\bsuper::super::"
    r"|\bLoaded\b"
    r"|\bDos\b")
# The machine files' own parent is the crate root (pc.rs) or pc (pc/*.rs):
# names of DOS and BIOS modules reached through super:: are refused too.
UPPER = ("services", "vectors", "console", "files", "planar", "brk",
         "stoplog", "session", "int09", "int14", "aux_device", "memory_arena",
         "bios_disk", "boot", "xms", "mouse", "exec", "lfn", "nls")
SUPER = re.compile(r"\bsuper::(?:%s)\b" % "|".join(UPPER))
BARE = re.compile(r"(?<![:\w])(?:%s)::" % "|".join(UPPER))


def code_only(line):
    """The line without its // comment and string literal contents."""
    out, quote, i = [], False, 0
    while i < len(line):
        c = line[i]
        if quote:
            if c == "\\":
                i += 2
                continue
            if c == '"':
                quote = False
                out.append(c)
        elif c == '"':
            quote = True
            out.append(c)
        elif line.startswith("//", i):
            break
        else:
            out.append(c)
        i += 1
    return "".join(out)


def findings(text):
    for number, line in enumerate(text.splitlines(), 1):
        code = code_only(line)
        for pattern in (FORBIDDEN, SUPER, BARE):
            match = pattern.search(code)
            if match:
                yield number, match.group(0)
                break


def self_test():
    bad = ["use crate::dos::Loaded;", "super::super::load(x)",
           "fn f(p:&mut Loaded)", "services::timer_edge(p,now);",
           "super::planar::io(p)", "impl Firmware for Dos{}"]
    good = ['f.write_str("Loaded{..}")', "// crate::dos is above us",
            "use super::*;", "use crate::memory::Ram;",
            "F::deliver_irq(p,vector)?;", "ports::string_io(p)"]
    for line in bad:
        assert list(findings(line)), line
    for line in good:
        assert not list(findings(line)), line


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
    files = [ROOT / "src" / "pc.rs", *sorted((ROOT / "src" / "pc").glob("*.rs"))]
    failed = 0
    for path in files:
        for number, text in findings(path.read_text(encoding="utf-8")):
            print(f"{path.relative_to(ROOT).as_posix()}:{number}: machine code uses {text}")
            failed += 1
    if failed:
        print(f"{failed} reference(s) from the machine layer to DOS, BIOS or front ends")
        return 1
    print(f"machine layer: {len(files)} files, no references to DOS, BIOS or front ends")
    return 0


if __name__ == "__main__":
    sys.exit(main())
