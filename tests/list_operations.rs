use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

const DECLARATIONS: &str = "The application Lists manages batches.\n\nAn Input has:\n- values: List<Int>\n- names: List<Text>\n- index: Int\n- replacement: Text\n\nAn Output has:\n- values: List<Int>\n- names: List<Text>\n- count: Int\n- total: Int\n- item: Int\n- name: Text\n\n";
const NUMBERS: &str = "Function: square.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function executes:\n- the function returns local.n * local.n\n\nAction: Numbers.\nWhen Numbers happens:\n- the system creates an empty list into output.values\n- the system counts items in list input.values into output.count\n- the system computes input.index as 0\n- the system computes output.total as 0\n- while input.index < output.count:\n- the system gets item input.index from list input.values into output.item\n- the system calls square with n = output.item into output.item\n- the system appends integer output.item to list output.values into output.values\n- the system computes output.total as output.total + output.item\n- the system computes input.index as input.index + 1\n- end while\n- the system returns output.values\n- the system returns output.total\n- the system returns output.count\n- the system records a trace event named Calculated\n";
const NAMES: &str = "Action: Names.\nWhen Names happens:\n- the system counts items in list input.names into output.count\n- the system gets item input.index from list input.names into output.name\n- the system appends text input.replacement to list input.names into output.names\n- the system sets item -1 in list output.names to text output.name into output.names\n- the system returns output.names\n- the system returns output.name\n- the system returns output.count\n- the system records a trace event named Edited\n";

fn source(body: &str) -> String {
    format!("{DECLARATIONS}{body}")
}
fn package() -> AilPackage {
    load_ail_package_dir("bench/R001-add/program.ail").unwrap()
}
fn core(source: &str) -> AilCore {
    let package = package();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}
fn state(values: &[(&str, &str)]) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}
fn native(core: &AilCore, action: &str, state: &BTreeMap<String, String>) -> Output {
    run_binary(
        compile_ail_core_native_elf(core, action, "linux-x86_64-elf").unwrap(),
        state,
    )
}
fn run_binary(bytes: Vec<u8>, state: &BTreeMap<String, String>) -> Output {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "ail-list-{}-{}.elf",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let arguments = state
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>();
    let mut retries = 0;
    let output = loop {
        match Command::new(&path).args(&arguments).output() {
            Ok(output) => break output,
            Err(error) if error.raw_os_error() == Some(26) && retries < 10 => {
                retries += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => panic!("{error}"),
        }
    };
    fs::remove_file(path).unwrap();
    output
}

#[test]
fn integer_list_map_and_reduction_execute_through_core_ste_and_native() {
    let original = core(&source(NUMBERS));
    assert_eq!(
        original
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind == "ListOperation")
            .count(),
        4
    );
    assert!(
        check_ail_core(&original).is_empty(),
        "{:?}",
        check_ail_core(&original)
    );
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        for (numbers, values, total, count) in
            [("[1, -2, 3]", "[1,4,9]", "14", "3"), ("[]", "[]", "0", "0")]
        {
            let input = state(&[("input.values", numbers)]);
            let result = run_ail_bytecode_action(&program, "Numbers", input.clone()).unwrap();
            assert_eq!(result.final_state["input.values"], numbers);
            assert_eq!(result.final_state["output.values"], values);
            assert_eq!(result.final_state["output.total"], total);
            let output = native(&candidate, "Numbers", &input);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                format!("output.count={count}\noutput.total={total}\noutput.values={values}\n")
            );
        }
    }
}

#[test]
fn unicode_text_lists_decode_json_escapes_and_preserve_value_semantics() {
    let original = core(&source(NAMES));
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let input = state(&[
            (
                "input.names",
                r#"[ "é", "\ud83d\ude42", "line\nnext", "\u0085" ]"#,
            ),
            ("input.index", "-3"),
            ("input.replacement", "new"),
        ]);
        let result = run_ail_bytecode_action(&program, "Names", input.clone()).unwrap();
        assert_eq!(result.final_state["input.names"], input["input.names"]);
        assert_eq!(result.final_state["output.name"], "🙂");
        assert_eq!(
            result.final_state["output.names"],
            "[\"é\",\"🙂\",\"line\\nnext\",\"\\u0085\",\"🙂\"]"
        );
        let output = native(&candidate, "Names", &input);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "output.count=4\noutput.name=🙂\noutput.names=[\"é\",\"🙂\",\"line\\nnext\",\"\\u0085\",\"🙂\"]\n"
        );
    }
}

