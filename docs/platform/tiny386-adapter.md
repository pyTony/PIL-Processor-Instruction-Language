# Proposed Tiny386 device adapter

Research checked 26 October 2026. This is an interface design, not a completed
source import, licence audit or linked module.

## Reuse decision

Keep the PIL CPU and PC device implementation separate. Prefer selected
Tiny386 device sources compiled into a C static library for the first
embedded integration; shared linking can remain a desktop option.
Static linking is a technical choice, not a way to avoid obligations.

The Tiny386 top-level BSD-3-Clause licence permits source/binary
redistribution and modification with retained notices and its conditions.
The inspected VGA source has an MIT permission notice. Retain applicable
per-file notices and include them in binary-distribution documentation.

The README separately identifies optional FMOPL audio as LGPL and
SeaBIOS as LGPL-3. Keep firmware as separately tracked ROM artifacts;
retain its own licensing/source arrangements. Initially omit optional audio.
A separate linked module does not automatically exempt a combined distribution
from third-party licence conditions. Before importing code, pin an upstream
revision and inventory the exact chosen files and dependencies.

Sources:
- https://github.com/hchunhui/tiny386/blob/master/LICENSE
- https://github.com/hchunhui/tiny386/blob/master/vga.c
- https://github.com/hchunhui/tiny386#license

## Why an adapter is necessary

The inspected pc.h includes i386.h/amd64.h and aliases a CPU type.
Its existing PC integration is therefore not an independent device-only API.
Refactor the CPU-facing calls into callbacks while preserving device internals.
Keep those changes small and isolated for upstream updates.

Source: https://github.com/hchunhui/tiny386/blob/master/pc.h

## Ownership

| PIL CPU owns | PC module owns |
|---|---|
| Registers, selectors and cached descriptors | PIC, PIT, CMOS, keyboard controller |
| Instruction decode and execution | VGA, disk controller, DMA and ports |
| Segmentation, paging, privilege checks | Physical address map and ROM mappings |
| Guest exception delivery and interrupt eligibility | IRQ/NMI/reset/A20 signal state |
| Instruction/restart boundaries | Deterministic device deadlines and clock |

The CPU performs architectural translation before physical bus access.
The platform applies its A20 wiring policy at the physical-address boundary.
Mapping changes and DMA writes notify the CPU's future decode caches.
Do not return Tiny386 CPU structures or Rust references across FFI.

## Proposed ABI

Names below are new adapter entry points, not existing Tiny386 exports:

```c
typedef struct PilPc PilPc;
typedef struct {
    uint32_t value;
    uint32_t status;
} PilBusResult;

PilBusResult pil_pc_phys_read(PilPc *, uint32_t addr, uint8_t bytes);
uint32_t pil_pc_phys_write(PilPc *, uint32_t addr,
                          uint8_t bytes, uint32_t value);
PilBusResult pil_pc_io_read(PilPc *, uint16_t port, uint8_t bytes);
uint32_t pil_pc_io_write(PilPc *, uint16_t port,
                        uint8_t bytes, uint32_t value);
uint64_t pil_pc_next_deadline(PilPc *);
void pil_pc_advance_to(PilPc *, uint64_t guest_ticks);
bool pil_pc_irq_pending(PilPc *);
uint8_t pil_pc_acknowledge_irq(PilPc *);
```

The eventual header includes stdint.h/stdbool.h and an ABI version plus
create/destroy/reset/configuration operations. Bytes must be 1, 2 or 4;
invalid widths return a host-interface error. Such errors are not automatically
x86 #PF exceptions: architectural faults belong to CPU translation/checks.
Return data and status separately so valid all-ones device values remain valid.

Add callbacks for NMI/reset/A20 changes, physical mapping changes, DMA writes,
video dirty regions and disk operations. Define IRQ acknowledge as an effect;
testing the IRQ level must not acknowledge it. Keep pending interrupts while
the CPU masks them.

Choose one monotonic guest clock and document its units. Instruction budgets
are execution quotas, not an assertion of cycle accuracy. HLT may advance
to the next device deadline. Deliver events at architecturally legal
instruction or REP checkpoints.

For initial correctness use callbacks for bus accesses. Add a validated RAM
fast path later while retaining MMIO, ROM, DMA and code-invalidation handling.

## Integration acceptance

1. Test the adapter first with Tiny386's original CPU.
2. Record device traces for a small DOS workload.
3. Substitute the PIL CPU while retaining machine configuration and firmware.
4. Compare I/O transactions, interrupt acknowledgement and visible results.
5. Test DOS boot, then Windows standard mode, then enhanced mode.

This reuses existing PC hardware work while keeping both PIL execution
backends independent of the selected device implementation.
