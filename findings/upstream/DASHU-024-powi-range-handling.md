# `powi` range handling misclassifies representable endpoints, ignores directed rounding, and unwraps range errors before reciprocal

**Versions:** dashu-float 0.5.0 and master `40f465b62e5d8f4198efc43871e3ce601d03dc93`
**Existing fix:** none known. No matching issue was found as of 2026-07-20.

## Summary

`Context::powi`'s range handling has three coupled defects that share one pipeline:

1. It classifies overflow/underflow with an `f64` estimate that cannot distinguish the last representable exponent from the first unrepresentable one, so exact, directly constructible boundary powers panic or return zero.
2. When it does return a genuine range error, `FBig::powi` maps it to infinity/zero via `unwrap_fp`, ignoring the rounding mode and sign.
3. The negative-exponent path unwraps the positive intermediate to infinity *before* taking the reciprocal, feeding infinity into finite-only division, which panics.

The three sections below correspond to the three defects; a single coherent rewrite of `Context::powi` resolves all of them.

## Reproduce

```bash
cargo run --example reproduce_dashu_024   # representable-boundary panics / zero
cargo run --example reproduce_dashu_025   # directed range saturation + reciprocal panic
```

---

## Section 1 — Representable-boundary failures

`FBig::powi` panics for `2^isize::MAX`, `(-2)^isize::MAX`, and `2^isize::MIN`, and returns zero for `(1/2)^(isize::MAX + 1)`. Each of these exact results is a finite `FBig` constructible with `from_parts`:

```text
2^isize::MAX    = FBig::from_parts( 1, isize::MAX)
(-2)^isize::MAX = FBig::from_parts(-1, isize::MAX)   // isize::MAX is odd
2^isize::MIN    = FBig::from_parts( 1, isize::MIN)
```

```rust
use dashu::{float::{FBig, round::mode::Up}, integer::IBig};

let base = FBig::<Up>::try_from(2.0f64).unwrap().with_precision(53).value();
let expected = FBig::<Up>::from_parts(IBig::ONE, isize::MAX);
assert!(!expected.repr().is_infinite());
let _ = base.powi(IBig::from(isize::MAX)); // panics: "arithmetic operations with the infinity are not allowed!"
```

As an asymmetric control, `(1/2)^isize::MAX` succeeds and returns exponent `-isize::MAX`; increasing the power by one should return exponent `isize::MIN`, but returns zero.

### Root cause

The range guard in [`float/src/exp.rs`](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/exp.rs#L139-L162) estimates the result exponent through `f64`:

```rust
let base_log2 = base.log2_est() as f64;
let threshold = (isize::MAX as f64) * (B.log2_est() as f64);
let exp_f64 = i64::try_from(&exp).ok().map(|e| e as f64);
let overflows = match exp_f64 {
    Some(e) => e * base_log2 > threshold,
    None => base_log2 != 0.0,
};
```

Around `2^63`, `isize::MAX as f64`, `isize::MAX + 1`, and nearby integer exponents are indistinguishable as `f64`, so the guard cannot separate the representable endpoint from the first out-of-range exponent. Depending on the path, binary exponentiation reaches infinity and the final

```rust
Ok(res.with_precision(self.precision))   // exp.rs:187
```

rounds an infinity and panics, or the guard reports underflow at an exact representable endpoint and returns zero.

---

## Section 2 — Genuine out-of-range results ignore directed rounding

For powers genuinely beyond the exponent range, `FBig::powi` converts `Context` overflow/underflow to the same infinity/zero regardless of rounding direction:

```rust
use dashu::{float::{FBig, round::mode::Down}, integer::IBig};

let exponent = IBig::from(isize::MAX) + IBig::ONE;
let two = FBig::<Down>::from_parts(IBig::ONE, 1);
let observed = two.powi(exponent);
assert!(observed.repr().is_infinite());   // Down should return the largest finite FBig
```

### Root cause

`Context::powi` returns `FpError::Overflow`/`Underflow`, and the convenience method unwraps it with [`unwrap_fp`](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/error.rs#L131-L136):

```rust
Err(FpError::Overflow(sign))  => FBig::new(Repr::infinity_with_sign(sign), *self),
Err(FpError::Underflow(sign)) => FBig::new(Repr::zero_with_sign(sign), *self),
```

This is mode-blind. Downward rounding of a positive overflow should return the largest finite endpoint, not `+inf`; the corresponding negative-overflow and signed-underflow cases each need the endpoint selected by direction and sign.

---

## Section 3 — Negative-exponent reciprocal panic

The negative-exponent path in [`float/src/exp.rs`](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/exp.rs#L116-L127) reverses the mode, recurses, unwraps to infinity, then divides:

```rust
let pow = rev_context.unwrap_fp(rev_context.powi(base, exp.into()));
let inv = rev_context.unwrap_fp_repr(rev_context.repr_div(Repr::one(), pow.repr));
```

When the positive intermediate saturates to infinity, `repr_div(1, inf)` feeds infinity into finite-only arithmetic and panics — even though the final reciprocal (e.g. `2^isize::MIN`) is a representable finite endpoint.

---

## Expected result

- Exact powers of the radix, and exact results whose exponent equals a representable boundary, must return the corresponding finite value without rounding or panic.
- For a positive result above the range, `Down` returns the largest finite value and `Up` may return `+inf`; negative-overflow and signed-underflow cases select the endpoint by sign and direction.
- The negative-exponent path must select the directed reciprocal endpoint directly instead of materializing an infinity and dividing.
- Every finite-input range result must be marked inexact.

## Impact

Public exact operations panic or return the wrong side of the exact result at the stated exponent range, invalidating interval and conservative-bound computations, and some algebraically equivalent reciprocal forms panic rather than returning any endpoint. OpenDP's primitive adapter masks these by structurally classifying extreme primitive overflow/underflow before invoking raw `FBig::powi`.

## Suggested resolution

Rewrite `Context::powi` range handling so that it:

1. handles exact powers of the radix structurally — `(B^k)^n = B^(k·n)` — with checked integer exponent arithmetic, no `f64` estimate;
2. replaces the boolean `f64` `overflows` estimate with an exact or conservative integer bound that distinguishes endpoint equality from genuine overflow;
3. never calls `unwrap_fp` inside another arithmetic implementation — preserve the `FpError` (or a richer range relation) until the final reciprocal/rounding step is known;
4. selects the final endpoint from mathematical sign, above-vs-below-range, rounding mode, and whether the final op is a reciprocal;
5. marks every finite-input range result inexact.

## Relation to other findings

Shares the "finite range result collapsed into a mode-independent limiting value" architecture with [`DASHU-023`](DASHU-023-upward-expm1-saturates-negative-one.md) (`exp`/`exp_m1` saturation). Both would benefit from a shared range-endpoint helper that turns a typed range relation into a correctly directed result, but they are filed separately because 023 is localized to `exp_internal` and is not repaired by fixing `unwrap_fp` alone (`exp_m1` returns `Exact(-1)` directly).
