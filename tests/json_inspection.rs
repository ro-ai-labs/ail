use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application Inspection manages records.\n\nA Data has:\n- document: Json\n- copy: Json\n- path: Text\n- raw: Text\n- index: Int\n\nAn Output has:\n- kind: Text\n- keys: List<Text>\n- raw: Text\n- count: Int\n\n";

fn package() -> AilPackage {
    load_ail_package_dir("bench/R001-add/program.ail").unwrap()
}

fn source(body: &str) -> String {
    format!(
        "{DECLARATIONS}Action: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Values\n"
    )
}

fn core(text: &str) -> AilCore {
    let package = package();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, text).unwrap(),
    )
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "ail-json-inspection-{}-{}",
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
                "JSON inspection blocked: {}",
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

fn brief(output: &Output) -> String {
    format!(
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout[..output.stdout.len().min(500)]),
        String::from_utf8_lossy(&output.stderr[..output.stderr.len().min(1_500)])
    )
}

fn failed(output: &Output, category: &str) {
    assert!(!output.status.success(), "{}", brief(output));
    assert!(output.stdout.is_empty(), "{}", brief(output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(category),
        "expected {category}: {}",
        brief(output)
    );
}

fn command(opcode: &str, operands: &[(&str, &str)]) -> AilBytecodeInstruction {
    AilBytecodeInstruction {
        opcode: opcode.into(),
        operands: state(operands),
    }
}

fn raw_program(instructions: Vec<AilBytecodeInstruction>) -> AilBytecodeProgram {
    let mut program =
        compile_ail_core_bytecode(&core(&source("- the system assigns 0 to output.count")))
            .unwrap();
    let code = &mut program.actions.get_mut("Run").unwrap().instructions;
    *code = instructions;
    code.push(command("RETURN_SUCCESS", &[]));
    program
}

fn with_functions(functions: &str, body: &str) -> String {
    format!(
        "{DECLARATIONS}{functions}\nAction: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Finished\n"
    )
}

#[test]
fn json_kind_and_keys_run_through_source_saved_core_rendered_source_ste_vm_and_native() {
    let original = core(&source(
        "- the system gets JSON kind from data.document into output.kind\n- the system gets JSON keys from data.document into output.keys\n- the system counts items in list output.keys into output.count\n- the system returns output.kind\n- the system returns output.keys\n- the system returns output.count",
    ));
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let ste = compile_ail_core_ste(&candidate).unwrap();
        assert!(ste.contains("Get JSON kind from field"));
        assert!(ste.contains("Get JSON keys from field"));
        let program = ail::ste::parse_ail_ste(&ste).unwrap();
        let input = state(&[(
            "data.document",
            r#"{"b":null,"a":1e99999999999,"b":false,"":{},"\u0000":[],"é":1,"e\u0301":2,"\ud83d\ude42":3}"#,
        )]);
        let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert!(vm.trace.contains(&"got JSON kind output.kind".into()));
        assert!(vm.trace.contains(&"got JSON keys output.keys".into()));
        assert_eq!(vm.final_state["output.kind"], "object");
        assert_eq!(vm.final_state["output.count"], "8");
        assert_eq!(vm.final_state["data.document"], input["data.document"]);
        let expected = "[\"b\",\"a\",\"b\",\"\",\"\\u0000\",\"é\",\"é\",\"🙂\"]";
        assert_eq!(vm.final_state["output.keys"], expected);
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(
            elf.stdout,
            format!("output.count=8\noutput.keys={expected}\noutput.kind=object\n").as_bytes()
        );
    }
}

fn seed(key: &str, value: &str) -> AilBytecodeInstruction {
    command(
        "SET_FIELD",
        &[("key", key), ("value", value), ("text", "seed")],
    )
}

