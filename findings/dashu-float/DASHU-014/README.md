# DASHU-014: Downward ln near one is one ULP too low

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Confirmed on dashu-float 0.5.0.

## Summary

For input 0x3ff0000000005500 under downward rounding, Dashu ln returns one f64 below the MPFR result.

## Impact

The returned bound is unnecessarily low and not correctly rounded.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none opendp_sequences findings/dashu-float/DASHU-014/inputs/DASHU-014.input
```

## Evidence

- `DASHU-014.input`: 119 bytes, SHA-256 `ebd2454fff1799826b0d63e384bfc23eeb7bc53b34f35ba873bb5878affb773f`; expects `operation=ln reason=composed correctly rounded result differs from MPFR`

## Deduplication rationale

The sequence inputs agree at the failing step, isolating a standalone ln discrepancy distinct from log2.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
