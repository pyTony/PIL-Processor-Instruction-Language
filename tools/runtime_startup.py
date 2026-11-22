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


def last_error_path(root: Path) -> Path:
    return root / "target" / "runtime-checks" / "last-error.txt"


def _cli_newer_than_inputs(root: Path, output: Path) -> bool:
    inputs = [root / n for n in ("Cargo.toml", "Cargo.lock", "build.rs")]
    inputs += [p for folder in ("src", "isa", ".cargo") for p in (root / folder).rglob("*") if p.is_file()]
    return not any(p.is_file() and p.stat().st_mtime_ns > output.stat().st_mtime_ns for p in inputs)


def _pid_alive(pid: int) -> bool:
    if pid <= 0:
        return False
    if os.name == "nt":
        import ctypes
        kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel.OpenProcess.argtypes = (ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong)
        kernel.OpenProcess.restype = ctypes.c_void_p
        kernel.GetExitCodeProcess.argtypes = (ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong))
        kernel.GetExitCodeProcess.restype = ctypes.c_int
        kernel.CloseHandle.argtypes = (ctypes.c_void_p,)
        kernel.CloseHandle.restype = ctypes.c_int
        handle = kernel.OpenProcess(0x1000, False, pid)
        if not handle:
            return ctypes.get_last_error() == 5
        try:
            exit_code = ctypes.c_ulong()
            if kernel.GetExitCodeProcess(handle, ctypes.byref(exit_code)):
                return exit_code.value == 259
            return False
        finally:
            kernel.CloseHandle(handle)
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except OSError:
        return True


def _active_build_marker(root: Path) -> bool:
    marker = root / "target" / "pil-build-in-progress"
    if not marker.exists():
        return False
    pid_file = marker / "pid"
    if not pid_file.is_file():
        return True
    try:
        pid = int(pid_file.read_text(encoding="utf-8").strip())
    except (OSError, ValueError):
        return True
    return _pid_alive(pid)


def _file_in_use(path: Path) -> bool:
    if os.name != "nt" or not path.is_file():
        return False
    try:
        with open(path, "r+b"):
            return False
    except PermissionError:
        return True
    except FileNotFoundError:
        return False


def _write_stamp(root: Path, output: Path, kind: str, inputs_fp: str, output_sha: str) -> None:
    stamp = stamp_path(root, output)
    stamp.parent.mkdir(parents=True, exist_ok=True)
    stamp.write_text(json.dumps({"kind": kind, "inputs": inputs_fp,
                                 "output": output_sha}) + "\n", encoding="utf-8")


def current(root: Path, output: Path, kind: str) -> bool:
    if _active_build_marker(root) or not output.is_file():
        return False
    stamp_file = stamp_path(root, output)
    try:
        stamp = json.loads(stamp_file.read_text(encoding="utf-8"))
        output_sha = hashlib.sha256(output.read_bytes()).hexdigest()
        if isinstance(stamp, dict) and stamp.get("kind") == kind:
            if stamp.get("inputs") == fingerprint(root) and stamp.get("output") == output_sha:
                return True
            if (kind == "cli" and stamp.get("output") != output_sha
                    and output.stat().st_mtime_ns >= stamp_file.stat().st_mtime_ns
                    and _cli_newer_than_inputs(root, output)):
                return True
        return False
    except (OSError, ValueError):
        # Adopt a pre-existing CLI build using the Retro launcher's original
        # timestamp check. A library needs one Cargo check to verify its feature.
        if kind == "cli":
            return _cli_newer_than_inputs(root, output)
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


def publish_latest(root: Path, source: Path) -> Path:
    """Copy a verified shared library to target/runtime-builds/<sha>/ and update dine.latest.json."""
    if not source.is_file():
        raise ValueError(f"cannot publish missing shared library: {source}")
    data = source.read_bytes()
    full_sha = hashlib.sha256(data).hexdigest()
    short = full_sha[:16]
    builds_dir = root / "target" / "runtime-builds"
    version_dir = builds_dir / short
    version_dir.mkdir(parents=True, exist_ok=True)
    copy = version_dir / source.name
    if not copy.is_file() or hashlib.sha256(copy.read_bytes()).hexdigest() != full_sha:
        pending = copy.with_name(copy.name + ".pending")
        shutil.copy2(source, pending)
        pending.replace(copy)
    latest = source.parent / "dine.latest.json"
    record = {
        "format": 1,
        "shared_runtime": source.name,
        "shared_runtime_dir": Path(os.path.relpath(version_dir, source.parent)).as_posix(),
        "sha256": full_sha,
    }
    pending_json = latest.with_name(latest.name + ".pending")
    pending_json.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    pending_json.replace(latest)
    for entry in builds_dir.iterdir():
        if entry.name != short:
            try:
                if entry.is_dir() and not entry.is_symlink():
                    shutil.rmtree(entry)
                else:
                    entry.unlink()
            except OSError:
                pass
    return latest


