# `exp_m1` allocates memory proportional to huge exponent magnitude in debug and fuzz builds

**Versions:** dashu-float 0.5.0 and master `40f465b62e5d8f4198efc43871e3ce601d03dc93`  
**Existing fix:** none known. No matching issue was found as of 2026-07-20.

## Summary

For large-magnitude finite inputs, `FBig::exp_m1` computes `exp(x)` and subtracts one. In debug and libFuzzer builds, aligning that subtraction allocates memory proportional to the result exponent gap rather than the requested precision. The behavior occurs for both signs.

At inputs `1e8` and `-1e8`, precision 2, and `Up` rounding, the supplied counting allocator measures more than 20 MB peak live heap for each debug call and less than 1 KB for each release call.

```text
debug:  20,422,236 peak live heap bytes
release:       788 peak live heap bytes
```

Mutation fuzzing over the same raw API grew beyond a 2 GB worker RSS limit before the target was changed to isolate this known resource class.

## Reproduce

```bash
cargo run --example reproduce_dashu_027
cargo run --release --example reproduce_dashu_027
```

The reproducer uses a global counting allocator, so it measures live requested heap directly and does not depend on platform-specific RSS reporting.

## Expected result

Memory use should be bounded by the requested output/work precision and grow at most logarithmically with the result exponent. Subtracting one from a value whose exponent is vastly larger than the retained precision should be handled as a rounding/sticky-bit decision without materializing the entire exponent gap.

## Impact

A compact finite input at tiny precision can drive memory use into gigabytes, terminating debug services and fuzz workers. The numeric value controls resource consumption even though the returned significand retains only two bits.

## Relation to other findings

This is separate from DASHU-026, which is a negative range-boundary panic. DASHU-027 is an allocation-complexity defect in the final subtraction of one for either input sign. It is also separate from DASHU-023's incorrect directed saturation.

## Suggested resolution

Specialize the final `exp(x) - 1` step when the exponent gap proves that one cannot affect retained digits except through the rounding sticky bit. Add allocation-bounded debug tests at increasing positive inputs and fixed small precision.
