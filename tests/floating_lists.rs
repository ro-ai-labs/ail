use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application FloatingLists manages numerical batches.\n\nA Data has:\n- values: List<Float>\n- saved: List<Float>\n- raw: Text\n- document: Json\n- path: Text\n- index: Int\n- item: Float\n- bias: Float\n\nAn Output has:\n- values: List<Float>\n- item: Float\n- count: Int\n- raw: Text\n- document: Json\n\n";

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
            "ail-floating-lists-{}-{}",
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
                "Float list operation blocked: {}",
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
fn floating_lists_execute_all_five_operations_through_source_core_ste_and_native() {
    let text = source(
        "- the system creates an empty list into output.values\n- the system counts items in list data.values into output.count\n- the system gets item -1 from list data.values into output.item\n- the system appends floating -data.bias / 2 to list data.values into output.values\n- the system sets item 0 in list output.values to floating output.item + data.bias into output.values\n- the system returns output.values\n- the system returns output.item\n- the system returns output.count",
    );
    let original = core(&text);
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let input = state(&[("data.values", "[+001, .5, 1., -0]"), ("data.bias", "2")]);
        let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(vm.final_state["output.values"], "[2e0,5e-1,1e0,-0e0,-1e0]");
        assert_eq!(vm.final_state["data.values"], input["data.values"]);
        assert_eq!(vm.final_state["output.item"], "-0e0");
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(
            elf.stdout,
            b"output.count=4\noutput.item=-0e0\noutput.values=[2e0,5e-1,1e0,-0e0,-1e0]\n"
        );
    }
}

fn seed(key: &str, value: &str) -> AilBytecodeInstruction {
    command(
        "SET_FIELD",
        &[("key", key), ("value", value), ("text", "seed")],
    )
}

fn identity(program: &mut AilBytecodeProgram, returns_list: bool) {
    program.actions.insert(
        "identity".into(),
        AilBytecodeAction {
            name: "identity".into(),
            instructions: vec![
                command(
                    "VALUE_FUNCTION",
                    &[
                        ("parameters", "values:List<Float>"),
                        ("locals", ""),
                        ("result", if returns_list { "List<Float>" } else { "Int" }),
                    ],
                ),
                command(
                    "RETURN_VALUE",
                    &[
                        ("type", if returns_list { "List<Float>" } else { "Int" }),
                        (
                            "expression",
                            if returns_list {
                                "f:local.values"
                            } else {
                                "i:7"
                            },
                        ),
                    ],
                ),
            ],
        },
    );
}

