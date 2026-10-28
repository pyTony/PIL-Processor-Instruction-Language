#!/usr/bin/env python3
"""Original self-checking COM fixtures run unchanged in DOSBox-X and PIL.

This is an emulator differential/expected-state smoke test, not an external
80386 hardware suite, cycle benchmark, BIOS test or whole-DOS conformance test.
No assembler, third-party guest binaries or runtime dependencies are required.
"""
from pathlib import Path
import argparse
import hashlib
import json
import re
import subprocess
import time

ARITH = 0x8d5
DF = 0x400
SENTINEL_DX = 0xc0deface


def parity_flags(result, width):
    return (0x40 if result == 0 else 0) | (0x80 if result & (1 << (width-1)) else 0) | (4 if (result & 255).bit_count() % 2 == 0 else 0)


def merge(old, result, width):
    mask = (1 << width)-1
    return (old & ~mask) | (result & mask)


def arithmetic(a, b, width, subtract=False, carry=0):
    mask = (1 << width)-1
    a &= mask
    b &= mask
    wide = a-b-carry if subtract else a+b+carry
    result = wide & mask
    flags = parity_flags(result, width)
    if (a < b+carry) if subtract else (wide > mask):
        flags |= 1
    if (a ^ b ^ result) & 16:
        flags |= 16
    sign = 1 << (width-1)
    if (((a ^ b) if subtract else ~(a ^ b)) & (a ^ result) & sign):
        flags |= 0x800
    return result, flags


def cases():
    result = []
    def add(name, width, a, b, opcode, expected, flags, flag_mask=ARITH | DF, incoming=0):
        initial_a = merge(0x13579bdf, a, width)
        initial_b = merge(0x2468ace0, b, width)
        result.append(dict(name=name, width=width, eax=initial_a, ecx=initial_b,
                           edx=SENTINEL_DX, code=opcode.hex(),
                           expected_eax=merge(initial_a, expected, width),
                           expected_edx=SENTINEL_DX, flags=flags,
                           flags_mask=flag_mask, incoming_flags=0x202 | incoming))
    binary = [('add', 0x00), ('sub', 0x28), ('adc', 0x10), ('sbb', 0x18), ('cmp', 0x38)]
    for width in (8, 16, 32):
        mask = (1 << width)-1
        sign = 1 << (width-1)
        prefix = bytes([0x66]) if width == 32 else b''
        for name, opcode in binary:
            for index, (a, b) in enumerate([(0, 0), (mask, 1), (sign-1, 1), (sign, 1), (0, mask)]):
                for carry in ((0, 1) if name in ('adc', 'sbb') else (0,)):
                    value, flags = arithmetic(a, b, width, name in ('sub', 'sbb', 'cmp'), carry)
                    add(f'{name}{width}_{index}_c{carry}', width, a, b,
                        prefix+bytes([opcode+(width != 8), 0xc8]),
                        a if name == 'cmp' else value, flags, incoming=carry)
        for name, opcode in [('and', 0x20), ('or', 0x08), ('xor', 0x30), ('test', 0x84)]:
            for index, (a, b) in enumerate([(0, mask), (mask, sign), (sign, sign-1)]):
                value = (a & b) if name in ('and', 'test') else (a | b) if name == 'or' else (a ^ b)
                add(f'{name}{width}_{index}', width, a, b, prefix+bytes([opcode+(width != 8), 0xc8]),
                    a if name == 'test' else value, parity_flags(value, width),
                    flag_mask=(ARITH & ~16) | DF, incoming=ARITH)
        for name, ext in [('inc', 0), ('dec', 1), ('not', 2), ('neg', 3)]:
            for index, a in enumerate([0, sign-1, sign, mask]):
                incoming = ARITH if index % 2 else 1
                if name in ('inc', 'dec'):
                    value, flags = arithmetic(a, 1, width, name == 'dec')
                    flags = (flags & ~1) | (incoming & 1)
                    opcode = 0xfe+(width != 8)
                elif name == 'neg':
                    value, flags = arithmetic(0, a, width, True)
                    opcode = 0xf6+(width != 8)
                else:
                    value, flags = (~a) & mask, incoming
                    opcode = 0xf6+(width != 8)
                add(f'{name}{width}_{index}', width, a, 0,
                    prefix+bytes([opcode, 0xc0 | (ext << 3)]), value, flags, incoming=incoming)
        for index, b in enumerate([0, mask]):
            add(f'mov{width}_{index}', width, sign, b, prefix+bytes([0x88+(width != 8), 0xc8]),
                b, ARITH, incoming=ARITH)
        # Valid unsigned division only. Divide-error delivery is deliberately excluded.
        for index, (dividend, divisor) in enumerate([(0, 1), (mask, 1), (mask-1, mask), (mask*3+1, 7), (mask*mask, mask)]):
            q, rem = divmod(dividend, divisor)
            assert q <= mask
            initial_a = (0x13570000 | dividend) if width == 8 else merge(0x13579bdf, dividend, width)
            initial_d = SENTINEL_DX if width == 8 else merge(0xc0deface, dividend >> width, width)
            expected_a = merge(initial_a, (rem << 8) | q, 16) if width == 8 else merge(initial_a, q, width)
            expected_d = initial_d if width == 8 else merge(initial_d, rem, width)
            result.append(dict(name=f'div{width}_{index}', width=width, eax=initial_a,
                               ecx=merge(0x2468ace0, divisor, width), edx=initial_d,
                               code=(prefix+bytes([0xf6+(width != 8), 0xf1])).hex(),
                               expected_eax=expected_a, expected_edx=expected_d,
                               flags=0, flags_mask=DF, incoming_flags=0x202))
    return result


