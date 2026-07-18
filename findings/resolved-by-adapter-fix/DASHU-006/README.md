# DASHU-006: Downward ln1p of the minimum subnormal rounds upward

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Still reproduces with dashu-float 0.5.0.

## Summary

Downward ln1p of f64::from_bits(1) returns the same minimum subnormal, while MPFR rounds the slightly smaller exact result to +0.

## Impact

The purported lower bound is above the exact result.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none directed_unary findings/dashu-float/DASHU-006/inputs/DASHU-006.input
```

## Evidence

- `DASHU-006.input`: 16 bytes, SHA-256 `598458d43635e7e8024a5adbff0132cd56f2eb5d48493b9e737ea95500705b3c`; expects `operation=ln1p reason=correctly rounded value differs from MPFR`

## Deduplication rationale

A standalone unary boundary bug with a distinct operation and output pattern.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
