use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn package(rung: &str) -> AilPackage {
    load_ail_package_dir(format!("bench/{rung}/program.ail")).unwrap()
}
fn core(rung: &str, source: &str) -> AilCore {
    let package = package(rung);
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}
fn state(args: &[&str]) -> BTreeMap<String, String> {
    args.iter()
        .map(|s| s.split_once('=').unwrap())
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
fn native(program: &AilBytecodeProgram, action: &str, args: &[&str]) -> std::process::Output {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "ail-control-{}-{}",
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
    // A concurrent test can briefly inherit an open writer across fork/exec.
    let mut attempts = 0;
    let output = loop {
        match Command::new(&path).args(args).output() {
            Err(error) if error.raw_os_error() == Some(26) && attempts < 20 => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            result => break result.unwrap(),
        }
    };
    fs::remove_file(path).unwrap();
    output
}
fn check_results(rung: &str, action: &str, args: &[&str], expected: &[&str]) {
    let core = core(rung, &package(rung).spec_text);
    assert!(
        check_ail_core(&core).is_empty(),
        "{:?}",
        check_ail_core(&core)
    );
    let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&core).unwrap()).unwrap();
    assert_eq!(
        run_ail_bytecode_action(&program, action, state(args))
            .unwrap()
            .output_state(),
        state(expected)
    );
    let output = native(&program, action, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = String::from_utf8(output.stdout).unwrap();
    assert_eq!(state(&output.lines().collect::<Vec<_>>()), state(expected));
}
#[test]
fn source_integer_comparisons_branch_on_runtime_signed_values() {
    for (a, b, want) in [
        ("7", "9", "9"),
        ("-9", "-7", "-7"),
        ("4", "4", "4"),
        (
            "-9223372036854775808",
            "9223372036854775807",
            "9223372036854775807",
        ),
    ] {
        check_results(
            "R003-max",
            "Max",
            &[&format!("n.a={a}"), &format!("n.b={b}")],
            &[&format!("out.max={want}")],
        );
    }
}
#[test]
fn source_nested_branches_and_loops_compute_independent_results() {
    check_results(
        "R004-abs-sign",
        "AbsSign",
        &["n.value=-42"],
        &["out.absolute=42", "out.sign=-1"],
    );
    check_results(
        "R007-sum-to-n",
        "SumToN",
        &["n.value=100"],
        &["out.total=5050"],
    );
    check_results("R008-gcd", "Gcd", &["n.a=-48", "n.b=18"], &["out.result=6"]);
    check_results(
        "R009-collatz",
        "Collatz",
        &["n.value=27"],
        &["out.steps=111"],
    );
    check_results(
        "R010-is-prime",
        "IsPrime",
        &["n.value=91"],
        &["out.prime=0"],
    );
    check_results(
        "R010-is-prime",
        "IsPrime",
        &["n.value=97"],
        &["out.prime=1"],
    );
}
#[test]
fn typed_control_flow_survives_saved_core_and_source_rendering() {
    let original = core("R009-collatz", &package("R009-collatz").spec_text);
    assert!(
        original
            .graph
            .nodes
            .iter()
            .any(|n| n.kind == "IntegerComparison" && n.type_name.as_deref() == Some("Bool"))
    );
    for candidate in [
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core("R009-collatz", &render_ail_spec_from_core(&original)),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let result = run_ail_bytecode_action(&program, "Collatz", state(&["n.value=6"])).unwrap();
        assert_eq!(result.output_state(), state(&["out.steps=8"]));
    }
}