#[test]
fn all_six_kinds_classify_lossless_numbers_without_float_dependencies() {
    let program = raw_program(vec![
        command(
            "JSON_KIND",
            &[("key", "out.kind"), ("source", "input.json")],
        ),
        command("OUTPUT_FIELD", &[("key", "out.kind")]),
    ]);
    let scratch = Scratch::new();
    let binary = scratch.binary(&program, "Run");
    let elf = fs::read(&binary).unwrap();
    let phoff = u64::from_le_bytes(elf[32..40].try_into().unwrap()) as usize;
    let phsize = u16::from_le_bytes(elf[54..56].try_into().unwrap()) as usize;
    let phnum = u16::from_le_bytes(elf[56..58].try_into().unwrap()) as usize;
    for n in 0..phnum {
        let start = phoff + n * phsize;
        let address = u64::from_le_bytes(elf[start + 16..start + 24].try_into().unwrap());
        assert!(
            !(0x10000000..0x20000000).contains(&address),
            "inspection linked the Float runtime"
        );
    }
    for (raw, kind) in [
        ("null", "null"),
        ("true", "boolean"),
        ("false", "boolean"),
        ("-0.000e99999999999999999999", "number"),
        ("1234567890123456789012345678901234567890", "number"),
        (r#""\u0000\ud83d\ude42""#, "string"),
        ("[null, {\"a\":true}]", "array"),
        ("{}", "object"),
    ] {
        let input = state(&[("input.json", raw)]);
        let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(vm.final_state["out.kind"], kind);
        assert_eq!(vm.final_state["input.json"], raw);
        let elf = native(&binary, &resources(&[], &[]), &input);
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(elf.stdout, format!("out.kind={kind}\n").as_bytes());
    }
}

#[test]
fn whole_source_validation_precedes_root_type_and_preserves_error_privacy() {
    let scratch = Scratch::new();
    for opcode in ["JSON_KIND", "JSON_KEYS"] {
        let program = raw_program(vec![command(
            opcode,
            &[("key", "out.value"), ("source", "input.json")],
        )]);
        let binary = scratch.binary(&program, "Run");
        for raw in [
            "",
            "null trailing",
            "[1,]",
            "{\"ok\":0,\"hidden\":PRIVATE_VALUE}",
            "{\"ok\":0,\"hidden\":\"\\ud800\"}",
            "{\"x\":1,}",
            "[NaN]",
            "{\"x\":01}",
            "{\"x\":true}{}",
        ] {
            let input = state(&[("input.json", raw)]);
            let error = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap_err();
            assert!(error.contains("AIL-RUNTIME-JSON:"), "{opcode}: {error}");
            assert!(!error.contains("PRIVATE_VALUE"));
            let elf = native(&binary, &resources(&[], &[]), &input);
            failed(&elf, "AIL-RUNTIME-JSON:");
            assert!(!String::from_utf8_lossy(&elf.stderr).contains("PRIVATE_VALUE"));
        }
        failed(
            &native(&binary, &resources(&[], &[]), &BTreeMap::new()),
            "AIL-RUNTIME-JSON:",
        );
    }
    let keys = raw_program(vec![command(
        "JSON_KEYS",
        &[("key", "out.keys"), ("source", "input.json")],
    )]);
    let binary = scratch.binary(&keys, "Run");
    for raw in ["null", "true", "0", "\"object\"", "[]", "[{}]"] {
        let input = state(&[("input.json", raw)]);
        assert!(
            run_ail_bytecode_action(&keys, "Run", input.clone())
                .unwrap_err()
                .contains("AIL-RUNTIME-JSON:")
        );
        failed(
            &native(&binary, &resources(&[], &[]), &input),
            "AIL-RUNTIME-JSON:",
        );
    }
}

#[test]
fn key_results_and_aliases_survive_later_parses_and_function_frame_release() {
    let original = core(&with_functions(
        "Function: keys.\nThe function needs:\n- document: Json\nThe function produces:\n- result: List<Text>\nThe function stores:\n- result: List<Text>\nThe function executes:\n- the function gets JSON keys from local.document into local.result\n- the function returns local.result\nFunction: kind.\nThe function needs:\n- document: Json\nThe function produces:\n- result: Text\nThe function stores:\n- result: Text\nThe function executes:\n- the function gets JSON kind from local.document into local.result\n- the function returns local.result",
        "- the system calls keys with document = data.document into output.keys\n- the system assigns [false, 1e999999, {\"junk\":null}] to data.copy\n- the system calls kind with document = data.copy into output.kind\n- the system counts items in list output.keys into output.count\n- the system returns output.keys\n- the system returns output.kind\n- the system returns output.count",
    ));
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let input = state(&[("data.document", r#"{"\u0000":1,"a/b~c":2,"é":3}"#)]);
        let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(
            vm.final_state["output.keys"],
            "[\"\\u0000\",\"a/b~c\",\"é\"]"
        );
        assert!(!vm.final_state.keys().any(|k| k.starts_with("local.")));
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(
            elf.stdout,
            "output.count=3\noutput.keys=[\"\\u0000\",\"a/b~c\",\"é\"]\noutput.kind=array\n"
                .as_bytes()
        );
    }
    for (opcode, expected) in [("JSON_KEYS", r#"["old","old"]"#), ("JSON_KIND", "object")] {
        let program = raw_program(vec![
            seed("data.json", r#"{"old":0,"old":1}"#),
            command(
                "COPY_FIELD",
                &[
                    ("key", "data.copy"),
                    ("source", "data.json"),
                    ("text", "snapshot"),
                ],
            ),
            command(opcode, &[("key", "data.json"), ("source", "data.json")]),
            command("JSON_KEYS", &[("key", "out.keys"), ("source", "data.copy")]),
            command("OUTPUT_FIELD", &[("key", "data.json")]),
            command("OUTPUT_FIELD", &[("key", "out.keys")]),
        ]);
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(vm.final_state["data.json"], expected);
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(
            elf.stdout,
            format!("data.json={expected}\nout.keys=[\"old\",\"old\"]\n").as_bytes()
        );
    }
}

#[test]
fn enumeration_preserves_duplicate_keys_but_lookup_remains_ambiguous() {
    let program = raw_program(vec![
        command(
            "JSON_KEYS",
            &[("key", "out.keys"), ("source", "input.json")],
        ),
        command(
            "JSON_GET",
            &[
                ("key", "out.value"),
                ("source", "input.json"),
                ("path", "input.path"),
                ("type", "Json"),
            ],
        ),
    ]);
    let input = state(&[
        ("input.json", r#"{"a":1,"\u0061":2}"#),
        ("input.path", "/a"),
    ]);
    assert!(
        run_ail_bytecode_action(&program, "Run", input.clone())
            .unwrap_err()
            .contains("AIL-RUNTIME-JSON-PATH:")
    );
    let scratch = Scratch::new();
    failed(
        &native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        ),
        "AIL-RUNTIME-JSON-PATH:",
    );
}

#[test]
fn inspection_enforces_raw_canonical_depth_and_value_limits_before_root_type() {
    let scratch = Scratch::new();
    let full_object = format!("{{{}}}", vec!["\"k\":0"; 65_535].join(","));
    let cases = vec![
        ("raw exact", format!("{{}}{}", " ".repeat(1_048_574)), true),
        ("raw over", format!("{{}}{}", " ".repeat(1_048_575)), false),
        (
            "depth exact",
            format!("{}0{}", "[".repeat(64), "]".repeat(64)),
            true,
        ),
        (
            "depth over",
            format!("{}0{}", "[".repeat(65), "]".repeat(65)),
            false,
        ),
        ("members exact", full_object, true),
        (
            "members over",
            format!("{{{}}}", vec!["\"k\":0"; 65_536].join(",")),
            false,
        ),
        (
            "hidden canonical over",
            format!("{{\"hidden\":\"{}\"}}", "\u{85}".repeat(174_763)),
            false,
        ),
    ];
    for (name, raw, valid) in cases {
        for opcode in ["JSON_KIND", "JSON_KEYS"] {
            let expected = if !valid {
                Some("JSON-LIMIT")
            } else if opcode == "JSON_KEYS" && raw.starts_with('[') {
                Some("JSON")
            } else {
                None
            };
            let mut steps = vec![
                seed("input.json", &raw),
                command(opcode, &[("source", "input.json"), ("key", "out.value")]),
            ];
            if opcode == "JSON_KEYS" {
                steps.push(command(
                    "LIST_LENGTH",
                    &[
                        ("source", "out.value"),
                        ("key", "out.count"),
                        ("type", "Text"),
                    ],
                ));
                steps.push(command("OUTPUT_FIELD", &[("key", "out.count")]));
            } else {
                steps.push(command("OUTPUT_FIELD", &[("key", "out.value")]));
            }
            let program = raw_program(steps);
            let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new());
            let elf = native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &BTreeMap::new(),
            );
            if let Some(category) = expected {
                assert!(
                    vm.unwrap_err()
                        .contains(&format!("AIL-RUNTIME-{category}:")),
                    "{name} {opcode}"
                );
                failed(&elf, &format!("AIL-RUNTIME-{category}:"));
            } else {
                assert!(vm.is_ok(), "{name} {opcode}: {vm:?}");
                assert!(elf.status.success(), "{name} {opcode}: {}", brief(&elf));
                if name == "members exact" && opcode == "JSON_KEYS" {
                    assert_eq!(elf.stdout, b"out.count=65535\n");
                }
            }
        }
    }
}

#[test]
fn exact_protocol_profiles_and_core_roles_reject_forgery_before_execution() {
    for opcode in ["JSON_KIND", "JSON_KEYS"] {
        let original = raw_program(vec![command(
            opcode,
            &[("source", "data.document"), ("key", "output.kind")],
        )]);
        let ste = ail::ste::render_ail_ste(&original).unwrap();
        for (key, value) in [
            ("path", "data.path"),
            ("type", "Text"),
            ("source", "not a field"),
            ("key", "bare"),
            ("ignored", "metadata"),
        ] {
            let mut program = original.clone();
            program.actions.get_mut("Run").unwrap().instructions[0]
                .operands
                .insert(key.into(), value.into());
            assert!(!verify_ail_bytecode(&program).is_empty());
            assert!(ail::ste::render_ail_ste(&program).is_err());
            assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
        }
        for profile in ["Compiler", "UI", "AgentTool", "System"] {
            let mut program = original.clone();
            program.profile = profile.into();
            assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
            assert!(
                ail::ste::parse_ail_ste(&ste.replace("\"Application\"", &format!("\"{profile}\"")))
                    .is_err()
            );
        }
    }
    let original = core(&source(
        "- the system gets JSON keys from data.document into output.keys\n- the system gets JSON kind from data.document into output.kind",
    ));
    for kind in [
        "json_access_step",
        "json_access_source",
        "json_access_destination",
    ] {
        let mut candidate = original.clone();
        let mut edge = candidate
            .graph
            .edges
            .iter()
            .find(|e| e.kind == kind)
            .unwrap()
            .clone();
        edge.id = format!("duplicate-{kind}");
        edge.ordinal = candidate.graph.edges.len();
        candidate.graph.edges.push(edge);
        assert!(compile_ail_core_bytecode(&candidate).is_err(), "{kind}");
    }
    for (key, value) in [
        ("value_type", "Json"),
        ("json_access_kind", "count"),
        ("ignored", "opaque"),
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "JsonAccess")
            .unwrap()
            .attributes
            .insert(key.into(), value.into());
        assert!(compile_ail_core_ste(&candidate).is_err(), "{key}");
    }
    let mut path = original.clone();
    let access = path
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "JsonAccess")
        .unwrap()
        .clone();
    let field = path
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Field" && n.name == "Data.path")
        .unwrap()
        .clone();
    path.graph
        .add_edge("json_access_path", &access, &field, BTreeMap::new());
    assert!(compile_ail_core_bytecode(&path).is_err());
}

#[test]
fn semantic_secret_operands_types_and_descriptive_body_guards_are_enforced() {
    let original = core(&source(
        "- the system gets JSON keys from data.document into output.keys\n- the system gets JSON kind from data.document into output.kind",
    ));
    for (name, code) in [
        ("Data.document", "AIL005"),
        ("Output.kind", "AIL002"),
        ("Output.keys", "AIL002"),
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name == name)
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        candidate
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        let candidate = parse_ail_core_text(&render_ail_core(&candidate)).unwrap();
        assert!(
            check_ail_core(&candidate).iter().any(|d| d.contains(code)),
            "{name}: {:?}",
            check_ail_core(&candidate)
        );
    }
    for statement in [
        "the system gets JSON kind from data.raw into output.kind",
        "the system gets JSON kind from data.document into output.keys",
        "the system gets JSON keys from data.document into output.kind",
    ] {
        assert!(
            compile_ail_core_bytecode(&core(&source(&format!("- {statement}")))).is_err(),
            "{statement}"
        );
    }
    let application = package();
    let compiler = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for suffix in [
        "gets JSON kind from local.document into local.result",
        "gets JSON keys from local.document into local.result",
        "gets JSON kind",
        "gets JSON keys from",
    ] {
        for subject in ["the function", "the system"] {
            let statement = format!("{subject} {suffix}");
            let legacy = format!(
                "{DECLARATIONS}Function: legacy.\nThe function needs:\n- document: Json\nThe function produces:\n- result: Text\nWhen legacy runs:\n- {statement}\n- the function returns document unchanged\n"
            );
            let pass = compiler.spec_text.replace(
                "- the system reads every edge",
                &format!("- {statement}\n- the system reads every edge"),
            );
            for (package, text) in [(&application, legacy), (&compiler, pass)] {
                assert!(
                    parse_ail_package_spec_text(package, &text).is_err(),
                    "descriptive body accepted {statement}"
                );
            }
        }
    }
}

