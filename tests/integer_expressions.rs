use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use ail::ail::*;

fn package() -> AilPackage {
    load_ail_package_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/R001-add/program.ail"),
    )
    .unwrap()
}

fn core_with(source: &str) -> AilCore {
    let package = package();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}

#[test]
fn integer_expression_has_typed_core_nodes_and_evaluates_runtime_operands() {
    let core = core_with(&package().spec_text);
    assert!(
        check_ail_core(&core).is_empty(),
        "{:?}",
        check_ail_core(&core)
    );
    assert!(
        core.graph
            .nodes
            .iter()
            .any(|n| n.kind == "IntegerExpression" && n.type_name.as_deref() == Some("Int"))
    );
    assert!(core.graph.edges.iter().any(|e| e.kind == "left_operand"));
    let bytecode = compile_ail_core_bytecode(&core).unwrap();
    let result = run_ail_bytecode_action(
        &bytecode,
        "AddNumbers",
        BTreeMap::from([
            ("sum.a".into(), "007".into()),
            ("sum.b".into(), "-0".into()),
        ]),
    )
    .unwrap();
    assert_eq!(result.final_state["sum.total"], "7");
    assert_eq!(result.final_state["sum.a"], "007");
    assert_eq!(
        render_ail_core(&parse_ail_core_text(&render_ail_core(&core)).unwrap()),
        render_ail_core(&core)
    );
}

#[test]
fn arithmetic_rejects_unknown_fields_text_operands_and_text_destinations() {
    for source in [
        package()
            .spec_text
            .replace("sum.a + sum.b", "sum.a + sum.missing"),
        package()
            .spec_text
            .replace("sum.a + sum.b", "sum.a + sum.label"),
        package()
            .spec_text
            .replace("computes sum.total", "computes sum.label"),
        package().spec_text.replace("sum.a + sum.b", "sum.a +"),
    ] {
        let core = core_with(&source);
        let diagnostics = check_ail_core(&core).join("\n");
        assert!(diagnostics.contains("AIL-EXPR-"), "{diagnostics}");
        assert!(compile_ail_core_bytecode(&core).is_err());
    }
}

#[test]
fn expression_source_order_and_parentheses_survive_core_and_ste_round_trips() {
    let source = package().spec_text.replace("computes sum.total as sum.a + sum.b", "computes sum.total as (sum.a + sum.b) * 3\n- the system computes sum.total as sum.total - 1");
    let core = core_with(&source);
    assert!(
        check_ail_core(&core).is_empty(),
        "{:?}",
        check_ail_core(&core)
    );
    for candidate in [
        core.clone(),
        parse_ail_core_text(&render_ail_core(&core)).unwrap(),
        core_with(&render_ail_spec_from_core(&core)),
    ] {
        let ste = compile_ail_core_ste(&candidate).unwrap();
        let bytecode = ail::ste::parse_ail_ste(&ste).unwrap();
        let result = run_ail_bytecode_action(
            &bytecode,
            "AddNumbers",
            BTreeMap::from([("sum.a".into(), "4".into()), ("sum.b".into(), "5".into())]),
        )
        .unwrap();
        assert_eq!(result.final_state["sum.total"], "26");
    }
}

