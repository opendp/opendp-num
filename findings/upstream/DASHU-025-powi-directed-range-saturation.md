# `powi` ignores directed endpoints outside the exponent range and can panic during reciprocal

**Versions:** dashu-float 0.5.0 and master `40f465b62e5d8f4198efc43871e3ce601d03dc93`  
**Existing fix:** none known. No matching issue was found as of 2026-07-20.

## Summary

For exact powers genuinely outside `FBig`'s exponent range, `FBig::powi` converts `Context` overflow and underflow errors to infinity or zero without respecting the rounding direction. The negative-exponent implementation can then panic while taking the reciprocal of that saturated intermediate.

This occurs in debug and release builds and under both `Up` and `Down` rounding.

## Reproduce

```rust
use dashu::{
    float::{FBig, round::mode::Down},
    integer::IBig,
};

fn main() {
    let exponent = IBig::from(isize::MAX) + IBig::ONE;
    let two = FBig::<Down>::from_parts(IBig::ONE, 1);

    let observed = two.powi(exponent);
    assert!(observed.repr().is_infinite());

    // This is the correct downward endpoint and is finite.
    let expected = FBig::<Down>::from_parts(IBig::ONE, isize::MAX);
    assert!(!expected.repr().is_infinite());
}
```

The repository reproducer also covers the out-of-range reciprocal panic:

```bash
cargo run --example reproduce_dashu_025
```

## Expected result

For a positive exact result above the range, `Down` should return the largest finite value and `Up` may return positive infinity. Corresponding negative overflow and signed underflow cases need the endpoint selected by their direction and sign.

The negative-exponent path should return a directed zero/minimum-magnitude endpoint, not perform finite-only division with an infinity intermediate.

## Root cause

The early range guard in `Context::powi` returns `FpError::Overflow` or `FpError::Underflow`. The `FBig::powi` convenience method immediately unwraps that error to an infinity or zero value. This loses the directed finite endpoint. Negative exponents reverse the rounding mode, recursively call `powi`, and then divide one by the saturated result; an infinity intermediate reaches arithmetic that rejects infinity and panics.

## Impact

The returned value can lie on the wrong side of the exact result, invalidating interval and conservative-bound computations. Algebraically equivalent reciprocal forms may panic instead of returning any range endpoint.

## Suggested resolution

Make range-error conversion aware of rounding mode and sign. For negative exponents, classify the final reciprocal range directly or propagate a typed range result through the reciprocal rather than materializing infinity and feeding it to finite-only division.

