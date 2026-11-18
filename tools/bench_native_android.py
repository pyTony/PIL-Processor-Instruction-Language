#!/usr/bin/env python3
"""Benchmark the same native PIL-generated x86 loop at Rust O0/O1/O2.

Builds are sequential; no release Cargo build is started. The same generated
Rust input is compiled three times with rustc for the current host.
"""
import argparse
import csv
import json
from pathlib import Path
import re
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target" / "native-opt-levels"
PATTERN = re.compile(r"^NATIVE_RESULT median_ns=(\d+) instructions=(\d+) samples=",
                     re.MULTILINE)


def run(args, *, stdout=None):
    return subprocess.run(args, cwd=ROOT, check=True, stdout=stdout)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--iterations", type=int, default=10000)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--emit-only", action="store_true",
                        help="write Rust source only, for inspection")
    parser.add_argument("--asm", action="store_true",
                        help="also emit assembly for each optimization level (extra compile)")
    args = parser.parse_args()
    if not 1 <= args.iterations <= 100000:
        parser.error("iterations must be 1..100000")
    if not 1 <= args.samples <= 9:
        parser.error("samples must be 1..9")
    pil = ROOT / "target" / "debug" / "pil"
    if not pil.is_file():
        parser.error("missing target/debug/pil; first run sh tools/build_termux.sh --compact")

    OUT.mkdir(parents=True, exist_ok=True)
    source = OUT / "loop.rs"
    with source.open("wb") as output:
        run([str(pil), "--emit-loop-rust", str(args.iterations),
             str(args.samples)], stdout=output)
    print(f"Generated source: {source} ({source.stat().st_size:,} bytes)", flush=True)
    if args.emit_only:
        return 0

    rows = []
    # Build each level independently, avoiding a large parallel Cargo build.
    for opt in (0, 1, 2):
        binary = OUT / f"loop_O{opt}"
        command = ["rustc", "--edition=2021", "-C", f"opt-level={opt}",
                   "-C", "debuginfo=0", "-C", "codegen-units=4",
                   "-C", "panic=abort", "-A", "unused_assignments",
                   str(source), "-o", str(binary)]
        print(f"Compiling O{opt}...", flush=True)
        begin = time.monotonic()
        with (OUT / f"O{opt}.compile.log").open("wb") as log:
            result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
        compile_s = time.monotonic() - begin
        if result.returncode != 0:
            print(f"O{opt} compile failed (exit {result.returncode}); "
                  f"see {OUT / f'O{opt}.compile.log'}", file=sys.stderr)
            return result.returncode

        # Each generated sample verifies registers, flags, RAM, stop and step count.
        output = subprocess.check_output([str(binary), str(OUT / f"O{opt}.trace"),
                                          "bench"], cwd=ROOT, text=True)
        (OUT / f"O{opt}.run.log").write_text(output, encoding="utf-8")
        match = PATTERN.search(output)
        if not match:
            print(f"O{opt} missing benchmark result; see run log", file=sys.stderr)
            return 1
        ns, instructions = map(int, match.groups())
        if ns <= 0 or instructions != 3 * args.iterations + 3:
            print(f"O{opt} invalid result: {match.group(0)!r}", file=sys.stderr)
            return 1
        row = {"opt": f"O{opt}", "compile_s": round(compile_s, 3),
               "binary_bytes": binary.stat().st_size, "median_ns": ns,
               "guest_instructions": instructions,
               "guest_ips": round(instructions * 1e9 / ns)}
        rows.append(row)
        print(f"  {row['compile_s']:.3f} s compile; {row['binary_bytes']:,} bytes; "
              f"{row['guest_ips']:,} guest instr/s", flush=True)
        # Save partial output even if a later optimization level exceeds phone RAM.
        (OUT / "report.json").write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8")
        with (OUT / "report.csv").open("w", newline="", encoding="utf-8") as file:
            writer = csv.DictWriter(file, fieldnames=rows[0].keys())
            writer.writeheader()
            writer.writerows(rows)
        if args.asm:
            asm = OUT / f"loop_O{opt}.s"
            asm_cmd = command[:-2] + ["--emit=asm", "-o", str(asm)]
            print(f"  Emitting ARM64 assembly: {asm}", flush=True)
            with (OUT / f"O{opt}.asm.log").open("wb") as log:
                asm_result = subprocess.run(asm_cmd, cwd=ROOT,
                                            stdout=log, stderr=subprocess.STDOUT)
            if asm_result.returncode:
                print(f"  Assembly emission failed; inspect O{opt}.asm.log",
                      file=sys.stderr)
                return asm_result.returncode
    base = rows[0]["median_ns"]
    print("\nLevel | compile s | binary bytes | median ms | guest instr/s | vs O0")
    for row in rows:
        print(f"{row['opt']:>5} | {row['compile_s']:>9.3f} | "
              f"{row['binary_bytes']:>12,} | {row['median_ns']/1e6:>9.3f} | "
              f"{row['guest_ips']:>13,} | {base/row['median_ns']:>5.2f}x")
    print(f"Reports: {OUT / 'report.csv'} and {OUT / 'report.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
