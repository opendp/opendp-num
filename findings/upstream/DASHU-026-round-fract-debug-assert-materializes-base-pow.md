# Debug assertion in `Round::round_fract` materializes `B^exponent_gap`, causing unbounded allocation and boundary panics

**Versions:** dashu-float 0.5.0 and master `40f465b62e5d8f4198efc43871e3ce601d03dc93`
**Existing fix:** none known. No matching issue was found as of 2026-07-20.

## Summary

When an add/subtract aligns a tiny operand that sits far below the retained precision, the low part is represented as a sparse sticky remainder: a `±1` significand paired with an enormous denominator exponent. That pair is handed to the generic rounding helper `Round::round_fract`, whose **debug-only** assertion validates the sticky remainder by constructing `B^precision` — where `precision` is the exponent *gap*, not the requested output precision. Constructing that integer allocates memory proportional to the gap and, at the extreme, exhausts Dashu's integer allocation guard.

`FBig::exp_m1` reaches this path directly: for large-magnitude finite `x` it forms `exp(x)` and subtracts one, and the subtraction produces exactly such a sparse sticky remainder. The defect is therefore visible through `exp_m1` but lives in generic rounding, and any operation that produces a sparse sticky remainder with a huge denominator exponent can trigger it.

The behavior is **profile-dependent**: the assertion is a `debug_assert!`, so release builds compile it out, and the actual rounding logic never materializes the bound (see "Why release is unaffected" below).

## Manifestations

Both of the following are the same defect at different magnitudes:

1. **Allocation growth (resource-exhaustion).** At `x = 1e8` and `x = -1e8`, precision 2, `Up` rounding, an instrumented counting allocator measures more than 20 MB of peak live heap per debug call and less than 1 KB per release call. Mutation fuzzing over the raw API grew past a 2 GB worker RSS limit.

   ```text
   debug:  20,422,236 peak live heap bytes
   release:       788 peak live heap bytes
   ```

   The `2^~1.44e8`-bit integer implied by `ediff ≈ 1e8 / ln(2)` accounts for the ~18–20 MB measured.

2. **Boundary panic (availability).** A generated matrix of 129 consecutive `f64` inputs around the point where `x / ln(2)` reaches `isize::MIN`, eight precisions, and both directed modes exercises 2,064 cases and observes 1,035 panics in a debug build and none in release. Here the implied power is so large it reaches Dashu's integer allocation guard. The minimized member is `f64::from_bits(0xc3d62e42fefa39ef)`, precision 2, `Up` rounding; the panic text is `out of memory`.

## Reproduce

```bash
# Allocation growth at ±1e8
cargo run --example reproduce_dashu_027
cargo run --release --example reproduce_dashu_027

# 2,064-case negative-boundary sweep
cargo run --example reproduce_dashu_026
cargo run --release --example reproduce_dashu_026
```

Expected output:

```text
DASHU-026 boundary sweep complete: profile=debug cases=2064 panics=1035
DASHU-026 boundary sweep complete: profile=release cases=2064 panics=0
DASHU-027 reproduced: exp_m1(1e8) profile=debug peak_heap_bytes=20422236
```

## Root cause

The low part of an aligned subtraction is set to a sparse sticky remainder in [`float/src/add.rs`](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/add.rs#L466) (line 466, and the symmetric branch at line 367):

```rust
low = (lhs.significand.signum(), ediff);
```

`low.0` is a `±1` significand; `low.1` is `ediff`, the (potentially enormous) exponent gap. This is intended to avoid constructing an integer spanning that gap. The pair is passed to rounding at [line 306](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/add.rs#L306):

```rust
let adjust = R::round_fract::<B>(&significand, low.0, low.1);
```

so `round_fract` receives `fract = ±1` and `precision = ediff`. Its first line, in [`float/src/round.rs`](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/round.rs#L108) (line 108), is:

```rust
// this assertion is costly, so only check in debug mode
debug_assert!(fract.clone().unsigned_abs() < UBig::from_word(B).pow(precision));
```

To check that a `±1` remainder is below `B^ediff`, the assertion constructs the full `B^ediff` — an integer with roughly `ediff·log2(B)` bits. That is the entire measured allocation and the source of the allocation-guard panic.

### Why release is unaffected

Beyond the `debug_assert!`, the only other site that could construct `B^precision` is the exact fallback in the rounding comparison at [round.rs line 126](https://github.com/cmpute/dashu/blob/40f465b62e5d8f4198efc43871e3ce601d03dc93/float/src/round.rs#L126):

```rust
if lb + 0.999 > b_ub * precision as f32 {
    Ordering::Greater
} else if ub + 1.001 < b_lb * precision as f32 {
    Ordering::Less
} else {
    (fmag << 1).cmp(&UBig::from_word(B).pow(precision))   // line 126
}
```

For a `±1` remainder against a huge `precision`, the `log2_bounds` coarse comparison (lines 121–124) resolves the ordering to `Less` and short-circuits, so line 126 is never reached. Release builds therefore allocate less than 1 KB — proving the cost is the `debug_assert!` at line 108, not the alignment or subtraction itself.

## Expected result

Compilation profile must not change whether a finite public operation panics, and memory use must be bounded by the requested work precision, growing at most logarithmically with the result exponent. Validating a sparse sticky remainder must not materialize `B^gap`.

## Impact

A compact finite input at tiny precision drives memory into the tens of megabytes to gigabytes in debug and libFuzzer builds, terminating debug services and fuzz workers; abort-on-panic fuzz profiles stop at the first member of the boundary region and cannot explore later cases. The returned significand retains only two bits, yet the numeric value controls resource consumption. OpenDP's primitive adapter preclassifies sufficiently large/negative primitive `expm1` inputs and masks the raw backend path.

## Suggested resolution

Replace the constructed upper bound with a digit-count check, which is equivalent for an integer `fract`:

> `|fract| < B^precision`  ⟺  `fract` has at most `precision` base-`B` digits.

So the assertion can inspect the number of base-`B` digits of `fract` instead of building `B^precision`, for example:

```rust
debug_assert!(fract.is_zero() || base_b_digit_len::<B>(&fract) <= precision);
```

(The exact helper name in `dashu-int` may differ; the invariant is the point.) Add the ±1e8 allocation probe and the 2,064-case boundary matrix as regression tests in both debug and release profiles.

## Relation to other findings

Distinct from [`DASHU-023`](DASHU-023-upward-expm1-saturates-negative-one.md), which is a *non-panicking* directed-rounding error after `floor(x / ln B)` no longer fits `isize`. This defect occurs in the adjacent boundary region and in the large-magnitude interior, and is a generic-rounding allocation/assertion problem rather than a range-reduction saturation problem.
