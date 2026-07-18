# DASHU-013: Upward expm1 of the minimum normal f64 rounds downward

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Confirmed on dashu-float 0.5.0.

## Summary

Upward expm1 of f64::MIN_POSITIVE returns the input unchanged, while MPFR returns the next larger f64.

## Impact

The purported upper bound is below the exact result.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none directed_unary findings/dashu-float/DASHU-013/inputs/DASHU-013.input
```

## Evidence

- `DASHU-013.input`: 16 bytes, SHA-256 `1f9e74b0a4ddf85bb4041457db9f01bad5ca588568b16543c605aa6fa99d127d`; expects `operation=expm1 reason=correctly rounded value differs from MPFR`

## Deduplication rationale

Kept separate from the expm1 overflow error because this is a small-input rounding defect on the opposite boundary.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
