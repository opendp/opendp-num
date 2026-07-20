# DASHU-015: IBig/UBig to_f64 reports an inexact conversion as Exact and rounds to the wrong side

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Contract: `backend_conformance`. Owner: `backend`. Masked by adapter: `true`.

Latest release check: Reproduces directly against dashu 0.5.0. opendp-num defends against it by not trusting the Approximation tag, so it no longer manifests through the adapter, but the dashu defect itself remains.

## Summary

IBig::from(-(2^128 - 1)).to_f64() returns Approximation::Exact(-3.402823669209385e38) even though -(2^128 - 1) is not representable as f64. Root cause (dashu-int-0.5.0/src/convert.rs:1130-1140, to_f64_small; to_f32_small is not affected, being guarded by is_infinite): the exactness test is `let back = f as DoubleWord; back.partial_cmp(&dword)`, but `f as DoubleWord` is a SATURATING float->int cast. For dword = DoubleWord::MAX (2^128-1 on 64-bit), f rounds up to 2^128 and `(2^128) as u128` saturates back to u128::MAX == dword, so the comparison reports Equal and the value is tagged Exact. It only misfires at exactly DoubleWord::MAX; nearby values saturate to a larger `back` and are correctly Inexact.

## Impact

A caller trusting the Exact tag (or the Approximation sign for directed rounding) gets a silently incorrect, non-directed conversion at the type-maximum boundary. opendp-num defends by re-deriving the rounding from the exact rational and ignoring the tag.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo run --example root_cause --features all-backends
```

## Evidence

Reproduced directly against the library API; see the command above.

## Deduplication rationale

A single dashu-int conversion exactness-reporting defect caused by a saturating float->int cast in the exactness test. Verified directly via examples/root_cause.rs and by source audit.

## Reporting note

This is a direct provider probe. The opendp-num adapter does not expose this arbitrary-precision float conversion path, so the backend defect is retained even though it does not currently violate the public uniformity surface.
