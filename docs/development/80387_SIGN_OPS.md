# 80387 phase 6: FCHS and FABS

This focused addition continues the opt-in 80387 backend on `dev`.
The default absent-coprocessor behaviour is unchanged.

`FCHS` reached `dev` first (d5949c0, with FNSTSW m16 and FILD); this
increment adds `FABS` to the same arm in src/fpu_present.rs and the
sign-bit tests below.

## Supported

- `D9 E0` (`FCHS`): toggle only bit 15 of ST(0)'s sign/exponent word.
- `D9 E1` (`FABS`): clear only bit 15 of ST(0)'s sign/exponent word.
- Preserve the 64-bit significand, exponent, tag classification and TOP.
  Signed zero remains signed, and the operations require no precision mode.
- Clear x87 condition code C1 on success; leave CPU EFLAGS unchanged.
- Empty ST(0) gives an explicit stack fault and leaves state unchanged.

Tests include real-mode DOS COM execution, exact extended80 memory stores,
round trips on signed nonzero and zero values, C1 clearing and underflow
retirement behaviour.

## Still unsupported

`FTST` (`D9 E4`), broader special-value arithmetic, precision-control
modes other than 64-bit nearest/even, and full x87 exception delivery.
The existing Turbo Pascal hardware-FPU test must be rerun with the
rebuilt engine: this increment alone does not certify `{$N+}` or claim
the next compiler stop is fixed.
