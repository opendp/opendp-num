# DASHU-021: High-precision DBig to primitive conversion panics with debug assertions

Status: confirmed on the locked baseline. Confidence: high. Classification: `panic`.

Contract: `backend_conformance`. Owner: `backend`. Masked by adapter: `true`.

Latest release check: Confirmed on dashu-float 0.5.0. Upstream PR #91 (afdc90a) removes the assertion and rounds over-wide quotients; the PR was open when verified on 2026-07-20.

## Summary

DBig conversion of an over-wide decimal significand reaches the base-changing division with more dividend digits than its fixed target precision permits. A debug assertion aborts conversion; optimized release mode disables that assertion and completes, creating profile-dependent observable behavior.

## Impact

A valid finite arbitrary-precision decimal can abort debug builds during to_f64/to_f32. Release builds take a different path and may silently double-round over-wide quotients, so debug and release do not enforce the same conversion contract.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none backend_float_conversion findings/dashu-float/DASHU-021/inputs/DASHU-021-high-precision-decimal.input
```

## Evidence

- `DASHU-021-high-precision-decimal.input`: 35 bytes, SHA-256 `af8038980850d0dfe8ad47d7a6e0d99b74c540e85f65d7b9fd272e2df3db0aa9`; expects `assertion failed: lhs.digits() <= self.precision + rhs.digits()`

## Deduplication rationale

Separated from DASHU-020 because this is a construction-width assertion/profile-equivalence failure in base-changing division, while DASHU-020 is a demonstrated one-ULP subnormal result caused by fixed-width intermediate rounding. Upstream PR #91 addresses both mechanisms.

## Reporting note

This is a direct provider probe. The opendp-num adapter does not expose this arbitrary-precision float conversion path, so the backend defect is retained even though it does not currently violate the public uniformity surface.
