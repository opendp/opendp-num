# DASHU-011: Downward negative power of f64::MAX returns a negative subnormal

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Still reproduces with dashu-float 0.5.0.

## Summary

For powi(f64::MAX, -53) under downward rounding, Dashu returns -f64::from_bits(1), while MPFR returns +0.

## Impact

The result has the wrong sign and lies on the opposite side of zero from the positive exact value.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none directed_unary findings/dashu-float/DASHU-011/inputs/DASHU-011.input
```

## Evidence

- `DASHU-011.input`: 16 bytes, SHA-256 `b77aabab6bbe6181c0646fa1bad7d6aba8448cb4164926454e0e5b8f3d597d4e`; expects `operation=powi reason=correctly rounded value differs from MPFR`

## Deduplication rationale

A distinct powi sign/underflow defect, separate from ln1p's positive subnormal boundary error.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
