# DASHU-015: IBig/UBig to_f64 reports an inexact conversion as Exact and rounds to the wrong side

Status: confirmed on the locked baseline. Confidence: high. Classification: `incorrect-result`.

Latest release check: Reproduces directly against dashu 0.5.0. opendp-num defends against it by not trusting the Approximation tag, so it no longer manifests through the adapter, but the dashu defect itself remains.

## Summary

IBig::from(-(2^128 - 1)).to_f64() returns Approximation::Exact(-3.402823669209385e38) even though -(2^128 - 1) is not representable as f64, and the returned value is on the wrong side of the exact value (larger magnitude). A caller that trusts the Exact tag gets an incorrect, non-directed result.

## Impact

Any consumer relying on to_f64 to signal exactness, or on the directed rounding implied by the Approximation sign, receives a silently incorrect conversion.

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

A single dashu conversion-rounding/exactness-reporting defect. Distinct from the correctly-rounded-arithmetic behavior of RBig.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
