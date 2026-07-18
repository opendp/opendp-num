# opendp-num

A narrow numerical capability layer for OpenDP. It keeps provider-native number types and adds only contracts whose semantics must be normalized.

## Design

- `Backend` associates a provider with native natural, integer, and rational types.
- `ExactBinary<Op, T>` and `ExactUnary<Op, T>` are implemented on zero-sized backend markers, so every provider returns its native owned type while preserving efficient borrowed or lazy evaluation internally.
- `CheckedBinary`, `Convert`, `FromParts`, and `IntoParts` cover semantics that standard operators cannot express safely.
- Directed operations are atomic: a provider implements only supported `(operation, primitive type)` combinations.
- All providers may be enabled together for differential testing; there is no mutually exclusive active-backend feature.
- MPFR is both a usable backend and the bit-for-bit oracle for directed primitive results.

## Current support

| Capability | Dashu | Malachite | Rug/MPFR |
|---|---:|---:|---:|
| Exact borrowed integer operators | yes | yes | yes |
| Exact borrowed rational operators | yes | basic operators | yes |
| Canonical rational construction/decomposition | yes | pending adapter | yes |
| Checked rational division | corrected adapter | pending adapter | yes |
| Directed primitive add/subtract/multiply/divide | f32/f64 | pending qualification | f32/f64 oracle |
| Directed sqrt/ln/log2/ln1p/exp/expm1/powi | f32/f64 | intentionally omitted until released/qualified | f32/f64 oracle |
| Directed exact-number conversion | rational/integer/natural to f32/f64 | pending qualification | rational/integer to f32/f64 |

Omitted implementations are compile-time absence, not runtime fallback.

## Validation

```bash
cargo test --all-features
cargo bench --all-features
cd fuzz
./check_coverage.py
./ci_smoke.sh
./run_campaign.py
```

The continuous campaign covers 51 audited OpenDP operation contracts across eight targets, including ALP-specific precision/floor/fraction behavior and composed directed-operation sequences. It uses per-core processes, persistent shared corpora, boundary seeds, value profiling, structured violation reports, runner-level timeout/crash reports, and report aggregation.

Confirmed, conservatively deduplicated upstream findings live under [`findings/`](findings/). Raw fuzzer reports are intentionally kept separate from this publishable evidence layer.

See [FUZZING.md](FUZZING.md) for the complete operation matrix, continuous runner, corpus strategy, logging format, and reproduction workflow.

The Dashu 0.5 baseline has been validated with `cargo test --all-features`, the 51-operation coverage audit, complete fuzz-target builds, retained-finding replay, and an all-target campaign round. The machine-readable verification record is [`findings/verification.json`](findings/verification.json).
