# DASHU-015: Directed big-integer-to-f64 conversion rounds toward larger magnitude regardless of direction

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Confirmed on Dashu 0.5.0.

## Summary

Converting -(2^128 - 1) to f64 under upward rounding returns -3.402823669209385e38 instead of MPFR's less-negative adjacent value, and converting +(2^128 - 1) as a natural under downward rounding returns 3.402823669209385e38 instead of MPFR's smaller adjacent value.

## Impact

Around the 2^128 magnitude, Dashu's directed integer/natural conversion always rounds to the larger-magnitude neighbor, violating the requested directed bound in both directions.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none conversions findings/dashu/DASHU-015/inputs/DASHU-015.input
cargo fuzz run --sanitizer none conversions findings/dashu/DASHU-015/inputs/DASHU-015-b.input
cargo fuzz run --sanitizer none conversions findings/dashu/DASHU-015/inputs/DASHU-015-natural.input
```

## Evidence

- `DASHU-015.input`: 126 bytes, SHA-256 `5c9f513e3b755ed1645882b132b8a689708d8130bf64c9941529ca3c51ac1967`; expects `operation=integer_to_f64 reason=directed conversion differs from MPFR`
- `DASHU-015-b.input`: 150 bytes, SHA-256 `b1d7c89eb4cc5fddd7e98b9d9ac9eb29ea18050a398be3ebb15e9a1b6239e191`; expects `operation=integer_to_f64 reason=directed conversion differs from MPFR`
- `DASHU-015-natural.input`: 161 bytes, SHA-256 `c30a3afeb043e000da6fb628873bf298cdd0ae341b653f78df034f4282e30302`; expects `operation=natural_to_f64 reason=directed conversion differs from MPFR`

## Deduplication rationale

The upward signed-integer and downward natural manifestations share the same integer conversion path and the same too-large-magnitude symptom on the same 2^128-1 magnitude, so they are grouped as one directed integer-conversion rounding defect. Kept separate from rational/nearest conversion behavior, which uses a different path and rounding mode.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
