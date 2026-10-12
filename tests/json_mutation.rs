use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application Mutation manages records.\n\nA Data has:\n- raw: Text\n- path: Text\n- document: Json\n- copy: Json\n- count: Int\n- index: Int\n- item: Int\n- total: Int\n- text: Text\n- flag: Bool\n- values: List<Int>\n- names: List<Text>\n\nAn Output has:\n- text: Text\n- value: Int\n- count: Int\n- document: Json\n\n";

fn package() -> AilPackage {
    load_ail_package_dir("bench/R001-add/program.ail").unwrap()
}

fn source(body: &str) -> String {
    format!(
        "{DECLARATIONS}Action: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Mutation\n"
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
            "ail-json-mutation-{}-{}",
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
                "structured JSON operation blocked: {}",
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

#[test]
fn source_mutation_preserves_order_across_core_ste_vm_and_native() {
    let scratch = Scratch::new();
    let original = core(&source(
        "- the system assigns {\"keep\":1e+0,\"old\":0} to data.document\n- the system assigns \"/old\" to data.path\n- the system removes JSON value at path data.path from data.document into data.copy\n- the system assigns 7 to data.item\n- the system assigns \"/new\" to data.path\n- the system sets JSON value at path data.path in data.copy to data.item into output.document\n- the system returns output.document\n- the system returns data.document",
    ));
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let instructions = &program.actions["Run"].instructions;
        assert!(instructions.iter().any(|i| i.opcode == "JSON_REMOVE"));
        assert!(instructions.iter().any(|i| i.opcode == "JSON_SET"));
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(
            result.final_state["output.document"],
            "{\"keep\":1e+0,\"new\":7}"
        );
        assert_eq!(
            result.final_state["data.document"],
            "{\"keep\":1e+0,\"old\":0}"
        );
        assert!(result.trace.contains(&"removed JSON data.copy".into()));
        assert!(result.trace.contains(&"set JSON output.document".into()));
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(
            output.stdout,
            b"data.document={\"keep\":1e+0,\"old\":0}\noutput.document={\"keep\":1e+0,\"new\":7}\n"
        );
    }
}

fn set(type_name: &str) -> AilBytecodeInstruction {
    command(
        "JSON_SET",
        &[
            ("key", "out.document"),
            ("source", "input.document"),
            ("path", "input.path"),
            ("value", "input.value"),
            ("type", type_name),
        ],
    )
}

fn remove() -> AilBytecodeInstruction {
    command(
        "JSON_REMOVE",
        &[
            ("key", "out.document"),
            ("source", "input.document"),
            ("path", "input.path"),
        ],
    )
}

fn seed(key: &str, value: &str) -> AilBytecodeInstruction {
    command(
        "SET_FIELD",
        &[
            ("key", key),
            ("value", value),
            ("text", "JSON mutation fixture"),
        ],
    )
}

#[test]
fn all_six_value_types_use_format_transport_rules_and_source_type_declarations() {
    let scratch = Scratch::new();
    for (field, raw, expected) in [
        ("data.item", "+00042", "42"),
        ("data.flag", "true", "true"),
        ("data.text", "é🙂\0\u{85}", "\"é🙂\\u0000\\u0085\""),
        (
            "data.values",
            " [ 1,-0,-9223372036854775808 ] ",
            "[1,0,-9223372036854775808]",
        ),
        (
            "data.names",
            r#"["\ud83d\ude42","\u0000","\u007f"]"#,
            "[\"🙂\",\"\\u0000\",\"\\u007f\"]",
        ),
        (
            "data.copy",
            r#"{"n":-0.00E+09,"n":9999999999999999999999999999}"#,
            r#"{"n":-0.00E+09,"n":9999999999999999999999999999}"#,
        ),
    ] {
        let original = core(&source(&format!(
            "- the system sets JSON value at path data.path in data.document to {field} into output.document\n- the system returns output.document"
        )));
        for candidate in [
            original.clone(),
            parse_ail_core_text(&render_ail_core(&original)).unwrap(),
            core(&render_ail_spec_from_core(&original)),
        ] {
            let mut program =
                ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
            program
                .actions
                .get_mut("Run")
                .unwrap()
                .instructions
                .insert(0, seed(field, raw));
            let input = state(&[
                (
                    "data.document",
                    "{\"before\":1e0,\"value\":null,\"after\":-0}",
                ),
                ("data.path", "/value"),
            ]);
            let expected = format!("{{\"before\":1e0,\"value\":{expected},\"after\":-0}}");
            let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
            assert_eq!(result.final_state["output.document"], expected);
            let output = native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &input,
            );
            assert!(output.status.success(), "{}", brief(&output));
            assert_eq!(
                output.stdout,
                format!("output.document={expected}\n").as_bytes()
            );
        }
    }
}

