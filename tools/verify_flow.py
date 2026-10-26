"""Independent description-level Jcc/context oracle. Does not execute Rust."""
from verify_descriptions import CONDITIONS, FlowFault, interpret, tokens


def condition(name, f):
    c, p, z, s, o = (bool(f & (1 << b)) for b in (0, 2, 6, 7, 11))
    return {"o": o, "no": not o, "b": c, "ae": not c, "e": z, "ne": not z,
            "be": c or z, "a": not c and not z, "s": s, "ns": not s,
            "p": p, "np": not p, "l": s != o, "ge": s == o,
            "le": z or s != o, "g": not z and s == o}[name]


def flag_word(bits):
    return 0x202 | sum(((bits >> i) & 1) << b
                       for i, b in enumerate((0, 2, 4, 6, 7, 11)))


def main():
    count = 0
    for name in CONDITIONS:
        for width in (16, 32):
            code = tokens("j" + name, width)
            for bits in range(64):
                flags = flag_word(bits)
                for next_ip, disp, disp_bits, limit in (
                    (0x12345678, -128, 8, 0xffffffff),
                    (0, -1, 8, 0xffffffff),
                    (0xffffffff, 1, 8, 0xffffffff),
                    (0x100, 127, 8, 0x100),
                    (0x100, 128, 8, 0xffffffff),
                    (0x100, 1, width, 0x101),
                ):
                    flow = dict(next=next_ip, displacement=disp,
                                bits=disp_bits, limit=limit)
                    target = (next_ip + disp) % (1 << width)
                    take = condition(name, flags)
                    invalid = not -(1 << (disp_bits - 1)) <= disp < (1 << (disp_bits - 1))
                    try:
                        got = interpret(code, 7, 9, width, flags, 16,
                                        flow=flow, current_eip=0x100)
                    except ValueError:
                        assert invalid
                    except FlowFault as fault:
                        assert not invalid and take and target > limit
                        assert fault.eip == 0x100 and fault.error_code == 0
                    else:
                        assert not invalid and not (take and target > limit)
                        assert got == (7, flags, 16, target if take else next_ip)
                    count += 1
    # Force a limit fault after staged register and flag changes.
    text = """const replacement 32 42
write dst replacement
const clear 1 0
flag CF clear
undef_flag AF
const target 32 257
const take 1 1
branch_if target take
preserve_flags remaining
retire"""
    code = [line.split() for line in text.splitlines()]
    try:
        interpret(code, 7, 9, 32, 0x213, 1,
                  flow=dict(next=0x106, displacement=0, bits=32, limit=256),
                  current_eip=0x100)
    except FlowFault as fault:
        assert fault.eip == 0x100
    else:
        raise AssertionError("expected a staged-effect branch fault")
    print(f"{count:,} Jcc/context cases plus staged-fault check passed.")
    print("Rust compilation and exception delivery are separate checks.")


if __name__ == "__main__":
    main()
