# DASHU-024: powi panics or returns zero at exact representable isize exponent boundaries

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Contract: `backend_conformance`. Owner: `backend`. Masked by adapter: `true`.

Latest release check: Reproduces with dashu-float 0.5.0 and current Dashu master 40f465b62e5d8f4198efc43871e3ce601d03dc93 in debug and release builds. No matching upstream issue or fix was found when verified on 2026-07-20.

## Summary

FBig::powi panics for 2^isize::MAX, (-2)^isize::MAX, and 2^isize::MIN, while (1/2)^(isize::MAX+1) returns zero. Each exact result is a finite FBig that can be constructed directly with from_parts. The range guard converts isize endpoints and arbitrary exponents to f64, losing endpoint equality; subsequent powering either creates an intermediate infinity or misclassifies the exact lower endpoint as underflow.

## Impact

Valid exact power operations panic or return zero in both debug and release at the public representation's exponent endpoints. Direct callers cannot compute values that FBig itself can represent, and must avoid boundary exponents or construct the result manually. The opendp-num adapter structurally classifies sufficiently extreme primitive powi results before entering this path.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root:

```bash
cargo run --example reproduce_dashu_024
```

## Evidence

Reproduced directly against the library API; see the command above.

## Deduplication rationale

Separate from DASHU-023's exp/exp_m1 directed-rounding saturation and DASHU-022's unlimited-precision special-case panics. This defect is specific to powi range guarding and intermediate infinity at exact representable exponent boundaries. Positive, negative, and odd-negative-base manifestations share that mechanism and are grouped together.

## Reporting note

This is a direct provider probe found by extending the PR #2801 audit to literal primitive extremes and FBig's exact isize exponent boundaries.
