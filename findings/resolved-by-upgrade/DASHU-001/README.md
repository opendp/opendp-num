# DASHU-001: Directed binary operations do not preserve IEEE signed zero

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Not reproduced with Dashu 0.5.0; appears fixed in the latest release.

## Summary

Dashu-backed directed multiplication and division turn -0 into +0, while exact cancellation under downward rounding also returns +0 instead of MPFR's -0.

## Impact

Bit-for-bit directed rounding contracts and algorithms that distinguish the sign of zero can observe an incorrect result.

## Tested baseline

`dashu 0.4.3`, `dashu-float 0.4.4`, `dashu-int 0.4.3`, `dashu-ratio 0.4.2`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Historical reproduction inputs

Run from the repository root after installing `cargo-fuzz`. On the current Dashu 0.5 baseline these inputs are expected to complete without reproducing the former mismatch:

```bash
cargo fuzz run --sanitizer none directed_binary findings/resolved-by-upgrade/DASHU-001/inputs/DASHU-001-div.input
cargo fuzz run --sanitizer none directed_binary findings/resolved-by-upgrade/DASHU-001/inputs/DASHU-001-mul.input
cargo fuzz run --sanitizer none directed_binary findings/resolved-by-upgrade/DASHU-001/inputs/DASHU-001-sub.input
```

## Evidence

- `DASHU-001-div.input`: 21 bytes, SHA-256 `17caedd6d503fab76047212f33d897dbcce7cad771168d9fe5c64b616cd2d77a`; expects `operation=div reason=correctly rounded value differs from MPFR`
- `DASHU-001-mul.input`: 21 bytes, SHA-256 `dfe4196e711a234e426d5140bb652b85331b99dd42eff0babd1adf9624ce9a86`; expects `operation=mul reason=correctly rounded value differs from MPFR`
- `DASHU-001-sub.input`: 21 bytes, SHA-256 `bb0b0c82bf6593ce20a140f313b858339759266c5d020e4ab9d890c2c9df5851`; expects `operation=sub reason=correctly rounded value differs from MPFR`

## Deduplication rationale

The multiply, divide, and subtract manifestations are grouped because all differ only in the sign bit of an exact zero result. They should be split upstream if the implementations have independent sign-selection paths.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
