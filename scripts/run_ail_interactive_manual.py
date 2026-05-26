#!/usr/bin/env python3
"""Run or print the AIL interactive manual chapters.

The manual runner is deterministic by default. It lists chapters, prints the
commands a reader can run, and only executes local verification commands when
`--run-checks` is supplied. Live LLM commands stay opt-in through
`--include-live`.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from dataclasses import dataclass


@dataclass(frozen=True)
class ManualCommand:
    label: str
    command: tuple[str, ...]
    live: bool = False
    evidence: tuple[str, ...] = ()

    def shell_line(self) -> str:
        return " ".join(self.command)


@dataclass(frozen=True)
class ManualChapter:
    chapter_id: str
    title: str
    doc: str
    purpose: str
    commands: tuple[ManualCommand, ...]


CHAPTERS: tuple[ManualChapter, ...] = (
    ManualChapter(
        chapter_id="user-story-mode",
        title="User Story Mode",
        doc="docs/ail/manual/01-user-story-mode.md",
        purpose="Start from a story file, involve the toolchain agent, and produce checked artifacts.",
        commands=(
            ManualCommand(
                label="show-story-mode-live-command",
                command=("python3", "scripts/run_v03_story_llm_harness.py", "--dry-run"),
            ),
            ManualCommand(
                label="verify-story-mode-local",
                command=(
                    "cargo",
                    "test",
                    "cli_ail_story_builds_checked_artifacts_from_story_file",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=(
                    "story-mode-report.txt",
                    "manifest.ail-story.txt",
                ),
            ),
            ManualCommand(
                label="verify-story-agent-entrypoint-local",
                command=(
                    "cargo",
                    "test",
                    "cli_ail_story_agent_records_story_entrypoint_before_compile",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=(
                    "agent-trace.txt",
                    "manifest.ail-story.txt",
                ),
            ),
            ManualCommand(
                label="run-story-mode-live",
                command=("python3", "scripts/run_v03_story_llm_harness.py"),
                live=True,
            ),
            ManualCommand(
                label="direct-ail-story-live",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-story",
                    "examples/support_ticket.ail",
                    "--story-file",
                    "examples/stories/example-30.md",
                    "--agent",
                    "examples/ail_toolchain_agent.ail",
                    "--artifact-dir",
                    "/tmp/ail-user-story-mode",
                    "--llm-endpoint",
                    "http://inteligentia-pro-1:8080/v1/chat/completions",
                ),
                live=True,
            ),
        ),
    ),
    ManualChapter(
        chapter_id="examples-release",
        title="Examples Release Replay",
        doc="examples/README.md",
        purpose="Replay the full examples catalog and inspect learning evidence.",
        commands=(
            ManualCommand(
                label="replay-examples-release",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-examples",
                    "examples",
                    "--artifact-dir",
                    "/tmp/ail-manual-examples",
                    "--release-evidence",
                ),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="prompt-interaction",
        title="Prompt Interaction",
        doc="docs/ail/prompts/README.md",
        purpose="Inspect prompt-pack surfaces and stored request/response replay.",
        commands=(
            ManualCommand(
                label="list-prompts",
                command=("rg", "--files", "docs/ail/prompts"),
            ),
            ManualCommand(
                label="run-prompt-corpus",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-prompt-corpus",
                    "docs/ail/corpus/prompts",
                    "--artifact-dir",
                    "/tmp/ail-manual-prompt-corpus",
                ),
                evidence=(
                    "prompt-corpus-portability.txt",
                    "manifest.ail-prompt-corpus.txt",
                ),
            ),
            ManualCommand(
                label="replay-examples-prompt-surfaces",
                command=(
                    "cargo",
                    "run",
                    "--",
                    "ail-examples",
                    "examples",
                    "--artifact-dir",
                    "/tmp/ail-manual-prompt-examples",
                    "--release-evidence",
                ),
                evidence=("AIL-Examples-Report", "prompt-count"),
            ),
            ManualCommand(
                label="inspect-capture-help",
                command=("python3", "scripts/capture_example_transcripts.py", "--help"),
            ),
        ),
    ),
    ManualChapter(
        chapter_id="agent-entrypoint",
        title="Agent Entrypoint",
        doc="examples/agents/README.md",
        purpose="Inspect Codex agent roles and the AIL toolchain-agent package.",
        commands=(
            ManualCommand(
                label="show-agent-guides",
                command=("rg", "--files", "examples/agents"),
            ),
            ManualCommand(
                label="check-toolchain-agent",
                command=("cargo", "run", "--", "ail-check", "examples/ail_toolchain_agent.ail"),
            ),
            ManualCommand(
                label="verify-toolchain-agent-package",
                command=(
                    "cargo",
                    "test",
                    "ail_toolchain_agent_package_lowers_to_verified_bytecode",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=("agent.ailbc.json",),
            ),
            ManualCommand(
                label="verify-agent-build-entrypoint",
                command=(
                    "cargo",
                    "test",
                    "cli_ail_build_runs_toolchain_agent_bytecode",
                    "--test",
                    "ail_toolchain",
                ),
                evidence=(
                    "agent.ailbc.json",
                    "agent-trace.txt",
                ),
            ),
        ),
    ),
)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="List, print, or run deterministic AIL interactive manual chapters."
    )
    parser.add_argument("--list", action="store_true", help="List manual chapters")
    parser.add_argument("--chapter", help="Chapter id to print or run")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print chapter commands without executing them",
    )
    parser.add_argument(
        "--run-checks",
        action="store_true",
        help="Run deterministic non-live commands for the selected chapter",
    )
    parser.add_argument(
        "--include-live",
        action="store_true",
        help="Include live LLM/network commands when printing or running a chapter",
    )
    return parser.parse_args(argv)


def chapter_by_id(chapter_id: str) -> ManualChapter:
    for chapter in CHAPTERS:
        if chapter.chapter_id == chapter_id:
            return chapter
    valid = ", ".join(chapter.chapter_id for chapter in CHAPTERS)
    raise SystemExit(f"unknown chapter {chapter_id}; valid chapters: {valid}")


def print_chapter_list() -> None:
    print("AIL-Interactive-Manual:")
    for chapter in CHAPTERS:
        print(f"chapter {chapter.chapter_id} {chapter.title}")
        print(f"doc {chapter.doc}")
        print(f"purpose {chapter.purpose}")


def chapter_commands(chapter: ManualChapter, include_live: bool) -> list[ManualCommand]:
    return [
        command for command in chapter.commands if include_live or not command.live
    ]


def print_chapter(chapter: ManualChapter, include_live: bool) -> None:
    print("AIL-Interactive-Manual-Chapter:")
    print(f"id {chapter.chapter_id}")
    print(f"title {chapter.title}")
    print(f"doc {chapter.doc}")
    print(f"purpose {chapter.purpose}")
    for index, command in enumerate(chapter_commands(chapter, include_live), start=1):
        print(f"step {index} {command.label}")
        print(f"live {str(command.live).lower()}")
        print(command.shell_line())
        for evidence in command.evidence:
            print(f"evidence {evidence}")


def run_chapter_checks(chapter: ManualChapter, include_live: bool) -> int:
    commands = chapter_commands(chapter, include_live)
    if not commands:
        print(f"chapter {chapter.chapter_id} has no runnable commands")
        return 0
    for command in commands:
        print(f"running {command.label}: {command.shell_line()}")
        completed = subprocess.run(command.command, check=False)
        if completed.returncode != 0:
            return completed.returncode
    return 0


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.list or not args.chapter:
        print_chapter_list()
        if not args.chapter:
            return 0
    chapter = chapter_by_id(args.chapter)
    if args.run_checks:
        return run_chapter_checks(chapter, args.include_live)
    print_chapter(chapter, args.include_live or args.dry_run)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
