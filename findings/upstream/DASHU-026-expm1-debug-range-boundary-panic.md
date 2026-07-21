# `exp_m1` panics near the negative exponent-range boundary in debug and fuzz builds

**Versions:** dashu-float 0.5.0 and master `40f465b62e5d8f4198efc43871e3ce601d03dc93`  
**Existing fix:** none known. No matching issue was found as of 2026-07-20.

## Summary

`FBig::exp_m1` panics for finite inputs around the point where `x / ln(2)` reaches `isize::MIN` when debug assertions are enabled. A generated matrix of 129 consecutive `f64` inputs, eight precisions, and both directed modes exercises 2,064 cases and observes 1,035 panics in a debug build. The same matrix observes no panics in a normal release build.

The minimized matrix member is `f64::from_bits(0xc3d62e42fefa39ef)`, precision 2, `Up` rounding. The panic text is:

```text
out of memory
```

## Reproduce

```bash
cargo run --example reproduce_dashu_026
cargo run --release --example reproduce_dashu_026
```

Expected reproducer output:

```text
DASHU-026 boundary sweep complete: profile=debug cases=2064 panics=1035
DASHU-026 boundary sweep complete: profile=release cases=2064 panics=0
```

## Expected result

Compilation profile must not change whether a finite public operation panics. These inputs are in the extreme negative range, so `exp_m1` should return the correctly directed value near `-1` or a documented typed range result.

## Impact

Finite public inputs can abort debug and libFuzzer workers. Besides the direct availability problem, abort-on-panic fuzz profiles stop at the first member of this broad boundary region and cannot explore later cases unless each boundary probe is isolated.

## Relation to the saturation issue

This is separate from DASHU-023. That issue covers a non-panicking branch after `floor(x / ln(2))` no longer fits `isize`, where upward rounding is discarded. This issue occurs in the adjacent boundary region and reaches Dashu's integer allocation guard before returning a value.

## Suggested resolution

Audit the `isize::MIN` range-reduction and shift/subtraction path with checked exponent arithmetic before any integer allocation-size conversion. Add the supplied cross-product matrix as a regression test in both debug and release profiles.

