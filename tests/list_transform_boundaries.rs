use ail::ail::*;
use std::collections::BTreeMap;

fn core(start: &str, end: &str) -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let source = format!(
        "The application Lists manages lists.\n\nA Data has:\n- values: List<Int>\n- result: List<Int>\n- start: Int\n- end: Int\n\nFunction: window.\nThe function needs:\n- values: List<Int>\n- start: Int\n- end: Int\nThe function produces:\n- result: List<Int>\nThe function stores:\n- result: List<Int>\nThe function executes:\n- the function slices list local.values from {start} to {end} into local.result\n- the function returns local.result\n\nAction: Run.\nWhen Run happens:\n- the system calls window with values = data.values; start = data.start; end = data.end into data.result\n- the system returns data.result\n- the system records a trace event named Done\n"
    );
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, &source).unwrap(),
    )
}

fn operand(core: &AilCore, name: &str) -> String {
    core.graph
        .edges
        .iter()
        .find(|edge| {
            edge.kind == "value_expression_operand"
                && edge.attributes.get("name").is_some_and(|n| n == name)
        })
        .unwrap()
        .target
        .clone()
}

fn child(core: &AilCore, id: &str, role: &str) -> String {
    core.graph
        .edges
        .iter()
        .find(|edge| edge.source == id && edge.kind == role)
        .unwrap()
        .target
        .clone()
}

fn set_literal(core: &mut AilCore, id: &str, value: String) {
    core.graph
        .nodes
        .iter_mut()
        .find(|node| node.id == id)
        .unwrap()
        .attributes
        .insert("value".into(), value);
}

fn routes(core: &AilCore) -> [(&str, AilCore); 2] {
    [
        ("direct", core.clone()),
        (
            "saved",
            parse_ail_core_text(&render_ail_core(core)).unwrap(),
        ),
    ]
}

fn rejects(core: &AilCore) {
    let accepted: Vec<_> = routes(core)
        .into_iter()
        .filter_map(|(route, core)| match compile_ail_core_bytecode(&core) {
            Ok(_) => Some(route),
            Err(error) => {
                assert!(
                    error.contains("AIL-FUNCTION-001"),
                    "expected local expression diagnostic for {route}: {error}"
                );
                None
            }
        })
        .collect();
    assert!(
        accepted.is_empty(),
        "malformed Core accepted by {accepted:?}"
    );
}

fn accepts(core: &AilCore) {
    for (route, core) in routes(core) {
        compile_ail_core_bytecode(&core)
            .unwrap_or_else(|error| panic!("valid {route} Core rejected: {error}"));
    }
}

#[test]
fn local_slice_bounds_preserve_named_roles_and_roundtrip() {
    let original = core("local.start + 1", "local.end - 1");
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let rendered = elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, &render_ail_spec_from_core(&original)).unwrap(),
    );
    let input: BTreeMap<String, String> = [
        ("data.values", "[10,20,30,40]"),
        ("data.start", "0"),
        ("data.end", "4"),
    ]
    .into_iter()
    .map(|(key, value)| (key.into(), value.into()))
    .collect();
    for candidate in routes(&original)
        .into_iter()
        .map(|(_, core)| core)
        .chain([rendered])
    {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let output = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(output.final_state["data.result"], "[20,30]");
    }
}

#[test]
fn local_slice_rejects_a_shared_bound_root() {
    let mut bad = core("0", "1");
    let start = operand(&bad, "start");
    bad.graph
        .edges
        .iter_mut()
        .find(|edge| {
            edge.kind == "value_expression_operand"
                && edge.attributes.get("name").is_some_and(|n| n == "end")
        })
        .unwrap()
        .target = start;
    rejects(&bad);
}

#[test]
fn local_slice_rejects_shared_descendants_within_and_between_bounds() {
    for other_bound in [false, true] {
        let mut bad = core("0 + 1", "2 + 3");
        let start = operand(&bad, "start");
        let parent = if other_bound {
            operand(&bad, "end")
        } else {
            start.clone()
        };
        let shared = child(&bad, &start, "left_operand");
        bad.graph
            .edges
            .iter_mut()
            .find(|edge| edge.source == parent && edge.kind == "right_operand")
            .unwrap()
            .target = shared;
        rejects(&bad);
    }
}

#[test]
fn local_slice_rejects_foreign_parents_of_roots_and_descendants() {
    for descendant in [false, true] {
        let mut bad = core("0 + 1", "2");
        let root = operand(&bad, "start");
        let target = if descendant {
            child(&bad, &root, "left_operand")
        } else {
            root
        };
        let mut edge = bad
            .graph
            .edges
            .iter()
            .find(|edge| edge.target == target)
            .unwrap()
            .clone();
        edge.id = "foreign-list-bound-parent".into();
        edge.kind = "observes".into();
        edge.attributes.clear();
        edge.source = bad
            .graph
            .nodes
            .iter()
            .find(|node| node.kind == "Action")
            .unwrap()
            .id
            .clone();
        edge.ordinal = bad.graph.edges.len();
        bad.graph.edges.push(edge);
        rejects(&bad);
    }
}

#[test]
fn local_slice_applies_raw_literal_byte_limit_before_normalization() {
    let mut candidate = core("0", "1");
    let start = operand(&candidate, "start");
    // Include the two-byte i: token prefix in the raw postfix budget.
    set_literal(&mut candidate, &start, "0".repeat(65_534));
    accepts(&candidate);
    for length in [65_535, 65_537] {
        set_literal(&mut candidate, &start, "0".repeat(length));
        rejects(&candidate);
    }
}

#[test]
fn local_slice_applies_aggregate_raw_postfix_byte_limit() {
    let mut candidate = core("0 + 0", "1");
    let start = operand(&candidate, "start");
    let left = child(&candidate, &start, "left_operand");
    let right = child(&candidate, &start, "right_operand");
    // Two i: prefixes, two spaces and '+' add seven bytes to the literals.
    set_literal(&mut candidate, &left, "0".repeat(32_764));
    set_literal(&mut candidate, &right, "0".repeat(32_765));
    accepts(&candidate);
    set_literal(&mut candidate, &right, "0".repeat(32_766));
    rejects(&candidate);
}

#[test]
fn local_slice_rejects_extra_root_and_descendant_operand_metadata() {
    for descendant in [false, true] {
        let mut bad = core("0 + 1", "2");
        let start = operand(&bad, "start");
        let target = if descendant {
            child(&bad, &start, "left_operand")
        } else {
            start
        };
        bad.graph
            .edges
            .iter_mut()
            .find(|edge| edge.target == target)
            .unwrap()
            .attributes
            .insert("ignored_bound".into(), "true".into());
        rejects(&bad);
    }
}
