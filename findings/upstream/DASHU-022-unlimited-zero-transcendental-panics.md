# Exact primitive zero converts to an unlimited-precision `FBig` that exact transcendental special cases reject

**Versions:** dashu-float 0.5.0 and master `40f465b62e5d8f4198efc43871e3ce601d03dc93`  
**Existing fix:** none known. Related issue [#45](https://github.com/cmpute/dashu/issues/45) established that exact zero intentionally has unlimited precision, but concerned loss of an explicitly requested precision and did not cover these operation panics.

## Summary

The public conversion `FBig::<R>::try_from(0.0)` succeeds but returns precision 0, which Dashu defines as unlimited precision. Public `exp`, `exp_m1`, `sqrt`, and `ln_1p` calls then panic because they reject unlimited precision before reaching their exact zero special cases.

All four results are exact and require no caller-selected approximation precision:

```text
exp(0)   = 1
expm1(0) = 0
sqrt(0)  = 0
ln1p(0)  = 0
```

This is one shared precision-state incompatibility, not four independent numerical defects.

## Reproduce

```rust
use std::panic::{AssertUnwindSafe, catch_unwind};
use dashu::float::{FBig, round::mode::Up};

fn main() {
    let zero = FBig::<Up>::try_from(0.0f64).unwrap();
    assert_eq!(zero.precision(), 0);

    assert!(catch_unwind(AssertUnwindSafe(|| zero.exp())).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| zero.exp_m1())).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| zero.sqrt())).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| zero.ln_1p())).is_err());
}
```

The repository also contains a deterministic reproducer:

```bash
cargo run --example reproduce_dashu_022
```

## Observed result

Each operation panics in both debug and release builds:

```text
precision cannot be 0 (unlimited) for this operation!
```

This reproduces with dashu-float 0.5.0 and current master `40f465b62e5d8f4198efc43871e3ce601d03dc93` as of 2026-07-20.

## Expected result

The conversion returns `Ok(FBig)` for a valid finite primitive. The exact zero identities above should therefore return exact `FBig` values without requiring the caller to invent a finite precision.

If unlimited inputs are intentionally unsupported even for exact special cases, all affected public operation documentation should state that precondition. Currently `sqrt` documents it, while `exp`, `exp_m1`, and `ln_1p` do not.

## Root cause

Primitive conversion infers its context from the decoded mantissa width in [`float/src/convert.rs`](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/convert.rs#L82-L91):

```rust
let bits = man.unsigned_abs().bit_len();
let context = Context::new(bits);
```

For `±0.0`, `man == 0`, so `bit_len()` is 0 and the public conversion returns an unlimited-precision value. This is consistent with the intentional precision of `FBig::ZERO` described in issue #45.

The problem is the order of operation preconditions and exact shortcuts. For example, [`exp_internal`](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/exp.rs#L331-L350) calls:

```rust
assert_limited_precision(self.precision);

if x.significand.is_zero() {
    // exact exp(±0) / exp_m1(±0) handling
    // ...
}
```

[`sqrt`](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/root.rs#L88-L99) and [`ln_internal`](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/log.rs#L252-L270) have the same ordering: they assert limited precision before checking the exact zero case. The exact `FBig::ONE` constant also has precision 0, so `FBig::<Up>::ONE.ln()` panics before the exact `ln(1) = 0` shortcut. In contrast, `try_from(1.0)`, `try_from(0.5)`, and powers of two receive the normalized primitive mantissa width (53 for f64, 24 for f32) and remain consumable.

## Impact

A value successfully produced by a public exact conversion cannot be passed to several public operations on valid-domain inputs. Downstream callers must know to apply `with_precision(24/53)` after every primitive conversion or catch a panic. OpenDP PR [#2801](https://github.com/opendp/opendp/pull/2801) adds that normalization, so the adapter masks the raw backend defect.

## Suggested resolution

Handle mathematically exact special cases before `assert_limited_precision`, preserving the input sign where required:

- `exp(±0) -> 1`;
- `exp_m1(±0) -> ±0`;
- `sqrt(±0) -> ±0`;
- `ln_1p(±0) -> ±0`;
- `ln(1) -> 0`.

The limited-precision assertion can remain for cases that genuinely require approximation. Alternatively, if the intended contract is that these operations reject every unlimited-precision input, make that precondition consistent across their public documentation and consider returning a typed error rather than panicking on a value created by a public conversion.
