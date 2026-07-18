# DASHU-007: Directed log2 differs from MPFR across magnitude ranges

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Still reproduces with dashu-float 0.5.0.

## Summary

Dashu log2 differs from MPFR across magnitude ranges and even for exact powers of two: near 2^-24 under downward rounding, at f64::MIN_POSITIVE under upward rounding, log2(1.0) downward returns a small negative value instead of 0, and log2(f64::MAX) downward is loose by many ULPs. dashu-float exposes only f32-precision log2_bounds, so no directed log2 is correctly rounded.

## Impact

The error is many ULPs rather than a tie-breaking or signed-zero difference, and even inputs whose log2 is an exact integer are not returned exactly.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none opendp_sequences findings/dashu-float/DASHU-007/inputs/DASHU-007.input
cargo fuzz run --sanitizer none directed_unary findings/dashu-float/DASHU-007/inputs/DASHU-007-direct.input
cargo fuzz run --sanitizer none directed_unary findings/dashu-float/DASHU-007/inputs/DASHU-007-one.input
cargo fuzz run --sanitizer none directed_unary findings/dashu-float/DASHU-007/inputs/DASHU-007-max.input
```

## Evidence

- `DASHU-007.input`: 119 bytes, SHA-256 `738fa008053d1d447af5b6e85831c889880a783d0e5cb3502f91b96269ea0c53`; expects `operation=log2 reason=composed correctly rounded result differs from MPFR`
- `DASHU-007-direct.input`: 16 bytes, SHA-256 `f3e1b32e5cd854b919a2bf8a501262d914e4f90dab16c56a2a9f8f95ed1f277a`; expects `operation=log2 reason=correctly rounded value differs from MPFR`
- `DASHU-007-one.input`: 16 bytes, SHA-256 `a953ac42b9072fefd3c7d517e60047cf03ca79536841904e39b60902f46968c4`; expects `operation=log2 reason=correctly rounded value differs from MPFR`
- `DASHU-007-max.input`: 16 bytes, SHA-256 `02a5a7c330f6f2226a1fa8a5ac43934632fc75737c4d8315c8215a1056ec98f8`; expects `operation=log2 reason=correctly rounded value differs from MPFR`

## Deduplication rationale

The direct, exact-power-of-two, and sequence inputs all isolate the same log2 implementation (the f32-precision log2_bounds); the discrepancies vary in size but share the operation-level root.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
