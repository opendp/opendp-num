# DASHU-018: Adding -0 to a tiny positive f64 produces a vastly larger value

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Confirmed on Dashu 0.5.0.

## Summary

Under upward rounding, adding -0 to 0x0000003ff0020000 returns 0x3d80000000000000 instead of preserving the tiny positive operand.

## Impact

The output is approximately 2^-39 rather than a subnormal-scale value, a large magnitude error in a basic directed addition.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none directed_binary findings/dashu/DASHU-018/inputs/DASHU-018.input
```

## Evidence

- `DASHU-018.input`: 18 bytes, SHA-256 `1a1ce3103149b39247c6c81000c148ea4962a1f4e1c3f787df81ac288491bc55`; expects `operation=add reason=correctly rounded value differs from MPFR`

## Deduplication rationale

Kept separate from one-ULP addition findings because this result changes the exponent by hundreds of bins and specifically involves a signed-zero operand.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
