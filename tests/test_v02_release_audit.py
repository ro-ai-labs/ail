import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT_PATH = REPO_ROOT / "scripts" / "run_v02_release_audit.py"


def load_audit_module():
    spec = importlib.util.spec_from_file_location("run_v02_release_audit", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class V02ReleaseAuditTest(unittest.TestCase):
    def test_plan_uses_bundle_local_artifact_paths(self):
        audit = load_audit_module()
        bundle_root = Path("/tmp/ail-v02-release-evidence")

        plan = audit.build_v02_audit_plan(bundle_root)
        names = [step.name for step in plan]
        commands = [" ".join(step.command) for step in plan]

        self.assertIn("conformance-support", names)
        self.assertIn("build-support", names)
        self.assertIn("spec-roundtrip", names)
        self.assertIn("e2e-corpus", names)
        e2e_step = plan[names.index("e2e-corpus")]
        self.assertIn("model-executor-manifest.txt", e2e_step.required_files)
        self.assertIn("model-executor-manifest.fingerprint.txt", e2e_step.required_files)
        build_command = plan[names.index("build-support")].command
        self.assertIn("--spec-file", build_command)
        self.assertIn("examples/support_ticket.ail/spec.ail-spec.md", build_command)
        self.assertNotIn("--prompt", build_command)
        self.assertIn(
            str(bundle_root / "artifacts" / "v02-build-support" / "checked.ail-core.txt"),
            commands[names.index("spec-roundtrip")],
        )
        for step in plan:
            if step.artifact_dir is not None:
                self.assertTrue(
                    str(step.artifact_dir).startswith(str(bundle_root / "artifacts")),
                    f"{step.name} artifact dir escaped bundle root: {step.artifact_dir}",
                )

    def test_verify_artifact_dir_requires_manifest_and_matching_fingerprint(self):
        audit = load_audit_module()
        with tempfile.TemporaryDirectory() as tmp:
            artifact_dir = Path(tmp)
            manifest = "AIL-Test-Manifest:\nentry target.txt fnv64:1234\n"
            artifact_dir.joinpath("manifest.ail-test.txt").write_text(manifest)
            artifact_dir.joinpath("manifest.fingerprint.txt").write_text(
                audit.fnv64_fingerprint(manifest.encode("utf-8")) + "\n"
            )
            model_executor_manifest = "AIL-E2E-Model-Executor-Manifest:\nentry-count 100\n"
            artifact_dir.joinpath("model-executor-manifest.txt").write_text(
                model_executor_manifest
            )
            artifact_dir.joinpath("model-executor-manifest.fingerprint.txt").write_text(
                audit.fnv64_fingerprint(model_executor_manifest.encode("utf-8"))
                + "\n"
            )

            verified = audit.verify_artifact_dir(
                artifact_dir,
                "manifest.ail-test.txt",
                (
                    "model-executor-manifest.txt",
                    "model-executor-manifest.fingerprint.txt",
                ),
            )

            self.assertEqual(
                verified,
                [
                    f"artifact-dir {artifact_dir}",
                    "artifact-manifest manifest.ail-test.txt "
                    + audit.fnv64_fingerprint(manifest.encode("utf-8")),
                    "artifact-manifest-fingerprint manifest.fingerprint.txt ok",
                    "artifact-required-file model-executor-manifest.txt ok",
                    "artifact-required-file model-executor-manifest.fingerprint.txt ok",
                    "artifact-required-fingerprint model-executor-manifest.fingerprint.txt ok",
                ],
            )

            artifact_dir.joinpath("manifest.fingerprint.txt").write_text("fnv64:bad\n")
            with self.assertRaisesRegex(ValueError, "manifest fingerprint mismatch"):
                audit.verify_artifact_dir(
                    artifact_dir,
                    "manifest.ail-test.txt",
                    (
                        "model-executor-manifest.txt",
                        "model-executor-manifest.fingerprint.txt",
                    ),
                )

    def test_dry_run_writes_fingerprinted_release_manifest(self):
        with tempfile.TemporaryDirectory() as tmp:
            bundle_root = Path(tmp) / "bundle"
            result = subprocess.run(
                [
                    "python3",
                    str(SCRIPT_PATH),
                    "--bundle-root",
                    str(bundle_root),
                    "--dry-run",
                ],
                cwd=REPO_ROOT,
                text=True,
                capture_output=True,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            manifest_path = bundle_root / "release-audit-manifest.txt"
            fingerprint_path = bundle_root / "release-audit-manifest.fingerprint.txt"
            self.assertTrue(manifest_path.exists())
            self.assertTrue(fingerprint_path.exists())
            manifest = manifest_path.read_text()
            self.assertIn("AIL-v0.2-Release-Audit-Manifest:", manifest)
            self.assertIn("mode dry-run", manifest)
            self.assertIn("step cargo-test command cargo test", manifest)
            self.assertIn(
                "step e2e-corpus command cargo run -- ail-e2e-corpus "
                "docs/ail/corpus/e2e",
                manifest,
            )
            self.assertIn(
                f"artifact-dir {bundle_root / 'artifacts' / 'v02-e2e-corpus'}",
                manifest,
            )
            self.assertEqual(
                fingerprint_path.read_text().strip(),
                load_audit_module().fnv64_fingerprint(manifest.encode("utf-8")),
            )


if __name__ == "__main__":
    unittest.main()