fn custom(body: &str) -> String {
    format!(
        "The application Control Test manages loops.\n\nA Work has:\n\n- i: Int\n- j: Int\n- total: Int\n- text: Text\n\nAction: run.\n\nWhen run happens:\n\n{}\n- the system returns work.total\n- the system records a trace event named Calculated\n",
        body.lines()
            .map(|line| format!("- {line}\n"))
            .collect::<String>()
    )
}
fn custom_core(body: &str) -> AilCore {
    core("R003-max", &custom(body))
}
fn compare_custom(body: &str, expected: &str) {
    let core = custom_core(body);
    for candidate in [
        core.clone(),
        parse_ail_core_text(&render_ail_core(&core)).unwrap(),
        crate::core("R003-max", &render_ail_spec_from_core(&core)),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        assert_eq!(
            run_ail_bytecode_action(&program, "Run", BTreeMap::new())
                .unwrap()
                .output_state(),
            state(&[expected])
        );
        let output = native(&program, "Run", &[]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{expected}\n")
        );
    }
}
#[test]
fn continue_and_break_target_the_nearest_loop_through_nested_branches() {
    compare_custom(
        "the system computes work.i as 0\nthe system computes work.total as 0\nwhile work.i < 4:\nthe system computes work.i as work.i + 1\nif work.i == 2:\ncontinue\nend if\nthe system computes work.j as 0\nwhile work.j < 5:\nthe system computes work.j as work.j + 1\nif work.j == 3:\nbreak\nend if\nthe system computes work.total as work.total + work.i\nend while\nend while",
        "work.total=16",
    );
}
#[test]
fn unchosen_branches_and_zero_trip_loops_do_not_evaluate_body_expressions() {
    compare_custom("if 2 < 1:\nthe system computes work.total as 1 / 0\notherwise:\nthe system computes work.total as 7\nend if\nwhile 3 == 4:\nthe system computes work.total as work.text + 1\nend while".replace("work.text", "work.j").as_str(),"work.total=7");
}
#[test]
fn source_rejects_malformed_blocks_and_declarations_inside_blocks() {
    for body in [
        "otherwise:",
        "end if",
        "end while",
        "break",
        "continue",
        "if 1 < 2:",
        "while 1 < 2:\nend if",
        "if 1 < 2:\notherwise:\notherwise:\nend if",
        "if 1 < 2:\nend while",
        "if 1 < 2:\nthe system returns work.total\nend if",
        "if 1 < 2:\nthe system records a trace event named Inside\nend if",
        "if 1 < 2",
        "while 1 < 2",
        "if 1 < 2 < 3:\nend if",
    ] {
        let package = package("R003-max");
        let checked = parse_ail_package_spec_text(&package, &custom(body))
            .map(|doc| elaborate_ail_core(&package, &doc));
        if let Ok(core) = checked {
            assert!(!check_ail_core(&core).is_empty(), "accepted: {body}");
            assert!(
                compile_ail_core_bytecode(&core).is_err(),
                "compiled: {body}"
            );
        }
    }
}
#[test]
fn comparisons_require_declared_integer_fields_even_in_unchosen_conditions() {
    for expression in ["work.text == 7", "work.missing != 0"] {
        let core = custom_core(&format!(
            "if {expression}:\nthe system computes work.total as 1\nend if"
        ));
        assert!(!check_ail_core(&core).is_empty());
        assert!(compile_ail_core_bytecode(&core).is_err());
    }
}
#[test]
fn edited_comparison_nodes_change_execution_and_bad_graphs_are_rejected() {
    let mut core = custom_core(
        "if 2 < 3:\nthe system computes work.total as 1\notherwise:\nthe system computes work.total as 2\nend if",
    );
    core.graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "IntegerComparison")
        .unwrap()
        .attributes
        .insert("comparison".into(), ">".into());
    let saved = parse_ail_core_text(&render_ail_core(&core)).unwrap();
    assert_eq!(
        run_ail_bytecode_action(
            &compile_ail_core_bytecode(&saved).unwrap(),
            "Run",
            BTreeMap::new()
        )
        .unwrap()
        .output_state(),
        state(&["work.total=2"])
    );
    let mut missing = core.clone();
    missing.graph.edges.retain(|e| e.kind != "comparison_right");
    let mut wrong_type = core.clone();
    wrong_type
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "IntegerComparison")
        .unwrap()
        .type_name = Some("Int".into());
    let mut missing_order = core.clone();
    for e in &mut missing_order.graph.edges {
        e.attributes.remove("step_index");
    }
    let mut bad_target = core;
    let field = bad_target
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Field")
        .unwrap()
        .id
        .clone();
    bad_target
        .graph
        .edges
        .iter_mut()
        .find(|e| e.kind == "control_step")
        .unwrap()
        .target = field;
    for invalid in [missing, wrong_type, missing_order, bad_target] {
        assert!(!check_ail_core(&invalid).is_empty());
        assert!(compile_ail_core_bytecode(&invalid).is_err());
        let _ = render_ail_spec_from_core(&invalid);
    }
}
#[test]
fn imported_conditions_qualify_both_operands_and_calls_prefix_branch_labels() {
    let child = package("R007-sum-to-n");
    let mut parent = package("R003-max");
    parent.spec_text="The application Root manages loops.\n\nAction: invoke.\n\nWhen invoke happens:\n\n- the system calls Math.SumToN\n- the system records a trace event named Invoked\n".into();
    parent.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Math".into(),
            resolved_package: None,
            registry_identity: None,
        },
        package: Box::new(child),
    });
    let core = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    let program = compile_ail_core_bytecode(&core).unwrap();
    let vm = run_ail_bytecode_action(&program, "Invoke", state(&["math.n.value=10"])).unwrap();
    assert_eq!(vm.final_state["math.out.total"], "55");
    let output = native(&program, "Invoke", &["math.n.value=10"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        state(&output.lines().collect::<Vec<_>>()),
        vm.output_state()
    );
}
#[test]
fn bytecode_verifier_checks_integer_branch_operands_and_target() {
    let original = compile_ail_core_bytecode(&custom_core(
        "if 1 < 2:\nthe system computes work.total as 1\nend if",
    ))
    .unwrap();
    for (operand, value, code) in [
        ("left", "i:1 +", "AILBC010"),
        ("right", "f:bad", "AILBC010"),
        ("comparison", "===", "AILBC010"),
        ("label", "missing_label", "AILBC003"),
    ] {
        let mut program = original.clone();
        program
            .actions
            .get_mut("Run")
            .unwrap()
            .instructions
            .iter_mut()
            .find(|i| i.opcode == "BRANCH_INT")
            .unwrap()
            .operands
            .insert(operand.into(), value.into());
        assert!(
            verify_ail_bytecode(&program)
                .iter()
                .any(|s| s.contains(code))
        );
        assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
        assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
    }
}
#[test]
fn nonterminating_source_loops_fail_at_the_execution_limit_on_both_backends() {
    let program = compile_ail_core_bytecode(&custom_core(
        "the system computes work.total as 0\nwhile 1 == 1:\ncontinue\nend while",
    ))
    .unwrap();
    assert!(
        run_ail_bytecode_action(&program, "Run", BTreeMap::new())
            .unwrap_err()
            .contains("step limit")
    );
    let output = native(&program, "Run", &[]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("step limit"));
}

