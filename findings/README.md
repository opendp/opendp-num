# Curated fuzzer findings

These are conservatively deduplicated findings from differential and property fuzzing of `opendp-num`. Raw runner failures are intentionally excluded: every listed reproducer must pass `fuzz/verify_findings.py` on the locked baseline.

| ID | Library | Kind | Finding |
|---|---|---|---|
| [DASHU-007](dashu-float/DASHU-007/) | dashu-float | incorrect-result | Directed log2 differs from MPFR across magnitude ranges |
| [DASHU-008](dashu-int/DASHU-008/) | dashu-int | panic | Rational construction triggers internal allocation panic in GCD |
| [DASHU-015](dashu/DASHU-015/) | dashu | incorrect-result | IBig/UBig to_f64 reports an inexact conversion as Exact and rounds to the wrong side |

## Reproduce everything

```bash
python3 fuzz/verify_findings.py
```

See `METHODOLOGY.md` for the evidence, deduplication, quarantine, and upstream-validation policy.
