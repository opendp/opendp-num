# DASHU-012: Upward addition near -1 skips one representable f64

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Confirmed on Dashu 0.5.0.

## Summary

Adding the minimum positive subnormal to the f64 immediately above -1 under upward rounding returns 0xbfeffffffffffffd instead of MPFR's 0xbfeffffffffffffe.

## Impact

The result is one representable value too high and is not correctly rounded.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none directed_binary findings/dashu/DASHU-012/inputs/DASHU-012.input
cargo fuzz run --sanitizer none opendp_sequences findings/dashu/DASHU-012/inputs/DASHU-012-sequence.input
```

## Evidence

- `DASHU-012.input`: 21 bytes, SHA-256 `2ac064056c89fcc4c1f25d21aeb4cf7a62a9c4df056862d60d8e3a3f9070069d`; expects `operation=add reason=correctly rounded value differs from MPFR`
- `DASHU-012-sequence.input`: 64 bytes, SHA-256 `6b33c11111eefc08c39f5f1348234622ca995a9403631a42c83ff14cea8d5768`; expects `operation=add reason=composed correctly rounded result differs from MPFR`

## Deduplication rationale

A distinct addition normalization case; kept separate from subtraction near f64::MAX.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
