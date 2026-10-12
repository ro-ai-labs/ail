use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application Files manages text.\n\nA Data has:\n- text: Text\n- copy: Text\n- suffix: Text\n- count: Int\n\nAn Output has:\n- text: Text\n- count: Int\n\n";

fn package() -> AilPackage {
    load_ail_package_dir("bench/R001-add/program.ail").unwrap()
}

fn source(body: &str) -> String {
    format!(
        "{DECLARATIONS}Action: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Completed\n"
    )
}

fn core(source: &str) -> AilCore {
    let package = package();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}

#[test]
fn source_file_operations_reach_checked_core() {
    let candidate = core(&source(
        "- the system reads text from file resource input into data.text\n- the system writes text data.text to file resource output",
    ));
    assert_eq!(
        candidate
            .graph
            .nodes
            .iter()
            .filter(|node| node.kind == "FileOperation")
            .count(),
        2
    );
    assert!(
        check_ail_core(&candidate).is_empty(),
        "{:?}",
        check_ail_core(&candidate)
    );
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "ail-files-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn file(&self, name: &str, value: &[u8]) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, value).unwrap();
        path
    }

    fn binary(&self, program: &AilBytecodeProgram, action: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = self.path("program.elf");
        fs::write(
            &path,
            compile_ail_bytecode_native_elf(program, action, "linux-x86_64-elf").unwrap(),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn resources(reads: &[(&str, &Path)], writes: &[(&str, &Path)]) -> AilFileResources {
    AilFileResources {
        reads: reads
            .iter()
            .map(|(name, path)| ((*name).into(), path.to_path_buf()))
            .collect(),
        writes: writes
            .iter()
            .map(|(name, path)| ((*name).into(), path.to_path_buf()))
            .collect(),
    }
}

fn state(values: &[(&str, &str)]) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect()
}

fn command_output(command: &mut Command) -> Output {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut attempts = 0;
    let mut child = loop {
        match command.spawn() {
            Ok(child) => break child,
            Err(error) if error.raw_os_error() == Some(26) && attempts < 10 => {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("cannot start test process: {error}"),
        }
    };
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let stdout_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            let _ = stdout_reader.join().unwrap();
            let stderr = stderr_reader.join().unwrap();
            panic!(
                "file operation blocked: {}",
                String::from_utf8_lossy(&stderr[..stderr.len().min(2_000)])
            );
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Output {
        status: child.wait().unwrap(),
        stdout: stdout_reader.join().unwrap(),
        stderr: stderr_reader.join().unwrap(),
    }
}

fn native(binary: &Path, grants: &AilFileResources, values: &BTreeMap<String, String>) -> Output {
    let mut command = Command::new(binary);
    for (name, path) in &grants.reads {
        command.arg(format!("--read-file={name}={}", path.display()));
    }
    for (name, path) in &grants.writes {
        command.arg(format!("--write-file={name}={}", path.display()));
    }
    for (key, value) in values {
        command.arg(format!("{key}={value}"));
    }
    command_output(&mut command)
}

fn error(output: &Output, category: &str) {
    assert!(
        !output.status.success(),
        "expected {category}, got {output:?}"
    );
    assert!(
        output.stdout.is_empty(),
        "native failure emitted result output: {output:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(category),
        "expected {category}: {output:?}"
    );
}

fn compiled(body: &str) -> AilBytecodeProgram {
    compile_ail_core_bytecode(&core(&source(body))).unwrap()
}

const TRANSFORM: &str = "- the system reads text from file resource input into data.text\n- the system assigns \"!\" to data.suffix\n- the system joins text data.text and data.suffix into output.text\n- the system counts characters in output.text into output.count\n- the system writes text output.text to file resource output\n- the system returns output.count";

#[test]
fn files_transform_through_source_saved_core_rendered_source_ste_and_native() {
    let scratch = Scratch::new();
    let input = scratch.file("input with = sign.txt", "é\0🙂\n".as_bytes());
    let output = scratch.path("output.txt");
    let grants = resources(&[("input", &input)], &[("output", &output)]);
    let original = core(&source(TRANSFORM));
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let binary = scratch.binary(&program, "Run");
        fs::write(&output, b"long old value to truncate").unwrap();
        let vm =
            run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
        assert_eq!(vm.output_state(), state(&[("output.count", "5")]));
        assert_eq!(fs::read(&output).unwrap(), "é\0🙂\n!".as_bytes());
        assert!(
            vm.trace
                .iter()
                .all(|line| !line.contains(&input.display().to_string())
                    && !line.contains(&output.display().to_string())
                    && !line.contains("é\0🙂"))
        );
        fs::write(&output, b"long old value to truncate").unwrap();
        let result = native(&binary, &grants, &BTreeMap::new());
        assert!(result.status.success(), "{result:?}");
        assert_eq!(result.stdout, b"output.count=5\n");
        assert_eq!(fs::read(&output).unwrap(), "é\0🙂\n!".as_bytes());
    }
}

#[test]
fn file_grants_are_directional_and_cannot_come_from_state_or_package_metadata() {
    let scratch = Scratch::new();
    let input = scratch.file("input.txt", b"content");
    let output = scratch.file("output.txt", b"keep");
    for (body, reversed) in [
        (
            "- the system reads text from file resource input into data.text",
            resources(&[], &[("input", &input)]),
        ),
        (
            "- the system writes text data.text to file resource output",
            resources(&[("output", &output)], &[]),
        ),
    ] {
        let mut program = compiled(body);
        program.capability_grants.push(AilCapabilityGrant {
            package: program.package_name.clone(),
            capability: "files".into(),
            effects: vec!["read-file".into(), "write-file".into()],
            approvals: vec![],
        });
        let binary = scratch.binary(&program, "Run");
        let values = state(&[
            ("data.text", "new"),
            ("input", input.to_str().unwrap()),
            ("output", output.to_str().unwrap()),
            ("read-file.input", input.to_str().unwrap()),
            ("write-file.output", output.to_str().unwrap()),
        ]);
        let denied = run_ail_bytecode_action(&program, "Run", values.clone()).unwrap_err();
        assert!(denied.contains("AIL-RUNTIME-CAPABILITY"), "{denied}");
        for grants in [resources(&[], &[]), reversed] {
            let denied =
                run_ail_bytecode_action_with_files(&program, "Run", values.clone(), &grants)
                    .unwrap_err();
            assert!(denied.contains("AIL-RUNTIME-CAPABILITY"), "{denied}");
            error(&native(&binary, &grants, &values), "AIL-RUNTIME-CAPABILITY");
        }
        assert_eq!(fs::read(&output).unwrap(), b"keep");
    }
}

#[test]
fn reads_validate_empty_utf8_and_size_before_writing_any_output() {
    let scratch = Scratch::new();
    let input = scratch.path("input.txt");
    let output = scratch.path("output.txt");
    let grants = resources(&[("input", &input)], &[("output", &output)]);
    let program = compiled(
        "- the system reads text from file resource input into data.text\n- the system counts characters in data.text into output.count\n- the system writes text data.text to file resource output\n- the system returns output.count",
    );
    let binary = scratch.binary(&program, "Run");
    for (bytes, expected) in [
        (vec![], Ok(0)),
        ("é\0🙂".as_bytes().to_vec(), Ok(3)),
        (vec![b'x'; 1_048_576], Ok(1_048_576)),
        ("🙂".repeat(262_144).into_bytes(), Ok(262_144)),
        (vec![0xff], Err("AIL-RUNTIME-TEXT:")),
        (vec![0xed, 0xa0, 0x80], Err("AIL-RUNTIME-TEXT:")),
        (vec![b'x'; 1_048_577], Err("AIL-RUNTIME-TEXT-LIMIT")),
        (vec![0xff; 1_048_577], Err("AIL-RUNTIME-TEXT-LIMIT")),
    ] {
        fs::write(&input, &bytes).unwrap();
        for backend in ["vm", "native"] {
            fs::write(&output, b"unchanged").unwrap();
            if backend == "vm" {
                let result =
                    run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants);
                match expected {
                    Ok(count) => assert_eq!(
                        result.unwrap().final_state["output.count"],
                        count.to_string()
                    ),
                    Err(category) => assert!(result.unwrap_err().contains(category)),
                }
            } else {
                let result = native(&binary, &grants, &BTreeMap::new());
                match expected {
                    Ok(count) => {
                        assert!(result.status.success(), "{result:?}");
                        assert_eq!(result.stdout, format!("output.count={count}\n").as_bytes());
                    }
                    Err(category) => error(&result, category),
                }
            }
            assert_eq!(
                fs::read(&output).unwrap(),
                if expected.is_ok() {
                    bytes.as_slice()
                } else {
                    b"unchanged"
                }
            );
        }
    }
}