#[test]
fn arithmetic_cli_selects_results_and_preserves_text_outputs() {
    let output = Command::new(env!("CARGO_BIN_EXE_ail"))
        .args([
            "ail-run",
            "bench/R001b-expression-typed/program.ail",
            "--action",
            "TypedAddNumbers",
            "sum.a=007",
            "sum.b=1",
            "sum.label=007",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("sum.total=8"), "{stdout}");
    assert!(stdout.contains("sum.label=007"), "{stdout}");
    assert!(!stdout.contains("\nsum.a="), "{stdout}");
}

#[test]
fn arithmetic_native_executes_typed_values_and_observes_prior_assignments() {
    let source = package().spec_text.replace("computes sum.total as sum.a + sum.b", "computes sum.total as (sum.a + sum.b) * 3\n- the system computes sum.total as sum.total - 1");
    let core = core_with(&source);
    let bytes = compile_ail_core_native_elf(&core, "AddNumbers", "linux-x86_64-elf").unwrap();
    let path = std::env::temp_dir().join(format!("ail-expression-native-{}", std::process::id()));
    fs::write(&path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let output = Command::new(&path)
        .args(["sum.a=4", "sum.b=5"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "sum.total=26\n");
}

fn state(a: &str, b: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("sum.a".into(), a.into()), ("sum.b".into(), b.into())])
}

#[test]
fn checked_core_ast_edits_control_execution_and_malformed_graphs_are_rejected() {
    let mut core = core_with(&package().spec_text);
    core.graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "IntegerExpression" && n.attributes.contains_key("operator"))
        .unwrap()
        .attributes
        .insert("operator".into(), "*".into());
    for core in [
        core.clone(),
        parse_ail_core_text(&render_ail_core(&core)).unwrap(),
    ] {
        let result = run_ail_bytecode_action(
            &compile_ail_core_bytecode(&core).unwrap(),
            "AddNumbers",
            state("4", "5"),
        )
        .unwrap();
        assert_eq!(result.final_state["sum.total"], "20");
    }
    let original = core_with(&package().spec_text);
    let mut missing = original.clone();
    missing.graph.edges.retain(|e| e.kind != "right_operand");
    let mut cycle = original.clone();
    let edge = cycle
        .graph
        .edges
        .iter_mut()
        .find(|e| e.kind == "left_operand")
        .unwrap();
    edge.target = edge.source.clone();
    let mut untyped = original.clone();
    untyped
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "IntegerExpression")
        .unwrap()
        .type_name = Some("Text".into());
    for invalid in [missing, cycle, untyped] {
        assert!(
            check_ail_core(&invalid)
                .iter()
                .any(|s| s.contains("AIL-EXPR-"))
        );
        assert!(compile_ail_core_bytecode(&invalid).is_err());
    }
}

#[test]
fn mixed_legacy_and_expression_writes_keep_every_assignment_in_order() {
    let source = package().spec_text.replace("computes sum.total as sum.a + sum.b", "changes the sum total to 9\n- the system computes sum.total as sum.total + sum.a\n- the system changes the sum total to 2\n- the system computes sum.total as sum.total * sum.b");
    let core = core_with(&source);
    for candidate in [
        core.clone(),
        parse_ail_core_text(&render_ail_core(&core)).unwrap(),
        core_with(&render_ail_spec_from_core(&core)),
    ] {
        let result = run_ail_bytecode_action(
            &compile_ail_core_bytecode(&candidate).unwrap(),
            "AddNumbers",
            state("4", "5"),
        )
        .unwrap();
        assert_eq!(result.final_state["sum.total"], "10");
    }
}

#[test]
fn literal_increment_checks_destination_type_and_variable_increment_evaluates() {
    let source = package().spec_text.replace(
        "computes sum.total as sum.a + sum.b",
        "increments the sum label by 1",
    );
    assert!(compile_ail_core_bytecode(&core_with(&source)).is_err());
    let source = package()
        .spec_text
        .replace(
            "computes sum.total as sum.a + sum.b",
            "increments the sum a by sum.b",
        )
        .replace("returns sum.total", "returns sum.a");
    let core = core_with(&source);
    let result = run_ail_bytecode_action(
        &compile_ail_core_bytecode(&core).unwrap(),
        "AddNumbers",
        state("7", "9"),
    )
    .unwrap();
    assert_eq!(
        result.output_state(),
        BTreeMap::from([("sum.a".into(), "16".into())])
    );
}

#[test]
fn document_runner_executes_expressions_and_rejects_text_arithmetic() {
    let document = parse_ail_package_document(&package()).unwrap();
    let result = run_ail_action(&document, "AddNumbers", state("7", "9")).unwrap();
    assert_eq!(
        result.output_state(),
        BTreeMap::from([("sum.total".into(), "16".into())])
    );
    let source = package()
        .spec_text
        .replace("sum.a + sum.b", "sum.a + sum.label");
    let document = parse_ail_package_spec_text(&package(), &source).unwrap();
    assert!(run_ail_action(&document, "AddNumbers", state("7", "9")).is_err());
}

#[test]
fn expressions_and_outputs_are_qualified_in_imported_actions() {
    let mut parent = package();
    let imported = parent.clone();
    parent.spec_text = "The application Root manages arithmetic.\n".into();
    parent.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Math".into(),
            resolved_package: None,
            registry_identity: None,
        },
        package: Box::new(imported),
    });
    let core = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    let program = compile_ail_core_bytecode(&core).unwrap();
    let result = run_ail_bytecode_action(
        &program,
        "Math.AddNumbers",
        BTreeMap::from([
            ("math.sum.a".into(), "7".into()),
            ("math.sum.b".into(), "9".into()),
        ]),
    )
    .unwrap();
    assert_eq!(
        result.output_state(),
        BTreeMap::from([("math.sum.total".into(), "16".into())])
    );
}