class ComBuilder:
    def __init__(self):
        self.code = bytearray()
        self.labels = {}
        self.fixups = []
        self.data = []

    def emit(self, data):
        self.code.extend(data)

    def word(self, value):
        self.emit(int(value).to_bytes(2, 'little'))

    def mov32(self, register, value):
        self.emit(bytes([0x66, 0xb8+register])+int(value).to_bytes(4, 'little'))

    def label(self, name):
        assert name not in self.labels
        self.labels[name] = len(self.code)

    def ref(self, name, relative=False):
        self.fixups.append((len(self.code), name, relative))
        self.word(0)

    def branch(self, name, conditional=False):
        self.emit(bytes.fromhex('0f85') if conditional else bytes([0xe9]))
        self.ref(name, True)

    def puts(self, text):
        name = f'text_{len(self.data)}'
        self.data.append((name, text.encode('ascii')+b'$'))
        self.emit(bytes([0xba]))
        self.ref(name)
        self.emit(bytes.fromhex('b409cd21'))

    def cmp32(self, register, expected):
        self.emit(bytes([0x66, 0x81, 0xf8+register])+int(expected).to_bytes(4, 'little'))

    def finish(self):
        for name, data in self.data:
            self.label(name)
            self.emit(data)
        self.label('fail_count')
        self.word(0)
        assert len(self.code) <= 0xff00, f'COM size {len(self.code)} exceeds DOS limit'
        for pos, name, relative in self.fixups:
            value = self.labels[name]-pos-2 if relative else self.labels[name]+0x100
            self.code[pos:pos+2] = (value & 0xffff).to_bytes(2, 'little')
        return bytes(self.code)