#[test]
fn set_remove_pointer_rules_preserve_pairs_numbers_and_array_order() {
    let scratch = Scratch::new();
    for (operation, raw, path, value, expected) in [
        (
            "set",
            "{\"a\":1,\"b\":2,\"a\":3}",
            "/b",
            "9",
            "{\"a\":1,\"b\":9,\"a\":3}",
        ),
        (
            "remove",
            "{\"a\":1,\"b\":2,\"a\":3}",
            "/b",
            "null",
            "{\"a\":1,\"a\":3}",
        ),
        ("set", "{\"b\":2}", "/a", "1", "{\"b\":2,\"a\":1}"),
        ("set", "{}", "/", "1", "{\"\":1}"),
        ("set", "{}", "/-", "1", "{\"-\":1}"),
        ("set", "{}", "/~01", "1", "{\"~1\":1}"),
        ("set", "{}", "/\0", "1", "{\"\\u0000\":1}"),
        (
            "set",
            "{\"a/b\":{\"~k\":[0,1]}}",
            "/a~1b/~0k/-",
            "2",
            "{\"a/b\":{\"~k\":[0,1,2]}}",
        ),
        (
            "set",
            "[1e0,-0,3]",
            "/1",
            "[true,null]",
            "[1e0,[true,null],3]",
        ),
        ("remove", "[1e0,-0,3]", "/1", "null", "[1e0,3]"),
        ("remove", "[1]", "/0", "null", "[]"),
        ("set", "null", "", "[1,2]", "[1,2]"),
        ("set", "{\"unused\":1}", "", "null", "null"),
        (
            "remove",
            "{\"é\":1,\"e\\u0301\":2}",
            "/é",
            "null",
            "{\"é\":2}",
        ),
    ] {
        let program = raw_program(vec![
            seed("input.path", path),
            seed("input.value", value),
            if operation == "set" {
                set("Json")
            } else {
                remove()
            },
            command("OUTPUT_FIELD", &[("key", "out.document")]),
        ]);
        let input = state(&[("input.document", raw)]);
        let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(
            result.final_state["out.document"], expected,
            "{operation}/{raw}/{path}"
        );
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(
            output.stdout,
            format!("out.document={expected}\n").as_bytes()
        );
    }
}

#[test]
fn malformed_source_path_target_and_value_errors_follow_required_priority() {
    let scratch = Scratch::new();
    for (operation, raw, path, value, category) in [
        ("set", "bad", "/~3", "bad", "AIL-RUNTIME-JSON:"),
        (
            "set",
            "{}",
            "/missing/child",
            "bad",
            "AIL-RUNTIME-JSON-PATH:",
        ),
        (
            "set",
            "{}",
            "/good/later~3",
            "bad",
            "AIL-RUNTIME-JSON-PATH:",
        ),
        ("set", "{}", "#", "bad", "AIL-RUNTIME-JSON-PATH:"),
        ("set", "[]", "/0", "bad", "AIL-RUNTIME-JSON-PATH:"),
        ("set", "[0]", "/01", "bad", "AIL-RUNTIME-JSON-PATH:"),
        ("set", "[0]", "/-1", "bad", "AIL-RUNTIME-JSON-PATH:"),
        (
            "set",
            "[0]",
            "/1844674407370955161600000",
            "bad",
            "AIL-RUNTIME-JSON-PATH:",
        ),
        ("set", "[]", "/-/x", "bad", "AIL-RUNTIME-JSON-PATH:"),
        ("set", "null", "/a", "bad", "AIL-RUNTIME-JSON-PATH:"),
        (
            "set",
            "{\"a\":0,\"\\u0061\":1}",
            "/a",
            "bad",
            "AIL-RUNTIME-JSON-PATH:",
        ),
        (
            "remove",
            "{\"a\":0,\"\\u0061\":1}",
            "/a",
            "null",
            "AIL-RUNTIME-JSON-PATH:",
        ),
        ("remove", "{}", "", "null", "AIL-RUNTIME-JSON-PATH:"),
        ("remove", "[0]", "/-", "null", "AIL-RUNTIME-JSON-PATH:"),
        ("remove", "[0]", "/1", "null", "AIL-RUNTIME-JSON-PATH:"),
        ("remove", "{}", "/missing", "null", "AIL-RUNTIME-JSON-PATH:"),
        ("set", "{}", "/new", "bad", "AIL-RUNTIME-JSON:"),
        ("set", "{}", "", "bad", "AIL-RUNTIME-JSON:"),
    ] {
        let program = raw_program(vec![
            if operation == "set" {
                set("Json")
            } else {
                remove()
            },
            command("OUTPUT_FIELD", &[("key", "out.document")]),
        ]);
        let input = state(&[
            ("input.document", raw),
            ("input.path", path),
            ("input.value", value),
            ("out.document", "preserved"),
        ]);
        let error = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap_err();
        assert!(
            error.contains(category),
            "{operation}/{raw}/{path}: {error}"
        );
        failed(
            &native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &input,
            ),
            category,
        );
    }
    let program = raw_program(vec![set("Json")]);
    let binary = scratch.binary(&program, "Run");
    for (path, category) in [
        ("/missing/child", "AIL-RUNTIME-JSON-PATH:"),
        ("/valid", "AIL-RUNTIME-JSON:"),
    ] {
        let input = state(&[("input.document", "{}"), ("input.path", path)]);
        assert!(
            run_ail_bytecode_action(&program, "Run", input.clone())
                .unwrap_err()
                .contains(category)
        );
        failed(&native(&binary, &resources(&[], &[]), &input), category);
    }
}