#[test]
fn writes_check_grant_then_text_before_opening_or_truncating() {
    let scratch = Scratch::new();
    let output = scratch.file("output.txt", b"untouched");
    let grants = resources(&[], &[("output", &output)]);
    let initial = compiled("- the system writes text data.text to file resource output");
    for oversized in [false, true] {
        let mut program = initial.clone();
        if oversized {
            program.actions.get_mut("Run").unwrap().instructions.insert(
                0,
                AilBytecodeInstruction {
                    opcode: "SET_FIELD".into(),
                    operands: state(&[
                        ("key", "data.text"),
                        ("value", &"x".repeat(1_048_577)),
                        ("text", "oversize input fixture"),
                    ]),
                },
            );
        }
        let binary = scratch.binary(&program, "Run");
        for (bindings, category) in [
            (
                &grants,
                if oversized {
                    "AIL-RUNTIME-TEXT-LIMIT"
                } else {
                    "AIL-RUNTIME-TEXT:"
                },
            ),
            (&resources(&[], &[]), "AIL-RUNTIME-CAPABILITY"),
        ] {
            let denied =
                run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), bindings)
                    .unwrap_err();
            assert!(denied.contains(category), "{denied}");
            error(&native(&binary, bindings, &BTreeMap::new()), category);
            assert_eq!(fs::read(&output).unwrap(), b"untouched");
        }
    }
}

