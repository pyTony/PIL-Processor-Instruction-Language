#!/usr/bin/env python3
"""Acceptance probe for an LZEXE-compressed DOS graphics EXE in the PIL hybrid backend.

The program is supplied by the tester, not stored in this repository.
First inspect its MZ/LZEXE header, then profile it in a drive-isolated DOS
machine, compile the observed blocks and compare bounded interpreter/hybrid
sessions. This tool does not claim unobserved paths or unsupported services
work. Windows GUI inspection remains a separate manual test.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
from dos_terminal import Session  # noqa: E402

COMPARE = (
    "status", "detail", "steps", "cs", "eip", "registers", "selectors", "eflags",
    "msw", "screen", "columns", "cursor", "graphics13", "palette",
    "graphics_cga", "cga_mode", "cga_color", "cga_mode_control",
    "graphics_planar", "planar_mode", "planar_width", "planar_height",
    "planar_rgb", "beeps", "tone", "output",
)


def inspect_mz(program):
    data = program.read_bytes()
    if len(data) < 32 or data[:2] not in (b"MZ", b"ZM"):
        raise ValueError("expected a DOS MZ/ZM executable")
    word = lambda offset: struct.unpack_from("<H", data, offset)[0]
    pages, last = word(4), word(2)
    declared = pages * 512 - ((512 - last) % 512)
    header = word(8) * 16
    if not pages or header < 28 or header >= declared or declared > len(data):
        raise ValueError("invalid MZ size or header")
    return {
        "file": str(program),
        "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
        "mz_declared": declared,
        "header_bytes": header,
        "cs": word(22), "ip": word(20),
        "ss": word(14), "sp": word(16),
        "min_alloc_paragraphs": word(10),
        "packed_with_lzexe": data[28:32] == b"LZ91",
    }


def summary(frame):
    fields = {k: frame.get(k) for k in COMPARE}
    # Keep compact reports: full video/screen pixels may be megabytes of hex.
    return {
        k: {
            "sha256": hashlib.sha256(v.encode("utf-8")).hexdigest(),
            "chars": len(v),
        } if k in ("screen", "graphics13", "graphics_cga",
                   "graphics_planar", "palette", "planar_rgb") and isinstance(v, str)
        else v
        for k, v in fields.items()
    }


def play(engine, program, count, quantum, keys):
    frames = []
    session = Session(engine, program, stop_log="")
    try:
        frames.append(summary(session.last))
        key_index = 0
        for index in range(count):
            prior = session.last
            if prior["status"] not in ("running", "waiting_input"):
                break
            # A key is submitted only at an actual BIOS input wait.
            if prior["status"] == "waiting_input" and key_index < len(keys):
                ascii_code = keys[key_index]
                scan = 0x1c if ascii_code == 13 else 0
                reply = session.send(f"key {ascii_code} {scan}")
                if not reply["ok"]:
                    raise RuntimeError(f"BIOS key rejected: {reply.get('text')}")
                key_index += 1
            step = session.send(f"run {quantum} buffered")
            frame = session.send("snapshot")
            frame["output"] = step.get("output", "")
            frames.append(summary(frame))
            print(f"  {Path(engine).name}: slice {index + 1}, "
                  f"{frame['steps']} steps, {frame['status']}, "
                  f"{frame.get('native_block_instructions', 0)} compiled instructions",
                  flush=True)
            if frame["status"] not in ("running", "waiting_input"):
                break
        return frames, {
            "keys_consumed": key_index,
            "native_blocks": session.last.get("native_blocks", 0),
            "native_block_instructions": session.last.get("native_block_instructions", 0),
        }
    finally:
        session.close()



def reusable_hybrid(program, output):
    """Return an existing verified hybrid only for the exact original guest."""
    manifest_path = output / "manifest.json"
    if not manifest_path.is_file():
        raise ValueError("no prior verified hybrid manifest; run without --reuse first")
    record = json.loads(manifest_path.read_text(encoding="utf-8"))
    expected = "pil-" + program.stem.lower() + (".exe" if os.name == "nt" else "")
    if (record.get("guest_sha256") != hashlib.sha256(program.read_bytes()).hexdigest()
            or record.get("guest_name", "").casefold() != program.name.casefold()
            or record.get("engine") != expected
            or record.get("guard_tests_verified") is not True):
        raise ValueError("previous hybrid does not match this guest or lacks guard verification")
    engine = output / expected
    if not engine.is_file():
        raise ValueError("the previous verified hybrid executable is missing")
    if hashlib.sha256(engine.read_bytes()).hexdigest() != record.get("engine_sha256"):
        raise ValueError("previous hybrid executable differs from its verified manifest")
    compiler = subprocess.check_output(["rustc", "-vV"], text=True)
    host = next((line.split(": ", 1)[1] for line in compiler.splitlines()
                 if line.startswith("host: ")), None)
    if host != record.get("target_triple"):
        raise ValueError("previous hybrid executable targets a different host")
    return engine


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("program", type=Path, help="original DOS LINES.EXE (not a native .exe)")
    parser.add_argument("--engine", type=Path, default=ROOT / "target" / "release" /
                        ("pil.exe" if os.name == "nt" else "pil"))
    parser.add_argument("--output-dir", type=Path, default=ROOT / "target" / "lines-hybrid")
    parser.add_argument("--profile-steps", type=int, default=2_000_000)
    parser.add_argument("--sites", type=int, default=512)
    parser.add_argument("--run-steps", type=int, default=100_000)
    parser.add_argument("--slices", type=int, default=6)
    parser.add_argument("--keys", type=Path, help="optional DOS key script, replayed at BIOS waits")
    parser.add_argument("--scan-only", action="store_true", help="inspect EXE without building or executing")
    parser.add_argument("--reuse", action="store_true", help="reuse an existing hash-verified compiled hybrid without profiling/building")
    args = parser.parse_args()
    program = args.program.resolve()
    if not program.is_file() or program.suffix.lower() != ".exe":
        parser.error("expected an existing DOS .EXE file")
    try:
        info = inspect_mz(program)
    except ValueError as exc:
        parser.error(str(exc))
    print(json.dumps(info, indent=2), flush=True)
    if args.scan_only:
        return 0
    if not (1 <= args.run_steps <= 1_000_000 and 1 <= args.slices <= 100):
        parser.error("--run-steps must be 1..1000000 and --slices 1..100")
    if not (1 <= args.profile_steps <= 120_000_000 and 1 <= args.sites <= 65_536):
        parser.error("invalid profiling step/site budget")
    engine = args.engine.resolve()
    if not engine.is_file():
        parser.error(f"engine not found: {engine}; run cargo build --release --locked first")
    keys = args.keys.read_bytes() if args.keys else b""
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if args.reuse:
        try:
            hybrid = reusable_hybrid(program, output)
        except (ValueError, OSError, json.JSONDecodeError) as exc:
            parser.error(f"--reuse: {exc}")
        print(f"Reusing guard-verified hybrid: {hybrid}", flush=True)
    else:
        hybrid = output / ("pil-" + program.stem.lower() + (".exe" if os.name == "nt" else ""))
    print("Reference interpreter run", flush=True)
    reference, reference_stats = play(engine, program, args.slices, args.run_steps, keys)
    if not args.reuse:
        print("Building profiled native blocks (interpretation remains available)", flush=True)
        subprocess.run([sys.executable, str(ROOT / "tools" / "build_dos_shadow.py"),
                        str(program), "--engine", str(engine),
                        "--output-dir", str(output),
                        "--steps", str(args.profile_steps), "--sites", str(args.sites),
                        "--verify"] + (["--keys", str(args.keys.resolve())] if args.keys else []),
                       cwd=ROOT, check=True)
    print("Hybrid run", flush=True)
    compiled, compiled_stats = play(hybrid, program, args.slices, args.run_steps, keys)
    mismatches = []
    for index, (left, right) in enumerate(zip(reference, compiled)):
        for key in COMPARE:
            if left.get(key) != right.get(key):
                mismatches.append({"slice": index, "field": key,
                                   "reference": left.get(key), "hybrid": right.get(key)})
    if len(reference) != len(compiled):
        mismatches.append({"field": "frame_count", "reference": len(reference),
                           "hybrid": len(compiled)})
    terminal = reference[-1]["status"]
    graphics_observed = any(
        frame.get(field) is not None
        for frame in reference for field in ("graphics13", "graphics_cga", "graphics_planar")
    )
    result = {
        "guest": info, "reference": reference, "hybrid": compiled,
        "reference_stats": reference_stats, "compiled_stats": compiled_stats,
        "mismatches": mismatches, "graphics_observed": graphics_observed,
        "conclusion": "mismatch" if mismatches else "unsupported_service" if terminal == "stopped"
                      else "matching_with_compiled_blocks" if compiled_stats["native_block_instructions"]
                      else "matching_interpreter_fallback_only",
    }
    report = output / "lines-comparison.json"
    report.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n",
                      encoding="utf-8")
    print(f"Report: {report}", flush=True)
    print(f"Result: {result['conclusion']}; "
          f"native block instructions={compiled_stats['native_block_instructions']}; "
          f"graphics observed={graphics_observed}", flush=True)
    if mismatches:
        print("First mismatches:", json.dumps(mismatches[:5], indent=2), flush=True)
        return 1
    if terminal == "stopped":
        print(f"Both engines stopped: {reference[-1]['detail']}", flush=True)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
