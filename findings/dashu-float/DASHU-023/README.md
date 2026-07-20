# DASHU-023: Upward exp_m1 returns Exact(-1) when range reduction exceeds isize

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Contract: `backend_conformance`. Owner: `backend`. Masked by adapter: `true`.

Latest release check: Reproduces with dashu-float 0.5.0 and current Dashu master 40f465b62e5d8f4198efc43871e3ce601d03dc93 in debug and release builds. No matching upstream issue or fix was found when verified on 2026-07-20.

## Summary

For the finite exact input x = -2^63 at precision 1, FBig::<Up>::exp_m1 returns -1 and the underlying Context API labels it Exact. Because 0 < exp(x) < 1/2, the exact expm1 result lies strictly between -1 and -1/2, so the correctly upward-rounded precision-1 result is -1/2. The failure occurs when the range-reduction integer floor(x / ln 2) does not fit isize and the exp_m1 branch unconditionally saturates to Exact(-1).

## Impact

The upward result is below the true finite value, invalidating its directed-rounding guarantee, and its Exact metadata is false. OpenDP PR #2801 masks this path with a conservative primitive-range expm1 shortcut, but direct Dashu callers remain affected.

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

Separate from DASHU-022, which is an unlimited-precision panic on exact zero, and from DASHU-020/PR #91, which is double rounding during final conversion to primitive subnormals. The astronomical exp result is not filed separately because the exact positive result is already outside FBig's exponent range; representable exp cases and tested powi cases did not saturate prematurely.

## Reporting note

This is a direct provider probe. OpenDP PR #2801 avoids the backend saturation branch by returning a mathematically justified directed primitive bound for sufficiently negative inputs.
