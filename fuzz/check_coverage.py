#!/usr/bin/env python3
"""Validate that every operation in the audited OpenDP surface has a live fuzz harness."""

from __future__ import annotations

import json
import re
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
    witness_source = (ROOT / "src" / "witnesses.rs").read_text()
    witnesses = set(re.findall(r"pub fn (witness_[a-z0-9_]+)\s*\(", witness_source))

    if manifest.get("schema") != 2:
        errors.append("operation manifest must use schema 2")

    classified_targets: set[str] = set()
    for target in manifest.get("targets", []):
        name = target.get("name")
        contract = target.get("contract")
        witness = target.get("witness")
        if name in classified_targets:
            errors.append(f"duplicate target classification: {name}")
        classified_targets.add(name)
        if contract not in {"uniformity", "backend_conformance"}:
            errors.append(f"target {name!r} has invalid contract category {contract!r}")
        if witness not in witnesses:
            errors.append(f"target {name!r} references missing compiled witness {witness!r}")

    missing_classifications = sorted(set(bins) - classified_targets)
    unknown_classifications = sorted(classified_targets - set(bins))
    if missing_classifications:
        errors.append("fuzz targets without contract classification: " + ", ".join(missing_classifications))
    if unknown_classifications:
        errors.append("classified targets absent from Cargo.toml: " + ", ".join(unknown_classifications))

    contract_ids: set[str] = set()
    target_contracts = {entry["name"]: entry["contract"] for entry in manifest.get("targets", [])}
    required_contract_fields = {"id", "contract", "provider", "operation", "target", "witness"}
    for contract in manifest.get("contracts", []):
        missing = sorted(required_contract_fields - set(contract))
        if missing:
            errors.append(f"contract entry missing {', '.join(missing)}: {contract!r}")
            continue
        contract_id = contract["id"]
        if contract_id in contract_ids:
            errors.append(f"duplicate contract id: {contract_id}")
        contract_ids.add(contract_id)
        if contract["contract"] not in {"uniformity", "backend_conformance"}:
            errors.append(f"{contract_id}: invalid contract category")
        if target_contracts.get(contract["target"]) != contract["contract"]:
            errors.append(f"{contract_id}: target classification disagrees with contract entry")
        if contract["witness"] not in witnesses:
            errors.append(f"{contract_id}: missing compiled witness {contract['witness']!r}")

    powi_entries = [entry for entry in manifest.get("contracts", []) if entry.get("operation") == "powi"]
    if not powi_entries or any(
        entry.get("input_types", {}).get("exponent") != "arbitrary_precision_signed_integer"
        for entry in powi_entries
    ):
        errors.append("powi contracts must retain arbitrary-precision signed exponents")
    capability_source = (ROOT.parent / "src" / "capability.rs").read_text()
    if "DirectedPowI<T, E: ?Sized" not in capability_source or "exponent: &E" not in capability_source:
        errors.append("DirectedPowI has been narrowed away from a borrowed provider-native exponent")
    for needle in [
        "DirectedPowI<f64, IBig>",
        "DirectedPowI<f32, IBig>",
        "DirectedPowI<f64, Integer>",
        "DirectedPowI<f32, Integer>",
    ]:
        if needle not in witness_source:
            errors.append(f"typed power witness is missing exact signature {needle!r}")

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

    if errors:
        print("Numerical contract coverage check failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    families: dict[str, int] = {}
    for operation in manifest["operations"]:
        families[operation["family"]] = families.get(operation["family"], 0) + 1
    summary = ", ".join(f"{name}={count}" for name, count in sorted(families.items()))
    snapshot = manifest["opendp_snapshot"]
    print(
        f"coverage manifest valid: {len(manifest['contracts'])} typed contracts, "
        f"{len(manifest['operations'])} legacy operation audits; {summary}; "
        f"OpenDP snapshot {snapshot['commit']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