#[test]
fn destination_can_alias_source_path_or_value_without_mutating_copies() {
    let scratch = Scratch::new();
    for (destination, type_name, value, expected) in [
        (
            "input.document",
            "Json",
            "{\"n\":1e0}",
            "{\"x\":{\"n\":1e0}}",
        ),
        ("input.path", "Text", "é🙂", "{\"x\":\"é🙂\"}"),
        ("input.value", "Json", "[1,-0]", "{\"x\":[1,-0]}"),
    ] {
        let mut instruction = set(type_name);
        instruction
            .operands
            .insert("key".into(), destination.into());
        let program = raw_program(vec![
            command(
                "COPY_FIELD",
                &[
                    ("key", "out.copy"),
                    ("source", destination),
                    ("text", "immutable alias"),
                ],
            ),
            instruction,
            command(
                "COPY_FIELD",
                &[
                    ("key", "out.result"),
                    ("source", destination),
                    ("text", "result"),
                ],
            ),
            command("OUTPUT_FIELD", &[("key", "out.copy")]),
            command("OUTPUT_FIELD", &[("key", "out.result")]),
        ]);
        let input = state(&[
            ("input.document", "{\"x\":0}"),
            ("input.path", "/x"),
            ("input.value", value),
        ]);
        let previous = input[destination].clone();
        let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(result.final_state["out.copy"], previous);
        assert_eq!(result.final_state["out.result"], expected);
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(
            output.stdout,
            format!("out.copy={previous}\nout.result={expected}\n").as_bytes()
        );
    }
}

