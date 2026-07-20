# DASHU-008: Rational reduction: Lehmer GCD reaches Burnikel-Ziegler division that overflows a pre-sized scratchpad

Status: confirmed on the locked baseline. Confidence: high. Classification: `panic`.

Contract: `uniformity`. Owner: `backend`. Masked by adapter: `false`.

Latest release check: Still reproduces with dashu-int 0.5.0.

## Summary

Constructing/reducing a fuzzed rational calls plain GCD (dashu-ratio reduce -> gcd_in_place, Lehmer). The scratchpad is sized ONCE from the initial operand lengths (gcd_ops.rs:140 -> div::memory_requirement_exact, div/mod.rs:258-265 - which returns zero_layout when the initial pair is within threshold::simple()=32). But each euclidean step's division dispatches on the CURRENT lengths (div/mod.rs:285): as Lehmer shrinks y while x stays large, a later lopsided step enters Burnikel-Ziegler divide-and-conquer division, whose multiplication temporary (divide_conquer.rs:127 -> mul -> karatsuba.rs:94) allocates scratch that was never reserved, so allocate_slice_initialize's expect panics at dashu-int-0.5.0/src/memory.rs:150 ("internal error: not enough memory allocated").

## Impact

Valid large integer inputs can abort the process during rational normalization.

## Tested baseline

`dashu 0.5.0`, `dashu-float 0.5.0`, `dashu-int 0.5.0`, `dashu-ratio 0.5.0`, `malachite 0.9.2`, `rug 1.30.0`.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
cargo fuzz run --sanitizer none exact_rational findings/dashu-int/DASHU-008/inputs/DASHU-008.input
cargo fuzz run --sanitizer none exact_rational findings/dashu-int/DASHU-008/inputs/DASHU-008-b.input
```

## Evidence

- `DASHU-008.input`: 3136 bytes, SHA-256 `4078a4a54d400215b345560eec0bf32b986533a1d5f7c2874c792226ab608825`; expects `internal error: not enough memory allocated`
- `DASHU-008-b.input`: 4005 bytes, SHA-256 `c4aa7ce3dc1650e56a295419082487a28174c1191b66c5d375ef5ad8eabad719`; expects `internal error: not enough memory allocated`

## Deduplication rationale

One dashu-int allocation defect: the scratchpad reservation (initial lengths) and the per-step division dispatch (current lengths) use divergent predicates. Traced by source audit; both retained reproducers hit memory.rs:150.

## Reporting note

This report describes behavior observed through opendp-num's backend-neutral uniformity contract. The retained evidence identifies whether the cause is in a provider or in the adapter.
