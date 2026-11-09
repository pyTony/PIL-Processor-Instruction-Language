# Guarded block execution upgrade

Generated blocks retain their maximum of 16 instructions. The dispatcher can
follow observed branches, calls and returns across blocks for at most 256
instructions per invocation, bounded by the caller's remaining step budget.
Single-instruction blocks also use this path.

Ordinary real-mode segment relocation is accepted when every cache remains an
ordinary real-mode cache and all other machine state matches. Code bytes are
checked in the live code segment against its limit and the allowed executable
windows. Paging, changed code, unusual segment caches and incompatible state
still fall back to the shared interpreter. Segment/address-space changes return
to the outer DOS loop before another block can use a stale image context.

For a conservative set of unpaged register-only PIL operations, generated code
avoids redundant RAM paging synchronization. Other operations retain the normal
wrapper. Backend eligibility hints avoid flushing lazy flags on unsupported
sites. Installed timer handlers and interrupt shadows retain the existing
instruction-level scheduling restrictions.

This upgrade does not scan or load the shadow directory, embed DOS programs in
ordinary release builds, add native binary loading, or change the standalone DOS
runtime. The existing explicit generated-backend build hook remains available.
An ordinary release build remains an interpreter unless an executor is explicitly
supplied; these compiler upgrades alone do not give it MANDVGA's native speedup.

## Instruction profiles and unchanged code pages

Each recorded instruction is tagged from its PIL semantics as register-only,
memory, machine-context, or guarded. The emitter uses those tags to select a
RAM-free execution helper for proven register-only operations and to terminate
blocks at memory stores or context changes. The profile report includes tag
counts. This does not infer that an unobserved branch can never be taken.

The ordinary interpreter also tracks physical code pages (4 KiB). While a
watched page is untouched, a matching decode-cache entry skips byte comparisons.
The first successful write to that page permanently restores the original byte
checks. Guest write permissions are unchanged. Stores, bulk/debugger writes,
A20 aliases, and paging accessed/dirty-bit writes participate in invalidation.
Paged and planar instruction spans retain byte validation. Cache metadata is
separate from architectural state, and RAM clones have distinct cache identities.

Windows validation: 667 Rust tests passed, 2 ignored, including new profile and
code-page cases. Generated loop guards separately compare CPU, memory, segments,
flags, output, and retirement counts across small budgets. A local release loop
sample (100000 iterations, five measurements) changed median from 31.677 ms to
29.380 ms; this is a narrow interpreter measurement, not a general speed claim.
