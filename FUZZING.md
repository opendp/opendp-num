# Continuous fuzzing

The fuzz suite is organized by input shape rather than by backend. This keeps mutations productive while covering the complete numerical surface audited from OpenDP commit `8cc809d38ac620e42bc6ca1c7ed3a1c19b2c0b02`.

The machine-readable inventory is `fuzz/operation_manifest.json`. Its checker fails when a listed operation loses its harness or a fuzz target is added without being classified:

```bash
cd fuzz
./check_coverage.py
```

## Coverage

| Target | Operations and contracts |
|---|---|
| `exact_integer` | signed add/subtract/multiply/negate/absolute value/ordering; natural division/remainder; Euclidean GCD; `UBig::from_be_bytes`; bit length; inverse and distributive identities; Dashu vs Malachite vs Rug |
| `exact_rational` | canonical construction/decomposition; add/subtract/multiply/checked divide/negate; floor and nearest rounding with ties away from zero; square/power-of-two; ordering; Dashu vs Rug |
| `directed_binary` | correctly rounded `f32` and `f64` add/subtract/multiply/divide in both directions; Dashu vs MPFR bit-for-bit; lower bound no greater than upper bound |
| `directed_unary` | correctly rounded `sqrt`, `ln`, `log2`, `ln1p`, `exp`, `expm1`, and signed-integer power for `f32` and `f64`; Dashu vs MPFR bit-for-bit; lower bound no greater than upper bound |
| `conversions` | rational/integer/natural to `f32`/`f64` under down/nearest/up rounding; exact `f32`/`f64` to rational; directed `f64` to `f32` |
| `primitive_casts` | exact and saturating `IBig`/`UBig` conversion to every Rust primitive integer type, concentrated at each target type’s minimum and maximum |
| `alp_primitives` | Dashu `with_precision` under Down/Up/Zero; exact floor/fraction decomposition; upward-rounded reciprocal probability; parameter comparison; ALP scale/truncate/multiply/floor/fraction pipeline |
| `opendp_sequences` | bytecode-generated compositions of every directed arithmetic and transcendental operation, checked after every step against MPFR |

The manifest currently records 51 operation-level contracts across exact integers, naturals, rationals, conversions, directed primitive arithmetic, ALP-specific `FBig` behavior, and composed expressions. When OpenDP adds another numerical operation, update the manifest and add the operation to an isolated target plus `opendp_sequences` when composable.

## Search-space strategy

The harnesses deliberately combine several kinds of input generation:

- raw bit mutation for full IEEE `f32`/`f64` coverage, including all NaN payloads;
- weighted selection of signed zeros, subnormal boundaries, normal boundaries, infinities, adjacent ULPs, and exponential overflow/underflow neighborhoods;
- arbitrary-precision integers derived independently from up to 4096 input bytes;
- targeted powers of two through exponent 8192;
- rational numerators and denominators mutated independently;
- signed integer exponents concentrated around `-1`, `0`, `1`, mantissa widths, exponent limits, and large magnitudes;
- operation bytecode sequences of up to 32 steps;
- target-specific dictionaries and deterministic boundary corpora;
- libFuzzer comparison tracing plus `-use_value_profile=1`.

Each core runs a separate libFuzzer process with a unique log. Workers for the same target share a persistent corpus and reload discoveries every five seconds. This avoids libFuzzer `-jobs` log collisions while retaining cross-worker corpus sharing.

## One-command continuous campaign

Install a nightly Rust toolchain and `cargo-fuzz`, then run:

```bash
cargo install cargo-fuzz
cd fuzz
./run_campaign.py
```

The default campaign:

