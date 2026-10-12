use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application Lists manages collections.\n\nA Data has:\n- left: List<Int>\n- right: List<Int>\n- values: List<Int>\n- floats: List<Float>\n- texts: List<Text>\n- start: Int\n- end: Int\n- raw: Text\n\nAn Output has:\n- values: List<Int>\n- floats: List<Float>\n- texts: List<Text>\n- count: Int\n- raw: Text\n\n";

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
            "ail-list-transformations-{}-{}",
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
    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            let _ = stdout_reader.join().unwrap();
            let stderr = stderr_reader.join().unwrap();
            panic!(
                "List transformation blocked: {}",
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
fn list_transformations_execute_through_source_saved_core_rendered_ste_and_native() {
    let text = source(
        "- the system assigns [3,1,2] to data.left\n- the system assigns [0,2] to data.right\n- the system concatenates list data.left and list data.right into data.values\n- the system sorts list data.values in ascending order into data.values\n- the system slices list data.values from -4 to 99 into output.values\n- the system reverses list output.values into output.values\n- the system returns output.values",
    );
    let original = core(&text);
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let ste = compile_ail_core_ste(&candidate).unwrap();
        assert!(
            ste.contains("Concatenate "),
            "new operation was silently omitted"
        );
        let program = ail::ste::parse_ail_ste(&ste).unwrap();
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(vm.final_state["output.values"], "[3,2,2,1]");
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(output.stdout, b"output.values=[3,2,2,1]\n");
    }
}

fn seed(key: &str, value: &str) -> AilBytecodeInstruction {
    command(
        "SET_FIELD",
        &[("key", key), ("value", value), ("text", "fixture")],
    )
}

fn assert_backends(program: &AilBytecodeProgram, expected: Result<&str, &str>) {
    let vm = run_ail_bytecode_action(program, "Run", BTreeMap::new());
    let scratch = Scratch::new();
    let output = native(
        &scratch.binary(program, "Run"),
        &resources(&[], &[]),
        &BTreeMap::new(),
    );
    match expected {
        Ok(value) => {
            assert_eq!(vm.unwrap().final_state["out.value"], value);
            assert!(output.status.success(), "{}", brief(&output));
            assert_eq!(output.stdout, format!("out.value={value}\n").as_bytes());
        }
        Err(category) => {
            let marker = format!("AIL-RUNTIME-{category}:");
            assert!(vm.unwrap_err().contains(&marker));
            failed(&output, &marker);
        }
    }
}

#[test]
fn stable_float_sort_keeps_nan_last_and_signed_zero_order_in_both_directions() {
    let original = core(&source(
        "- the system sorts list data.floats in ascending order into output.floats\n- the system sorts list data.texts in descending order into output.texts\n- the system returns output.floats\n- the system returns output.texts",
    ));
    let input = state(&[
        ("data.floats", "[nan,-0,0,-inf,2,+nan,-0,inf,0,-2]"),
        (
            "data.texts",
            r#"["é","e\u0301","\u0000","a","aa","a","🙂"]"#,
        ),
    ]);
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(
            vm.final_state["output.floats"],
            "[-inf,-2e0,-0e0,0e0,-0e0,0e0,2e0,inf,nan,nan]"
        );
        assert_eq!(
            vm.final_state["output.texts"],
            "[\"🙂\",\"é\",\"é\",\"aa\",\"a\",\"a\",\"\\u0000\"]"
        );
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(
            output.stdout,
            format!(
                "output.floats={}\noutput.texts={}\n",
                vm.final_state["output.floats"], vm.final_state["output.texts"]
            )
            .as_bytes()
        );
    }
    let program = raw_program(vec![
        seed("data.values", &input["data.floats"]),
        command(
            "LIST_SORT",
            &[
                ("key", "out.value"),
                ("source", "data.values"),
                ("type", "Float"),
                ("order", "descending"),
            ],
        ),
        command("OUTPUT_FIELD", &[("key", "out.value")]),
    ]);
    assert_backends(
        &program,
        Ok("[inf,2e0,-0e0,0e0,-0e0,0e0,-2e0,-inf,nan,nan]"),
    );
}

