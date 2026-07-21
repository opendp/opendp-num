# `powi` panics or returns zero at exact representable `isize` exponent boundaries

**Versions:** dashu-float 0.5.0 and master `40f465b62e5d8f4198efc43871e3ce601d03dc93`  
**Existing fix:** none known. No matching issue was found as of 2026-07-20.

## Summary

`FBig::powi` panics for `2^isize::MAX`, `(-2)^isize::MAX`, and `2^isize::MIN`. It also returns zero for `(1/2)^(isize::MAX + 1)`. Each exact result is a finite `FBig` and can be constructed directly with `FBig::from_parts`.

The failure occurs in both debug and release builds and under both upward and downward rounding.

## Reproduce

```rust
use dashu::{
    float::{FBig, round::mode::Up},
    integer::IBig,
};

fn main() {
    let base = FBig::<Up>::try_from(2.0f64)
        .unwrap()
        .with_precision(53)
        .value();

    // This directly constructed exact result is finite.
    let expected = FBig::<Up>::from_parts(IBig::ONE, isize::MAX);
    assert!(!expected.repr().is_infinite());

    // Panics instead of returning `expected`.
    let _ = base.powi(IBig::from(isize::MAX));
}
```

The repository contains a reproducer covering all three manifestations:

```bash
cargo run --example reproduce_dashu_024
```

## Observed result

```text
arithmetic operations with the infinity are not allowed!
```

The backtrace ends at `Context::repr_round` through `FBig::with_precision`, called from the final step of `Context::powi`.

## Expected result

These identities are exact and within the public representation's exponent range:

```text
2^isize::MAX    = FBig::from_parts( 1, isize::MAX)
(-2)^isize::MAX = FBig::from_parts(-1, isize::MAX) // isize::MAX is odd
2^isize::MIN    = FBig::from_parts( 1, isize::MIN)
```

All should return the corresponding finite value without rounding or panic. As an asymmetric control, `(1/2)^isize::MAX` succeeds and returns exponent `-isize::MAX`; increasing its power by one should return exponent `isize::MIN`, but currently returns zero.

## Root cause

The range guard in [`float/src/exp.rs`](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/exp.rs#L136-L186) estimates the result exponent through `f64`:

```rust
let threshold = (isize::MAX as f64) * (B.log2_est() as f64);
let exp_f64 = i64::try_from(&exp).ok().map(|e| e as f64);
let overflows = match exp_f64 {
    Some(e) => e * base_log2 > threshold,
    None => base_log2 != 0.0,
};
```

On a 64-bit target, both `isize::MAX as f64` and nearby converted positive exponents round to `2^63`. That loses the distinction between the representable endpoint and the first out-of-range exponent. Depending on the path, binary exponentiation reaches infinity or the guard reports underflow at an exact representable endpoint. In the former case:

```rust
Ok(res.with_precision(self.precision))
```

attempts to round infinity and panics. For `isize::MIN`, the negative-exponent reciprocal path similarly creates an intermediate infinity even though its reciprocal is the finite lower endpoint.

## Impact

Public exact operations panic on values at the stated range of the public representation. Callers cannot use `powi` to produce boundary values that they can construct directly. OpenDP's primitive adapter masks these particular cases by structurally classifying extreme primitive overflow and underflow before invoking raw `FBig::powi`.

## Suggested resolution

Handle exact powers of the radix without floating-point range estimation, and make the general guard distinguish representable endpoint equality from true overflow using integer arithmetic or conservative directed bounds. The negative-exponent path should preserve the exact reciprocal endpoint instead of converting the positive intermediate to infinity first.

Independently, avoid passing an infinity result into `with_precision`; if an operation genuinely exceeds the representation range, propagate a typed overflow/underflow result or return the documented convenience-layer range value without performing finite-only rounding.
