# DASHU-027: exp_m1 allocates memory proportional to a huge positive exponent in debug and fuzz builds

Status: confirmed on the locked baseline. Confidence: high. Classification: `resource-exhaustion`.

Contract: `backend_conformance`. Owner: `backend`. Masked by adapter: `true`.

Latest release check: Reproduces with dashu-float 0.5.0 and current Dashu master 40f465b62e5d8f4198efc43871e3ce601d03dc93. At input 1e8 and precision 2, the instrumented debug build peaks above 20 MB of live heap while release stays below 1 KB; mutation fuzzing reached the 2 GB worker limit. No matching upstream issue or fix was found when verified on 2026-07-20.

## Summary

For a huge positive but finite FBig input, exp_m1 forms exp(x) and subtracts one. In debug and libFuzzer profiles, aligning that insignificant subtraction allocates storage proportional to the result exponent rather than the requested precision. The instrumented reproducer measures 20,422,236 peak live heap bytes for exp_m1(1e8) at precision 2, versus 788 bytes in release. Mutated inputs scaled this path past libFuzzer's 2 GB RSS limit.

## Impact

A small public input object and precision can trigger memory growth controlled by the numeric value, allowing denial of service and repeatedly terminating fuzz workers before a full campaign completes. OpenDP's primitive adapter preclassifies large expm1 overflow and masks the raw backend path.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root:

```bash
cargo run --example reproduce_dashu_027
```

## Evidence

Reproduced directly against the library API; see the command above.

## Deduplication rationale

Separate from DASHU-026's negative-boundary panic. DASHU-027 is a positive-input allocation-complexity defect in the final subtraction of one; it returns successfully at smaller examples but scales to worker OOM. It is also separate from the mathematically incorrect directed saturation in DASHU-023.

## Reporting note

This is a direct provider resource probe. The global counting allocator measures live heap without relying on platform-specific RSS tools; run the example in both debug and release to see the profile difference.
