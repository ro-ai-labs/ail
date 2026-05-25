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
    requests = []

    def do_POST(self):
        body = self.rfile.read(int(self.headers["Content-Length"])).decode()
        self.__class__.requests.append({"path": self.path, "body": json.loads(body)})
        payload = {"content": self.__class__.response_text}
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
            self.assertIn("capture-origin-count deterministic-seed 99", report)
            self.assertIn("capture-origin-count live-llm 1", report)
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


if __name__ == "__main__":
    unittest.main()
