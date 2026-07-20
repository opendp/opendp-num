# DASHU-022: Exact primitive zero converts to an unlimited-precision FBig that exact transcendental special cases reject

Status: confirmed on the locked baseline. Confidence: high. Classification: `panic`.

Contract: `backend_conformance`. Owner: `backend`. Masked by adapter: `true`.

Latest release check: Reproduces with dashu-float 0.5.0 and current Dashu master 40f465b62e5d8f4198efc43871e3ce601d03dc93 in debug and release builds. No upstream fix was present when verified on 2026-07-20; related issue #45 established that exact zero intentionally uses unlimited precision but did not cover these operation panics.

## Summary

FBig::<R>::try_from(0.0) succeeds and assigns precision 0, which Dashu defines as unlimited precision. exp, exp_m1, sqrt, and ln_1p then assert that precision is limited before reaching their exact zero shortcuts, so valid operations with exact results panic. The public FBig::ONE constant exposes the same precision-state incompatibility for ln. Nonzero primitive conversions such as one and powers of two carry the normal primitive mantissa precision and do not trigger it.

## Impact

A value produced successfully by a public exact conversion cannot be consumed by several public operations on inputs whose outputs are mathematically exact. Callers must know to add an arbitrary finite precision after converting zero, or catch a panic. OpenDP PR #2801 masks the defect by applying with_precision(24/53) to every primitive conversion.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root:

```bash
cargo run --example reproduce_dashu_022
```

## Evidence

Reproduced directly against the library API; see the command above.

## Deduplication rationale

One shared precision-state root cause: exact public zero/one representations carry Context::new(0), while multiple operations check assert_limited_precision before their exact special-case branches. The four zero operations and the ONE/ln manifestation are grouped together. Premature exp and powi saturation did not reproduce while their exact results remained in FBig range; the distinct astronomical exp_m1 directed-rounding defect is tracked as DASHU-023.

## Reporting note

This is a direct provider probe. The opendp-num and OpenDP adapters explicitly assign primitive mantissa precision before invoking Dashu transcendentals, so the public adapter surface currently masks the backend panic.