#[test]
fn file_effects_follow_calls_loops_and_same_path_order_without_mutating_copies() {
    let scratch = Scratch::new();
    let path = scratch.path("shared.txt");
    let grants = resources(&[("shared", &path)], &[("shared", &path)]);
    let text = format!(
        "{}\nAction: Worker.\nWhen Worker happens:\n- the system reads text from file resource shared into data.text\n- the system joins text data.text and data.suffix into data.text\n- the system writes text data.text to file resource shared\n- the system records a trace event named Worked\n",
        source(
            "- the system reads text from file resource shared into data.copy\n- the system assigns \"!\" to data.suffix\n- the system assigns 0 to data.count\n- while data.count < 3:\n- the system calls Worker\n- the system computes data.count as data.count + 1\n- end while\n- the system reads text from file resource shared into output.text\n- the system returns data.copy\n- the system returns output.text"
        )
    );
    let candidate = core(&text);
    let program =
        compile_ail_core_bytecode(&parse_ail_core_text(&render_ail_core(&candidate)).unwrap())
            .unwrap();
    let binary = scratch.binary(&program, "Run");
    fs::write(&path, b"seed").unwrap();
    let vm = run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
    assert_eq!(
        vm.output_state(),
        state(&[("data.copy", "seed"), ("output.text", "seed!!!")])
    );
    assert_eq!(fs::read(&path).unwrap(), b"seed!!!");
    fs::write(&path, b"seed").unwrap();
    let result = native(&binary, &grants, &BTreeMap::new());
    assert!(result.status.success(), "{result:?}");
    assert_eq!(result.stdout, b"data.copy=seed\noutput.text=seed!!!\n");
    assert_eq!(fs::read(&path).unwrap(), b"seed!!!");
}

#[test]
fn later_failure_keeps_completed_writes_and_untaken_branch_needs_no_grant() {
    let scratch = Scratch::new();
    let output = scratch.path("output.txt");
    let grants = resources(&[], &[("output", &output)]);
    let program = compiled(
        "- the system assigns \"committed\" to data.text\n- if 1 == 0:\n- the system reads text from file resource untaken into data.text\n- end if\n- the system writes text data.text to file resource output\n- the system reads text from file resource denied into data.text",
    );
    let binary = scratch.binary(&program, "Run");
    let denied =
        run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap_err();
    assert!(denied.contains("AIL-RUNTIME-CAPABILITY"), "{denied}");
    assert_eq!(fs::read(&output).unwrap(), b"committed");
    fs::remove_file(&output).unwrap();
    error(
        &native(&binary, &grants, &BTreeMap::new()),
        "AIL-RUNTIME-CAPABILITY",
    );
    assert_eq!(fs::read(&output).unwrap(), b"committed");
}

