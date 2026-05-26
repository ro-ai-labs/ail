#!/usr/bin/env python3
"""Run and bundle the AIL v0.2 release audit command set."""

from __future__ import annotations

import argparse
import dataclasses
import subprocess
import sys
from pathlib import Path


@dataclasses.dataclass(frozen=True)
class AuditStep:
    name: str
    command: list[str]
    artifact_dir: Path | None = None
    manifest_name: str | None = None
    required_files: tuple[str, ...] = ()


@dataclasses.dataclass(frozen=True)
class AuditResult:
    step: AuditStep
    status: str
    returncode: int | None = None
    stdout_log: Path | None = None
    stderr_log: Path | None = None
    artifact_lines: tuple[str, ...] = ()


def fnv64_fingerprint(data: bytes) -> str:
    value = 0xCBF29CE484222325
    for byte in data:
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv64:{value:016x}"


def build_v02_audit_plan(bundle_root: Path) -> list[AuditStep]:
    artifacts = bundle_root / "artifacts"
    build_support = artifacts / "v02-build-support"
    close_ticket = artifacts / "v02-close-ticket"
    return [
        AuditStep("cargo-fmt", ["cargo", "fmt", "--check"]),
        AuditStep("git-diff-check", ["git", "diff", "--check"]),
        AuditStep("cargo-check", ["cargo", "check"]),
        AuditStep("cargo-test", ["cargo", "test"]),
        AuditStep("cargo-clippy", ["cargo", "clippy", "--all-targets", "--", "-D", "warnings"]),
        AuditStep(
            "conformance-support",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/support_ticket.ail",
                "--artifact-dir",
                str(artifacts / "v02-conformance-support"),
            ],
            artifacts / "v02-conformance-support",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "conformance-refund",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/refund_tool.ail",
                "--artifact-dir",
                str(artifacts / "v02-conformance-refund"),
            ],
            artifacts / "v02-conformance-refund",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "conformance-compiler",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/compiler_pass.ail",
                "--artifact-dir",
                str(artifacts / "v02-conformance-compiler"),
            ],
            artifacts / "v02-conformance-compiler",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "conformance-system",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/network_driver.ail",
                "--artifact-dir",
                str(artifacts / "v02-conformance-system"),
            ],
            artifacts / "v02-conformance-system",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "conformance-std-collections",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/ail_std_collections.ail",
                "--artifact-dir",
                str(artifacts / "v02-conformance-std-collections"),
            ],
            artifacts / "v02-conformance-std-collections",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "conformance-c-interop",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/c_interop.ail",
                "--artifact-dir",
                str(artifacts / "v02-conformance-c-interop"),
            ],
            artifacts / "v02-conformance-c-interop",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "conformance-ui",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/ui_workflow.ail",
                "--artifact-dir",
                str(artifacts / "v02-conformance-ui"),
            ],
            artifacts / "v02-conformance-ui",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "conformance-incident-response",
            [
                "cargo",
                "run",
                "--",
                "ail-conformance",
                "examples/incident_response.ail",
                "--artifact-dir",
                str(artifacts / "v02-conformance-incident-response"),
            ],
            artifacts / "v02-conformance-incident-response",
            "manifest.ail-conformance.txt",
        ),
        AuditStep(
            "build-support",
            [
                "cargo",
                "run",
                "--",
                "ail-build",
                "examples/support_ticket.ail",
                "--spec-file",
                "examples/support_ticket.ail/spec.ail-spec.md",
                "--agent",
                "examples/ail_toolchain_agent.ail",
                "--artifact-dir",
                str(build_support),
                "--target",
                "linux-x86_64-elf",
                "--action",
                "CloseTicket",
                "--out",
                str(close_ticket),
            ],
            build_support,
            "manifest.ail-build.txt",
        ),
        AuditStep(
            "wasm-host-contract",
            [
                "cargo",
                "run",
                "--",
                "ail-compile",
                "examples/c_interop.ail",
                "--target",
                "wasm32-unknown-sandbox-wasm",
                "--all-actions",
                "--agent",
                "examples/ail_toolchain_agent.ail",
                "--artifact-dir",
                str(artifacts / "v02-wasm-host-contract"),
            ],
            artifacts / "v02-wasm-host-contract",
            "manifest.ail-compile.txt",
        ),
        AuditStep(
            "darwin-contract",
            [
                "cargo",
                "run",
                "--",
                "ail-compile",
                "examples/support_ticket.ail",
                "--target",
                "aarch64-apple-darwin-libsystem-macho",
                "--action",
                "CloseTicket",
                "--artifact-dir",
                str(artifacts / "v02-darwin-contract"),
            ],
            artifacts / "v02-darwin-contract",
            "manifest.ail-compile.txt",
        ),
        AuditStep(
            "spec-roundtrip",
            [
                "cargo",
                "run",
                "--",
                "ail-spec",
                "--core-file",
                str(build_support / "checked.ail-core.txt"),
                "--artifact-dir",
                str(artifacts / "v02-spec-roundtrip"),
            ],
            artifacts / "v02-spec-roundtrip",
            "manifest.ail-spec.txt",
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
                str(artifacts / "v02-bootstrap"),
            ],
            artifacts / "v02-bootstrap",
            "manifest.ail-bootstrap.txt",
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
                str(artifacts / "v02-examples"),
                "--release-evidence",
            ],
            artifacts / "v02-examples",
            "manifest.ail-examples.txt",
            ("model-executor-manifest.txt", "model-executor-manifest.fingerprint.txt"),
        ),
    ]