#[test]
fn vm_checks_integer_errors_and_postfix_verifier_rejects_bad_programs() {
    for (expression, a, b, error) in [
        ("sum.a + sum.b", "9223372036854775807", "1", "OVERFLOW"),
        ("sum.a / sum.b", "1", "0", "DIVZERO"),
        ("sum.a % sum.b", "-9223372036854775808", "-1", "OVERFLOW"),
        ("sum.a * sum.b", "three", "2", "INT"),
        ("sum.a - sum.b", "9223372036854775808", "2", "INT"),
    ] {
        let core = core_with(&package().spec_text.replace("sum.a + sum.b", expression));
        let program = compile_ail_core_bytecode(&core).unwrap();
        assert!(
            run_ail_bytecode_action(&program, "AddNumbers", state(a, b))
                .unwrap_err()
                .contains(&format!("AIL-RUNTIME-{error}"))
        );
    }
    let mut program = compile_ail_core_bytecode(&core_with(&package().spec_text)).unwrap();
    program
        .actions
        .get_mut("AddNumbers")
        .unwrap()
        .instructions
        .iter_mut()
        .find(|i| i.opcode == "EVAL_INT")
        .unwrap()
        .operands
        .insert("expression".into(), "i:1 +".into());
    assert!(
        verify_ail_bytecode(&program)
            .iter()
            .any(|s| s.contains("AILBC008"))
    );
    assert!(ail::ste::render_ail_ste(&program).is_err());
}

fn native_run(program: &AilBytecodeProgram, action: &str, args: &[&str]) -> std::process::Output {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "ail-arithmetic-integration-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(
        &path,
        compile_ail_bytecode_native_elf(program, action, "linux-x86_64-elf").unwrap(),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let output = Command::new(&path).args(args).output().unwrap();
    fs::remove_file(path).unwrap();
    output
}

#[test]
fn calls_and_computations_execute_in_source_order_through_all_stages() {
    let source = package().spec_text.replace("computes sum.total as sum.a + sum.b", "computes sum.a as 2\n- the system calls Double\n- the system computes sum.total as sum.b + 1") + "\nAction: double.\n\nWhen double happens:\n\n- the system computes sum.b as sum.a * 2\n- the system returns sum.b\n- the system records a trace event named Doubled\n";
    let core = core_with(&source);
    for candidate in [
        core.clone(),
        parse_ail_core_text(&render_ail_core(&core)).unwrap(),
        core_with(&render_ail_spec_from_core(&core)),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let result = run_ail_bytecode_action(&program, "AddNumbers", BTreeMap::new()).unwrap();
        assert_eq!(
            result.output_state(),
            BTreeMap::from([("sum.total".into(), "5".into())])
        );
        let output = native_run(&program, "AddNumbers", &[]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"sum.total=5\n");
    }
}

#[test]
fn nested_action_outputs_are_validated_but_do_not_select_root_results() {
    let source = package().spec_text.replace(
        "computes sum.total as sum.a + sum.b",
        "calls Child\n- the system computes sum.total as 9",
    ) + "\nAction: child.\n\nWhen child happens:\n\n- the system returns sum.label\n- the system records a trace event named ChildReturned\n";
    let program = compile_ail_core_bytecode(&core_with(&source)).unwrap();
    assert!(
        run_ail_bytecode_action(&program, "AddNumbers", BTreeMap::new())
            .unwrap_err()
            .contains("AIL-RUNTIME-OUTPUT")
    );
    let missing = native_run(&program, "AddNumbers", &[]);
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("AIL-RUNTIME-OUTPUT"));
    let output = native_run(&program, "AddNumbers", &["sum.label=007"]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"sum.total=9\n");
}

