use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

const DECLARATIONS: &str = "The application Constants manages values.\n\nA Data has:\n- value: Int\n- flag: Bool\n- text: Text\n- suffix: Text\n- numbers: List<Int>\n- names: List<Text>\n- copy: List<Text>\n\nAn Output has:\n- value: Int\n- flag: Bool\n- text: Text\n- count: Int\n- names: List<Text>\n\n";
const BODY: &str = r#"Action: Run.
When Run happens:
- the system assigns -9223372036854775808 to data.value
- the system assigns true to output.flag
- the system assigns "Hello, to data.text.\n\u0000\ud83d\ude42" to data.text
- the system assigns "!" to data.suffix
- the system assigns [1, -2, 3] to data.numbers
- the system assigns ["é", "\ud83d\ude42", "line\nnext"] to data.names
- the system joins text data.text and data.suffix into output.text
- the system counts items in list data.numbers into output.count
- the system gets item -2 from list data.numbers into output.value
- the system appends text data.suffix to list data.names into output.names
- the system returns output.value
- the system returns output.flag
- the system returns output.text
- the system returns output.count
- the system returns output.names
- the system records a trace event named Constructed
"#;

fn package() -> AilPackage {
    load_ail_package_dir("bench/R001-add/program.ail").unwrap()
}
fn source(body: &str) -> String {
    format!("{DECLARATIONS}{body}")
}
fn core(source: &str) -> AilCore {
    let package = package();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}
fn native(program: &AilCore, action: &str) -> Output {
    run_binary(compile_ail_core_native_elf(program, action, "linux-x86_64-elf").unwrap())
}
fn run_binary(bytes: Vec<u8>) -> Output {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "ail-literal-{}-{}.elf",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let mut retries = 0;
    let result = loop {
        match Command::new(&path).output() {
            Ok(output) => break output,
            Err(error) if error.raw_os_error() == Some(26) && retries < 10 => {
                retries += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => panic!("{error}"),
        }
    };
    fs::remove_file(path).unwrap();
    result
}

#[test]
fn typed_constants_construct_values_through_source_core_ste_and_native() {
    let original = core(&source(BODY));
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
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(result.final_state["data.value"], "-9223372036854775808");
        assert_eq!(result.final_state["output.flag"], "true");
        assert_eq!(
            result.final_state["output.text"],
            "Hello, to data.text.\n\0🙂!"
        );
        assert_eq!(
            result.final_state["output.names"],
            "[\"é\",\"🙂\",\"line\\nnext\",\"!\"]"
        );
        assert_eq!(result.final_state["output.value"], "-2");
        let output = native(&candidate, "Run");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout,"output.count=3\noutput.flag=true\noutput.names=[\"é\",\"🙂\",\"line\\nnext\",\"!\"]\noutput.text=Hello, to data.text.\n\0🙂!\noutput.value=-2\n".as_bytes());
    }
}

fn action(body: &str) -> String {
    source(&format!(
        "Action: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Completed\n"
    ))
}

#[test]
fn literal_only_programs_run_without_preexisting_runtime_fields() {
    let source = action(
        "- the system assigns \"\" to output.text\n- the system assigns false to output.flag\n- the system assigns -9223372036854775808 to output.value\n- the system assigns [] to output.names",
    );
    let package = package();
    let document = parse_ail_package_spec_text(&package, &source).unwrap();
    let result = run_ail_action(&document, "Run", BTreeMap::new()).unwrap();
    assert_eq!(result.final_state["output.text"], "");
    assert_eq!(result.final_state["output.flag"], "false");
    assert_eq!(result.final_state["output.value"], "-9223372036854775808");
    assert_eq!(result.final_state["output.names"], "[]");
    let output = native(&elaborate_ail_core(&package, &document), "Run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"output.flag=false\noutput.names=[]\noutput.text=\noutput.value=-9223372036854775808\n"
    );
}

