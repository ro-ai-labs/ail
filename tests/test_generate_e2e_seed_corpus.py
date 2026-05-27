import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "generate_e2e_seed_corpus.py"


class GenerateE2eSeedCorpusTest(unittest.TestCase):
    def test_generated_seed_corpus_is_explicitly_legacy_non_release(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir) / "seed-examples"
            subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--output-dir",
                    str(output_dir),
                ],
                check=True,
                cwd=ROOT,
            )

            catalog = (output_dir / "examples.md").read_text()
            self.assertIn("# AIL Legacy Deterministic Seed Corpus", catalog)
            self.assertIn("corpus-kind: legacy-deterministic-seed", catalog)
            self.assertIn("release-evidence: false", catalog)
            self.assertIn("not release evidence", catalog)
            self.assertIn("capture-origin: deterministic-seed", catalog)

            story = (output_dir / "stories" / "example-0.md").read_text()
            self.assertIn("semantic-anchors:", story)


if __name__ == "__main__":
    unittest.main()
