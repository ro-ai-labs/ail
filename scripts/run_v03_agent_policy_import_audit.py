#!/usr/bin/env python3
"""Bundle deterministic AgentTool policy handoff import evidence for v0.3."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

from run_v02_release_audit import fnv64_fingerprint


ROOT = Path(__file__).resolve().parents[1]
POLICY_HANDOFF_TRACE = "PolicyHandoffApprovedScenario40"


def fnv64_text(text: str) -> str:
    return fnv64_fingerprint(text.encode("utf-8"))


def read_fingerprinted_text(path: Path, fingerprint_path: Path, label: str) -> tuple[str, str]:
    text = path.read_text(encoding="utf-8")
    expected = fingerprint_path.read_text(encoding="utf-8").strip()
    actual = fnv64_text(text)
    if expected != actual:
        raise SystemExit(f"{label} fingerprint mismatch: expected {expected} got {actual}")
    return text, actual


def run_command(command: list[str]) -> None:
    subprocess.run(command, cwd=ROOT, check=True)


def copy_fingerprinted_text(
    source_path: Path,
    source_fingerprint_path: Path,
    output_dir: Path,
    output_name: str,
    label: str,
) -> str:
    text, fingerprint = read_fingerprinted_text(source_path, source_fingerprint_path, label)
    (output_dir / output_name).write_text(text, encoding="utf-8")
    (output_dir / f"{output_name.removesuffix('.txt')}.fingerprint.txt").write_text(
        fingerprint + "\n",
        encoding="utf-8",
    )
    return fingerprint


def read_plan(capture_plan_dir: Path) -> tuple[dict[str, object], str]:
    plan_path = capture_plan_dir / "agent-policy-capture-plan.json"
    fingerprint_path = capture_plan_dir / "agent-policy-capture-plan.fingerprint.txt"
    plan_text = plan_path.read_text(encoding="utf-8")
    expected = fingerprint_path.read_text(encoding="utf-8").strip()
    actual = fnv64_text(plan_text)
    if expected != actual:
        raise SystemExit(f"capture plan fingerprint mismatch: expected {expected} got {actual}")
    plan = json.loads(plan_text)
    if not isinstance(plan, dict):
        raise SystemExit("agent policy capture plan must be an object")
    return plan, actual


def plan_string(plan: dict[str, object], field: str) -> str:
    value = plan.get(field)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"agent policy capture plan is missing {field}")
    return value


def require_line(text: str, line: str, label: str) -> None:
    if line not in text:
        raise SystemExit(f"{label} missing {line}")


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-corpus", default="examples")
    parser.add_argument("--examples-artifacts", required=True)
    parser.add_argument("--source-entry-id", default="example-40")
    parser.add_argument("--output-dir", default="/tmp/ail-v03-agent-policy-import")
    parser.add_argument("--proposed-entry-id")
    parser.add_argument("--semantic-task", default="refund-tool-policy-handoff-import-40")
    args = parser.parse_args(argv)
    args.examples_artifacts = Path(args.examples_artifacts)
    args.output_dir = Path(args.output_dir)
    return args


def build_report(
    args: argparse.Namespace,
    capture_plan_fingerprint: str,
    import_report_fingerprint: str,
    handoff_report_fingerprint: str,
    checked_core_fingerprint: str,
    proposed_entry_id: str,
) -> str:
    lines = [
        "AIL-v0.3-Agent-Policy-Import-Audit:",
        f"base-corpus {args.base_corpus}",
        f"examples-artifacts {args.examples_artifacts}",
        f"source-entry-id {args.source_entry_id}",
        f"proposed-entry-id {proposed_entry_id}",
        f"capture-plan capture-plan/agent-policy-capture-plan.json {capture_plan_fingerprint}",
        "import-demo import-work/agent-policy-import-demo-report.txt "
        f"{import_report_fingerprint}",
        "multi-agent-handoff handoff/agent-policy-multi-agent-handoff-report.txt "
        f"{handoff_report_fingerprint}",
        "policy-handoff-imported true",
        "policy-handoff-replayed true",
        "multi-agent-execution-evidence deterministic-role-handoff",
        f"output-corpus-entry {proposed_entry_id}",
        "output-artifact "
        f"examples/{proposed_entry_id}/checked.ail-core.txt {checked_core_fingerprint}",
        "audit-result accepted",
        "",
    ]
    return "\n".join(lines)


def write_manifest(
    output_dir: Path,
    report: str,
    capture_plan_fingerprint: str,
    import_report_fingerprint: str,
    handoff_report_fingerprint: str,
    checked_core_fingerprint: str,
    proposed_entry_id: str,
) -> None:
    report_fingerprint = fnv64_text(report)
    (output_dir / "agent-policy-import-audit-report.txt").write_text(
        report,
        encoding="utf-8",
    )
    (output_dir / "agent-policy-import-audit-report.fingerprint.txt").write_text(
        report_fingerprint + "\n",
        encoding="utf-8",
    )
    manifest = "\n".join(
        [
            "AIL-v0.3-Agent-Policy-Import-Manifest:",
            f"report agent-policy-import-audit-report.txt {report_fingerprint}",
            f"artifact agent-policy-capture-plan.json {capture_plan_fingerprint}",
            f"artifact agent-policy-import-demo-report.txt {import_report_fingerprint}",
            "artifact agent-policy-multi-agent-handoff-report.txt "
            f"{handoff_report_fingerprint}",
            "artifact "
            f"output-artifacts/examples/{proposed_entry_id}/checked.ail-core.txt "
            f"{checked_core_fingerprint}",
            "audit-result accepted",
            "",
        ]
    )
    (output_dir / "manifest.v03-agent-policy-import.txt").write_text(
        manifest,
        encoding="utf-8",
    )
    (output_dir / "manifest.fingerprint.txt").write_text(
        fnv64_text(manifest) + "\n",
        encoding="utf-8",
    )


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if args.output_dir.exists():
        shutil.rmtree(args.output_dir)
    args.output_dir.mkdir(parents=True)

    capture_plan_dir = args.output_dir / "capture-plan"
    import_work_dir = args.output_dir / "import-work"
    output_corpus = args.output_dir / "output-corpus"
    output_artifacts = args.output_dir / "output-artifacts"
    handoff_dir = args.output_dir / "handoff"

    capture_command = [
        "python3",
        "scripts/run_v03_agent_policy_capture_plan.py",
        "--examples-artifacts",
        str(args.examples_artifacts),
        "--entry-id",
        args.source_entry_id,
        "--output-dir",
        str(capture_plan_dir),
    ]
    if args.proposed_entry_id:
        capture_command.extend(["--proposed-entry-id", args.proposed_entry_id])
    run_command(capture_command)

    plan, capture_plan_fingerprint = read_plan(capture_plan_dir)
    proposed_entry_id = plan_string(plan, "proposed_entry_id")

    run_command(
        [
            "python3",
            "scripts/run_v03_agent_policy_import_demo.py",
            "--base-corpus",
            args.base_corpus,
            "--examples-artifacts",
            str(args.examples_artifacts),
            "--capture-plan-dir",
            str(capture_plan_dir),
            "--source-entry-id",
            args.source_entry_id,
            "--work-dir",
            str(import_work_dir),
            "--output-corpus",
            str(output_corpus),
            "--output-artifacts",
            str(output_artifacts),
            "--semantic-task",
            args.semantic_task,
        ]
    )

    run_command(
        [
            "python3",
            "scripts/run_v03_agent_policy_multi_agent_handoff.py",
            "--examples-artifacts",
            str(args.examples_artifacts),
            "--capture-plan-dir",
            str(capture_plan_dir),
            "--import-work-dir",
            str(import_work_dir),
            "--output-artifacts",
            str(output_artifacts),
            "--source-entry-id",
            args.source_entry_id,
            "--output-dir",
            str(handoff_dir),
        ]
    )

    import_report_text, import_report_fingerprint = read_fingerprinted_text(
        import_work_dir / "agent-policy-import-demo-report.txt",
        import_work_dir / "agent-policy-import-demo-report.fingerprint.txt",
        "agent policy import demo report",
    )
    handoff_report_text, handoff_report_fingerprint = read_fingerprinted_text(
        handoff_dir / "agent-policy-multi-agent-handoff-report.txt",
        handoff_dir / "agent-policy-multi-agent-handoff-report.fingerprint.txt",
        "agent policy multi-agent handoff report",
    )
    for required in [
        f"source-entry-id {args.source_entry_id}",
        f"proposed-entry-id {proposed_entry_id}",
        "source-preserved true",
        "proposed-accepted true",
        "policy-handoff-imported true",
        "policy-handoff-replayed true",
    ]:
        require_line(import_report_text, required, "agent policy import demo report")
    for required in [
        f"source-entry-id {args.source_entry_id}",
        f"proposed-entry-id {proposed_entry_id}",
        "agent-contracts-result accepted",
        "multi-agent-execution-evidence deterministic-role-handoff",
    ]:
        require_line(handoff_report_text, required, "agent policy multi-agent handoff report")

    examples_text = (output_corpus / "examples.md").read_text(encoding="utf-8")
    require_line(examples_text, f"## Example: {proposed_entry_id}", "output corpus")
    checked_core_path = (
        output_artifacts / "examples" / proposed_entry_id / "checked.ail-core.txt"
    )
    checked_core_text = checked_core_path.read_text(encoding="utf-8")
    require_line(checked_core_text, f"node Trace {POLICY_HANDOFF_TRACE}", "checked Core")
    checked_core_fingerprint = fnv64_text(checked_core_text)

    shutil.copy2(
        capture_plan_dir / "agent-policy-capture-plan.json",
        args.output_dir / "agent-policy-capture-plan.json",
    )
    (args.output_dir / "agent-policy-capture-plan.fingerprint.txt").write_text(
        capture_plan_fingerprint + "\n",
        encoding="utf-8",
    )
    copy_fingerprinted_text(
        import_work_dir / "agent-policy-import-demo-report.txt",
        import_work_dir / "agent-policy-import-demo-report.fingerprint.txt",
        args.output_dir,
        "agent-policy-import-demo-report.txt",
        "agent policy import demo report",
    )
    copy_fingerprinted_text(
        handoff_dir / "agent-policy-multi-agent-handoff-report.txt",
        handoff_dir / "agent-policy-multi-agent-handoff-report.fingerprint.txt",
        args.output_dir,
        "agent-policy-multi-agent-handoff-report.txt",
        "agent policy multi-agent handoff report",
    )

    report = build_report(
        args,
        capture_plan_fingerprint,
        import_report_fingerprint,
        handoff_report_fingerprint,
        checked_core_fingerprint,
        proposed_entry_id,
    )
    write_manifest(
        args.output_dir,
        report,
        capture_plan_fingerprint,
        import_report_fingerprint,
        handoff_report_fingerprint,
        checked_core_fingerprint,
        proposed_entry_id,
    )
    sys.stdout.write(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
