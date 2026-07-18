# DASHU-005: Downward expm1 reports overflow when f64::MAX is representable

Status: confirmed on the locked baseline. Confidence: high. Classification: `wrong-error`.

Latest release check: Still reproduces with dashu-float 0.5.0.

## Summary

For a large finite input under downward rounding, Dashu reports overflow while MPFR returns f64::MAX.

## Impact

A valid finite directed result is rejected as an error at the overflow boundary.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none directed_unary findings/dashu-float/DASHU-005/inputs/DASHU-005.input
```

## Evidence

- `DASHU-005.input`: 16 bytes, SHA-256 `402aaa7a0ce71226ad38688f54d1b0f2a6166d592a8b77584aec3ecccbd2c224`; expects `operation=expm1 reason=Dashu returned an error for a valid MPFR result`

## Deduplication rationale

Kept separate from the exp panic because the observable contract and failure mode differ.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
