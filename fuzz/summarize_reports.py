#!/usr/bin/env python3
"""Summarize structured OpenDP numeric fuzz violations."""

from __future__ import annotations

import argparse
import collections
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reports", type=Path, default=ROOT / "reports")
    parser.add_argument("--json", action="store_true", help="emit machine-readable JSON")
    parser.add_argument("--fail-if-any", action="store_true")
    parser.add_argument("--limit", type=int, default=50)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    records: list[dict[str, object]] = []
    malformed: list[str] = []
    for path in sorted(args.reports.rglob("*.json")):
        try:
            record = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError) as error:
            malformed.append(f"{path}: {error}")
            continue
        record["report_path"] = str(path)
        records.append(record)

    groups: dict[tuple[str, str, str], list[dict[str, object]]] = collections.defaultdict(list)
    for record in records:
        target = str(record.get("target", "unknown"))
        operation = str(record.get("operation", record.get("category", "unknown")))
        reason = str(record.get("reason", "unspecified"))
        groups[(target, operation, reason)].append(record)

    entries = []
    for (target, operation, reason), grouped in groups.items():
        latest = max(grouped, key=lambda item: int(item.get("timestamp_unix", 0)))
        entries.append(
            {
                "count": len(grouped),
                "target": target,
                "operation": operation,
                "reason": reason,
                "latest_timestamp_unix": latest.get("timestamp_unix"),
                "latest_report": latest.get("report_path"),
                "reproducer": latest.get("reproducer", latest.get("artifact")),
            }
        )
    entries.sort(key=lambda entry: (-entry["count"], entry["target"], entry["operation"], entry["reason"]))
    entries = entries[: max(0, args.limit)]

    if args.json:
        print(json.dumps({"total_reports": len(records), "groups": entries, "malformed": malformed}, indent=2))
    else:
        print(f"reports={len(records)} groups={len(groups)} malformed={len(malformed)}")
        for entry in entries:
            print(
                f"{entry['count']:>5}  {entry['target']}/{entry['operation']}: {entry['reason']}\n"
                f"       latest={entry['latest_report']} reproducer={entry['reproducer']}"
            )
        for item in malformed:
            print(f"malformed: {item}", file=sys.stderr)

    if malformed:
        return 2
    if args.fail_if_any and records:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