#[test]
fn runtime_float_list_tokens_are_distinct_from_strict_finite_json_and_literals() {
    let scratch = Scratch::new();
    let runtime = raw_program(vec![
        command(
            "LIST_APPEND",
            &[
                ("key", "out.values"),
                ("source", "input.values"),
                ("type", "Float"),
                ("value", "d:-0"),
            ],
        ),
        command("OUTPUT_FIELD", &[("key", "out.values")]),
    ]);
    let binary = scratch.binary(&runtime, "Run");
    let input = state(&[(
        "input.values",
        " \t[ +001, .5, 1., -0, +inf, -inf, nan, -nan, 5e-324 ]\r\n",
    )]);
    let expected = "[1e0,5e-1,1e0,-0e0,inf,-inf,nan,nan,5e-324,-0e0]";
    assert_eq!(
        run_ail_bytecode_action(&runtime, "Run", input.clone())
            .unwrap()
            .final_state["out.values"],
        expected
    );
    let elf = native(&binary, &resources(&[], &[]), &input);
    assert!(elf.status.success(), "{}", brief(&elf));
    assert_eq!(elf.stdout, format!("out.values={expected}\n").as_bytes());
    for opcode in ["JSON_PARSE", "JSON_FORMAT"] {
        let program = raw_program(vec![
            command(
                opcode,
                &[
                    ("key", "out.values"),
                    ("source", "input.values"),
                    ("type", "List<Float>"),
                ],
            ),
            command("OUTPUT_FIELD", &[("key", "out.values")]),
        ]);
        let binary = scratch.binary(&program, "Run");
        for value in ["[nan]", "[inf]", "[-inf]", "[1e9999]", "[1,nan]", "[\"1\"]"] {
            let input = state(&[("input.values", value)]);
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
        for value in ["[+1]", "[01]", "[.5]", "[1.]"] {
            let input = state(&[("input.values", value)]);
            let vm = run_ail_bytecode_action(&program, "Run", input.clone());
            let elf = native(&binary, &resources(&[], &[]), &input);
            if opcode == "JSON_PARSE" {
                assert!(vm.unwrap_err().contains("AIL-RUNTIME-JSON:"));
                failed(&elf, "AIL-RUNTIME-JSON:");
            } else {
                assert!(vm.is_ok());
                assert!(elf.status.success(), "{}", brief(&elf));
            }
        }
    }
    for literal in [
        "[+1]", "[01]", "[.5]", "[1.]", "[nan]", "[inf]", "[1e9999]", "[true]", "[[1]]",
    ] {
        let program = raw_program(vec![command(
            "ASSIGN_LITERAL",
            &[
                ("key", "out.values"),
                ("type", "List<Float>"),
                ("value", literal),
            ],
        )]);
        assert!(
            !verify_ail_bytecode(&program).is_empty(),
            "literal accepted {literal}"
        );
        assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
    }
}

#[test]
fn malformed_lists_and_error_priority_match_native_without_exposing_values() {
    let scratch = Scratch::new();
    let program = raw_program(vec![command(
        "LIST_SET",
        &[
            ("key", "out.values"),
            ("source", "input.values"),
            ("type", "Float"),
            ("index", "f:input.index"),
            ("value", "f:input.value d:0 /"),
        ],
    )]);
    let binary = scratch.binary(&program, "Run");
    for raw in [
        "",
        "null",
        "{}",
        "[true]",
        "[null]",
        "[\"1\"]",
        "[[1]]",
        "[,]",
        "[1,]",
        "[1,,2]",
        "[1] extra",
        "[NaN]",
        "[Infinity]",
        "[1_0]",
        "[1 e2]",
        "[+ inf]",
        "[\u{b}1]",
        "\u{85}[1]",
        "[1, PRIVATE_VALUE]",
    ] {
        let input = state(&[
            ("input.values", raw),
            ("input.index", "bad"),
            ("input.value", "PRIVATE_VALUE"),
        ]);
        let error = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap_err();
        assert!(error.contains("AIL-RUNTIME-LIST:"), "{raw}: {error}");
        assert!(!error.contains("PRIVATE_VALUE"));
        let elf = native(&binary, &resources(&[], &[]), &input);
        failed(&elf, "AIL-RUNTIME-LIST:");
        assert!(!String::from_utf8_lossy(&elf.stderr).contains("PRIVATE_VALUE"));
    }
    for (raw, index, value, category) in [
        ("[1]", "bad", "bad", "INT"),
        ("[]", "0", "bad", "INDEX"),
        ("[1]", "-2", "bad", "INDEX"),
        ("[1]", "-9223372036854775808", "bad", "INDEX"),
        ("[1]", "0", "bad", "FLOAT"),
        ("[1]", "-1", "1", "DIVZERO"),
    ] {
        let input = state(&[
            ("input.values", raw),
            ("input.index", index),
            ("input.value", value),
        ]);
        let expected = format!("AIL-RUNTIME-{category}:");
        assert!(
            run_ail_bytecode_action(&program, "Run", input.clone())
                .unwrap_err()
                .contains(&expected)
        );
        failed(&native(&binary, &resources(&[], &[]), &input), &expected);
    }
}

#[test]
fn floating_list_function_loops_and_aliases_keep_caller_storage_immutable() {
    let functions = "Function: scale.\nThe function needs:\n- bias: Float\n- values: List<Float>\nThe function produces:\n- result: List<Float>\nThe function stores:\n- count: Int\n- index: Int\n- item: Float\n- result: List<Float>\nThe function executes:\n- the function creates an empty list into local.result\n- the function counts items in list local.values into local.count\n- the function assigns 0 to local.index\n- while local.index < local.count:\n- the function gets item local.index from list local.values into local.item\n- the function appends floating -(local.item * local.bias) to list local.result into local.result\n- the function computes local.index as local.index + 1\n- end while\n- the function returns local.result";
    let original = core(&with_functions(
        functions,
        "- the system calls scale with bias = data.bias; values = data.values into output.values\n- the system calls scale with bias = data.bias; values = output.values into output.values\n- the system returns output.values",
    ));
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let input = state(&[("data.values", "[.5, -0, inf, nan]"), ("data.bias", "2")]);
        let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(vm.final_state["output.values"], "[2e0,-0e0,inf,nan]");
        assert_eq!(vm.final_state["data.values"], input["data.values"]);
        assert!(!vm.final_state.keys().any(|k| k.starts_with("local.")));
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(elf.stdout, b"output.values=[2e0,-0e0,inf,nan]\n");
    }
}

