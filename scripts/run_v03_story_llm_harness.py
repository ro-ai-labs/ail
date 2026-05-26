#!/usr/bin/env python3
"""Run the AIL v0.3 story-mode live LLM harness.

This script is intentionally not part of the default test suite. It contacts a
llama.cpp-compatible server and then runs `ail-story` through Cargo.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import urllib.request


DEFAULT_ENDPOINT = "http://inteligentia-pro-1:8080/"
DEFAULT_PACKAGE = "examples/support_ticket.ail"
DEFAULT_STORY_FILE = "examples/stories/example-30.md"
DEFAULT_AGENT = "examples/ail_toolchain_agent.ail"
DEFAULT_ARTIFACT_DIR = "/tmp/ail-v03-story-llm"


def endpoint_join(endpoint: str, suffix: str) -> str:
    return endpoint.rstrip("/") + "/" + suffix.lstrip("/")


def build_ail_story_command(args: argparse.Namespace) -> list[str]:
    command = [
        "cargo",
        "run",
        "--",
        "ail-story",
        args.package,
        "--story-file",
        args.story_file,
        "--artifact-dir",
        args.artifact_dir,
        "--llm-endpoint",
        args.endpoint,
    ]
    if args.agent:
        command.extend(["--agent", args.agent])
    if args.target or args.action or args.out:
        if not (args.target and args.action and args.out):
            raise SystemExit("--target, --action, and --out must be supplied together")
        command.extend(["--target", args.target, "--action", args.action, "--out", args.out])
    return command


def check_models(endpoint: str) -> None:
    models_url = endpoint_join(endpoint, "/v1/models")
    with urllib.request.urlopen(models_url, timeout=10) as response:
        body = response.read().decode("utf-8", errors="replace")
    print(f"models endpoint: {models_url}")
    print(body)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run ail-story against the v0.3 live llama.cpp endpoint. "
            "Use --dry-run to print commands without network access."
        ),
        epilog=f"Default live endpoint: {DEFAULT_ENDPOINT}",
    )
    parser.add_argument(
        "--endpoint",
        default=DEFAULT_ENDPOINT,
        help=f"LLM endpoint (default: {DEFAULT_ENDPOINT})",
    )
    parser.add_argument(
        "--package",
        default=DEFAULT_PACKAGE,
        help=f"AIL package (default: {DEFAULT_PACKAGE})",
    )
    parser.add_argument(
        "--story-file",
        default=DEFAULT_STORY_FILE,
        help=f"Story file passed to ail-story (default: {DEFAULT_STORY_FILE})",
    )
    parser.add_argument(
        "--agent",
        default=DEFAULT_AGENT,
        help=f"Toolchain agent package or bytecode (default: {DEFAULT_AGENT})",
    )
    parser.add_argument(
        "--artifact-dir",
        default=DEFAULT_ARTIFACT_DIR,
        help=f"Artifact directory (default: {DEFAULT_ARTIFACT_DIR})",
    )
    parser.add_argument(
        "--target",
        default=None,
        help="Optional native target, such as linux-x86_64-elf",
    )
    parser.add_argument(
        "--action",
        default=None,
        help="Required with --target; action to compile",
    )
    parser.add_argument("--out", default=None, help="Required with --target; native output path")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print the model check and ail-story commands without running them",
    )
    parser.add_argument(
        "--skip-model-check",
        action="store_true",
        help="Skip the /v1/models check before running ail-story",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    command = build_ail_story_command(args)
    if args.dry_run:
        print("model check:")
        print("curl -sS " + endpoint_join(args.endpoint, "/v1/models"))
        print("ail-story:")
        print(" ".join(command))
        print("artifacts:")
        print(args.artifact_dir)
        return 0
    if not args.skip_model_check:
        check_models(args.endpoint)
    completed = subprocess.run(command, check=False)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
