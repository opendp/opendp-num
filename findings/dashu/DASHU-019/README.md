# DASHU-019: Directed multiply of two positive minimum subnormals returns the negative minimum subnormal

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Confirmed on Dashu 0.5.0.

## Summary

Multiplying f64::from_bits(1) by itself under downward rounding returns -f64::from_bits(1) (0x8000000000000001), while MPFR returns +0. The exact product 2^-2148 is positive and underflows to +0.

## Impact

The product of two positive operands is returned with a negative sign, an impossible result that lies on the wrong side of zero; downstream directed lower bounds derived from it are invalid.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none directed_binary findings/dashu/DASHU-019/inputs/DASHU-019.input
```

## Evidence

- `DASHU-019.input`: 11 bytes, SHA-256 `949d8f50bf63fc665c43dac8a070e19a9131421abfc7c573a0832af330c10904`; expects `operation=mul reason=correctly rounded value differs from MPFR`

## Deduplication rationale

A standalone directed multiplication sign/underflow defect. Kept separate from the exp (DASHU-004) and powi (DASHU-011) underflow-sign findings because it is the multiply operation on a different backend path, and separate from the addition and division rounding findings because those preserve sign and differ by at most one representable value.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
