#!/usr/bin/env python3
"""Independent end-to-end checks for the Rust service and optional Tk frontend."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import tempfile
import time
ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--engine', type=Path)
parser.add_argument('--frontend', type=Path, default=ROOT/'tools/pil_debugger.py')
parser.add_argument('--window', action='store_true')
args = parser.parse_args()
binary = (args.engine or Path(os.environ.get('CARGO_TARGET_DIR', ROOT/'target'))/'debug'/('pil.exe' if os.name=='nt' else 'pil')).resolve()
spec = importlib.util.spec_from_file_location('pil_frontend', args.frontend)
frontend = importlib.util.module_from_spec(spec)
spec.loader.exec_module(frontend)


def receive(client):
    event, command, value = client.events.get(timeout=15)
    assert event == 'reply', value
    return value


def command(client, text):
    assert client.request(text), text
    return receive(client)


client = frontend.EngineClient([str(binary)])
try:
    initial = receive(client)
    assert initial['seq']==0 and initial['state']['registers'][0]==3
    assert initial['state']['eip']==0x100 and initial['guest_output']==[]
    assert client.request('run 1')
    assert not client.request('run 1')  # at most one outstanding command
    result = receive(client)
    assert result['status']=='budget' and result['state']['registers'][0]==2
    assert result['state']['eip']==0x102
    for invalid in ['run 257', 'time 1000000', 'dosprobe secret.com', 'view 999999']:
        before=result['state']['registers'],result['state']['eip']
        result=command(client,invalid)
        assert not result['ok'] and (result['state']['registers'],result['state']['eip'])==before
    command(client,'demo hello')
    result=command(client,'run 256')
    assert result['status']=='halt' and bytes(result['guest_output'])==b'Hello, world!\n'
    assert 'serial output:' not in result['text']
    assert command(client,'snapshot')['guest_output']==[]
    command(client,'demo echo')
    assert command(client,'run')['status']=='waiting_rx'
    command(client,'serial rx 00')
    result=command(client,'run')
    assert result['status']=='halt' and result['guest_output']==[0]
    assert result['state']['eip']==0x202
    command(client,'demo counter')
    command(client,'break 0x100')
    assert command(client,'run')['status']=='breakpoint'
    result=command(client,'run')
    assert result['state']['registers'][0]==2 and result['state']['eip']==0x100
    assert result['status']=='breakpoint'  # one skip, then loop returns
    assert frontend.next_command(False,{'ok':True,'status':'budget'}) is None
    assert frontend.next_command(True,{'ok':True,'status':'tx_blocked'})=='serial tx'
    assert frontend.next_command(True,{'ok':True,'status':'waiting_rx'}) is None
    for invalid in ['step\nrun', 'step\rrun', 'x'*4096]:
        try:client.request(invalid)
        except ValueError:pass
        else:raise AssertionError('multiline/oversized client command accepted')
    assert command(client,'quit')['quit']
finally:
    client.close()

with tempfile.TemporaryDirectory() as tmp:
    path=Path(tmp)/'raw image with spaces.bin'
    path.write_bytes(bytes.fromhex('b041 e6e9 f4'))
    client=frontend.EngineClient([str(binary)],path)
    try:
        receive(client)
        result=command(client,'run')
        assert result['status']=='halt' and result['guest_output']==[65]
    finally:client.close()

with tempfile.TemporaryDirectory() as tmp:
    path=Path(tmp)/'oversized.bin';path.write_bytes(b'\x90'*4096)
    client=frontend.EngineClient([str(binary)],path)
    try:
        event,_,detail=client.events.get(timeout=15)
        assert event=='error' and 'image must contain' in detail
    finally:client.close()

result=subprocess.run([str(binary),'--debug-service'],input=b'x'*4097,capture_output=True,timeout=15,check=True)
lines=[json.loads(line) for line in result.stdout.splitlines()]
assert len(lines)==2 and not lines[1]['ok'] and lines[1]['quit']
assert lines[0]['state']['registers']==lines[1]['state']['registers']
result=subprocess.run([str(binary),'--debug-service'],input=b'',capture_output=True,timeout=15,check=True)
assert len(result.stdout.splitlines())==1
print('PASS: service JSON, literal CPU states, binary output, waits, breakpoints, bounds, EOF and raw paths; frontend transport/control model')

if args.window:
    import tkinter as tk
    root=tk.Tk();root.withdraw()
    app=frontend.make_app(root,binary)
    def wait_until(test):
        limit=time.monotonic()+15
        while not test():
            assert time.monotonic()<limit, 'Tk smoke check timed out'
            root.update();time.sleep(.01)
    try:
        wait_until(lambda:not app.busy)
        assert app.send('demo hello')
        wait_until(lambda:not app.busy)
        app.run()
        wait_until(lambda:not app.busy and not app.running)
        assert app.guest==b'Hello, world!\n'
        assert len(app.regs.get_children())==10
        assert app.send('demo loop')
        wait_until(lambda:not app.busy)
        app.run();app.pause()
        wait_until(lambda:not app.busy)
        state=app.state['eip'],app.state['registers'][:]
        until=time.monotonic()+.15
        while time.monotonic()<until:root.update();time.sleep(.01)
        assert not app.running and (app.state['eip'],app.state['registers'])==state
    finally:app.close()
    print('PASS: real Tk widgets render backend states/output; cooperative Pause prevents further quanta')
