#!/usr/bin/env python3
"""Build a supported DOS COM/MZ EXE as a standalone native host executable.

The compiler rejects unsupported reachable code. This is not the Dine shadow
backend: the generated binary has no x86 decoder or interpreter fallback.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("program", type=Path, help="existing DOS .COM or MZ .EXE")
    parser.add_argument("--engine", type=Path, help="existing unmodified PIL engine; otherwise build release")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "programs" / "compiled")
    parser.add_argument("--force", action="store_true", help="replace artifacts for the same guest name")
    args = parser.parse_args()

    guest = args.program.resolve()
    if not guest.is_file() or guest.suffix.lower() not in (".com", ".exe"):
        parser.error("program must be an existing DOS .COM or .EXE")
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    # Preserve the DOS extension on Windows to avoid conflating FOO.COM
    # and FOO.EXE. On Unix use an explicitly native-looking suffix.
    name = guest.name + (".exe" if os.name == "nt" else ".native")
    executable = output_dir / name
    source = output_dir / (name + ".rs")
    manifest = output_dir / (name + ".json")
    artifacts = (executable, source, manifest)
    if not args.force and any(path.exists() for path in artifacts):
        parser.error("output already exists; choose --output-dir or use --force")

    env = os.environ.copy()
    env.pop("PIL_SHADOW_SOURCE", None)
    env.pop("CARGO_BUILD_TARGET", None)
    if args.engine:
        engine = args.engine.resolve()
        if not engine.is_file():
            parser.error("--engine must name an existing PIL executable")
    else:
        # Keep the standalone source generator separate from any linked
        # shadow-engine build that the caller may have configured.
        env["CARGO_TARGET_DIR"] = str(ROOT / "target")
        subprocess.run(["cargo", "build", "--release", "--locked"], cwd=ROOT, env=env, check=True)
        engine = ROOT / "target" / "release" / ("pil.exe" if os.name == "nt" else "pil")

    rustc = subprocess.check_output(["rustc", "-vV"], text=True)
    host = next((line.split(": ", 1)[1] for line in rustc.splitlines()
                 if line.startswith("host: ")), "unknown")

    # No generated files are published if translation or compilation fails.
    with tempfile.TemporaryDirectory(prefix="dos-native-", dir=output_dir) as tmp:
        work = Path(tmp)
        pending_source = work / "program.rs"
        pending_binary = work / name
        subprocess.run([str(engine), "--emit-dos-standalone", str(guest), str(pending_source)],
                       cwd=ROOT, env=env, check=True)
        subprocess.run(["rustc", "--edition=2021", "-O", "-Ccodegen-units=1", "-A", "unused",
                        str(pending_source), "-o", str(pending_binary)], cwd=ROOT, check=True)
        record = {
            "format": 1,
            "mode": "standalone",
            "automatic_dos_drive_loading": False,
            "guest_name": guest.name,
            "guest_sha256": sha256(guest),
            "host": host,
            "rustc": rustc.splitlines()[0],
            "engine": str(engine),
            "engine_sha256": sha256(engine),
            "source": source.name,
            "source_sha256": sha256(pending_source),
            "executable": executable.name,
            "executable_sha256": sha256(pending_binary),
            "services": "standalone headless subset; no file/keyboard/EXEC API",
        }
        pending_manifest = work / "manifest.json"
        pending_manifest.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
        # Publish only after native rustc has succeeded. Replacements occur on
        # the same filesystem; a failed compile preserves previous artifacts.
        os.replace(pending_source, source)
        os.replace(pending_binary, executable)
        os.replace(pending_manifest, manifest)

    print(f"Native executable: {executable}")
    print(f"Inspectable Rust:   {source}")
    print(f"Build identity:     {manifest}")


if __name__ == "__main__":
    main()
