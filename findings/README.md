# Curated fuzzer findings

These are conservatively deduplicated findings from differential and property fuzzing of `opendp-num`. Raw runner failures are intentionally excluded: every listed reproducer must pass `fuzz/verify_findings.py` on the locked baseline.

| ID | Library | Kind | Finding |
|---|---|---|---|
| [DASHU-002](dashu/DASHU-002/) | dashu | incorrect-result | Downward subtraction can step one extra f64 lower |
| [DASHU-004](dashu-float/DASHU-004/) | dashu-float | incorrect-result | Directed f32 exp underflow returns an incorrect zero or negative subnormal |
| [DASHU-005](dashu-float/DASHU-005/) | dashu-float | wrong-error | Downward expm1 reports overflow when f64::MAX is representable |
| [DASHU-006](dashu-float/DASHU-006/) | dashu-float | incorrect-result | Downward ln1p of the minimum subnormal rounds upward |
| [DASHU-007](dashu-float/DASHU-007/) | dashu-float | incorrect-result | Directed log2 differs from MPFR across magnitude ranges |
| [DASHU-008](dashu-int/DASHU-008/) | dashu-int | panic | Rational construction triggers internal allocation panic in GCD |
| [DASHU-010](dashu/DASHU-010/) | dashu | wrong-error | Directed division reports overflow when the correctly rounded result is exactly +/-f64::MAX |
| [DASHU-011](dashu-float/DASHU-011/) | dashu-float | incorrect-result | Downward negative power of f64::MAX returns a negative subnormal |
| [DASHU-012](dashu/DASHU-012/) | dashu | incorrect-result | Upward addition near -1 skips one representable f64 |
| [DASHU-013](dashu-float/DASHU-013/) | dashu-float | incorrect-result | Upward expm1 of the minimum normal f64 rounds downward |
| [DASHU-014](dashu-float/DASHU-014/) | dashu-float | incorrect-result | Downward ln near one is one ULP too low |
| [DASHU-015](dashu/DASHU-015/) | dashu | incorrect-result | Directed big-integer-to-f64 conversion rounds toward larger magnitude regardless of direction |
| [DASHU-016](dashu/DASHU-016/) | dashu | incorrect-result | Upward directed division is one ULP too high |
| [DASHU-018](dashu/DASHU-018/) | dashu | incorrect-result | Adding -0 to a tiny positive f64 produces a vastly larger value |
| [DASHU-019](dashu/DASHU-019/) | dashu | incorrect-result | Directed multiply of two positive minimum subnormals returns the negative minimum subnormal |

## Reproduce everything

```bash
python3 fuzz/verify_findings.py
```

See `METHODOLOGY.md` for the evidence, deduplication, quarantine, and upstream-validation policy.