#[test]
fn json_mutation_core_protocol_requires_exact_types_profiles_ownership_and_metadata() {
    let original = core(&source(
        "- the system sets JSON value at path data.path in data.document to data.item into output.document\n- the system removes JSON value at path data.path from data.document into data.copy",
    ));
    for edge_kind in [
        "json_mutation_step",
        "json_mutation_source",
        "json_mutation_destination",
        "json_mutation_path",
        "json_mutation_value",
    ] {
        let mut candidate = original.clone();
        candidate.graph.edges.push(
            candidate
                .graph
                .edges
                .iter()
                .find(|edge| edge.kind == edge_kind)
                .unwrap()
                .clone(),
        );
        assert!(
            compile_ail_core_bytecode(&parse_ail_core_text(&render_ail_core(&candidate)).unwrap())
                .is_err(),
            "{edge_kind}"
        );
        let mut candidate = original.clone();
        candidate
            .graph
            .edges
            .iter_mut()
            .find(|edge| edge.kind == edge_kind)
            .unwrap()
            .attributes
            .insert("ignored".into(), "value".into());
        assert!(
            compile_ail_core_bytecode(&candidate).is_err(),
            "{edge_kind}"
        );
    }
    for (key, value) in [
        ("json_mutation_kind", "eval"),
        ("value_type", "Decimal"),
        ("ignored", "value"),
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "JsonMutation")
            .unwrap()
            .attributes
            .insert(key.into(), value.into());
        assert!(compile_ail_core_bytecode(&candidate).is_err(), "{key}");
    }
    let mut typed = original.clone();
    typed
        .graph
        .nodes
        .iter_mut()
        .find(|node| node.kind == "JsonMutation")
        .unwrap()
        .type_name = Some("Text".into());
    assert!(compile_ail_core_bytecode(&typed).is_err());
    for attribute in ["function_owner", "local_name"] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "Field" && node.name == "Data.document")
            .unwrap()
            .attributes
            .insert(attribute.into(), "private".into());
        assert!(compile_ail_core_bytecode(&candidate).is_err());
    }
    let mut compiler = original.clone();
    compiler
        .graph
        .nodes
        .iter_mut()
        .find(|node| node.kind == "Action")
        .unwrap()
        .attributes
        .insert("kind".into(), "CompilerPass".into());
    assert!(compile_ail_core_bytecode(&compiler).is_err());
    let valid = compile_ail_core_bytecode(&original).unwrap();
    for profile in ["Compiler", "AgentTool", "UI", "System"] {
        let mut candidate = original.clone();
        candidate.package.profile = profile.into();
        assert!(compile_ail_core_bytecode(&candidate).is_err());
        let mut program = valid.clone();
        program.profile = profile.into();
        assert!(!verify_ail_bytecode(&program).is_empty(), "{profile}");
        assert!(ail::ste::render_ail_ste(&program).is_err());
        assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
    }
    for opcode in ["JSON_SET", "JSON_REMOVE"] {
        for mutation in ["missing", "type", "extra", "field"] {
            let mut program = valid.clone();
            let instruction = program
                .actions
                .get_mut("Run")
                .unwrap()
                .instructions
                .iter_mut()
                .find(|i| i.opcode == opcode)
                .unwrap();
            match mutation {
                "missing" => {
                    instruction.operands.remove("source");
                }
                "type" => {
                    instruction
                        .operands
                        .insert("type".into(), "Map<Text>".into());
                }
                "extra" => {
                    instruction.operands.insert("literal".into(), "1".into());
                }
                _ => {
                    instruction
                        .operands
                        .insert("source".into(), "not a field".into());
                }
            }
            assert!(
                !verify_ail_bytecode(&program).is_empty(),
                "{opcode}/{mutation}"
            );
            assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
        }
    }
}

#[test]
fn source_rejects_mistyped_json_mutation_and_compiler_pass_statements() {
    for body in [
        "- the system sets JSON value at path data.item in data.document to data.item into output.document",
        "- the system removes JSON value at path data.path from data.raw into data.copy",
        "- the system sets JSON value at path data.path in data.document to missing.raw into output.document",
    ] {
        let package = package();
        match parse_ail_package_spec_text(&package, &source(body)) {
            Err(error) => assert!(error.contains("AIL-JSON-001"), "{error}"),
            Ok(document) => assert!(
                compile_ail_core_bytecode(&elaborate_ail_core(&package, &document)).is_err()
            ),
        }
    }
    for body in [
        "- the system sets JSON value at path data.path in data.document to data.item into output.document",
        "- the system removes JSON value at path data.path from data.document into data.copy",
    ] {
        let mut package = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
        package.spec_text = package
            .spec_text
            .replace("- the system reads every edge whose kind is reads", body);
        assert!(parse_ail_package_document(&package).is_err());
    }
}

