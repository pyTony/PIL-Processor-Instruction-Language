"""Independent flat-RAM description oracle. Does not execute Rust."""
from verify_descriptions import (
    ARITHMETIC_MASK, BASE, FlowFault, MemoryFault, interpret, oracle, tokens,
)


def main():
    count = 0
    variants = [(op, width) for op in ("load_ram", "store_ram", "add_ram")
                for width in (8, 16, 32)] + [("store_then_fault", 32)]
    for op, width in variants:
        if op == "store_then_fault":
            text = (BASE / "store_ram.pil").read_text().replace(
                "preserve_flags all",
                "const t 32 1280\nconst c 1 1\nbranch_if t c\npreserve_flags all")
            text = text.replace("$W", "32")
            code = [line.split("#", 1)[0].split() for line in text.splitlines()
                    if line.split("#", 1)[0].strip()]
        else:
            code = tokens(op, width)
        for address in (0, 1, 4, 5, 7, 8, 0xffffffff):
            for value in (0, 1, 0x7fffffff, 0xffffffff):
                for protected in (False, True):
                    ram = bytearray([255] * 8)
                    before = ram[:]
                    readonly = ((4, 5),) if protected else ()
                    size, mask = width // 8, (1 << width) - 1
                    outside = address + size > 8
                    write = op != "load_ram"
                    blocked = protected and write and address < 5 and address + size > 4
                    try:
                        got = interpret(
                            code, 0xabcdff99, value, width, 0x203,
                            ARITHMETIC_MASK, current_eip=0x400, ram=ram,
                            address=address, readonly=readonly,
                            flow=dict(next=0x402, displacement=0, bits=32, limit=0x4ff),
                        )
                    except MemoryFault as fault:
                        assert outside or blocked
                        # RMW first performs a read, so out-of-range faults are reads.
                        expected_write = write and not (outside and op == "add_ram")
                        assert (fault.eip, fault.address, fault.bytes, fault.write) == (
                            0x400, address, size, expected_write)
                        assert ram == before
                    except FlowFault as fault:
                        assert op == "store_then_fault" and not outside and not blocked
                        assert fault.eip == 0x400 and ram == before
                    else:
                        assert not outside and not blocked and op != "store_then_fault"
                        flags, undefined, destination = 0x203, ARITHMETIC_MASK, 0xabcdff99
                        expected = before[:]
                        if op == "load_ram":
                            destination = (destination & ~mask) | mask
                        elif op == "store_ram":
                            expected[address:address + size] = (value & mask).to_bytes(size, "little")
                        else:
                            result, flags, undefined = oracle("add", mask, value, width, 0x203)
                            expected[address:address + size] = result.to_bytes(size, "little")
                        assert got == (destination, flags, undefined, 0x402)
                        assert ram == expected
                    count += 1
    print(f"{count} flat-RAM description/fault cases passed.")
    print("These are emulator bounds/protection faults, not architectural x86 #PF.")


if __name__ == "__main__":
    main()
