# High-precision `DBig::to_f64` panics in debug builds during base-changing division

**Versions:** dashu-float 0.5.0  
**Existing fix:** open PR [#91, “Round to_f64/to_f32 once, to the subnormal-aware precision”](https://github.com/cmpute/dashu/pull/91) (`afdc90a`)

## Summary

Converting a valid, finite decimal `DBig` with a significand wider than the target primitive precision trips a debug assertion in `Context::repr_div`. Release builds disable the assertion and complete, so conversion has profile-dependent panic behavior.

## Reproduce

```rust
use dashu::{float::DBig, integer::IBig};

fn main() {
    let significand = IBig::from(1_234_567_890_123_456_789_012_345_678_901u128);
    let source = DBig::from_parts(significand, -13);
    println!("{:?}", source.to_f64());
}
```

With dashu-float 0.5.0 in the default debug profile:

```text
thread 'main' panicked at dashu-float-0.5.0/src/div.rs:313:9:
assertion failed: lhs.digits() <= self.precision + rhs.digits()
```

The same program completes in release mode:

```text
Inexact(1.2345678901234568e17, AddOne)
```

## Root cause

Primitive conversion creates a 53-bit context and changes the source from base 10 to base 2. The base-changing division can receive a dividend whose quotient already has more digits than the requested precision, but `Context::repr_div` assumes that the caller bounded it:

```rust
// this method don't deal with the case where lhs significand is too large
debug_assert!(lhs.digits() <= self.precision + rhs.digits());
```

The input is a valid `DBig`; over-wide source precision is expected for arbitrary-precision-to-primitive conversion. It should be rounded, not rejected by an internal assertion. Removing the assertion alone would only hide the profile difference: the oversized quotient must be rounded while preserving the division remainder so tie classification remains correct.

## Suggested resolution

PR #91 already handles this case in `repr_div`. When the quotient exceeds the requested precision, it splits off the low digits and folds both those digits and the original division remainder into the rounding numerator. This avoids the debug panic and prevents the corresponding release path from double-rounding an over-wide quotient.

The PR includes this reproducer as `123456789012345678.9012345678901` and several additional high-precision decimal cases. This draft is therefore supporting evidence for reviewing/merging PR #91, not a request for a separate fix.
