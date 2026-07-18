#!/usr/bin/env python3
"""Validate that every operation in the audited OpenDP surface has a live fuzz harness."""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def main() -> int:
    manifest = json.loads((ROOT / "operation_manifest.json").read_text())
    cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
    bins = {entry["name"]: entry["path"] for entry in cargo.get("bin", [])}
    errors: list[str] = []
    seen: set[tuple[str, str]] = set()

    for operation in manifest["operations"]:
        key = (operation["family"], operation["name"])
        if key in seen:
            errors.append(f"duplicate operation entry: {key[0]}/{key[1]}")
        seen.add(key)

        target = operation["target"]
        relative = bins.get(target)
        if relative is None:
            errors.append(f"{key[0]}/{key[1]} references missing target {target!r}")
            continue
        source_path = ROOT / relative
        if not source_path.is_file():
            errors.append(f"target {target!r} source is missing: {source_path}")
            continue
        source = source_path.read_text()
        for needle in operation.get("needles", []):
            if needle not in source:
                errors.append(
                    f"{key[0]}/{key[1]}: {needle!r} not found in {source_path.relative_to(ROOT)}"
                )

    unmanifested = sorted(set(bins) - {entry["target"] for entry in manifest["operations"]})
    if unmanifested:
        errors.append("fuzz targets absent from operation manifest: " + ", ".join(unmanifested))

    if errors:
        print("OpenDP numerical fuzz coverage check failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    families: dict[str, int] = {}
    for operation in manifest["operations"]:
        families[operation["family"]] = families.get(operation["family"], 0) + 1
    summary = ", ".join(f"{name}={count}" for name, count in sorted(families.items()))
    snapshot = manifest["opendp_snapshot"]
    print(
        f"coverage manifest valid: {len(manifest['operations'])} operations; {summary}; "
        f"OpenDP snapshot {snapshot['commit']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
