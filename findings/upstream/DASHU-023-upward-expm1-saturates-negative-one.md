# Upward `exp_m1` returns `Exact(-1)` for a finite input whose range reduction exceeds `isize`

**Versions:** dashu-float 0.5.0 and master `40f465b62e5d8f4198efc43871e3ce601d03dc93`  
**Existing fix:** none known. No matching issue was found as of 2026-07-20.

## Summary

For the finite, exactly represented input `x = -2^63` at precision 1, `FBig::<Up>::exp_m1()` returns `-1`. The underlying public `Context::exp_m1()` API additionally labels that result `Approximation::Exact`.

The exact value is strictly greater than `-1`, and the correctly upward-rounded precision-1 result is `-0.5`. The returned value therefore violates upward directed rounding as well as carrying false exactness metadata.

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

Both debug and release builds return `Exact(-1)` for upward and downward rounding:

```text
Up   exp_m1(-2^63) = Exact(-1)
Down exp_m1(-2^63) = Exact(-1)
```

This reproduces with dashu-float 0.5.0 and current master `40f465b62e5d8f4198efc43871e3ce601d03dc93` as of 2026-07-20.

## Expected result

Since `x = -2^63 < -ln(2)`:

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

The `minus_one` branch treats the finite limiting value as if the input were negative infinity. It does not consult the rounding mode or output precision, and reports the limiting value as exact.

## Impact

An upward-rounded value is used as an upper bound, but `-1` is below every finite `exp_m1(x)`. Direct Dashu callers can therefore receive an invalid directed bound. Callers using the `Approximation` result also receive incorrect exactness metadata.

OpenDP PR [#2801](https://github.com/opendp/opendp/pull/2801) masks this defect with conservative primitive-range fast paths. Its thresholds such as `-37` are not Dashu's failure threshold; they are earlier points at which the exact primitive result can already be bounded by adjacent floating-point values.

This is unrelated to Dashu PR [#91](https://github.com/cmpute/dashu/pull/91), which addresses double rounding during final conversion to primitive subnormals. The information loss here occurs inside `exp_m1`, before primitive conversion.

No separate issue is proposed for `exp(-2^63) == 0`: the exact positive result is below the exponent range representable by `FBig`, whose exponent is an `isize`. The `exp_m1` result remains ordinary and representable near `-1`, which is why this case is a directed-rounding defect rather than an unavoidable range underflow.

## Suggested resolution

Preserve the underflow information and apply the active rounding mode to the representable result near `-1`, rather than unconditionally returning `Exact(-1)`. In particular, for negative finite inputs in this branch:

- modes rounding upward must return the next representable value above `-1` at the requested precision;
- modes rounding downward may return `-1`;
- either result must be marked inexact.

An equivalent implementation could construct a positive underflow approximation for `exp(x)` and then subtract one under the requested context, provided it avoids losing the directed rounding information at the exponent-range boundary.
