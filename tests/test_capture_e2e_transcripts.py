import json
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


REQUIRED_PROMPTS = (
    "interview.system.md",
    "requirements.system.md",
    "spec-draft.system.md",
    "core-draft.system.md",
    "repair.system.md",
    "diagnostic-repair.system.md",
    "core-to-spec.system.md",
    "core-to-summary.system.md",
    "flow-patch.system.md",
    "trace-debug.system.md",
    "interop.system.md",
)


def fnv64(text):
    value = 0xCBF29CE484222325
    for byte in text.encode():
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"fnv64:{value:016x}"


def write_text(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def write_prompt_llm_review_fixture(artifact_dir, empty_content_for=None):
    artifact_dir.mkdir(parents=True, exist_ok=True)
    models_text = (
        json.dumps({"object": "list", "data": [{"id": "test-model"}]}, sort_keys=True)
        + "\n"
    )
    write_text(artifact_dir / "models.json", models_text)
    write_text(artifact_dir / "models.fingerprint.txt", fnv64(models_text) + "\n")
    report_lines = [
        "AIL-Prompt-LLM-Harness:",
        "endpoint http://127.0.0.1:8080/v1/chat/completions",
        "models-url http://127.0.0.1:8080/v1/models",
        f"prompt-count {len(REQUIRED_PROMPTS)}",
    ]
    manifest_lines = [
        "AIL-Prompt-LLM-Harness-Manifest:",
        "artifact models models.json models.fingerprint.txt",
    ]
    for prompt_name in REQUIRED_PROMPTS:
        prompt_path = ROOT / "docs" / "ail" / "prompts" / prompt_name
        prompt_rel = f"docs/ail/prompts/{prompt_name}"
        prompt_text = prompt_path.read_text()
        stem = prompt_name.removesuffix(".system.md")
        content = "" if empty_content_for == prompt_name else f"Blocking questions for {stem}.\n"
        response = {"choices": [{"message": {"content": content.strip()}}], "model": "test-model"}
        request_text = json.dumps(
            {
                "endpoint": "http://127.0.0.1:8080/v1/chat/completions",
                "method": "POST",
                "prompt_file": prompt_rel,
                "prompt_fingerprint": fnv64(prompt_text),
                "body": {
                    "messages": [
                        {"role": "system", "content": prompt_text},
                        {"role": "user", "content": "AIL prompt-pack live probe."},
                    ],
                    "max_tokens": 64,
                    "temperature": 0.0,
                    "stream": False,
                },
            },
            indent=2,
            sort_keys=True,
        ) + "\n"
        response_text = json.dumps(response, indent=2, sort_keys=True) + "\n"
        content_text = content
        write_text(artifact_dir / "requests" / f"{stem}.json", request_text)
        write_text(
            artifact_dir / "requests" / f"{stem}.fingerprint.txt",
            fnv64(request_text) + "\n",
        )
        write_text(artifact_dir / "responses" / f"{stem}.json", response_text)
        write_text(
            artifact_dir / "responses" / f"{stem}.fingerprint.txt",
            fnv64(response_text) + "\n",
        )
        write_text(artifact_dir / "content" / f"{stem}.txt", content_text)
        write_text(
            artifact_dir / "content" / f"{stem}.fingerprint.txt",
            fnv64(content_text) + "\n",
        )
        report_lines.append(
            f"prompt {prompt_rel} prompt-fingerprint {fnv64(prompt_text)} "
            f"response-fingerprint {fnv64(response_text)} content-bytes {len(content_text.encode())}"
        )
        manifest_lines.append(
            f"artifact {prompt_rel} "
            f"request requests/{stem}.json requests/{stem}.fingerprint.txt "
            f"response responses/{stem}.json responses/{stem}.fingerprint.txt "
            f"content content/{stem}.txt content/{stem}.fingerprint.txt"
        )
    report_text = "\n".join(report_lines) + "\n"
    manifest_text = "\n".join(manifest_lines) + "\n"
    write_text(artifact_dir / "prompt-llm-harness-report.txt", report_text)
    write_text(
        artifact_dir / "prompt-llm-harness-report.fingerprint.txt",
        fnv64(report_text) + "\n",
    )
    write_text(artifact_dir / "manifest.v03-prompt-llm.txt", manifest_text)
    write_text(artifact_dir / "manifest.fingerprint.txt", fnv64(manifest_text) + "\n")


class _CompletionHandler(BaseHTTPRequestHandler):
    response_text = ""
    response_payload = None
    requests = []

    def do_POST(self):
        body = self.rfile.read(int(self.headers["Content-Length"])).decode()
        self.__class__.requests.append({"path": self.path, "body": json.loads(body)})
        payload = self.__class__.response_payload or {"content": self.__class__.response_text}
        encoded = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def log_message(self, _format, *args):
        return


class CaptureE2eTranscriptsTest(unittest.TestCase):
    def test_example_capture_script_aliases_are_documented_and_callable(self):
        for script in [
            "scripts/capture_example_transcripts.py",
            "scripts/capture_codex_example_transcript.py",
            "scripts/capture_example_batch.py",
        ]:
            output = subprocess.run(
                [sys.executable, script, "--help"],
                cwd=ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            self.assertEqual(
                output.returncode,
                0,
                f"{script}\nstdout:\n{output.stdout}\nstderr:\n{output.stderr}",
            )

        examples_readme = (ROOT / "examples" / "README.md").read_text()
        corpus_readme = (ROOT / "docs" / "ail" / "corpus" / "README.md").read_text()
        for script in [
            "scripts/capture_example_transcripts.py",
            "scripts/capture_codex_example_transcript.py",
            "scripts/capture_example_batch.py",
        ]:
            self.assertIn(script, examples_readme)
        self.assertIn("scripts/capture_example_transcripts.py", corpus_readme)
        self.assertIn("scripts/capture_codex_example_transcript.py", corpus_readme)
        self.assertIn("scripts/capture_example_batch.py", corpus_readme)

    def test_prompt_llm_harness_review_accepts_complete_artifact_bundle(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-prompt-llm-review-"))
        try:
            write_prompt_llm_review_fixture(artifact_dir)
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_prompt_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("AIL-Prompt-LLM-Harness-Review:", review.stdout)
            self.assertIn("prompt-count 11", review.stdout)
            self.assertIn("content-nonempty-count 11", review.stdout)
            self.assertIn("review-result accepted", review.stdout)
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_prompt_llm_harness_review_rejects_empty_content(self):
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-prompt-llm-review-empty-"))
        try:
            write_prompt_llm_review_fixture(
                artifact_dir, empty_content_for="requirements.system.md"
            )
            review = subprocess.run(
                [
                    "python3",
                    "scripts/run_v03_prompt_llm_harness.py",
                    "--review-artifacts",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertNotEqual(
                review.returncode,
                0,
                f"stdout:\n{review.stdout}\nstderr:\n{review.stderr}",
            )
            self.assertIn("review-result rejected", review.stdout)
            self.assertIn(
                "empty content docs/ail/prompts/requirements.system.md",
                review.stdout,
            )
        finally:
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_capture_replaces_seed_entry_with_live_llm_transcript(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-capture-artifacts-"))
        server = None
        try:
            _CompletionHandler.requests = []
            _CompletionHandler.response_payload = None
            _CompletionHandler.response_text = (
                ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
            ).read_text()
            server = HTTPServer(("127.0.0.1", 0), _CompletionHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_example_transcripts.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--entry-id",
                    "example-30",
                    "--endpoint",
                    f"http://127.0.0.1:{server.server_port}/completion",
                    "--endpoint-label",
                    "test-live-endpoint",
                    "--executor-label",
                    "test-live-model",
                    "--semantic-task",
                    "support-ticket-live-capture-30",
                    "--prompt",
                    "Produce the Support Ticket AIL-Spec for live capture replay.",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            examples = (output_dir / "examples.md").read_text()
            self.assertIn("semantic-task: support-ticket-live-capture-30", examples)
            self.assertIn("capture-origin: live-llm", examples)
            self.assertIn("executor-label: test-live-model", examples)
            self.assertIn("endpoint-label: test-live-endpoint", examples)

            request = json.loads((output_dir / "requests" / "example-30.json").read_text())
            self.assertEqual(request["endpoint"], f"http://127.0.0.1:{server.server_port}/completion")
            self.assertEqual(request["body"]["temperature"], 0.0)
            self.assertIn("Support Ticket", request["body"]["prompt"])
            self.assertEqual(_CompletionHandler.requests[0]["path"], "/completion")

            response = json.loads((output_dir / "responses" / "example-30.json").read_text())
            self.assertIn("content", response)
            self.assertIn("AIL-Spec", response["content"])

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "examples-report.txt").read_text()
            self.assertNotIn("capture-origin-count deterministic-seed", report)
            self.assertIn("capture-origin-count live-llm 5", report)
            self.assertIn("capture-origin-count live-codex 111", report)
            self.assertIn(
                "entry example-30 source "
                + str(output_dir / "examples.md")
                + " semantic-task support-ticket-live-capture-30 executor-family llm-http capture-origin live-llm target linux-x86_64-elf",
                report,
            )
        finally:
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_capture_chat_completion_transcript_replays_offline(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-chat-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-chat-capture-artifacts-"))
        server = None
        try:
            spec_text = (
                ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
            ).read_text()
            _CompletionHandler.requests = []
            _CompletionHandler.response_text = ""
            _CompletionHandler.response_payload = {
                "choices": [{"message": {"content": spec_text}}],
                "model": "test-chat-model",
            }
            server = HTTPServer(("127.0.0.1", 0), _CompletionHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_example_transcripts.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--entry-id",
                    "example-32",
                    "--endpoint",
                    f"http://127.0.0.1:{server.server_port}/v1/chat/completions",
                    "--endpoint-label",
                    "test-chat-endpoint",
                    "--executor-label",
                    "test-chat-model",
                    "--semantic-task",
                    "support-ticket-live-chat-capture-32",
                    "--prompt",
                    "Produce the Support Ticket AIL-Spec for live chat capture replay.",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            request = json.loads((output_dir / "requests" / "example-32.json").read_text())
            self.assertEqual(
                request["endpoint"],
                f"http://127.0.0.1:{server.server_port}/v1/chat/completions",
            )
            self.assertIn("messages", request["body"])
            self.assertEqual(request["body"]["messages"][0]["role"], "user")
            self.assertFalse(request["body"]["chat_template_kwargs"]["enable_thinking"])
            self.assertEqual(_CompletionHandler.requests[0]["path"], "/v1/chat/completions")

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "examples-report.txt").read_text()
            self.assertIn("capture-origin-count live-llm 4", report)
            self.assertIn("capture-origin-count live-codex 112", report)
            self.assertIn("entry example-32", report)
        finally:
            _CompletionHandler.response_payload = None
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_capture_uses_schema_input_json_file_for_spec_draft_prompt(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-input-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-input-capture-artifacts-"))
        input_json = Path(tempfile.mkdtemp(prefix="ail-examples-live-input-json-")) / "input.json"
        task_prompt = Path(tempfile.mkdtemp(prefix="ail-examples-live-task-prompt-")) / "task.txt"
        server = None
        try:
            input_payload = {
                "profile": "Application",
                "package_manifest": (
                    ROOT / "examples" / "support_ticket.ail" / "ail-package.md"
                ).read_text(),
                "required_features": ["things", "actions", "failures", "guarantees", "traces"],
                "requirements": (
                    "AIL-Requirements:\n"
                    "- The application manages customer support tickets.\n"
                    "- The CloseTicket action is performed by a support agent.\n"
                    "- CloseTicket requires the ticket to exist and status not to be Closed.\n"
                    "- CloseTicket changes ticket status to Closed.\n"
                    "- CloseTicket records trace event TicketClosed.\n"
                ),
            }
            input_json.write_text(json.dumps(input_payload, indent=2, sort_keys=True) + "\n")
            task_prompt.write_text(
                "Draft the canonical Support Ticket AIL-Spec from the input JSON.\n"
            )
            spec_text = (
                ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
            ).read_text()
            _CompletionHandler.requests = []
            _CompletionHandler.response_text = ""
            _CompletionHandler.response_payload = {
                "choices": [{"message": {"content": spec_text}}],
                "model": "test-chat-model",
            }
            server = HTTPServer(("127.0.0.1", 0), _CompletionHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_example_transcripts.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--entry-id",
                    "example-32",
                    "--endpoint",
                    f"http://127.0.0.1:{server.server_port}/v1/chat/completions",
                    "--endpoint-label",
                    "test-chat-endpoint",
                    "--executor-label",
                    "test-chat-model",
                    "--semantic-task",
                    "support-ticket-live-input-capture-32",
                    "--prompt-file",
                    str(task_prompt),
                    "--input-json-file",
                    str(input_json),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            request = json.loads((output_dir / "requests" / "example-32.json").read_text())
            prompt = request["body"]["messages"][0]["content"]
            self.assertIn("INPUT JSON:", prompt)
            self.assertIn('"requirements"', prompt)
            self.assertIn("CloseTicket records trace event TicketClosed", prompt)
            self.assertIn("Draft the canonical Support Ticket AIL-Spec", prompt)
            self.assertNotIn("USER REQUEST:", prompt)

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
        finally:
            _CompletionHandler.response_payload = None
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)
            shutil.rmtree(input_json.parent, ignore_errors=True)
            shutil.rmtree(task_prompt.parent, ignore_errors=True)

    def test_capture_codex_transcript_imports_live_codex_entry(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-codex-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-codex-capture-artifacts-"))
        transcript_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-codex-transcript-"))
        try:
            request_json = transcript_dir / "request.json"
            response_json = transcript_dir / "response.json"
            request_json.write_text(
                json.dumps(
                    {
                        "agent": "codex-ail-spec-writer",
                        "model": "codex-test-model",
                        "task": "Draft and validate the Support Ticket AIL-Spec.",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )
            response_json.write_text(
                json.dumps(
                    {
                        "content": (
                            ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
                        ).read_text(),
                        "model": "codex-test-model",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_codex_example_transcript.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--entry-id",
                    "example-99",
                    "--executor-label",
                    "codex-ail-spec-writer-test",
                    "--semantic-task",
                    "support-ticket-live-codex-capture-99",
                    "--request-json-file",
                    str(request_json),
                    "--response-json-file",
                    str(response_json),
                    "--checker-result",
                    "accepted",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            examples = (output_dir / "examples.md").read_text()
            self.assertIn("semantic-task: support-ticket-live-codex-capture-99", examples)
            self.assertIn("executor-family: codex-skill-agent", examples)
            self.assertIn("capture-origin: live-codex", examples)
            self.assertIn("executor-label: codex-ail-spec-writer-test", examples)
            example_99 = examples.split("## Example: example-99", 1)[1]
            self.assertNotIn("endpoint-label:", example_99)

            request = json.loads((output_dir / "requests" / "example-99.json").read_text())
            self.assertEqual(request["agent"], "codex-ail-spec-writer")
            response = json.loads((output_dir / "responses" / "example-99.json").read_text())
            self.assertIn("AIL-Spec", response["content"])

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "examples-report.txt").read_text()
            self.assertNotIn("capture-origin-count deterministic-seed", report)
            self.assertIn("capture-origin-count live-llm 4", report)
            self.assertIn("capture-origin-count live-codex 112", report)
            self.assertIn("checker-result-count accepted 109", report)
            self.assertIn("entry example-99", report)
            self.assertIn("capture-origin live-codex", report)
        finally:
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)
            shutil.rmtree(transcript_dir, ignore_errors=True)

    def test_batch_capture_preserves_previous_live_entries(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-batch-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-batch-capture-artifacts-"))
        transcript_dir = Path(tempfile.mkdtemp(prefix="ail-examples-live-batch-transcript-"))
        server = None
        try:
            spec_text = (
                ROOT / "examples" / "support_ticket.ail" / "spec.ail-spec.md"
            ).read_text()
            _CompletionHandler.requests = []
            _CompletionHandler.response_text = ""
            _CompletionHandler.response_payload = {
                "choices": [{"message": {"content": spec_text}}],
                "model": "test-chat-model",
            }
            server = HTTPServer(("127.0.0.1", 0), _CompletionHandler)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()

            codex_request = transcript_dir / "codex-request.json"
            codex_response = transcript_dir / "codex-response.json"
            batch_plan = transcript_dir / "batch-plan.json"
            codex_request.write_text(
                json.dumps(
                    {
                        "agent_contract": (
                            "examples/agents/codex-ail-spec-writer.md"
                        ),
                        "agent_contract_version": "0.1.0",
                        "executor_label": "codex-ail-spec-writer-test",
                        "task": "Draft canonical Support Ticket AIL-Spec.",
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )
            codex_response.write_text(
                json.dumps({"artifact_text": spec_text, "model": "codex-test-model"})
                + "\n"
            )
            batch_plan.write_text(
                json.dumps(
                    {
                        "entries": [
                            {
                                "entry_id": "example-30",
                                "executor_family": "llm-http",
                                "endpoint": (
                                    f"http://127.0.0.1:{server.server_port}"
                                    "/v1/chat/completions"
                                ),
                                "endpoint_label": "test-chat-endpoint",
                                "executor_label": "test-chat-model",
                                "semantic_task": "support-ticket-live-batch-30",
                                "prompt": (
                                    "Produce the Support Ticket AIL-Spec for live "
                                    "batch capture replay."
                                ),
                            },
                            {
                                "entry_id": "example-99",
                                "executor_family": "codex-skill-agent",
                                "executor_label": "codex-ail-spec-writer-test",
                                "semantic_task": "support-ticket-live-codex-batch-99",
                                "request_json_file": str(codex_request),
                                "response_json_file": str(codex_response),
                                "checker_result": "accepted",
                            },
                        ]
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )

            capture = subprocess.run(
                [
                    "python3",
                    "scripts/capture_example_batch.py",
                    "--base-corpus",
                    "examples",
                    "--output-dir",
                    str(output_dir),
                    "--plan-json",
                    str(batch_plan),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(capture.returncode, 0, capture.stderr)

            examples = (output_dir / "examples.md").read_text()
            self.assertIn("semantic-task: support-ticket-live-batch-30", examples)
            self.assertIn("semantic-task: support-ticket-live-codex-batch-99", examples)
            self.assertIn("capture-origin: live-codex", examples)
            self.assertIn("semantic-task: support-ticket-live-spec-input-32", examples)
            self.assertEqual(_CompletionHandler.requests[0]["path"], "/v1/chat/completions")

            replay = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--",
                    "ail-examples",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "examples-report.txt").read_text()
            self.assertNotIn("capture-origin-count deterministic-seed", report)
            self.assertIn("capture-origin-count live-llm 5", report)
            self.assertIn("capture-origin-count live-codex 111", report)
            self.assertIn("entry example-30", report)
            self.assertIn("entry example-32", report)
            self.assertIn("entry example-99", report)
        finally:
            _CompletionHandler.response_payload = None
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)
            shutil.rmtree(transcript_dir, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
