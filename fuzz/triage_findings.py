#!/usr/bin/env python3
"""Generate a publishable, library-oriented findings tree from the curated registry."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def render_finding(finding: dict, baseline: dict, inputs: list[dict]) -> str:
    versions = ", ".join(f"`{name} {version}`" for name, version in baseline.items())
    if inputs:
        reproduction_lead = "Run from the repository root after installing `cargo-fuzz`:"
        reproductions = "\n".join(
            f"cargo fuzz run --sanitizer none {item['target']} findings/{finding['library']}/{finding['id']}/inputs/{Path(item['path']).name}"
            for item in inputs
        )
        evidence = "\n".join(
            f"- `{Path(item['path']).name}`: {item['bytes']} bytes, SHA-256 `{item['sha256']}`; expects `{item['expected']}`"
            for item in inputs
        )
    else:
        # Probe-verified finding: reproduced directly against the library API.
        reproduction_lead = finding.get(
            "reproduction_lead",
            "Run from the repository root after installing `cargo-fuzz`:",
        )
        reproductions = finding.get("direct_reproduction", "")
        evidence = "Reproduced directly against the library API; see the command above."
    contract = finding.get("contract", "uniformity")
    owner = finding.get("owner", "backend")
    masked = finding.get("masked_by_adapter", False)
    default_reporting_note = (
        "This is a direct provider probe. The opendp-num adapter does not expose this "
        "arbitrary-precision float conversion path, so the backend defect is retained even "
        "though it does not currently violate the public uniformity surface."
        if contract == "backend_conformance"
        else
        "This report describes behavior observed through opendp-num's backend-neutral "
        "uniformity contract. The retained evidence identifies whether the cause is in a "
        "provider or in the adapter."
    )
    reporting_note = finding.get("reporting_note", default_reporting_note)
    return f"""# {finding['id']}: {finding['title']}

Status: confirmed on the locked baseline. Confidence: {finding['confidence']}. Classification: `{finding['classification']}`.

Contract: `{contract}`. Owner: `{owner}`. Masked by adapter: `{str(masked).lower()}`.

Latest release check: {finding['upstream_status']}

## Summary

{finding['summary']}

## Impact

{finding['impact']}

## Tested baseline

{versions}.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

{reproduction_lead}

```bash
{reproductions}
```

## Evidence

{evidence}

## Deduplication rationale

{finding['deduplication']}

## Reporting note

{reporting_note}
"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", type=Path, default=ROOT / "known_findings.json")
    parser.add_argument("--output", type=Path, default=ROOT.parent / "findings")
    args = parser.parse_args()

    registry = args.registry.resolve()
    output = args.output.resolve()
    payload = json.loads(registry.read_text())
    index = []

    for finding in payload["findings"]:
        directory = output / finding["library"] / finding["id"]
        inputs_dir = directory / "inputs"
        reports_dir = directory / "reports"
        inputs_dir.mkdir(parents=True, exist_ok=True)
        rendered_inputs = []
        for item in finding["inputs"]:
            source = registry.parent / item["path"]
            target = inputs_dir / source.name
            shutil.copyfile(source, target)
            rendered_inputs.append({
                **item,
                "path": str(target.relative_to(ROOT.parent)),
                "bytes": target.stat().st_size,
                "sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
            })
        copied_reports = []
        for report in finding["reports"]:
            source = registry.parent / report
            if source.is_file():
                reports_dir.mkdir(parents=True, exist_ok=True)
                target = reports_dir / source.name
                shutil.copyfile(source, target)
                copied_reports.append(str(target.relative_to(ROOT.parent)))
        (directory / "README.md").write_text(render_finding(finding, payload["baseline"], rendered_inputs))
        index.append({
            **{key: finding[key] for key in ("id", "library", "title", "classification", "confidence")},
            "contract": finding.get("contract", "uniformity"),
            "owner": finding.get("owner", "backend"),
            "masked_by_adapter": finding.get("masked_by_adapter", False),
            "directory": str(directory.relative_to(ROOT.parent)),
            "inputs": rendered_inputs,
            "reports": copied_reports,
        })

    rows = "\n".join(
        f"| [{item['id']}]({item['library']}/{item['id']}/) | {item['library']} | {item['contract']} | {item['classification']} | {item['title']} |"
        for item in index
    )
    (output / "README.md").write_text(f"""# Curated fuzzer findings

These are conservatively deduplicated findings from differential and property fuzzing of `opendp-num`. Raw runner failures are intentionally excluded: every listed reproducer must pass `fuzz/verify_findings.py` on the locked baseline.

| ID | Library | Contract | Kind | Finding |
|---|---|---|---|---|
{rows}

## Reproduce everything

```bash
python3 fuzz/verify_findings.py
```

See `METHODOLOGY.md` for the evidence, deduplication, quarantine, and upstream-validation policy.
""")
    (output / "index.json").write_text(json.dumps({
        "schema": 1,
        "baseline": payload["baseline"],
        "findings": index,
    }, indent=2, sort_keys=True) + "\n")
    print(f"generated {len(index)} findings under {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
