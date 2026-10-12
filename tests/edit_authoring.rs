use ail::ail::*;
use ail::core_model::json_string;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

const DESTRUCTIVE: &str = "The application Audit manages numbers.\nAction: Demo.\nWhen Demo happens:\n- the system records a trace event named Done\n";
const SOURCE: &str = "The application Audit manages numbers.\nA Data has:\n- value: Int\nAction: Demo.\nWhen Demo happens:\n- the system computes data.value as 7 + 2\n- the system returns data.value\n- the system records a trace event named Done\n";

struct Fixture(PathBuf);
impl Fixture {
    fn new(source: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ail-edit-authoring-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("ail-package.md"), "name: edit-authoring\nversion: 0.1.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things, actions, failures, guarantees, traces\nconformance: first-slice\n").unwrap();
        fs::write(root.join("spec.ail-spec.md"), source).unwrap();
        Self(root)
    }
    fn path(&self, name: &str) -> String {
        self.0.join(name).to_str().unwrap().into()
    }
    fn run(&self, command: &str, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ail"))
            .arg(command)
            .arg(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
    fn base(&self) -> AilCore {
        let p = load_ail_package_dir(&self.0).unwrap();
        elaborate_ail_core(&p, &parse_ail_package_document(&p).unwrap())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn text(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn saved_spec_rejection_precedes_native_and_accepted_publication() {
    let f = Fixture::new(SOURCE);
    fs::write(f.path("candidate.md"), SOURCE.replace("7 + 2", "7 - 2")).unwrap();
    let output = f.run(
        "ail-build",
        &[
            "--spec-file",
            &f.path("candidate.md"),
            "--agent",
            "examples/ail_toolchain_agent.ail",
            "--artifact-dir",
            &f.path("artifacts"),
            "--target",
            "linux-x86_64-elf",
            "--action",
            "Demo",
            "--out",
            &f.path("native"),
        ],
    );
    assert!(!output.status.success(), "{}", text(&output));
    assert!(text(&output).contains("AIL-EDIT-"), "{}", text(&output));
    assert!(!f.0.join("artifacts").exists());
    assert!(!f.0.join("native").exists());
    assert_eq!(
        fs::read_to_string(f.path("spec.ail-spec.md")).unwrap(),
        SOURCE
    );
}

#[test]
fn empty_source_requires_explicit_greenfield_and_accepts_saved_spec() {
    let f = Fixture::new("");
    fs::write(f.path("candidate.md"), SOURCE).unwrap();
    let output = f.run("ail-build", &["--spec-file", &f.path("candidate.md")]);
    assert!(!output.status.success(), "{}", text(&output));
    assert!(text(&output).contains("--greenfield"), "{}", text(&output));
    let output = f.run(
        "ail-build",
        &[
            "--spec-file",
            &f.path("candidate.md"),
            "--greenfield",
            "--artifact-dir",
            &f.path("artifacts"),
        ],
    );
    assert!(output.status.success(), "{}", text(&output));
    let evidence = fs::read_to_string(f.path("artifacts/admission.ail-edit.txt")).unwrap();
    assert!(evidence.contains("policy Greenfield"));
    assert!(evidence.contains("admitted true"));
}

#[test]
fn stale_patch_fails_before_requirements_network() {
    let f = Fixture::new(SOURCE);
    let core = f.base();
    fs::write(
        f.path("patch.json"),
        format!(
            r#"{{"schema":"ail-core.patch.v0","package":{},"base_hash":"stale","ops":[]}}"#,
            json_string(&core.package.name)
        ),
    )
    .unwrap();
    let output = f.run(
        "ail-build",
        &[
            "--prompt",
            "change",
            "--expected-patch",
            &f.path("patch.json"),
            "--llm-endpoint",
            "http://127.0.0.1:1",
        ],
    );
    assert!(!output.status.success());
    assert!(
        text(&output).contains("AIL-EDIT-TARGET"),
        "{}",
        text(&output)
    );
    assert!(!text(&output).contains("failed to connect"));
}

#[test]
fn standalone_core_build_optional_baseline_is_enforced() {
    let f = Fixture::new(SOURCE);
    fs::write(f.path("baseline.core"), render_ail_core(&f.base())).unwrap();
    let p = load_ail_package_dir(&f.0).unwrap();
    let candidate = elaborate_ail_core(
        &p,
        &parse_ail_package_spec_text(&p, &SOURCE.replace("7 + 2", "7 - 2")).unwrap(),
    );
    fs::write(f.path("candidate.core"), render_ail_core(&candidate)).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_ail"))
            .args(args)
            .output()
            .unwrap()
    };
    let output = run(&["ail-build", "--core-file", &f.path("candidate.core")]);
    assert!(output.status.success(), "{}", text(&output));
    let output = run(&[
        "ail-build",
        "--core-file",
        &f.path("candidate.core"),
        "--baseline",
        &f.path("baseline.core"),
        "--artifact-dir",
        &f.path("artifacts"),
    ]);
    assert!(!output.status.success());
    assert!(
        text(&output).contains("AIL-EDIT-REJECTED"),
        "{}",
        text(&output)
    );
    assert!(!f.0.join("artifacts").exists());
    let patch = arithmetic_patch(&f);
    for baseline in [f.path("baseline.core"), f.0.to_str().unwrap().to_string()] {
        let output = run(&[
            "ail-build",
            "--core-file",
            &f.path("candidate.core"),
            "--baseline",
            &baseline,
            "--expected-patch",
            &patch,
            "--artifact-dir",
            &f.path("artifacts"),
        ]);
        assert!(output.status.success(), "{}", text(&output));
        let evidence = fs::read_to_string(f.path("artifacts/admission.ail-edit.txt")).unwrap();
        assert!(evidence.contains("exact-expected-target-matching true"));
    }
}

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

const REQUIREMENTS: &str = "AIL-Requirements:\n- The application manages Data fields value and an action Demo.\n- Demo requires no input and guarantees an integer result.\n- No failure is expected; the action records trace Done.\n";

struct Model {
    endpoint: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Model {
    fn new(responses: Vec<String>, mutation: Option<(PathBuf, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!(
            "http://{}/v1/chat/completions",
            listener.local_addr().unwrap()
        );
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let captured = Arc::clone(&requests);
        let stopped = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            for response in responses {
                let (mut stream, _) = loop {
                    match listener.accept() {
                        Ok(connection) => break connection,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            if stopped.load(Ordering::SeqCst) || Instant::now() > deadline {
                                return;
                            }
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(e) => panic!("{e}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                let mut bytes = Vec::new();
                let header_end = loop {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                    if bytes.ends_with(b"\r\n\r\n") {
                        break bytes.len();
                    }
                };
                let length = String::from_utf8_lossy(&bytes)
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                bytes.resize(header_end + length, 0);
                stream.read_exact(&mut bytes[header_end..]).unwrap();
                captured
                    .lock()
                    .unwrap()
                    .push(String::from_utf8(bytes[header_end..].to_vec()).unwrap());
                if let Some((path, content)) = &mutation {
                    fs::write(path, content).unwrap();
                }
                let body = format!(
                    r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
                    json_string(&response)
                );
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            }
        });
        Self {
            endpoint,
            requests,
            stop,
            thread: Some(thread),
        }
    }
    fn finish(mut self) -> Vec<String> {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
        self.requests.lock().unwrap().clone()
    }
}
impl Drop for Model {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}
fn envelope(kind: &str, text: &str) -> String {
    format!(
        r#"{{"artifact_kind":{},"artifact_text":{},"questions":[],"checker_handoff":{{"must_check":true,"expected_profile":"Application","expected_features":[]}}}}"#,
        json_string(kind),
        json_string(text)
    )
}

#[test]
fn model_authoring_rejects_destructive_draft_generated_requirements_and_repair() {
    for command in ["ail-draft", "ail-spec", "ail-build"] {
        let f = Fixture::new(SOURCE);
        fs::write(f.path("requirements.md"), REQUIREMENTS).unwrap();
        let changed = DESTRUCTIVE.to_string();
        let package = load_ail_package_dir(&f.0).unwrap();
        let core = elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, &changed).unwrap(),
        );
        assert!(
            check_ail_core(&core).is_empty(),
            "stub must pass the isolated checker"
        );
        let responses = match command {
            "ail-draft" => vec![changed.clone()],
            "ail-spec" => vec![changed.clone(), changed.clone()],
            _ => vec![REQUIREMENTS.into(), changed.clone(), changed.clone()],
        };
        let expected_requests = responses.len();
        let model = Model::new(responses, None);
        let mut args = vec![
            "--prompt",
            "Preserve the source",
            "--llm-endpoint",
            &model.endpoint,
        ];
        let req_path = f.path("requirements.md");
        let artifact_dir = f.path("artifacts");
        if command == "ail-spec" {
            args.extend(["--requirements-file", &req_path]);
        }
        if command == "ail-build" {
            args.extend([
                "--artifact-dir",
                &artifact_dir,
                "--agent",
                "examples/ail_toolchain_agent.ail",
            ]);
        }
        let output = f.run(command, &args);
        let requests = model.finish();
        assert!(!output.status.success(), "{command}: {}", text(&output));
        assert!(
            text(&output).contains("AIL-EDIT-REJECTED"),
            "{command}: {}",
            text(&output)
        );
        assert_eq!(
            requests.len(),
            expected_requests,
            "{command}: {}",
            text(&output)
        );
        for request in &requests {
            assert!(request.contains("IMMUTABLE EDIT AUTHORITY: PreserveBaseline"));
        }
        assert!(!f.0.join("artifacts").exists());
        assert_eq!(
            fs::read_to_string(f.path("spec.ail-spec.md")).unwrap(),
            SOURCE
        );
    }
}

fn write_story(f: &Fixture) {
    fs::write(f.path("story.md"), "# Audit story\nuser-story-id: edit-authoring\nuser-story: As an auditor I preserve Demo and its arithmetic.\nacceptance-criteria: checked requirements; checked spec; verified bytecode\nstory-journey: story-amendment\nstory-roundtrip: semantic-similar\nsemantic-anchors: Audit; Demo; Done\n").unwrap();
}

#[test]
fn story_rejects_destructive_spec_and_repair_before_acceptance() {
    let f = Fixture::new(SOURCE);
    write_story(&f);
    let changed = DESTRUCTIVE.to_string();
    let model = Model::new(
        vec![
            envelope("AIL-Requirements", REQUIREMENTS),
            envelope("AIL-Spec Canonical", &changed),
            envelope("AIL-Spec Canonical", &changed),
        ],
        None,
    );
    let output = f.run(
        "ail-story",
        &[
            "--story-file",
            &f.path("story.md"),
            "--llm-endpoint",
            &model.endpoint,
            "--agent",
            "examples/ail_toolchain_agent.ail",
            "--artifact-dir",
            &f.path("artifacts"),
            "--target",
            "linux-x86_64-elf",
            "--action",
            "Demo",
            "--out",
            &f.path("native"),
        ],
    );
    assert!(!output.status.success(), "{}", text(&output));
    assert!(
        text(&output).contains("AIL-EDIT-REJECTED"),
        "{}",
        text(&output)
    );
    assert_eq!(model.finish().len(), 3);
    assert!(!f.0.join("artifacts").exists());
    assert!(!f.0.join("native").exists());
    assert_eq!(
        fs::read_to_string(f.path("spec.ail-spec.md")).unwrap(),
        SOURCE
    );
}

#[test]
fn source_snapshot_changes_during_model_request_fail_before_publication() {
    for name in [
        "spec.ail-spec.md",
        "ail-package.md",
        "import/spec.ail-spec.md",
        "import/ail-package.md",
        "registry.md",
    ] {
        let f = Fixture::new(SOURCE);
        if name.starts_with("import/") || name == "registry.md" {
            fs::create_dir_all(f.0.join("import")).unwrap();
            fs::write(
                f.path("import/ail-package.md"),
                "name: imported\nversion: 0.1.0\nprofile: Application\nentry: spec.ail-spec.md\n",
            )
            .unwrap();
            fs::write(f.path("import/spec.ail-spec.md"), "The application Imported manages events.\nAction: Ping.\nWhen Ping happens:\n- the system records a trace event named Pinged\n").unwrap();
            let mut manifest = fs::read_to_string(f.path("ail-package.md")).unwrap();
            if name == "registry.md" {
                manifest.push_str("registry: registry.md\nimports: imported as shared\n");
                fs::write(
                    f.path("registry.md"),
                    "package: imported\nversion: 0.1.0\nidentity: local-test\npath: import\n",
                )
                .unwrap();
            } else {
                manifest.push_str("imports: ./import as shared\n");
            }
            fs::write(f.path("ail-package.md"), manifest).unwrap();
        }
        let path = f.0.join(name);
        let old = fs::read_to_string(&path).unwrap();
        let mutated = format!("{old}\n# Changed during model request\n");
        let model = Model::new(vec![SOURCE.into()], Some((path.clone(), mutated.clone())));
        fs::write(f.path("requirements.md"), REQUIREMENTS).unwrap();
        let output = f.run(
            "ail-build",
            &[
                "--prompt",
                "preserve",
                "--requirements-file",
                &f.path("requirements.md"),
                "--llm-endpoint",
                &model.endpoint,
                "--artifact-dir",
                &f.path("artifacts"),
                "--agent",
                "examples/ail_toolchain_agent.ail",
            ],
        );
        assert!(!output.status.success(), "{name}: {}", text(&output));
        assert!(
            text(&output).contains("AIL-EDIT-SNAPSHOT"),
            "{name}: {}",
            text(&output)
        );
        assert_eq!(model.finish().len(), 1);
        assert!(!f.0.join("artifacts").exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), mutated);
    }
}

#[test]
fn invalid_admission_flag_combinations_fail_before_model_requests() {
    let f = Fixture::new(SOURCE);
    let model = Model::new(vec![SOURCE.into()], None);
    let combos = vec![
        ("ail-draft", vec!["--greenfield"]),
        (
            "ail-draft",
            vec!["--greenfield", "--expected-patch", "absent"],
        ),
        (
            "ail-spec",
            vec!["--core-file", "absent", "--expected-patch", "absent"],
        ),
        ("ail-spec", vec!["--baseline", "absent"]),
        ("ail-story", vec!["--greenfield", "--baseline", "absent"]),
        ("ail-build", vec!["--baseline", "absent"]),
        (
            "ail-build",
            vec!["--core-file", "absent", "--expected-patch", "absent"],
        ),
        ("ail-build", vec!["--core-file", "absent", "--greenfield"]),
        (
            "ail-build",
            vec!["--spec-file", "absent", "--requirements-file", "absent"],
        ),
        (
            "ail-build",
            vec!["--spec-file", "absent", "--prompt", "ignored"],
        ),
        ("ail-check", vec!["--expected-patch", "absent"]),
        ("ail-compile", vec!["--greenfield"]),
    ];
    for (command, mut args) in combos {
        args.extend(["--llm-endpoint", &model.endpoint]);
        let output = f.run(command, &args);
        assert!(
            !output.status.success(),
            "{command} {args:?}: {}",
            text(&output)
        );
        assert!(!text(&output).contains("failed to connect"));
    }
    for command in ["ail-draft", "ail-spec", "ail-story", "ail-build"] {
        let output = f.run(
            command,
            &["--greenfield", "--llm-endpoint", &model.endpoint],
        );
        assert!(!output.status.success());
        assert!(
            text(&output).contains("AIL-EDIT-BASELINE"),
            "{}",
            text(&output)
        );
        for source in ["", "not a valid spec"] {
            let invalid = Fixture::new(source);
            let output = invalid.run(command, &["--llm-endpoint", &model.endpoint]);
            assert!(!output.status.success());
            assert!(
                text(&output).contains("AIL-EDIT-BASELINE"),
                "{}",
                text(&output)
            );
            if source.is_empty() {
                assert!(text(&output).contains("--greenfield"));
            } else {
                let output = invalid.run(
                    command,
                    &["--greenfield", "--llm-endpoint", &model.endpoint],
                );
                assert!(!output.status.success());
                assert!(text(&output).contains("greenfield requires empty local source"));
            }
        }
    }
    assert!(model.finish().is_empty());
}

fn arithmetic_patch(f: &Fixture) -> String {
    let base = f.base();
    let patch = format!(
        r#"{{"schema":"ail-core.patch.v0","package":{},"base_hash":{},"ops":[{{"op":"replace_node_attributes","target":"IntegerExpression:Demo.write0","attributes":{{"operator":"-"}}}},{{"op":"replace_edge_attributes","kind":"writes","source":"Action:Demo","target":"Field:Data.value","attributes":{{"provenance":"action:Demo.write:computes data.value as 7 - 2"}}}}]}}"#,
        json_string(&base.package.name),
        json_string(&ail_core_hash(&base))
    );
    let target = SOURCE.replace("7 + 2", "7 - 2");
    let package = load_ail_package_dir(&f.0).unwrap();
    let candidate = elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, &target).unwrap(),
    );
    let context = AilEditContext::expected_patch(&package, &patch).unwrap();
    assert!(
        context.check_candidate(&candidate).is_empty(),
        "{}",
        context.render_admission(&candidate)
    );
    fs::write(f.path("patch.json"), patch).unwrap();
    f.path("patch.json")
}
fn fingerprint(text: &str) -> String {
    let hash = text.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    format!("fnv64:{hash:016x}")
}

#[test]
fn explicit_greenfield_model_modes_succeed_and_story_manifest_binds_admission() {
    for command in ["ail-draft", "ail-spec", "ail-build", "ail-story"] {
        let f = Fixture::new("");
        write_story(&f);
        fs::write(f.path("requirements.md"), REQUIREMENTS).unwrap();
        let responses = match command {
            "ail-draft" | "ail-spec" => vec![SOURCE.into()],
            "ail-build" => vec![REQUIREMENTS.into(), SOURCE.into()],
            _ => vec![
                envelope("AIL-Requirements", REQUIREMENTS),
                envelope("AIL-Spec Canonical", SOURCE),
            ],
        };
        let expected_requests = responses.len();
        let model = Model::new(responses, None);
        let requirement_path = f.path("requirements.md");
        let story_path = f.path("story.md");
        let artifacts = f.path("artifacts");
        let mut args = vec!["--greenfield", "--llm-endpoint", &model.endpoint];
        if command == "ail-story" {
            args.extend(["--story-file", &story_path, "--artifact-dir", &artifacts]);
        } else {
            args.extend(["--prompt", "Author a new program"]);
        }
        if command == "ail-spec" {
            args.extend(["--requirements-file", &requirement_path]);
        }
        if command == "ail-build" {
            args.extend(["--artifact-dir", &artifacts]);
        }
        let output = f.run(command, &args);
        assert!(output.status.success(), "{command}: {}", text(&output));
        let requests = model.finish();
        assert_eq!(requests.len(), expected_requests, "{command}");
        assert!(
            requests
                .iter()
                .all(|r| r.contains("IMMUTABLE EDIT AUTHORITY: Greenfield"))
        );
        assert_eq!(fs::read_to_string(f.path("spec.ail-spec.md")).unwrap(), "");
        if matches!(command, "ail-build" | "ail-story") {
            let evidence = fs::read_to_string(f.path("artifacts/admission.ail-edit.txt")).unwrap();
            let manifest = fs::read_to_string(f.path("artifacts/manifest.ail-build.txt")).unwrap();
            assert!(evidence.contains("admitted true"));
            assert!(manifest.contains(&format!(
                "edit-admission admission.ail-edit.txt {}",
                fingerprint(&evidence)
            )));
            if command == "ail-story" {
                let manifest =
                    fs::read_to_string(f.path("artifacts/manifest.ail-story.txt")).unwrap();
                assert!(manifest.contains(&format!(
                    "edit-admission admission.ail-edit.txt {}",
                    fingerprint(&evidence)
                )));
                let comparison =
                    fs::read_to_string(f.path("artifacts/story-amendment-comparison.txt")).unwrap();
                assert!(comparison.contains("executable-edit-admission admitted"));
                assert!(comparison.contains("anchor-coverage-scope story-text-only"));
                assert!(manifest.contains(&format!(
                    "story-amendment-comparison story-amendment-comparison.txt {}",
                    fingerprint(&comparison)
                )));
            }
        }
    }
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn authorized_model_change_executes_vm_and_native_and_rejection_keeps_prior_artifacts() {
    let f = Fixture::new(SOURCE);
    let baseline = compile_ail_core_bytecode(&f.base()).unwrap();
    let before =
        run_ail_bytecode_action(&baseline, "Demo", std::collections::BTreeMap::new()).unwrap();
    assert_eq!(before.final_state["data.value"], "9");
    let patch = arithmetic_patch(&f);
    let changed = SOURCE.replace("7 + 2", "7 - 2");
    let model = Model::new(vec![REQUIREMENTS.into(), changed.clone()], None);
    let args = [
        "--prompt",
        "Change addition to subtraction",
        "--expected-patch",
        &patch,
        "--llm-endpoint",
        &model.endpoint,
        "--artifact-dir",
        &f.path("artifacts"),
        "--target",
        "linux-x86_64-elf",
        "--action",
        "Demo",
        "--out",
        &f.path("native"),
    ];
    let output = f.run("ail-build", &args);
    assert!(output.status.success(), "{}", text(&output));
    let requests = model.finish();
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .all(|r| r.contains("IMMUTABLE EDIT AUTHORITY: ExpectedPatchTarget"))
    );
    assert!(
        requests
            .iter()
            .all(|r| !r.contains("Required source trace events to preserve exactly:"))
    );
    let bytecode_text = fs::read_to_string(f.path("artifacts/artifact.ailbc.json")).unwrap();
    let bytecode = parse_ail_bytecode(&bytecode_text).unwrap();
    assert!(verify_ail_bytecode(&bytecode).is_empty());
    let run =
        run_ail_bytecode_action(&bytecode, "Demo", std::collections::BTreeMap::new()).unwrap();
    assert_eq!(run.final_state["data.value"], "5");
    let native = Command::new(f.path("native")).output().unwrap();
    assert!(native.status.success());
    assert_eq!(String::from_utf8(native.stdout).unwrap(), "data.value=5\n");
    let evidence = fs::read_to_string(f.path("artifacts/admission.ail-edit.txt")).unwrap();
    assert!(evidence.contains("policy ExpectedPatchTarget"));
    assert!(evidence.contains("exact-expected-target-matching true"));
    assert!(evidence.contains("admitted true"));
    let old_files = fs::read_dir(f.path("artifacts"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            (path.clone(), fs::read(path).unwrap())
        })
        .collect::<Vec<_>>();
    let old_native = fs::read(f.path("native")).unwrap();
    fs::write(
        f.path("candidate.md"),
        changed.replace("Done", "UnrelatedTrace"),
    )
    .unwrap();
    let output = f.run(
        "ail-build",
        &[
            "--spec-file",
            &f.path("candidate.md"),
            "--expected-patch",
            &patch,
            "--artifact-dir",
            &f.path("artifacts"),
            "--target",
            "linux-x86_64-elf",
            "--action",
            "Demo",
            "--out",
            &f.path("native"),
        ],
    );
    assert!(!output.status.success());
    assert!(text(&output).contains("AIL-EDIT-REJECTED"));
    for (path, bytes) in old_files {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert_eq!(fs::read(f.path("native")).unwrap(), old_native);
    assert_eq!(
        fs::read_to_string(f.path("spec.ail-spec.md")).unwrap(),
        SOURCE
    );
}

#[test]
fn explicit_compiler_pass_evidence_distinguishes_authoring_and_transformation() {
    let f = Fixture::new(SOURCE);
    fs::write(f.path("candidate.md"), SOURCE).unwrap();
    let output = f.run(
        "ail-build",
        &[
            "--spec-file",
            &f.path("candidate.md"),
            "--pass",
            "examples/compiler_pass.ail",
            "--artifact-dir",
            &f.path("artifacts"),
        ],
    );
    assert!(output.status.success(), "{}", text(&output));
    let evidence = fs::read_to_string(f.path("artifacts/admission.ail-edit.txt")).unwrap();
    assert!(evidence.contains("admitted true"));
    assert!(evidence.contains("authoring-proof-scope pre-pass-candidate"));
    assert!(evidence.contains("transformation-checked true"));
    assert!(evidence.contains("pass-behavior-preservation not-claimed"));
    let manifest = fs::read_to_string(f.path("artifacts/manifest.ail-build.txt")).unwrap();
    assert!(manifest.contains(&format!(
        "edit-admission admission.ail-edit.txt {}",
        fingerprint(&evidence)
    )));
}

#[test]
fn standalone_core_build_identifies_malformed_baseline_authority() {
    let f = Fixture::new(SOURCE);
    fs::write(f.path("candidate.core"), render_ail_core(&f.base())).unwrap();
    fs::write(f.path("invalid.core"), "malformed").unwrap();
    fs::write(f.path("spec.ail-spec.md"), "malformed").unwrap();
    for baseline in [f.path("invalid.core"), f.0.to_str().unwrap().to_string()] {
        let output = Command::new(env!("CARGO_BIN_EXE_ail"))
            .args([
                "ail-build",
                "--core-file",
                &f.path("candidate.core"),
                "--baseline",
                &baseline,
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            text(&output).contains("AIL-EDIT-BASELINE"),
            "{}",
            text(&output)
        );
    }
}

#[test]
fn positional_package_cannot_select_standalone_core_build() {
    let f = Fixture::new(SOURCE);
    let base = f.base();
    let package = load_ail_package_dir(&f.0).unwrap();
    let candidate = elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, &SOURCE.replace("7 + 2", "7 - 2")).unwrap(),
    );
    assert!(check_ail_core(&candidate).is_empty());
    fs::write(f.path("candidate.core"), render_ail_core(&candidate)).unwrap();
    fs::write(f.path("baseline.core"), render_ail_core(&base)).unwrap();
    let patch = arithmetic_patch(&f);
    for mode in ["unguarded", "baseline", "expected-target"] {
        let candidate_path = f.path("candidate.core");
        let baseline_path = f.path("baseline.core");
        let artifacts = f.path("artifacts");
        let native = f.path("native");
        let mut args = vec![
            "--core-file",
            &candidate_path,
            "--artifact-dir",
            &artifacts,
            "--target",
            "linux-x86_64-elf",
            "--action",
            "Demo",
            "--out",
            &native,
        ];
        if mode != "unguarded" {
            args.extend(["--baseline", &baseline_path]);
        }
        if mode == "expected-target" {
            args.extend(["--expected-patch", &patch]);
        }
        let output = f.run("ail-build", &args);
        assert!(!output.status.success(), "{mode}: {}", text(&output));
        assert!(
            text(&output)
                .contains("ail-build cannot combine a positional package with --core-file"),
            "{mode}: {}",
            text(&output)
        );
        assert!(!f.0.join("artifacts").exists(), "{mode}");
        assert!(!f.0.join("native").exists(), "{mode}");
        assert_eq!(
            fs::read_to_string(f.path("spec.ail-spec.md")).unwrap(),
            SOURCE
        );
    }
}