fn operation(body: &str, output: &str) -> String {
    source(&format!(
        "Action: Run.\nWhen Run happens:\n{body}\n- the system returns {output}\n- the system records a trace event named Completed\n"
    ))
}

#[test]
fn list_inputs_are_strictly_typed_flat_json_and_indices_are_checked() {
    let get_integer = core(&operation(
        "- the system gets item input.index from list input.values into output.item",
        "output.item",
    ));
    let get_text = core(&operation(
        "- the system gets item input.index from list input.names into output.name",
        "output.name",
    ));
    for (core, key, value, index, error) in [
        (
            &get_integer,
            "input.values",
            "[1, \"2\"]",
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_integer,
            "input.values",
            "[01]",
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_integer,
            "input.values",
            "[+1]",
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_integer,
            "input.values",
            "[1e0]",
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_integer,
            "input.values",
            "[1.0]",
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_integer,
            "input.values",
            "[9223372036854775808]",
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_integer,
            "input.values",
            "[-9223372036854775809]",
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_integer,
            "input.values",
            "[[[[]]]]",
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_integer,
            "input.values",
            "[1,]",
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_integer,
            "input.values",
            "[1] trailing",
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_integer,
            "input.values",
            "[1,2]",
            "2",
            "AIL-RUNTIME-INDEX",
        ),
        (
            &get_integer,
            "input.values",
            "[1,2]",
            "-3",
            "AIL-RUNTIME-INDEX",
        ),
        (
            &get_integer,
            "input.values",
            "[1,2]",
            "-9223372036854775808",
            "AIL-RUNTIME-INDEX",
        ),
        (&get_integer, "input.values", "[]", "0", "AIL-RUNTIME-INDEX"),
        (&get_text, "input.names", "[1]", "0", "AIL-RUNTIME-LIST"),
        (
            &get_text,
            "input.names",
            r#"["\ud800"]"#,
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_text,
            "input.names",
            r#"["\ud800\u0041"]"#,
            "0",
            "AIL-RUNTIME-LIST",
        ),
        (
            &get_text,
            "input.names",
            "[\"a\nb\"]",
            "0",
            "AIL-RUNTIME-LIST",
        ),
    ] {
        let input = state(&[(key, value), ("input.index", index)]);
        let program = compile_ail_core_bytecode(core).unwrap();
        let vm_error = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap_err();
        assert!(vm_error.contains(error), "{value} {index}: {vm_error}");
        let output = native(core, "Run", &input);
        assert!(!output.status.success(), "{value} {index}");
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(error),
            "{value} {index}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let input = state(&[
        (
            "input.values",
            "[ -9223372036854775808, -0, 9223372036854775807 ]",
        ),
        ("input.index", "-1"),
    ]);
    let program = compile_ail_core_bytecode(&get_integer).unwrap();
    assert_eq!(
        run_ail_bytecode_action(&program, "Run", input.clone())
            .unwrap()
            .final_state["output.item"],
        "9223372036854775807"
    );
    assert_eq!(
        native(&get_integer, "Run", &input).stdout,
        b"output.item=9223372036854775807\n"
    );
}

#[test]
fn list_copy_and_updates_keep_previous_values_and_read_before_writing() {
    let source = operation(
        "- the system changes the input values as the output values\n- the system appends integer 7 to list input.values into input.values\n- the system sets item -1 in list input.values to integer input.index + 1 into input.values\n- the system gets item -1 from list input.values into output.item\n- the system returns output.item",
        "output.values",
    );
    let original = core(&source);
    for candidate in [
        original.clone(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let input = state(&[("input.values", "[1,2]"), ("input.index", "8")]);
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(result.final_state["output.values"], "[1,2]");
        assert_eq!(result.final_state["input.values"], "[1,2,9]");
        assert_eq!(result.final_state["output.item"], "9");
        let output = native(&candidate, "Run", &input);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"output.item=9\noutput.values=[1,2]\n");
    }
}

