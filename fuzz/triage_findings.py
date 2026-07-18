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
    reproductions = "\n".join(
        f"cargo fuzz run --sanitizer none {item['target']} findings/{finding['library']}/{finding['id']}/inputs/{Path(item['path']).name}"
        for item in inputs
    )
    evidence = "\n".join(
        f"- `{Path(item['path']).name}`: {item['bytes']} bytes, SHA-256 `{item['sha256']}`; expects `{item['expected']}`"
        for item in inputs
    )
    return f"""# {finding['id']}: {finding['title']}

Status: confirmed on the locked baseline. Confidence: {finding['confidence']}. Classification: `{finding['classification']}`.

Latest release check: {finding['upstream_status']}

## Summary

{finding['summary']}

## Impact

{finding['impact']}

## Tested baseline

{versions}.

The repository-level `findings/verification.json` records the most recent automated replay. A separate latest-upstream check is required before filing if the status table does not say it was tested.

## Reproduce

Run from the repository root after installing `cargo-fuzz`:

```bash
{reproductions}
```

## Evidence

{evidence}

## Deduplication rationale

{finding['deduplication']}

## Reporting note

This report describes behavior observed through `opendp-num`'s Dashu adapter and compares directed primitive results bit-for-bit with Rug/MPFR. Upstream maintainers should confirm whether the defect belongs in Dashu itself or in the adapter before assigning it.
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
            "directory": str(directory.relative_to(ROOT.parent)),
            "inputs": rendered_inputs,
            "reports": copied_reports,
        })

    rows = "\n".join(
        f"| [{item['id']}]({item['library']}/{item['id']}/) | {item['library']} | {item['classification']} | {item['title']} |"
        for item in index
    )
    (output / "README.md").write_text(f"""# Curated fuzzer findings

These are conservatively deduplicated findings from differential and property fuzzing of `opendp-num`. Raw runner failures are intentionally excluded: every listed reproducer must pass `fuzz/verify_findings.py` on the locked baseline.

| ID | Library | Kind | Finding |
|---|---|---|---|
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
