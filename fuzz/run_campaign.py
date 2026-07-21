#!/usr/bin/env python3
"""Continuously rotate libFuzzer workers across the OpenDP numeric contracts."""

from __future__ import annotations

import argparse
import dataclasses
import hashlib
import json
import math
import os
import re
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path


@dataclasses.dataclass(frozen=True)
class Target:
    name: str
    weight: int
    max_len: int
    timeout: int
    dictionary: str


TARGETS = (
    Target("exact_integer", weight=2, max_len=4096, timeout=20, dictionary="integer.dict"),
    Target("exact_rational", weight=2, max_len=3000, timeout=25, dictionary="integer.dict"),
    Target("directed_unary", weight=4, max_len=64, timeout=15, dictionary="float.dict"),
    Target("directed_binary", weight=4, max_len=64, timeout=15, dictionary="float.dict"),
    Target("conversions", weight=3, max_len=3000, timeout=20, dictionary="float.dict"),
    Target("backend_float_conversion", weight=3, max_len=1024, timeout=10, dictionary="float.dict"),
    Target("backend_float_extremes", weight=4, max_len=64, timeout=10, dictionary="float.dict"),
    Target("primitive_casts", weight=2, max_len=4096, timeout=20, dictionary="integer.dict"),
    Target("alp_primitives", weight=2, max_len=64, timeout=20, dictionary="float.dict"),
    Target("opendp_sequences", weight=5, max_len=4096, timeout=25, dictionary="sequence.dict"),
    Target("malachite_float", weight=4, max_len=4096, timeout=20, dictionary="float.dict"),
)

STOP = False
ACTIVE: list[subprocess.Popen[bytes]] = []
TERMINATION_GRACE_SECONDS = 5.0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cores", type=int, default=max(1, os.cpu_count() or 1))
    parser.add_argument("--slice-seconds", type=int, default=30 * 60)
    parser.add_argument("--rounds", type=int, default=0, help="0 means run forever")
    parser.add_argument("--rss-limit-mb", type=int, default=4096)
    parser.add_argument(
        "--sanitizer",
        choices=("none", "address"),
        default="none",
        help="continuous numerical campaigns default to none for throughput; use address periodically",
    )
    parser.add_argument("--target", action="append", choices=[target.name for target in TARGETS])
    parser.add_argument("--cmin-every", type=int, default=0, help="corpus minimization interval in rounds")
    parser.add_argument("--stop-on-violation", action="store_true")
    parser.add_argument(
        "--keep-clean-logs",
        action="store_true",
        help="retain successful worker logs; failure logs are always retained",
    )
    parser.add_argument(
        "--log-retention-days",
        type=int,
        default=14,
        help="delete retained worker logs older than this many days; 0 disables pruning",
    )
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "--include-known-findings",
        action="store_true",
        help="do not quarantine exact inputs registered in known_findings.json",
    )
    return parser.parse_args()


def handle_signal(signum: int, _frame: object) -> None:
    global STOP
    STOP = True
    print(f"\nreceived signal {signum}; terminating fuzz workers", file=sys.stderr)
    for process in ACTIVE:
        terminate_process_group(process)


def terminate_process_group(process: subprocess.Popen[bytes]) -> None:
    """Terminate cargo-fuzz and the libFuzzer process it launched."""
    if process.poll() is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass


def kill_process_group(process: subprocess.Popen[bytes]) -> None:
    """Force-kill a fuzz worker process group after its grace period."""
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


def allocate_workers(targets: list[Target], cores: int, round_index: int) -> dict[Target, int]:
    cores = max(1, cores)
    if cores < len(targets):
        schedule: list[Target] = []
        for target in targets:
            schedule.extend([target] * target.weight)
        selected: list[Target] = []
        cursor = round_index * cores
        while len(selected) < cores:
            candidate = schedule[cursor % len(schedule)]
            cursor += 1
            if candidate not in selected:
                selected.append(candidate)
        return {target: 1 for target in selected}

    allocation = {target: 1 for target in targets}
    remaining = cores - len(targets)
    if remaining == 0:
        return allocation

    total_weight = sum(target.weight for target in targets)
    shares = {target: remaining * target.weight / total_weight for target in targets}
    for target, share in shares.items():
        extra = math.floor(share)
        allocation[target] += extra
        remaining -= extra
    for target in sorted(targets, key=lambda item: shares[item] % 1, reverse=True)[:remaining]:
        allocation[target] += 1
    return allocation


