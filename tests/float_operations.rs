use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application Floating manages measurements.\n\nA Data has:\n- a: Float\n- b: Float\n- n: Int\n- count: Int\n- raw: Text\n- document: Json\n- path: Text\n\nAn Output has:\n- value: Float\n- other: Float\n- count: Int\n- raw: Text\n- document: Json\n\n";

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
            "ail-float-operations-{}-{}",
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
                "Float operation blocked: {}",
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
fn mixed_controls_functions_and_signed_zero_survive_all_public_routes() {
    let text = with_functions(
        "Function: scale.\nThe function needs:\n- n: Int\n- value: Float\nThe function produces:\n- result: Float\nThe function stores:\n- divisor: Float\n- result: Float\nThe function executes:\n- the function converts integer local.n to floating local.divisor\n- the function computes floating local.result as local.value / local.divisor\n- the function returns local.result",
        "- the system assigns 0 to data.count\n- the system computes floating data.a as 0\n- while floating data.a < 3:\n- the system computes floating data.a as data.a + 1\n- if data.count == 1:\n- the system computes data.count as data.count + 1\n- continue\n- end if\n- the system computes data.count as data.count + 1\n- end while\n- the system calls scale with n = data.count; value = data.a into output.value\n- the system computes floating output.other as -0\n- if floating nan != nan:\n- the system computes output.count as data.count + 10\n- otherwise:\n- the system assigns 99 to output.count\n- end if\n- the system returns output.value\n- the system returns output.other\n- the system returns output.count",
    );
    let original = core(&text);
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let ste = compile_ail_core_ste(&candidate).unwrap();
        assert!(ste.contains("Compute floating expression"));
        assert!(ste.contains("If floating comparison"));
        let program = ail::ste::parse_ail_ste(&ste).unwrap();
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(result.final_state["output.value"], "1e0");
        assert_eq!(result.final_state["output.other"], "-0e0");
        assert_eq!(result.final_state["output.count"], "13");
        assert!(
            !result
                .final_state
                .keys()
                .any(|key| key.starts_with("local."))
        );
        let binary = scratch.binary(&program, "Run");
        let result = command_output(Command::new(&binary).env_clear());
        assert!(result.status.success(), "{}", brief(&result));
        assert_eq!(
            result.stdout,
            b"output.count=13\noutput.other=-0e0\noutput.value=1e0\n"
        );
        // The embedded Float runtime must not introduce a loader or dynamic section.
        let elf = fs::read(binary).unwrap();
        let phoff = u64::from_le_bytes(elf[32..40].try_into().unwrap()) as usize;
        let phsize = u16::from_le_bytes(elf[54..56].try_into().unwrap()) as usize;
        let phnum = u16::from_le_bytes(elf[56..58].try_into().unwrap()) as usize;
        for i in 0..phnum {
            let p = phoff + i * phsize;
            assert!(!matches!(
                u32::from_le_bytes(elf[p..p + 4].try_into().unwrap()),
                2 | 3
            ));
        }
    }
}