#[test]
fn repeated_calls_to_one_action_preserve_source_order_and_multiplicity() {
    let source = package().spec_text.replace("computes sum.total as sum.a + sum.b", "computes sum.a as 1\n- the system calls Bump\n- the system repeats Bump 2 times\n- the system calls Bump\n- the system computes sum.total as sum.a") + "\nAction: bump.\n\nWhen bump happens:\n\n- the system increments the sum a by 1\n- the system returns sum.a\n- the system records a trace event named Bumped\n";
    let core = core_with(&source);
    for candidate in [
        core.clone(),
        parse_ail_core_text(&render_ail_core(&core)).unwrap(),
        core_with(&render_ail_spec_from_core(&core)),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        assert_eq!(
            run_ail_bytecode_action(&program, "AddNumbers", BTreeMap::new())
                .unwrap()
                .output_state()["sum.total"],
            "5"
        );
        let native = native_run(&program, "AddNumbers", &[]);
        assert!(
            native.status.success(),
            "{}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(native.stdout, b"sum.total=5\n");
    }
}

#[test]
fn edited_core_cannot_exceed_expression_depth_or_silently_discard_computation() {
    use ail::core_model::attr;
    let source = package()
        .spec_text
        .replace("sum.a + sum.b", &format!("0{}", "+1".repeat(127)));
    let mut core = core_with(&source);
    assert!(check_ail_core(&core).is_empty());
    let program = compile_ail_core_bytecode(&core).unwrap();
    assert_eq!(
        run_ail_bytecode_action(&program, "AddNumbers", BTreeMap::new())
            .unwrap()
            .final_state["sum.total"],
        "127"
    );
    let old = core
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "IntegerExpression" && n.name == "AddNumbers.write0")
        .unwrap()
        .clone();
    let leaf = core
        .graph
        .nodes
        .iter()
        .find(|n| {
            n.kind == "IntegerExpression"
                && n.attributes.get("expression_kind").map(String::as_str) == Some("literal")
        })
        .unwrap()
        .clone();
    let provenance_id = &core
        .graph
        .edges
        .iter()
        .find(|e| e.kind == "has_provenance" && e.source == old.id)
        .unwrap()
        .target;
    let provenance = core
        .graph
        .nodes
        .iter()
        .find(|n| &n.id == provenance_id)
        .unwrap()
        .clone();
    let root = core.graph.add_node(
        "IntegerExpression",
        "AddNumbers.wrapped",
        Some("Int".into()),
        attr(&[("expression_kind", "binary"), ("operator", "+")]),
    );
    core.graph
        .add_edge("left_operand", &root, &old, BTreeMap::new());
    core.graph
        .add_edge("right_operand", &root, &leaf, BTreeMap::new());
    core.graph
        .add_edge("has_provenance", &root, &provenance, BTreeMap::new());
    core.graph
        .edges
        .iter_mut()
        .find(|e| e.kind == "writes")
        .unwrap()
        .attributes
        .insert(
            "expression".into(),
            "IntegerExpression:AddNumbers.wrapped".into(),
        );
    assert!(
        check_ail_core(&core)
            .iter()
            .any(|s| s.contains("AIL-EXPR-001"))
    );
    assert!(compile_ail_core_bytecode(&core).is_err());
}

#[test]
fn native_literal_increment_checks_overflow_without_explicit_outputs() {
    let source = package()
        .spec_text
        .replace(
            "computes sum.total as sum.a + sum.b",
            "increments the sum a by 1",
        )
        .replace("- the system returns sum.total\n", "");
    let program = compile_ail_core_bytecode(&core_with(&source)).unwrap();
    assert!(
        run_ail_bytecode_action(&program, "AddNumbers", state("9223372036854775807", "0"))
            .unwrap_err()
            .contains("AIL-RUNTIME-OVERFLOW")
    );
    let result = native_run(&program, "AddNumbers", &["sum.a=9223372036854775807"]);
    assert!(
        !result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    assert!(String::from_utf8_lossy(&result.stderr).contains("AIL-RUNTIME-OVERFLOW"));
    assert!(result.stdout.is_empty());
}

#[test]
fn invalid_ordered_core_edges_are_rejected_without_compiler_panics() {
    let core = core_with(&package().spec_text);
    let mut hidden_write = core.clone();
    hidden_write
        .graph
        .edges
        .iter_mut()
        .find(|e| e.kind == "writes")
        .unwrap()
        .attributes
        .insert("inferred_expression".into(), "true".into());
    let mut wrong_call = core.clone();
    let action = wrong_call
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Action")
        .unwrap()
        .clone();
    let field = wrong_call
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Field")
        .unwrap()
        .clone();
    wrong_call.graph.add_edge(
        "calls",
        &action,
        &field,
        ail::core_model::attr(&[("step_index", "1")]),
    );
    for invalid in [hidden_write, wrong_call] {
        assert!(
            check_ail_core(&invalid)
                .iter()
                .any(|s| s.contains("AIL-EXPR-002"))
        );
        assert!(compile_ail_core_bytecode(&invalid).is_err());
        // The inspection API also handles unchecked graphs without indexing panics.
        let _ = render_ail_spec_from_core(&invalid);
    }
}
