#!/usr/bin/env python3
"""Audit rejected-example repair tutorials and corrected repair proofs."""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path

from run_v02_release_audit import fnv64_fingerprint


REPAIR_SIGNAL = (
    "Rejected examples need repair tutorials that convert diagnostics into "
    "corrected specs."
)
REPAIR_ARTIFACTS: tuple[tuple[str, str, str], ...] = (
    ("diagnostics", "diagnostics.txt", "diagnostics.fingerprint.txt"),
    ("repair-tutorial", "repair-tutorial.txt", "repair-tutorial.fingerprint.txt"),
    (
        "repair-candidate",
        "repair-candidate.ail-spec.md",
        "repair-candidate.fingerprint.txt",
    ),
    (
        "repair-checked-core",
        "repair-checked.ail-core.txt",
        "repair-checked.ail-core.fingerprint.txt",
    ),
    (
        "repair-bytecode",
        "repair-artifact.ailbc.json",
        "repair-artifact.ailbc.fingerprint.txt",
    ),
    ("repair-diff", "repair-diff.txt", "repair-diff.fingerprint.txt"),
    (
        "repair-promotion-review",
        "repair-promotion-review.txt",
        "repair-promotion-review.fingerprint.txt",
    ),
)


@dataclass(frozen=True)
class RepairEntry:
    entry_id: str
    failure_taxonomy: str
    expected_diagnostic: str


def fnv64_text(text: str) -> str:
    return fnv64_fingerprint(text.encode("utf-8"))


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def require_contains(text: str, needle: str, label: str) -> None:
    require(needle in text, f"{label} missing {needle}")


