# DASHU-002: Downward subtraction can step one extra f64 lower

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Still reproduces with Dashu 0.5.0.

## Summary

Both near f64::MAX and near 2.0, Dashu-backed downward subtraction returns one representable value below MPFR's correctly rounded result.

## Impact

The returned lower bound is unnecessarily loose by one representable value and is not correctly rounded.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none directed_binary findings/dashu/DASHU-002/inputs/DASHU-002.input
cargo fuzz run --sanitizer none opendp_sequences findings/dashu/DASHU-002/inputs/DASHU-002-sequence.input
```

## Evidence

- `DASHU-002.input`: 21 bytes, SHA-256 `d97613344280e1f19c512e07fa9c26eaeafb063529e931b32f469877a489e185`; expects `operation=sub reason=correctly rounded value differs from MPFR`
- `DASHU-002-sequence.input`: 56 bytes, SHA-256 `e9c35e00f18ed36a5d691f13e39bbfb4d3fa99be8e8f7f7a58bbe4450db8acb4`; expects `operation=sub reason=composed correctly rounded result differs from MPFR`

## Deduplication rationale

The two magnitude ranges share the same subtraction, direction, and one-extra-ULP symptom and are grouped as a normalization/rounding defect.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
