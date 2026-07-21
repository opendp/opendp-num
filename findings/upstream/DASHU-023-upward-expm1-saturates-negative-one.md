# Astronomically negative `exp` and `exp_m1` discard upward rounding

**Versions:** dashu-float 0.5.0 and master `40f465b62e5d8f4198efc43871e3ce601d03dc93`  
**Existing fix:** none known. No matching issue was found as of 2026-07-20.

## Summary

For the finite, exactly represented input `x = -2^63` at precision 1:

- `FBig::<Up>::exp()` returns zero;
- `FBig::<Up>::exp_m1()` returns `-1`, and the underlying public `Context::exp_m1()` API labels it `Approximation::Exact`.

Both exact values are strictly greater than the returned results. The correctly upward-rounded `FBig` results are respectively the minimum positive value `2^isize::MIN` and, at precision 1, `-0.5`. Both returns therefore violate upward directed rounding; `exp_m1` also carries false exactness metadata.

## Reproduce

```rust
use dashu::{
    base::Approximation,
    float::{FBig, round::mode::Up},
    integer::IBig,
};

fn main() {
    let x = FBig::<Up>::from_parts(-IBig::ONE, 63); // exactly -2^63
    assert_eq!(x.precision(), 1);

    let exp = x.exp();
    assert!(exp.repr().significand().is_zero());
    let expected_exp = FBig::<Up>::from_parts(IBig::ONE, isize::MIN);
    assert!(expected_exp > exp);

    let approximation = x
        .context()
        .exp_m1(x.repr(), None)
        .expect("finite exp_m1 input");

    assert!(matches!(
        approximation,
        Approximation::Exact(ref value) if value == &FBig::<Up>::NEG_ONE
    ));
}
```

The repository also contains a deterministic reproducer:

```bash
cargo run --example reproduce_dashu_023
```

## Observed result

Both debug and release builds return zero for `exp` and `Exact(-1)` for `exp_m1`, regardless of upward or downward rounding:

```text
Up   exp(-2^63)    = 0
Down exp(-2^63)    = 0
Up   exp_m1(-2^63) = Exact(-1)
Down exp_m1(-2^63) = Exact(-1)
```

This reproduces with dashu-float 0.5.0 and current master `40f465b62e5d8f4198efc43871e3ce601d03dc93` as of 2026-07-20.

## Expected result

For finite `x`, `exp(x)` is strictly positive. At this input it is below the smallest positive `FBig`, `2^isize::MIN`, so directed range rounding requires:

```text
Up   exp(x) = 2^isize::MIN
Down exp(x) = 0
```

For `exp_m1`, since `x = -2^63 < -ln(2)`:

```text
0 < exp(x) < 1/2
-1 < exp_m1(x) < -1/2
```

At binary precision 1, the adjacent representable values enclosing the exact result are `-1` and `-1/2`. Therefore:

- upward rounding must return `-1/2` and mark it inexact;
- downward rounding may return `-1`, but must mark it inexact rather than exact.

## Root cause

The range reduction computes `s = floor(x / ln(B))`. In [`float/src/exp.rs`](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/exp.rs#L397-L411), failure to convert `s` to `isize` takes an explicit saturation branch:

```rust
let s: isize = match s.try_into() {
    Ok(v) => v,
    Err(_) => {
        return if input_sign == Sign::Positive {
            Err(FpError::Overflow(Sign::Positive))
        } else if minus_one {
            Ok(Exact(-FBig::ONE))
        } else {
            Err(FpError::Underflow(Sign::Positive))
        };
    }
};
```

Both negative branches discard the rounding mode. The `exp` branch produces `Underflow`, which the convenience layer maps to zero for every mode. The `minus_one` branch treats the finite limiting value as if the input were negative infinity and reports `-1` as exact.

## Impact

An upward-rounded value is used as an upper bound, but zero is below every finite `exp(x)` and `-1` is below every finite `exp_m1(x)`. Direct Dashu callers can therefore receive invalid directed bounds. Callers using the `exp_m1` `Approximation` result also receive incorrect exactness metadata.

OpenDP PR [#2801](https://github.com/opendp/opendp/pull/2801) masks this defect with conservative primitive-range fast paths. Its thresholds such as `-37` are not Dashu's failure threshold; they are earlier points at which the exact primitive result can already be bounded by adjacent floating-point values.

This is unrelated to Dashu PR [#91](https://github.com/cmpute/dashu/pull/91), which addresses double rounding during final conversion to primitive subnormals. The information loss here occurs inside `exp`/`exp_m1`, before primitive conversion. At the primitive layer, upward `exp(-f64::MAX)` must likewise return `f64::MIN_POSITIVE_SUBNORMAL`, not zero.

## Suggested resolution

Preserve the underflow information and apply the active rounding mode rather than unconditionally returning zero or `Exact(-1)`. In particular, for negative finite inputs in this branch:

- upward `exp` must return the minimum positive `FBig`, while downward `exp` may return zero;
- modes rounding upward must return the next representable value above `-1` at the requested precision;
- modes rounding downward may return `-1`;
- every saturated result must be marked inexact.

An equivalent implementation could construct a positive underflow approximation for `exp(x)` and then subtract one under the requested context, provided it avoids losing the directed rounding information at the exponent-range boundary.