#[test]
fn conversion_edges_and_float_transport_errors_match_native() {
    let scratch = Scratch::new();
    for (opcode, values) in [
        (
            "FLOAT_TO_INT",
            vec![
                ("-9223372036854775808", Ok("-9223372036854775808")),
                ("9223372036854774784", Ok("9223372036854774784")),
                ("9223372036854775808", Err("FLOAT")),
                ("-9223372036854777856", Err("FLOAT")),
                ("-3.999", Ok("-3")),
                ("-0", Ok("0")),
                ("inf", Err("FLOAT")),
                ("nan", Err("FLOAT")),
                ("1_0", Err("FLOAT")),
                (" 1", Err("FLOAT")),
                ("NaN", Err("FLOAT")),
            ],
        ),
        (
            "INT_TO_FLOAT",
            vec![
                ("9223372036854775807", Ok("9.223372036854776e18")),
                ("-9223372036854775808", Ok("-9.223372036854776e18")),
                ("9007199254740993", Ok("9.007199254740992e15")),
                ("1.0", Err("INT")),
                ("9223372036854775808", Err("INT")),
            ],
        ),
    ] {
        let program = raw_program(vec![
            command(opcode, &[("key", "out.value"), ("source", "input.value")]),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        let binary = scratch.binary(&program, "Run");
        for (input, expected) in values {
            let values = state(&[("input.value", input)]);
            let vm = run_ail_bytecode_action(&program, "Run", values.clone());
            let elf = native(&binary, &resources(&[], &[]), &values);
            match expected {
                Ok(value) => {
                    assert_eq!(vm.unwrap().final_state["out.value"], value);
                    assert!(elf.status.success(), "{}", brief(&elf));
                    assert_eq!(elf.stdout, format!("out.value={value}\n").as_bytes());
                }
                Err(category) => {
                    assert!(
                        vm.unwrap_err()
                            .contains(&format!("AIL-RUNTIME-{category}:"))
                    );
                    failed(&elf, &format!("AIL-RUNTIME-{category}:"));
                }
            }
        }
        assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
        failed(
            &native(&binary, &resources(&[], &[]), &BTreeMap::new()),
            if opcode == "INT_TO_FLOAT" {
                "AIL-RUNTIME-INT:"
            } else {
                "AIL-RUNTIME-FLOAT:"
            },
        );
    }
    for expression in ["f:input.value d:0 /", "f:input.value d:-0 %"] {
        let program = raw_program(vec![command(
            "EVAL_FLOAT",
            &[("key", "out.value"), ("expression", expression)],
        )]);
        let values = state(&[("input.value", "nan")]);
        assert!(
            run_ail_bytecode_action(&program, "Run", values.clone())
                .unwrap_err()
                .contains("AIL-RUNTIME-DIVZERO:")
        );
        failed(
            &native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &values,
            ),
            "AIL-RUNTIME-DIVZERO:",
        );
    }
}

#[test]
fn json_float_bridges_preserve_negative_zero_and_lossless_json_neighbors() {
    let original = core(&source(
        "- the system parses JSON text data.raw into data.a\n- the system sets JSON value at path data.path in data.document to data.a into data.document\n- the system gets JSON value at path data.path from data.document into output.value\n- the system formats output.value as JSON text into output.raw\n- the system returns output.raw\n- the system returns data.document",
    ));
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let values = state(&[
            ("data.raw", " -0.000e+123 "),
            (
                "data.document",
                r#"{"keep":1.2345678901234567890123456789,"value":0}"#,
            ),
            ("data.path", "/value"),
        ]);
        let vm = run_ail_bytecode_action(&program, "Run", values.clone()).unwrap();
        assert_eq!(vm.final_state["output.value"], "-0e0");
        assert_eq!(
            vm.final_state["data.document"],
            r#"{"keep":1.2345678901234567890123456789,"value":-0e0}"#
        );
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &values,
        );
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(elf.stdout,b"data.document={\"keep\":1.2345678901234567890123456789,\"value\":-0e0}\noutput.raw=-0e0\n");
    }
    for (opcode, extras) in [
        ("JSON_PARSE", vec![]),
        ("JSON_FORMAT", vec![]),
        ("JSON_GET", vec![("path", "input.path")]),
        (
            "JSON_SET",
            vec![("path", "input.path"), ("value", "input.float")],
        ),
    ] {
        let mut operands = vec![
            ("key", "out.value"),
            ("source", "input.value"),
            ("type", "Float"),
        ];
        operands.extend(extras);
        let program = raw_program(vec![command(opcode, &operands)]);
        let binary = scratch.binary(&program, "Run");
        for raw in ["nan", "inf", "1e9999"] {
            let values = state(&[
                ("input.value", raw),
                ("input.path", ""),
                ("input.float", raw),
            ]);
            assert!(
                run_ail_bytecode_action(&program, "Run", values.clone())
                    .unwrap_err()
                    .contains("AIL-RUNTIME-JSON:"),
                "{opcode} {raw}"
            );
            failed(
                &native(&binary, &resources(&[], &[]), &values),
                "AIL-RUNTIME-JSON:",
            );
        }
    }
}

