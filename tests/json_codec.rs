use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application Json manages values.\n\nA Data has:\n- raw: Text\n- text: Text\n- copy: Text\n- value: Int\n- flag: Bool\n- values: List<Int>\n- names: List<Text>\n- count: Int\n- index: Int\n- item: Int\n\nAn Output has:\n- text: Text\n- total: Text\n- count: Int\n\n";

fn package() -> AilPackage {
    load_ail_package_dir("bench/R001-add/program.ail").unwrap()
}
fn source(body: &str) -> String {
    format!(
        "{DECLARATIONS}Action: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Converted\n"
    )
}
fn core(text: &str) -> AilCore {
    let package = package();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, text).unwrap(),
    )
}

#[test]
fn source_json_conversion_reaches_executable_core() {
    let candidate = core(&source(
        "- the system assigns \"[1, -2, 3]\" to data.raw\n- the system parses JSON text data.raw into data.values\n- the system formats data.values as JSON text into output.text\n- the system returns output.text",
    ));
    let program = compile_ail_core_bytecode(&candidate).unwrap();
    let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(result.final_state["output.text"], "[1,-2,3]");
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "ail-json-{}-{}",
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

fn conversion(opcode: &str, type_name: &str, initial: Option<&str>) -> AilBytecodeProgram {
    let mut program =
        compile_ail_core_bytecode(&core(&source("- the system assigns 0 to output.count")))
            .unwrap();
    let mut instructions = Vec::new();
    if let Some(value) = initial {
        instructions.push(command(
            "SET_FIELD",
            &[
                ("key", "input.raw"),
                ("value", value),
                ("text", "runtime transport fixture"),
            ],
        ));
    }
    instructions.extend([
        command(
            opcode,
            &[
                ("key", "output.text"),
                ("source", "input.raw"),
                ("type", type_name),
            ],
        ),
        command("OUTPUT_FIELD", &[("key", "output.text")]),
        command("RETURN_SUCCESS", &[]),
    ]);
    program.actions.get_mut("Run").unwrap().instructions = instructions;
    program
}

#[test]
fn all_types_round_trip_through_source_saved_core_rendered_source_ste_and_native() {
    let scratch = Scratch::new();
    for (field, raw, decoded, canonical) in [
        ("data.value", " -0 ", "0", "0"),
        ("data.flag", " true\r\n", "true", "true"),
        (
            "data.text",
            r#""é\ud83d\ude42\u0000\b\n""#,
            "é🙂\0\x08\n",
            "\"é🙂\\u0000\\u0008\\n\"",
        ),
        (
            "data.values",
            " [1,-0,-9223372036854775808] ",
            "[1,0,-9223372036854775808]",
            "[1,0,-9223372036854775808]",
        ),
        (
            "data.names",
            r#"["\u00e9", "\ud83d\ude42", "\u0085"]"#,
            "[\"é\",\"🙂\",\"\\u0085\"]",
            "[\"é\",\"🙂\",\"\\u0085\"]",
        ),
    ] {
        let original = core(&source(&format!(
            "- the system parses JSON text data.raw into {field}\n- the system formats {field} as JSON text into output.text\n- the system returns output.text"
        )));
        for candidate in [
            original.clone(),
            parse_ail_core_text(&render_ail_core(&original)).unwrap(),
            core(&render_ail_spec_from_core(&original)),
        ] {
            let program =
                ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
            let input = state(&[("data.raw", raw)]);
            let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
            assert_eq!(result.final_state[field], decoded);
            assert_eq!(result.final_state["output.text"], canonical);
            assert!(result.trace.contains(&format!("parsed JSON {field}")));
            assert!(result.trace.contains(&"formatted JSON output.text".into()));
            let output = native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &input,
            );
            assert!(output.status.success(), "{}", brief(&output));
            assert_eq!(
                output.stdout,
                format!("output.text={canonical}\n").as_bytes()
            );
        }
    }
}

#[test]
fn format_uses_integer_transport_rules_and_canonical_text_and_lists() {
    let scratch = Scratch::new();
    for (type_name, raw, expected) in [
        ("Int", "+00042", "42"),
        ("Int", "-000", "0"),
        ("Int", "-09223372036854775808", "-9223372036854775808"),
        ("Bool", "false", "false"),
        (
            "Text",
            "\0\x08\x0c\n\r\t\x7f\u{85}é🙂",
            "\"\\u0000\\u0008\\u000c\\n\\r\\t\\u007f\\u0085é🙂\"",
        ),
        ("List<Int>", " [ 1, -0, 2 ] ", "[1,0,2]"),
        (
            "List<Text>",
            r#"["\ud83d\ude42", "\u0000"]"#,
            "[\"🙂\",\"\\u0000\"]",
        ),
    ] {
        let program = conversion("JSON_FORMAT", type_name, Some(raw));
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(result.final_state["output.text"], expected);
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(
            output.stdout,
            format!("output.text={expected}\n").as_bytes()
        );
    }
}