#[test]
fn literal_assignments_preserve_copies_and_execute_in_loop_call_order() {
    let body = "- the system assigns [\"old\"] to data.names\n- the system changes the data names as the data copy\n- the system assigns \"original\" to data.text\n- the system changes the data text as the output text\n- the system assigns 0 to data.value\n- while data.value < 3:\n- the system calls Replace\n- the system computes data.value as data.value + 1\n- end while\n- the system gets item 0 from list data.copy into data.suffix\n- the system appends text data.suffix to list data.names into output.names\n- the system returns output.names\n- the system returns output.text\n- the system returns data.value";
    let source = format!(
        "{}\nAction: Replace.\nWhen Replace happens:\n- if data.value == 1:\n- the system assigns [\"new\"] to data.names\n- the system assigns \"changed\" to data.text\n- otherwise:\n- the system assigns true to data.flag\n- end if\n- the system records a trace event named Replaced\n",
        action(body)
    );
    let original = core(&source);
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(result.final_state["data.value"], "3");
        assert_eq!(result.final_state["output.text"], "original");
        assert_eq!(result.final_state["output.names"], "[\"new\",\"old\"]");
        let output = native(&candidate, "Run");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            b"data.value=3\noutput.names=[\"new\",\"old\"]\noutput.text=original\n"
        );
    }
}

#[test]
fn malformed_or_mistyped_source_literals_are_rejected_before_execution() {
    for (literal, field) in [
        ("null", "data.text"),
        ("{}", "data.text"),
        ("\"unterminated", "data.text"),
        ("\"\\ud800\"", "data.text"),
        ("\"\\x00\"", "data.text"),
        ("\"ok\" trailing", "data.text"),
        ("\"ok\"", "data.value"),
        ("1", "data.flag"),
        ("True", "data.flag"),
        ("false", "data.text"),
        ("+1", "data.value"),
        ("01", "data.value"),
        ("1.0", "data.value"),
        ("1e0", "data.value"),
        ("9223372036854775808", "data.value"),
        ("-9223372036854775809", "data.value"),
        ("[1,\"wrong\"]", "data.numbers"),
        ("[[1]]", "data.numbers"),
        ("[true]", "data.numbers"),
        ("[1,]", "data.numbers"),
        ("[1]", "data.names"),
        ("\"ok\"", "unknown.field"),
        ("[\"\\ud800\"]", "data.names"),
        ("\u{85}1", "data.value"),
    ] {
        let input = action(&format!("- the system assigns {literal} to {field}"));
        let package = package();
        match parse_ail_package_spec_text(&package, &input) {
            Err(error) => assert!(error.contains("AIL-LITERAL-001"), "{literal}: {error}"),
            Ok(document) => {
                let candidate = elaborate_ail_core(&package, &document);
                assert!(
                    check_ail_core(&candidate)
                        .iter()
                        .any(|e| e.contains("AIL-LITERAL-001")),
                    "{literal} {field}"
                );
                assert!(compile_ail_core_bytecode(&candidate).is_err());
            }
        }
    }
}

