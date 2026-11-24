#!/usr/bin/env python3
"""Check that the machine and BIOS layers do not reach into the layers above them.

crates/pil-pc holds the PC (CPU, memory, devices): it may use pil-cpu and its
own modules, never BIOS, DOS or front ends.
crates/pil-bios holds the AT/VGA BIOS firmware: it may use pil-cpu, pil-pc and
its own modules, never Dine's DOS or the front ends.
Since both are separate crates, rustc refuses upward dependencies already; this
check also catches forbidden identifiers in code before a build. Comments and
string literals are not checked.

--self-test runs the rules on built-in samples first.
"""
import argparse
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]

PC_FORBIDDEN = re.compile(
    r"\bcrate::(?:dos|bios|host)\b"
    r"|\bpil_bios\b"
    r"|\bsuper::super::"
    r"|\bLoaded\b"
    r"|\bDos\b")
PC_UPPER = ("services", "vectors", "console", "files", "planar", "brk",
            "stoplog", "session", "int09", "int14", "aux_device", "memory_arena",
            "bios_disk", "boot", "xms", "mouse", "exec", "lfn", "nls", "ansi")
PC_SUPER = re.compile(r"\bsuper::(?:%s)\b" % "|".join(PC_UPPER))
PC_BARE = re.compile(r"(?<![:\w])(?:%s)::" % "|".join(PC_UPPER))

BIOS_FORBIDDEN = re.compile(
    r"\bcrate::(?:dos|host)\b"
    r"|\bdine\b"
    r"|\bLoaded\b"
    r"|\bDos\b"
    r"|\.\bdos\b")
DOS_ONLY = ("files", "brk", "stoplog", "session", "aux_device", "memory_arena",
            "xms", "mouse", "exec", "lfn", "nls", "ansi", "folder_disk", "guest_keymap")
BIOS_SUPER = re.compile(r"\bsuper::(?:%s)\b" % "|".join(DOS_ONLY))
BIOS_BARE = re.compile(r"(?<![:\w])(?:%s)::" % "|".join(DOS_ONLY))


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


def findings(text, patterns):
    for number, line in enumerate(text.splitlines(), 1):
        code = code_only(line)
        for pattern in patterns:
            match = pattern.search(code)
            if match:
                yield number, match.group(0)
                break


def self_test():
    pc_patterns = (PC_FORBIDDEN, PC_SUPER, PC_BARE)
    bios_patterns = (BIOS_FORBIDDEN, BIOS_SUPER, BIOS_BARE)
    bad_pc = ["use crate::dos::Loaded;", "super::super::load(x)",
              "fn f(p:&mut Loaded)", "services::timer_edge(p,now);",
              "super::planar::io(p)", "impl Firmware for Dos{}"]
    good_pc = ['f.write_str("Loaded{..}")', "// crate::dos is above us",
               "use super::*;", "use crate::memory::Ram;",
               "F::deliver_irq(p,vector)?;", "ports::string_io(p)"]
    for line in bad_pc:
        assert list(findings(line, pc_patterns)), line
    for line in good_pc:
        assert not list(findings(line, pc_patterns)), line
    bad_bios = ["fn f(p:&mut Loaded)", "if p.dos.flushed {}",
                "ansi::installed(p)", "brk::bios_break(p)", "super::files::short(n)"]
    good_bios = ["pub fn video<F>(p:&mut Machine<F>,ah:u8)", "planar::geometry(mode)",
                 "vectors::deliver_irq(p,vector)", "// Dine's Dos sits above Bios"]
    for line in bad_bios:
        assert list(findings(line, bios_patterns)), line
    for line in good_bios:
        assert not list(findings(line, bios_patterns)), line


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
    pc_files = sorted((ROOT / "crates" / "pil-pc" / "src").glob("*.rs"))
    bios_files = sorted((ROOT / "crates" / "pil-bios" / "src").glob("*.rs"))
    failed = 0
    for path in pc_files:
        for number, text in findings(path.read_text(encoding="utf-8"), (PC_FORBIDDEN, PC_SUPER, PC_BARE)):
            print(f"{path.relative_to(ROOT).as_posix()}:{number}: machine code uses {text}")
            failed += 1
    for path in bios_files:
        for number, text in findings(path.read_text(encoding="utf-8"), (BIOS_FORBIDDEN, BIOS_SUPER, BIOS_BARE)):
            print(f"{path.relative_to(ROOT).as_posix()}:{number}: BIOS code uses {text}")
            failed += 1
    if failed:
        print(f"{failed} upward layer reference(s) in pil-pc or pil-bios")
        return 1
    print(f"machine layer: {len(pc_files)} files; BIOS layer: {len(bios_files)} files; no upward references")
    return 0


if __name__ == "__main__":
    sys.exit(main())