#[test]
fn json_float_arrays_preserve_lossless_neighbors_and_signed_zero() {
    let text = source(
        "- the system parses JSON text data.raw into data.values\n- the system sets JSON value at path data.path in data.document to data.values into data.document\n- the system gets JSON value at path data.path from data.document into output.values\n- the system formats output.values as JSON text into output.raw\n- the system returns output.raw\n- the system returns data.document",
    );
    let original = core(&text);
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let input = state(&[
            (
                "data.raw",
                "[ -0.0, 4.9406564584124654e-324, 1e-9999, 0.1 ]",
            ),
            (
                "data.document",
                r#"{"keep":1.2345678901234567890123456789,"array":[]}"#,
            ),
            ("data.path", "/array"),
        ]);
        let expected = r#"{"keep":1.2345678901234567890123456789,"array":[-0e0,5e-324,0e0,1e-1]}"#;
        let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(vm.final_state["data.document"], expected);
        assert_eq!(vm.final_state["output.raw"], "[-0e0,5e-324,0e0,1e-1]");
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(
            elf.stdout,
            format!("data.document={expected}\noutput.raw=[-0e0,5e-324,0e0,1e-1]\n").as_bytes()
        );
    }
}

#[test]
fn borrowed_float_list_parameters_allow_canonical_expansion_until_returned() {
    let scratch = Scratch::new();
    let raw = format!("[{}]", vec!["100000000000003"; 65_535].join(","));
    assert!(raw.len() < 1_048_576);
    for returns_list in [false, true] {
        let mut program = raw_program(vec![
            seed("input.values", &raw),
            command(
                "LIST_LENGTH",
                &[
                    ("key", "out.count"),
                    ("source", "input.values"),
                    ("type", "Float"),
                ],
            ),
            command(
                "LIST_GET",
                &[
                    ("key", "out.item"),
                    ("source", "input.values"),
                    ("type", "Float"),
                    ("index", "i:-1"),
                ],
            ),
            command(
                "CALL_VALUE",
                &[
                    ("target", "identity"),
                    ("arguments", "values=f:input.values"),
                    ("key", "out.value"),
                ],
            ),
            command("OUTPUT_FIELD", &[("key", "out.count")]),
            command("OUTPUT_FIELD", &[("key", "out.item")]),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        identity(&mut program, returns_list);
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new());
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        if returns_list {
            assert!(vm.unwrap_err().contains("AIL-RUNTIME-LIST-LIMIT:"));
            failed(&elf, "AIL-RUNTIME-LIST-LIMIT:");
        } else {
            assert_eq!(vm.unwrap().final_state["out.value"], "7");
            assert!(elf.status.success(), "{}", brief(&elf));
            assert_eq!(
                elf.stdout,
                b"out.count=65535\nout.item=1.00000000000003e14\nout.value=7\n"
            );
        }
    }
}

#[test]
fn flat_float_literal_and_json_conversion_accept_all_65536_items() {
    let scratch = Scratch::new();
    for (opcode, type_name) in [
        ("ASSIGN_LITERAL", "List<Float>"),
        ("JSON_PARSE", "List<Float>"),
        ("JSON_PARSE", "Json"),
    ] {
        for count in [65_536, 65_537] {
            let raw = format!("[{}]", vec!["-0"; count].join(","));
            let mut code = vec![seed("input.raw", &raw)];
            code.push(if opcode == "ASSIGN_LITERAL" {
                command(
                    opcode,
                    &[("key", "out.values"), ("type", type_name), ("value", &raw)],
                )
            } else {
                command(
                    opcode,
                    &[
                        ("key", "out.values"),
                        ("type", type_name),
                        ("source", "input.raw"),
                    ],
                )
            });
            code.push(command(
                "LIST_LENGTH",
                &[
                    ("key", "out.count"),
                    ("type", "Float"),
                    ("source", "out.values"),
                ],
            ));
            code.push(command("OUTPUT_FIELD", &[("key", "out.count")]));
            let program = raw_program(code);
            if opcode == "ASSIGN_LITERAL" && count > 65_536 {
                assert!(
                    compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err()
                );
                assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
                continue;
            }
            let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new());
            let elf = native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &BTreeMap::new(),
            );
            if count == 65_536 && type_name == "List<Float>" {
                assert_eq!(vm.unwrap().final_state["out.count"], "65536");
                assert!(elf.status.success(), "{}", brief(&elf));
                assert_eq!(elf.stdout, b"out.count=65536\n");
            } else {
                assert!(vm.unwrap_err().contains("AIL-RUNTIME-JSON-LIMIT:"));
                failed(&elf, "AIL-RUNTIME-JSON-LIMIT:");
            }
        }
    }
}

