#!/usr/bin/env python3
"""Run a deterministic UI patch import demo against a corpus copy."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def fnv64(text: str) -> str:
    value = 0xCBF29CE484222325
    for byte in text.encode():
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv64:{value:016x}"


def write_json(path: Path, payload: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def read_plan(capture_plan_dir: Path) -> tuple[dict[str, object], str]:
    plan_path = capture_plan_dir / "ui-patch-capture-plan.json"
    fingerprint_path = capture_plan_dir / "ui-patch-capture-plan.fingerprint.txt"
    plan_text = plan_path.read_text()
    expected = fingerprint_path.read_text().strip()
    actual = fnv64(plan_text)
    if expected != actual:
        raise SystemExit(
            f"capture plan fingerprint mismatch: expected {expected} got {actual}"
        )
    plan = json.loads(plan_text)
    if not isinstance(plan, dict):
        raise SystemExit("capture plan must be an object")
    return plan, actual


def plan_string(plan: dict[str, object], field: str) -> str:
    value = plan.get(field)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"capture plan is missing {field}")
    return value


def run_command(command: list[str]) -> None:
    subprocess.run(command, cwd=ROOT, check=True)


def run_text_command(command: list[str]) -> str:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    return completed.stdout.rstrip("\n")


def extract_artifact_text(response_path: Path) -> str:
    response = json.loads(response_path.read_text())
    if not isinstance(response, dict):
        raise SystemExit(f"response artifact must be an object: {response_path}")
    for field in ["content", "artifact_text"]:
        value = response.get(field)
        if isinstance(value, str) and value.strip():
            return value.strip()
    raise SystemExit(f"response artifact is missing content/artifact_text: {response_path}")


def add_review_status_field(source_spec_text: str) -> str:
    if "- reviewStatus: State<Pending, Approved>" in source_spec_text:
        return source_spec_text
    marker = "- title: Text\n- status: State<New, Open, Overdue, Refunded>"
    replacement = (
        "- title: Text\n"
        "- reviewStatus: State<Pending, Approved>\n"
        "- status: State<New, Open, Overdue, Refunded>"
    )
    if marker not in source_spec_text:
        raise SystemExit("source UI spec is missing the Ticket title/status field block")
    return source_spec_text.replace(marker, replacement, 1)


def section_for_entry(examples_text: str, entry_id: str) -> str:
    marker = f"## Example: {entry_id}"
    if marker not in examples_text:
        raise SystemExit(f"examples output is missing {entry_id}")
    section = examples_text.split(marker, 1)[1]
    return section.split("## Example:", 1)[0]


def line_present(text: str, line: str) -> bool:
    return any(candidate.strip() == line for candidate in text.splitlines())


def require_report_line(report_text: str, line: str) -> None:
    if line not in report_text:
        raise SystemExit(f"replay report missing {line}")


def require_plan_value(plan: dict[str, object], field: str, expected: object) -> None:
    actual = plan.get(field)
    if actual != expected:
        raise SystemExit(
            f"capture plan {field} expected {expected}, got {actual or '<missing>'}"
        )


def build_flow_edit(source_core_text: str, plan: dict[str, object]) -> dict[str, object]:
    return {
        "schema": "ail-flow.edit.v0",
        "package": "ui-workflow",
        "base_hash": f"ail-core:{fnv64(source_core_text)}",
        "source_view": "DataTable:Ticket",
        "edits": [
            {
                "op": "DataTable.addField",
                "target": "Thing:Ticket",
                "name": "reviewStatus",
                "type": "State<Pending, Approved>",
                "secret": "false",
                "provenance": [
                    "flow:DataTable:Ticket.field:reviewStatus",
                    f"ui-review-patch:{plan_string(plan, 'source_entry_id')}",
                ],
            }
        ],
    }


def build_batch_plan(
    args: argparse.Namespace,
    plan: dict[str, object],
    request_path: Path,
    response_path: Path,
    batch_plan_path: Path,
) -> None:
    source_entry_id = plan_string(plan, "source_entry_id")
    proposed_entry_id = plan_string(plan, "proposed_entry_id")
    write_json(
        batch_plan_path,
        {
            "entries": [
                {
                    "entry_id": proposed_entry_id,
                    "source_entry_id": source_entry_id,
                    "executor_family": "codex-skill-agent",
                    "executor_label": args.executor_label,
                    "semantic_task": args.semantic_task,
                    "request_json_file": str(request_path),
                    "response_json_file": str(response_path),
                    "checker_result": "accepted",
                    "ui_patch_capture_plan_json": str(
                        args.capture_plan_dir / "ui-patch-capture-plan.json"
                    ),
                }
            ]
        },
    )


def validate_plan(plan: dict[str, object], source_entry_id: str) -> None:
    require_plan_value(plan, "artifact_kind", "AIL-UI-Patch-Capture-Plan")
    require_plan_value(plan, "source_entry_id", source_entry_id)
    require_plan_value(plan, "status", "plan-only")
    require_plan_value(plan, "patch_import_decision", "accepted-for-import")
    require_plan_value(plan, "patch_command", "ail-flow-edit")
    require_plan_value(plan, "patch_import_status", "proposed-only")
    require_plan_value(plan, "human_approval_required", True)
    require_plan_value(plan, "must_supply_request_response_json", True)
    require_plan_value(plan, "preserve_source_entry", True)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Append a human-approved UI patch entry to a corpus copy and replay it."
    )
    parser.add_argument("--base-corpus", default="examples")
    parser.add_argument("--examples-artifacts", default="/tmp/ail-v03-ui-patch")
    parser.add_argument("--capture-plan-dir", default="/tmp/ail-v03-ui-patch-capture-plan")
    parser.add_argument("--source-entry-id", default="example-108")
    parser.add_argument("--work-dir", default="/tmp/ail-v03-ui-patch-import-work")
    parser.add_argument("--output-corpus", default="/tmp/ail-v03-ui-patch-import-corpus")
    parser.add_argument("--output-artifacts", default="/tmp/ail-v03-ui-patch-import-artifacts")
    parser.add_argument("--executor-label", default="codex-ail-ui-patch-reviewer-demo")
    parser.add_argument("--semantic-task", default="ui-workflow-human-approved-patch-108")
    parsed = parser.parse_args(argv)
    parsed.examples_artifacts = Path(parsed.examples_artifacts)
    parsed.capture_plan_dir = Path(parsed.capture_plan_dir)
    parsed.work_dir = Path(parsed.work_dir)
    parsed.output_corpus = Path(parsed.output_corpus)
    parsed.output_artifacts = Path(parsed.output_artifacts)
    return parsed


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    plan, plan_fingerprint = read_plan(args.capture_plan_dir)
    validate_plan(plan, args.source_entry_id)
    source_entry_id = plan_string(plan, "source_entry_id")
    proposed_entry_id = plan_string(plan, "proposed_entry_id")

    args.work_dir.mkdir(parents=True, exist_ok=True)
    if args.output_artifacts.exists():
        shutil.rmtree(args.output_artifacts)
    request_path = args.work_dir / "approved-ui-patch-request.json"
    response_path = args.work_dir / "approved-ui-patch-response.json"
    flow_edit_path = args.work_dir / "approved-ui-patch.ail-flow.edit.json"
    patched_core_path = args.work_dir / "approved-ui-patch.checked.ail-core.txt"
    patched_spec_path = args.work_dir / "approved-ui-patch.ail-spec.md"
    core_roundtrip_spec_path = args.work_dir / "approved-ui-patch.core-roundtrip.ail-spec.md"
    batch_plan_path = args.work_dir / "human-approved-ui-patch-batch.json"

    entry_artifact_dir = args.examples_artifacts / "examples" / source_entry_id
    source_core_path = entry_artifact_dir / "checked.ail-core.txt"
    ui_review_patch_path = entry_artifact_dir / "ui-review-patch.txt"
    source_response_path = ROOT / args.base_corpus / "responses" / f"{source_entry_id}.json"
    source_core_text = source_core_path.read_text()
    ui_review_patch_text = ui_review_patch_path.read_text()
    source_spec_text = extract_artifact_text(source_response_path)
    source_core_fingerprint_preserved = (
        fnv64(source_core_text) == plan_string(plan, "checked_core_fingerprint")
    )
    ui_review_patch_fingerprint_preserved = (
        fnv64(ui_review_patch_text) == plan_string(plan, "ui_review_patch_fingerprint")
    )
    if not source_core_fingerprint_preserved or not ui_review_patch_fingerprint_preserved:
        raise SystemExit("source UI patch evidence does not match capture plan fingerprints")

    flow_edit = build_flow_edit(source_core_text, plan)
    write_json(flow_edit_path, flow_edit)
    patched_core_text = run_text_command(
        [
            "cargo",
            "run",
            "--quiet",
            "--",
            "ail-flow-edit",
            "--core-file",
            str(source_core_path),
            str(flow_edit_path),
        ]
    )
    patched_core_path.write_text(patched_core_text)
    flow_edit_applied = (
        "node Field Ticket.reviewStatus" in patched_core_text
        and patched_core_text != source_core_text
    )
    if not flow_edit_applied:
        raise SystemExit("AIL-Flow edit did not add Ticket.reviewStatus")

    core_roundtrip_spec_text = run_text_command(
        [
            "cargo",
            "run",
            "--quiet",
            "--",
            "ail-spec",
            "--core-file",
            str(patched_core_path),
        ]
    )
    core_roundtrip_spec_path.write_text(core_roundtrip_spec_text)
    patched_spec_text = add_review_status_field(source_spec_text)
    patched_spec_path.write_text(patched_spec_text)
    write_json(
        request_path,
        {
            "approval_mode": "deterministic-demo",
            "core_roundtrip_spec_fingerprint": fnv64(core_roundtrip_spec_text),
            "executor_label": args.executor_label,
            "flow_edit": flow_edit,
            "flow_edit_fingerprint": fnv64(json.dumps(flow_edit, indent=2, sort_keys=True) + "\n"),
            "promoted_spec_fingerprint": fnv64(patched_spec_text),
            "source_entry_id": source_entry_id,
            "source_spec_fingerprint": fnv64(source_spec_text),
            "task": "Approve the UI review patch and replay the patched spec.",
            "ui_patch_capture_plan_fingerprint": plan_fingerprint,
            "ui_review_patch_fingerprint": plan_string(plan, "ui_review_patch_fingerprint"),
        },
    )
    write_json(
        response_path,
        {
            "artifact_text": patched_spec_text,
            "model": "human-approved-ui-patch-import-demo",
        },
    )
    build_batch_plan(args, plan, request_path, response_path, batch_plan_path)

    run_command(
        [
            sys.executable,
            "scripts/capture_example_batch.py",
            "--base-corpus",
            args.base_corpus,
            "--output-dir",
            str(args.output_corpus),
            "--plan-json",
            str(batch_plan_path),
        ]
    )
    run_command(
        [
            "cargo",
            "run",
            "--quiet",
            "--",
            "ail-examples",
            str(args.output_corpus),
            "--artifact-dir",
            str(args.output_artifacts),
            "--release-evidence",
        ]
    )

    examples_text = (args.output_corpus / "examples.md").read_text()
    source_section = section_for_entry(examples_text, source_entry_id)
    proposed_section = section_for_entry(examples_text, proposed_entry_id)
    source_preserved = line_present(source_section, "checker-result: accepted")
    proposed_accepted = line_present(proposed_section, "checker-result: accepted")
    patched_core_replayed_path = (
        args.output_artifacts / "examples" / proposed_entry_id / "checked.ail-core.txt"
    )
    patched_core_replayed = (
        patched_core_replayed_path.exists()
        and "node Field Ticket.reviewStatus" in patched_core_replayed_path.read_text()
    )
    report_text = (args.output_artifacts / "examples-report.txt").read_text()
    for line in [
        "entry-count 124",
        "checker-result-count accepted 115",
        "checker-result-count rejected 9",
        f"entry {source_entry_id} ",
        f"entry {proposed_entry_id} ",
    ]:
        require_report_line(report_text, line)
    output_lines = [
        "AIL-UI-Patch-Import-Demo:",
        f"source-entry-id {source_entry_id}",
        f"proposed-entry-id {proposed_entry_id}",
        f"source-preserved {str(source_preserved).lower()}",
        f"proposed-accepted {str(proposed_accepted).lower()}",
        "ui-review-patch-fingerprint-preserved "
        f"{str(ui_review_patch_fingerprint_preserved).lower()}",
        "checked-core-fingerprint-preserved "
        f"{str(source_core_fingerprint_preserved).lower()}",
        f"flow-edit-applied {str(flow_edit_applied).lower()}",
        f"patched-core-replayed {str(patched_core_replayed).lower()}",
        "entry-count 124",
        "checker-result-count accepted 115",
        "checker-result-count rejected 9",
        f"flow-edit {flow_edit_path}",
        f"batch-plan {batch_plan_path}",
        f"output-corpus {args.output_corpus}",
        f"output-artifacts {args.output_artifacts}",
        "",
    ]
    output_text = "\n".join(output_lines)
    if (
        not source_preserved
        or not proposed_accepted
        or not patched_core_replayed
        or not source_core_fingerprint_preserved
        or not ui_review_patch_fingerprint_preserved
    ):
        raise SystemExit(output_text)
    report_path = args.work_dir / "ui-patch-import-demo-report.txt"
    report_path.write_text(output_text)
    (args.work_dir / "ui-patch-import-demo-report.fingerprint.txt").write_text(
        fnv64(output_text) + "\n"
    )
    sys.stdout.write(output_text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
