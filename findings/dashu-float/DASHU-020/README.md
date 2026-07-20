# DASHU-020: FBig and DBig double-round values adjacent to primitive subnormal halfways

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Contract: `backend_conformance`. Owner: `backend`. Masked by adapter: `true`.

Latest release check: Confirmed on dashu-float 0.5.0. Upstream PR #91 (afdc90a) contains a proposed fix and equivalent regression tests; the PR was open when verified on 2026-07-20.

## Summary

Raw FBig/DBig conversion first rounds to a fixed 53/24-bit intermediate and then rounds again to the exponent-dependent f64/f32 subnormal grid. Values just below a subnormal halfway can land on the halfway during the first rounding and then round to the adjacent even subnormal, one ULP above the correct result.

## Impact

Direct Dashu arbitrary-precision float conversion can silently return the wrong primitive bits in both binary and decimal construction paths. The opendp-num adapter does not expose FBig-to-primitive conversion, so its existing exact-number conversion contracts do not reveal this backend defect.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none backend_float_conversion findings/dashu-float/DASHU-020/inputs/DASHU-020-f64-binary.input
cargo fuzz run --sanitizer none backend_float_conversion findings/dashu-float/DASHU-020/inputs/DASHU-020-f32-binary.input
cargo fuzz run --sanitizer none backend_float_conversion findings/dashu-float/DASHU-020/inputs/DASHU-020-f64-decimal.input
cargo fuzz run --sanitizer none backend_float_conversion findings/dashu-float/DASHU-020/inputs/DASHU-020-f32-decimal.input
```

## Evidence

- `DASHU-020-f64-binary.input`: 25 bytes, SHA-256 `7f090d3aa55417bf2af707bd2db85a2f564a785be597aeb9eab349507601da30`; expects `operation=to_f64 reason=raw backend conversion differs from exact-rational oracle`
- `DASHU-020-f32-binary.input`: 17 bytes, SHA-256 `ce6079637801d0d43eaeadd86f4a6fc174b608f3f2004c9b439a29d63481e4d0`; expects `operation=to_f32 reason=raw backend conversion differs from exact-rational oracle`
- `DASHU-020-f64-decimal.input`: 685 bytes, SHA-256 `7569902ffe71e7f5b3b9cc25a83301f735c09447a1b72abf1d2bd8d644e150fb`; expects `operation=to_f64 reason=raw backend conversion differs from exact-rational oracle`
- `DASHU-020-f32-decimal.input`: 123 bytes, SHA-256 `89086a415ffb3016a067ddd6ac96326f2c6966d1f2ea36caa2bbe9cdff9f3e6a`; expects `operation=to_f32 reason=raw backend conversion differs from exact-rational oracle`

## Deduplication rationale

The f32/f64 and binary/decimal manifestations share the fixed-width-intermediate double-rounding mechanism documented and fixed together in upstream PR #91. They are retained as four construction/format witnesses for one backend defect.

## Reporting note

This is a direct provider probe. The opendp-num adapter does not expose this arbitrary-precision float conversion path, so the backend defect is retained even though it does not currently violate the public uniformity surface.