#[test]
fn imports_qualify_resource_names_and_field_names() {
    let scratch = Scratch::new();
    let input = scratch.file("input.txt", b"imported");
    let output = scratch.path("output.txt");
    let mut child = package();
    child.metadata.name = "file-child".into();
    child.spec_text = source(
        "- the system reads text from file resource input into data.text\n- the system writes text data.text to file resource output\n- the system returns data.text",
    );
    let mut parent = package();
    parent.spec_text = "The application Parent manages text.\nAction: Run.\nWhen Run happens:\n- the system calls Files.Run\n- the system returns files.data.text\n- the system records a trace event named Done\n".into();
    parent.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Files".into(),
            resolved_package: None,
            registry_identity: None,
        },
        package: Box::new(child),
    });
    let candidate = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    let program = ail::ste::parse_ail_ste(
        &compile_ail_core_ste(&parse_ail_core_text(&render_ail_core(&candidate)).unwrap()).unwrap(),
    )
    .unwrap();
    let grants = resources(&[("files.input", &input)], &[("files.output", &output)]);
    let vm = run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
    assert_eq!(vm.output_state(), state(&[("files.data.text", "imported")]));
    let result = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
    assert!(result.status.success(), "{result:?}");
    assert_eq!(result.stdout, b"files.data.text=imported\n");
    assert_eq!(fs::read(&output).unwrap(), b"imported");
    let unqualified = resources(&[("input", &input)], &[("output", &output)]);
    assert!(
        run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &unqualified)
            .unwrap_err()
            .contains("AIL-RUNTIME-CAPABILITY")
    );
}

#[test]
fn malformed_file_core_and_bytecode_fail_before_effects() {
    let original = core(&source(
        "- the system reads text from file resource input into data.text\n- the system writes text data.text to file resource output",
    ));
    for kind in ["file_step", "file_destination", "file_source"] {
        let mut candidate = original.clone();
        candidate.graph.edges.push(
            candidate
                .graph
                .edges
                .iter()
                .find(|edge| edge.kind == kind)
                .unwrap()
                .clone(),
        );
        assert!(
            compile_ail_core_bytecode(&parse_ail_core_text(&render_ail_core(&candidate)).unwrap())
                .is_err(),
            "duplicate {kind}"
        );
    }
    for (key, value) in [
        ("file_kind", "delete"),
        ("resource", "../outside"),
        ("hidden", "ignored"),
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "FileOperation")
            .unwrap()
            .attributes
            .insert(key.into(), value.into());
        assert!(compile_ail_core_bytecode(&candidate).is_err(), "{key}");
    }
    for profile in ["Compiler", "AgentTool", "UI", "System"] {
        let mut candidate = original.clone();
        candidate.package.profile = profile.into();
        assert!(compile_ail_core_bytecode(&candidate).is_err(), "{profile}");
    }
    for attribute in ["function_owner", "local_name"] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "Field" && node.name == "Data.text")
            .unwrap()
            .attributes
            .insert(attribute.into(), "private".into());
        assert!(
            compile_ail_core_bytecode(&candidate).is_err(),
            "{attribute}"
        );
    }
    let mut candidate = original.clone();
    candidate
        .graph
        .nodes
        .iter_mut()
        .find(|node| node.kind == "Action")
        .unwrap()
        .attributes
        .insert("kind".into(), "CompilerPass".into());
    assert!(compile_ail_core_bytecode(&candidate).is_err());
    let program = compile_ail_core_bytecode(&original).unwrap();
    for opcode in ["FILE_READ_TEXT", "FILE_WRITE_TEXT"] {
        for mutation in ["resource", "extra", "missing", "field"] {
            let mut broken = program.clone();
            let instruction = broken
                .actions
                .get_mut("Run")
                .unwrap()
                .instructions
                .iter_mut()
                .find(|instruction| instruction.opcode == opcode)
                .unwrap();
            match mutation {
                "resource" => {
                    instruction
                        .operands
                        .insert("resource".into(), "a..b".into());
                }
                "extra" => {
                    instruction
                        .operands
                        .insert("path".into(), "/tmp/ambient".into());
                }
                "missing" => {
                    instruction.operands.remove("resource");
                }
                _ => {
                    instruction.operands.insert(
                        if opcode == "FILE_READ_TEXT" {
                            "key"
                        } else {
                            "source"
                        }
                        .into(),
                        "not a field".into(),
                    );
                }
            }
            assert!(
                !verify_ail_bytecode(&broken).is_empty(),
                "{opcode}/{mutation}"
            );
            assert!(ail::ste::render_ail_ste(&broken).is_err());
            assert!(run_ail_bytecode_action(&broken, "Run", BTreeMap::new()).is_err());
            assert!(compile_ail_bytecode_native_elf(&broken, "Run", "linux-x86_64-elf").is_err());
        }
    }
}

