#!/usr/bin/env python3
"""Bundle deterministic Systems profile runtime and migration evidence."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from run_v02_release_audit import fnv64_fingerprint


ROOT = Path(__file__).resolve().parents[1]
PACKAGE = "examples/network_driver.ail"
SYSTEMS_SIGNAL = (
    "Systems profile needs unsupported-target migration guidance and broader "
    "transmit/interrupt runtime variants."
)
UNSUPPORTED_TARGET_GUIDANCE = (
    "move linux-only syscall effects behind target-support metadata or choose "
    "linux-x86_64-elf"
)


@dataclass(frozen=True)
class RuntimeVariant:
    label: str
    action: str
    spec_file: str | None
    trace_event: str
    output_name: str


VARIANTS: tuple[RuntimeVariant, ...] = (
    RuntimeVariant(
        label="receive",
        action="NetworkPacketReceiver",
        spec_file=None,
        trace_event="PacketReceived",
        output_name="network-packet-receiver.elf",
    ),
    RuntimeVariant(
        label="transmit",
        action="NetworkPacketTransmitter",
        spec_file=f"{PACKAGE}/examples/accepted/packet-transmit-minimal.ail-spec.md",
        trace_event="PacketTransmitted",
        output_name="network-packet-transmitter.elf",
    ),
    RuntimeVariant(
        label="interrupt-handler",
        action="TimerInterruptHandler",
        spec_file=f"{PACKAGE}/examples/accepted/interrupt-context-minimal.ail-spec.md",
        trace_event="TimerInterruptHandled",
        output_name="timer-interrupt-handler.elf",
    ),
)


def fnv64_text(text: str) -> str:
    return fnv64_fingerprint(text.encode("utf-8"))


def run_command(command: list[str]) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise SystemExit(
            "command failed: "
            + " ".join(command)
            + "\nstdout:\n"
            + completed.stdout
            + "\nstderr:\n"
            + completed.stderr
        )
    return completed


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def require_line(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"{label} missing {needle}")


def verify_unsupported_target_entry() -> str:
    catalog = read_text(ROOT / "examples" / "examples.md")
    start = catalog.find("## Example: example-104")
    if start == -1:
        raise SystemExit("examples catalog missing example-104")
    end = catalog.find("## Example:", start + 1)
    entry = catalog[start:] if end == -1 else catalog[start:end]
    for required in [
        "semantic-task: system-linux-syscall-darwin-unsupported-104",
        "package: examples/darwin_linux_effect.ail",
        "checker-result: rejected",
        "target: aarch64-apple-darwin-libsystem-macho",
        "expected-diagnostic: AIL-BACKEND-001",
        "failure-taxonomy: unsupported-target",
    ]:
        require_line(entry, required, "unsupported target catalog entry")
    return fnv64_text(entry)


def write_text_with_fingerprint(path: Path, text: str) -> str:
    path.write_text(text, encoding="utf-8")
    fingerprint = fnv64_text(text)
    path.with_name(path.name.removesuffix(".txt") + ".fingerprint.txt").write_text(
        fingerprint + "\n",
        encoding="utf-8",
    )
    return fingerprint


def run_conformance(artifact_dir: Path) -> str:
    conformance_dir = artifact_dir / "conformance"
    run_command(
        [
            "cargo",
            "run",
            "--quiet",
            "--",
            "ail-conformance",
            PACKAGE,
            "--artifact-dir",
            str(conformance_dir),
        ]
    )
    report = read_text(conformance_dir / "conformance-report.txt")
    for required in [
        "accepted: packet-receive-minimal.ail-spec.md",
        "accepted: packet-transmit-minimal.ail-spec.md",
        "accepted: interrupt-context-minimal.ail-spec.md",
        "rejected: interrupt-context-blocking-effect.ail-spec.md AIL033",
        "rejected: scheduler-task-unknown-context.ail-spec.md AIL035",
        "rejected: interrupt-mask-unknown-context.ail-spec.md AIL040",
    ]:
        require_line(report, required, "conformance report")
    expected = fnv64_text(report)
    actual = read_text(conformance_dir / "conformance-report.fingerprint.txt").strip()
    if actual != expected:
        raise SystemExit(
            f"conformance report fingerprint mismatch: expected {expected} got {actual}"
        )
    return expected


def compile_variant(artifact_dir: Path, variant: RuntimeVariant) -> tuple[str, str]:
    variant_dir = artifact_dir / variant.label
    output_path = artifact_dir / variant.output_name
    command = [
        "cargo",
        "run",
        "--quiet",
        "--",
        "ail-compile",
        PACKAGE,
        "--action",
        variant.action,
        "--target",
        "linux-x86_64-elf",
        "--out",
        str(output_path),
        "--artifact-dir",
        str(variant_dir),
    ]
    if variant.spec_file is not None:
        command[6:6] = ["--spec-file", variant.spec_file]
    run_command(command)
    manifest = read_text(variant_dir / "manifest.ail-compile.txt")
    require_line(
        manifest,
        "machine-bytecode-contract linux-x86_64-elf",
        f"{variant.label} compile manifest",
    )
    manifest_fingerprint = fnv64_text(manifest)
    actual_manifest_fingerprint = read_text(variant_dir / "manifest.fingerprint.txt").strip()
    if actual_manifest_fingerprint != manifest_fingerprint:
        raise SystemExit(
            f"{variant.label} manifest fingerprint mismatch: "
            f"expected {manifest_fingerprint} got {actual_manifest_fingerprint}"
        )

    completed = run_command([str(output_path)])
    trace = completed.stdout + completed.stderr
    require_line(trace, f"system component ", f"{variant.label} runtime trace")
    require_line(trace, f"trace {variant.trace_event}", f"{variant.label} runtime trace")
    trace_fingerprint = write_text_with_fingerprint(
        artifact_dir / f"{variant.label}-runtime-trace.txt",
        trace,
    )
    return manifest_fingerprint, trace_fingerprint


def build_report(
    artifact_dir: Path,
    conformance_fingerprint: str,
    unsupported_target_fingerprint: str,
    compile_fingerprints: dict[str, str],
    trace_fingerprints: dict[str, str],
) -> str:
    lines = [
        "AIL-v0.3-Systems-Profile-Audit:",
        f"package {PACKAGE}",
        f"v03-roadmap-signal {SYSTEMS_SIGNAL}",
        f"conformance conformance/conformance-report.txt {conformance_fingerprint}",
        "accepted-fixture packet-receive-minimal.ail-spec.md",
        "accepted-fixture packet-transmit-minimal.ail-spec.md",
        "accepted-fixture interrupt-context-minimal.ail-spec.md",
        "rejected-fixture interrupt-context-blocking-effect.ail-spec.md AIL033",
        "rejected-fixture scheduler-task-unknown-context.ail-spec.md AIL035",
        "rejected-fixture interrupt-mask-unknown-context.ail-spec.md AIL040",
        "unsupported-target-migration example-104 AIL-BACKEND-001 "
        "aarch64-apple-darwin-libsystem-macho",
        f"unsupported-target-entry-fingerprint {unsupported_target_fingerprint}",
        f"unsupported-target-guidance {UNSUPPORTED_TARGET_GUIDANCE}",
    ]
    for variant in VARIANTS:
        lines.extend(
            [
                "runtime-variant "
                f"{variant.label} action {variant.action} target linux-x86_64-elf "
                f"trace {variant.trace_event}",
                "runtime-variant-compile-manifest "
                f"{variant.label} {variant.label}/manifest.ail-compile.txt "
                f"{compile_fingerprints[variant.label]}",
                "runtime-variant-trace "
                f"{variant.label} {variant.label}-runtime-trace.txt "
                f"{trace_fingerprints[variant.label]}",
            ]
        )
    lines.extend(
        [
            "transmit-runtime-variant true",
            "interrupt-handler-runtime-variant true",
            f"artifact-dir {artifact_dir}",
            "audit-result accepted",
            "",
        ]
    )
    return "\n".join(lines)


def write_manifest(
    artifact_dir: Path,
    report_fingerprint: str,
    conformance_fingerprint: str,
    compile_fingerprints: dict[str, str],
    trace_fingerprints: dict[str, str],
) -> None:
    lines = [
        "AIL-v0.3-Systems-Profile-Audit-Manifest:",
        f"report systems-profile-audit-report.txt {report_fingerprint}",
        f"conformance conformance/conformance-report.txt {conformance_fingerprint}",
    ]
    for variant in VARIANTS:
        lines.extend(
            [
                f"compile-manifest {variant.label}/manifest.ail-compile.txt "
                f"{compile_fingerprints[variant.label]}",
                f"runtime-trace {variant.label}-runtime-trace.txt "
                f"{trace_fingerprints[variant.label]}",
            ]
        )
    lines.extend(["audit-result accepted", ""])
    manifest = "\n".join(lines)
    (artifact_dir / "manifest.v03-systems-profile-audit.txt").write_text(
        manifest,
        encoding="utf-8",
    )
    (artifact_dir / "manifest.fingerprint.txt").write_text(
        fnv64_text(manifest) + "\n",
        encoding="utf-8",
    )


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--artifact-dir",
        default="/tmp/ail-v03-systems-profile-audit",
        type=Path,
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    artifact_dir = args.artifact_dir
    if artifact_dir.exists():
        shutil.rmtree(artifact_dir)
    artifact_dir.mkdir(parents=True)

    conformance_fingerprint = run_conformance(artifact_dir)
    unsupported_target_fingerprint = verify_unsupported_target_entry()
    compile_fingerprints: dict[str, str] = {}
    trace_fingerprints: dict[str, str] = {}
    for variant in VARIANTS:
        compile_fingerprint, trace_fingerprint = compile_variant(artifact_dir, variant)
        compile_fingerprints[variant.label] = compile_fingerprint
        trace_fingerprints[variant.label] = trace_fingerprint

    report = build_report(
        artifact_dir,
        conformance_fingerprint,
        unsupported_target_fingerprint,
        compile_fingerprints,
        trace_fingerprints,
    )
    report_fingerprint = write_text_with_fingerprint(
        artifact_dir / "systems-profile-audit-report.txt",
        report,
    )
    write_manifest(
        artifact_dir,
        report_fingerprint,
        conformance_fingerprint,
        compile_fingerprints,
        trace_fingerprints,
    )
    print(report, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
