#!/usr/bin/env python3
"""Audit v0.3 roadmap signals against the release status registry."""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass
from pathlib import Path

from run_v02_release_audit import fnv64_fingerprint


@dataclass(frozen=True)
class RoadmapSignal:
    text: str
    count: int
    line: str


@dataclass(frozen=True)
class SignalStatus:
    status: str
    rationale: str
    evidence: str


def parse_roadmap_signals(roadmap_text: str) -> list[RoadmapSignal]:
    signals: list[RoadmapSignal] = []
    for raw_line in roadmap_text.splitlines():
        line = raw_line.strip()
        if not line.startswith("signal ") or " count " not in line:
            continue
        signal_text, rest = line.removeprefix("signal ").split(" count ", 1)
        count_token = rest.split(maxsplit=1)[0]
        try:
            count = int(count_token)
        except ValueError as error:
            raise ValueError(f"roadmap signal count is not an integer: {line}") from error
        signals.append(RoadmapSignal(signal_text.strip(), count, line))
    return signals


def parse_status_registry(status_text: str) -> dict[str, SignalStatus]:
    entries: dict[str, SignalStatus] = {}
    current: dict[str, str] = {}

    def finish_current() -> None:
        nonlocal current
        if not current:
            return
        signal = current.get("signal", "").strip()
        status = current.get("status", "").strip()
        rationale = current.get("rationale", "").strip()
        evidence = current.get("evidence", "").strip()
        if not signal:
            raise ValueError("signal status block is missing signal")
        if status not in {"promoted", "deferred"}:
            raise ValueError(
                f"signal status for {signal} must be promoted or deferred, found {status or '<missing>'}"
            )
        if not rationale:
            raise ValueError(f"signal status for {signal} is missing rationale")
        if not evidence:
            raise ValueError(f"signal status for {signal} is missing evidence")
        if signal in entries:
            raise ValueError(f"duplicate signal status block: {signal}")
        entries[signal] = SignalStatus(status=status, rationale=rationale, evidence=evidence)
        current = {}

    for raw_line in status_text.splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            finish_current()
            continue
        if ":" not in line:
            if not current:
                continue
            raise ValueError(f"signal status line must use key: value format: {raw_line}")
        key, value = line.split(":", 1)
        key = key.strip()
        if key not in {"signal", "status", "rationale", "evidence"}:
            raise ValueError(f"unknown signal status key: {key}")
        current[key] = value.strip()
    finish_current()
    return entries


def render_report(
    roadmap_file: Path,
    status_file: Path,
    signals: list[RoadmapSignal],
    statuses: dict[str, SignalStatus],
    min_count: int,
) -> str:
    high_count = [signal for signal in signals if signal.count >= min_count]
    low_count = [signal for signal in signals if signal.count < min_count]
    lines = [
        "AIL-v0.3-Roadmap-Signal-Status-Audit:",
        f"roadmap-file {roadmap_file}",
        f"status-file {status_file}",
        f"high-count-threshold {min_count}",
        f"roadmap-signal-count {len(signals)}",
        f"high-count-signal-count {len(high_count)}",
        f"low-count-signal-count {len(low_count)}",
        f"status-entry-count {len(statuses)}",
    ]

    missing: list[str] = []
    promoted_count = 0
    deferred_count = 0
    for signal in high_count:
        status = statuses.get(signal.text)
        if status is None:
            missing.append(signal.text)
            lines.append(f"signal-status {signal.text} count {signal.count} status missing")
            continue
        if status.status == "promoted":
            promoted_count += 1
        else:
            deferred_count += 1
        lines.append(
            f"signal-status {signal.text} count {signal.count} status {status.status}"
        )
        lines.append(f"signal-status-evidence {signal.text} {status.evidence}")

    unused = sorted(set(statuses).difference(signal.text for signal in signals))
    lines.append(f"promoted-count {promoted_count}")
    lines.append(f"deferred-count {deferred_count}")
    lines.append(f"missing-status-count {len(missing)}")
    for signal in missing:
        lines.append(f"missing-status {signal}")
    lines.append(f"unused-status-count {len(unused)}")
    for signal in unused:
        lines.append(f"unused-status {signal}")
    lines.append("audit-result accepted" if not missing else "audit-result rejected")
    return "\n".join(lines) + "\n"


def write_outputs(output_dir: Path, report: str) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    report_path = output_dir / "v03-roadmap-signal-status.txt"
    report_fingerprint = fnv64_fingerprint(report.encode("utf-8"))
    report_path.write_text(report, encoding="utf-8")
    (output_dir / "v03-roadmap-signal-status.fingerprint.txt").write_text(
        report_fingerprint + "\n",
        encoding="utf-8",
    )
    manifest = "\n".join(
        [
            "AIL-v0.3-Roadmap-Signal-Status-Manifest:",
            f"report v03-roadmap-signal-status.txt {report_fingerprint}",
            "audit-result accepted"
            if "audit-result accepted" in report
            else "audit-result rejected",
            "",
        ]
    )
    (output_dir / "manifest.v03-roadmap-signal-status.txt").write_text(
        manifest,
        encoding="utf-8",
    )
    (output_dir / "manifest.fingerprint.txt").write_text(
        fnv64_fingerprint(manifest.encode("utf-8")) + "\n",
        encoding="utf-8",
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--roadmap-file", type=Path, required=True)
    parser.add_argument(
        "--status-file",
        type=Path,
        default=Path("docs/ail/v03-roadmap-signal-status.md"),
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("/tmp/ail-v03-roadmap-signal-status"),
    )
    parser.add_argument(
        "--min-count",
        type=int,
        default=5,
        help="minimum roadmap count that requires an explicit promoted/deferred status",
    )
    args = parser.parse_args(argv)

    try:
        roadmap_text = args.roadmap_file.read_text(encoding="utf-8")
        status_text = args.status_file.read_text(encoding="utf-8")
        signals = parse_roadmap_signals(roadmap_text)
        statuses = parse_status_registry(status_text)
        report = render_report(
            args.roadmap_file, args.status_file, signals, statuses, args.min_count
        )
        write_outputs(args.output_dir, report)
        sys.stdout.write(report)
        if "audit-result rejected" in report:
            missing = [
                line.removeprefix("missing-status ")
                for line in report.splitlines()
                if line.startswith("missing-status ")
            ]
            if missing:
                print(
                    f"missing classification for high-count roadmap signal: {missing[0]}",
                    file=sys.stderr,
                )
            return 1
        return 0
    except OSError as error:
        print(f"signal status audit failed: {error}", file=sys.stderr)
        return 1
    except ValueError as error:
        print(f"signal status audit failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