#[test]
fn source_rejects_bad_resource_names_wrong_types_and_compiler_pass_effects() {
    for name in [
        "/tmp/input".to_string(),
        "a..b".into(),
        "a-b".into(),
        ".input".into(),
        "9input".into(),
        "é".into(),
        "a".repeat(257),
    ] {
        let text = source(&format!(
            "- the system reads text from file resource {name} into data.text"
        ));
        let package = package();
        match parse_ail_package_spec_text(&package, &text) {
            Err(_) => {}
            Ok(document) => assert!(
                compile_ail_core_bytecode(&elaborate_ail_core(&package, &document)).is_err(),
                "{name}"
            ),
        }
    }
    for body in [
        "- the system reads text from file resource input into data.count",
        "- the system writes text data.count to file resource output",
        "- the system reads text from file resource input into missing.text",
    ] {
        let package = package();
        match parse_ail_package_spec_text(&package, &source(body)) {
            Err(_) => {}
            Ok(document) => assert!(
                compile_ail_core_bytecode(&elaborate_ail_core(&package, &document)).is_err(),
                "{body}"
            ),
        }
    }
    let mut package = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    package.spec_text = package.spec_text.replace(
        "- the system reads every edge whose kind is reads",
        "- the system reads text from file resource input into data.text",
    );
    assert!(parse_ail_package_document(&package).is_err());
}

#[test]
fn semantic_file_operands_enforce_secret_reads_and_writes_without_descriptive_edges() {
    for (body, code) in [
        (
            "- the system reads text from file resource input into data.text",
            "AIL002",
        ),
        (
            "- the system writes text data.text to file resource output",
            "AIL005",
        ),
    ] {
        let mut candidate = core(&source(body));
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "Field" && node.name == "Data.text")
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        candidate
            .graph
            .edges
            .retain(|edge| !matches!(edge.kind.as_str(), "reads" | "writes"));
        let saved = parse_ail_core_text(&render_ail_core(&candidate)).unwrap();
        assert!(
            check_ail_core(&saved)
                .iter()
                .any(|diagnostic| diagnostic.contains(code)),
            "{:?}",
            check_ail_core(&saved)
        );
        assert!(compile_ail_core_bytecode(&saved).is_err());
        let owner = candidate
            .graph
            .nodes
            .iter()
            .find(|node| node.kind == "Action")
            .unwrap()
            .clone();
        let field = candidate
            .graph
            .nodes
            .iter()
            .find(|node| node.kind == "Field" && node.name == "Data.text")
            .unwrap()
            .clone();
        candidate
            .graph
            .add_edge("protects_secret", &owner, &field, BTreeMap::new());
        assert!(
            check_ail_core(&candidate).is_empty(),
            "{:?}",
            check_ail_core(&candidate)
        );
    }
}

#[test]
fn file_errors_reject_nonregular_paths_and_do_not_create_parent_directories() {
    let scratch = Scratch::new();
    let directory = scratch.path("directory");
    fs::create_dir(&directory).unwrap();
    let missing = scratch.path("missing");
    let nested = scratch.path("absent/child.txt");
    for (body, read) in [
        (
            "- the system reads text from file resource file into data.text",
            true,
        ),
        (
            "- the system writes text data.text to file resource file",
            false,
        ),
    ] {
        let program = compiled(body);
        let binary = scratch.binary(&program, "Run");
        for path in if read {
            vec![
                directory.as_path(),
                missing.as_path(),
                Path::new("/dev/null"),
            ]
        } else {
            vec![
                directory.as_path(),
                nested.as_path(),
                Path::new("/dev/null"),
            ]
        } {
            let grants = if read {
                resources(&[("file", path)], &[])
            } else {
                resources(&[], &[("file", path)])
            };
            let values = state(&[("data.text", "value")]);
            let failed =
                run_ail_bytecode_action_with_files(&program, "Run", values.clone(), &grants)
                    .unwrap_err();
            assert!(failed.contains("AIL-RUNTIME-IO"), "{failed}");
            error(&native(&binary, &grants, &values), "AIL-RUNTIME-IO");
        }
    }
    assert!(!scratch.path("absent").exists());
}

