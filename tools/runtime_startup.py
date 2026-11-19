"""Shared make-style startup checks for PIL's CLI and Dine library."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


def git_status(root: Path, *, gui: bool = False) -> str:
    if not (root / ".git").exists():
        return ""
    try:
        result = subprocess.run(["git", "status", "--porcelain=v2", "--branch"],
                                cwd=root, capture_output=True, text=True)
    except OSError:
        print("Warning: Git is unavailable; repository status could not be checked.", file=sys.stderr)
        return ""
    if result.returncode:
        print("Warning: could not read Git status.", file=sys.stderr)
        return ""
    lines = result.stdout.splitlines()
    branch = next((s[len("# branch.head "):] for s in lines if s.startswith("# branch.head ")), "unknown")
    ahead = next((s[len("# branch.ab "):] for s in lines if s.startswith("# branch.ab ")), "")
    tracked = sum(s.startswith(("1 ", "2 ", "u ")) for s in lines)
    untracked = sum(s.startswith("? ") for s in lines)
    text = f"Git: {branch}; {tracked} changed, {untracked} untracked"
    if ahead:
        text += f"; ahead/behind {ahead} (known upstream state)"
    print(text, file=sys.stderr)
    if branch not in ("dev", "main"):
        warning = f"Warning: branch is {branch}, not dev/main."
        print(warning, file=sys.stderr)
        if gui and os.name == "nt":
            import ctypes
            ctypes.windll.user32.MessageBoxW(None, text + "\n\n" + warning, "PIL startup", 0x30)
    return text


def fingerprint(root: Path) -> str:
    inputs = [root / name for name in ("Cargo.toml", "Cargo.lock", "build.rs",
                                      ".cargo/config", ".cargo/config.toml")]
    inputs += [p for folder in ("src", "isa") for p in (root / folder).rglob("*") if p.is_file()]
    digest = hashlib.sha256()
    for path in sorted(set(inputs)):
        if path.is_file():
            digest.update(path.relative_to(root).as_posix().encode() + b"\0")
            digest.update(path.read_bytes())
            digest.update(b"\0")
    for key, value in sorted(os.environ.items()):
        if key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "RUSTC_WRAPPER", "RUSTUP_TOOLCHAIN") or key.startswith("CARGO_PROFILE_"):
            digest.update((key + "=" + value).encode() + b"\0")
    return digest.hexdigest()


def stamp_path(root: Path, output: Path) -> Path:
    identity = hashlib.sha256(str(output.resolve()).encode()).hexdigest()[:16]
    return root / "target" / "runtime-checks" / (identity + ".json")


def current(root: Path, output: Path, kind: str) -> bool:
    if (root / "target" / "pil-build-in-progress").exists() or not output.is_file():
        return False
    try:
        stamp = json.loads(stamp_path(root, output).read_text(encoding="utf-8"))
        return (isinstance(stamp, dict) and stamp.get("kind") == kind
                and stamp.get("inputs") == fingerprint(root)
                and stamp.get("output") == hashlib.sha256(output.read_bytes()).hexdigest())
    except (OSError, ValueError):
        # Adopt a pre-existing CLI build using the Retro launcher's original
        # timestamp check. A library needs one Cargo check to verify its feature.
        if kind == "cli":
            inputs = [root / n for n in ("Cargo.toml", "Cargo.lock", "build.rs")]
            inputs += [p for folder in ("src", "isa", ".cargo") for p in (root / folder).rglob("*") if p.is_file()]
            return not any(p.is_file() and p.stat().st_mtime_ns > output.stat().st_mtime_ns for p in inputs)
        return False


def record_copy(root: Path, source: Path, output: Path) -> None:
    """Keep a deployed shared library tied to its verified common build."""
    if not current(root, source, "library"):
        return
    data = stamp_path(root, source).read_text(encoding="utf-8")
    record = json.loads(data)
    if record["output"] == hashlib.sha256(output.read_bytes()).hexdigest():
        destination = stamp_path(root, output)
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(data, encoding="utf-8")


def ensure(root: Path, output: Path, kind: str, *, check_only: bool = False) -> Path:
    root, output = root.resolve(), output.resolve()
    if not output.is_relative_to(root) and output.is_file():
        print(f"Using explicitly selected external runtime: {output}", file=sys.stderr)
        return output
    if current(root, output, kind):
        stamp = stamp_path(root, output)
        if kind == "cli" and not stamp.exists() and not check_only and (root / "Cargo.toml").is_file():
            stamp.parent.mkdir(parents=True, exist_ok=True)
            stamp.write_text(json.dumps({"kind": kind, "inputs": fingerprint(root),
                                        "output": hashlib.sha256(output.read_bytes()).hexdigest()}) + "\n", encoding="utf-8")
        print(f"Runtime up to date: {output}", file=sys.stderr)
        return output
    marker = root / "target" / "pil-build-in-progress"
    if marker.exists():
        raise ValueError("A shared runtime build is in progress; wait for it to finish.")
    if check_only:
        raise ValueError(f"Runtime needs a freshness/build check: {output}")
    if not (root / "Cargo.toml").is_file() and output.is_file():
        return output  # Installed binary without a source checkout.
    if shutil.which("cargo") is None:
        raise ValueError("Runtime is missing or stale; install Cargo to build it.")
    inputs = fingerprint(root)
    output.parent.mkdir(parents=True, exist_ok=True)
    marker.parent.mkdir(parents=True, exist_ok=True)
    marker.mkdir()
    profile = "debug" if output.parent.name == "debug" else "release"
    command = ["cargo", "build", "--locked"]
    if profile == "release":
        command.append("--release")
    command += ["--lib", "--features", "dine-library"] if kind == "library" else ["--bin", "pil"]
    env = os.environ.copy()
    env.pop("CARGO_BUILD_TARGET", None)
    env.pop("CARGO_TARGET_DIR", None)
    try:
        print(f"Checking/building shared {kind} runtime: {output}", file=sys.stderr)
        subprocess.run(command, cwd=root, env=env, check=True)
        name = output.name if kind == "library" else ("pil.exe" if os.name == "nt" else "pil")
        built = root / "target" / profile / name
        if not built.is_file():
            raise ValueError(f"Build did not produce {built}")
        if built != output:
            shutil.copy2(built, output)
        stamp = stamp_path(root, output)
        stamp.parent.mkdir(parents=True, exist_ok=True)
        stamp.write_text(json.dumps({"kind": kind, "inputs": inputs,
                                    "output": hashlib.sha256(output.read_bytes()).hexdigest()}) + "\n", encoding="utf-8")
        return output
    finally:
        marker.rmdir()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--kind", choices=("cli", "library"), default="cli")
    parser.add_argument("--check-only", action="store_true")
    parser.add_argument("--gui", action="store_true")
    args = parser.parse_args()
    try:
        git_status(args.root, gui=args.gui)
        ensure(args.root, args.engine, args.kind, check_only=args.check_only)
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"PIL startup: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