#[test]
fn json_mutation_semantic_operands_protect_secret_reads_and_writes() {
    for (body, field, code) in [
        (
            "- the system sets JSON value at path data.path in data.document to data.item into output.document",
            "Data.item",
            "AIL005",
        ),
        (
            "- the system sets JSON value at path data.path in data.document to data.item into output.document",
            "Data.path",
            "AIL005",
        ),
        (
            "- the system sets JSON value at path data.path in data.document to data.item into output.document",
            "Data.document",
            "AIL005",
        ),
        (
            "- the system sets JSON value at path data.path in data.document to data.item into output.document",
            "Output.document",
            "AIL002",
        ),
        (
            "- the system removes JSON value at path data.path from data.document into data.copy",
            "Data.document",
            "AIL005",
        ),
        (
            "- the system removes JSON value at path data.path from data.document into data.copy",
            "Data.copy",
            "AIL002",
        ),
    ] {
        let mut candidate = core(&source(body));
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "Field" && node.name == field)
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
            .find(|node| node.kind == "Field" && node.name == field)
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
fn functions_reject_mutation_instead_of_demoting_it_to_prose() {
    for statement in [
        "the system sets JSON value at path data.path in data.document to data.item into output.document",
        "the system removes JSON value at path data.path from data.document into data.copy",
    ] {
        for executable in [false, true] {
            let text = format!(
                "{DECLARATIONS}Function: Transform.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\n{}\n- {statement}\n- the function returns local.n\n",
                if executable {
                    "The function executes:"
                } else {
                    "When Transform runs:"
                }
            );
            let error = parse_ail_package_spec_text(&package(), &text).unwrap_err();
            assert!(
                error.contains("AIL-JSON-001") || error.contains("AIL-FUNCTION-001"),
                "{error}"
            );
        }
    }
}

#[test]
fn imports_namespace_field_operands_but_preserve_literal_member_names() {
    let scratch = Scratch::new();
    let mut child = package();
    child.metadata.name = "mutation-child".into();
    child.spec_text = source(
        "- the system assigns \"/data.item\" to data.path\n- the system sets JSON value at path data.path in data.document to data.item into output.document\n- the system assigns \"/old\" to data.path\n- the system removes JSON value at path data.path from output.document into output.document\n- the system returns output.document",
    );
    let mut parent = package();
    parent.spec_text = "The application Parent manages records.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system returns child.output.document\n- the system records a trace event named Done\n".into();
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
    let original = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    let program = ail::ste::parse_ail_ste(
        &compile_ail_core_ste(&parse_ail_core_text(&render_ail_core(&original)).unwrap()).unwrap(),
    )
    .unwrap();
    let input = state(&[
        ("child.data.document", "{\"old\":0}"),
        ("child.data.item", "7"),
    ]);
    let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
    assert_eq!(
        result.final_state["child.output.document"],
        "{\"data.item\":7}"
    );
    let output = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(output.stdout, b"child.output.document={\"data.item\":7}\n");
}

#[test]
fn mutation_checks_source_value_and_final_structural_limits_independently() {
    let scratch = Scratch::new();
    let nested = |depth: usize| format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
    let full_nodes = format!("[{}0]", "0,".repeat(65_534));
    let exact_bytes = format!("\"{}\"", "x".repeat(1_048_574));
    for (opcode, raw, path, value, error) in [
        ("set", "null".into(), "".into(), nested(64), None),
        ("set", "[]".into(), "/-".into(), nested(63), None),
        (
            "set",
            "[]".into(),
            "/-".into(),
            nested(64),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        ("set", nested(64), "/0".into(), nested(63), None),
        (
            "set",
            nested(65),
            "".into(),
            "0".into(),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (
            "set",
            "null".into(),
            "".into(),
            nested(65),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        ("set", full_nodes.clone(), "/0".into(), "0".into(), None),
        (
            "set",
            full_nodes.clone(),
            "/-".into(),
            "0".into(),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        ("remove", full_nodes, "/0".into(), "null".into(), None),
        ("set", "null".into(), "".into(), exact_bytes.clone(), None),
        (
            "set",
            "[]".into(),
            "/-".into(),
            exact_bytes.clone(),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        ("set", exact_bytes, "".into(), "0".into(), None),
        (
            "set",
            format!("\"{}\"", "x".repeat(1_048_575)),
            "".into(),
            "0".into(),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (
            "remove",
            format!("{{\"big\":\"{}\"}}", "\u{7f}".repeat(180_000)),
            "/big".into(),
            "null".into(),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (
            "set",
            "{}".into(),
            "/bad/child".into(),
            "x".repeat(1_048_577),
            Some("AIL-RUNTIME-JSON-PATH:"),
        ),
        (
            "set",
            "{}".into(),
            "/valid".into(),
            "x".repeat(1_048_577),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (
            "set",
            "{}".into(),
            "x".repeat(1_048_577),
            "bad".into(),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
    ] {
        let program = raw_program(vec![
            seed("input.document", &raw),
            seed("input.path", &path),
            seed("input.value", &value),
            if opcode == "set" {
                set("Json")
            } else {
                remove()
            },
            command(
                "TEXT_LENGTH",
                &[("key", "out.count"), ("source", "out.document")],
            ),
            command("OUTPUT_FIELD", &[("key", "out.count")]),
        ]);
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new());
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        if let Some(category) = error {
            assert!(
                vm.unwrap_err().contains(category),
                "{opcode} path length {}",
                path.len()
            );
            failed(&output, category);
        } else {
            assert!(vm.is_ok(), "{opcode}: {vm:?}");
            assert!(output.status.success(), "{opcode}: {}", brief(&output));
        }
    }
}

#[test]
fn only_final_json_bytes_consume_the_shared_list_arena() {
    let scratch = Scratch::new();
    let raw_text = "x".repeat(1_048_574);
    for extra in [false, true] {
        let mut instructions = vec![
            seed("input.document", "null"),
            seed("input.path", ""),
            seed("input.value", &raw_text),
        ];
        // A Text value formatted into a root Json occupies exactly 1 MiB.
        // Mutation must not consume the text arena for that intermediate format.
        instructions.extend((0..64).map(|_| set("Text")));
        instructions.push(command(
            "JSON_FORMAT",
            &[
                ("key", "out.text"),
                ("source", "input.value"),
                ("type", "Text"),
            ],
        ));
        instructions.push(command(
            "ASSIGN_LITERAL",
            &[("key", "small.json"), ("type", "Json"), ("value", "{}")],
        ));
        if extra {
            instructions.push(command(
                "JSON_PARSE",
                &[
                    ("key", "extra.json"),
                    ("type", "Json"),
                    ("source", "small.json"),
                ],
            ));
        }
        instructions.extend([
            command(
                "TEXT_LENGTH",
                &[("key", "out.count"), ("source", "out.document")],
            ),
            command("OUTPUT_FIELD", &[("key", "out.count")]),
        ]);
        let program = raw_program(instructions);
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new());
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        if extra {
            assert!(vm.unwrap_err().contains("AIL-RUNTIME-JSON-LIMIT"));
            failed(&output, "AIL-RUNTIME-JSON-LIMIT");
        } else {
            assert_eq!(vm.unwrap().final_state["out.count"], "1048576");
            assert!(output.status.success(), "{}", brief(&output));
            assert_eq!(output.stdout, b"out.count=1048576\n");
        }
    }
}

#[test]
fn nested_files_compute_values_remove_fields_and_append_records_through_every_route() {
    let scratch = Scratch::new();
    let input = scratch.file("input.json", br#"{"records":[{"value":3,"private":"remove","name":"one"},{"value":-2,"private":"remove","name":"\ud83d\ude42"}]}"#);
    let report = scratch.path("report.json");
    let grants = resources(&[("input", &input)], &[("report", &report)]);
    let body = "- the system reads text from file resource input into data.raw\n- the system parses JSON text data.raw into data.document\n- the system assigns \"/records\" to data.path\n- the system gets JSON value at path data.path from data.document into data.copy\n- the system counts entries in JSON data.copy into data.count\n- the system assigns [] to output.document\n- the system assigns 0 to data.index\n- the system assigns \"/\" to data.text\n- while data.index < data.count:\n- the system formats data.index as JSON text into data.raw\n- the system joins text data.text and data.raw into data.path\n- the system gets JSON value at path data.path from data.copy into data.document\n- the system assigns \"/private\" to data.path\n- the system removes JSON value at path data.path from data.document into data.document\n- the system assigns \"/value\" to data.path\n- the system gets JSON value at path data.path from data.document into data.item\n- the system computes data.item as data.item * data.item\n- the system sets JSON value at path data.path in data.document to data.item into data.document\n- the system assigns \"/-\" to data.path\n- the system sets JSON value at path data.path in output.document to data.document into output.document\n- the system computes data.index as data.index + 1\n- end while\n- the system formats output.document as JSON text into output.text\n- the system writes text output.text to file resource report\n- the system returns data.count";
    let original = core(&source(body));
    let expected = "[{\"value\":9,\"name\":\"one\"},{\"value\":4,\"name\":\"🙂\"}]";
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let result =
            run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
        assert_eq!(result.final_state["data.count"], "2");
        assert_eq!(fs::read(&report).unwrap(), expected.as_bytes());
        fs::remove_file(&report).unwrap();
        let output = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(output.stdout, b"data.count=2\n");
        assert_eq!(fs::read(&report).unwrap(), expected.as_bytes());
    }
}
