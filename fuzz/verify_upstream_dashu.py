#!/usr/bin/env python3
"""Replay the PR #2801 Dashu audit on 0.5.0 and an optional master checkout."""

from __future__ import annotations

import argparse
import io
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

REPOSITORY = Path(__file__).resolve().parent.parent
AUDIT_SOURCE = REPOSITORY / "examples" / "audit_dashu_pr2801.rs"
EXPECTED = (
    "PR2801 audit: precision-state issue reproduced; exp, exp_m1, and powi "
    "saturation candidates did not reproduce in raw FBig"
)


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        errors="replace",
        check=False,
    )


def check(command: list[str], cwd: Path, label: str) -> bool:
    result = run(command, cwd)
    passed = result.returncode == 0 and EXPECTED in result.stdout
    print(f"{label}: {'PASS' if passed else 'FAIL'}")
    if not passed:
        print(result.stdout[-4000:])
    return passed


def archive_checkout(checkout: Path, destination: Path) -> str:
    commit_result = run(["git", "rev-parse", "HEAD"], checkout)
    if commit_result.returncode != 0:
        raise RuntimeError(commit_result.stdout)
    commit = commit_result.stdout.strip()
    archive = subprocess.run(
        ["git", "archive", "--format=tar", commit],
        cwd=checkout,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if archive.returncode != 0:
        raise RuntimeError(archive.stderr.decode(errors="replace"))
    destination.mkdir(parents=True)
    with tarfile.open(fileobj=io.BytesIO(archive.stdout), mode="r:") as payload:
        payload.extractall(destination, filter="data")
    return commit


def check_master(checkout: Path) -> bool:
    with tempfile.TemporaryDirectory(prefix="opendp-num-dashu-master-") as raw_temp:
        temp = Path(raw_temp)
        dashu = temp / "dashu"
        commit = archive_checkout(checkout, dashu)
        probe = temp / "probe"
        (probe / "src").mkdir(parents=True)
        shutil.copyfile(AUDIT_SOURCE, probe / "src" / "main.rs")
        (probe / "Cargo.toml").write_text(
            """[package]
name = "dashu-pr2801-master-audit"
version = "0.0.0"
edition = "2024"

[dependencies]
dashu = { path = "../dashu", features = ["num-traits_v02"] }
"""
        )
        manifest = probe / "Cargo.toml"
        debug = check(
            ["cargo", "run", "--offline", "--manifest-path", str(manifest)],
            probe,
            f"master {commit[:12]} debug",
        )
        release = check(
            [
                "cargo",
                "run",
                "--offline",
                "--release",
                "--manifest-path",
                str(manifest),
            ],
            probe,
            f"master {commit[:12]} release",
        )
        return debug and release


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--master-checkout",
        type=Path,
        help="Dashu checkout whose committed HEAD should be tested as current master",
    )
    args = parser.parse_args()

    passed = check(
        ["cargo", "run", "--example", "audit_dashu_pr2801"],
        REPOSITORY,
        "dashu 0.5.0 debug",
    )
    passed &= check(
        ["cargo", "run", "--release", "--example", "audit_dashu_pr2801"],
        REPOSITORY,
        "dashu 0.5.0 release",
    )
    if args.master_checkout:
        passed &= check_master(args.master_checkout.resolve())
    else:
        print("master: SKIP (pass --master-checkout)")
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
