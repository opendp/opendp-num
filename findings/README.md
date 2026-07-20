# Curated fuzzer findings

These are conservatively deduplicated findings from differential and property fuzzing of `opendp-num`. Raw runner failures are intentionally excluded: every listed reproducer must pass `fuzz/verify_findings.py` on the locked baseline.

| ID | Library | Contract | Kind | Finding |
|---|---|---|---|---|
| [DASHU-007](dashu-float/DASHU-007/) | dashu-float | uniformity | incorrect-result | Directed log2 differs from MPFR across magnitude ranges |
| [DASHU-008](dashu-int/DASHU-008/) | dashu-int | uniformity | panic | Rational reduction: Lehmer GCD reaches Burnikel-Ziegler division that overflows a pre-sized scratchpad |
| [DASHU-015](dashu/DASHU-015/) | dashu | backend_conformance | incorrect-result | IBig/UBig to_f64 reports an inexact conversion as Exact and rounds to the wrong side |
| [DASHU-020](dashu-float/DASHU-020/) | dashu-float | backend_conformance | incorrect-result | FBig and DBig double-round values adjacent to primitive subnormal halfways |
| [DASHU-021](dashu-float/DASHU-021/) | dashu-float | backend_conformance | panic | High-precision DBig to primitive conversion panics with debug assertions |
| [DASHU-022](dashu-float/DASHU-022/) | dashu-float | backend_conformance | panic | Exact primitive zero converts to an unlimited-precision FBig that exact transcendental special cases reject |
| [DASHU-023](dashu-float/DASHU-023/) | dashu-float | backend_conformance | incorrect-result | Upward exp_m1 returns Exact(-1) when range reduction exceeds isize |

## Reproduce everything

```bash
python3 fuzz/verify_findings.py
```

See `METHODOLOGY.md` for the evidence, deduplication, quarantine, and upstream-validation policy.
