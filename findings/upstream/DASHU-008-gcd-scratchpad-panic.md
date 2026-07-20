# GCD panics (`not enough memory allocated`) when a Lehmer step enters Burnikel–Ziegler division

**Versions:** dashu-int 0.5.0 (reached via dashu-ratio rational reduction)

## Summary
Reducing/constructing a rational whose numerator and denominator are large integers of **similar word-length** can panic during GCD with `internal error: not enough memory allocated`. The GCD scratchpad is sized from the *initial* operand lengths, but a later euclidean step dispatches on the *current* lengths and enters divide-and-conquer division that needs scratch that was never reserved.

## Reproduce
Reduce a rational whose numerator/denominator are large integers of nearly equal word-length (`|len(a) - len(b)| ≤ 32` words, both ≳ 64 words), e.g. via `RBig::from_parts(a, b)` or `(&a).gcd(&b)`. A concrete 3136-byte reproducer input is available in the opendp-num fuzz corpus (finding DASHU-008). Crash signature:
```
thread panicked at dashu-int-0.5.0/src/memory.rs:150:
internal error: not enough memory allocated
```

## Root cause
The scratchpad is reserved **once**, from the initial lengths — `gcd_ops.rs:140` → `div::memory_requirement_exact` (`div/mod.rs:258-265`), which even returns `zero_layout()` when `rhs_len ≤ 32 || lhs_len - rhs_len ≤ 32`:
```rust
if rhs_len <= threshold::simple() || lhs_len - rhs_len <= threshold::simple() {
    memory::zero_layout()               // reservation
} else { divide_conquer::memory_requirement_exact(lhs_len, rhs_len) }
```
But every euclidean step's division dispatches on the **current** lengths (`div/mod.rs:285`):
```rust
if rhs.len() <= threshold::simple() || lhs.len() - rhs.len() <= threshold::simple() {
    simple::div_rem_in_place(...)                       // no allocation
} else { divide_conquer::div_rem_in_place(..., memory) } // allocates from scratchpad
```
As Lehmer shrinks `y` while `x` stays large, a later step can satisfy the B–Z predicate even though the initial pair did not (e.g. an initial near-equal-length pair reserves `zero_layout()`). B–Z's multiplication temporary (`div/divide_conquer.rs:127` → `mul` → `mul/karatsuba.rs:94`) then allocates from the scratchpad, and `allocate_slice_initialize` (`memory.rs:150`) `expect`s and panics.

## Suggested fix
Size the reservation for the worst-case per-step dispatch (the reservation predicate must upper-bound every step's B–Z entry), or have the euclidean step fall back to the non-allocating `simple` path when the reserved scratch is insufficient.
