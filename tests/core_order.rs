use ail::ail::*;
use std::collections::BTreeMap;

const DECLARATIONS: &str = "The application Order manages values.\nA Data has:\n- value: Int\n- other: Int\n- text: Text\n- suffix: Text\n- names: List<Text>\n";
const SET_COMPUTE: &str = "Action: Set.\nWhen Set happens:\n- the system assigns 7 to data.value\n- the system records a trace event named SetDone\nAction: Compute.\nWhen Compute happens:\n- the system computes data.value as data.value + 2\n- the system records a trace event named ComputeDone\nAction: Demo.\nWhen Demo happens:\n- the system calls Set\n- the system calls Compute\n- the system returns data.value\n- the system records a trace event named Done\n";

fn core(body: &str) -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let document = parse_ail_package_spec_text(&package, &format!("{DECLARATIONS}{body}")).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert!(
        check_ail_core(&core).is_empty(),
        "{:?}",
        check_ail_core(&core)
    );
    core
}

fn assert_permutations(original: &AilCore, action: &str, expected: &[(&str, &str)]) {
    let input = BTreeMap::from([("data.value".into(), "0".into())]);
    let document = ail_document_from_core(original);
    let ste = compile_ail_core_ste(original).unwrap();
    let program = compile_ail_core_bytecode(original).unwrap();
    let result = run_ail_bytecode_action(&program, action, input.clone()).unwrap();
    for (key, value) in expected {
        assert_eq!(result.final_state[*key], *value);
    }
    let mut reversed = original.clone();
    reversed.graph.edges.reverse();
    let mut rotated = original.clone();
    let rotation = rotated.graph.edges.len() / 2;
    rotated.graph.edges.rotate_left(rotation);
    for candidate in [
        reversed,
        rotated,
        parse_ail_core_text(&render_ail_core(original)).unwrap(),
    ] {
        assert!(
            check_ail_core(&candidate).is_empty(),
            "{:?}",
            check_ail_core(&candidate)
        );
        assert_eq!(render_ail_core(&candidate), render_ail_core(original));
        assert_eq!(ail_core_hash(&candidate), ail_core_hash(original));
        let candidate_program = compile_ail_core_bytecode(&candidate).unwrap();
        // Compare actual execution before the structural checks: legacy calls
        // reproduced a checked 9 -> 7 scalar change despite the equal hash.
        assert_eq!(
            run_ail_bytecode_action(&candidate_program, action, input.clone()).unwrap(),
            result
        );
        assert_eq!(ail_document_from_core(&candidate), document);
        assert_eq!(compile_ail_core_ste(&candidate).unwrap(), ste);
        assert_eq!(candidate_program, program);
        assert_eq!(ail::ste::parse_ail_ste(&ste).unwrap(), program);
    }
}

#[test]
fn legacy_set_compute_calls_follow_ordinals_after_storage_permutation() {
    let mut original = core(SET_COMPUTE);
    for edge in &mut original.graph.edges {
        if edge.kind == "calls" {
            edge.attributes.remove("step_index");
        }
    }
    assert_permutations(&original, "Demo", &[("data.value", "9")]);
}

#[test]
fn legacy_writes_follow_ordinals_after_storage_permutation() {
    let mut original = core(
        "Action: Demo.\nWhen Demo happens:\n- the system computes data.value as 7\n- the system computes data.value as data.value + 2\n- the system returns data.value\n- the system records a trace event named Done\n",
    );
    for edge in &mut original.graph.edges {
        if edge.kind == "writes" {
            edge.attributes.remove("step_index");
        }
    }
    assert_permutations(&original, "Demo", &[("data.value", "9")]);
}

#[test]
fn explicit_steps_keep_mixed_decoder_operation_indices_consistent() {
    let body = "Action: Demo.\nWhen Demo happens:\n- the system assigns 1 to data.value\n- the system assigns \"first\" to data.text\n- the system assigns \"!\" to data.suffix\n- the system joins text data.text and data.suffix into data.text\n- the system computes data.value as data.value + 2\n- the system calls Compute\n- the system assigns \"second\" to data.text\n- the system joins text data.text and data.suffix into data.text\n- the system computes data.other as data.value * 2\n- the system returns data.value\n- the system returns data.other\n- the system returns data.text\n- the system records a trace event named Done\nAction: Compute.\nWhen Compute happens:\n- the system computes data.value as data.value + 2\n- the system records a trace event named ComputeDone\n";
    assert_permutations(
        &core(body),
        "Demo",
        &[
            ("data.value", "5"),
            ("data.other", "10"),
            ("data.text", "second!"),
        ],
    );
}

