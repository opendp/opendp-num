# DASHU-024: powi range handling misclassifies representable endpoints, ignores directed rounding, and unwraps range errors before reciprocal

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result` (with an out-of-range reciprocal `panic` manifestation).

Contract: `backend_conformance`. Owner: `backend`. Masked by adapter: `true`.

Latest release check: Reproduces with dashu-float 0.5.0 and current Dashu master 40f465b62e5d8f4198efc43871e3ce601d03dc93 in debug and release builds. Root cause verified against source (exp.rs:116-127,139-162,187; error.rs:131-136). No matching upstream issue or fix was found when verified on 2026-07-20.

## Summary

`Context::powi` range handling has three coupled defects sharing one pipeline:

1. **Representable-boundary misclassification.** The guard estimates the result exponent with an `f64` (`exp.rs:139-162`); around `2^63` it cannot distinguish `isize::MAX`, `isize::MAX+1`, and neighbours. `2^isize::MAX`, `(-2)^isize::MAX`, `2^isize::MIN` panic (`res.with_precision` rounds an intermediate infinity at `exp.rs:187`) and `(1/2)^(isize::MAX+1)` returns zero, though each exact result is a finite `FBig`.
2. **Mode-blind range saturation.** Genuine `FpError::Overflow`/`Underflow` is unwrapped by `unwrap_fp` (`error.rs:131-136`) to `±inf`/signed zero regardless of rounding mode; e.g. `Down 2^(isize::MAX+1)` returns `+inf` instead of the largest finite value.
3. **Reciprocal panic.** The negative-exponent path (`exp.rs:116-127`) does `unwrap_fp(powi(...))` → infinity, then `repr_div(1, inf)`, feeding infinity into finite-only division and panicking, though the finite reciprocal endpoint is representable.

## Impact

Public exact operations panic or return the wrong side of the exact result at the exponent range, invalidating interval and conservative-bound computations; some equivalent reciprocal forms panic rather than returning an endpoint. OpenDP's adapter structurally classifies extreme primitive overflow/underflow before invoking raw `FBig::powi` and masks this path.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root:

```bash
cargo run --example reproduce_dashu_024   # representable-boundary panics / zero
cargo run --example reproduce_dashu_025   # directed range saturation + reciprocal panic
```

Both reproducers are retained because they exercise distinct observable symptoms (representable-boundary vs. genuine out-of-range directed saturation and reciprocal panic) of the same `Context::powi` range-handling pipeline.

## Evidence

Reproduced directly against the library API; see the commands above. Root cause verified by source read of `float/src/exp.rs:116-127,139-162,187` and `float/src/error.rs:131-136` at commit `40f465b`.

## Deduplication rationale

DASHU-024 and the former DASHU-025 are the same broken `Context::powi` range-handling pipeline and are merged. Shares the "finite range result collapsed into a mode-independent limiting value" architecture with DASHU-023, but kept separate: DASHU-023 is localized to `exp_internal` and is not repaired by fixing `unwrap_fp` alone (its `exp_m1` branch returns `Exact(-1)` directly).

## Reporting note

Direct provider probes found by extending the PR #2801 audit to literal primitive extremes and FBig's exact `isize` exponent boundaries.
