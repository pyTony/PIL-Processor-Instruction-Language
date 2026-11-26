#!/usr/bin/env python3
"""Build with cargo and fail if rustc reports any warning.

Default: `cargo build --release --locked`, which builds both the `pil`
executable as a thin caller of the `dine` library. Extra arguments replace
the cargo subcommand arguments, for example:

    python tools/check_build_warnings.py build --release --locked --lib --features dine-library

Compiler diagnostics are read from Cargo's structured message output.
"""
import json
import subprocess
import sys


def main(argv):
    args = argv or ["build", "--release", "--locked"]
    command = ["cargo", *args, "--message-format=json-diagnostic-short"]
    process = subprocess.run(command, stdout=subprocess.PIPE, text=True,
                             encoding="utf-8", errors="replace")
    errors = []
    warnings = []
    for line in process.stdout.splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get("reason") != "compiler-message":
            continue
        diagnostic = message.get("message", {})
        level = diagnostic.get("level")
        if level in ("warning", "error"):
            rendered = (diagnostic.get("rendered") or diagnostic.get("message") or "").strip()
            target = message.get("target", {}).get("name", "?")
            entry = f"[{target}] {rendered}"
            if level == "error":
                errors.append(entry)
            else:
                warnings.append(entry)
    if process.returncode != 0:
        for error in errors:
            print(error)
        print(f"cargo {' '.join(args)} failed with exit code {process.returncode}")
        return process.returncode
    for warning in warnings:
        print(warning)
    print(f"cargo {' '.join(args)}: {len(warnings)} rustc warning(s)")
    return 1 if warnings else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