def parse_key_values(text: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in text.splitlines():
        if not line or line.endswith(":") or " " not in line:
            continue
        key, value = line.split(" ", 1)
        values[key] = value
    return values


def catalog_sections(catalog_text: str) -> list[dict[str, str]]:
    sections: list[dict[str, str]] = []
    for raw_section in catalog_text.split("## Example: ")[1:]:
        lines = raw_section.splitlines()
        if not lines:
            continue
        values = {"entry-id": lines[0].strip()}
        for line in lines[1:]:
            if ": " not in line:
                continue
            key, value = line.split(": ", 1)
            values[key] = value
        sections.append(values)
    return sections


def repair_signal_entries(base_corpus: Path) -> list[RepairEntry]:
    catalog = read_text(base_corpus / "examples.md")
    entries = []
    for section in catalog_sections(catalog):
        if (
            section.get("checker-result") == "rejected"
            and section.get("v0.3-signal") == REPAIR_SIGNAL
        ):
            entries.append(
                RepairEntry(
                    entry_id=section["entry-id"],
                    failure_taxonomy=section["failure-taxonomy"],
                    expected_diagnostic=section["expected-diagnostic"],
                )
            )
    return entries


def rejected_entry_count(base_corpus: Path) -> int:
    catalog = read_text(base_corpus / "examples.md")
    return sum(
        1
        for section in catalog_sections(catalog)
        if section.get("checker-result") == "rejected"
    )


def report_count(report_text: str, key: str) -> str:
    prefix = key + " "
    for line in report_text.splitlines():
        if line.startswith(prefix):
            return line.removeprefix(prefix)
    raise SystemExit(f"examples report missing {key}")


def verified_artifact(
    entry_dir: Path,
    entry_id: str,
    artifact_label: str,
    artifact_name: str,
    fingerprint_name: str,
    examples_report: str,
    manifest: str,
) -> tuple[str, str, str]:
    artifact_path = entry_dir / artifact_name
    fingerprint_path = entry_dir / fingerprint_name
    require(artifact_path.is_file(), f"{entry_id} missing {artifact_name}")
    require(fingerprint_path.is_file(), f"{entry_id} missing {fingerprint_name}")
    text = read_text(artifact_path)
    expected = fnv64_text(text)
    actual = read_text(fingerprint_path).strip()
    require(
        actual == expected,
        f"{entry_id} {artifact_name} fingerprint expected {expected}, got {actual}",
    )
    relative = f"examples/{entry_id}/{artifact_name}"
    line = f"entry-artifact {entry_id} {artifact_label} {relative} {actual}"
    require_contains(examples_report, line, "examples report")
    require_contains(manifest, line, "examples manifest")
    return artifact_label, relative, actual


def verify_entry(
    artifact_root: Path,
    entry: RepairEntry,
    examples_report: str,
    manifest: str,
) -> tuple[str, list[tuple[str, str, str]]]:
    entry_dir = artifact_root / "examples" / entry.entry_id
    require(entry_dir.is_dir(), f"missing artifact dir for {entry.entry_id}")

    artifact_lines = [
        verified_artifact(
            entry_dir,
            entry.entry_id,
            label,
            artifact_name,
            fingerprint_name,
            examples_report,
            manifest,
        )
        for label, artifact_name, fingerprint_name in REPAIR_ARTIFACTS
    ]

    tutorial = read_text(entry_dir / "repair-tutorial.txt")
    require_contains(tutorial, "AIL-Repair-Tutorial:", f"{entry.entry_id} tutorial")
    require_contains(tutorial, f"entry {entry.entry_id}", f"{entry.entry_id} tutorial")
    require_contains(
        tutorial,
        f"failure-taxonomy {entry.failure_taxonomy}",
        f"{entry.entry_id} tutorial",
    )
    require_contains(
        tutorial,
        f"expected-diagnostic {entry.expected_diagnostic}",
        f"{entry.entry_id} tutorial",
    )
    for step in [
        "repair-step 1 Preserve the rejected transcript",
        "repair-step 2 Draft a corrected spec",
        "repair-step 3 Replay ail-examples",
    ]:
        require_contains(tutorial, step, f"{entry.entry_id} tutorial")

    candidate = read_text(entry_dir / "repair-candidate.ail-spec.md")
    require(candidate.strip() != "", f"{entry.entry_id} repair candidate is empty")
    core = read_text(entry_dir / "repair-checked.ail-core.txt")
    require_contains(core, "package:", f"{entry.entry_id} checked Core")
    json.loads(read_text(entry_dir / "repair-artifact.ailbc.json"))

    diff = read_text(entry_dir / "repair-diff.txt")
    require_contains(diff, "AIL-Repair-Diff:", f"{entry.entry_id} diff")
    require_contains(
        diff,
        "checker-result rejected-to-repaired",
        f"{entry.entry_id} diff",
    )
    require_contains(
        diff,
        f"failure-taxonomy {entry.failure_taxonomy}",
        f"{entry.entry_id} diff",
    )
    require_contains(
        diff,
        f"expected-diagnostic {entry.expected_diagnostic}",
        f"{entry.entry_id} diff",
    )
    require_contains(
        diff,
        "expected-diagnostic-removed true",
        f"{entry.entry_id} diff",
    )
    require_contains(
        diff,
        "semantic-anchor-missing-count 0",
        f"{entry.entry_id} diff",
    )
    diff_values = parse_key_values(diff)
    repair_evidence_kind = diff_values.get("repair-evidence-kind", "")
    require(
        repair_evidence_kind in {"repair-vm-trace", "repair-target-report"},
        f"{entry.entry_id} unknown repair evidence kind {repair_evidence_kind}",
    )
    evidence_artifact = verified_artifact(
        entry_dir,
        entry.entry_id,
        repair_evidence_kind,
        f"{repair_evidence_kind}.txt",
        f"{repair_evidence_kind}.fingerprint.txt",
        examples_report,
        manifest,
    )
    artifact_lines.append(evidence_artifact)

    review = read_text(entry_dir / "repair-promotion-review.txt")
    require_contains(
        review,
        "AIL-Repair-Promotion-Review:",
        f"{entry.entry_id} promotion review",
    )
    values = parse_key_values(review)
    expected_values = {
        "entry": entry.entry_id,
        "promotion-decision": "accepted-for-promotion",
        "human-approval-required": "true",
        "checker-result": "rejected-to-repaired",
        "failure-taxonomy": entry.failure_taxonomy,
        "expected-diagnostic": entry.expected_diagnostic,
        "expected-diagnostic-removed": "true",
        "repair-evidence-kind": repair_evidence_kind,
        "semantic-anchor-missing-count": "0",
    }
    for key, expected in expected_values.items():
        actual = values.get(key)
        require(
            actual == expected,
            f"{entry.entry_id} promotion review {key} expected {expected}, got {actual}",
        )
    for label, _, fingerprint in artifact_lines:
        review_key = {
            "diagnostics": "diagnostics-fingerprint",
            "repair-tutorial": "repair-tutorial-fingerprint",
            "repair-candidate": "repair-candidate-fingerprint",
            "repair-checked-core": "repair-checked-core-fingerprint",
            "repair-bytecode": "repair-bytecode-fingerprint",
            repair_evidence_kind: "repair-evidence-fingerprint",
            "repair-diff": "repair-diff-fingerprint",
        }.get(label)
        if review_key is not None:
            require(
                values.get(review_key) == fingerprint,
                f"{entry.entry_id} promotion review {review_key} mismatch",
            )
    return repair_evidence_kind, artifact_lines


def build_report(
    base_corpus: Path,
    examples_artifacts: Path,
    entries: list[RepairEntry],
    total_rejected: int,
    evidence_by_entry: dict[str, str],
    artifacts_by_entry: dict[str, list[tuple[str, str, str]]],
) -> str:
    failure_taxonomies = sorted({entry.failure_taxonomy for entry in entries})
    lines = [
        "AIL-v0.3-Rejected-Repair-Audit:",
        f"base-corpus {base_corpus}",
        f"examples-artifacts {examples_artifacts}",
        f"v03-roadmap-signal {REPAIR_SIGNAL}",
        f"signal-entry-count {len(entries)}",
        f"total-rejected-entry-count {total_rejected}",
        f"failure-taxonomy-count {len(failure_taxonomies)}",
    ]
    for taxonomy in failure_taxonomies:
        lines.append(f"failure-taxonomy {taxonomy}")
    for entry in entries:
        repair_evidence_kind = evidence_by_entry[entry.entry_id]
        lines.append(
            f"entry {entry.entry_id} failure-taxonomy {entry.failure_taxonomy} "
            f"expected-diagnostic {entry.expected_diagnostic} "
            f"repair-evidence-kind {repair_evidence_kind}"
        )
        for label, relative, fingerprint in artifacts_by_entry[entry.entry_id]:
            lines.append(
                f"entry-artifact {entry.entry_id} {label} {relative} {fingerprint}"
            )
    lines.extend(
        [
            f"repair-tutorial-count {len(entries)}",
            f"repair-candidate-count {len(entries)}",
            f"repair-checked-core-count {len(entries)}",
            f"repair-bytecode-count {len(entries)}",
            f"repair-evidence-count {len(entries)}",
            f"repair-diff-count {len(entries)}",
            f"repair-promotion-review-count {len(entries)}",
            f"expected-diagnostic-removed-count {len(entries)}",
            "semantic-anchor-missing-count 0",
            f"promotion-ready-count {len(entries)}",
            "audit-result accepted",
            "",
        ]
    )
    return "\n".join(lines)


def write_artifacts(
    artifact_dir: Path,
    report: str,
    examples_artifacts: Path,
    artifacts_by_entry: dict[str, list[tuple[str, str, str]]],
) -> None:
    artifact_dir.mkdir(parents=True, exist_ok=True)
    report_fingerprint = fnv64_text(report)
    (artifact_dir / "rejected-repair-audit-report.txt").write_text(
        report,
        encoding="utf-8",
    )
    (artifact_dir / "rejected-repair-audit-report.fingerprint.txt").write_text(
        report_fingerprint + "\n",
        encoding="utf-8",
    )

    manifest_lines = [
        "AIL-v0.3-Rejected-Repair-Audit-Manifest:",
        f"report rejected-repair-audit-report.txt {report_fingerprint}",
        f"examples-artifacts {examples_artifacts}",
    ]
    for entry_id, artifacts in artifacts_by_entry.items():
        for label, _, fingerprint in artifacts:
            manifest_lines.append(f"entry {entry_id} {label} {fingerprint}")
    manifest_lines.extend(["audit-result accepted", ""])
    manifest = "\n".join(manifest_lines)
    (artifact_dir / "manifest.v03-rejected-repair-audit.txt").write_text(
        manifest,
        encoding="utf-8",
    )
    (artifact_dir / "manifest.fingerprint.txt").write_text(
        fnv64_text(manifest) + "\n",
        encoding="utf-8",
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-corpus", default="examples", type=Path)
    parser.add_argument(
        "--examples-artifacts",
        default="/tmp/ail-v03-examples",
        type=Path,
    )
    parser.add_argument(
        "--artifact-dir",
        default="/tmp/ail-v03-rejected-repair-audit",
        type=Path,
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.artifact_dir.exists():
        shutil.rmtree(args.artifact_dir)
    examples_report = read_text(args.examples_artifacts / "examples-report.txt")
    manifest = read_text(args.examples_artifacts / "manifest.ail-examples.txt")
    entries = repair_signal_entries(args.base_corpus)
    require(len(entries) == 8, f"expected 8 repair signal entries, got {len(entries)}")
    total_rejected = rejected_entry_count(args.base_corpus)
    require(total_rejected == 9, f"expected 9 rejected entries, got {total_rejected}")
    require(
        report_count(examples_report, "checker-result-count rejected") == str(total_rejected),
        "examples report rejected count does not match catalog",
    )

    evidence_by_entry: dict[str, str] = {}
    artifacts_by_entry: dict[str, list[tuple[str, str, str]]] = {}
    for entry in entries:
        repair_evidence_kind, artifact_lines = verify_entry(
            args.examples_artifacts,
            entry,
            examples_report,
            manifest,
        )
        evidence_by_entry[entry.entry_id] = repair_evidence_kind
        artifacts_by_entry[entry.entry_id] = artifact_lines

    report = build_report(
        args.base_corpus,
        args.examples_artifacts,
        entries,
        total_rejected,
        evidence_by_entry,
        artifacts_by_entry,
    )
    write_artifacts(args.artifact_dir, report, args.examples_artifacts, artifacts_by_entry)
    sys.stdout.write(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
