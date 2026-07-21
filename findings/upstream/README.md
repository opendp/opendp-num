# Ready-to-file upstream issues

## Dashu

Draft issues for the genuine dashu defects found via differential fuzzing + source audit against the 0.5.0 baseline. Each has a standalone reproducer (or crash signature), a source-pinned root cause, and a suggested fix.

- [`DASHU-015`](DASHU-015-to_f64-false-exact.md) — `to_f64` false `Exact` at `DoubleWord::MAX` (saturating-cast exactness test). *Fix staged on the `../dashu` fork branch `fix/int-to-f64-false-exact`.*
- [`DASHU-008`](DASHU-008-gcd-scratchpad-panic.md) — GCD panic: scratchpad sized from initial lengths, division dispatched on current lengths.
- [`DASHU-020`](DASHU-020-subnormal-primitive-double-rounding.md) — `FBig`/`DBig` conversion double-rounds values adjacent to primitive subnormal halfways. *Already addressed by open PR [#91](https://github.com/cmpute/dashu/pull/91).*
- [`DASHU-021`](DASHU-021-high-precision-dbig-debug-panic.md) — high-precision `DBig` conversion trips a debug assertion in base-changing division. *Already addressed by open PR [#91](https://github.com/cmpute/dashu/pull/91).*
- [`DASHU-022`](DASHU-022-unlimited-zero-transcendental-panics.md) — exact primitive zero converts to an unlimited-precision `FBig` that exact transcendental special cases reject. *Still present on master `40f465b`; no fix known.*
- [`DASHU-023`](DASHU-023-upward-expm1-saturates-negative-one.md) — upward `exp(-2^63)` returns zero and `exp_m1(-2^63)` returns `Exact(-1)`; both discard the required upward range endpoint. *Still present on master `40f465b`; no fix known.*
- [`DASHU-024`](DASHU-024-powi-representable-exponent-boundary-panics.md) — `powi` panics or returns zero at `isize::{MAX,MIN}` even when the exact result is a finite, directly constructible `FBig`. *Still present on master `40f465b`; no fix known.*
- [`DASHU-025`](DASHU-025-powi-directed-range-saturation.md) — out-of-range `powi` ignores directed finite endpoints and its negative-exponent reciprocal path can panic. *Still present on master `40f465b`; no fix known.*
- [`DASHU-026`](DASHU-026-expm1-debug-range-boundary-panic.md) — `exp_m1` panics for a broad finite-input matrix near the negative exponent-range boundary in debug and fuzz builds. *Still present on master `40f465b`; no fix known.*
- [`DASHU-027`](DASHU-027-expm1-debug-allocation-growth.md) — large-magnitude `exp_m1` allocates memory proportional to the exponent gap in debug/fuzz builds; mutation runs exceeded 2 GB RSS. *Still present on master `40f465b`; no fix known.*

## Malachite

Nothing found.