def verify_artifact_dir(
    artifact_dir: Path, manifest_name: str, required_files: tuple[str, ...] = ()
) -> list[str]:
    manifest_path = artifact_dir / manifest_name
    fingerprint_path = artifact_dir / "manifest.fingerprint.txt"
    if not artifact_dir.is_dir():
        raise ValueError(f"artifact dir is missing: {artifact_dir}")
    if not manifest_path.is_file():
        raise ValueError(f"artifact manifest is missing: {manifest_path}")
    if not fingerprint_path.is_file():
        raise ValueError(f"artifact manifest fingerprint is missing: {fingerprint_path}")
    manifest_bytes = manifest_path.read_bytes()
    expected = fnv64_fingerprint(manifest_bytes)
    actual = fingerprint_path.read_text(encoding="utf-8").strip()
    if actual != expected:
        raise ValueError(
            f"manifest fingerprint mismatch for {manifest_path}: expected {expected}, found {actual}"
        )
    lines = [
        f"artifact-dir {artifact_dir}",
        f"artifact-manifest {manifest_name} {expected}",
        "artifact-manifest-fingerprint manifest.fingerprint.txt ok",
    ]
    for required_file in required_files:
        required_path = artifact_dir / required_file
        if not required_path.is_file():
            raise ValueError(f"required artifact file is missing: {required_path}")
        lines.append(f"artifact-required-file {required_file} ok")
        if required_file.endswith(".fingerprint.txt"):
            artifact_name = required_file.removesuffix(".fingerprint.txt") + ".txt"
            artifact_path = artifact_dir / artifact_name
            if artifact_path.is_file():
                expected_required = fnv64_fingerprint(artifact_path.read_bytes())
                actual_required = required_path.read_text(encoding="utf-8").strip()
                if actual_required != expected_required:
                    raise ValueError(
                        f"required artifact fingerprint mismatch for {required_path}: expected {expected_required}, found {actual_required}"
                    )
                lines.append(f"artifact-required-fingerprint {required_file} ok")
    return lines


def render_release_manifest(results: list[AuditResult], mode: str) -> str:
    lines = ["AIL-v0.2-Release-Audit-Manifest:", f"mode {mode}"]
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


def run_plan(bundle_root: Path, dry_run: bool) -> int:
    plan = build_v02_audit_plan(bundle_root)
    logs_dir = bundle_root / "logs"
    results: list[AuditResult] = []
    if dry_run:
        results = [AuditResult(step=step, status="planned") for step in plan]
        write_release_manifest(bundle_root, results, "dry-run")
        return 0

    logs_dir.mkdir(parents=True, exist_ok=True)
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
        result = AuditResult(
            step=step,
            status=status,
            returncode=completed.returncode,
            stdout_log=stdout_log,
            stderr_log=stderr_log,
            artifact_lines=artifact_lines,
        )
        results.append(result)
        write_release_manifest(bundle_root, results, "run")
        if completed.returncode != 0:
            return completed.returncode
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--bundle-root",
        type=Path,
        default=Path("/tmp/ail-v02-release-evidence"),
        help="directory that receives release audit logs, artifacts, and manifest",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="write the planned command manifest without executing commands",
    )
    args = parser.parse_args(argv)
    try:
        return run_plan(args.bundle_root, args.dry_run)
    except ValueError as error:
        print(f"release audit failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