def ensure_tools(root: Path, dry_run: bool) -> None:
    if dry_run:
        return
    if shutil.which("cargo") is None:
        raise SystemExit("cargo is not installed")
    result = subprocess.run(
        ["cargo", "fuzz", "--help"],
        cwd=root,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if result.returncode != 0:
        raise SystemExit("cargo-fuzz is not installed; run `cargo install cargo-fuzz`")


def seed_corpora(root: Path, dry_run: bool) -> None:
    command = [sys.executable, str(root / "seed_corpus.py")]
    if dry_run:
        print("+", " ".join(command))
        return
    subprocess.run(command, cwd=root, check=True)


def quarantine_known_inputs(root: Path, dry_run: bool) -> None:
    command = [sys.executable, str(root / "quarantine_known.py"), "--apply"]
    if dry_run:
        print("+", " ".join(command))
        return
    subprocess.run(command, cwd=root, check=True)


def preflight_build(root: Path, targets: list[Target], args: argparse.Namespace) -> bool:
    """Build targets before launching workers so one compiler error cannot cancel a round."""
    for target in targets:
        command = [
            "cargo",
            "fuzz",
            "build",
            "--sanitizer",
            args.sanitizer,
            target.name,
        ]
        print(f"[{target.name}] preflight build")
        if args.dry_run:
            print("+", " ".join(command))
            continue
        result = subprocess.run(command, cwd=root, check=False)
        if result.returncode != 0:
            print(f"[{target.name}] preflight build failed", file=sys.stderr)
            return False
    return True


def launch_round(
    root: Path,
    allocation: dict[Target, int],
    args: argparse.Namespace,
    round_index: int,
) -> tuple[bool, bool]:
    """Return (saw_violation, saw_infrastructure_failure)."""
    timestamp = time.strftime("%Y%m%d-%H%M%S")
    processes: list[tuple[Target, int, subprocess.Popen[bytes], Path, object]] = []
    ACTIVE.clear()

    for target, workers in allocation.items():
        corpus = root / "corpus" / target.name
        artifacts = root / "artifacts" / target.name
        reports = root / "reports"
        logs = root / "logs" / target.name
        for directory in (corpus, artifacts, reports, logs):
            directory.mkdir(parents=True, exist_ok=True)

        for worker_index in range(workers):
            log_path = logs / (
                f"{timestamp}-round-{round_index:06d}-worker-{worker_index:02d}.log"
            )
            command = [
                "cargo",
                "fuzz",
                "run",
                "--sanitizer",
                args.sanitizer,
                target.name,
                str(corpus),
                "--",
                f"-max_total_time={args.slice_seconds}",
                f"-max_len={target.max_len}",
                f"-timeout={target.timeout}",
                f"-rss_limit_mb={args.rss_limit_mb}",
                f"-artifact_prefix={artifacts}{os.sep}",
                f"-dict={root / 'dictionaries' / target.dictionary}",
                "-print_final_stats=1",
                "-use_value_profile=1",
                "-reduce_inputs=1",
                "-report_slow_units=5",
                "-reload=5",
            ]
            print(f"[{target.name}:{worker_index}] log={log_path}")
            if args.dry_run:
                print("+", " ".join(command))
                continue

            log_file = log_path.open("wb")
            environment = os.environ.copy()
            environment["OPENDP_NUM_FUZZ_REPORT_DIR"] = str(reports)
            environment["OPENDP_NUM_FUZZ_WORKER"] = f"{target.name}:{worker_index}"
            environment.setdefault("RUST_BACKTRACE", "1")
            process = subprocess.Popen(
                command,
                cwd=root,
                env=environment,
                stdout=log_file,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            ACTIVE.append(process)
            processes.append((target, worker_index, process, log_path, log_file))

    if args.dry_run:
        return False, False

    saw_violation = False
    saw_infrastructure_failure = False
    pending = list(processes)
    terminate_remaining = False
    cancelled: set[subprocess.Popen[bytes]] = set()
    termination_deadlines: dict[subprocess.Popen[bytes], float] = {}

    while pending:
        made_progress = False
        for entry in list(pending):
            target, worker_index, process, log_path, log_file = entry
            return_code = process.poll()
            if return_code is None:
                continue

            made_progress = True
            pending.remove(entry)
            if process in ACTIVE:
                ACTIVE.remove(process)
            log_file.close()
            text = log_path.read_text(errors="replace")
            category, reason = classify_log(text, return_code)

            if process in cancelled and category == "infrastructure_failure":
                category = "cancelled"
                reason = "worker was cancelled after another worker failed"

            if category in {"contract_violation", "timeout", "out_of_memory", "crash"}:
                saw_violation = True
                report = write_runner_report(
                    root, target.name, worker_index, category, reason, log_path, text
                )
                print(
                    f"[{target.name}:{worker_index}] {category.replace('_', ' ')} recorded; report={report}",
                    file=sys.stderr,
                )
                terminate_remaining |= args.stop_on_violation
            elif category == "infrastructure_failure" and not STOP:
                saw_infrastructure_failure = True
                report = write_runner_report(
                    root, target.name, worker_index, category, reason, log_path, text
                )
                print(
                    f"[{target.name}:{worker_index}] infrastructure/build failure; report={report}",
                    file=sys.stderr,
                )
                terminate_remaining = True
            elif category == "cancelled":
                print(f"[{target.name}:{worker_index}] cancelled")
                if not args.keep_clean_logs:
                    log_path.unlink(missing_ok=True)
            else:
                print(f"[{target.name}:{worker_index}] slice complete")
                if not args.keep_clean_logs:
                    log_path.unlink(missing_ok=True)

        if terminate_remaining:
            for _, _, process, _, _ in pending:
                if process.poll() is None:
                    cancelled.add(process)
                    termination_deadlines[process] = time.monotonic() + TERMINATION_GRACE_SECONDS
                    terminate_process_group(process)
            terminate_remaining = False

        for process, deadline in list(termination_deadlines.items()):
            if process.poll() is not None:
                termination_deadlines.pop(process, None)
            elif time.monotonic() >= deadline:
                kill_process_group(process)
                termination_deadlines.pop(process, None)

        if pending and not made_progress:
            time.sleep(0.25)

    # The cargo process can exit before a descendant that ignored SIGTERM.
    # A final group kill is harmless if the process group is already gone.
    for process in cancelled:
        kill_process_group(process)
    ACTIVE.clear()
    prune_old_logs(root, args.log_retention_days)
    return saw_violation, saw_infrastructure_failure


def prune_old_logs(root: Path, retention_days: int) -> None:
    if retention_days <= 0:
        return
    cutoff = time.time() - retention_days * 24 * 60 * 60
    logs = root / "logs"
    if not logs.exists():
        return
    for path in logs.rglob("*.log"):
        try:
            if path.stat().st_mtime < cutoff:
                path.unlink()
        except FileNotFoundError:
            pass

def classify_log(text: str, return_code: int) -> tuple[str, str]:
    lower = text.lower()
    if "opendp_num_violation" in lower:
        marker = next(
            (line.strip() for line in text.splitlines() if "OPENDP_NUM_VIOLATION" in line),
            "numerical contract violation",
        )
        return "contract_violation", marker
    if "error: libfuzzer: timeout" in lower or "timeout after" in lower:
        return "timeout", "libFuzzer execution timeout"
    if any(token in lower for token in ("out-of-memory", "out of memory", "allocation-size-too-big")):
        return "out_of_memory", "fuzz input exceeded the configured memory budget"
    if "test unit written to" in lower or "summary: libfuzzer" in lower or "deadly signal" in lower:
        return "crash", "backend panic, abort, or sanitizer-detected crash"
    if return_code != 0:
        return "infrastructure_failure", "cargo-fuzz build or runner process exited unsuccessfully"
    return "clean", "slice completed without a recorded violation"


def write_runner_report(
    root: Path,
    target: str,
    worker_index: int,
    category: str,
    reason: str,
    log_path: Path,
    text: str,
) -> Path:
    report_dir = root / "reports" / "runner"
    report_dir.mkdir(parents=True, exist_ok=True)
    digest = hashlib.sha256(
        (target + "\0" + str(worker_index) + "\0" + category + "\0" + reason + "\0" + text).encode(errors="replace")
    ).hexdigest()[:20]
    report_path = report_dir / f"{target}-{digest}.json"
    artifact_match = re.findall(r"Test unit written to (.+)", text)
    tail = "\n".join(text.splitlines()[-200:])
    backend_targets = {
        "backend_float_conversion": "dashu",
        "backend_float_extremes": "dashu",
        "malachite_float": "malachite",
    }
    contract = "backend_conformance" if target in backend_targets else "uniformity"
    owner = (
        "resource_behavior"
        if category in {"timeout", "out_of_memory"}
        else "harness"
        if category == "infrastructure_failure"
        else "backend"
        if contract == "backend_conformance"
        else "unspecified"
    )
    payload = {
        "schema": 2,
        "timestamp_unix": int(time.time()),
        "target": target,
        "worker_index": worker_index,
        "category": category,
        "reason": reason,
        "contract": contract,
        "provider": backend_targets.get(target, "multiple"),
        "owner": owner,
        "construction": "unknown_from_runner",
        "source_type": "unknown_from_runner",
        "source_precision": "unknown_from_runner",
        "significand_bits": "unknown_from_runner",
        "oracle": "unknown_from_runner",
        "expected_class": "clean_completion",
        "observed_class": category,
        "masked_by_adapter": "unknown_from_runner",
        "adapter_result": "not_evaluated",
        "raw_backend_result": reason,
        "log": str(log_path),
        "artifact": artifact_match[-1].strip() if artifact_match else None,
        "log_tail": tail,
    }
    temporary = report_path.with_suffix(".json.tmp")
    temporary.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    temporary.replace(report_path)
    return report_path


def minimize_corpora(root: Path, targets: list[Target], args: argparse.Namespace) -> bool:
    for target in targets:
        corpus = root / "corpus" / target.name
        command = [
            "cargo",
            "fuzz",
            "cmin",
            target.name,
            str(corpus),
            "--",
            f"-max_len={target.max_len}",
        ]
        print(f"[{target.name}] minimizing corpus")
        if args.dry_run:
            print("+", " ".join(command))
            continue
        result = subprocess.run(command, cwd=root, check=False)
        if result.returncode != 0:
            print(f"[{target.name}] corpus minimization failed", file=sys.stderr)
            return False
    return True


def main() -> int:
    args = parse_args()
    root = Path(__file__).resolve().parent
    selected = [target for target in TARGETS if not args.target or target.name in args.target]
    signal.signal(signal.SIGINT, handle_signal)
    signal.signal(signal.SIGTERM, handle_signal)

    ensure_tools(root, args.dry_run)
    if not args.include_known_findings:
        quarantine_known_inputs(root, args.dry_run)
    seed_corpora(root, args.dry_run)
    if not preflight_build(root, selected, args):
        return 2

    round_index = 0
    while not STOP and (args.rounds == 0 or round_index < args.rounds):
        allocation = allocate_workers(selected, args.cores, round_index)
        print(
            f"campaign round {round_index}: "
            + ", ".join(f"{target.name}={workers}" for target, workers in allocation.items())
        )
        violation, infrastructure_failure = launch_round(root, allocation, args, round_index)
        if infrastructure_failure:
            return 2
        if violation and args.stop_on_violation:
            return 1
        round_index += 1
        if args.cmin_every and round_index % args.cmin_every == 0:
            if not minimize_corpora(root, selected, args):
                return 2

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
