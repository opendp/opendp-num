# DASHU-010: Directed division reports overflow when the correctly rounded result is exactly +/-f64::MAX

Status: confirmed on the locked baseline. Confidence: high. Classification: `wrong-error`.

Latest release check: All three retained manifestations still reproduce with Dashu 0.5.0.

## Summary

Dividing 1 by a tiny positive f64 under downward rounding reports overflow while MPFR returns +f64::MAX, and dividing a value near -1 by a tiny positive subnormal under upward rounding reports overflow while MPFR returns the representable -f64::MAX.

## Impact

A valid finite directed result that lands exactly on the +/-f64::MAX boundary is rejected as an error rather than returned.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none directed_binary findings/dashu/DASHU-010/inputs/DASHU-010-direct.input
cargo fuzz run --sanitizer none directed_binary findings/dashu/DASHU-010/inputs/DASHU-010-up.input
cargo fuzz run --sanitizer none opendp_sequences findings/dashu/DASHU-010/inputs/DASHU-010-sequence.input
```

## Evidence

- `DASHU-010-direct.input`: 16 bytes, SHA-256 `5fb655feb6c0b4056fa240d6f32b1a803be4e76e75f964270ccb4d72f47608bb`; expects `operation=div reason=Dashu returned an error for a valid MPFR result`
- `DASHU-010-up.input`: 17 bytes, SHA-256 `a4225336d5f216ba2e48b7ea8352c1b42250966ffef67ba5e777be1a3e0a4a1b`; expects `operation=div reason=Dashu returned an error for a valid MPFR result`
- `DASHU-010-sequence.input`: 85 bytes, SHA-256 `a7512d1e5e10d5b5f70d45307b7228a62d914f042a6d1c1e17c5bcf1e4b025b6`; expects `operation=div reason=composed operation success/error behavior differs from MPFR`

## Deduplication rationale

The downward/+f64::MAX and upward/-f64::MAX binary manifestations and the composed-sequence report share one root cause: the overflow check fires at the f64::MAX magnitude boundary even though the correctly rounded directed result is representable. They are grouped as one division overflow-boundary defect independent of sign and rounding direction.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
