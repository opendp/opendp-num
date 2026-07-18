# DASHU-008: Rational construction triggers internal allocation panic in GCD

Status: confirmed on the locked baseline. Confidence: high. Classification: `panic`.

Latest release check: Still reproduces with dashu-int 0.5.0.

## Summary

Constructing and reducing a fuzzed rational reaches dashu-int's divide-and-conquer GCD and panics with `internal error: not enough memory allocated`.

## Impact

Valid large integer inputs can panic during rational normalization.

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

A unique raw backend panic identified by its dashu-int memory.rs assertion and GCD stack.

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