#[test]
fn malformed_source_and_core_repeat_counts_are_rejected() {
    let p = package("R003-max");
    for count in ["nonsense", "0", "-1", "18446744073709551616"] {
        assert!(
            parse_ail_package_spec_text(
                &p,
                &custom(&format!(
                    "if 1 == 1:\nthe system repeats helper {count} times\nend if"
                ))
            )
            .is_err(),
            "accepted count {count}"
        );
    }
    let source = custom("the system repeats helper 2 times")
        + "\nAction: helper.\nWhen helper happens:\n- the system computes work.total as 1\n- the system records a trace event named Helped\n";
    let original = core("R003-max", &source);
    assert!(check_ail_core(&original).is_empty());
    for count in [None, Some("garbage"), Some("0"), Some("-1")] {
        let mut edited = original.clone();
        let edge = edited
            .graph
            .edges
            .iter_mut()
            .find(|e| e.kind == "repeats")
            .unwrap();
        edge.attributes.remove("count");
        if let Some(count) = count {
            edge.attributes.insert("count".into(), count.into());
        }
        assert!(
            !check_ail_core(&edited).is_empty(),
            "accepted count {count:?}"
        );
        assert!(compile_ail_core_bytecode(&edited).is_err());
    }
}

#[test]
fn explicitly_sequential_repetitions_execute_inside_branches() {
    let source = custom(
        "the system computes work.total as 0\nif 1 == 1:\nthe system repeats Add one 3 times sequentially\nend if",
    ) + "\nAction: Add one.\nWhen Add one happens:\n- the system increments the work total by 1\n- the system returns work.total\n- the system records a trace event named Added\n";
    let program = compile_ail_core_bytecode(&core("R003-max", &source)).unwrap();
    assert_eq!(
        run_ail_bytecode_action(&program, "Run", BTreeMap::new())
            .unwrap()
            .output_state(),
        state(&["work.total=3"])
    );
    let result = native(&program, "Run", &[]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "work.total=3\n");
}
#[test]
fn misplaced_condition_edges_are_rejected() {
    use ail::core_model::Edge;
    let original = custom_core("if 1 < 2:\nthe system computes work.total as 1\nend if");
    let comparison = original
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "IntegerComparison")
        .unwrap();
    let end = original
        .graph
        .nodes
        .iter()
        .find(|n| {
            n.attributes
                .get("control_kind")
                .is_some_and(|k| k == "end_if")
        })
        .unwrap();
    let expression = original
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "IntegerExpression")
        .unwrap();
    for (kind, source, target) in [
        ("left_operand", comparison, expression),
        ("comparison_left", end, expression),
        ("condition", expression, comparison),
    ] {
        let mut edited = original.clone();
        edited
            .graph
            .edges
            .push(Edge::new(kind, source, target, BTreeMap::new()));
        assert!(
            !check_ail_core(&edited).is_empty(),
            "accepted misplaced {kind}"
        );
        assert!(compile_ail_core_bytecode(&edited).is_err());
    }
}

