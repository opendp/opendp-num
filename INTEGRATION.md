# OpenDP integration sequence

1. Add `opendp-num` to `rust/Cargo.toml` workspace members and depend on it from `opendp` with the `dashu` feature.
2. Move direct exact-number imports behind backend-associated aliases in one OpenDP module.
3. Migrate correctness-sensitive arithmetic first: rational division, rational-to-float conversion, directed primitive arithmetic, and directed transcendentals.
4. Move ALP's precision truncation, floor/fraction decomposition, reciprocal probability, and parameter comparison behind narrow semantic functions rather than exposing a general `BigFloat` API.
5. Keep ordinary primitive `Alerting*` traits in OpenDP; they are broader application traits, not backend capabilities.
6. Add an architecture check rejecting `dashu`, `malachite`, and `rug` imports outside `opendp-num`.
7. Run the contract suite under each individual feature and `all-backends`.
8. Run MPFR differential tests and fuzzing in dedicated Linux jobs so ordinary Windows packaging remains C-free.
9. Extend a backend's directed capabilities operation-by-operation only as needed. Missing support remains a missing trait implementation, never a runtime fallback.
10. Change OpenDP's selected backend only after the full mechanism suite, retained fuzz corpus, and performance thresholds pass.

## First compiler-validation pass

```bash
cargo fmt --all -- --check
cargo check -p opendp-num --all-features --all-targets
cargo clippy -p opendp-num --all-features --all-targets -- -D warnings
cargo test -p opendp-num --no-default-features --features dashu
cargo test -p opendp-num --no-default-features --features malachite
cargo test -p opendp-num --no-default-features --features mpfr
cargo test -p opendp-num --all-features
cargo bench -p opendp-num --all-features --no-run
```

## Fuzz-target build and smoke pass

```bash
cargo install cargo-fuzz
cd opendp-num/fuzz
./check_coverage.py
./ci_smoke.sh
```

The coverage checker is pinned to the audited OpenDP commit in `operation_manifest.json`. Re-audit and update that manifest whenever numerical call sites change.

## Continuous qualification

Run the high-throughput numerical campaign on persistent Linux storage:

```bash
cd opendp-num/fuzz
./run_campaign.py --cmin-every 48
```

Run a smaller periodic native-memory campaign:

```bash
./run_campaign.py --sanitizer address --slice-seconds 600 --rounds 1
```

Inspect findings:

```bash
./summarize_reports.py
```

Retain and periodically back up:

```text
fuzz/corpus/
fuzz/artifacts/
fuzz/reports/
fuzz/logs/
```

The included `opendp-num-fuzz.service` is an example systemd unit. Adjust paths, CPU/memory limits, and the service account before installation.