- uses all available logical cores;
- runs indefinitely in 30-minute slices;
- gives more processes to directed rounding and composed-expression targets;
- rotates targets when fewer cores than targets are available;
- keeps persistent per-target corpora across slices and restarts;
- seeds high-value boundary inputs before every launch;
- runs with sanitizers disabled for numerical throughput;
- enables value profiling and input reduction;
- restarts the campaign after a violation so unrelated search continues;
- deletes successful worker logs by default while retaining all failure logs and structured reports;
- prunes retained logs after 14 days by default to prevent unbounded disk growth;
- stops on build or infrastructure failures instead of looping uselessly.
- moves exact reproducers registered in `known_findings.json` out of active corpora before seeding, so known crashes do not immediately block new discovery.

Useful options:

```bash
# Four cores, five-minute slices
./run_campaign.py --cores 4 --slice-seconds 300

# Run only correctness-sensitive floating-point targets
./run_campaign.py \
  --target directed_unary \
  --target directed_binary \
  --target conversions \
  --target alp_primitives

# Periodic memory-safety campaign
./run_campaign.py --sanitizer address --slice-seconds 600

# Stop at the first numerical violation
./run_campaign.py --stop-on-violation

# Periodically minimize every corpus
./run_campaign.py --cmin-every 48

# Retain successful logs as well as failures
./run_campaign.py --keep-clean-logs --log-retention-days 30

# Print the exact cargo-fuzz commands without executing them
./run_campaign.py --dry-run --rounds 1

# Deliberately replay known findings as part of a campaign
./run_campaign.py --include-known-findings
```

The pure-Rust numerical correctness campaign defaults to `--sanitizer none` for throughput. Run a smaller periodic `--sanitizer address` campaign because Rug/MPFR and transitive dependencies include native code.

## Failure and error logging

A numerical contract failure writes the raw input and a structured report before panicking:

```text
fuzz/reports/<target>/<stable-id>.input
fuzz/reports/<target>/<stable-id>.json
```

The JSON record contains:

- target, operation, and human-readable failed contract;
- primitive bit patterns or exact decimal operands;
- rounding direction and power exponent;
- Dashu/Malachite/MPFR outputs or error classifications;
- sequence step and expression trace when applicable;
- exact FBig representation for ALP failures;
- path to the raw reproducer.

The target also emits a searchable line:

```text
OPENDP_NUM_VIOLATION target=... operation=... reason=... report=...
```

The campaign supervisor separately classifies and records contract violations, execution timeouts, memory-budget failures, crashes/sanitizer findings, and build/infrastructure failures under:

```text
fuzz/reports/runner/
```

Failure workers retain complete stdout/stderr under (successful logs are deleted unless `--keep-clean-logs` is set):

```text
fuzz/logs/<target>/
```

Summarize and group findings by target, operation, and reason:

```bash
./summarize_reports.py
./summarize_reports.py --json
./summarize_reports.py --fail-if-any
```

## Reproduction and minimization

```bash
cargo fuzz run --sanitizer none directed_unary \
  fuzz/reports/directed_unary/<id>.input

cargo fuzz tmin directed_unary \
  fuzz/reports/directed_unary/<id>.input
```

To preserve a regression, copy the minimized input into the corresponding corpus and add a deterministic unit/property test explaining the violated contract.

For publication-oriented triage, the reviewed registry and curated output are managed separately:

```bash
./verify_findings.py
./triage_findings.py
./quarantine_known.py --apply
```

See `findings/METHODOLOGY.md` for the identity-preserving minimization and deduplication policy.

## CI strategy

Use three layers:

1. **Pull requests:** `./ci_smoke.sh`, normally 30–60 seconds per target. This verifies the coverage manifest, builds every target, and replays retained corpora.
2. **Continuous numerical runner:** `./run_campaign.py --sanitizer none` on persistent Linux storage for maximum execution rate.
3. **Periodic native-memory runner:** a shorter `./run_campaign.py --sanitizer address` campaign.

Retain and back up:

```text
fuzz/corpus/
fuzz/artifacts/
fuzz/reports/
fuzz/logs/
```

Do not make ordinary Windows packaging jobs depend on MPFR. The differential fuzz campaign belongs on a dedicated Linux runner.