def build(tests):
    b = ComBuilder()
    b.puts('PIL-DOSBOX CPU CHECK v1\r\n')
    for index, case in enumerate(tests, 1):
        tag = f'{index:03d} {case["name"]}'
        fail, done = f'fail_{index}', f'done_{index}'
        for reg, value in [(0, case['eax']), (1, case['ecx']), (2, case['edx']),
                           (3, 0xdead0000), (6, 0x11223344), (7, 0x55667788)]:
            b.mov32(reg, value)
        b.emit(bytes([0x68]))
        b.word(case['incoming_flags'])
        b.emit(bytes([0x9d]))  # POPF; data/flags setup is identical in both engines.
        b.emit(bytes.fromhex(case['code']))
        b.emit(bytes.fromhex('9c5b'))  # PUSHF; POP BX, before any flag-changing checks.
        for reg, expected in [(0, case['expected_eax']), (1, case['ecx']),
                              (2, case['expected_edx']), (6, 0x11223344), (7, 0x55667788)]:
            b.cmp32(reg, expected)
            b.branch(fail, True)
        b.emit(bytes.fromhex('81e3'))  # AND BX, defined-flag mask.
        b.word(case['flags_mask'])
        b.emit(bytes.fromhex('81fb'))  # CMP BX, expected defined/preserved bits.
        b.word(case['flags'] & case['flags_mask'])
        b.branch(fail, True)
        b.puts('PASS '+tag+'\r\n')
        b.branch(done)
        b.label(fail)
        b.puts('FAIL '+tag+'\r\n')
        b.emit(bytes.fromhex('ff06'))
        b.ref('fail_count')  # INC word [fail_count]
        b.label(done)
    b.puts(f'COMPLETE {len(tests)} CASES\r\n')
    b.emit(bytes.fromhex('a1'))
    b.ref('fail_count')
    b.emit(bytes.fromhex('09c0'))  # OR AX,AX
    b.emit(bytes.fromhex('7405b8014ccd21b8004ccd21'))
    return b.finish()


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def validate_output(text, tests, engine):
    rows = re.findall(r'^(PASS|FAIL) (\d{3}) (\S+)$', text.replace('\r', ''), re.M)
    expected = [(f'{i:03d}', c['name']) for i, c in enumerate(tests, 1)]
    actual = [(n, name) for _, n, name in rows]
    errors = []
    if actual != expected:
        errors.append(f'{engine}: missing, duplicated or out-of-order test rows')
    errors.extend(f'{engine}: FAIL {n} {name}' for status, n, name in rows if status == 'FAIL')
    if f'COMPLETE {len(tests)} CASES' not in text:
        errors.append(f'{engine}: completion marker missing')
    return dict(passed=sum(s == 'PASS' for s, _, _ in rows), failed=sum(s == 'FAIL' for s, _, _ in rows), errors=errors)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, required=True)
    parser.add_argument('--dosbox', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--timeout', type=int, default=90)
    parser.add_argument('--negative-control', action='store_true',
                        help='Deliberately corrupt the first expected result; both engines must fail it')
    args = parser.parse_args()
    directory = args.out.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    tests = cases()
    if args.negative_control:
        tests[0]['expected_eax'] ^= 1
    guest = directory/'CPUTEST.COM'
    guest.write_bytes(build(tests))
    (directory/'cases.json').write_text(json.dumps(tests, indent=2)+'\n', encoding='utf-8')
    config = directory/'dosbox.conf'
    config.write_text('[sdl]\nfullscreen=false\n[dosbox]\nmemsize=16\n[cpu]\ncore=normal\ncputype=386\ncycles=fixed 10000\n[mixer]\nnosound=true\n[autoexec]\n@echo off\nmount c "'+str(directory)+'"\nc:\nCPUTEST.COM > DBOX.TXT\nexit\n', encoding='utf-8')
    started = time.time()
    initial_hashes = dict(com=sha(guest), pil=sha(args.engine), dosbox=sha(args.dosbox))
    run_options = dict(timeout=args.timeout, capture_output=True, text=True, errors='replace')
    if hasattr(subprocess, 'CREATE_NO_WINDOW'):
        run_options['creationflags'] = subprocess.CREATE_NO_WINDOW
    dosbox = subprocess.run([str(args.dosbox.resolve()), '-conf', str(config), '-silent', '-fastlaunch', '-time-limit', str(args.timeout)], cwd=directory, **run_options)
    (directory/'dosbox-host.txt').write_text(dosbox.stdout+'\n'+dosbox.stderr, encoding='utf-8')
    box_file = directory/'DBOX.TXT'
    box_text = box_file.read_text(encoding='ascii') if box_file.exists() else ''
    pil = subprocess.run([str(args.engine.resolve()), '--dos-probe', '--steps', '100000', str(guest)], **run_options)
    (directory/'pil-report.txt').write_text(pil.stdout+'\n'+pil.stderr, encoding='utf-8')
    match = re.search(r'^OUTPUT \(escaped bytes\): ?(.*)$', pil.stdout, re.M)
    pil_text = bytes(match.group(1), 'ascii').decode('unicode_escape') if match else ''
    (directory/'pil-output.txt').write_text(pil_text, encoding='ascii')
    box_result = validate_output(box_text, tests, 'DOSBox-X')
    pil_result = validate_output(pil_text, tests, 'PIL')
    errors = box_result['errors']+pil_result['errors']
    final_hashes = dict(com=sha(guest), pil=sha(args.engine), dosbox=sha(args.dosbox))
    if initial_hashes != final_hashes:
        errors.append('A tested executable or COM file changed during this run')
    if box_text.replace('\r', '') != pil_text.replace('\r', ''):
        errors.append('CPU test output differs between DOSBox-X and PIL')
    if dosbox.returncode != 0 or pil.returncode != 0 or 'STOP Exit(0)' not in pil.stdout:
        errors.append('An engine failed, timed out, or PIL did not reach successful DOS exit')
    report = dict(kind='original self-checking DOS COM emulator comparison',
                  hardware_or_external_suite=False, negative_control=args.negative_control,
                  tests=len(tests), com_bytes=guest.stat().st_size,
                  initial_hashes=initial_hashes,
                  com_sha256=sha(guest), dosbox=str(args.dosbox.resolve()), dosbox_sha256=sha(args.dosbox),
                  pil=str(args.engine.resolve()), pil_sha256=sha(args.engine),
                  dosbox_cpu='386', dosbox_core='normal', dosbox_result=box_result,
                  pil_result=pil_result, elapsed_seconds=time.time()-started, errors=errors,
                  excluded=['divide-error delivery', 'undefined flag values', 'BIOS/devices', 'protected/V86 mode', 'cycle timing'])
    (directory/'result.json').write_text(json.dumps(report, indent=2)+'\n', encoding='utf-8')
    print(json.dumps(report, indent=2))
    raise SystemExit(bool(errors))


if __name__ == '__main__':
    main()