#[test]
fn imported_inspection_functions_qualify_globals_and_preserve_local_operands() {
    let mut child = package();
    child.metadata.name = "inspection-child".into();
    child.spec_text = with_functions(
        "Function: inspect.\nThe function needs:\n- document: Json\nThe function produces:\n- result: List<Text>\nThe function stores:\n- result: List<Text>\nThe function executes:\n- the function gets JSON keys from local.document into local.result\n- the function returns local.result",
        "- the system calls inspect with document = data.document into output.keys\n- the system gets JSON kind from data.document into output.kind\n- the system returns output.keys\n- the system returns output.kind",
    );
    let mut parent = package();
    parent.spec_text="The application Parent manages records.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system returns child.output.keys\n- the system returns child.output.kind\n- the system records a trace event named Done\n".into();
    parent.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Child".into(),
            resolved_package: None,
            registry_identity: None,
        },
        package: Box::new(child),
    });
    let core = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    let program = ail::ste::parse_ail_ste(
        &compile_ail_core_ste(&parse_ail_core_text(&render_ail_core(&core)).unwrap()).unwrap(),
    )
    .unwrap();
    let input = state(&[(
        "child.data.document",
        r#"{"local.document":null,"data.document":0}"#,
    )]);
    let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
    assert_eq!(
        vm.final_state["child.output.keys"],
        r#"["local.document","data.document"]"#
    );
    let scratch = Scratch::new();
    let elf = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(elf.status.success(), "{}", brief(&elf));
    assert_eq!(
        elf.stdout,
        b"child.output.keys=[\"local.document\",\"data.document\"]\nchild.output.kind=object\n"
    );
}

#[test]
fn file_inspection_writes_decoded_ordered_keys_as_json() {
    let program=compile_ail_core_bytecode(&core(&source("- the system reads text from file resource input into data.raw\n- the system parses JSON text data.raw into data.document\n- the system gets JSON keys from data.document into output.keys\n- the system formats output.keys as JSON text into output.raw\n- the system writes text output.raw to file resource report\n- the system counts items in list output.keys into output.count\n- the system returns output.count"))).unwrap();
    let scratch = Scratch::new();
    let input = scratch.file(
        "input file=json",
        br#"{"\u0000":null,"a/b~c":[],"\ud83d\ude42":{},"\u0000":1e9999}"#,
    );
    let report = scratch.path("report file=json");
    let grants = resources(&[("input", &input)], &[("report", &report)]);
    run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
    let expected = "[\"\\u0000\",\"a/b~c\",\"🙂\",\"\\u0000\"]";
    assert_eq!(fs::read(&report).unwrap(), expected.as_bytes());
    let elf = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
    assert!(elf.status.success(), "{}", brief(&elf));
    assert_eq!(elf.stdout, b"output.count=4\n");
    assert_eq!(fs::read(&report).unwrap(), expected.as_bytes());
}
