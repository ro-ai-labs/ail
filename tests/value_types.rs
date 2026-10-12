use ail::ail::*;
use std::collections::BTreeMap;

const DECLS: &str = "The application Values manages trees.\n\nType: Tree<T>.\nTree has variants:\n- Empty\n- Node(left: Tree<T>, right: Tree<T>, value: T)\n\nA Data has:\n- tree: Tree<Int>\n- empty: Tree<Int>\n- flag: Bool\n- value: Int\n- error: Text\n\n";

fn core(source: &str) -> AilCore {
    let p = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    elaborate_ail_core(&p, &parse_ail_package_spec_text(&p, source).unwrap())
}
fn source(body: &str) -> String {
    format!(
        "{DECLS}Action: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Done\n"
    )
}
fn program(body: &str) -> AilBytecodeProgram {
    compile_ail_core_bytecode(&core(&source(body))).unwrap()
}

#[test]
fn recursive_variants_execute_through_core_and_ste() {
    let c = core(&source(
        "- the system constructs Tree<Int> variant Empty with none into data.empty\n- the system constructs Tree<Int> variant Node with value = 7; left = data.empty; right = data.empty into data.tree\n- the system tests Tree<Int> variant Node of data.tree into data.flag\n- the system gets field value of Tree<Int> variant Node from data.tree into data.value\n- the system returns data.tree\n- the system returns data.flag\n- the system returns data.value",
    ));
    let saved = parse_ail_core_text(&render_ail_core(&c)).unwrap();
    let p = compile_ail_core_bytecode(&saved).unwrap();
    let p = ail::ste::parse_ail_ste(&ail::ste::render_ail_ste(&p).unwrap()).unwrap();
    let result = run_ail_bytecode_action(&p, "Run", BTreeMap::new()).unwrap();
    assert_eq!(
        result.final_state["data.tree"],
        "[\"Node\",[\"Empty\"],[\"Empty\"],\"7\"]"
    );
    assert_eq!(result.final_state["data.flag"], "true");
    assert_eq!(result.final_state["data.value"], "7");
}

#[test]
fn custom_function_arguments_locals_and_return_are_executable() {
    let functions = "Function: identity.\nThe function needs:\n- tree: Tree<Int>\nThe function produces:\n- result: Tree<Int>\nThe function stores:\n- empty: Tree<Int>\n- result: Tree<Int>\nThe function executes:\n- the function constructs Tree<Int> variant Empty with none into local.empty\n- the function constructs Tree<Int> variant Node with left = local.tree; right = local.empty; value = 4 into local.result\n- the function returns local.result\n\n";
    let c = core(&format!(
        "{DECLS}{functions}Action: Run.\nWhen Run happens:\n- the system constructs Tree<Int> variant Empty with none into data.tree\n- the system calls identity with tree = data.tree into data.tree\n- the system returns data.tree\n- the system records a trace event named Done\n"
    ));
    let p = compile_ail_core_bytecode(&c).unwrap();
    let result = run_ail_bytecode_action(&p, "Run", BTreeMap::new()).unwrap();
    assert_eq!(
        result.final_state["data.tree"],
        "[\"Node\",[\"Empty\"],[\"Empty\"],\"4\"]"
    );
}

#[test]
fn a_variant_test_validates_unselected_recursive_payloads() {
    let p = program("- the system tests Tree<Int> variant Empty of data.tree into data.flag");
    let state = BTreeMap::from([(
        "data.tree".into(),
        "[\"Node\",[\"Empty\"],[\"Bad\"],\"7\"]".into(),
    )]);
    assert!(
        run_ail_bytecode_action(&p, "Run", state)
            .unwrap_err()
            .starts_with("AIL-RUNTIME-VALUE:")
    );
}

#[test]
fn nominal_mismatch_and_incomplete_constructors_reject() {
    for statement in [
        "- the system constructs Tree<Int> variant Node with value = 7 into data.tree",
        "- the system constructs Tree<Int> variant Unknown with none into data.tree",
        "- the system tests Tree<Int> variant Empty of data.tree into data.value",
        "- the system gets field missing of Tree<Int> variant Node from data.tree into data.value",
    ] {
        assert!(
            compile_ail_core_bytecode(&core(&source(statement))).is_err(),
            "{statement}"
        );
    }
}

#[test]
fn duplicate_type_variant_and_payload_declarations_reject() {
    let p = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    for declarations in [
        "Type: A.\nA has variants:\n- Empty\nType: A.\nA has variants:\n- Other\n",
        "Type: A.\nA has variants:\n- Empty\n- Empty\n",
        "Type: A.\nA has variants:\n- Pair(value: Int, value: Text)\n",
    ] {
        assert!(
            parse_ail_package_spec_text(&p, declarations).is_err(),
            "{declarations}"
        );
    }
}
