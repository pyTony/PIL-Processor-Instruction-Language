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


def signed_value(v, width):
    sign = 1 << (width - 1)
    v &= (1 << width) - 1
    return v - (1 << width) if (v & sign) else v


def cases():
    result = []
    def add(name, width, a, b, opcode, expected, flags, flag_mask=ARITH | DF, incoming=0, expected_ecx=None):
        initial_a = merge(0x13579bdf, a, width)
        initial_b = merge(0x2468ace0, b, width)
        exp_b = initial_b if expected_ecx is None else merge(initial_b, expected_ecx, width)
        result.append(dict(name=name, width=width, eax=initial_a, ecx=initial_b,
                           edx=SENTINEL_DX, code=opcode.hex(),
                           expected_eax=merge(initial_a, expected, width),
                           expected_ecx=exp_b,
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
            initial_b = merge(0x2468ace0, divisor, width)
            result.append(dict(name=f'div{width}_{index}', width=width, eax=initial_a,
                               ecx=initial_b, edx=initial_d,
                               code=(prefix+bytes([0xf6+(width != 8), 0xf1])).hex(),
                               expected_eax=expected_a, expected_ecx=initial_b,
                               expected_edx=expected_d,
                               flags=0, flags_mask=DF, incoming_flags=0x202))
        # Shifts (SHL, SHR, SAR): count=0 (CL), count=1 (D0/D1), count=4 (CL)
        for name, ext in [('shl', 4), ('shr', 5), ('sar', 7)]:
            a0 = sign | 3
            add(f'{name}{width}_cl0', width, a0, 0,
                prefix+bytes([0xd2+(width != 8), 0xc0 | (ext << 3)]),
                a0, ARITH, flag_mask=ARITH | DF, incoming=ARITH)
            if name == 'shl':
                r1 = (a0 << 1) & mask
                cf1 = (a0 >> (width - 1)) & 1
                of1 = (((r1 >> (width - 1)) & 1) ^ cf1) << 11
            elif name == 'shr':
                r1 = a0 >> 1
                cf1 = a0 & 1
                of1 = ((a0 >> (width - 1)) & 1) << 11
            else:
                r1 = (a0 >> 1) | (a0 & sign)
                cf1 = a0 & 1
                of1 = 0
            add(f'{name}{width}_1', width, a0, 1,
                prefix+bytes([0xd0+(width != 8), 0xc0 | (ext << 3)]),
                r1, parity_flags(r1, width) | cf1 | of1,
                flag_mask=(ARITH & ~16) | DF, incoming=1)
            a4 = sign | 0x5a
            if name == 'shl':
                r4 = (a4 << 4) & mask
                cf4 = (a4 >> (width - 4)) & 1
            elif name == 'shr':
                r4 = a4 >> 4
                cf4 = (a4 >> 3) & 1
            else:
                r4 = (a4 >> 4) | ((mask ^ (mask >> 4)) if (a4 & sign) else 0)
                cf4 = (a4 >> 3) & 1
            add(f'{name}{width}_cl4', width, a4, 4,
                prefix+bytes([0xd2+(width != 8), 0xc0 | (ext << 3)]),
                r4, parity_flags(r4, width) | cf4,
                flag_mask=0x0c5 | DF, incoming=ARITH)
            if name == 'shl':
                ri3 = (a4 << 3) & mask
                cfi3 = (a4 >> (width - 3)) & 1
            elif name == 'shr':
                ri3 = a4 >> 3
                cfi3 = (a4 >> 2) & 1
            else:
                ri3 = (a4 >> 3) | ((mask ^ (mask >> 3)) if (a4 & sign) else 0)
                cfi3 = (a4 >> 2) & 1
            add(f'{name}{width}_imm3', width, a4, 0,
                prefix+bytes([0xc0+(width != 8), 0xc0 | (ext << 3), 3]),
                ri3, parity_flags(ri3, width) | cfi3,
                flag_mask=0x0c5 | DF, incoming=ARITH)
        # Rotates (ROL, ROR): count=1 (D0/D1), count=4 (CL), count=3 (C0/C1 imm8)
        ar = sign | 1
        rol1 = ((ar << 1) & mask) | ((ar >> (width - 1)) & 1)
        rol1_cf = rol1 & 1
        rol1_of = (((rol1 >> (width - 1)) & 1) ^ rol1_cf) << 11
        add(f'rol{width}_1', width, ar, 1,
            prefix+bytes([0xd0+(width != 8), 0xc0]),
            rol1, (0x0d4 & ~1) | rol1_cf | rol1_of,
            flag_mask=ARITH | DF, incoming=0x0d4)
        ror1 = (ar >> 1) | ((ar & 1) << (width - 1))
        ror1_cf = (ror1 >> (width - 1)) & 1
        ror1_of = (((ror1 >> (width - 1)) & 1) ^ ((ror1 >> (width - 2)) & 1)) << 11
        add(f'ror{width}_1', width, ar, 1,
            prefix+bytes([0xd0+(width != 8), 0xc8]),
            ror1, (0x0d4 & ~1) | ror1_cf | ror1_of,
            flag_mask=ARITH | DF, incoming=0x0d4)
        ar4 = sign | 0x35
        rol4 = ((ar4 << 4) & mask) | ((ar4 >> (width - 4)) & 0x0f)
        add(f'rol{width}_cl4', width, ar4, 4,
            prefix+bytes([0xd2+(width != 8), 0xc0]),
            rol4, (0x0d4 & ~1) | (rol4 & 1),
            flag_mask=(ARITH & ~0x800) | DF, incoming=0x0d4)
        ror4 = (ar4 >> 4) | ((ar4 & 0x0f) << (width - 4))
        add(f'ror{width}_cl4', width, ar4, 4,
            prefix+bytes([0xd2+(width != 8), 0xc8]),
            ror4, (0x0d4 & ~1) | ((ror4 >> (width - 1)) & 1),
            flag_mask=(ARITH & ~0x800) | DF, incoming=0x0d4)
        rol3 = ((ar4 << 3) & mask) | ((ar4 >> (width - 3)) & 0x07)
        add(f'rol{width}_imm3', width, ar4, 0,
            prefix+bytes([0xc0+(width != 8), 0xc0, 3]),
            rol3, (0x0d4 & ~1) | (rol3 & 1),
            flag_mask=(ARITH & ~0x800) | DF, incoming=0x0d4)
        ror3 = (ar4 >> 3) | ((ar4 & 0x07) << (width - 3))
        add(f'ror{width}_imm3', width, ar4, 0,
            prefix+bytes([0xc0+(width != 8), 0xc8, 3]),
            ror3, (0x0d4 & ~1) | ((ror3 >> (width - 1)) & 1),
            flag_mask=(ARITH & ~0x800) | DF, incoming=0x0d4)
        # XCHG register forms (86 C8 for 8-bit, [66] 91 for 16/32-bit)
        for index, (a, b) in enumerate([(0x12, 0xfe), (sign, sign - 1)]):
            xchg_code = bytes([0x86, 0xc8]) if width == 8 else prefix + bytes([0x91])
            add(f'xchg{width}_{index}', width, a, b, xchg_code,
                b, ARITH, incoming=ARITH, expected_ecx=a)
    # Extensions and sign conversions (MOVZX, MOVSX, CBW/CWDE, CWD/CDQ)
    for dst_w, pfx in [(16, b''), (32, b'\x66')]:
        for idx, bval in enumerate([0x7f, 0x80]):
            add(f'movzx_b{dst_w}_{idx}', dst_w, 0xffff, bval, pfx + bytes([0x0f, 0xb6, 0xc1]),
                bval, ARITH, incoming=ARITH)
            sext = bval if bval < 0x80 else ((1 << dst_w) - 0x100 + bval)
            add(f'movsx_b{dst_w}_{idx}', dst_w, 0, bval, pfx + bytes([0x0f, 0xbe, 0xc1]),
                sext, ARITH, incoming=ARITH)
    for idx, wval in enumerate([0x7fff, 0x8000]):
        add(f'movzx_w32_{idx}', 32, 0xffffffff, wval, bytes([0x66, 0x0f, 0xb7, 0xc1]),
            wval, ARITH, incoming=ARITH)
        sext = wval if wval < 0x8000 else (0xffff0000 | wval)
        add(f'movsx_w32_{idx}', 32, 0, wval, bytes([0x66, 0x0f, 0xbf, 0xc1]),
            sext, ARITH, incoming=ARITH)
    for idx, aval in enumerate([0x7f, 0x80]):
        sext16 = aval if aval < 0x80 else (0xff00 | aval)
        add(f'cbw_{idx}', 16, aval, 0, bytes([0x98]), sext16, ARITH, incoming=ARITH)
    for idx, aval in enumerate([0x7fff, 0x8000]):
        sext32 = aval if aval < 0x8000 else (0xffff0000 | aval)
        add(f'cwde_{idx}', 32, aval, 0, bytes([0x66, 0x98]), sext32, ARITH, incoming=ARITH)
    for width, pfx, mname in [(16, b'', 'cwd'), (32, b'\x66', 'cdq')]:
        mask = (1 << width) - 1
        sign = 1 << (width - 1)
        for idx, aval in enumerate([sign - 1, sign]):
            exp_dx = mask if (aval & sign) else 0
            init_a = merge(0x13579bdf, aval, width)
            init_b = 0x2468ace0
            init_d = SENTINEL_DX
            result.append(dict(name=f'{mname}_{idx}', width=width, eax=init_a, ecx=init_b,
                               edx=init_d, code=(pfx + bytes([0x99])).hex(),
                               expected_eax=init_a, expected_ecx=init_b,
                               expected_edx=merge(init_d, exp_dx, width),
                               flags=ARITH, flags_mask=ARITH | DF, incoming_flags=0x202 | ARITH))
    # Carry, direction, and AH flag transfers (CLC, STC, CMC, STD, CLD, LAHF, SAHF)
    for carry in (0, 1):
        base_f = (ARITH & ~1) | carry
        add(f'clc_c{carry}', 16, 0x1234, 0, bytes([0xf8]), 0x1234, ARITH & ~1, incoming=base_f)
        add(f'stc_c{carry}', 16, 0x1234, 0, bytes([0xf9]), 0x1234, ARITH | 1, incoming=base_f)
        add(f'cmc_c{carry}', 16, 0x1234, 0, bytes([0xf5]), 0x1234, (ARITH & ~1) | (carry ^ 1), incoming=base_f)
    add('std_arith0', 16, 0x1234, 0, bytes([0xfd]), 0x1234, DF, flag_mask=ARITH | DF, incoming=0)
    add('std_arith1', 16, 0x1234, 0, bytes([0xfd]), 0x1234, ARITH | DF, flag_mask=ARITH | DF, incoming=ARITH)
    add('cld_arith0', 16, 0x1234, 0, bytes([0xfc]), 0x1234, 0, flag_mask=ARITH | DF, incoming=DF)
    add('cld_arith1', 16, 0x1234, 0, bytes([0xfc]), 0x1234, ARITH, flag_mask=ARITH | DF, incoming=ARITH | DF)
    add('lahf_set', 16, 0x0055, 0, bytes([0x9f]), (0xd7 << 8) | 0x55, 0x0d5, incoming=0x0d5)
    add('sahf_clear', 16, 0x00aa, 0, bytes([0x9e]), 0x00aa, 0x800, incoming=ARITH)
    # Signed truncated IMUL (0F AF C1 and 6B C1 imm8) in 16-bit and 32-bit
    for width, pfx in [(16, b''), (32, b'\x66')]:
        mask = (1 << width) - 1
        sign = 1 << (width - 1)
        for idx, (a, b) in enumerate([(3, (-5) & mask), (7, 9), (sign - 1, 2), (sign, 2)]):
            full = signed_value(a, width) * signed_value(b, width)
            trunc = full & mask
            overflow = full != signed_value(trunc, width)
            cf_of = (0x801 if overflow else 0)
            add(f'imul{width}_{idx}', width, a, b, pfx + bytes([0x0f, 0xaf, 0xc1]),
                trunc, cf_of, flag_mask=0x801 | DF, incoming=ARITH)
        for idx, (b, imm) in enumerate([(10, (-3) & 0xff), (sign - 1, 2)]):
            full = signed_value(b, width) * signed_value(imm, 8)
            trunc = full & mask
            overflow = full != signed_value(trunc, width)
            cf_of = (0x801 if overflow else 0)
            add(f'imul_imm{width}_{idx}', width, 0, b, pfx + bytes([0x6b, 0xc1, imm]),
                trunc, cf_of, flag_mask=0x801 | DF, incoming=ARITH)
    # SETcc AL and LEA
    for cname, cc, f_true, f_false in [
        ('setz', 0x94, 0x040, 0x000),
        ('setnz', 0x95, 0x000, 0x040),
        ('setb', 0x92, 0x001, 0x000),
        ('setl', 0x9c, 0x080, 0x880),
        ('setg', 0x9f, 0x000, 0x040),
    ]:
        add(f'{cname}_1', 8, 0xaa, 0, bytes([0x0f, cc, 0xc0]), 1, f_true, incoming=f_true)
        add(f'{cname}_0', 8, 0xaa, 0, bytes([0x0f, cc, 0xc0]), 0, f_false, incoming=f_false)
    # LEA AX, [SI+0x1234] (SI=0x3344), LEA AX, [DI+0x1234] (DI=0x7788), and 32-bit LEA EAX
    add('lea16_si', 16, 0, 0, bytes([0x8d, 0x84, 0x34, 0x12]),
        (0x3344 + 0x1234) & 0xffff, ARITH, incoming=ARITH)
    add('lea16_di', 16, 0, 0, bytes([0x8d, 0x85, 0x34, 0x12]),
        (0x7788 + 0x1234) & 0xffff, ARITH, incoming=ARITH)
    add('lea32_ecxdx', 32, 0, 0x00010000, bytes([0x67, 0x66, 0x8d, 0x84, 0x11, 0x34, 0x12, 0x00, 0x00]),
        (0x00010000 + SENTINEL_DX + 0x1234) & 0xffffffff, ARITH, incoming=ARITH)
    add('lea32_esiedi', 32, 0, 0, bytes([0x67, 0x66, 0x8d, 0x84, 0x3e, 0x10, 0x20, 0x00, 0x00]),
        (0x11223344 + 0x55667788 + 0x2010) & 0xffffffff, ARITH, incoming=ARITH)
    # Unsigned MUL r/m (F6/F7 /4) with CL/CX/ECX; CF=OF report a non-zero upper half.
    for width, pfx in [(8, b''), (16, b''), (32, b'\x66')]:
        mask = (1 << width) - 1
        sign = 1 << (width - 1)
        for idx, (a, b) in enumerate([(0, mask), (3, 5), (sign, 2), (mask, mask)]):
            product = a * b
            initial_a = merge(0x13579bdf, a, width)
            initial_b = merge(0x2468ace0, b, width)
            if width == 8:
                expected_a, expected_d = merge(initial_a, product, 16), SENTINEL_DX
            else:
                expected_a = merge(initial_a, product & mask, width)
                expected_d = merge(SENTINEL_DX, product >> width, width)
            result.append(dict(name=f'mul{width}_{idx}', width=width, eax=initial_a, ecx=initial_b,
                               edx=SENTINEL_DX, code=(pfx+bytes([0xf6+(width != 8), 0xe1])).hex(),
                               expected_eax=expected_a, expected_ecx=initial_b, expected_edx=expected_d,
                               flags=0x801 if product >> width else 0, flags_mask=0x801 | DF,
                               incoming_flags=0x202 | ARITH))
    # Rotates through carry (RCL /2, RCR /3): count 1 (D0/D1) with CF 0 and 1, CL=9 and imm8=3.
    for width, pfx in [(8, b''), (16, b''), (32, b'\x66')]:
        mask = (1 << width) - 1
        for name, ext in [('rcl', 2), ('rcr', 3)]:
            a = (1 << (width - 1)) | 0x35
            # Four forms per width keep CPUTEST.COM inside the 64 KB COM limit.
            for label, count, carry, code in [('1', 1, 0, bytes([0xd0 + (width != 8), 0xc0 | (ext << 3)])),
                                              ('1', 1, 1, bytes([0xd0 + (width != 8), 0xc0 | (ext << 3)])),
                                              ('cl9', 9, 1, bytes([0xd2 + (width != 8), 0xc0 | (ext << 3)])),
                                              ('imm3', 3, 0, bytes([0xc0 + (width != 8), 0xc0 | (ext << 3), 3]))]:
                turns = (count & 31) % (width + 1)
                v = (carry << width) | a
                ring = (1 << (width + 1)) - 1
                if name == 'rcl':
                    r = ((v << turns) | (v >> (width + 1 - turns))) & ring
                else:
                    r = ((v >> turns) | (v << (width + 1 - turns))) & ring
                value, cf = r & mask, r >> width
                flags = 0x0d4 | cf
                flag_mask = 1 | DF
                if count == 1:
                    if name == 'rcl':
                        of = ((value >> (width - 1)) & 1) ^ cf
                    else:
                        of = ((a >> (width - 1)) & 1) ^ carry
                    flags |= of << 11
                    flag_mask |= 0x800
                add(f'{name}{width}_{label}_c{carry}', width, a, count if label != '1' else 0,
                    pfx + code, value, flags, flag_mask=flag_mask, incoming=0x0d4 | carry)
    # DAA/DAS from the Intel SDM pseudocode; OF is undefined.
    def decimal_adjust(al, cf, af, subtract):
        old_al, old_cf, cf = al, cf, 0
        if (al & 0x0f) > 9 or af:
            borrow = al < 6 if subtract else al + 6 > 0xff
            al = (al - 6 if subtract else al + 6) & 0xff
            cf, af = old_cf | borrow, 1
        else:
            af = 0
        if old_al > 0x99 or old_cf:
            al = (al - 0x60 if subtract else al + 0x60) & 0xff
            cf = 1
        return al, (parity_flags(al, 8) & ~0x11) | cf | (af << 4)
    for name, opcode in [('daa', 0x27), ('das', 0x2f)]:
        for index, (al, cf, af) in enumerate([(0x0f, 0, 0), (0x9a, 0, 0), (0x99, 1, 0), (0x45, 0, 1), (0x05, 1, 1)]):
            value, flags = decimal_adjust(al, cf, af, name == 'das')
            add(f'{name}_{index}', 8, al, 0, bytes([opcode]), value, flags,
                flag_mask=(ARITH & ~0x800) | DF, incoming=cf | (af << 4))
    # PUSHA/POPA round trip, and the first popped slot is DI/EDI.
    add('pusha16_popa', 16, 0x1234, 0x5678, bytes.fromhex('6031c031c961'), 0x1234, 0, flag_mask=DF)
    add('pusha16_order', 16, 0x1234, 0, bytes.fromhex('605883c40e'), 0x7788, 0, flag_mask=DF)
    add('pushad_popad', 32, 0x12345678, 0x9abcdef0, bytes.fromhex('66606631c06631c96661'), 0x12345678, 0, flag_mask=DF)
    add('pushad_order', 32, 0x12345678, 0, bytes.fromhex('6660665883c41c'), 0x55667788, 0, flag_mask=DF)
    # LOOP/LOOPE/LOOPNE jumping to themselves: CX counts down, flags stay.
    for name, opcode, incoming, after in [('loop', 0xe2, 0, 0), ('loopne_nz', 0xe0, 0, 0),
                                          ('loopne_z', 0xe0, 0x40, 2), ('loope_z', 0xe1, 0x40, 0),
                                          ('loope_nz', 0xe1, 0, 2)]:
        add(name, 16, 0, 3, bytes([opcode, 0xfe]), 0, incoming, incoming=incoming, expected_ecx=after)
    add('loop_a32', 32, 0, 2, bytes([0x67, 0xe2, 0xfd]), 0, ARITH, incoming=ARITH, expected_ecx=0)
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
        # Identical strings share one copy so the CPU suite fits in one COM.
        data = text.encode('ascii')+b'$'
        name = next((n for n, d in self.data if d == data), None)
        if name is None:
            name = f'text_{len(self.data)}'
            self.data.append((name, data))
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
    positions = []
    for index, case in enumerate(tests, 1):
        tag = f'{index:03d} {case["name"]}'
        fail, done = f'fail_{index}', f'done_{index}'
        entry_pos = len(b.code)
        for reg, value in [(0, case['eax']), (1, case['ecx']), (2, case['edx']),
                           (3, 0xdead0000), (6, 0x11223344), (7, 0x55667788)]:
            b.mov32(reg, value)
        b.emit(bytes([0x68]))
        b.word(case['incoming_flags'])
        b.emit(bytes([0x9d]))  # POPF; data/flags setup is identical in both engines.
        test_pos = len(b.code)
        b.emit(bytes.fromhex(case['code']))
        b.emit(bytes.fromhex('9c5bfc'))  # PUSHF; POP BX; CLD so DF=1 tests never affect DOS INT 21h.
        check_pos = len(b.code)
        for reg, expected in [(0, case['expected_eax']), (1, case.get('expected_ecx', case['ecx'])),
                              (2, case['expected_edx']), (6, 0x11223344), (7, 0x55667788)]:
            b.cmp32(reg, expected)
            b.branch(fail, True)
        b.emit(bytes.fromhex('81e3'))  # AND BX, defined-flag mask.
        b.word(case['flags_mask'])
        b.emit(bytes.fromhex('81fb'))  # CMP BX, expected defined/preserved bits.
        b.word(case['flags'] & case['flags_mask'])
        b.branch(fail, True)
        b.puts('PASS ')
        b.puts(tag+'\r\n')
        b.branch(done)
        b.label(fail)
        b.puts('FAIL ')
        b.puts(tag+'\r\n')
        b.emit(bytes.fromhex('ff06'))
        b.ref('fail_count')  # INC word [fail_count]
        b.label(done)
        positions.append((index, case, entry_pos, test_pos, check_pos, fail, done))
    b.puts(f'COMPLETE {len(tests)} CASES\r\n')
    b.emit(bytes.fromhex('a1'))
    b.ref('fail_count')
    b.emit(bytes.fromhex('09c0'))  # OR AX,AX
    b.emit(bytes.fromhex('7405b8014ccd21b8004ccd21'))
    com_bytes = b.finish()
    for index, case, entry_pos, test_pos, check_pos, fail, done in positions:
        case['index'] = index
        case['entry_eip'] = 0x100 + entry_pos
        case['test_eip'] = 0x100 + test_pos
        case['check_eip'] = 0x100 + check_pos
        case['fail_eip'] = 0x100 + b.labels[fail]
        case['done_eip'] = 0x100 + b.labels[done]
    return com_bytes


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
    parser.add_argument('--suite', choices=('cpu', 'console'), default='cpu')
    parser.add_argument('--timeout', type=int, default=90)
    parser.add_argument('--negative-control', action='store_true',
                        help='Deliberately corrupt the first expected result; both engines must fail it')
    args = parser.parse_args()
    directory = args.out.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    tests = cases()
    if args.negative_control:
        tests[0]['expected_eax'] ^= 1
    if args.suite == 'console':
        from verify_dos_console import build_console
        tests, guest_bytes = build_console(args.negative_control)
    else:
        guest_bytes = build(tests)
    guest = directory/'CPUTEST.COM'
    guest.write_bytes(guest_bytes)
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
                  hardware_or_external_suite=False, suite=args.suite, negative_control=args.negative_control,
                  tests=len(tests), com_bytes=guest.stat().st_size,
                  initial_hashes=initial_hashes,
                  com_sha256=sha(guest), dosbox=str(args.dosbox.resolve()), dosbox_sha256=sha(args.dosbox),
                  pil=str(args.engine.resolve()), pil_sha256=sha(args.engine),
                  dosbox_cpu='386', dosbox_core='normal', dosbox_result=box_result,
                  pil_result=pil_result, elapsed_seconds=time.time()-started, errors=errors,
                  excluded=['divide-error delivery', 'undefined flag values', 'full BIOS/device conformance', 'protected/V86 mode', 'cycle timing'])
    (directory/'result.json').write_text(json.dumps(report, indent=2)+'\n', encoding='utf-8')
    print(json.dumps(report, indent=2))
    raise SystemExit(bool(errors))


if __name__ == '__main__':
    main()