fn duplicate_calls(legacy: bool) {
    let body = SET_COMPUTE.replace(
        "- the system calls Compute\n",
        "- the system calls Compute\n- the system calls Compute\n",
    );
    let mut original = core(&body);
    assert_eq!(
        original
            .graph
            .edges
            .iter()
            .filter(|e| e.kind == "calls")
            .count(),
        3
    );
    if legacy {
        for edge in &mut original.graph.edges {
            if edge.kind == "calls" {
                edge.attributes.remove("step_index");
            }
        }
    }
    assert_permutations(&original, "Demo", &[("data.value", "11")]);
}

#[test]
fn duplicate_explicit_calls_preserve_occurrences() {
    duplicate_calls(false);
}

#[test]
fn duplicate_legacy_calls_preserve_occurrences() {
    duplicate_calls(true);
}

#[test]
fn returns_and_traces_keep_ordinal_order() {
    let original = core(
        "Action: Demo.\nWhen Demo happens:\n- the system assigns 7 to data.value\n- the system assigns 9 to data.other\n- the system returns data.value\n- the system returns data.other\n- the system records a trace event named First\n- the system records a trace event named Second\n",
    );
    let result = run_ail_bytecode_action(
        &compile_ail_core_bytecode(&original).unwrap(),
        "Demo",
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(result.returned_fields, ["data.value", "data.other"]);
    assert_permutations(
        &original,
        "Demo",
        &[("data.value", "7"), ("data.other", "9")],
    );
}

#[test]
fn executable_integer_function_bodies_and_calls_are_storage_invariant() {
    let body = "Function: pair.\nThe function needs:\n- a: Int\n- b: Int\nThe function produces:\n- result: Int\nThe function stores:\n- result: Int\nThe function executes:\n- the function computes local.result as local.a * 10\n- the function computes local.result as local.result + local.b\n- the function returns local.result\nAction: Demo.\nWhen Demo happens:\n- the system calls pair with b = 5; a = 4 into data.value\n- the system calls pair with a = 2; b = 3 into data.other\n- the system returns data.value\n- the system returns data.other\n- the system records a trace event named Done\n";
    assert_permutations(
        &core(body),
        "Demo",
        &[("data.value", "45"), ("data.other", "23")],
    );
}

#[test]
fn executable_value_function_bodies_and_calls_are_storage_invariant() {
    let body = "Function: decorate.\nThe function needs:\n- text: Text\nThe function produces:\n- result: Text\nThe function stores:\n- suffix: Text\n- result: Text\nThe function executes:\n- the function assigns \"!\" to local.suffix\n- the function joins text local.text and local.suffix into local.result\n- the function returns local.result\nAction: Demo.\nWhen Demo happens:\n- the system assigns \"hello\" to data.text\n- the system calls decorate with text = data.text into data.text\n- the system calls decorate with text = data.text into data.text\n- the system returns data.text\n- the system records a trace event named Done\n";
    assert_permutations(&core(body), "Demo", &[("data.text", "hello!!")]);
}

#[test]
fn duplicate_semantic_ordinals_fail_closed_even_across_owners_and_relations() {
    let original = core(SET_COMPUTE);
    let indices: Vec<_> = original
        .graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind != "has_provenance")
        .map(|(i, _)| i)
        .collect();
    for second in [indices[1], *indices.last().unwrap()] {
        let mut ambiguous = original.clone();
        ambiguous.graph.edges[second].ordinal = ambiguous.graph.edges[indices[0]].ordinal;
        assert!(
            check_ail_core_diagnostics(&ambiguous)
                .iter()
                .any(|d| d.code == "AIL-CORE-ORDER-001")
        );
        assert!(compile_ail_core_ste(&ambiguous).is_err());
        assert!(compile_ail_core_bytecode(&ambiguous).is_err());
    }
}

#[test]
fn ordinal_gaps_remain_legal() {
    let mut original = core(SET_COMPUTE);
    for edge in &mut original.graph.edges {
        edge.ordinal = edge.ordinal * 2 + 3;
    }
    assert!(
        original
            .graph
            .edges
            .iter()
            .filter(|e| e.kind == "has_provenance")
            .count()
            > 1
    );
    assert_permutations(&original, "Demo", &[("data.value", "9")]);
}

#[test]
fn duplicate_provenance_ordinals_fail_closed() {
    let original = core(SET_COMPUTE);
    let provenance: Vec<_> = original
        .graph
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == "has_provenance")
        .map(|(i, _)| i)
        .collect();
    // Provenance payloads can participate in legacy decoding, so graph-wide
    // ordinal uniqueness includes provenance, even on different owners.
    let mut ambiguous = original.clone();
    ambiguous.graph.edges[provenance[1]].ordinal = ambiguous.graph.edges[provenance[0]].ordinal;
    assert!(
        check_ail_core_diagnostics(&ambiguous)
            .iter()
            .any(|d| d.code == "AIL-CORE-ORDER-001")
    );
    assert!(compile_ail_core_bytecode(&ambiguous).is_err());
    let semantic = original
        .graph
        .edges
        .iter()
        .position(|e| e.kind != "has_provenance")
        .unwrap();
    ambiguous = original.clone();
    ambiguous.graph.edges[provenance[0]].ordinal = ambiguous.graph.edges[semantic].ordinal;
    assert!(
        check_ail_core_diagnostics(&ambiguous)
            .iter()
            .any(|d| d.code == "AIL-CORE-ORDER-001")
    );
}

