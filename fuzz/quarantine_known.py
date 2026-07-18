#!/usr/bin/env python3
"""Remove registered reproducer bytes from active fuzz corpora, recoverably."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def known_input_hashes(registry: Path = ROOT / "known_findings.json") -> dict[str, set[str]]:
    payload = json.loads(registry.read_text())
    hashes: dict[str, set[str]] = defaultdict(set)
    for finding in payload["findings"]:
        for item in finding["inputs"]:
            path = registry.parent / item["path"]
            if not path.is_file():
                raise FileNotFoundError(f"registered input does not exist: {path}")
            hashes[item["target"]].add(hashlib.sha256(path.read_bytes()).hexdigest())
    return dict(hashes)


def quarantine(registry: Path, corpus: Path, destination: Path, apply: bool) -> int:
    registry_payload = json.loads(registry.read_text())
    hashes = known_input_hashes(registry)
    matches: list[tuple[Path, Path]] = []
    for target, target_hashes in hashes.items():
        target_corpus = corpus / target
        if not target_corpus.exists():
            continue
        for path in target_corpus.iterdir():
            if not path.is_file():
                continue
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            if digest in target_hashes:
                matches.append((path, destination / target / digest))

    for target, limit in registry_payload.get("campaign_limits", {}).items():
        target_corpus = corpus / target
        if not target_corpus.exists():
            continue
        max_len = int(limit["max_len"])
        for path in target_corpus.iterdir():
            if path.is_file() and path.stat().st_size > max_len:
                digest = hashlib.sha256(path.read_bytes()).hexdigest()
                candidate = (path, destination / target / "oversized" / digest)
                if candidate not in matches:
                    matches.append(candidate)

    for source, target in matches:
        print(f"{'quarantine' if apply else 'would quarantine'} {source} -> {target}")
        if not apply:
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        if target.exists():
            source.unlink()
        else:
            shutil.move(source, target)
    print(f"known corpus matches={len(matches)} applied={apply}")
    return len(matches)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", type=Path, default=ROOT / "known_findings.json")
    parser.add_argument("--corpus", type=Path, default=ROOT / "corpus")
    parser.add_argument("--destination", type=Path, default=ROOT / "quarantine" / "corpus")
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    quarantine(args.registry.resolve(), args.corpus.resolve(), args.destination.resolve(), args.apply)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
