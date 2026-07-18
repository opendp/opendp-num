# DASHU-003: Nearest rational-to-f32 conversion disagrees with MPFR

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Neither retained manifestation reproduces with Dashu 0.5.0; appears fixed in the latest release.

## Summary

Two distinct negative rational inputs convert to an adjacent but incorrect f32 value under nearest rounding.

## Impact

Exact rational values may be rounded to the wrong primitive float, breaking correctly-rounded conversion guarantees.

## Tested baseline

`dashu 0.4.3`, `dashu-float 0.4.4`, `dashu-int 0.4.3`, `dashu-ratio 0.4.2`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Historical reproduction inputs

Run from the repository root after installing `cargo-fuzz`. On the current Dashu 0.5 baseline these inputs are expected to complete without reproducing the former mismatch:

```bash
cargo fuzz run --sanitizer none conversions findings/resolved-by-upgrade/DASHU-003/inputs/DASHU-003-a.input
cargo fuzz run --sanitizer none conversions findings/resolved-by-upgrade/DASHU-003/inputs/DASHU-003-b.input
```

## Evidence

- `DASHU-003-a.input`: 35 bytes, SHA-256 `aaceb8e3e3b278af4a5d5ac4efff8bac10fe753070c775bc047091edcf8876d6`; expects `operation=rational_to_f32 reason=directed conversion differs from MPFR`
- `DASHU-003-b.input`: 82 bytes, SHA-256 `38af20a5f00f0b527a695948e7930c5c351646c7b852a845a7509f458593f647`; expects `operation=rational_to_f32 reason=directed conversion differs from MPFR`

## Deduplication rationale

The two reports share the same operation, direction, and adjacent-float symptom and are treated as manifestations of one conversion defect.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