#[test]
fn floating_list_core_roles_types_profiles_and_secret_negated_operands_are_checked() {
    let original = core(&source(
        "- the system appends floating -data.bias to list data.values into output.values\n- the system sets item data.index in list output.values to floating data.item + data.bias into output.values",
    ));
    for kind in [
        "list_step",
        "list_value",
        "list_source",
        "list_destination",
        "list_index",
        "unary_operand",
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
        assert!(
            compile_ail_core_bytecode(&candidate).is_err(),
            "duplicate {kind}"
        );
    }
    for profile in ["Compiler", "UI", "AgentTool", "System"] {
        let mut candidate = original.clone();
        candidate.package.profile = profile.into();
        assert!(compile_ail_core_ste(&candidate).is_err(), "{profile}");
    }
    for (name, category) in [
        ("Data.bias", "AIL005"),
        ("Data.item", "AIL005"),
        ("Data.values", "AIL005"),
        ("Data.index", "AIL005"),
        ("Output.values", "AIL002"),
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
        let diagnostics = check_ail_core(&candidate);
        assert!(
            diagnostics.iter().any(|d| d.contains(category)),
            "{name}: {diagnostics:?}"
        );
    }
    for body in [
        "- the system appends integer 1 to list data.values into output.values",
        "- the system appends floating data.index to list data.values into output.values",
        "- the system gets item data.bias from list data.values into output.item",
        "- the system gets item 0 from list data.values into output.count",
    ] {
        let package = package();
        if let Ok(document) = parse_ail_package_spec_text(&package, &source(body)) {
            assert!(
                compile_ail_core_bytecode(&elaborate_ail_core(&package, &document)).is_err(),
                "accepted {body}"
            );
        }
    }
}

#[test]
fn malformed_float_list_protocol_and_direct_ste_fail_before_execution() {
    let original = raw_program(vec![command(
        "LIST_SET",
        &[
            ("type", "Float"),
            ("source", "input.values"),
            ("key", "out.values"),
            ("index", "i:0"),
            ("value", "f:input.value d:2 *"),
        ],
    )]);
    let ste = ail::ste::render_ail_ste(&original).unwrap();
    for invalid in [
        ste.replace("f:input.value d:2 *", "f:input.value i:2 *"),
        ste.replace("\"i:0\"", "\"d:0\""),
        ste.replace("\"Application\"", "\"Compiler\""),
    ] {
        assert_ne!(ste, invalid);
        assert!(
            ail::ste::parse_ail_ste(&invalid).is_err(),
            "accepted {invalid}"
        );
    }
    for (key, value) in [
        ("type", "List<Float>"),
        ("type", "Bool"),
        ("index", "d:0"),
        ("value", "i:1"),
        ("value", "d:1 d:2"),
        ("value", "f:bad"),
        ("key", "not a key"),
        ("ignored", "metadata"),
    ] {
        let mut program = original.clone();
        program.actions.get_mut("Run").unwrap().instructions[0]
            .operands
            .insert(key.into(), value.into());
        assert!(!verify_ail_bytecode(&program).is_empty(), "{key}={value}");
        assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
        assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
    }
}

