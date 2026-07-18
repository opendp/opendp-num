# DASHU-004: Directed f32 exp underflow returns an incorrect zero or negative subnormal

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Confirmed on dashu-float 0.5.0.

## Summary

At large negative finite f32 inputs, upward exp returns +0 instead of the minimum positive subnormal, while downward exp can return a negative minimum subnormal instead of +0.

## Impact

The upward result is below the exact positive value, and the downward manifestation has an impossible negative sign for exp.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none directed_unary findings/dashu-float/DASHU-004/inputs/DASHU-004.input
cargo fuzz run --sanitizer none directed_unary findings/dashu-float/DASHU-004/inputs/DASHU-017.input
```

## Evidence

- `DASHU-004.input`: 16 bytes, SHA-256 `99320f2b34a61aad5e4d06033e667bedf636586c43e22eab344e4ac26ffb33e5`; expects `operation=exp reason=correctly rounded value differs from MPFR`
- `DASHU-017.input`: 16 bytes, SHA-256 `950388e2a9c85a3e755ed8903e2e6429a3803de34144a6692a3146dc4c2bc292`; expects `operation=exp reason=correctly rounded value differs from MPFR`

## Deduplication rationale

The two manifestations are grouped as one f32 exponential-underflow/sign defect. The panic seen with Dashu 0.4 is not part of this 0.5 report.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