#[test]
fn saved_literal_core_checks_types_ownership_profiles_edges_and_protocol() {
    let original = core(&source(BODY));
    for kind in ["literal_step", "literal_destination"] {
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
        assert!(compile_ail_core_bytecode(&saved).is_err(), "{kind}");
    }
    for (attribute, value) in [
        ("value_type", "Decimal"),
        ("literal", "null"),
        ("unexpected", "value"),
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "LiteralAssignment")
            .unwrap()
            .attributes
            .insert(attribute.into(), value.into());
        assert!(
            compile_ail_core_bytecode(&candidate).is_err(),
            "{attribute}"
        );
    }
    for profile in ["Compiler", "AgentTool", "UI", "System"] {
        let mut candidate = original.clone();
        candidate.package.profile = profile.into();
        assert!(
            check_ail_core(&candidate)
                .iter()
                .any(|e| e.contains("AIL-LITERAL-001")),
            "{profile}"
        );
    }
    let mut local = original.clone();
    local
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Field" && n.name == "Data.value")
        .unwrap()
        .attributes
        .insert("function_owner".into(), "other".into());
    assert!(compile_ail_core_bytecode(&local).is_err());
    let mut compiler = original.clone();
    compiler
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Action")
        .unwrap()
        .attributes
        .insert("kind".into(), "CompilerPass".into());
    assert!(
        check_ail_core(&compiler)
            .iter()
            .any(|e| e.contains("AIL-LITERAL-001"))
    );
    let program = compile_ail_core_bytecode(&original).unwrap();
    for (operand, value) in [
        ("type", "Decimal"),
        ("value", "1.0"),
        ("key", "not a field"),
        ("hidden", "ignored"),
    ] {
        let mut candidate = program.clone();
        candidate
            .actions
            .get_mut("Run")
            .unwrap()
            .instructions
            .iter_mut()
            .find(|i| i.opcode == "ASSIGN_LITERAL")
            .unwrap()
            .operands
            .insert(operand.into(), value.into());
        assert!(!verify_ail_bytecode(&candidate).is_empty(), "{operand}");
        assert!(ail::ste::render_ail_ste(&candidate).is_err());
    }
}

#[test]
fn literal_secret_writes_require_protection_without_descriptive_write_edges() {
    let mut candidate = core(&action("- the system assigns \"secret\" to data.text"));
    candidate
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Field" && n.name == "Data.text")
        .unwrap()
        .attributes
        .insert("secret".into(), "true".into());
    candidate.graph.edges.retain(|e| e.kind != "writes");
    let saved = parse_ail_core_text(&render_ail_core(&candidate)).unwrap();
    assert!(check_ail_core(&saved).iter().any(|e| e.contains("AIL002")));
    assert!(compile_ail_core_bytecode(&saved).is_err());
    let owner = candidate
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Action")
        .unwrap()
        .clone();
    let destination = candidate
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Field" && n.name == "Data.text")
        .unwrap()
        .clone();
    candidate
        .graph
        .add_edge("protects_secret", &owner, &destination, BTreeMap::new());
    assert!(
        check_ail_core(&candidate).is_empty(),
        "{:?}",
        check_ail_core(&candidate)
    );
}

#[test]
fn imported_literals_namespace_destinations_but_preserve_text_data() {
    let mut child = package();
    child.metadata.name = "constant-child".into();
    child.spec_text = action(
        "- the system assigns \"data.value to output.text\" to output.text\n- the system returns output.text",
    );
    let mut parent = package();
    parent.spec_text="The application Parent manages constants.\nAction: Run.\nWhen Run happens:\n- the system calls Constants.Run\n- the system returns constants.output.text\n- the system records a trace event named Finished\n".into();
    parent.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Constants".into(),
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
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(
            result.output_state(),
            BTreeMap::from([(
                "constants.output.text".into(),
                "data.value to output.text".into()
            )])
        );
        let output = native(&candidate, "Run");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            b"constants.output.text=data.value to output.text\n"
        );
    }
}