#[test]
fn float_function_parameters_validate_before_recursion_and_caller_values_stay_immutable() {
    let text = with_functions(
        "Function: descend.\nThe function needs:\n- n: Int\n- value: Float\nThe function produces:\n- result: Float\nThe function stores:\n- result: Float\nThe function executes:\n- if local.n == 0:\n- the function returns local.value\n- end if\n- the function computes floating local.result as -local.value\n- the function calls descend with n = local.n - 1; value = local.result into local.result\n- the function returns local.result",
        "- the system calls descend with n = data.n; value = data.a into output.value\n- the system returns output.value",
    );
    let program = compile_ail_core_bytecode(&core(&text)).unwrap();
    let scratch = Scratch::new();
    let binary = scratch.binary(&program, "Run");
    for (n, input, expected) in [
        ("63", "-0", Ok("0e0")),
        ("62", "-0", Ok("-0e0")),
        ("64", "1", Err("DEPTH")),
        ("100", "NaN", Err("FLOAT")),
    ] {
        let values = state(&[("data.a", input), ("data.n", n)]);
        let vm = run_ail_bytecode_action(&program, "Run", values.clone());
        let elf = native(&binary, &resources(&[], &[]), &values);
        match expected {
            Ok(expected) => {
                let vm = vm.unwrap();
                assert_eq!(vm.final_state["data.a"], input);
                assert_eq!(vm.final_state["output.value"], expected);
                assert!(elf.status.success(), "{}", brief(&elf));
                assert_eq!(elf.stdout, format!("output.value={expected}\n").as_bytes());
            }
            Err(category) => {
                assert!(
                    vm.unwrap_err()
                        .contains(&format!("AIL-RUNTIME-{category}:"))
                );
                failed(&elf, &format!("AIL-RUNTIME-{category}:"));
            }
        }
    }
}

#[test]
fn float_protocol_rejects_malformed_operands_profiles_and_local_type_forgery() {
    let good = raw_program(vec![command(
        "EVAL_FLOAT",
        &[("key", "out.value"), ("expression", "f:input.value d:1 +")],
    )]);
    let mut variants = Vec::new();
    for (key, value) in [
        ("expression", "i:1"),
        ("expression", "d:1 d:2"),
        ("expression", "d:1 +"),
        ("expression", "d:NaN"),
        ("expression", "f:badfield"),
        ("key", "not a field"),
        ("ignored", "metadata"),
    ] {
        let mut candidate = good.clone();
        candidate.actions.get_mut("Run").unwrap().instructions[0]
            .operands
            .insert(key.into(), value.into());
        variants.push(candidate);
    }
    for profile in ["Compiler", "UI", "AgentTool", "System"] {
        let mut candidate = good.clone();
        candidate.profile = profile.into();
        variants.push(candidate);
    }
    for instruction in [
        command("INT_TO_FLOAT", &[("key", "out.value")]),
        command(
            "FLOAT_TO_INT",
            &[
                ("key", "out.value"),
                ("source", "input.value"),
                ("extra", ""),
            ],
        ),
        command(
            "BRANCH_FLOAT",
            &[
                ("left", "d:1"),
                ("comparison", "="),
                ("right", "d:2"),
                ("label", "missing"),
            ],
        ),
        command(
            "BRANCH_FLOAT",
            &[
                ("left", "d:1"),
                ("comparison", "<"),
                ("right", "d:2"),
                ("label", "missing"),
            ],
        ),
    ] {
        variants.push(raw_program(vec![instruction]));
    }
    let mut local = raw_program(vec![command(
        "CALL_VALUE",
        &[
            ("key", "out.value"),
            ("target", "bad"),
            ("arguments", "x=f:input.value"),
        ],
    )]);
    local.actions.insert(
        "bad".into(),
        AilBytecodeAction {
            name: "bad".into(),
            instructions: vec![
                command(
                    "VALUE_FUNCTION",
                    &[
                        ("parameters", "x:Int"),
                        ("locals", "result:Float"),
                        ("result", "Float"),
                    ],
                ),
                command(
                    "EVAL_FLOAT",
                    &[("key", "local.result"), ("expression", "f:local.x d:1 +")],
                ),
                command(
                    "RETURN_VALUE",
                    &[("type", "Float"), ("expression", "f:local.result")],
                ),
            ],
        },
    );
    variants.push(local);
    for program in variants {
        assert!(
            !verify_ail_bytecode(&program).is_empty(),
            "accepted {:?}",
            program.actions
        );
        assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
        assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
        assert!(ail::ste::render_ail_ste(&program).is_err());
    }
}

