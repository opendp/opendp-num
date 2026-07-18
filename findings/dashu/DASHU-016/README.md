# DASHU-016: Upward directed division is one ULP too high

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Confirmed on Dashu 0.5.0.

## Summary

Dividing two finite f64 values near the normal/subnormal boundary under upward rounding returns one representable value above MPFR's correctly rounded result.

## Impact

The result is a valid upper bound but is not the tight correctly-rounded value promised by the contract.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none directed_binary findings/dashu/DASHU-016/inputs/DASHU-016.input
```

## Evidence

- `DASHU-016.input`: 21 bytes, SHA-256 `7fdf3255576459a4dc57c78ed4a08f798974203f84d197d74c52d3c271cba219`; expects `operation=div reason=correctly rounded value differs from MPFR`

## Deduplication rationale

Kept separate from DASHU-010 because that finding is an erroneous overflow result rather than an in-range one-ULP discrepancy.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
