#!/usr/bin/env python3
"""Write a deterministic AIL v0.3 system-prompt harness run plan."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from run_v03_prompt_llm_harness import (
    DEFAULT_ENDPOINT,
    DEFAULT_PROMPT_DIR,
    REQUIRED_PROMPTS,
)


def fnv64(data: bytes) -> str:
    value = 0xCBF29CE484222325
    for byte in data:
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv64:{value:016x}"


def command_line(parts: list[str]) -> str:
    return " ".join(parts)


def prompt_inventory(prompt_dir: Path) -> list[dict[str, str]]:
    prompts: list[dict[str, str]] = []
    for name in REQUIRED_PROMPTS:
        path = prompt_dir / name
        if not path.is_file():
            raise SystemExit(f"missing prompt file {path}")
        data = path.read_bytes()
        prompts.append(
            {
                "path": str(path),
                "fingerprint": fnv64(data),
            }
        )
    return prompts


def build_commands(
    artifact_dir: Path, endpoint: str, skip_model_check: bool
) -> list[dict[str, list[str]]]:
    prompt_artifacts = artifact_dir / "prompt-llm"
    story_artifacts = artifact_dir / "story-llm"
    manual_artifacts = artifact_dir / "manual-live"
    examples_artifacts = artifact_dir / "examples"
    roadmap_artifacts = artifact_dir / "roadmap"
    commands = [
        {
            "label": "agent-contracts",
            "command": ["cargo", "run", "--", "ail-agent-contracts", "examples/agents"],
        },
        {
            "label": "prompt-dry-run",
            "command": [
                "python3",
                "scripts/run_v03_prompt_llm_harness.py",
                "--dry-run",
                "--endpoint",
                endpoint,
                "--artifact-dir",
                str(prompt_artifacts),
            ],
        },
        {
            "label": "prompt-live",
            "command": [
                "python3",
                "scripts/run_v03_prompt_llm_harness.py",
                "--endpoint",
                endpoint,
                "--artifact-dir",
                str(prompt_artifacts),
            ],
        },
        {
            "label": "prompt-review",
            "command": [
                "python3",
                "scripts/run_v03_prompt_llm_harness.py",
                "--review-artifacts",
                str(prompt_artifacts),
            ],
        },
        {
            "label": "story-review",
            "command": [
                "python3",
                "scripts/run_v03_story_llm_harness.py",
                "--review-artifacts",
                str(story_artifacts),
            ],
        },
        {
            "label": "manual-live-prompt",
            "command": [
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "prompt-interaction",
                "--run-checks",
                "--include-live",
                "--live-endpoint",
                endpoint,
                "--live-artifact-root",
                str(manual_artifacts),
            ],
        },
        {
            "label": "manual-live-story",
            "command": [
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "user-story-mode",
                "--run-checks",
                "--include-live",
                "--live-endpoint",
                endpoint,
                "--live-artifact-root",
                str(manual_artifacts),
            ],
        },
        {
            "label": "manual-live-authoring-gate",
            "command": [
                "python3",
                "scripts/run_ail_interactive_manual.py",
                "--chapter",
                "v03-authoring-gate",
                "--run-checks",
                "--include-live",
                "--live-endpoint",
                endpoint,
                "--live-artifact-root",
                str(manual_artifacts),
            ],
        },
        {
            "label": "examples-release",
            "command": [
                "cargo",
                "run",
                "--",
                "ail-examples",
                "examples",
                "--artifact-dir",
                str(examples_artifacts),
                "--release-evidence",
            ],
        },
        {
            "label": "roadmap",
            "command": [
                "cargo",
                "run",
                "--",
                "ail-v03-roadmap",
                "examples",
                "--artifact-dir",
                str(roadmap_artifacts),
                "--release-evidence",
            ],
        },
    ]
    if skip_model_check:
        for command in commands:
            if command["label"] in {"prompt-dry-run", "prompt-live"}:
                command["command"].append("--skip-model-check")
            elif command["label"] == "prompt-review":
                command["command"].append("--allow-skipped-model-check")
            elif command["label"].startswith("manual-live-"):
                command["command"].append("--skip-model-check")
    return commands


def render_text(plan: dict[str, object]) -> str:
    lines = [
        "AIL-v0.3-System-Prompt-Harness-Plan:",
        f"endpoint {plan['endpoint']}",
        f"artifact-dir {plan['artifact_dir']}",
        f"prompt-dir {plan['prompt_dir']}",
        f"prompt-count {plan['prompt_count']}",
        f"model-check-policy {plan['model_check_policy']}",
        f"reviewer-handoff {plan['reviewer_handoff']}",
        f"promotion-policy {plan['promotion_policy']}",
    ]
    for prompt in plan["prompts"]:
        lines.append(f"prompt-file {prompt['path']} {prompt['fingerprint']}")
    for command in plan["commands"]:
        lines.append(f"command {command['label']} {command_line(command['command'])}")
    lines.extend(
        [
            "required-evidence model-check present",
            "required-evidence model-check-model-id",
            "required-evidence prompt-envelope-valid-count",
            "required-evidence prompt-envelope-invalid-count",
            "required-evidence story-prompt-envelope-valid-count",
            "required-evidence agent-trace.fingerprint.txt",
            "required-evidence examples-report.txt",
            "required-evidence v03-roadmap.txt",
        ]
    )
    return "\n".join(lines) + "\n"


def write_plan(args: argparse.Namespace) -> str:
    artifact_dir = Path(args.artifact_dir)
    prompt_dir = Path(args.prompt_dir)
    prompts = prompt_inventory(prompt_dir)
    plan = {
        "artifact_kind": "AIL-v0.3-System-Prompt-Harness-Plan",
        "endpoint": args.endpoint,
        "artifact_dir": str(artifact_dir),
        "prompt_dir": str(prompt_dir),
        "prompt_count": len(prompts),
        "model_check_policy": (
            "skipped-local-only" if args.skip_model_check else "required-hosted-model-list"
        ),
        "skip_model_check": args.skip_model_check,
        "reviewer_handoff": "examples/agents/skills/ail-prompt-interaction-reviewer/SKILL.md",
        "promotion_policy": "do-not-promote-generated-content",
        "prompts": prompts,
        "commands": build_commands(artifact_dir, args.endpoint, args.skip_model_check),
    }
    text = render_text(plan)
    json_text = json.dumps(plan, indent=2, sort_keys=True) + "\n"
    text_fingerprint = fnv64(text.encode("utf-8"))
    json_fingerprint = fnv64(json_text.encode("utf-8"))
    manifest = "\n".join(
        [
            "AIL-v0.3-System-Prompt-Harness-Plan-Manifest:",
            f"plan system-prompt-harness-plan.txt {text_fingerprint}",
            f"json system-prompt-harness-plan.json {json_fingerprint}",
            "fingerprint system-prompt-harness-plan.fingerprint.txt",
            f"prompt-count {len(prompts)}",
            f"model-check-policy {plan['model_check_policy']}",
        ]
    ) + "\n"
    artifact_dir.mkdir(parents=True, exist_ok=True)
    (artifact_dir / "system-prompt-harness-plan.txt").write_text(
        text, encoding="utf-8"
    )
    (artifact_dir / "system-prompt-harness-plan.json").write_text(
        json_text, encoding="utf-8"
    )
    (artifact_dir / "system-prompt-harness-plan.fingerprint.txt").write_text(
        text_fingerprint + "\n", encoding="utf-8"
    )
    (artifact_dir / "manifest.v03-system-prompt-harness-plan.txt").write_text(
        manifest, encoding="utf-8"
    )
    (artifact_dir / "manifest.fingerprint.txt").write_text(
        fnv64(manifest.encode("utf-8")) + "\n", encoding="utf-8"
    )
    return text


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--endpoint", default=DEFAULT_ENDPOINT)
    parser.add_argument("--prompt-dir", default=DEFAULT_PROMPT_DIR)
    parser.add_argument("--artifact-dir", default="/tmp/ail-v03-system-prompt-harness-plan")
    parser.add_argument(
        "--skip-model-check",
        action="store_true",
        help="Plan a local fake-server run; hosted evidence must not use this flag.",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    text = write_plan(parse_args(argv))
    print(text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
