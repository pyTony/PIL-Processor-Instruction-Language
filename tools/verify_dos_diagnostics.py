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
    # A ROM installs #UD, faults, returns past UD2, then halts elsewhere.
    rom = bytearray(65536)
    rom[0xfff0:0xfff5] = bytes.fromhex('ea000100f0')
    code = bytes.fromhex('31c08ed8c70618000002c7061a0000f00f0bfaf4')
    rom[0x100:0x100+len(code)] = code
    rom[0x200:0x207] = bytes.fromhex('5883c00250cf90')
    rom_path = root / 'test rom.bin'
    rom_path.write_bytes(rom)
    rom_log = root / 'rom stops.jsonl'
    result = subprocess.run([str(engine), '--stop-log', str(rom_log), '--run-rom', str(rom_path), '--steps', '100'], capture_output=True, text=True, check=True)
    assert 'Exception #06 at F000:0110' in result.stdout, result.stdout
    record = json.loads(rom_log.read_text())
    assert record['status'] == 'stopped'
    assert record['program'] == str(rom_path)
    assert record['location'] != 'F000:0110'
    assert record['exception_count'] == 1
    fault = record['exceptions'][0]
    assert fault['vector'] == 6 and fault['location'] == 'F000:0110'
    assert fault['instruction_bytes'].startswith('0F 0B')
    assert len(fault['registers']) == 8
    assert record['time_unix_ms'] > 0
    # Direct engine sessions also log without a Python frontend.
    direct_log = root / 'direct.jsonl'
    result = subprocess.run([str(engine), '--stop-log', str(direct_log), '--dos-session', str(program)], input='run 10\nsnapshot\nquit\n', capture_output=True, text=True, check=True)
    assert len(direct_log.read_text().splitlines()) == 1
    assert all(json.loads(line)['protocol'] == 1 for line in result.stdout.splitlines())
    # Empty path disables logging; it must not create the default log.
    subprocess.run([str(engine), '--stop-log', '', '--run-rom', str(rom_path), '--steps', '100'], cwd=root, capture_output=True, check=True)
    assert not (root / 'PILdos-stops.jsonl').exists()
    # Default ROM logging appends across runs, including budget stops.
    rom[0x200] = 0xcf  # IRET retries the same faulting instruction.
    rom_path.write_bytes(rom)
    for _ in range(2):
        subprocess.run([str(engine), '--run-rom', str(rom_path), '--steps', '300'], cwd=root, capture_output=True, check=True)
    records = [json.loads(line) for line in (root / 'PILdos-stops.jsonl').read_text().splitlines()]
    assert len(records) == 2
    assert all(r['status'] == 'running' and r['exception_count'] > 64 and len(r['exceptions']) == 64 for r in records)
    # An unwritable log reports a warning without breaking protocol replies.
    result = subprocess.run([str(engine), '--stop-log', str(root), '--dos-session', str(program)], input='run 10\nquit\n', capture_output=True, text=True, check=True)
    assert 'Cannot write stop log' in result.stderr
    assert all(json.loads(line)['protocol'] == 1 for line in result.stdout.splitlines())

print('PASS: strict/ignored I/O, unchanged output, profile JSON and engine-owned DOS/ROM stop logs with original #UD sites')