#[test]
fn compiler_pass_actions_cannot_silently_discard_control_steps() {
    let mut core = custom_core("if 1 < 2:\nthe system computes work.total as 1\nend if");
    core.graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Action")
        .unwrap()
        .attributes
        .insert("kind".into(), "CompilerPass".into());
    assert!(!check_ail_core(&core).is_empty());
    assert!(compile_ail_core_bytecode(&core).is_err());
}

#[test]
fn excessive_control_nesting_is_rejected_before_execution() {
    for depth in [64, 65] {
        let mut body = "if 1 == 1:\n".repeat(depth);
        body.push_str("the system computes work.total as 7\n");
        body.push_str(&"end if\n".repeat(depth));
        let core = custom_core(&body);
        if depth == 64 {
            let program = compile_ail_core_bytecode(&core).unwrap();
            assert_eq!(
                run_ail_bytecode_action(&program, "Run", BTreeMap::new())
                    .unwrap()
                    .output_state(),
                state(&["work.total=7"])
            );
        } else {
            assert!(
                compile_ail_core_bytecode(&core)
                    .unwrap_err()
                    .contains("nesting")
            );
        }
    }
}
#[test]
fn looped_calls_receive_separate_execution_budgets() {
    let mut source = custom(
        "the system computes work.i as 0\nwhile work.i < 2000:\nthe system calls helper\nthe system computes work.i as work.i + 1\nend while\nthe system computes work.total as work.i",
    );
    source.push_str("\nAction: helper.\nWhen helper happens:\n");
    for _ in 0..100 {
        source.push_str("- the system computes work.j as 1\n");
    }
    source.push_str("- the system records a trace event named Helped\n");
    let program = compile_ail_core_bytecode(&core("R003-max", &source)).unwrap();
    let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(vm.output_state(), state(&["work.total=2000"]));
    let result = native(&program, "Run", &[]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
            .lines()
            .last()
            .unwrap_or("")
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "work.total=2000\n"
    );
}
