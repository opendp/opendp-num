#!/usr/bin/env python3
"""Run the deterministic raw-Dashu extreme suite without mutating live corpora."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FUZZ = ROOT / "fuzz"
TARGET = "backend_float_extremes"


def run(command: list[str], *, cwd: Path = ROOT) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--mutation-runs",
        type=int,
        default=0,
        help="after the complete seed replay, run this many additional mutations in the temp corpus",
    )
    args = parser.parse_args()
    if shutil.which("cargo") is None:
        raise SystemExit("cargo is not installed")

    # Normal Cargo profiles unwind panics, so these generated audits can
    # inspect every boundary case instead of losing the process at the first
    # known Dashu failure.
    for example in (
        "audit_dashu_pr2801",
        "reproduce_dashu_024",
        "reproduce_dashu_025",
        "reproduce_dashu_026",
        "reproduce_dashu_027",
    ):
        run(["cargo", "run", "--quiet", "--example", example])
        run(["cargo", "run", "--quiet", "--release", "--example", example])

    # Regenerate the corpus in a fresh directory. This ensures the reported
    # count is the full deterministic matrix, unaffected by prior mutations or
    # corpus minimization.
    with tempfile.TemporaryDirectory(prefix="opendp-raw-extremes-") as temporary:
        corpus_root = Path(temporary) / "corpus"
        run(
            [
                sys.executable,
                str(FUZZ / "seed_corpus.py"),
                "--corpus-root",
                str(corpus_root),
            ]
        )
        corpus = corpus_root / TARGET
        seed_count = sum(path.is_file() for path in corpus.iterdir())
        if seed_count != 5_888:
            raise SystemExit(f"expected 5,888 raw extreme seeds, found {seed_count}")

        run(
            [
                "cargo",
                "+nightly-2026-02-19",
                "fuzz",
                "build",
                "--sanitizer",
                "none",
                TARGET,
            ]
        )
        # libFuzzer numbers INITED as seed_count + 1. Stop there so every seed
        # is replayed exactly once and no mutation is written into the temp
        # corpus. Mutation campaigns remain the job of run_campaign.py.
        run(
            [
                "cargo",
                "+nightly-2026-02-19",
                "fuzz",
                "run",
                "--sanitizer",
                "none",
                TARGET,
                str(corpus),
                "--",
                f"-runs={seed_count + 1}",
                "-max_len=16",
                "-timeout=10",
                "-rss_limit_mb=2048",
            ]
        )
        if args.mutation_runs:
            run(
                [
                    "cargo",
                    "+nightly-2026-02-19",
                    "fuzz",
                    "run",
                    "--sanitizer",
                    "none",
                    TARGET,
                    str(corpus),
                    "--",
                    f"-runs={seed_count + 1 + args.mutation_runs}",
                    "-max_len=16",
                    "-timeout=10",
                    "-rss_limit_mb=2048",
                ]
            )

    print(
        "raw extreme verification passed: 5,888 seeds plus debug/release boundary audits"
        f" and {args.mutation_runs:,} mutations"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