#[test]
fn float_core_rejects_duplicate_edges_metadata_and_derived_secret_accesses() {
    let original = core(&source(
        "- the system computes floating output.value as data.a + data.b\n- if floating data.a < data.b:\n- the system converts floating data.a to integer output.count\n- end if",
    ));
    for kind in [
        "float_step",
        "float_destination",
        "float_source",
        "float_expression",
        "left_operand",
        "right_operand",
        "comparison_left",
    ] {
        let mut candidate = original.clone();
        let mut edge = candidate
            .graph
            .edges
            .iter()
            .find(|edge| edge.kind == kind)
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
    for kind in ["FloatOperation", "FloatExpression", "FloatComparison"] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == kind)
            .unwrap()
            .attributes
            .insert("ignored".into(), "payload".into());
        assert!(
            compile_ail_core_bytecode(&candidate).is_err(),
            "metadata {kind}"
        );
    }
    for (name, category) in [
        ("Data.a", "AIL005"),
        ("Data.b", "AIL005"),
        ("Output.value", "AIL002"),
        ("Output.count", "AIL002"),
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "Field" && node.name == name)
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        candidate
            .graph
            .edges
            .retain(|edge| !matches!(edge.kind.as_str(), "reads" | "writes"));
        let candidate = parse_ail_core_text(&render_ail_core(&candidate)).unwrap();
        let diagnostics = check_ail_core(&candidate);
        assert!(
            diagnostics.iter().any(|message| message.contains(category)),
            "{name}: {diagnostics:?}"
        );
        assert!(compile_ail_core_ste(&candidate).is_err());
    }
}

#[test]
fn nested_file_dataset_runs_float_computation_and_emits_a_json_report() {
    let text = source(
        "- the system reads text from file resource input into data.raw\n- the system parses JSON text data.raw into data.document\n- the system assigns \"/nested/value\" to data.path\n- the system gets JSON value at path data.path from data.document into data.a\n- the system computes floating output.value as (data.a + 0.5) / 2\n- the system formats output.value as JSON text into output.raw\n- the system writes text output.raw to file resource output\n- the system returns output.value",
    );
    let program = compile_ail_core_bytecode(&core(&text)).unwrap();
    let scratch = Scratch::new();
    let input = scratch.file(
        "input file=json",
        br#"{"nested":{"value":-9.5},"note":"\u0000\ud83d\ude42"}"#,
    );
    let output = scratch.file("output file=json", b"old");
    let grants = resources(&[("input", &input)], &[("output", &output)]);
    let vm = run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
    assert_eq!(vm.final_state["output.value"], "-4.5e0");
    assert_eq!(fs::read(&output).unwrap(), b"-4.5e0");
    fs::write(&output, b"old again").unwrap();
    let elf = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
    assert!(elf.status.success(), "{}", brief(&elf));
    assert_eq!(elf.stdout, b"output.value=-4.5e0\n");
    assert_eq!(fs::read(output).unwrap(), b"-4.5e0");
}

#[test]
fn direct_ste_and_source_do_not_bypass_float_types_or_profile_restrictions() {
    let program = raw_program(vec![command(
        "EVAL_FLOAT",
        &[("key", "out.value"), ("expression", "f:input.value d:1 +")],
    )]);
    let ste = ail::ste::render_ail_ste(&program).unwrap();
    for malformed in [
        ste.replace("f:input.value d:1 +", "f:input.value i:1 +"),
        ste.replace("f:input.value d:1 +", "f:input.value d:1 + d:2"),
        ste.replace("\"Application\"", "\"Compiler\""),
        ste.replace(
            "Compute floating expression",
            "Compute floating expressions",
        ),
    ] {
        assert_ne!(ste, malformed);
        assert!(ail::ste::parse_ail_ste(&malformed).is_err());
    }
    for body in [
        "- the system computes floating output.value as data.n + 1",
        "- the system computes floating output.count as data.a + 1",
        "- the system converts integer data.a to floating output.value",
        "- the system converts floating data.a to integer output.value",
        "- if floating data.n < 1:\n- the system assigns 1 to output.count\n- end if",
        "- the system assigns 1e9999 to output.value",
        "- the system assigns \"1.5\" to output.value",
    ] {
        let package = package();
        if let Ok(document) = parse_ail_package_spec_text(&package, &source(body)) {
            assert!(
                compile_ail_core_ste(&elaborate_ail_core(&package, &document)).is_err(),
                "accepted {body}"
            );
        }
    }
    let package = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for statement in [
        "the system computes floating output.value as data.a + 1",
        "the system converts integer data.n to floating output.value",
        "the system converts floating data.a to integer output.count",
    ] {
        let source = package.spec_text.replace(
            "- the system reads every edge",
            &format!("- {statement}\n- the system reads every edge"),
        );
        let error = parse_ail_package_spec_text(&package, &source).unwrap_err();
        assert!(error.contains("AIL-FLOAT-001"), "{statement}: {error}");
    }
}

