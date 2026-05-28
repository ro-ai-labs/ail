#!/usr/bin/env python3
"""Write deterministic multi-story User Story promotion batch evidence."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from run_v02_release_audit import fnv64_fingerprint


ROOT = Path(__file__).resolve().parents[1]
STORY_PROMOTION_SIGNAL = (
    "User Story mode needs reviewer-produced promotion decisions and multi-story "
    "promotion variants after deterministic promotion imports are replayed."
)
REVIEWER_CONTRACT = "examples/agents/codex-ail-story-promotion-reviewer.md"
REVIEWER_SKILL = "examples/agents/skills/ail-story-promotion-reviewer/SKILL.md"


def fnv64_text(text: str) -> str:
    return fnv64_fingerprint(text.encode("utf-8"))


def resolve_workspace_path(path: str | Path) -> Path:
    candidate = Path(path)
    if candidate.is_absolute():
        return candidate
    return ROOT / candidate


def read_fingerprinted_text(path: Path, fingerprint_path: Path, label: str) -> tuple[str, str]:
    text = path.read_text(encoding="utf-8")
    expected = fingerprint_path.read_text(encoding="utf-8").strip()
    actual = fnv64_text(text)
    if expected != actual:
        raise SystemExit(f"{label} fingerprint mismatch: expected {expected} got {actual}")
    return text, actual


def parse_catalog(corpus: Path) -> list[dict[str, str]]:
    catalog_path = corpus / "examples.md"
    text = catalog_path.read_text(encoding="utf-8")
    entries: list[dict[str, str]] = []
    for block in text.split("## Example: ")[1:]:
        lines = block.splitlines()
        if not lines:
            continue
        entry: dict[str, str] = {"entry-id": lines[0].strip()}
        for line in lines[1:]:
            if not line.strip():
                continue
            if line.startswith("## "):
                break
            if ": " in line:
                key, value = line.split(": ", 1)
                entry[key.strip()] = value.strip()
        entries.append(entry)
    return entries


def story_entry_sort_key(entry: dict[str, str]) -> tuple[int, str]:
    entry_id = entry["entry-id"]
    match = re.match(r"example-(\d+)(.*)", entry_id)
    if match:
        return (int(match.group(1)), match.group(2))
    return (10_000_000, entry_id)


def require_line(text: str, line: str, label: str) -> None:
    if line not in text:
        raise SystemExit(f"{label} missing {line}")


def line_value(text: str, key: str, label: str) -> str:
    prefix = key + " "
    for line in text.splitlines():
        if line.startswith(prefix):
            return line[len(prefix) :].strip()
    raise SystemExit(f"{label} missing {key}")


def require_existing(path: Path, label: str) -> None:
    if not path.exists():
        raise SystemExit(f"{label} is missing: {path}")


def collect_story_promotion_entries(
    base_corpus: Path, examples_artifacts: Path, min_entry_count: int
) -> tuple[list[dict[str, object]], str, str, str, str]:
    examples_report_text, examples_report_fingerprint = read_fingerprinted_text(
        examples_artifacts / "examples-report.txt",
        examples_artifacts / "examples-report.fingerprint.txt",
        "examples report",
    )
    roadmap_text, roadmap_fingerprint = read_fingerprinted_text(
        examples_artifacts / "v03-roadmap.txt",
        examples_artifacts / "v03-roadmap.fingerprint.txt",
        "v0.3 roadmap",
    )

    catalog_entries = [
        entry
        for entry in parse_catalog(base_corpus)
        if entry.get("checker-result") == "accepted"
        and entry.get("capability-under-test") == "user-story-mode-promotion"
    ]
    catalog_entries.sort(key=story_entry_sort_key)
    if len(catalog_entries) < min_entry_count:
        raise SystemExit(
            f"expected at least {min_entry_count} story promotion entries, "
            f"found {len(catalog_entries)}"
        )

    signal_line = f"signal {STORY_PROMOTION_SIGNAL} count {len(catalog_entries)}"
    require_line(roadmap_text, signal_line, "v0.3 roadmap")

    entries: list[dict[str, object]] = []
    for entry in catalog_entries:
        entry_id = entry["entry-id"]
        review_rel = Path("examples") / entry_id / "story-promotion-review.txt"
        review_fingerprint_rel = Path("examples") / entry_id / (
            "story-promotion-review.fingerprint.txt"
        )
        review_text, review_fingerprint = read_fingerprinted_text(
            examples_artifacts / review_rel,
            examples_artifacts / review_fingerprint_rel,
            f"{entry_id} story promotion review",
        )
        require_line(review_text, "AIL-Story-Promotion-Review:", entry_id)
        require_line(review_text, f"entry {entry_id}", entry_id)
        require_line(review_text, "promotion-decision accepted-for-promotion", entry_id)
        require_line(review_text, "human-approval-required true", entry_id)
        require_line(
            review_text,
            "story-promotion-review-artifact deterministic-text",
            entry_id,
        )
        source_entry_id = line_value(review_text, "source-entry", entry_id)

        for metadata_key in [
            "semantic-task",
            "package",
            "profile",
            "program-domain",
            "user-story-id",
            "request-file",
            "response-file",
            "story-file",
            "story-artifacts",
            "target",
            "vm-action",
        ]:
            if metadata_key not in entry:
                raise SystemExit(f"{entry_id} catalog entry is missing {metadata_key}")

        request_file = Path(entry["request-file"])
        response_file = Path(entry["response-file"])
        story_file = Path(entry["story-file"])
        story_artifacts = Path(entry["story-artifacts"])
        require_existing(base_corpus / request_file, f"{entry_id} request file")
        require_existing(base_corpus / response_file, f"{entry_id} response file")
        require_existing(base_corpus / story_file, f"{entry_id} story file")
        require_existing(base_corpus / story_artifacts, f"{entry_id} story artifacts")

        entries.append(
            {
                "entry_id": entry_id,
                "source_entry_id": source_entry_id,
                "semantic_task": entry["semantic-task"],
                "package": entry["package"],
                "profile": entry["profile"],
                "program_domain": entry["program-domain"],
                "user_story_id": entry["user-story-id"],
                "target": entry["target"],
                "vm_action": entry["vm-action"],
                "request_file": str(request_file),
                "response_file": str(response_file),
                "story_file": str(story_file),
                "story_artifacts": str(story_artifacts),
                "review_file": str(review_rel),
                "review_fingerprint_file": str(review_fingerprint_rel),
                "review_fingerprint": review_fingerprint,
            }
        )
        require_line(
            examples_report_text,
            f"entry-artifact {entry_id} story-promotion-review {review_rel} {review_fingerprint}",
            "examples report",
        )
    return (
        entries,
        examples_report_text,
        examples_report_fingerprint,
        roadmap_text,
        roadmap_fingerprint,
    )


def build_plan(args: argparse.Namespace) -> dict[str, object]:
    base_corpus = resolve_workspace_path(args.base_corpus)
    examples_artifacts = resolve_workspace_path(args.examples_artifacts)
    (
        entries,
        _examples_report_text,
        examples_report_fingerprint,
        _roadmap_text,
        roadmap_fingerprint,
    ) = collect_story_promotion_entries(
        base_corpus, examples_artifacts, args.min_entry_count
    )
    review_fingerprints = [str(entry["review_fingerprint"]) for entry in entries]
    duplicate_count = len(review_fingerprints) - len(set(review_fingerprints))
    return {
        "artifact_kind": "AIL-v0.3-Story-Promotion-Batch-Plan",
        "base_corpus": args.base_corpus,
        "examples_artifacts": str(examples_artifacts),
        "entry_count": len(entries),
        "story_promotion_review_fingerprint_count": len(review_fingerprints),
        "story_promotion_review_fingerprint_duplicate_count": duplicate_count,
        "human_approval_required": True,
        "promotion_source": "human-approved-story-promotion-batch",
        "reviewer_contract": REVIEWER_CONTRACT,
        "reviewer_skill": REVIEWER_SKILL,
        "v03_roadmap_signal": STORY_PROMOTION_SIGNAL,
        "examples_report_fingerprint": examples_report_fingerprint,
        "v03_roadmap_fingerprint": roadmap_fingerprint,
        "entries": entries,
    }


def render_plan_text(plan: dict[str, object]) -> str:
    entries = plan["entries"]
    if not isinstance(entries, list):
        raise SystemExit("plan entries must be a list")
    lines = [
        "AIL-v0.3-Story-Promotion-Batch-Plan:",
        f"base-corpus {plan['base_corpus']}",
        f"examples-artifacts {plan['examples_artifacts']}",
        f"batch-entry-count {plan['entry_count']}",
        "story-promotion-review-fingerprint-count "
        f"{plan['story_promotion_review_fingerprint_count']}",
        "story-promotion-review-fingerprint-duplicate-count "
        f"{plan['story_promotion_review_fingerprint_duplicate_count']}",
        f"human-approval-required {str(plan['human_approval_required']).lower()}",
        f"promotion-source {plan['promotion_source']}",
        f"reviewer-contract {plan['reviewer_contract']}",
        f"reviewer-skill {plan['reviewer_skill']}",
        f"v03-roadmap-signal {plan['v03_roadmap_signal']}",
        f"examples-report examples-report.txt {plan['examples_report_fingerprint']}",
        f"v03-roadmap v03-roadmap.txt {plan['v03_roadmap_fingerprint']}",
    ]
    for entry in entries:
        if not isinstance(entry, dict):
            raise SystemExit("plan entry must be an object")
        entry_id = entry["entry_id"]
        lines.extend(
            [
                f"batch-entry {entry_id}",
                f"batch-entry-source {entry_id} {entry['source_entry_id']}",
                f"batch-entry-semantic-task {entry_id} {entry['semantic_task']}",
                f"batch-entry-package {entry_id} {entry['package']}",
                f"batch-entry-profile {entry_id} {entry['profile']}",
                f"batch-entry-program-domain {entry_id} {entry['program_domain']}",
                f"batch-entry-story-id {entry_id} {entry['user_story_id']}",
                f"batch-entry-target {entry_id} {entry['target']}",
                f"batch-entry-vm-action {entry_id} {entry['vm_action']}",
                f"batch-entry-request {entry_id} {entry['request_file']}",
                f"batch-entry-response {entry_id} {entry['response_file']}",
                f"batch-entry-story-file {entry_id} {entry['story_file']}",
                f"batch-entry-story-artifacts {entry_id} {entry['story_artifacts']}",
                f"batch-entry-review {entry_id} {entry['review_file']} "
                f"{entry['review_fingerprint']}",
                f"batch-entry-review-fingerprint {entry_id} "
                f"{entry['review_fingerprint_file']} {entry['review_fingerprint']}",
            ]
        )
    lines.extend(["audit-result accepted", ""])
    return "\n".join(str(line) for line in lines)


def render_manifest(plan: dict[str, object], plan_text: str, plan_json: str) -> str:
    lines = [
        "AIL-v0.3-Story-Promotion-Batch-Plan-Manifest:",
        f"plan story-promotion-batch-plan.txt {fnv64_text(plan_text)}",
        f"json story-promotion-batch-plan.json {fnv64_text(plan_json)}",
        "fingerprint story-promotion-batch-plan.fingerprint.txt",
        f"entry-count {plan['entry_count']}",
        "story-promotion-review-fingerprint-count "
        f"{plan['story_promotion_review_fingerprint_count']}",
        f"reviewer-contract {plan['reviewer_contract']}",
        f"reviewer-skill {plan['reviewer_skill']}",
        f"v03-roadmap-signal {plan['v03_roadmap_signal']}",
    ]
    entries = plan["entries"]
    if not isinstance(entries, list):
        raise SystemExit("plan entries must be a list")
    for entry in entries:
        if not isinstance(entry, dict):
            raise SystemExit("plan entry must be an object")
        lines.append(
            "story-promotion-review "
            f"{entry['entry_id']} {entry['review_file']} {entry['review_fingerprint']}"
        )
    lines.extend(["audit-result accepted", ""])
    return "\n".join(str(line) for line in lines)


def write_plan(args: argparse.Namespace) -> str:
    artifact_dir = resolve_workspace_path(args.artifact_dir)
    plan = build_plan(args)
    plan_text = render_plan_text(plan)
    plan_json = json.dumps(plan, indent=2, sort_keys=True) + "\n"
    manifest = render_manifest(plan, plan_text, plan_json)

    artifact_dir.mkdir(parents=True, exist_ok=True)
    (artifact_dir / "story-promotion-batch-plan.txt").write_text(
        plan_text, encoding="utf-8"
    )
    (artifact_dir / "story-promotion-batch-plan.json").write_text(
        plan_json, encoding="utf-8"
    )
    (artifact_dir / "story-promotion-batch-plan.fingerprint.txt").write_text(
        fnv64_text(plan_text) + "\n",
        encoding="utf-8",
    )
    (artifact_dir / "manifest.v03-story-promotion-batch-plan.txt").write_text(
        manifest, encoding="utf-8"
    )
    (artifact_dir / "manifest.fingerprint.txt").write_text(
        fnv64_text(manifest) + "\n",
        encoding="utf-8",
    )
    return plan_text


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-corpus", default="examples")
    parser.add_argument("--examples-artifacts", required=True)
    parser.add_argument(
        "--artifact-dir",
        default="/tmp/ail-v03-story-promotion-batch-plan",
    )
    parser.add_argument("--min-entry-count", type=int, default=4)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    plan_text = write_plan(parse_args(argv))
    print(plan_text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
