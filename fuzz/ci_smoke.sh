#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"
python3 check_coverage.py
python3 seed_corpus.py
python3 verify_backend_profiles.py

FUZZ_SANITIZER="${FUZZ_SANITIZER:-none}"

for target in \
  exact_integer \
  exact_rational \
  directed_unary \
  directed_binary \
  conversions \
  backend_float_conversion \
  primitive_casts \
  alp_primitives \
  opendp_sequences
do
  cargo fuzz run --sanitizer "$FUZZ_SANITIZER" "$target" "corpus/$target" -- \
    -max_total_time="${FUZZ_SECONDS_PER_TARGET:-60}" \
    -timeout=20 \
    -rss_limit_mb=4096 \
    -artifact_prefix="artifacts/$target/" \
    -use_value_profile=1 \
    -reduce_inputs=1 \
    -print_final_stats=1
done