#[test]
fn unused_float_parameters_validate_in_sorted_order_without_revealing_input() {
    let scratch = Scratch::new();
    for (parameters, expected) in [("a:Float;z:Int", "FLOAT"), ("a:Int;z:Float", "INT")] {
        let mut program = raw_program(vec![command(
            "CALL_VALUE",
            &[
                ("key", "out.value"),
                ("target", "constant"),
                ("arguments", "a=f:input.a;z=f:input.z"),
            ],
        )]);
        program.actions.insert(
            "constant".into(),
            AilBytecodeAction {
                name: "constant".into(),
                instructions: vec![
                    command(
                        "VALUE_FUNCTION",
                        &[
                            ("parameters", parameters),
                            ("locals", "result:Float"),
                            ("result", "Float"),
                        ],
                    ),
                    command(
                        "EVAL_FLOAT",
                        &[("key", "local.result"), ("expression", "d:1")],
                    ),
                    command(
                        "RETURN_VALUE",
                        &[("type", "Float"), ("expression", "f:local.result")],
                    ),
                ],
            },
        );
        let input = state(&[
            ("input.a", "PRIVATE_UNPARSEABLE_VALUE"),
            ("input.z", "PRIVATE_OTHER_VALUE"),
        ]);
        let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap_err();
        assert!(vm.contains(&format!("AIL-RUNTIME-{expected}:")), "{vm}");
        assert!(!vm.contains("PRIVATE_"));
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        failed(&elf, &format!("AIL-RUNTIME-{expected}:"));
        assert!(!String::from_utf8_lossy(&elf.stderr).contains("PRIVATE_"));
    }
}

#[test]
fn legacy_functions_reject_floating_execution_syntax_with_either_subject() {
    let package = package();
    for operation in [
        "computes floating local.result as local.value + 1",
        "converts integer local.n to floating local.result",
        "converts floating local.value to integer local.n",
    ] {
        for subject in ["the function", "the system"] {
            let text = format!(
                "{DECLARATIONS}Function: legacy.\nThe function needs:\n- value: Float\n- n: Int\nThe function produces:\n- result: Float\nWhen legacy runs:\n- {subject} {operation}\n- the function returns value unchanged\n"
            );
            let result = parse_ail_package_spec_text(&package, &text);
            assert!(
                result.is_err(),
                "legacy function accepted {subject} {operation}"
            );
            let error = result.unwrap_err();
            assert!(
                error.contains("AIL-FLOAT-001"),
                "{subject} {operation}: {error}"
            );
        }
    }
}

#[test]
fn descriptive_function_and_compiler_pass_bodies_reject_floating_controls() {
    let function_package = package();
    let compiler_package = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for (header, end) in [
        ("if floating local.value < 0:", "end if"),
        ("while floating local.value < 0:", "end while"),
    ] {
        let function_source = format!(
            "{DECLARATIONS}Function: legacy.\nThe function needs:\n- value: Float\nThe function produces:\n- result: Float\nWhen legacy runs:\n- {header}\n- the function returns value unchanged\n- {end}\n"
        );
        let pass_source = compiler_package.spec_text.replace(
            "- the system reads every edge",
            &format!("- {header}\n- the system reads every edge\n- {end}"),
        );
        for (package, text) in [
            (&function_package, function_source),
            (&compiler_package, pass_source),
        ] {
            let result = parse_ail_package_spec_text(package, &text);
            assert!(result.is_err(), "descriptive body accepted {header}");
            let error = result.unwrap_err();
            assert!(error.contains("AIL-FLOAT-001"), "{header}: {error}");
        }
    }
}
