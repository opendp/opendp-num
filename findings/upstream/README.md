# Ready-to-file upstream issues

## Dashu

Draft issues for the genuine dashu defects found via differential fuzzing + source audit against the 0.5.0 baseline. Each has a standalone reproducer (or crash signature), a source-pinned root cause, and a suggested fix.

- [`DASHU-015`](DASHU-015-to_f64-false-exact.md) — `to_f64` false `Exact` at `DoubleWord::MAX` (saturating-cast exactness test). *Fix staged on the `../dashu` fork branch `fix/int-to-f64-false-exact`.*
- [`DASHU-008`](DASHU-008-gcd-scratchpad-panic.md) — GCD panic: scratchpad sized from initial lengths, division dispatched on current lengths.
- [`DASHU-020`](DASHU-020-subnormal-primitive-double-rounding.md) — `FBig`/`DBig` conversion double-rounds values adjacent to primitive subnormal halfways. *Already addressed by open PR [#91](https://github.com/cmpute/dashu/pull/91).*
- [`DASHU-021`](DASHU-021-high-precision-dbig-debug-panic.md) — high-precision `DBig` conversion trips a debug assertion in base-changing division. *Already addressed by open PR [#91](https://github.com/cmpute/dashu/pull/91).*
- [`DASHU-022`](DASHU-022-unlimited-zero-transcendental-panics.md) — exact primitive zero converts to an unlimited-precision `FBig` that exact transcendental special cases reject. *Still present on master `40f465b`; no fix known.*
- [`DASHU-023`](DASHU-023-upward-expm1-saturates-negative-one.md) — upward `exp(-2^63)` returns zero and `exp_m1(-2^63)` returns `Exact(-1)`; both discard the required upward range endpoint. *Still present on master `40f465b`; no fix known.*
- [`DASHU-024`](DASHU-024-powi-range-handling.md) — `Context::powi` range handling has three coupled defects: an `f64` estimate misclassifies representable `isize::{MAX,MIN}` boundary powers (panic/zero), genuine range errors are unwrapped mode-blind by `unwrap_fp`, and the negative-exponent path feeds an infinity into finite-only reciprocal division (panic). Merges the former DASHU-024 and DASHU-025. *Still present on master `40f465b`; root cause verified against source, no upstream fix known.*
- [`DASHU-026`](DASHU-026-round-fract-debug-assert-materializes-base-pow.md) — a debug-only assertion in generic `Round::round_fract` constructs `B^exponent_gap` to validate a sparse sticky remainder, causing `exp_m1` boundary panics and ~20 MB allocation growth in debug/fuzz builds (release is unaffected). Merges the former DASHU-026 boundary panic and DASHU-027 allocation growth into one defect. *Still present on master `40f465b`; root cause verified against source, no upstream fix known.*

## Malachite

Nothing found.
