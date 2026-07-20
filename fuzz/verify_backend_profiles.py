#!/usr/bin/env python3
"""Verify retained backend conversion inputs in debug-assertion and release profiles."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def main() -> int:
    registry = json.loads((ROOT / "known_findings.json").read_text())
    failures = 0
    checks = 0
    for finding in registry["findings"]:
        for item in finding["inputs"]:
            if item["target"] != "backend_float_conversion":
                continue
            profiles = item.get(
                "profiles",
                {"debug_assertions": "failure", "release": "failure"},
            )
            for profile, expected_outcome in profiles.items():
                command = ["cargo", "fuzz", "run"]
                if profile == "release":
                    command.append("--release")
                command.extend([
                    "--sanitizer", "none", item["target"],
                    str(ROOT / item["path"]), "--", "-runs=1", "-print_final_stats=0",
                ])
                result = subprocess.run(
                    command,
                    cwd=ROOT,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    errors="replace",
                    check=False,
                )
                if expected_outcome == "pass":
                    passed = result.returncode == 0
                else:
                    passed = result.returncode != 0 and item["expected"] in result.stdout
                checks += 1
                failures += not passed
                print(
                    f"{finding['id']} {Path(item['path']).name} {profile}: "
                    f"{'PASS' if passed else 'FAIL'}"
                )
    print(f"backend profile replay={checks - failures}/{checks}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
