#!/usr/bin/env python3
"""Conservative CI routing. Unclassified paths and unavailable diffs run all jobs."""
import argparse
import os
from pathlib import Path
import subprocess

MODULES = ("core", "dos", "frontend", "web", "integration", "library", "backends", "delivery", "performance")
# Only these reviewed frontend inputs bypass Rust/emulator verification.
FRONTEND = {
    "tools/termux_web.py", "tools/termux_web.html", "tools/dos_terminal.py",
    "tools/pil_debugger.py", "tools/makeshortcut.py",
    "tools/PILdos.sh", "PILdos.vbs", "PILdos.ico", "tools/PILdos.png",
    "pildos.conf.example",
}
# These only select/build/load the shared runtime; they do not implement CPU
# instructions. ABI and delivery checks still verify the produced launchers.
RUNTIME_LAUNCHERS = {
    "tools/runtime_startup.py", "tools/build_dine.py",
    "compile-dos.cmd", "tools/compile-dos.sh", "PILbuild.cmd",
    "PILmedia.vbs", "PILmedia.cmd",
}
# Shared rendering and container import need DOS/container tests and integration,
# rather than the instruction oracles or generated CPU-backend comparisons.
DOS_SUPPORT = {
    "src/dine_window.rs", "src/dine_display.rs", "src/containers.rs",
    "tests/unit/containers.rs", "tests/unit/host_console.rs",
}


def classify(paths):
    selected = set()
    for path in paths:
        if path.endswith(".md") or path == "docs/SHA256SUMS":
            continue  # Documentation is still checked in the always-running job.
        if path == "tests/ci/test_runtime_startup.py":
            continue  # Already executed by Check's always-running selection job.
        if path in {"tools/termux_web.html", "tools/termux_web.py", "tests/gui/test_termux_web.py"}:
            selected.add("web")
        elif path in RUNTIME_LAUNCHERS:
            selected.update(("frontend", "library", "delivery"))
        elif path in FRONTEND or path.startswith("tests/gui/"):
            selected.add("frontend")
            if path in {"tests/gui/test_export_runtime.py", "tests/gui/test_build_dine.py"}:
                selected.add("delivery")
        elif path in DOS_SUPPORT or path == "src/dos.rs" or path.startswith(("crates/pil-pc/", "crates/pil-bios/", "src/dos/", "src/host/", "tests/unit/dos", "programs/dos/")):
            selected.update(("dos", "frontend", "integration", "library"))
        elif path.startswith(("src/", "crates/", "isa/", "tests/unit/", "programs/")) or path in {"Cargo.toml", "Cargo.lock"}:
            selected.update(("core", "dos", "frontend", "integration", "library", "backends"))
        elif path.startswith("tools/bench") or path.startswith("tools/compare."):
            selected.update(("core", "performance"))
        elif path in {"tools/package.py", "tools/export_runtime.py"}:
            selected.update(("frontend", "delivery"))
        else:
            return set(MODULES)
    return selected


def changed_paths(event, env):
    if event == "pull_request":
        revision = f"{env['PR_BASE']}...{env['PR_HEAD']}"
    elif event == "workflow_dispatch" and env.get("BASE_REF"):
        # Used after GITHUB_TOKEN refresh pushes: compare the entire branch.
        revision = f"origin/{env['BASE_REF']}...HEAD"
    elif event == "push":
        revision = f"{env['BEFORE']}..{env['AFTER']}"
    else:
        raise ValueError("No change range: full validation")
    result = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", "-z", revision],
        check=True, stdout=subprocess.PIPE,
    )
    return result.stdout.decode("utf-8").rstrip("\0").split("\0") if result.stdout else []


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--full", action="store_true")
    args = parser.parse_args()
    try:
        selected = set(MODULES) if args.full else classify(changed_paths(os.environ["EVENT"], os.environ))
    except (KeyError, ValueError, UnicodeError, subprocess.CalledProcessError) as error:
        print(f"Cannot determine affected paths ({error}); running all modules")
        selected = set(MODULES)
    lines = [f"{name}={str(name in selected).lower()}" for name in MODULES]
    print("\n".join(lines))
    with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as output:
        output.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