#[test]
fn malformed_runtime_values_use_json_errors_without_echoing_contents() {
    let scratch = Scratch::new();
    for (opcode, type_name, values) in [
        (
            "JSON_PARSE",
            "Int",
            vec![
                "+1",
                "01",
                "1.0",
                "1e0",
                "9223372036854775808",
                "null",
                "true",
                "\u{85}1",
            ],
        ),
        (
            "JSON_PARSE",
            "Bool",
            vec!["True", "1", "false true", "\"true\""],
        ),
        (
            "JSON_PARSE",
            "Text",
            vec![
                "\"\\ud800\"",
                "\"\\udc00\"",
                "\"bad\nline\"",
                "\"ok\" trailing",
                "[\"x\"]",
            ],
        ),
        (
            "JSON_PARSE",
            "List<Int>",
            vec!["[1,\"wrong\"]", "[[1]]", "[1e0]", "[1,]", "[1]\u{85}"],
        ),
        (
            "JSON_PARSE",
            "List<Text>",
            vec!["[1]", "[\"\\ud800\"]", "[\"x\",null]"],
        ),
        (
            "JSON_FORMAT",
            "Int",
            vec![
                " 1",
                "1 ",
                "1.0",
                "9223372036854775808",
                "confidential_bad_value",
            ],
        ),
        ("JSON_FORMAT", "Bool", vec!["True", "1", " true"]),
        (
            "JSON_FORMAT",
            "List<Int>",
            vec!["[01]", "[+1]", "[1,false]"],
        ),
        ("JSON_FORMAT", "List<Text>", vec!["[1]", "[\"\\ud800\"]"]),
    ] {
        let program = conversion(opcode, type_name, None);
        let binary = scratch.binary(&program, "Run");
        for raw in values {
            let input = state(&[("input.raw", raw), ("output.text", "preserved")]);
            let error = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap_err();
            assert!(error.contains("AIL-RUNTIME-JSON:"), "{error}");
            assert!(!error.contains("confidential_bad_value"), "{error}");
            let output = native(&binary, &resources(&[], &[]), &input);
            failed(&output, "AIL-RUNTIME-JSON:");
            assert!(!String::from_utf8_lossy(&output.stderr).contains("confidential_bad_value"));
        }
        let error = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap_err();
        assert!(error.contains("AIL-RUNTIME-JSON:"), "{error}");
        failed(
            &native(&binary, &resources(&[], &[]), &BTreeMap::new()),
            "AIL-RUNTIME-JSON:",
        );
    }
}

