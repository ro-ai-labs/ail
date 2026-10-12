use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application Structured manages records.\n\nA Data has:\n- raw: Text\n- path: Text\n- document: Json\n- copy: Json\n- count: Int\n- index: Int\n- item: Int\n- total: Int\n- text: Text\n- flag: Bool\n- values: List<Int>\n- names: List<Text>\n\nAn Output has:\n- text: Text\n- value: Int\n- count: Int\n- document: Json\n\n";

fn package() -> AilPackage {
    load_ail_package_dir("bench/R001-add/program.ail").unwrap()
}

fn source(body: &str) -> String {
    format!(
        "{DECLARATIONS}Action: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Structured\n"
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
fn nested_json_literal_parse_lookup_and_count_are_executable() {
    let candidate = core(&source(
        "- the system assigns {\"records\":[{\"value\":7},null]} to data.document\n- the system formats data.document as JSON text into data.raw\n- the system parses JSON text data.raw into data.copy\n- the system assigns \"/records/0/value\" to data.path\n- the system gets JSON value at path data.path from data.copy into output.value\n- the system counts entries in JSON data.copy into output.count\n- the system returns output.value\n- the system returns output.count",
    ));
    let program = compile_ail_core_bytecode(&candidate).unwrap();
    let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(result.final_state["output.value"], "7");
    assert_eq!(result.final_state["output.count"], "1");
    let instructions = &program.actions["Run"].instructions;
    for opcode in ["ASSIGN_LITERAL", "JSON_PARSE", "JSON_GET", "JSON_COUNT"] {
        assert!(
            instructions
                .iter()
                .any(|instruction| instruction.opcode == opcode),
            "missing {opcode}"
        );
    }
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "ail-structured-json-{}-{}",
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

fn get(type_name: &str) -> AilBytecodeInstruction {
    command(
        "JSON_GET",
        &[
            ("key", "out.value"),
            ("source", "input.raw"),
            ("path", "input.path"),
            ("type", type_name),
        ],
    )
}

fn seed(key: &str, value: &str) -> AilBytecodeInstruction {
    command(
        "SET_FIELD",
        &[
            ("key", key),
            ("value", value),
            ("text", "structured JSON fixture"),
        ],
    )
}

#[test]
fn lossless_json_source_core_ste_and_native_round_trips() {
    let scratch = Scratch::new();
    let literal = r#"{"records":[{"value":7,"text":"é\ud83d\ude42\u0000"}],"n":-0.000E+012,"huge":9999999999999999999999999,"a":true,"a":null,"[value_type=Int,literal=1],value_type=Text":"to data.document"}"#;
    let expected = "{\"records\":[{\"value\":7,\"text\":\"é🙂\\u0000\"}],\"n\":-0.000E+012,\"huge\":9999999999999999999999999,\"a\":true,\"a\":null,\"[value_type=Int,literal=1],value_type=Text\":\"to data.document\"}";
    let original = core(&source(&format!(
        "- the system assigns {literal} to data.document\n- the system formats data.document as JSON text into data.raw\n- the system parses JSON text data.raw into data.copy\n- the system assigns \"/records/0/value\" to data.path\n- the system gets JSON value at path data.path from data.copy into output.value\n- the system counts entries in JSON data.copy into output.count\n- the system formats data.copy as JSON text into output.text\n- the system returns output.value\n- the system returns output.count\n- the system returns output.text"
    )));
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let ste = compile_ail_core_ste(&candidate).unwrap();
        assert!(ste.contains("value at JSON path"));
        assert!(ste.contains("Count entries in JSON field"));
        let program = ail::ste::parse_ail_ste(&ste).unwrap();
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(result.final_state["output.text"], expected);
        assert_eq!(result.final_state["output.value"], "7");
        assert_eq!(result.final_state["output.count"], "6");
        assert!(result.trace.contains(&"got JSON output.value".into()));
        assert!(result.trace.contains(&"counted JSON output.count".into()));
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(
            output.stdout,
            format!("output.count=6\noutput.text={expected}\noutput.value=7\n").as_bytes()
        );
    }
}

#[test]
fn pointers_decode_once_preserve_unicode_and_extract_every_type() {
    let scratch = Scratch::new();
    let raw = r#"{"":7,"a/b":{"~key":["é\ud83d\ude42\u0000",true,-0,[1,-2],["a","\u0085"],{"x":1e+999}]},"~1":42,"\u0000":9,"é":10,"e\u0301":11,"dup":0,"\u0064up":1}"#;
    for (path, type_name, expected) in [
        ("/", "Int", "7"),
        ("/~01", "Int", "42"),
        ("/é", "Int", "10"),
        ("/e\u{301}", "Int", "11"),
        ("/a~1b/~0key/0", "Text", "é🙂\0"),
        ("/a~1b/~0key/1", "Bool", "true"),
        ("/a~1b/~0key/2", "Int", "0"),
        ("/a~1b/~0key/3", "List<Int>", "[1,-2]"),
        ("/a~1b/~0key/4", "List<Text>", "[\"a\",\"\\u0085\"]"),
        ("/a~1b/~0key/5", "Json", "{\"x\":1e+999}"),
        ("/a~1b/~0key/2", "Json", "-0"),
    ] {
        let program = raw_program(vec![
            get(type_name),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        let input = state(&[("input.raw", raw), ("input.path", path)]);
        let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(
            result.final_state["out.value"], expected,
            "{path}/{type_name}"
        );
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(output.stdout, format!("out.value={expected}\n").as_bytes());
    }
    // An actual NUL cannot travel in argv; keep the pointer in a static fixture.
    let program = raw_program(vec![
        seed("input.path", "/\0"),
        get("Int"),
        command("OUTPUT_FIELD", &[("key", "out.value")]),
    ]);
    let input = state(&[("input.raw", raw)]);
    assert_eq!(
        run_ail_bytecode_action(&program, "Run", input.clone())
            .unwrap()
            .final_state["out.value"],
        "9"
    );
    let output = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(output.stdout, b"out.value=9\n");
}

#[test]
fn malformed_sources_paths_and_typed_selections_fail_consistently() {
    let scratch = Scratch::new();
    for (type_name, raw, path, category) in [
        ("Json", "[1,]", "/bad~2", "AIL-RUNTIME-JSON:"),
        ("Json", "{\"a\":1,\"a\":2}", "/a", "AIL-RUNTIME-JSON-PATH:"),
        (
            "Json",
            "{\"a\":1,\"\\u0061\":2}",
            "/a",
            "AIL-RUNTIME-JSON-PATH:",
        ),
        ("Json", "[0]", "/00", "AIL-RUNTIME-JSON-PATH:"),
        ("Json", "[0]", "/-", "AIL-RUNTIME-JSON-PATH:"),
        ("Json", "[0]", "/-1", "AIL-RUNTIME-JSON-PATH:"),
        (
            "Json",
            "[0]",
            "/184467440737095516160000",
            "AIL-RUNTIME-JSON-PATH:",
        ),
        ("Json", "[0]", "#/0", "AIL-RUNTIME-JSON-PATH:"),
        (
            "Json",
            "{\"x\":{}}",
            "/missing/later~2",
            "AIL-RUNTIME-JSON-PATH:",
        ),
        ("Json", "{\"x\":0}", "/x/child", "AIL-RUNTIME-JSON-PATH:"),
        ("Int", "{\"n\":1e0}", "/n", "AIL-RUNTIME-JSON:"),
        (
            "Int",
            "{\"n\":9223372036854775808}",
            "/n",
            "AIL-RUNTIME-JSON:",
        ),
        ("Int", "{\"n\":1.0}", "/n", "AIL-RUNTIME-JSON:"),
        ("Text", "null", "", "AIL-RUNTIME-JSON:"),
        ("List<Int>", "[1,true]", "", "AIL-RUNTIME-JSON:"),
        ("List<Text>", "[\"x\",null]", "", "AIL-RUNTIME-JSON:"),
        (
            "Json",
            "{\"good\":0,\"bad\":\"\\ud800\"}",
            "/good",
            "AIL-RUNTIME-JSON:",
        ),
        (
            "Json",
            "{\"good\":0} trailing",
            "/good",
            "AIL-RUNTIME-JSON:",
        ),
        ("Json", "[01]", "", "AIL-RUNTIME-JSON:"),
        ("Json", "[+1]", "", "AIL-RUNTIME-JSON:"),
        ("Json", "[NaN]", "", "AIL-RUNTIME-JSON:"),
    ] {
        let program = raw_program(vec![
            get(type_name),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        let input = state(&[
            ("input.raw", raw),
            ("input.path", path),
            ("out.value", "untouched"),
        ]);
        let error = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap_err();
        assert!(error.contains(category), "{raw}/{path}: {error}");
        failed(
            &native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &input,
            ),
            category,
        );
    }
    let program = raw_program(vec![
        command(
            "JSON_COUNT",
            &[("key", "out.value"), ("source", "input.raw")],
        ),
        command("OUTPUT_FIELD", &[("key", "out.value")]),
    ]);
    let binary = scratch.binary(&program, "Run");
    for raw in ["null", "true", "1", "\"text\"", "{\"x\":}"] {
        let input = state(&[("input.raw", raw)]);
        assert!(
            run_ail_bytecode_action(&program, "Run", input.clone())
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
fn json_aliases_are_immutable_and_count_includes_duplicate_pairs() {
    let scratch = Scratch::new();
    let original = core(&source(
        "- the system parses JSON text data.raw into data.document\n- the system changes the data document as the data copy\n- the system gets JSON value at path data.path from data.document into data.document\n- the system counts entries in JSON data.document into output.count\n- the system formats data.copy as JSON text into output.text\n- the system returns output.count\n- the system returns output.text\n- the system returns data.document",
    ));
    let program = compile_ail_core_bytecode(&original).unwrap();
    let input = state(&[
        ("data.raw", "{\"x\":{\"a\":1,\"a\":2}}"),
        ("data.path", "/x"),
    ]);
    let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
    assert_eq!(result.final_state["data.copy"], "{\"x\":{\"a\":1,\"a\":2}}");
    assert_eq!(result.final_state["data.document"], "{\"a\":1,\"a\":2}");
    assert_eq!(result.final_state["output.count"], "2");
    let output = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(
        output.stdout,
        b"data.document={\"a\":1,\"a\":2}\noutput.count=2\noutput.text={\"x\":{\"a\":1,\"a\":2}}\n"
    );
}

#[test]
fn json_access_core_protocol_requires_exact_types_profiles_ownership_and_metadata() {
    let original = core(&source(
        "- the system gets JSON value at path data.path from data.document into data.names\n- the system counts entries in JSON data.document into output.count",
    ));
    for edge_kind in [
        "json_access_step",
        "json_access_source",
        "json_access_destination",
        "json_access_path",
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
        ("json_access_kind", "eval"),
        ("value_type", "Decimal"),
        ("ignored", "value"),
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "JsonAccess")
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
        .find(|node| node.kind == "JsonAccess")
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
    for opcode in ["JSON_GET", "JSON_COUNT"] {
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
fn source_rejects_mistyped_json_access_and_compiler_pass_statements() {
    for body in [
        "- the system gets JSON value at path data.value from data.document into data.names",
        "- the system counts entries in JSON data.raw into data.value",
        "- the system gets JSON value at path data.path from missing.raw into data.names",
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
        "- the system gets JSON value at path data.path from data.document into data.names",
        "- the system counts entries in JSON data.document into output.count",
    ] {
        let mut package = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
        package.spec_text = package
            .spec_text
            .replace("- the system reads every edge whose kind is reads", body);
        assert!(parse_ail_package_document(&package).is_err());
    }
}

#[test]
fn direct_json_literals_enforce_executable_profiles_in_all_backends() {
    for profile in ["Compiler", "System", "UI", "AgentTool"] {
        let mut program = raw_program(vec![command(
            "ASSIGN_LITERAL",
            &[
                ("key", "out.document"),
                ("type", "Json"),
                ("value", "{\"a\":null}"),
            ],
        )]);
        let ste = ail::ste::render_ail_ste(&program).unwrap();
        program.profile = profile.into();
        assert!(
            !verify_ail_bytecode(&program).is_empty(),
            "profile {profile}"
        );
        assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
        assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
        assert!(ail::ste::render_ail_ste(&program).is_err());
        let ste = ste.replace(
            "\"profile\":\"Application\"",
            &format!("\"profile\":\"{profile}\""),
        );
        assert!(ail::ste::parse_ail_ste(&ste).is_err());
    }
}

#[test]
fn function_bodies_reject_json_operations_instead_of_discarding_them() {
    let package = package();
    for section in ["When Read runs:", "The function executes:"] {
        for statement in [
            "the system gets JSON value at path data.path from data.document into output.value",
            "the system counts entries in JSON data.document into output.count",
            "the system parses JSON text data.raw into data.document",
            "the system formats data.document as JSON text into output.text",
        ] {
            let source = format!(
                "{DECLARATIONS}Function: Read.\nThe function needs:\n- value: Int\nThe function produces:\n- result: Int\n{section}\n- {statement}\n- the function returns local.value\n\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Done\n"
            );
            assert!(
                parse_ail_package_spec_text(&package, &source).is_err(),
                "{section}: {statement}"
            );
        }
    }
}

#[test]
fn json_access_semantic_operands_protect_secret_reads_and_writes() {
    for (body, field, code) in [
        (
            "- the system gets JSON value at path data.path from data.document into data.names",
            "Data.path",
            "AIL005",
        ),
        (
            "- the system gets JSON value at path data.path from data.document into data.names",
            "Data.document",
            "AIL005",
        ),
        (
            "- the system gets JSON value at path data.path from data.document into data.names",
            "Data.names",
            "AIL002",
        ),
        (
            "- the system counts entries in JSON data.document into output.count",
            "Data.document",
            "AIL005",
        ),
        (
            "- the system counts entries in JSON data.document into output.count",
            "Output.count",
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
fn imported_json_access_namespaces_fields_and_preserves_literal_paths() {
    let scratch = Scratch::new();
    let mut child = package();
    child.metadata.name = "structured-child".into();
    child.spec_text = source(
        "- the system parses JSON text data.raw into data.document\n- the system assigns \"/data.value\" to data.path\n- the system gets JSON value at path data.path from data.document into output.value\n- the system counts entries in JSON data.document into output.count\n- the system returns output.value",
    );
    let mut parent = package();
    parent.spec_text = "The application Parent manages records.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system returns child.output.value\n- the system returns child.output.count\n- the system records a trace event named Done\n".into();
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
    let input = state(&[("child.data.raw", "{\"data.value\":8,\"other\":null}")]);
    let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
    assert_eq!(result.final_state["child.output.value"], "8");
    assert_eq!(result.final_state["child.output.count"], "2");
    let output = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(
        output.stdout,
        b"child.output.count=2\nchild.output.value=8\n"
    );
}

#[test]
fn nested_record_files_are_processed_with_dynamic_pointers_and_integer_arithmetic() {
    let scratch = Scratch::new();
    let input = scratch.file("records.json", br#"{"records":[{"value":3,"name":"one"},{"value":-2,"name":"\ud83d\ude42"},{"value":10,"name":"three"}]}"#);
    let report = scratch.path("report.json");
    let name = scratch.path("name.txt");
    let grants = resources(
        &[("input", &input)],
        &[("report", &report), ("name", &name)],
    );
    let original = core(&source(
        "- the system reads text from file resource input into data.raw\n- the system parses JSON text data.raw into data.document\n- the system assigns \"/records\" to data.path\n- the system gets JSON value at path data.path from data.document into data.copy\n- the system counts entries in JSON data.copy into data.count\n- the system assigns 0 to data.index\n- the system assigns 0 to data.total\n- the system assigns \"/\" to data.text\n- while data.index < data.count:\n- the system formats data.index as JSON text into data.raw\n- the system joins text data.text and data.raw into data.path\n- the system gets JSON value at path data.path from data.copy into data.document\n- the system assigns \"/value\" to data.path\n- the system gets JSON value at path data.path from data.document into data.item\n- the system computes data.total as data.total + data.item * data.item\n- the system assigns \"/name\" to data.path\n- the system gets JSON value at path data.path from data.document into output.text\n- the system writes text output.text to file resource name\n- the system computes data.index as data.index + 1\n- end while\n- the system formats data.total as JSON text into output.text\n- the system writes text output.text to file resource report\n- the system returns data.total\n- the system returns data.count",
    ));
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let result =
            run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
        assert_eq!(result.final_state["data.total"], "113");
        assert_eq!(fs::read(&report).unwrap(), b"113");
        assert_eq!(fs::read(&name).unwrap(), b"three");
        fs::remove_file(&report).unwrap();
        fs::remove_file(&name).unwrap();
        let output = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(output.stdout, b"data.count=3\ndata.total=113\n");
        assert_eq!(fs::read(&report).unwrap(), b"113");
        assert_eq!(fs::read(&name).unwrap(), b"three");
    }
}

#[test]
fn recursive_json_depth_value_and_byte_limits_apply_to_entire_documents() {
    let scratch = Scratch::new();
    let exact_bytes = format!("\"{}\"", "x".repeat(1_048_574));
    let over_bytes = format!("\"{}\"", "x".repeat(1_048_575));
    for (raw, error) in [
        (format!("{}0{}", "[".repeat(64), "]".repeat(64)), None),
        (
            format!("{}0{}", "[".repeat(65), "]".repeat(65)),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (format!("[{}0]", "0,".repeat(65_534)), None),
        (
            format!("[{}0]", "0,".repeat(65_535)),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (exact_bytes, None),
        (over_bytes, Some("AIL-RUNTIME-JSON-LIMIT")),
        (
            format!("{{\"small\":0,\"large\":\"{}\"}}", "\u{7f}".repeat(180_000)),
            Some("AIL-RUNTIME-JSON-LIMIT"),
        ),
        (
            format!(
                "{{\"small\":0,\"large\":\"{}\",\"bad\":}}",
                "\u{7f}".repeat(180_000)
            ),
            Some("AIL-RUNTIME-JSON:"),
        ),
    ] {
        for opcode in ["JSON_PARSE", "JSON_FORMAT", "JSON_GET"] {
            let instruction = if opcode == "JSON_GET" {
                get("Json")
            } else {
                command(
                    opcode,
                    &[
                        ("key", "out.value"),
                        ("source", "input.raw"),
                        ("type", "Json"),
                    ],
                )
            };
            let program = raw_program(vec![
                seed("input.raw", &raw),
                seed("input.path", ""),
                instruction,
                command(
                    "TEXT_LENGTH",
                    &[("key", "out.count"), ("source", "out.value")],
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
                assert!(vm.unwrap_err().contains(category), "{opcode}");
                failed(&output, category);
            } else {
                assert!(vm.is_ok(), "{opcode}: {vm:?}");
                assert!(output.status.success(), "{opcode}: {}", brief(&output));
            }
        }
    }
    for (raw, path, category) in [
        ("{}", "x".repeat(1_048_577), "AIL-RUNTIME-JSON-LIMIT"),
        ("malformed", "x".repeat(1_048_577), "AIL-RUNTIME-JSON:"),
    ] {
        let program = raw_program(vec![
            seed("input.raw", raw),
            seed("input.path", &path),
            get("Json"),
        ]);
        assert!(
            run_ail_bytecode_action(&program, "Run", BTreeMap::new())
                .unwrap_err()
                .contains(category)
        );
        failed(
            &native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &BTreeMap::new(),
            ),
            category,
        );
    }
}

#[test]
fn json_results_share_the_list_arena_without_charging_scratch_or_literals() {
    let scratch = Scratch::new();
    let raw = format!("\"{}\"", "x".repeat(1_048_574));
    for extra in [false, true] {
        let mut instructions = vec![seed("input.raw", &raw), seed("input.path", "")];
        // Each canonical Json result is exactly 1 MiB, alternating parse and lookup.
        for n in 0..64 {
            instructions.push(if n % 2 == 0 {
                get("Json")
            } else {
                command(
                    "JSON_PARSE",
                    &[
                        ("key", "out.value"),
                        ("source", "input.raw"),
                        ("type", "Json"),
                    ],
                )
            });
        }
        // Constants and count/scalar lookup remain usable after filling that arena.
        instructions.push(command(
            "ASSIGN_LITERAL",
            &[
                ("key", "small.raw"),
                ("type", "Json"),
                ("value", "{\"n\":8}"),
            ],
        ));
        instructions.push(command(
            "JSON_COUNT",
            &[("key", "out.count"), ("source", "small.raw")],
        ));
        instructions.push(command(
            "JSON_FORMAT",
            &[
                ("key", "out.text"),
                ("source", "out.value"),
                ("type", "Json"),
            ],
        ));
        if extra {
            instructions.push(command(
                "JSON_PARSE",
                &[
                    ("key", "out.list"),
                    ("source", "small.list"),
                    ("type", "List<Int>"),
                ],
            ));
        }
        instructions.push(command("OUTPUT_FIELD", &[("key", "out.count")]));
        let program = raw_program(instructions);
        let input = state(&[("small.list", "[]")]);
        let vm = run_ail_bytecode_action(&program, "Run", input.clone());
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        if extra {
            assert!(vm.unwrap_err().contains("AIL-RUNTIME-JSON-LIMIT"));
            failed(&output, "AIL-RUNTIME-JSON-LIMIT");
        } else {
            assert_eq!(vm.unwrap().final_state["out.count"], "1");
            assert!(output.status.success(), "{}", brief(&output));
            assert_eq!(output.stdout, b"out.count=1\n");
        }
    }
}
