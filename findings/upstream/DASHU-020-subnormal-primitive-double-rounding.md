# `FBig`/`DBig` conversion double-rounds values adjacent to primitive subnormal halfways

**Versions:** dashu-float 0.5.0  
**Existing fix:** open PR [#91, “Round to_f64/to_f32 once, to the subnormal-aware precision”](https://github.com/cmpute/dashu/pull/91) (`afdc90a`)

## Summary

`FBig::to_f32` and `to_f64` first round to a fixed 24/53-bit binary intermediate. The primitive encoder then rounds that intermediate again to the smaller, magnitude-dependent significand width of a subnormal. A value just to one side of a subnormal halfway can therefore land exactly on the halfway during the first rounding and be sent to the wrong adjacent subnormal by the second rounding.

This affects binary `FBig` and decimal `FBig`/`DBig` construction. The minimized example below returns f32 bit pattern 4 when correct nearest-even rounding is bit pattern 3.

## Reproduce

```rust
use dashu::{
    base::Approximation,
    float::{FBig, round::mode::HalfEven},
    integer::IBig,
};

fn main() {
    // 0x0dff_ffff * 2^-175 is just below 3.5 * 2^-149.
    let source = FBig::<HalfEven, 2>::from_parts(IBig::from(0x0dff_ffffu32), -175);
    let approximation = source.to_f32();
    let rounded = match approximation {
        Approximation::Exact(value) | Approximation::Inexact(value, _) => value,
    };

    println!("{approximation:?}, bits={}", rounded.to_bits());
    assert_eq!(rounded.to_bits(), 3);
}
```

With dashu-float 0.5.0, in both debug and release profiles:

```text
Inexact(6e-45, NoOp), bits=4
assertion failed: left == right
  left: 4
 right: 3
```

## Expected result

Every positive f32 subnormal with bit pattern `k` is `k * 2^-149`. Here:

```text
0x0dff_ffff * 2^-175
= (3.5 - 2^-26) * 2^-149
```

The exact input lies strictly below the halfway between bit patterns 3 and 4, so nearest-even rounding must return bit pattern 3. Dashu returns bit pattern 4, one ULP above the correctly rounded result.

The differential corpus also retains equivalent f64 and decimal-base witnesses. All four construction/format variants reduce to the same fixed-width-intermediate double-rounding mechanism.

## Root cause

In dashu-float 0.5.0, `float/src/convert.rs`, `FBig::to_f32` and `to_f64` create a fixed-width context and then perform two rounding-capable stages:

```rust
let context = Context::<R>::new(24); // 53 for f64
context
    .convert_base::<B, 2>(self.repr.clone(), None)
    .and_then(|v| context.repr_round_ref(&v))
    .and_then(|v| v.into_f32_internal())
```

The fixed width is suitable for normal primitives, but a subnormal at this magnitude retains fewer than 24/53 significant bits. `into_f32_internal`/`into_f64_internal` consequently performs another rounding to the subnormal grid.

## Suggested resolution

PR #91 already implements the appropriate strategy: convert through a round-to-odd binary representation, derive the target precision from the value’s magnitude, and round once to that subnormal-aware width before encoding. Its regression tests cover both sides of f32/f64 halfways and both binary and decimal construction.

This draft is therefore primarily supporting evidence for reviewing/merging PR #91 rather than a request for a separate competing fix.
