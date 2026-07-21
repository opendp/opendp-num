# DASHU-023: Astronomical negative exp and exp_m1 violate upward rounding

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Contract: `backend_conformance`. Owner: `backend`. Masked by adapter: `true`.

Latest release check: Reproduces with dashu-float 0.5.0 and current Dashu master 40f465b62e5d8f4198efc43871e3ce601d03dc93 in debug and release builds. No matching upstream issue or fix was found when verified on 2026-07-20.

## Summary

For the finite exact input x = -2^63 at precision 1, FBig::<Up>::exp returns zero and exp_m1 returns Exact(-1). Upward exp should return FBig's minimum positive value, 2^isize::MIN, because the exact result is positive but below the exponent range. Because 0 < exp(x) < 1/2, upward expm1 should return -1/2 at precision 1. The shared failure occurs when floor(x / ln 2) does not fit isize and the range-reduction branch discards the rounding mode.

## Impact

Both upward results lie below their true finite values, invalidating directed upper bounds; exp_m1 additionally carries false Exact metadata. For primitive output, exp(-f64::MAX) must round upward to the minimum positive subnormal, not zero. OpenDP PR #2801 masks both paths with conservative primitive-range shortcuts, but direct Dashu callers remain affected.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root:

```bash
cargo run --example reproduce_dashu_023
```

## Evidence

Reproduced directly against the library API; see the command above.

## Deduplication rationale

The exp and exp_m1 manifestations share the same negative range-reduction overflow branch and loss of rounding direction, so they are grouped. This remains separate from DASHU-022's unlimited-precision panic, DASHU-024's powi exponent-boundary panic, and DASHU-020/PR #91's final primitive-conversion double rounding.

## Reporting note

This is a direct provider probe. OpenDP PR #2801 avoids both backend saturation results by returning mathematically justified directed primitive bounds for sufficiently negative inputs.