#[test]
fn fifo_bindings_fail_promptly_in_vm_cli_and_native() {
    let scratch = Scratch::new();
    let fifo = scratch.path("fifo");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    for read in [true, false] {
        let program = compiled(if read {
            "- the system reads text from file resource file into data.text"
        } else {
            "- the system writes text data.text to file resource file"
        });
        let bytecode = scratch.file("program.json", render_ail_bytecode(&program).as_bytes());
        let grants = if read {
            resources(&[("file", &fifo)], &[])
        } else {
            resources(&[], &[("file", &fifo)])
        };
        let result = native(
            &scratch.binary(&program, "Run"),
            &grants,
            &state(&[("data.text", "value")]),
        );
        error(&result, "AIL-RUNTIME-IO");
        let result = command_output(Command::new(env!("CARGO_BIN_EXE_ail")).args([
            "ail-vm",
            bytecode.to_str().unwrap(),
            "--action",
            "Run",
            &format!(
                "--{}-file=file={}",
                if read { "read" } else { "write" },
                fifo.display()
            ),
            "data.text=value",
        ]));
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("AIL-RUNTIME-IO"),
            "{result:?}"
        );
    }
}

#[test]
fn file_reads_charge_shared_text_budget_across_action_calls() {
    let scratch = Scratch::new();
    let input = scratch.file("input.txt", &vec![b'x'; 1_048_576]);
    let grants = resources(&[("input", &input)], &[]);
    for extra in [false, true] {
        let body = format!(
            "- the system calls Worker\n- the system calls Worker\n{}- the system returns data.count",
            if extra {
                "- the system reads text from file resource input into data.text\n"
            } else {
                ""
            }
        );
        let text = format!(
            "{}\nAction: Worker.\nWhen Worker happens:\n- the system assigns 0 to data.count\n- while data.count < 32:\n- the system reads text from file resource input into data.text\n- the system computes data.count as data.count + 1\n- end while\n- the system records a trace event named ReadBatch\n",
            source(&body)
        );
        let program = compile_ail_core_bytecode(&core(&text)).unwrap();
        let vm = run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants);
        let result = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
        if extra {
            assert!(vm.unwrap_err().contains("AIL-RUNTIME-TEXT-LIMIT"));
            error(&result, "AIL-RUNTIME-TEXT-LIMIT");
        } else {
            assert_eq!(vm.unwrap().output_state(), state(&[("data.count", "32")]));
            assert!(result.status.success(), "{result:?}");
            assert_eq!(result.stdout, b"data.count=32\n");
        }
    }
}

#[test]
fn direct_bytecode_file_operations_reject_unsupported_profiles_before_effects() {
    let original = compile_ail_core_bytecode(&core(&source(
        "- the system assigns \"test\" to data.text\n- the system writes text data.text to file resource output",
    ))).unwrap();
    for profile in ["Compiler", "System", "UI", "AgentTool", "unknown"] {
        let mut candidate = original.clone();
        candidate.profile = profile.into();
        assert!(
            verify_ail_bytecode(&candidate)
                .iter()
                .any(|d| d.contains("AIL-FILE-001")),
            "{profile}"
        );
        assert!(compile_ail_bytecode_native_elf(&candidate, "Run", "linux-x86_64-elf").is_err());
        assert!(ail::ste::render_ail_ste(&candidate).is_err());
        let direct = ail::ste::render_ail_ste(&original).unwrap().replace(
            "\"profile\":\"Application\"",
            &format!("\"profile\":\"{profile}\""),
        );
        assert!(ail::ste::parse_ail_ste(&direct).is_err());
        assert!(
            run_ail_bytecode_action(&candidate, "Run", BTreeMap::new())
                .unwrap_err()
                .contains("AIL-FILE-001")
        );
    }
    for profile in ["Application", "C interop"] {
        let mut candidate = original.clone();
        candidate.profile = profile.into();
        assert!(verify_ail_bytecode(&candidate).is_empty(), "{profile}");
    }
}

#[test]
fn vm_api_validates_every_file_binding_before_effects() {
    let scratch = Scratch::new();
    let output = scratch.file("output.txt", b"original");
    let program = compile_ail_core_bytecode(&core(&source(
        "- the system assigns \"replacement\" to data.text\n- the system writes text data.text to file resource output",
    ))).unwrap();
    for writing in [false, true] {
        for (name, path) in [
            ("bad name", "unused"),
            ("unused", ""),
            ("unused", "a\0b"),
            ("", "unused"),
        ] {
            let mut grants = resources(&[], &[("output", &output)]);
            let map = if writing {
                &mut grants.writes
            } else {
                &mut grants.reads
            };
            map.insert(name.into(), PathBuf::from(path));
            let result =
                run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants);
            assert!(
                result
                    .as_ref()
                    .is_err_and(|e| e.contains("AIL-RUNTIME-CAPABILITY")),
                "{writing}, {name:?}, {path:?}: {result:?}"
            );
            assert_eq!(fs::read(&output).unwrap(), b"original");
        }
    }
}
