#!/usr/bin/env python3
"""Replay registered findings and verify their expected failure identities."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shlex
import subprocess
import tempfile
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def resolved_dependencies(lockfile: Path) -> dict[str, str]:
    wanted = {"dashu", "dashu-float", "dashu-int", "dashu-ratio", "malachite", "rug"}
    payload = tomllib.loads(lockfile.read_text())
    return {
        package["name"]: package["version"]
        for package in payload["package"]
        if package["name"] in wanted
    }


def interesting_excerpt(text: str, expected: str) -> list[str]:
    needles = (expected, "OPENDP_NUM_VIOLATION", "panicked at", "SUMMARY: libFuzzer")
    lines = [line.strip() for line in text.splitlines() if any(needle in line for needle in needles)]
    return lines[-12:]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", type=Path, default=ROOT / "known_findings.json")
    parser.add_argument("--output", type=Path, default=ROOT.parent / "findings" / "verification.json")
    parser.add_argument("--sanitizer", default="none", choices=("none", "address"))
    parser.add_argument("--label", default="locked-baseline")
    args = parser.parse_args()

    registry = args.registry.resolve()
    payload = json.loads(registry.read_text())
    results = []
    failures = 0
    with tempfile.TemporaryDirectory(prefix="opendp-num-verify-") as report_dir:
        for finding in payload["findings"]:
            if not finding["inputs"] and finding.get("direct_reproduction"):
                command = shlex.split(finding["direct_reproduction"])
                expected = finding.get("direct_expected", "")
                started = time.monotonic()
                result = subprocess.run(
                    command,
                    cwd=ROOT.parent,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    errors="replace",
                    check=False,
                )
                duration = time.monotonic() - started
                reproduced = result.returncode == 0 and expected in result.stdout
                failures += not reproduced
                results.append({
                    "finding": finding["id"],
                    "target": "direct_provider_reproducer",
                    "input": None,
                    "sha256": None,
                    "bytes": None,
                    "expected": expected,
                    "reproduced": reproduced,
                    "outcome": "reproduced" if reproduced else "not_reproduced",
                    "exit_code": result.returncode,
                    "duration_seconds": round(duration, 3),
                    "output_excerpt": interesting_excerpt(result.stdout, expected),
                })
                print(f"{finding['id']} direct provider reproducer: {'PASS' if reproduced else 'FAIL'}")
            for item in finding["inputs"]:
                path = registry.parent / item["path"]
                command = [
                    "cargo", "fuzz", "run", "--sanitizer", args.sanitizer,
                    item["target"], str(path), "--", "-runs=1", "-print_final_stats=0",
                ]
                environment = os.environ.copy()
                environment["OPENDP_NUM_FUZZ_REPORT_DIR"] = report_dir
                started = time.monotonic()
                result = subprocess.run(
                    command,
                    cwd=ROOT,
                    env=environment,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    errors="replace",
                    check=False,
                )
                duration = time.monotonic() - started
                reproduced = result.returncode != 0 and item["expected"] in result.stdout
                outcome = "reproduced" if reproduced else (
                    "changed_failure" if result.returncode != 0 else "not_reproduced"
                )
                failures += not reproduced
                record = {
                    "finding": finding["id"],
                    "target": item["target"],
                    "input": item["path"],
                    "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                    "bytes": path.stat().st_size,
                    "expected": item["expected"],
                    "reproduced": reproduced,
                    "outcome": outcome,
                    "exit_code": result.returncode,
                    "duration_seconds": round(duration, 3),
                    "output_excerpt": interesting_excerpt(result.stdout, item["expected"]),
                }
                results.append(record)
                print(f"{finding['id']} {path.name}: {'PASS' if reproduced else 'FAIL'}")

    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps({
        "schema": 1,
        "timestamp_unix": int(time.time()),
        "sanitizer": args.sanitizer,
        "label": args.label,
        "baseline": payload["baseline"],
        "resolved_dependencies": resolved_dependencies(ROOT / "Cargo.lock"),
        "total": len(results),
        "passed": len(results) - failures,
        "failed": failures,
        "results": results,
    }, indent=2, sort_keys=True) + "\n")
    print(f"verification={output} passed={len(results) - failures}/{len(results)}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