#[test]
fn slices_clamp_extreme_bounds_but_validate_the_entire_source_and_both_expressions() {
    for (raw, start, end, expected) in [
        (
            "[0,1,2,3,4]",
            "i:-9223372036854775808",
            "i:9223372036854775807",
            Ok("[0,1,2,3,4]"),
        ),
        ("[0,1,2,3,4]", "i:-4", "i:-1", Ok("[1,2,3]")),
        ("[0,1,2,3,4]", "i:4", "i:-4", Ok("[]")),
        (
            "[]",
            "i:9223372036854775807",
            "i:-9223372036854775808",
            Ok("[]"),
        ),
        ("[0,true]", "i:999", "i:1 i:0 /", Err("LIST")),
        ("[0]", "f:missing.start", "i:1 i:0 /", Err("INT")),
        ("[0]", "i:1 i:0 /", "f:missing.end", Err("DIVZERO")),
        ("[]", "i:999", "i:1 i:0 /", Err("DIVZERO")),
        ("[]", "i:0", "i:9223372036854775807 i:1 +", Err("OVERFLOW")),
    ] {
        let program = raw_program(vec![
            seed("data.values", raw),
            command(
                "LIST_SLICE",
                &[
                    ("key", "out.value"),
                    ("source", "data.values"),
                    ("start", start),
                    ("end", end),
                    ("type", "Int"),
                ],
            ),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        assert_backends(&program, expected);
    }
}

#[test]
fn concat_validates_second_input_before_combined_limits_and_result_aliases_are_immutable() {
    for first in [
        format!("[{}]", vec!["0"; 65_536].join(",")),
        format!("[\"{}\"]", "\u{7f}".repeat(180_000)),
    ] {
        let element = if first.starts_with("[\"") {
            "Text"
        } else {
            "Int"
        };
        let program = raw_program(vec![
            seed("data.left", &first),
            seed("data.right", "[false]"),
            command(
                "LIST_CONCAT",
                &[
                    ("key", "out.value"),
                    ("source", "data.left"),
                    ("other", "data.right"),
                    ("type", element),
                ],
            ),
        ]);
        assert_backends(&program, Err("LIST"));
    }
    for element in ["Int", "Float", "Text"] {
        let (left, right, wanted) = match element {
            "Int" => ("[2,1]", "[3]", "[2,1,3]"),
            "Float" => ("[nan,-0]", "[0,inf]", "[nan,-0e0,0e0,inf]"),
            _ => (
                "[\"é\",\"\\u0000\"]",
                "[\"🙂\"]",
                "[\"é\",\"\\u0000\",\"🙂\"]",
            ),
        };
        for alias in ["data.left", "data.right"] {
            let program = raw_program(vec![
                seed("data.left", left),
                seed("data.right", right),
                command(
                    "COPY_FIELD",
                    &[
                        ("key", "data.saved"),
                        ("source", alias),
                        ("text", "snapshot"),
                    ],
                ),
                command(
                    "LIST_CONCAT",
                    &[
                        ("key", alias),
                        ("source", "data.left"),
                        ("other", "data.right"),
                        ("type", element),
                    ],
                ),
                command(
                    "COPY_FIELD",
                    &[("key", "out.value"), ("source", alias), ("text", "result")],
                ),
                command("OUTPUT_FIELD", &[("key", "out.value")]),
            ]);
            let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
            assert_eq!(
                vm.final_state["data.saved"],
                if alias == "data.left" { left } else { right }
            );
            assert_backends(&program, Ok(wanted));
        }
    }
}

#[test]
fn borrowed_canonical_overflow_can_be_sliced_but_full_results_enforce_bounds() {
    for (element, raw, small) in [
        (
            "Text",
            format!("[\"{}\",\"ok\"]", "\u{7f}".repeat(180_000)),
            "[\"ok\"]",
        ),
        (
            "Float",
            format!("[{}]", vec!["1234567890123"; 65_536].join(",")),
            "[1.234567890123e12]",
        ),
    ] {
        let program = raw_program(vec![
            seed("data.values", &raw),
            command(
                "LIST_SLICE",
                &[
                    ("key", "out.value"),
                    ("source", "data.values"),
                    ("start", "i:-1"),
                    ("end", "i:9223372036854775807"),
                    ("type", element),
                ],
            ),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        assert_backends(&program, Ok(small));
        for opcode in ["LIST_REVERSE", "LIST_SORT", "LIST_CONCAT"] {
            let mut operands = vec![
                ("key", "out.value"),
                ("source", "data.values"),
                ("type", element),
            ];
            if opcode == "LIST_SORT" {
                operands.push(("order", "ascending"));
            }
            if opcode == "LIST_CONCAT" {
                operands.push(("other", "data.empty"));
            }
            let program = raw_program(vec![
                seed("data.values", &raw),
                seed("data.empty", "[]"),
                command(opcode, &operands),
            ]);
            assert_backends(&program, Err("LIST-LIMIT"));
        }
    }
    for (size, expected) in [(1_048_572, Ok("1")), (1_048_573, Err("LIST-LIMIT"))] {
        let raw = format!("[\"{}\"]", "x".repeat(size));
        let program = raw_program(vec![
            seed("data.values", &raw),
            command(
                "LIST_REVERSE",
                &[
                    ("key", "data.result"),
                    ("source", "data.values"),
                    ("type", "Text"),
                ],
            ),
            command(
                "LIST_LENGTH",
                &[
                    ("key", "out.value"),
                    ("source", "data.result"),
                    ("type", "Text"),
                ],
            ),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        assert_backends(&program, expected);
    }
}

#[test]
fn function_local_transformations_preserve_frames_and_integer_bound_expressions() {
    let functions = "Function: window.\nThe function needs:\n- values: List<Int>\n- start: Int\n- end: Int\nThe function produces:\n- result: List<Int>\nThe function stores:\n- result: List<Int>\nThe function executes:\n- the function sorts list local.values in ascending order into local.result\n- the function slices list local.result from local.start + 1 to local.end - 1 into local.result\n- the function reverses list local.result into local.result\n- the function concatenates list local.result and list local.result into local.result\n- the function returns local.result";
    let original = core(&with_functions(
        functions,
        "- the system calls window with values = data.values; start = data.start; end = data.end into output.values\n- the system returns output.values",
    ));
    let input = state(&[
        ("data.values", "[3,0,2,1]"),
        ("data.start", "0"),
        ("data.end", "4"),
    ]);
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(vm.final_state["output.values"], "[2,1,2,1]");
        assert!(!vm.final_state.keys().any(|key| key.starts_with("local.")));
        assert!(
            !vm.trace
                .iter()
                .any(|line| line.contains("computed list local."))
        );
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(output.stdout, b"output.values=[2,1,2,1]\n");
    }
}

#[test]
fn exact_protocol_type_order_operands_and_profiles_reject_malformed_instructions() {
    for element in ["Int", "Float", "Text"] {
        for (opcode, extra) in [
            ("LIST_CONCAT", vec![("other", "data.right")]),
            ("LIST_SLICE", vec![("start", "i:-1"), ("end", "f:data.end")]),
            ("LIST_REVERSE", vec![]),
            ("LIST_SORT", vec![("order", "ascending")]),
        ] {
            let mut operands = vec![
                ("key", "out.value"),
                ("source", "data.values"),
                ("type", element),
            ];
            operands.extend(extra);
            let original = raw_program(vec![command(opcode, &operands)]);
            let ste = ail::ste::render_ail_ste(&original).unwrap();
            for removed in operands.iter().map(|(key, _)| *key).chain(["extra"]) {
                let mut forged = original.clone();
                let args = &mut forged.actions.get_mut("Run").unwrap().instructions[0].operands;
                if removed == "extra" {
                    args.insert("extra".into(), "ignored".into());
                } else {
                    args.remove(removed);
                }
                assert!(
                    verify_ail_bytecode(&forged)
                        .iter()
                        .any(|d| d.contains("AILBC013")),
                    "{opcode}/{element}/{removed}"
                );
                assert!(ail::ste::render_ail_ste(&forged).is_err());
                assert!(
                    compile_ail_bytecode_native_elf(&forged, "Run", "linux-x86_64-elf").is_err()
                );
            }
            for profile in ["Compiler", "System", "UI", "AgentTool"] {
                let mut forged = original.clone();
                forged.profile = profile.into();
                assert!(!verify_ail_bytecode(&forged).is_empty());
                assert!(
                    ail::ste::parse_ail_ste(
                        &ste.replace("\"Application\"", &format!("\"{profile}\""))
                    )
                    .is_err()
                );
            }
            for (key, value) in [("key", "bare"), ("source", ""), ("type", "Bool")] {
                let mut forged = original.clone();
                forged.actions.get_mut("Run").unwrap().instructions[0]
                    .operands
                    .insert(key.into(), value.into());
                assert!(!verify_ail_bytecode(&forged).is_empty());
            }
            if opcode == "LIST_SORT" {
                for order in ["", "Ascending", "reverse", "asc"] {
                    let mut forged = original.clone();
                    forged.actions.get_mut("Run").unwrap().instructions[0]
                        .operands
                        .insert("order".into(), order.into());
                    assert!(!verify_ail_bytecode(&forged).is_empty());
                }
            }
        }
    }
}

#[test]
fn core_exact_metadata_operand_ownership_and_all_semantic_secret_reads_are_checked() {
    let original = core(&source(
        "- the system concatenates list data.left and list data.right into data.values\n- the system slices list data.values from data.start + 1 to data.end - 1 into output.values\n- the system sorts list output.values in descending order into output.values",
    ));
    for kind in [
        "list_step",
        "list_source",
        "list_other",
        "list_destination",
        "list_start",
        "list_end",
    ] {
        let edge = original
            .graph
            .edges
            .iter()
            .find(|edge| edge.kind == kind)
            .unwrap()
            .clone();
        let mut duplicate = original.clone();
        let mut extra = edge.clone();
        extra.id = format!("duplicate-{kind}");
        extra.ordinal = duplicate.graph.edges.len();
        duplicate.graph.edges.push(extra);
        assert!(
            compile_ail_core_bytecode(&duplicate).is_err(),
            "duplicate {kind}"
        );
        let mut metadata = original.clone();
        metadata
            .graph
            .edges
            .iter_mut()
            .find(|e| e.id == edge.id)
            .unwrap()
            .attributes
            .insert("ignored".into(), "true".into());
        assert!(
            compile_ail_core_bytecode(&metadata).is_err(),
            "metadata {kind}"
        );
    }
    for name in [
        "Data.left",
        "Data.right",
        "Data.start",
        "Data.end",
        "Output.values",
    ] {
        let mut secret = original.clone();
        secret
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name == name)
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        secret
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        let code = if name == "Output.values" {
            "AIL002"
        } else {
            "AIL005"
        };
        let secret = parse_ail_core_text(&render_ail_core(&secret)).unwrap();
        assert!(
            check_ail_core(&secret).iter().any(|d| d.contains(code)),
            "{name}"
        );
    }
    for (attribute, value) in [
        ("sort_order", "invalid"),
        ("ignored", "true"),
        ("element_type", "Bool"),
    ] {
        let mut forged = original.clone();
        forged
            .graph
            .nodes
            .iter_mut()
            .find(|n| {
                n.kind == "ListOperation"
                    && n.attributes.get("list_kind").is_some_and(|v| v == "sort")
            })
            .unwrap()
            .attributes
            .insert(attribute.into(), value.into());
        assert!(compile_ail_core_ste(&forged).is_err());
    }
    for name in ["Data.right", "Data.start"] {
        let mut wrong = original.clone();
        wrong
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name == name)
            .unwrap()
            .type_name = Some("Text".into());
        assert!(compile_ail_core_bytecode(&wrong).is_err());
    }
    let mut forged = original.clone();
    let mut field = forged
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Field" && n.name == "Data.right")
        .unwrap()
        .clone();
    let old = field.id.clone();
    field.id = "forged-list-field".into();
    forged.graph.nodes.push(field.clone());
    forged
        .graph
        .edges
        .iter_mut()
        .find(|e| e.kind == "list_other" && e.target == old)
        .unwrap()
        .target = field.id;
    assert!(compile_ail_core_bytecode(&forged).is_err());
}

#[test]
fn legacy_compiler_and_executable_bodies_reject_punctuated_malformed_list_forms() {
    let application = package();
    let compiler = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for statement in [
        "concatenates list data.left and list data.right into data.values",
        "slices list data.values from 0 to 1 into data.values",
        "reverses list data.values into data.values",
        "sorts list data.values in ascending order into data.values",
        "concatenates list",
        "slices list",
        "reverses list",
        "sorts list",
        "concatenates",
        "reverses",
        "sorts",
    ] {
        for subject in ["the system", "the function"] {
            for punctuation in ["", "."] {
                let statement = format!("{subject} {statement}{punctuation}");
                let legacy = format!(
                    "{DECLARATIONS}Function: legacy.\nThe function needs:\n- values: List<Int>\nThe function produces:\n- result: List<Int>\nWhen legacy runs:\n- {statement}\n- the function returns values unchanged\n"
                );
                let pass = compiler.spec_text.replace(
                    "- the system reads every edge",
                    &format!("- {statement}\n- the system reads every edge"),
                );
                for (package, text) in [(&application, legacy), (&compiler, pass)] {
                    assert!(
                        parse_ail_package_spec_text(package, &text).is_err(),
                        "accepted {statement}"
                    );
                }
            }
        }
    }
    for statement in [
        "sorts list data.values into output.values",
        "sorts list data.values in reverse order into output.values",
        "slices list data.values from 0 into output.values",
        "concatenates list data.left and data.right into output.values",
    ] {
        let text = source(&format!("- the system {statement}"));
        let rejected = match parse_ail_package_spec_text(&application, &text) {
            Err(_) => true,
            Ok(document) => {
                compile_ail_core_bytecode(&elaborate_ail_core(&application, &document)).is_err()
            }
        };
        assert!(rejected, "accepted {statement}");
    }
}

#[test]
fn imported_lists_and_bound_fields_execute_without_renaming_sort_enums() {
    let mut child = package();
    child.metadata.name = "list-child".into();
    child.spec_text = source(
        "- the system concatenates list data.left and list data.right into data.values\n- the system sorts list data.values in descending order into data.values\n- the system slices list data.values from data.start + 1 to data.end into output.values\n- the system reverses list output.values into output.values\n- the system returns output.values",
    );
    let mut parent = package();
    parent.spec_text="The application Parent manages lists.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system returns child.output.values\n- the system records a trace event named Done\n".into();
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
        ("child.data.left", "[4,1]"),
        ("child.data.right", "[3,2]"),
        ("child.data.start", "0"),
        ("child.data.end", "-1"),
    ]);
    assert_eq!(
        run_ail_bytecode_action(&program, "Run", input.clone())
            .unwrap()
            .final_state["child.output.values"],
        "[2,3]"
    );
    let scratch = Scratch::new();
    let output = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(output.stdout, b"child.output.values=[2,3]\n");
}

#[test]
fn file_lists_sort_slice_reverse_concat_and_write_exact_json_report() {
    let text = source(
        "- the system reads text from file resource input into data.raw\n- the system parses JSON text data.raw into data.values\n- the system sorts list data.values in ascending order into data.values\n- the system slices list data.values from -3 to 999 into data.left\n- the system reverses list data.left into data.right\n- the system concatenates list data.left and list data.right into output.values\n- the system formats output.values as JSON text into output.raw\n- the system writes text output.raw to file resource report",
    );
    let program = compile_ail_core_bytecode(&core(&text)).unwrap();
    let scratch = Scratch::new();
    let input = scratch.file("input file=json", b"[8,-4,0,3,8,2]");
    let report = scratch.path("report file=json");
    let grants = resources(&[("input", &input)], &[("report", &report)]);
    run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
    assert_eq!(fs::read(&report).unwrap(), b"[3,8,8,8,8,3]");
    let output = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(fs::read(report).unwrap(), b"[3,8,8,8,8,3]");
}

#[test]
fn bound_expression_ownership_and_local_operation_scope_cannot_be_forged() {
    let original = core(&source(
        "- the system slices list data.values from data.start to data.end into output.values",
    ));
    let start = original
        .graph
        .edges
        .iter()
        .find(|edge| edge.kind == "list_start")
        .unwrap()
        .target
        .clone();
    let mut shared = original.clone();
    shared
        .graph
        .edges
        .iter_mut()
        .find(|edge| edge.kind == "list_end")
        .unwrap()
        .target = start;
    assert!(compile_ail_core_bytecode(&shared).is_err());
    for mutation in ["type", "metadata", "orphan"] {
        let mut forged = original.clone();
        let node = forged
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "ListOperation")
            .unwrap();
        match mutation {
            "type" => node.type_name = Some("List<Int>".into()),
            "metadata" => {
                node.attributes
                    .insert("sort_order".into(), "ascending".into());
            }
            _ => {
                let mut extra = node.clone();
                extra.id = "orphan-list-operation".into();
                forged.graph.nodes.push(extra);
            }
        }
        assert!(compile_ail_core_bytecode(&forged).is_err(), "{mutation}");
    }
    let function = "Function: window.\nThe function needs:\n- values: List<Int>\n- start: Int\nThe function produces:\n- result: List<Int>\nThe function stores:\n- result: List<Int>\nThe function executes:\n- the function slices list local.values from local.start to 99 into local.result\n- the function returns local.result";
    let original = core(&with_functions(
        function,
        "- the system calls window with values = data.values; start = data.start into output.values",
    ));
    let step = original
        .graph
        .nodes
        .iter()
        .find(|node| {
            node.kind == "ValueFunctionStep"
                && node
                    .attributes
                    .get("opcode")
                    .is_some_and(|opcode| opcode == "LIST_SLICE")
        })
        .unwrap();
    let field_edge = original
        .graph
        .edges
        .iter()
        .find(|edge| {
            edge.kind == "value_field_operand"
                && edge.source == step.id
                && edge
                    .attributes
                    .get("name")
                    .is_some_and(|role| role == "source")
        })
        .unwrap();
    for mutation in [
        "global_list",
        "foreign_local",
        "wrong_type",
        "duplicate_role",
    ] {
        let mut forged = original.clone();
        match mutation {
            "global_list" => {
                let id = forged
                    .graph
                    .nodes
                    .iter()
                    .find(|node| node.kind == "Field" && node.name == "Data.values")
                    .unwrap()
                    .id
                    .clone();
                forged
                    .graph
                    .edges
                    .iter_mut()
                    .find(|edge| edge.id == field_edge.id)
                    .unwrap()
                    .target = id;
            }
            "foreign_local" => {
                forged
                    .graph
                    .nodes
                    .iter_mut()
                    .find(|node| node.id == field_edge.target)
                    .unwrap()
                    .attributes
                    .insert("function_owner".into(), "foreign".into());
            }
            "wrong_type" => {
                forged
                    .graph
                    .edges
                    .iter_mut()
                    .find(|edge| edge.id == field_edge.id)
                    .unwrap()
                    .attributes
                    .insert("type".into(), "List<Text>".into());
            }
            _ => {
                let mut edge = field_edge.clone();
                edge.id = "duplicate-list-local".into();
                edge.ordinal = forged.graph.edges.len();
                forged.graph.edges.push(edge);
            }
        }
        assert!(compile_ail_core_bytecode(&forged).is_err(), "{mutation}");
    }
}