#[test]
fn text_conversions_are_safe_in_place_and_copies_remain_unchanged() {
    let scratch = Scratch::new();
    let original = core(&source(
        "- the system changes the data raw as the data copy\n- the system parses JSON text data.raw into data.raw\n- the system formats data.raw as JSON text into data.raw\n- the system returns data.copy\n- the system returns data.raw",
    ));
    let program = compile_ail_core_bytecode(&original).unwrap();
    let input = state(&[("data.raw", r#" "\ud83d\ude42\u0000" "#)]);
    let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
    assert_eq!(result.final_state["data.copy"], r#" "\ud83d\ude42\u0000" "#);
    assert_eq!(result.final_state["data.raw"], "\"🙂\\u0000\"");
    let output = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(
        output.stdout,
        "data.copy= \"\\ud83d\\ude42\\u0000\" \ndata.raw=\"🙂\\u0000\"\n".as_bytes()
    );
}

#[test]
fn core_and_direct_protocol_enforce_types_profiles_ownership_and_exact_metadata() {
    let original = core(&source(
        "- the system parses JSON text data.raw into data.names\n- the system formats data.names as JSON text into output.text",
    ));
    for edge_kind in ["json_step", "json_source", "json_destination"] {
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
        ("json_kind", "eval"),
        ("value_type", "Decimal"),
        ("ignored", "value"),
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "JsonConversion")
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
        .find(|node| node.kind == "JsonConversion")
        .unwrap()
        .type_name = Some("Text".into());
    assert!(compile_ail_core_bytecode(&typed).is_err());
    for attribute in ["function_owner", "local_name"] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "Field" && node.name == "Data.raw")
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
    for opcode in ["JSON_PARSE", "JSON_FORMAT"] {
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
fn source_rejects_mistyped_conversions_and_compiler_pass_statements() {
    for body in [
        "- the system parses JSON text data.value into data.names",
        "- the system formats data.names as JSON text into data.value",
        "- the system parses JSON text missing.raw into data.names",
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
        "- the system parses JSON text data.raw into data.names",
        "- the system formats data.names as JSON text into output.text",
    ] {
        let mut package = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
        package.spec_text = package
            .spec_text
            .replace("- the system reads every edge whose kind is reads", body);
        assert!(parse_ail_package_document(&package).is_err());
    }
}

#[test]
fn semantic_json_operands_protect_secret_reads_and_writes() {
    for (body, field, code) in [
        (
            "- the system parses JSON text data.raw into data.names",
            "Data.raw",
            "AIL005",
        ),
        (
            "- the system parses JSON text data.raw into data.names",
            "Data.names",
            "AIL002",
        ),
        (
            "- the system formats data.names as JSON text into output.text",
            "Data.names",
            "AIL005",
        ),
        (
            "- the system formats data.names as JSON text into output.text",
            "Output.text",
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
fn imported_json_conversions_namespace_fields_without_changing_types() {
    let scratch = Scratch::new();
    let mut child = package();
    child.metadata.name = "json-child".into();
    child.spec_text = source(
        "- the system parses JSON text data.raw into data.names\n- the system formats data.names as JSON text into output.text\n- the system returns output.text",
    );
    let mut parent = package();
    parent.spec_text = "The application Parent manages JSON.\nAction: Run.\nWhen Run happens:\n- the system calls Json.Run\n- the system returns json.output.text\n- the system records a trace event named Done\n".into();
    parent.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Json".into(),
            resolved_package: None,
            registry_identity: None,
        },
        package: Box::new(child),
    });
    let original = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    let program =
        compile_ail_core_bytecode(&parse_ail_core_text(&render_ail_core(&original)).unwrap())
            .unwrap();
    let input = state(&[("json.data.raw", r#"["\ud83d\ude42", "é"]"#)]);
    let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
    assert_eq!(result.final_state["json.output.text"], "[\"🙂\",\"é\"]");
    let output = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(
        output.stdout,
        "json.output.text=[\"🙂\",\"é\"]\n".as_bytes()
    );
}

#[test]
fn file_json_dataset_computes_squares_and_integer_aggregate() {
    let scratch = Scratch::new();
    let input = scratch.file("input.json", b" [1, -2, 3, 10] \n");
    let squared = scratch.path("squared.json");
    let total = scratch.path("total.json");
    let grants = resources(
        &[("input", &input)],
        &[("squared", &squared), ("total", &total)],
    );
    let text = format!(
        "{DECLARATIONS}Function: square.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function executes:\n- the function returns local.n * local.n\n\nAction: Run.\nWhen Run happens:\n- the system reads text from file resource input into data.raw\n- the system parses JSON text data.raw into data.values\n- the system counts items in list data.values into data.count\n- the system assigns 0 to data.index\n- the system assigns 0 to data.value\n- while data.index < data.count:\n- the system gets item data.index from list data.values into data.item\n- the system calls square with n = data.item into data.item\n- the system sets item data.index in list data.values to integer data.item into data.values\n- the system computes data.value as data.value + data.item\n- the system computes data.index as data.index + 1\n- end while\n- the system formats data.values as JSON text into output.text\n- the system formats data.value as JSON text into output.total\n- the system writes text output.text to file resource squared\n- the system writes text output.total to file resource total\n- the system returns data.count\n- the system records a trace event named Computed\n"
    );
    let original = core(&text);
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let result =
            run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
        assert_eq!(result.final_state["data.count"], "4");
        assert_eq!(fs::read(&squared).unwrap(), b"[1,4,9,100]");
        assert_eq!(fs::read(&total).unwrap(), b"114");
        fs::remove_file(&squared).unwrap();
        fs::remove_file(&total).unwrap();
        let output = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(output.stdout, b"data.count=4\n");
        assert_eq!(fs::read(&squared).unwrap(), b"[1,4,9,100]");
        assert_eq!(fs::read(&total).unwrap(), b"114");
    }
}

#[test]
fn input_result_and_item_limits_use_json_limit_errors() {
    let scratch = Scratch::new();
    for (opcode, type_name, raw, expected_error) in [
        (
            "JSON_PARSE",
            "Text",
            format!("\"{}\"", "x".repeat(1_048_574)),
            None,
        ),
        (
            "JSON_PARSE",
            "Text",
            format!("\"{}\"", "x".repeat(1_048_575)),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        ("JSON_FORMAT", "Text", "x".repeat(1_048_574), None),
        (
            "JSON_FORMAT",
            "Text",
            "x".repeat(1_048_575),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (
            "JSON_PARSE",
            "Int",
            "x".repeat(1_048_577),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (
            "JSON_FORMAT",
            "Bool",
            "x".repeat(1_048_577),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (
            "JSON_PARSE",
            "List<Int>",
            format!("[{}0]", "0,".repeat(65_535)),
            None,
        ),
        (
            "JSON_PARSE",
            "List<Int>",
            format!("[{}0]", "0,".repeat(65_536)),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (
            "JSON_PARSE",
            "List<Text>",
            format!("[\"{}\"]", "\u{7f}".repeat(200_000)),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (
            "JSON_FORMAT",
            "List<Text>",
            format!("[\"{}\",1]", "\u{7f}".repeat(200_000)),
            Some("AIL-RUNTIME-JSON:"),
        ),
    ] {
        let mut program = conversion(opcode, type_name, Some(&raw));
        let instructions = &mut program.actions.get_mut("Run").unwrap().instructions;
        instructions.pop();
        instructions.pop();
        instructions.extend([
            command(
                "TEXT_LENGTH",
                &[("source", "output.text"), ("key", "output.count")],
            ),
            command("OUTPUT_FIELD", &[("key", "output.count")]),
            command("RETURN_SUCCESS", &[]),
        ]);
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new());
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        if let Some(category) = expected_error {
            let error = vm.unwrap_err();
            assert!(error.contains(category), "{error}");
            failed(&output, category);
        } else {
            assert!(vm.is_ok());
            assert!(output.status.success(), "{}", brief(&output));
        }
    }
}

#[test]
fn json_budgets_are_shared_across_calls_without_intermediate_charges() {
    let scratch = Scratch::new();
    let list = format!("[\"{}\"]", "x".repeat(1_048_572));
    for (operation, source_field, raw, finish) in [
        (
            "the system parses JSON text data.raw into data.names",
            "data.raw",
            list.clone(),
            "- the system formats data.names as JSON text into output.text\n- the system counts characters in output.text into output.count",
        ),
        (
            "the system formats data.names as JSON text into output.text",
            "data.names",
            list.clone(),
            "- the system parses JSON text data.raw into data.names\n- the system counts items in list data.names into output.count",
        ),
        (
            "the system formats data.text as JSON text into output.text",
            "data.text",
            "x".repeat(1_048_574),
            "- the system parses JSON text data.raw into data.names\n- the system counts items in list data.names into output.count",
        ),
    ] {
        for extra in [false, true] {
            let text = format!(
                "{}\nAction: Worker.\nWhen Worker happens:\n- the system assigns 0 to data.count\n- while data.count < 32:\n- {operation}\n- the system computes data.count as data.count + 1\n- end while\n- the system records a trace event named Batch\n",
                source(&format!(
                    "- the system calls Worker\n- the system calls Worker\n{}\n{finish}\n- the system returns output.count",
                    if extra {
                        format!("- {operation}")
                    } else {
                        String::new()
                    }
                ))
            );
            let mut program = compile_ail_core_bytecode(&core(&text)).unwrap();
            let instructions = &mut program.actions.get_mut("Run").unwrap().instructions;
            instructions.insert(
                0,
                command(
                    "SET_FIELD",
                    &[
                        ("key", "data.raw"),
                        ("value", &list),
                        ("text", "JSON input fixture"),
                    ],
                ),
            );
            instructions.insert(
                1,
                command(
                    "SET_FIELD",
                    &[
                        ("key", source_field),
                        ("value", &raw),
                        ("text", "conversion input fixture"),
                    ],
                ),
            );
            let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new());
            let output = native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &BTreeMap::new(),
            );
            if extra {
                let error = vm.unwrap_err();
                assert!(error.contains("AIL-RUNTIME-JSON-LIMIT"), "{error}");
                failed(&output, "AIL-RUNTIME-JSON-LIMIT");
            } else {
                assert_eq!(
                    vm.unwrap().final_state["output.count"],
                    if operation.contains("parses") {
                        "1048576"
                    } else {
                        "1"
                    }
                );
                assert!(output.status.success(), "{}", brief(&output));
            }
        }
    }
}
