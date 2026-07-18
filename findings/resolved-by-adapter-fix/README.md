# Resolved by adapter fix

These findings were **opendp-num adapter defects**, not dashu bugs. They were
resolved by rewriting the directed rounding in `src/backend/dashu.rs` to:

- compute directed binary arithmetic via **exact `RBig`** then a **single
  correctly-rounded directed conversion** (no intermediate `FBig` rounding, so
  no double rounding);
- round every rational/integer/float→primitive result by **exact `RBig`
  comparison**, never trusting `to_f64`'s `Approximation` tag, and clamping
  correctly at signed zero and the subnormal boundary (no zero-crossing);
- size transcendental working precision to the input magnitude so underflow
  cases resolve to the correct side of zero;
- drop the native-float `is_infinite`/`is_nan` prechecks that rejected
  representable `±f64::MAX` results.

Each listed input **no longer reproduces** on the fixed adapter (`fuzz/verify_findings.py`
replay: all `FAIL` = not reproduced). They are retained here for provenance.

See [`../ROOT_CAUSE.md`](../ROOT_CAUSE.md) for the classification method and
per-finding evidence, and `examples/root_cause.rs` / `examples/verify_fix.rs`
for the direct dashu-API probe and the adapter regression check.

## Note on DASHU-015

DASHU-015 is a **genuine dashu bug** (`IBig::to_f64()` returns `Approximation::Exact`
for an inexact value). The adapter defends against it by not trusting the tag, so it
no longer manifests through opendp-num — but the dashu defect itself remains and is
worth filing upstream. Its evidence is `examples/root_cause.rs`, not a fuzz reproducer.
