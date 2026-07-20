# Findings methodology

The `findings/` tree is a curated layer over raw libFuzzer artifacts. It is designed for upstream maintainers, not as a dump of every process that exited nonzero.

Each active finding is classified as either a `uniformity` violation of the public backend-neutral contract or a `backend_conformance` violation found by directly probing a provider API. Backend probes remain publishable when an adapter masks the problem because they explain workarounds, guard dependency upgrades, and localize ownership.

## Admission criteria

A finding is listed only when its retained input reproduces on the locked dependency baseline and the observed failure identity matches the registry. MPFR-backed comparisons are bit-for-bit for primitive directed operations. Exact arithmetic uses independent backends or mathematical identities.

Raw infrastructure failures, cancelled workers, and minimization attempts that drift to a different already-known crash are excluded.

## Deduplication

Mechanical grouping starts with the target, operation, contract, output pattern, and first backend stack frame. A semantic pass then groups manifestations only when they plausibly share one correction. Ambiguous cases remain separate. Every finding README states its grouping rationale so maintainers can split or merge it.

The owner is recorded independently as `backend`, `adapter`, `contract_design`, `oracle`, `harness`, or `resource_behavior`. Contract category says where the failure was observed; owner says what must change.

## Minimization

The retained inputs are small libFuzzer reproducers when identity-preserving minimization is possible. Generic `cargo fuzz tmin` accepts any crash and can drift between known failures in a multi-contract target, so its output is accepted only after replaying and checking the exact expected failure marker. Fixed-layout inputs that are already one encoded operation are retained intact when further minimization would erase required fields.

## Quarantine and continued discovery

`python3 fuzz/quarantine_known.py --apply` moves exact registered inputs out of active corpora into `fuzz/quarantine/corpus/`. The move is recoverable. `seed_corpus.py` also avoids recreating registered deterministic seeds. Quarantine is exact-input based: a new byte sequence that exposes the same root cause is still recorded and should be deduplicated during the next triage pass.

The one exception is the registered `exact_rational` size ceiling. DASHU-008 continues to panic after identity-preserving minimization to 3136 bytes, so continuous discovery is capped at 3000 bytes and larger corpus entries are quarantined. The full reproducer remains part of the published evidence and direct verification is unaffected. This limitation is explicit in `known_findings.json`.

## Upstream validation

The locked baseline is the authoritative reproduction environment. Before filing upstream, each finding must also be checked against the newest released library and preferably its main branch. Major-version API changes are tested in a separate worktree or temporary copy so the original evidence remains reproducible. A failure to compile against a newer API is not evidence that the defect persists.

## Generated files

- `fuzz/known_findings.json` is the reviewed source of grouping decisions.
- `fuzz/known_inputs/` contains uploadable binary reproducers.
- `findings/index.json` is the machine-readable public index.
- `findings/verification.json` records the latest baseline replay.
- Per-finding reports retain structured expected and actual values where the harness emitted them.
