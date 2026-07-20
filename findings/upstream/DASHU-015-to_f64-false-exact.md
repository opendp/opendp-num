# `IBig`/`UBig` `to_f64` reports an inexact conversion as `Exact` at `DoubleWord::MAX`

**Versions:** dashu-int 0.5.0 (still present on `master`)

## Summary
`to_f64` tags the conversion of `DoubleWord::MAX` (`2^128 - 1` on 64-bit) as `Approximation::Exact`, even though that value is not representable in f64. Anyone relying on the `Exact`/`Inexact` tag (or the `Inexact` sign for directed rounding) gets a silently wrong answer. (`to_f32_small` is not affected: it has an `is_infinite` guard, and `f32::MAX < 2^128` so a finite f32 never saturates the round-trip.)

## Reproduce
```rust
use dashu_int::IBig;
use dashu_base::Approximation;

let n = (IBig::from(1) << 128) - IBig::from(1); // 2^128 - 1, not representable in f64
println!("{:?}", n.to_f64());
```
**Actual:** `Exact(3.402823669209385e38)`  (the value is `2^128`)
**Expected:** `Inexact(3.402823669209385e38, Positive)`

The negative value behaves identically (`Exact` instead of `Inexact(_, Negative)`).

## Root cause
`integer/src/convert.rs`, `to_f64_small` (v0.5.0 lines 1130-1140):
```rust
let f = dword as f64;
let back = f as DoubleWord;              // saturating float -> int cast
match back.partial_cmp(&dword).unwrap() {
    Ordering::Equal => Exact(f),         // <-- reached for dword == DoubleWord::MAX
    ...
}
```
The round-trip exactness test uses `f as DoubleWord`, which **saturates**. For `dword == DoubleWord::MAX`, `f` rounds up to `2^128`, and `(2^128) as u128` saturates back to `u128::MAX == dword`, so `partial_cmp` returns `Equal`. It only misfires at exactly `DoubleWord::MAX`; smaller values saturate to a strictly larger `back` and are correctly `Inexact`.

## Suggested fix
Guard the one saturating case before the round-trip test — when `f` rounds up to `DoubleWord::MAX as f64` it is strictly greater than `dword`:
```rust
let f = dword as f64;
if f == DoubleWord::MAX as f64 {
    return Inexact(f, Sign::Positive);
}
let back = f as DoubleWord;
// ...existing partial_cmp round-trip...
```
Verified: `±(2^128-1).to_f64()` then reports `Inexact`, while genuinely-exact values (e.g. `1024`) still report `Exact`.
