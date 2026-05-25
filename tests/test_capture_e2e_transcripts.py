import json
import shutil
import subprocess
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


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
    def test_capture_replaces_seed_entry_with_live_llm_transcript(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-e2e-live-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-e2e-live-capture-artifacts-"))
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
                    "scripts/capture_e2e_transcripts.py",
                    "--base-corpus",
                    "docs/ail/corpus/e2e",
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
                    "ail-e2e-corpus",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "e2e-corpus-report.txt").read_text()
            self.assertIn("capture-origin-count deterministic-seed 98", report)
            self.assertIn("capture-origin-count live-llm 2", report)
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
        output_dir = Path(tempfile.mkdtemp(prefix="ail-e2e-live-chat-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-e2e-live-chat-capture-artifacts-"))
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
                    "scripts/capture_e2e_transcripts.py",
                    "--base-corpus",
                    "docs/ail/corpus/e2e",
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
                    "ail-e2e-corpus",
                    str(output_dir),
                    "--artifact-dir",
                    str(artifact_dir),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(replay.returncode, 0, replay.stderr)
            report = (artifact_dir / "e2e-corpus-report.txt").read_text()
            self.assertIn("capture-origin-count live-llm 1", report)
            self.assertIn("entry example-32", report)
        finally:
            _CompletionHandler.response_payload = None
            if server is not None:
                server.shutdown()
                server.server_close()
            shutil.rmtree(output_dir, ignore_errors=True)
            shutil.rmtree(artifact_dir, ignore_errors=True)

    def test_capture_uses_schema_input_json_file_for_spec_draft_prompt(self):
        output_dir = Path(tempfile.mkdtemp(prefix="ail-e2e-live-input-capture-"))
        artifact_dir = Path(tempfile.mkdtemp(prefix="ail-e2e-live-input-capture-artifacts-"))
        input_json = Path(tempfile.mkdtemp(prefix="ail-e2e-live-input-json-")) / "input.json"
        task_prompt = Path(tempfile.mkdtemp(prefix="ail-e2e-live-task-prompt-")) / "task.txt"
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
                    "scripts/capture_e2e_transcripts.py",
                    "--base-corpus",
                    "docs/ail/corpus/e2e",
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
                    "ail-e2e-corpus",
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


if __name__ == "__main__":
    unittest.main()
