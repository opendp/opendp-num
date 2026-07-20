# opendp-num

A backend-neutral bignum uniformity testbed and numerical capability layer. It keeps provider-native number types and makes equivalent mathematical inputs, rounding, and error behavior directly comparable across providers.

## Design

- `Backend` associates a provider with native natural, integer, and rational types.
- `ExactBinary<Op, T>` and `ExactUnary<Op, T>` are implemented on zero-sized backend markers, so every provider returns its native owned type while preserving efficient borrowed or lazy evaluation internally.
- `CheckedBinary`, `Convert`, `FromParts`, and `IntoParts` cover semantics that standard operators cannot express safely.
- Directed operations are atomic: a provider implements only supported `(operation, primitive type)` combinations.
- `DirectedPowI<T, E>` accepts a borrowed provider-native exponent; Dashu `IBig` and Rug `Integer` paths are not narrowed to a primitive integer.
- All providers may be enabled together for differential testing; there is no mutually exclusive active-backend feature.
- Uniformity targets test the public backend-neutral semantics; backend-conformance targets directly probe provider states and APIs that adapters can mask.
- Exact rational/grid comparison is preferred for primitive conversion and basic arithmetic; MPFR remains the independent oracle for transcendental operations.

## Current support

| Capability | Dashu | Malachite | Rug/MPFR |
|---|---:|---:|---:|
| Exact borrowed integer operators | yes | yes | yes |
| Exact borrowed rational operators | yes | basic operators | yes |
| Canonical rational construction/decomposition | yes | pending adapter | yes |
| Checked rational division | corrected adapter | pending adapter | yes |
| Directed primitive add/subtract/multiply/divide | f32/f64 | pending qualification | f32/f64 oracle |
| Directed sqrt/ln/log2/ln1p/exp/expm1 | f32/f64 | intentionally omitted until released/qualified | f32/f64 oracle |
| Directed integer power | f32/f64 with `IBig` exponent | f32/f64 with `i32` pending big-exponent adapter | f32/f64 with `Integer` exponent |
| Directed exact-number conversion | rational/integer/natural to f32/f64 | pending qualification | rational/integer to f32/f64 |
| Raw arbitrary-float conversion probe | `FBig`/`DBig` to f32/f64 | comparable provider probe pending | oracle/reference role |

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

The campaign currently has ten classified targets. Its schema-2 manifest contains six typed P0 witnesses for arbitrary-precision power and raw Dashu float conversion, plus 51 legacy operation audits inherited from the OpenDP surface review. Those legacy needle checks remain useful regression coverage but are not claimed as complete typed proof of the numerical surface. The campaign uses per-core processes, persistent shared corpora, boundary seeds, value profiling, structured violation reports, runner-level timeout/crash reports, and report aggregation.

Confirmed, conservatively deduplicated findings live under [`findings/`](findings/). They identify whether a violation is a public uniformity failure or a direct backend-conformance failure, who owns the cause, and whether the adapter masks it. Raw fuzzer reports remain separate from this publishable evidence layer. Inputs that turned out to be `opendp-num` adapter defects rather than Dashu bugs have been fixed in `src/backend/dashu.rs` and retained under [`findings/caused-by-adapter/`](findings/caused-by-adapter/).

See [FUZZING.md](FUZZING.md) for the complete operation matrix, continuous runner, corpus strategy, logging format, and reproduction workflow.

The locked Dashu 0.5 baseline is validated with unit tests, typed-witness and legacy coverage checks, complete fuzz-target builds, retained-finding replay, and bounded campaign rounds. Raw `FBig`/`DBig` probing confirms that this baseline is affected by the conversion defects addressed by open Dashu PR #91; they are retained as DASHU-020 and DASHU-021 rather than hidden by the adapter. The machine-readable replay record is [`findings/verification.json`](findings/verification.json).