#[test]
fn file_float_array_transform_preserves_unicode_metadata() {
    let source = source(
        "- the system reads text from file resource input into data.raw\n- the system parses JSON text data.raw into data.document\n- the system assigns \"/values\" to data.path\n- the system gets JSON value at path data.path from data.document into data.values\n- the system appends floating -0 to list data.values into output.values\n- the system sets item 0 in list output.values to floating 0.5 into output.values\n- the system sets JSON value at path data.path in data.document to output.values into data.document\n- the system formats data.document as JSON text into output.raw\n- the system writes text output.raw to file resource report\n- the system counts items in list output.values into output.count\n- the system returns output.count",
    );
    let program = compile_ail_core_bytecode(&core(&source)).unwrap();
    let scratch = Scratch::new();
    let input = scratch.file(
        "input file=json",
        br#"{"values":[1,2,3],"note":"\u0000\ud83d\ude42"}"#,
    );
    let report = scratch.path("output file=json");
    let grants = resources(&[("input", &input)], &[("report", &report)]);
    run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
    let expected = "{\"values\":[5e-1,2e0,3e0,-0e0],\"note\":\"\\u0000🙂\"}";
    assert_eq!(fs::read(&report).unwrap(), expected.as_bytes());
    fs::write(&report, b"previous").unwrap();
    let elf = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
    assert!(elf.status.success(), "{}", brief(&elf));
    assert_eq!(elf.stdout, b"output.count=4\n");
    assert_eq!(fs::read(report).unwrap(), expected.as_bytes());
}

#[test]
fn imported_float_lists_namespace_expression_leaves_and_function_targets() {
    let mut child = package();
    child.metadata.name = "float-list-child".into();
    child.spec_text = with_functions(
        "Function: append.\nThe function needs:\n- value: Float\n- values: List<Float>\nThe function produces:\n- result: List<Float>\nThe function stores:\n- result: List<Float>\nThe function executes:\n- the function appends floating -local.value to list local.values into local.result\n- the function returns local.result",
        "- the system appends floating data.bias + 1 to list data.values into output.values\n- the system calls append with value = data.bias; values = output.values into output.values\n- the system returns output.values",
    );
    let mut parent = package();
    parent.spec_text="The application Parent manages numbers.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system returns child.output.values\n- the system records a trace event named Done\n".into();
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
    assert_eq!(
        program.actions["Child.append"].instructions[0].operands["parameters"],
        "value:Float;values:List<Float>"
    );
    let input = state(&[("child.data.values", "[-0]"), ("child.data.bias", "2")]);
    let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
    assert_eq!(vm.final_state["child.output.values"], "[-0e0,3e0,-2e0]");
    let scratch = Scratch::new();
    let elf = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(elf.status.success(), "{}", brief(&elf));
    assert_eq!(elf.stdout, b"child.output.values=[-0e0,3e0,-2e0]\n");
}

#[test]
fn float_list_function_recursion_and_unused_parameter_validation_preserve_limits() {
    let text = with_functions(
        "Function: descend.\nThe function needs:\n- n: Int\n- values: List<Float>\nThe function produces:\n- result: List<Float>\nThe function stores:\n- result: List<Float>\nThe function executes:\n- if local.n == 0:\n- the function returns local.values\n- end if\n- the function calls descend with n = local.n - 1; values = local.values into local.result\n- the function returns local.result",
        "- the system calls descend with n = data.index; values = data.values into output.values\n- the system returns output.values",
    );
    let program = compile_ail_core_bytecode(&core(&text)).unwrap();
    let scratch = Scratch::new();
    let binary = scratch.binary(&program, "Run");
    for (depth, raw, category) in [
        ("63", "[-0,nan,inf]", None),
        ("64", "[-0]", Some("DEPTH")),
        ("100", "[private invalid]", Some("LIST")),
    ] {
        let input = state(&[("data.index", depth), ("data.values", raw)]);
        let vm = run_ail_bytecode_action(&program, "Run", input.clone());
        let elf = native(&binary, &resources(&[], &[]), &input);
        if let Some(category) = category {
            assert!(
                vm.unwrap_err()
                    .contains(&format!("AIL-RUNTIME-{category}:"))
            );
            failed(&elf, &format!("AIL-RUNTIME-{category}:"));
        } else {
            assert_eq!(vm.unwrap().final_state["output.values"], "[-0e0,nan,inf]");
            assert!(elf.status.success(), "{}", brief(&elf));
            assert_eq!(elf.stdout, b"output.values=[-0e0,nan,inf]\n");
        }
    }
    let mut unused = raw_program(vec![command(
        "CALL_VALUE",
        &[
            ("target", "identity"),
            ("arguments", "values=f:data.values"),
            ("key", "out.value"),
        ],
    )]);
    identity(&mut unused, false);
    let input = state(&[("data.values", "[nan,invalid]")]);
    assert!(
        run_ail_bytecode_action(&unused, "Run", input.clone())
            .unwrap_err()
            .contains("AIL-RUNTIME-LIST:")
    );
    failed(
        &native(
            &scratch.binary(&unused, "Run"),
            &resources(&[], &[]),
            &input,
        ),
        "AIL-RUNTIME-LIST:",
    );
}

