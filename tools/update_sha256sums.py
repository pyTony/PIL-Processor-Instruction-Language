#!/usr/bin/env python3
"""Regenerate docs/SHA256SUMS from the files git tracks.

One line "{sha256}  {path}" per file in `git ls-files`, sorted by path,
docs/SHA256SUMS itself left out. Run it after `git add -A`, or after a
merge, so the list matches the tree. `--check` only reports whether the
file is up to date (exit 1 if not).
"""
import hashlib
import os
import subprocess
import sys

TARGET = "docs/SHA256SUMS"


def manifest(root):
    paths = subprocess.run(["git", "ls-files", "-z"], cwd=root, check=True,
                           stdout=subprocess.PIPE).stdout.decode("utf-8").split("\0")
    lines = []
    for path in sorted(p for p in paths if p and p != TARGET):
        full = os.path.join(root, path)
        if not os.path.isfile(full):
            continue  # deleted in the working tree
        with open(full, "rb") as handle:
            digest = hashlib.sha256(handle.read()).hexdigest()
        lines.append(f"{digest}  {path}\n")
    return "".join(lines)


def main(argv):
    root = subprocess.run(["git", "rev-parse", "--show-toplevel"], check=True,
                          stdout=subprocess.PIPE, text=True).stdout.strip()
    text = manifest(root)
    target = os.path.join(root, TARGET)
    try:
        with open(target, encoding="utf-8", newline="") as handle:
            current = handle.read()
    except FileNotFoundError:
        current = None
    if "--check" in argv:
        print(f"{TARGET}: {'up to date' if current == text else 'out of date'}")
        return 0 if current == text else 1
    if current != text:
        with open(target, "w", encoding="utf-8", newline="") as handle:
            handle.write(text)
        print(f"{TARGET}: rewritten ({text.count(chr(10))} files)")
    else:
        print(f"{TARGET}: already up to date")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
