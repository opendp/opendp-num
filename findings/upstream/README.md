# Ready-to-file upstream issues (github.com/cmpute/dashu)

Draft issues for the genuine dashu defects found via differential fuzzing + source audit against the 0.5.0 baseline. Each has a standalone reproducer (or crash signature), a source-pinned root cause, and a suggested fix.

- [`DASHU-015`](DASHU-015-to_f64-false-exact.md) — `to_f64` false `Exact` at `DoubleWord::MAX` (saturating-cast exactness test). *Fix staged on the `../dashu` fork branch `fix/int-to-f64-false-exact`.*
- [`DASHU-008`](DASHU-008-gcd-scratchpad-panic.md) — GCD panic: scratchpad sized from initial lengths, division dispatched on current lengths.
- [`DASHU-020`](DASHU-020-subnormal-primitive-double-rounding.md) — `FBig`/`DBig` conversion double-rounds values adjacent to primitive subnormal halfways. *Already addressed by open PR [#91](https://github.com/cmpute/dashu/pull/91).*
- [`DASHU-021`](DASHU-021-high-precision-dbig-debug-panic.md) — high-precision `DBig` conversion trips a debug assertion in base-changing division. *Already addressed by open PR [#91](https://github.com/cmpute/dashu/pull/91).*
- [`DASHU-022`](DASHU-022-unlimited-zero-transcendental-panics.md) — exact primitive zero converts to an unlimited-precision `FBig` that exact transcendental special cases reject. *Still present on master `40f465b`; no fix known.*
- [`DASHU-023`](DASHU-023-upward-expm1-saturates-negative-one.md) — upward `exp_m1(-2^63)` returns `Exact(-1)` even though the correctly rounded precision-1 result is `-0.5`. *Still present on master `40f465b`; no fix known.*

## Verify the PR #2801 audit

The baseline audit checks Dashu 0.5.0 in debug and release. Supplying a current Dashu checkout additionally archives and tests its committed `HEAD`, excluding uncommitted checkout changes:

```bash
python3 fuzz/verify_upstream_dashu.py --master-checkout ../dashu
```