#[test]
fn repeating_static_constants_does_not_consume_runtime_result_budgets() {
    let text = "x".repeat(1_048_576);
    let list = format!("[\"{}\"]", "y".repeat(1_048_572));
    let input = action(&format!(
        "- the system assigns 0 to data.value\n- while data.value < 70:\n- the system assigns \"{text}\" to data.text\n- the system assigns {list} to data.names\n- the system computes data.value as data.value + 1\n- end while\n- the system counts characters in data.text into output.value\n- the system counts items in list data.names into output.count\n- the system returns output.value\n- the system returns output.count"
    ));
    let candidate = core(&input);
    let program = compile_ail_core_bytecode(&candidate).unwrap();
    let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(result.final_state["output.value"], "1048576");
    assert_eq!(result.final_state["output.count"], "1");
    let output = native(&candidate, "Run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"output.count=1\noutput.value=1048576\n");
}

#[test]
fn compiler_pass_source_rejects_literal_statements_instead_of_recording_them() {
    let package = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for statement in [
        "the system assigns \"hello\" to data.text",
        "the system assigns true to data.flag",
    ] {
        let source = package.spec_text.replace(
            "- the system reads every edge",
            &format!("- {statement}\n- the system reads every edge"),
        );
        let parsed = parse_ail_package_spec_text(&package, &source);
        assert!(
            parsed.is_err(),
            "compiler pass accepted executable literal syntax"
        );
        assert!(parsed.unwrap_err().contains("AIL-LITERAL-001"));
    }
}

#[test]
fn compiler_pass_source_rejects_executable_text_and_list_statements() {
    let package = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for (statement, diagnostic) in [
        (
            "the system joins text data.a and data.b into data.c",
            "AIL-TEXT-001",
        ),
        (
            "the system creates an empty list into data.names",
            "AIL-LIST-001",
        ),
        (
            "the system appends integer 1 to list data.values into data.values",
            "AIL-LIST-001",
        ),
    ] {
        let source = package.spec_text.replace(
            "- the system reads every edge",
            &format!("- {statement}\n- the system reads every edge"),
        );
        let parsed = parse_ail_package_spec_text(&package, &source);
        assert!(parsed.is_err(), "compiler pass accepted {statement}");
        assert!(parsed.unwrap_err().contains(diagnostic));
    }
}

#[test]
fn saved_core_treats_literal_attribute_delimiters_as_opaque_data() {
    for (literal, expected) in [
        (r#""a,b=c""#, "a,b=c"),
        (r#""before [after]""#, "before [after]"),
        (
            r#""quoted \"x,y=z\" [literal=0]""#,
            "quoted \"x,y=z\" [literal=0]",
        ),
        (
            r#""backslash \\ and \\\" [x=y]""#,
            "backslash \\ and \\\" [x=y]",
        ),
        (
            r#""[value_type=Int,literal=1],value_type=Text""#,
            "[value_type=Int,literal=1],value_type=Text",
        ),
    ] {
        let original = core(&action(&format!(
            "- the system assigns {literal} to output.text\n- the system returns output.text"
        )));
        let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        for candidate in [saved.clone(), core(&render_ail_spec_from_core(&saved))] {
            let program = compile_ail_core_bytecode(&candidate).unwrap();
            assert_eq!(
                run_ail_bytecode_action(&program, "Run", BTreeMap::new())
                    .unwrap()
                    .final_state["output.text"],
                expected
            );
            let output = native(&candidate, "Run");
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                output.stdout,
                format!("output.text={expected}\n").as_bytes()
            );
        }
    }
}

#[test]
fn saved_core_rejects_duplicate_literal_attributes() {
    let original = core(&action("- the system assigns \"valid\" to output.text"));
    let saved = render_ail_core(&original)
        .replace("literal=\"valid\"", "literal=\"hidden\",literal=\"valid\"");
    assert!(parse_ail_core_text(&saved).is_err());
}

#[test]
fn saved_core_preserves_legacy_unquoted_labels_and_effects() {
    let original = core(&source(
        "Action: Say \" quote.\nWhen this action happens:\n- the system reads a \" marker\n- the system records a trace event named Done\n",
    ));
    let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
    let action = saved
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Action")
        .unwrap();
    assert_eq!(action.attributes["label"], "Say \" quote");
    assert_eq!(action.attributes["trigger"], "this action happens");
    let rendered = render_ail_spec_from_core(&saved);
    assert!(
        rendered.contains("- the system reads a \" marker"),
        "{rendered}"
    );
    assert!(check_ail_core(&core(&rendered)).is_empty());
}
