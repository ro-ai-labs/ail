#!/usr/bin/env python3
"""Run and bundle the AIL v0.3 release audit command set."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

from run_v02_release_audit import (
    AuditResult,
    AuditStep,
    fnv64_fingerprint,
    verify_artifact_dir,
)


def build_v03_audit_plan(
    bundle_root: Path, include_live: bool, live_artifact_root: Path
) -> list[AuditStep]:
    artifacts = bundle_root / "artifacts"
    steps = [
        AuditStep("cargo-fmt", ["cargo", "fmt", "--check"]),
        AuditStep("git-diff-check", ["git", "diff", "--check"]),
        AuditStep("cargo-check", ["cargo", "check"]),
        AuditStep("cargo-test", ["cargo", "test"]),
        AuditStep("cargo-clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]),
        AuditStep(
            "interactive-manual-all",
            ["python3", "scripts/run_ail_interactive_manual.py", "--all", "--run-checks"],
        ),
        AuditStep(
            "interactive-manual-v03-authoring-gate",
            [
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "v03-authoring-gate",
                "--run-checks",
            ],
        ),
        AuditStep("agent-contracts", ["cargo", "run", "--", "ail-agent-contracts", "examples/agents"]),
        AuditStep(
            "conformance-support-ticket",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/support_ticket.ail",
                "--artifact-dir",
                str(artifacts / "v03-conformance-support-ticket"),
            ],
            artifacts / "v03-conformance-support-ticket",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "conformance-secret-access",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/secret_access.ail",
                "--artifact-dir",
                str(artifacts / "v03-conformance-secret-access"),
            ],
            artifacts / "v03-conformance-secret-access",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "conformance-stateful-counter",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/stateful_counter.ail",
                "--artifact-dir",
                str(artifacts / "v03-conformance-stateful-counter"),
            ],
            artifacts / "v03-conformance-stateful-counter",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "conformance-incident-notifications",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/incident_notifications.ail",
                "--artifact-dir",
                str(artifacts / "v03-conformance-incident-notifications"),
            ],
            artifacts / "v03-conformance-incident-notifications",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "bootstrap",
            [
                "cargo",
                "run",
                "--",
                "ail-bootstrap",
                "examples/ail_toolchain_agent.ail",
                "--pass",
                "examples/compiler_pass.ail",
                "--agent",
                "examples/ail_toolchain_agent.ail",
                "--target",
                "linux-x86_64-elf",
                "--artifact-dir",
                str(artifacts / "v03-bootstrap"),
            ],
            artifacts / "v03-bootstrap",
            "manifest.ail-bootstrap.txt",
            (
                "bootstrap-pass-composition-report.txt",
                "bootstrap-pass-composition-report.fingerprint.txt",
            ),
        ),
        AuditStep(
            "examples",
            [
                "cargo",
                "run",
                "--",
                "ail-examples",
                "examples",
                "--artifact-dir",
                str(artifacts / "v03-examples"),
                "--release-evidence",
            ],
            artifacts / "v03-examples",
            "manifest.ail-examples.txt",
            (
                "examples-report.txt",
                "examples-report.fingerprint.txt",
                "v03-roadmap.txt",
                "v03-roadmap.fingerprint.txt",
                "model-executor-manifest.txt",
                "model-executor-manifest.fingerprint.txt",
            ),
        ),
        AuditStep(
            "roadmap",
            [
                "cargo",
                "run",
                "--",
                "ail-v03-roadmap",
                "examples",
                "--artifact-dir",
                str(artifacts / "v03-roadmap"),
                "--release-evidence",
            ],
            artifacts / "v03-roadmap",
            "manifest.ail-examples.txt",
            (
                "examples-report.txt",
                "examples-report.fingerprint.txt",
                "v03-roadmap.txt",
                "v03-roadmap.fingerprint.txt",
            ),
        ),
        AuditStep(
            "roadmap-signal-status",
            [
                "python3",
                "scripts/run_v03_signal_status_audit.py",
                "--roadmap-file",
                str(artifacts / "v03-roadmap" / "v03-roadmap.txt"),
                "--status-file",
                "docs/ail/v03-roadmap-signal-status.md",
                "--output-dir",
                str(artifacts / "v03-roadmap-signal-status"),
                "--min-count",
                "5",
            ],
            artifacts / "v03-roadmap-signal-status",
            "manifest.v03-roadmap-signal-status.txt",
            (
                "v03-roadmap-signal-status.txt",
                "v03-roadmap-signal-status.fingerprint.txt",
            ),
        ),
    ]
    if include_live:
        steps.extend(
            [
                AuditStep(
                    "prompt-llm-review",
                    [
                        "python3",
                        "scripts/run_v03_prompt_llm_harness.py",
                        "--review-artifacts",
                        str(live_artifact_root / "prompt-llm"),
                    ],
                ),
                AuditStep(
                    "story-llm-review",
                    [
                        "python3",
                        "scripts/run_v03_story_llm_harness.py",
                        "--review-artifacts",
                        str(live_artifact_root / "story-llm"),
                    ],
                ),
                AuditStep(
                    "agent-policy-live-review",
                    [
                        "python3",
                        "scripts/run_v03_agent_policy_live_reviewer_harness.py",
                        "--review-artifacts",
                        str(live_artifact_root / "agent-policy-live-review"),
                    ],
                ),
            ]
        )
    return steps


def render_release_manifest(results: list[AuditResult], mode: str) -> str:
    lines = ["AIL-v0.3-Release-Audit-Manifest:", f"mode {mode}"]
    for result in results:
        command = " ".join(result.step.command)
        lines.append(f"step {result.step.name} command {command}")
        lines.append(f"step {result.step.name} status {result.status}")
        if result.returncode is not None:
            lines.append(f"step {result.step.name} exit-code {result.returncode}")
        if result.stdout_log is not None:
            lines.append(f"step {result.step.name} stdout-log {result.stdout_log}")
        if result.stderr_log is not None:
            lines.append(f"step {result.step.name} stderr-log {result.stderr_log}")
        if result.step.artifact_dir is not None:
            lines.append(f"artifact-dir {result.step.artifact_dir}")
            lines.append(f"artifact-manifest-name {result.step.manifest_name}")
        if result.status == "planned":
            for required_file in result.step.required_files:
                lines.append(f"artifact-required-file {required_file} planned")
        lines.extend(result.artifact_lines)
    return "\n".join(lines) + "\n"


def write_release_manifest(bundle_root: Path, results: list[AuditResult], mode: str) -> None:
    bundle_root.mkdir(parents=True, exist_ok=True)
    manifest = render_release_manifest(results, mode)
    (bundle_root / "release-audit-manifest.txt").write_text(manifest, encoding="utf-8")
    (bundle_root / "release-audit-manifest.fingerprint.txt").write_text(
        fnv64_fingerprint(manifest.encode("utf-8")) + "\n",
        encoding="utf-8",
    )


def run_plan(
    bundle_root: Path, dry_run: bool, include_live: bool, live_artifact_root: Path
) -> int:
    plan = build_v03_audit_plan(bundle_root, include_live, live_artifact_root)
    logs_dir = bundle_root / "logs"
    if dry_run:
        write_release_manifest(
            bundle_root,
            [AuditResult(step=step, status="planned") for step in plan],
            "dry-run",
        )
        return 0

    logs_dir.mkdir(parents=True, exist_ok=True)
    results: list[AuditResult] = []
    for step in plan:
        stdout_log = logs_dir / f"{step.name}.stdout.txt"
        stderr_log = logs_dir / f"{step.name}.stderr.txt"
        completed = subprocess.run(step.command, text=True, capture_output=True)
        stdout_log.write_text(completed.stdout, encoding="utf-8")
        stderr_log.write_text(completed.stderr, encoding="utf-8")
        status = "ok" if completed.returncode == 0 else "failed"
        artifact_lines: tuple[str, ...] = ()
        if completed.returncode == 0 and step.artifact_dir is not None:
            assert step.manifest_name is not None
            artifact_lines = tuple(
                verify_artifact_dir(
                    step.artifact_dir, step.manifest_name, step.required_files
                )
            )
        results.append(
            AuditResult(
                step=step,
                status=status,
                returncode=completed.returncode,
                stdout_log=stdout_log,
                stderr_log=stderr_log,
                artifact_lines=artifact_lines,
            )
        )
        write_release_manifest(bundle_root, results, "run")
        if completed.returncode != 0:
            return completed.returncode
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--bundle-root",
        type=Path,
        default=Path("/tmp/ail-v03-release-evidence"),
        help="directory that receives release audit logs, artifacts, and manifest",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="write the planned command manifest without executing commands",
    )
    parser.add_argument(
        "--include-live",
        action="store_true",
        help="include offline review steps for previously captured hosted LLM artifacts",
    )
    parser.add_argument(
        "--live-artifact-root",
        type=Path,
        default=Path("/tmp/ail-v03-release-live"),
        help="root containing prompt-llm, story-llm, and agent-policy-live-review artifacts",
    )
    args = parser.parse_args(argv)
    try:
        return run_plan(
            args.bundle_root, args.dry_run, args.include_live, args.live_artifact_root
        )
    except ValueError as error:
        print(f"release audit failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