#[test]
fn list_type_ownership_order_and_protocol_are_checked() {
    let valid = source(&format!("{NUMBERS}\n{NAMES}"));
    for invalid in [
        valid.replace(
            "list input.values into output.count",
            "list input.index into output.count",
        ),
        valid.replace(
            "empty list into output.values",
            "empty list into output.item",
        ),
        valid.replace(
            "list input.names into output.name",
            "list input.names into output.item",
        ),
        valid.replace(
            "appends text input.replacement",
            "appends integer input.index",
        ),
        valid.replace(
            "gets item input.index from",
            "gets item input.replacement from",
        ),
        valid.replace("into output.names", "into output.values"),
    ] {
        let candidate = core(&invalid);
        assert!(!check_ail_core(&candidate).is_empty());
        assert!(compile_ail_core_bytecode(&candidate).is_err());
    }
    let original = core(&valid);
    for kind in [
        "list_source",
        "list_destination",
        "list_index",
        "list_value",
        "list_step",
    ] {
        let mut candidate = original.clone();
        candidate.graph.edges.push(
            candidate
                .graph
                .edges
                .iter()
                .find(|e| e.kind == kind)
                .unwrap()
                .clone(),
        );
        let saved = parse_ail_core_text(&render_ail_core(&candidate)).unwrap();
        assert!(!check_ail_core(&saved).is_empty(), "{kind}");
        assert!(compile_ail_core_bytecode(&saved).is_err());
    }
    for profile in ["Compiler", "AgentTool", "UI", "System"] {
        let mut candidate = original.clone();
        candidate.package.profile = profile.into();
        assert!(
            check_ail_core(&candidate)
                .iter()
                .any(|e| e.contains("AIL-LIST-")),
            "{profile}"
        );
    }
    let mut wrong_kind = original.clone();
    wrong_kind
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "ListOperation")
        .unwrap()
        .attributes
        .insert("element_type".into(), "Decimal".into());
    assert!(compile_ail_core_bytecode(&wrong_kind).is_err());
    let mut compiler_pass = original.clone();
    compiler_pass
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Action" && n.name == "Names")
        .unwrap()
        .attributes
        .insert("kind".into(), "CompilerPass".into());
    assert!(
        check_ail_core(&compiler_pass)
            .iter()
            .any(|e| e.contains("AIL-LIST-"))
    );
    let program = compile_ail_core_bytecode(&original).unwrap();
    for (opcode, operand, value) in [
        ("LIST_EMPTY", "hidden", "ignored"),
        ("LIST_LENGTH", "type", "Decimal"),
        ("LIST_GET", "index", "i:1 garbage"),
        ("LIST_APPEND", "value", "i:1 extra"),
        ("LIST_SET", "value", "not a field"),
    ] {
        let mut candidate = program.clone();
        let instruction = candidate
            .actions
            .values_mut()
            .flat_map(|a| a.instructions.iter_mut())
            .find(|i| i.opcode == opcode)
            .unwrap();
        instruction.operands.insert(operand.into(), value.into());
        assert!(!verify_ail_bytecode(&candidate).is_empty(), "{opcode}");
        assert!(ail::ste::render_ail_ste(&candidate).is_err());
    }
}

#[test]
fn list_secret_rules_cover_semantic_operands_without_descriptive_accesses() {
    let body = "- the system gets item input.index from list input.values into output.item\n- the system appends integer input.index + 1 to list input.values into output.values\n- the system appends text input.replacement to list input.names into output.names";
    let original = core(&operation(body, "output.values"));
    for (field, code) in [
        ("Input.values", "AIL005"),
        ("Input.names", "AIL005"),
        ("Input.index", "AIL005"),
        ("Input.replacement", "AIL005"),
        ("Output.values", "AIL002"),
        ("Output.item", "AIL002"),
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name == field)
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        candidate
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        let saved = parse_ail_core_text(&render_ail_core(&candidate)).unwrap();
        let diagnostics = check_ail_core(&saved).join("\n");
        assert!(diagnostics.contains(code), "{field}: {diagnostics}");
        assert!(compile_ail_core_bytecode(&saved).is_err());
        let owner = candidate
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "Action" && n.name == "Run")
            .unwrap()
            .clone();
        let target = candidate
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "Field" && n.name == field)
            .unwrap()
            .clone();
        candidate
            .graph
            .add_edge("protects_secret", &owner, &target, BTreeMap::new());
        assert!(
            check_ail_core(&candidate).is_empty(),
            "{:?}",
            check_ail_core(&candidate)
        );
    }
}

