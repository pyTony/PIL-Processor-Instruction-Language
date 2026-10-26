#!/usr/bin/env python3
"""Run the development-only actual-x86-64 CPU oracle; never claim an ARM pass."""
from pathlib import Path
import platform,subprocess,sys
root=Path(__file__).resolve().parents[1]
if platform.machine().lower() not in ('x86_64','amd64'):
    print(f'UNAVAILABLE: actual x86-64 CPU oracle on {platform.machine()}; no hardware verification performed.')
    raise SystemExit(2)
result=subprocess.run(['cargo','test','--locked','host_cpu_alu_matches_interpreter_and_generated_rust','--','--nocapture'],cwd=root,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
print(result.stdout,end='')
if result.returncode:raise SystemExit(result.returncode)
if 'PASS: HOST_CPU ' not in result.stdout:
    print('ERROR: cargo completed without an actual hardware-oracle result; check target/runner configuration.',file=sys.stderr)
    raise SystemExit(2)
