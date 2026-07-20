# Ready-to-file upstream issues (github.com/cmpute/dashu)

Draft issues for the genuine dashu defects found via differential fuzzing + source audit against the 0.5.0 baseline. Each has a standalone reproducer (or crash signature), a source-pinned root cause, and a suggested fix.

- [`DASHU-015`](DASHU-015-to_f64-false-exact.md) — `to_f64` false `Exact` at `DoubleWord::MAX` (saturating-cast exactness test). *Fix staged on the `../dashu` fork branch `fix/int-to-f64-false-exact`.*
- [`DASHU-008`](DASHU-008-gcd-scratchpad-panic.md) — GCD panic: scratchpad sized from initial lengths, division dispatched on current lengths.
- [`DASHU-020`](DASHU-020-subnormal-primitive-double-rounding.md) — `FBig`/`DBig` conversion double-rounds values adjacent to primitive subnormal halfways. *Already addressed by open PR [#91](https://github.com/cmpute/dashu/pull/91).*
- [`DASHU-021`](DASHU-021-high-precision-dbig-debug-panic.md) — high-precision `DBig` conversion trips a debug assertion in base-changing division. *Already addressed by open PR [#91](https://github.com/cmpute/dashu/pull/91).*