#[test]
fn list_validation_and_value_evaluation_have_deterministic_error_order() {
    let candidate = core(&operation(
        "- the system sets item input.index in list input.values to integer 1 / 0 into output.values",
        "output.values",
    ));
    let program = compile_ail_core_bytecode(&candidate).unwrap();
    for (values, index, error) in [
        ("bad", "bad", "AIL-RUNTIME-LIST"),
        ("[1]", "bad", "AIL-RUNTIME-INT"),
        ("[1]", "2", "AIL-RUNTIME-INDEX"),
        ("[1]", "0", "AIL-RUNTIME-DIVZERO"),
    ] {
        let input = state(&[("input.values", values), ("input.index", index)]);
        assert!(
            run_ail_bytecode_action(&program, "Run", input.clone())
                .unwrap_err()
                .contains(error)
        );
        let output = native(&candidate, "Run", &input);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(error),
            "{error}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn read_and_replacement_accept_inputs_whose_canonical_form_would_exceed_limit() {
    let raw = format!("[\"{}\"]", "\u{7f}".repeat(200_000));
    let cases = [
        (
            "- the system counts items in list input.names into output.count",
            "output.count",
            "output.count=1\n",
        ),
        (
            "- the system sets item 0 in list input.names to text input.replacement into output.names",
            "output.names",
            "output.names=[\"ok\"]\n",
        ),
    ];
    for (body, key, expected) in cases {
        let mut program = compile_ail_core_bytecode(&core(&operation(body, key))).unwrap();
        program.actions.get_mut("Run").unwrap().instructions.insert(
            0,
            AilBytecodeInstruction {
                opcode: "SET_FIELD".into(),
                operands: state(&[
                    ("key", "input.names"),
                    ("value", &raw),
                    ("text", "test literal"),
                ]),
            },
        );
        let input = state(&[("input.replacement", "ok")]);
        let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(format!("{key}={}\n", result.final_state[key]), expected);
        let output = run_binary(
            compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").unwrap(),
            &input,
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}

#[test]
fn list_and_extracted_text_budgets_are_shared_across_repeated_action_calls() {
    let raw = format!("[\"{}\"]", "x".repeat(1_048_560));
    for (step, error) in [
        (
            "the system appends text input.replacement to list input.names into output.names",
            "AIL-RUNTIME-LIST-LIMIT",
        ),
        (
            "the system gets item 0 from list input.names into output.name",
            "AIL-RUNTIME-TEXT-LIMIT",
        ),
    ] {
        let source = source(&format!(
            "Action: Run.\nWhen Run happens:\n- the system repeats Worker 2 times\n- the system records a trace event named Finished\n\nAction: Worker.\nWhen Worker happens:\n- the system computes input.index as 0\n- while input.index < 35:\n- {step}\n- the system computes input.index as input.index + 1\n- end while\n- the system records a trace event named Worked\n"
        ));
        let mut program = compile_ail_core_bytecode(&core(&source)).unwrap();
        program.actions.get_mut("Run").unwrap().instructions.insert(
            0,
            AilBytecodeInstruction {
                opcode: "SET_FIELD".into(),
                operands: state(&[
                    ("key", "input.names"),
                    ("value", &raw),
                    ("text", "test literal"),
                ]),
            },
        );
        let input = state(&[("input.replacement", "")]);
        let vm_error = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap_err();
        assert!(vm_error.contains(error), "{vm_error}");
        let output = run_binary(
            compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").unwrap(),
            &input,
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(error),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn public_document_and_imported_list_actions_execute() {
    let mut child = package();
    child.metadata.name = "list-child".into();
    child.spec_text = source(NUMBERS);
    let document = parse_ail_package_document(&child).unwrap();
    assert_eq!(
        run_ail_action(&document, "Numbers", state(&[("input.values", "[2,3]")]))
            .unwrap()
            .final_state["output.total"],
        "13"
    );
    let mut parent = package();
    parent.spec_text = "The application Parent manages lists.\nAction: Run.\nWhen Run happens:\n- the system calls Batch.Numbers\n- the system returns batch.output.values\n- the system records a trace event named Finished\n".into();
    parent.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Batch".into(),
            resolved_package: None,
            registry_identity: None,
        },
        package: Box::new(child),
    });
    let original = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let input = state(&[("batch.input.values", "[2,3]")]);
        assert_eq!(
            run_ail_bytecode_action(&program, "Run", input.clone())
                .unwrap()
                .output_state(),
            state(&[("batch.output.values", "[4,9]")])
        );
        let output = native(&candidate, "Run", &input);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"batch.output.values=[4,9]\n");
    }
}
