"""Exercise profiling, stop logs and unassigned OUT policy with a built engine."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
from dos_terminal import Session

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--engine', type=Path, required=True)
engine = parser.parse_args().engine.resolve()
with tempfile.TemporaryDirectory(prefix='pil-diagnostics-') as temp:
    root = Path(temp)
    program = root / 'probe.com'
    for port, allowed in [(0x5555, True), (0x201, False), (0x3c4, False)]:
        program.write_bytes(bytes([0xba, port & 255, port >> 8, 0xee, 0xb8, 0, 0x4c, 0xcd, 0x21]))
        plain = subprocess.run([str(engine), '--dos-probe', str(program)], capture_output=True, text=True, check=True)
        assert 'STOP Service(' in plain.stdout
        result = subprocess.run([str(engine), '--ignore-unassigned-out', '--dos-probe', str(program)], capture_output=True, text=True, check=True)
        assert ('STOP Exit(0)' in result.stdout) == allowed, result.stdout
        if allowed:
            assert 'Ignored unassigned OUT port 5555h' in result.stderr
    program.write_bytes(bytes.fromhex('b8004ccd21'))
    plain = subprocess.run([str(engine), '--dos-probe', str(program)], capture_output=True, check=True)
    profile = root / 'profile.json'
    result = subprocess.run([str(engine), '--profile', str(profile), '--dos-probe', str(program)], capture_output=True, check=True)
    assert result.stdout == plain.stdout
    assert json.loads(profile.read_text())['sites']
    program.write_bytes(bytes.fromhex('0f0b'))
    log = root / 'stops.jsonl'
    session = Session(engine, program, stop_log=log, profile=profile)
    try:
        assert session.send('run 10')['status'] == 'stopped'
        session.send('snapshot')
    finally:
        session.close()
    records = log.read_text().splitlines()
    assert len(records) == 1
    record = json.loads(records[0])
    assert record['instruction_bytes'].startswith('0F 0B')
    assert len(record['registers']) == 8
    assert json.loads(profile.read_text())['sites']
print('PASS: strict/ignored I/O, unchanged output, profile JSON and deduplicated stop logs')