#[test]
fn descriptive_functions_and_compiler_passes_reject_floating_list_statements() {
    let application = package();
    let compiler = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for operation in [
        "appends floating local.value to list local.values into local.result",
        "sets item 0 in list local.values to floating -local.value into local.result",
    ] {
        for subject in ["the system", "the function"] {
            let statement = format!("{subject} {operation}");
            let legacy = format!(
                "{DECLARATIONS}Function: legacy.\nThe function needs:\n- value: Float\n- values: List<Float>\nThe function produces:\n- result: List<Float>\nWhen legacy runs:\n- {statement}\n- the function returns values unchanged\n"
            );
            let pass = compiler.spec_text.replace(
                "- the system reads every edge",
                &format!("- {statement}\n- the system reads every edge"),
            );
            for (package, text) in [(&application, legacy), (&compiler, pass)] {
                let parsed = parse_ail_package_spec_text(package, &text);
                assert!(parsed.is_err(), "descriptive body accepted {statement}");
            }
        }
    }
}

#[test]
fn every_list_opcode_and_element_type_enforces_profiles_in_direct_ste_and_bytecode() {
    for (element, value) in [("Int", "i:1"), ("Float", "d:1"), ("Text", "input.text")] {
        for opcode in [
            "LIST_EMPTY",
            "LIST_LENGTH",
            "LIST_GET",
            "LIST_APPEND",
            "LIST_SET",
        ] {
            let mut operands = vec![("key", "out.value"), ("type", element)];
            if opcode != "LIST_EMPTY" {
                operands.push(("source", "input.values"));
            }
            if matches!(opcode, "LIST_GET" | "LIST_SET") {
                operands.push(("index", "i:0"));
            }
            if matches!(opcode, "LIST_APPEND" | "LIST_SET") {
                operands.push(("value", value));
            }
            let original = raw_program(vec![command(opcode, &operands)]);
            let ste = ail::ste::render_ail_ste(&original).unwrap();
            for profile in ["Application", "C interop"] {
                let mut program = original.clone();
                program.profile = profile.into();
                assert!(
                    verify_ail_bytecode(&program).is_empty(),
                    "{opcode} {element} {profile}"
                );
                assert!(
                    ail::ste::parse_ail_ste(
                        &ste.replace("\"Application\"", &format!("\"{profile}\""))
                    )
                    .is_ok()
                );
            }
            for profile in ["Compiler", "UI", "AgentTool", "System"] {
                let mut program = original.clone();
                program.profile = profile.into();
                let diagnostics = verify_ail_bytecode(&program);
                assert!(
                    diagnostics
                        .iter()
                        .any(|d| d.contains("AILBC013") && d.contains("profile")),
                    "{opcode} {element} {profile}: {diagnostics:?}"
                );
                assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
                assert!(
                    compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err()
                );
                assert!(ail::ste::render_ail_ste(&program).is_err());
                let invalid = ste.replace("\"Application\"", &format!("\"{profile}\""));
                assert!(
                    ail::ste::parse_ail_ste(&invalid).is_err(),
                    "{opcode} {element} {profile}"
                );
            }
        }
    }
}