#[test]
fn checked_patch_remove_then_add_allocates_after_retained_ordinals() {
    let mut original = core(SET_COMPUTE);
    for edge in &mut original.graph.edges {
        edge.ordinal = edge.ordinal * 3 + 100;
    }
    let max_ordinal = original
        .graph
        .edges
        .iter()
        .map(|e| e.ordinal)
        .max()
        .unwrap();
    let patch = format!(
        r#"{{"schema":"ail-core.patch.v0","base_hash":"{}","ops":[{{"op":"remove_edge","kind":"contains","source":"Application:Order","target":"Thing:Data"}},{{"op":"add_edge","kind":"contains","source":"Application:Order","target":"Thing:Data"}}]}}"#,
        ail_core_hash(&original)
    );
    let patched = apply_ail_core_patch_text(&original, &patch).unwrap();
    let appended = patched.graph.edges.last().unwrap();
    assert_eq!(appended.kind, "contains");
    assert!(
        appended.ordinal > max_ordinal,
        "appended ordinal {} must exceed retained maximum {max_ordinal}",
        appended.ordinal
    );
    for retained in &patched.graph.edges[..patched.graph.edges.len() - 1] {
        assert!(original.graph.edges.contains(retained));
    }
    assert!(
        check_ail_core(&patched).is_empty(),
        "{:?}",
        check_ail_core(&patched)
    );
    assert_permutations(&patched, "Demo", &[("data.value", "9")]);
}

#[test]
fn exhausted_ordinals_reject_checked_patch_without_mutating_baseline() {
    let mut original = core(SET_COMPUTE);
    original.graph.edges.last_mut().unwrap().ordinal = usize::MAX;
    assert!(check_ail_core(&original).is_empty());
    let snapshot = original.clone();
    let patch = format!(
        r#"{{"schema":"ail-core.patch.v0","base_hash":"{}","ops":[{{"op":"remove_edge","kind":"contains","source":"Application:Order","target":"Thing:Data"}},{{"op":"add_edge","kind":"contains","source":"Application:Order","target":"Thing:Data"}}]}}"#,
        ail_core_hash(&original)
    );
    let error = apply_ail_core_patch_text(&original, &patch).unwrap_err();
    assert!(error.contains("AIL-CORE-ORDER-001"), "{error}");
    assert_eq!(original, snapshot);
}

#[test]
fn first_attached_provenance_is_selected_by_ordinal() {
    let mut original = core(SET_COMPUTE);
    let action = original.graph.find_node("Action", "Demo").unwrap().clone();
    let alternate = original.graph.add_node(
        "Provenance",
        "alternate selected provenance",
        None,
        BTreeMap::new(),
    );
    original
        .graph
        .add_edge("has_provenance", &action, &alternate, BTreeMap::new());
    assert_eq!(
        ail_document_from_core(&original).actions["Demo"].provenance,
        "action:Demo"
    );
    assert_permutations(&original, "Demo", &[("data.value", "9")]);
}

#[test]
fn first_attached_provenance_payload_preserves_legacy_execution() {
    let mut original = core(
        "Action: Demo.\nWhen Demo happens:\n- the system increments data.value by 2\n- the system returns data.value\n- the system records a trace event named Done\n",
    );
    for edge in &mut original.graph.edges {
        if edge.kind == "writes" {
            edge.attributes.remove("provenance");
        }
    }
    let field = original
        .graph
        .find_node("Field", "Data.value")
        .unwrap()
        .clone();
    let first_provenance = original
        .graph
        .edges
        .iter()
        .find(|e| e.kind == "has_provenance" && e.source == field.id)
        .unwrap()
        .target
        .clone();
    original
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == first_provenance)
        .unwrap()
        .name = "action:Demo.write:increments data.value by 2".into();
    let alternate = original.graph.add_node(
        "Provenance",
        "action:Demo.write:increments data.value by 5",
        None,
        BTreeMap::new(),
    );
    original
        .graph
        .add_edge("has_provenance", &field, &alternate, BTreeMap::new());
    assert_eq!(
        ail_document_from_core(&original).actions["Demo"].writes,
        ["increments data.value by 2"]
    );
    assert_permutations(&original, "Demo", &[("data.value", "2")]);
}