def show_build_progress() -> None:
    """Make an otherwise hidden shortcut build visible until this process exits."""
    if os.name != "nt":
        return
    import ctypes
    kernel = ctypes.windll.kernel32
    kernel.GetConsoleWindow.restype = ctypes.c_void_p
    window = kernel.GetConsoleWindow()
    if not window:
        if not kernel.AllocConsole():
            return
        window = kernel.GetConsoleWindow()
        sys.stdout = open("CONOUT$", "w", encoding="utf-8", buffering=1)
        sys.stderr = open("CONOUT$", "w", encoding="utf-8", buffering=1)
    user = ctypes.windll.user32
    user.ShowWindow.argtypes = (ctypes.c_void_p, ctypes.c_int)
    user.IsWindowVisible.argtypes = (ctypes.c_void_p,)
    user.ShowWindow(window, 1)
    # WshShell.Run style 0 can override the process's first ShowWindow call.
    if not user.IsWindowVisible(window):
        user.ShowWindow(window, 1)
    kernel.SetConsoleTitleW("Updating PIL DOS")
    print("Updating PIL DOS. It will open when this finishes; please keep this window open.",
          file=sys.stderr, flush=True)


def ensure(root: Path, output: Path, kind: str, *, check_only: bool = False, gui: bool = False) -> Path:
    root, output = root.resolve(), output.resolve()
    if not output.is_relative_to(root) and output.is_file():
        print(f"Using explicitly selected external runtime: {output}", file=sys.stderr)
        return output
    if current(root, output, kind):
        if kind == "cli" and not check_only and (root / "Cargo.toml").is_file():
            inputs_fp = fingerprint(root)
            output_sha = hashlib.sha256(output.read_bytes()).hexdigest()
            stamp = None
            try:
                stamp = json.loads(stamp_path(root, output).read_text(encoding="utf-8"))
            except (OSError, ValueError):
                pass
            if not (isinstance(stamp, dict) and stamp.get("kind") == kind
                    and stamp.get("inputs") == inputs_fp and stamp.get("output") == output_sha):
                _write_stamp(root, output, kind, inputs_fp, output_sha)
        if kind == "library" and not check_only and output.is_relative_to((root / "target").resolve()):
            publish_latest(root, output)
        print(f"Runtime up to date: {output}", file=sys.stderr)
        return output
    marker = root / "target" / "pil-build-in-progress"
    if marker.exists():
        if _active_build_marker(root):
            raise ValueError("A shared runtime build is in progress; wait for it to finish.")
        shutil.rmtree(marker, ignore_errors=True)
    if check_only:
        raise ValueError(f"Runtime needs a freshness/build check: {output}")
    if not (root / "Cargo.toml").is_file() and output.is_file():
        return output  # Installed binary without a source checkout.
    if shutil.which("cargo") is None:
        raise ValueError("Runtime is missing or stale; install Cargo to build it.")
    if _file_in_use(output):
        raise ValueError(f"{output.name} is currently in use. Close any running PIL DOS window and try again.")
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
        (marker / "pid").write_text(f"{os.getpid()}\n", encoding="utf-8")
        if gui:
            show_build_progress()
        print(f"Checking/building shared {kind} runtime: {output}", file=sys.stderr)
        subprocess.run(command, cwd=root, env=env, check=True)
        name = output.name if kind == "library" else ("pil.exe" if os.name == "nt" else "pil")
        built = root / "target" / profile / name
        if not built.is_file():
            raise ValueError(f"Build did not produce {built}")
        if built != output:
            shutil.copy2(built, output)
        _write_stamp(root, output, kind, inputs, hashlib.sha256(output.read_bytes()).hexdigest())
        if kind == "library" and output.is_relative_to((root / "target").resolve()):
            publish_latest(root, output)
        return output
    finally:
        shutil.rmtree(marker, ignore_errors=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--kind", choices=("cli", "library"), default="cli")
    parser.add_argument("--check-only", action="store_true")
    parser.add_argument("--gui", action="store_true")
    args = parser.parse_args()
    err_file = last_error_path(args.root)
    if not args.check_only:
        try:
            err_file.unlink(missing_ok=True)
        except OSError:
            pass
    try:
        git_status(args.root, gui=args.gui)
        ensure(args.root, args.engine, args.kind, check_only=args.check_only, gui=args.gui)
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        message = f"PIL startup: {error}"
        if isinstance(error, subprocess.CalledProcessError) and _file_in_use(args.engine.resolve()):
            message += f"\n{args.engine.name} is currently in use. Close any running PIL DOS window and try again."
        print(message, file=sys.stderr)
        if not args.check_only:
            try:
                err_file.parent.mkdir(parents=True, exist_ok=True)
                err_file.write_text(message + "\n", encoding="utf-8")
            except OSError:
                pass
        return 1


if __name__ == "__main__":
    sys.exit(main())
