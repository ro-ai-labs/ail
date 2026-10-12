use ail::ail::*;
use std::collections::BTreeMap;
use std::process::Command;

const HEAD: &str = "The application Audit manages numbers.\nA Data has:\n- value: Int\n";
const DEMO: &str = "Action: Demo.\nWhen Demo happens:\n- the system computes data.value as 7 + 2\n- the system returns data.value\n- the system records a trace event named Done\n";
fn core(spec: &str) -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let document = parse_ail_package_spec_text(&package, spec).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert!(
        check_ail_core(&core).is_empty(),
        "{:?}",
        check_ail_core(&core)
    );
    core
}
fn base() -> AilCore {
    core(&format!("{HEAD}{DEMO}"))
}
fn reject(b: &AilCore, c: &AilCore) {
    let diff = compare_ail_core_semantics(b, c);
    assert!(!diff.is_equivalent(), "{}", render_ail_semantic_diff(&diff));
    assert!(
        !diff.preserves_baseline(),
        "{}",
        render_ail_semantic_diff(&diff)
    );
}
#[test]
fn executable_operators_literals_and_function_bodies_are_exact() {
    let original = format!("{HEAD}{DEMO}");
    for replacement in ["7 - 2", "8 + 2"] {
        reject(
            &core(&original),
            &core(&original.replace("7 + 2", replacement)),
        );
    }
    let function = format!(
        "{HEAD}Function: adjust.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function executes:\n- the function returns local.n + 2\n{DEMO}"
    );
    reject(
        &core(&function),
        &core(&function.replace("local.n + 2", "local.n - 2")),
    );
}
#[test]
fn variant_constructor_is_exact() {
    let original = "The application Audit manages choices.\nType: Choice.\nChoice has variants:\n- First\n- Second\nA Data has:\n- value: Choice\nAction: Demo.\nWhen Demo happens:\n- the system constructs Choice variant First with none into data.value\n- the system returns data.value\n- the system records a trace event named Done\n";
    reject(
        &core(original),
        &core(&original.replace("variant First", "variant Second")),
    );
}
#[test]
fn call_order_multiplicity_and_appended_steps_are_preserved() {
    let original = format!(
        "{HEAD}Action: Set.\nWhen Set happens:\n- the system assigns 7 to data.value\n- the system records a trace event named SetDone\nAction: Compute.\nWhen Compute happens:\n- the system computes data.value as data.value + 2\n- the system records a trace event named ComputeDone\nAction: Demo.\nWhen Demo happens:\n- the system calls Set\n- the system calls Compute\n- the system records a trace event named Done\n"
    );
    for candidate in [
        original.replace(
            "calls Set\n- the system calls Compute",
            "calls Compute\n- the system calls Set",
        ),
        original.replace(
            "calls Compute\n",
            "calls Compute\n- the system calls Compute\n",
        ),
    ] {
        reject(&core(&original), &core(&candidate));
    }
    reject(
        &base(),
        &core(&format!(
            "{HEAD}{}",
            DEMO.replace(
                "- the system returns",
                "- the system computes data.value as 0\n- the system returns"
            )
        )),
    );
}
#[test]
fn metadata_type_case_and_all_attributes_are_exact() {
    let original = base();
    let mut changed = original.clone();
    changed.package.version = "9.9.9".into();
    reject(&original, &changed);
    let mut changed = original.clone();
    changed.package.base_llm_endpoint.push_str("/different");
    reject(&original, &changed);
    let states = format!(
        "{}{}",
        HEAD.replace("Int", "State<Ready, Closed>"),
        DEMO.replace("- the system computes data.value as 7 + 2\n", "")
    );
    reject(&core(&states), &core(&states.replace("Ready", "ready")));
    let mut changed = original.clone();
    changed
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Action")
        .unwrap()
        .attributes
        .insert("review_note".into(), "Keep exact".into());
    reject(&original, &changed);
    let mut changed = original.clone();
    changed
        .graph
        .edges
        .iter_mut()
        .find(|e| e.kind == "returns_field")
        .unwrap()
        .attributes
        .insert("provenance".into(), "meaningful payload".into());
    reject(&original, &changed);
}
#[test]
fn invalid_graphs_never_equal_themselves() {
    let original = base();
    let mut cases = vec![];
    let mut c = original.clone();
    c.graph.nodes.push(c.graph.nodes[0].clone());
    cases.push(c);
    let mut c = original.clone();
    let mut n = c.graph.nodes[0].clone();
    n.id.push_str("duplicate");
    c.graph.nodes.push(n);
    cases.push(c);
    let mut c = original.clone();
    c.graph.edges[0].source = "missing".into();
    cases.push(c);
    let mut c = original.clone();
    c.graph.edges[1].ordinal = c.graph.edges[0].ordinal;
    cases.push(c);
    let mut c = original.clone();
    c.graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Field")
        .unwrap()
        .type_name = Some("int".into());
    cases.push(c);
    for c in cases {
        reject(&c, &c);
    }
}
#[test]
fn opaque_ids_storage_permutation_and_evidence_are_neutral() {
    let original = base();
    let mut changed = original.clone();
    let mut ids = BTreeMap::new();
    for (i, n) in changed.graph.nodes.iter_mut().enumerate() {
        ids.insert(n.id.clone(), format!("opaque-{i}"));
        n.id = format!("opaque-{i}");
    }
    for (i, e) in changed.graph.edges.iter_mut().enumerate() {
        e.id = format!("edge-{i}");
        e.source = ids[&e.source].clone();
        e.target = ids[&e.target].clone();
        if let Some(r) = e.attributes.get_mut("expression")
            && let Some(id) = ids.get(r)
        {
            *r = id.clone();
        }
    }
    changed.graph.nodes.reverse();
    changed.graph.edges.reverse();
    assert!(compare_ail_core_semantics(&original, &changed).is_equivalent());
    for n in changed
        .graph
        .nodes
        .iter_mut()
        .filter(|n| n.kind == "Provenance")
    {
        n.name.push_str(" reviewed");
    }
    assert!(compare_ail_core_semantics(&original, &changed).is_equivalent());
    assert!(
        compare_ail_core_semantics(
            &original,
            &parse_ail_core_text(&render_ail_core(&original)).unwrap()
        )
        .is_equivalent()
    );
}
#[test]
fn independent_action_function_and_nominal_type_are_allowed_only_directionally() {
    for addition in [
        "Action: Extra.\nWhen Extra happens:\n- the system computes data.value as 3 + 4\n- the system returns data.value\n- the system records a trace event named ExtraDone\n",
        "Function: extra.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function executes:\n- the function returns local.n + 1\n",
        "Type: Extra.\nExtra has variants:\n- New\n",
    ] {
        let candidate = core(&format!("{HEAD}{DEMO}{addition}"));
        let diff = compare_ail_core_semantics(&base(), &candidate);
        assert!(
            diff.preserves_baseline(),
            "{}",
            render_ail_semantic_diff(&diff)
        );
        assert!(!diff.is_equivalent());
        assert_eq!(
            diff.lowering_validation,
            AilSemanticLoweringValidation::ExistingRecordsPreserved
        );
    }
}
#[test]
fn failure_and_orphan_additions_require_explicit_target() {
    let original = base();
    let mut candidate = original.clone();
    candidate
        .graph
        .add_node("Failure", "PermissionDenied", None, BTreeMap::new());
    reject(&original, &candidate);
    let mut candidate = original.clone();
    candidate
        .graph
        .add_node("Trace", "Unowned", None, BTreeMap::new());
    reject(&original, &candidate);
}
#[test]
fn existing_type_variant_sets_are_frozen() {
    let original = format!("{HEAD}{DEMO}Type: Extra.\nExtra has variants:\n- First\n");
    reject(
        &core(&original),
        &core(&original.replace("- First\n", "- First\n- Second\n")),
    );
}
#[test]
fn executable_provenance_fallback_is_compared_before_evidence_is_omitted() {
    let mut original = core(&format!(
        "{HEAD}Action: Demo.\nWhen Demo happens:\n- the system changes the data value to 7\n- the system records a trace event named Done\n"
    ));
    let action = original
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Action")
        .unwrap()
        .clone();
    let edge = original
        .graph
        .edges
        .iter_mut()
        .find(|e| e.kind == "writes" && e.source == action.id)
        .unwrap();
    let target = edge.target.clone();
    edge.attributes.remove("provenance");
    let mut candidate = original.clone();
    let n = candidate
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == target)
        .unwrap();
    n.attributes.insert(
        "provenance".into(),
        format!("action:{}.write:changes the data value to 8", action.name),
    );
    reject(&original, &candidate);
    // Evidence-only names are generally ignorable; this attachment is executable.
    let mut original = original.clone();
    let target_node = original
        .graph
        .nodes
        .iter()
        .find(|n| n.id == target)
        .unwrap()
        .clone();
    let existing = original
        .graph
        .edges
        .iter()
        .filter(|e| e.source == target && e.kind == "has_provenance")
        .map(|e| e.target.clone())
        .collect::<Vec<_>>();
    original
        .graph
        .edges
        .retain(|e| !(e.source == target && e.kind == "has_provenance"));
    original.graph.nodes.retain(|n| !existing.contains(&n.id));
    let evidence = original.graph.add_node(
        "Provenance",
        format!("action:{}.write:changes the data value to 7", action.name),
        None,
        BTreeMap::new(),
    );
    original
        .graph
        .add_edge("has_provenance", &target_node, &evidence, BTreeMap::new());
    let mut candidate = original.clone();
    candidate
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == evidence.id)
        .unwrap()
        .name = format!("action:{}.write:changes the data value to 8", action.name);
    reject(&original, &candidate);
}
#[test]
fn global_ordinal_shifts_do_not_change_existing_owner_sequences() {
    let original = base();
    let mut c = original.clone();
    for e in &mut c.graph.edges {
        e.ordinal += 100;
    }
    assert!(compare_ail_core_semantics(&original, &c).is_equivalent());
}
#[test]
fn cli_saved_core_strict_default_preservation_and_validation() {
    let dir = std::env::temp_dir().join(format!("ail-edit-semantics-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let baseline = dir.join("baseline.core");
    let candidate = dir.join("candidate.core");
    std::fs::write(&baseline, render_ail_core(&base())).unwrap();
    let added = core(&format!(
        "{HEAD}{DEMO}Type: Extra.\nExtra has variants:\n- New\n"
    ));
    std::fs::write(&candidate, render_ail_core(&added)).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_ail"))
            .args(args)
            .output()
            .unwrap()
    };
    let args = [
        "ail-equivalence",
        "--core-file",
        candidate.to_str().unwrap(),
        "--baseline",
        baseline.to_str().unwrap(),
    ];
    let strict = run(&args);
    assert_eq!(strict.status.code(), Some(1), "{:?}", strict);
    let mut preserve = args.to_vec();
    preserve.push("--preserve-baseline");
    assert!(run(&preserve).status.success());
    assert!(
        run(&[
            "ail-equivalence",
            "--core-file",
            baseline.to_str().unwrap(),
            "--baseline",
            baseline.to_str().unwrap()
        ])
        .status
        .success()
    );
    assert!(
        !run(&[
            "ail-equivalence",
            "bench/R001-add/program.ail",
            "--core-file",
            candidate.to_str().unwrap(),
            "--baseline",
            baseline.to_str().unwrap()
        ])
        .status
        .success()
    );
    let mut invalid = base();
    invalid
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Field")
        .unwrap()
        .type_name = Some("int".into());
    std::fs::write(&baseline, render_ail_core(&invalid)).unwrap();
    let output = run(&preserve);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("baseline"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn schema_expression_references_are_id_neutral_but_literals_are_exact() {
    let original = base();
    let mut candidate = original.clone();
    let edge = candidate
        .graph
        .edges
        .iter_mut()
        .find(|e| e.kind == "writes" && e.attributes.contains_key("expression"))
        .unwrap();
    let spelling = edge.attributes["expression"].clone();
    let node = candidate
        .graph
        .nodes
        .iter()
        .find(|n| format!("{}:{}", n.kind, n.name) == spelling)
        .unwrap();
    edge.attributes.insert("expression".into(), node.id.clone());
    assert!(compare_ail_core_semantics(&original, &candidate).is_equivalent());
    let mut ambiguous = candidate.clone();
    let other = ambiguous
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "IntegerExpression" && n.id != node.id)
        .unwrap();
    other.id = spelling.clone();
    let edge = ambiguous
        .graph
        .edges
        .iter_mut()
        .find(|e| e.kind == "writes")
        .unwrap();
    edge.attributes.insert("expression".into(), spelling);
    reject(&ambiguous, &ambiguous);
    // A string that happens to be an ID in an arbitrary attr remains literal.
    let mut candidate = original.clone();
    candidate
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Action")
        .unwrap()
        .attributes
        .insert("literal".into(), node.id.clone());
    let mut changed = candidate.clone();
    changed
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Action")
        .unwrap()
        .attributes
        .insert("literal".into(), "different opaque id".into());
    reject(&candidate, &changed);
}
#[test]
fn runtime_and_generic_nominal_head_collisions_require_explicit_target() {
    let function = "Function: Demo.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function executes:\n- the function returns local.n + 1\n";
    reject(&base(), &core(&format!("{HEAD}{DEMO}{function}")));
    let generic = "Type: Extra<T>.\nExtra has variants:\n- New\n";
    let baseline = core(&format!("{HEAD}{DEMO}{generic}"));
    reject(
        &baseline,
        &core(&format!(
            "{HEAD}{DEMO}{generic}Type: Extra<U>.\nExtra has variants:\n- Other\n"
        )),
    );
    let mut reserved = base();
    reserved
        .graph
        .add_node("Type", "Int", None, BTreeMap::new());
    reject(&base(), &reserved);
}
#[test]
fn new_declarations_cannot_adopt_existing_internal_helpers() {
    let original = core(&format!(
        "{HEAD}{DEMO}Type: Existing.\nExisting has variants:\n- Old\n"
    ));
    let mut candidate = original.clone();
    let variant = candidate
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Variant")
        .unwrap()
        .clone();
    let root = candidate
        .graph
        .add_node("Type", "New", None, BTreeMap::new());
    candidate
        .graph
        .add_edge("contains", &root, &variant, BTreeMap::new());
    reject(&original, &candidate);
    let mut candidate = original.clone();
    let mut added = core(&format!(
        "{HEAD}{DEMO}Action: Extra.\nWhen Extra happens:\n- the system computes data.value as 3 + 4\n- the system records a trace event named Done\n"
    ));
    let old_expression = candidate
        .graph
        .edges
        .iter()
        .find(|e| e.kind == "writes")
        .unwrap()
        .attributes["expression"]
        .clone();
    added
        .graph
        .edges
        .iter_mut()
        .find(|e| {
            e.kind == "writes"
                && added
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == e.source && n.name == "Extra")
        })
        .unwrap()
        .attributes
        .insert("expression".into(), old_expression);
    reject(&candidate, &added);
    candidate.graph.add_node(
        "IntegerExpression",
        "Unowned",
        Some("Int".into()),
        BTreeMap::from([
            ("expression_kind".into(), "literal".into()),
            ("value".into(), "1".into()),
        ]),
    );
    reject(&original, &candidate);
}
fn example(path: &str) -> AilCore {
    let package = load_ail_package_dir(path).unwrap();
    let doc = parse_ail_package_document(&package).unwrap();
    let c = elaborate_ail_core(&package, &doc);
    assert!(check_ail_core(&c).is_empty(), "{:?}", check_ail_core(&c));
    c
}
#[test]
fn tool_and_failure_provenance_fallbacks_are_exact() {
    // Prefix spelling is singular, as consumed by the actual projection.
    let mut tool = example("examples/refund_tool.ail");
    let owner = tool
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Tool")
        .unwrap()
        .clone();
    let edge = tool
        .graph
        .edges
        .iter_mut()
        .find(|e| e.source == owner.id && e.kind == "calls")
        .unwrap();
    edge.attributes.remove("provenance");
    let target_id = edge.target.clone();
    tool.graph
        .nodes
        .iter_mut()
        .find(|n| n.id == target_id)
        .unwrap()
        .attributes
        .insert(
            "provenance".into(),
            format!("tool:{}.call:first", owner.name),
        );
    let mut changed = tool.clone();
    changed
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == target_id)
        .unwrap()
        .attributes
        .insert(
            "provenance".into(),
            format!("tool:{}.call:second", owner.name),
        );
    reject(&tool, &changed);
    // Failure handling has no edge payload fallback and derives payload from Effect evidence.
    let mut failure = example("examples/support_ticket.ail");
    let owner = failure
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Failure")
        .unwrap()
        .clone();
    let target_id = failure
        .graph
        .edges
        .iter()
        .find(|e| e.source == owner.id && e.kind == "handles_failure")
        .unwrap()
        .target
        .clone();
    failure
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == target_id)
        .unwrap()
        .attributes
        .remove("provenance");
    failure
        .graph
        .edges
        .retain(|e| !(e.source == target_id && e.kind == "has_provenance"));
    let target = failure
        .graph
        .nodes
        .iter()
        .find(|n| n.id == target_id)
        .unwrap()
        .clone();
    let evidence = failure.graph.add_node(
        "Provenance",
        format!("failure:{}.handling:first", owner.name),
        None,
        BTreeMap::new(),
    );
    failure
        .graph
        .add_edge("has_provenance", &target, &evidence, BTreeMap::new());
    let mut changed = failure.clone();
    changed
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == evidence.id)
        .unwrap()
        .name = format!("failure:{}.handling:second", owner.name);
    reject(&failure, &changed);
}
#[test]
fn first_attached_evidence_order_matters_but_unselected_evidence_does_not() {
    let mut original = core(&format!(
        "{HEAD}Action: Demo.\nWhen Demo happens:\n- the system increments data.value by 2\n- the system records a trace event named Done\n"
    ));
    let edge = original
        .graph
        .edges
        .iter_mut()
        .find(|e| e.kind == "writes")
        .unwrap();
    edge.attributes.remove("provenance");
    let target_id = edge.target.clone();
    original
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == target_id)
        .unwrap()
        .attributes
        .remove("provenance");
    original
        .graph
        .edges
        .retain(|e| !(e.source == target_id && e.kind == "has_provenance"));
    let target = original
        .graph
        .nodes
        .iter()
        .find(|n| n.id == target_id)
        .unwrap()
        .clone();
    let first = original
        .graph
        .add_node("Provenance", "unrelated evidence", None, BTreeMap::new());
    let first_edge = original
        .graph
        .add_edge("has_provenance", &target, &first, BTreeMap::new());
    let second = original.graph.add_node(
        "Provenance",
        "action:Demo.write:increments data.value by 5",
        None,
        BTreeMap::new(),
    );
    let second_edge = original
        .graph
        .add_edge("has_provenance", &target, &second, BTreeMap::new());
    let mut inert = original.clone();
    inert
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == second.id)
        .unwrap()
        .name
        .push_str(" ignored");
    assert!(compare_ail_core_semantics(&original, &inert).is_equivalent());
    assert_eq!(
        ail_document_from_core(&original).actions["Demo"].writes,
        vec![target.name]
    );
    let mut selected = original.clone();
    selected
        .graph
        .edges
        .iter_mut()
        .find(|e| e.id == first_edge.id)
        .unwrap()
        .ordinal = second_edge.ordinal;
    selected
        .graph
        .edges
        .iter_mut()
        .find(|e| e.id == second_edge.id)
        .unwrap()
        .ordinal = first_edge.ordinal;
    reject(&original, &selected);
}
#[test]
fn a_nonlowerable_profile_retains_structural_addition_admission() {
    let mut original = example("examples/compiler_pass.ail");
    original.package.profile = "AIL-Meta".into();
    assert!(check_ail_core(&original).is_empty());
    assert!(compile_ail_core_bytecode(&original).is_err());
    let mut candidate = original.clone();
    let root = candidate.graph.add_node(
        "Type",
        "Added",
        None,
        BTreeMap::from([("provenance".into(), "test:new type".into())]),
    );
    let variant = candidate.graph.add_node(
        "Variant",
        "Added.New",
        None,
        BTreeMap::from([("provenance".into(), "test:new variant".into())]),
    );
    candidate
        .graph
        .add_edge("contains", &root, &variant, BTreeMap::new());
    let provenance =
        candidate
            .graph
            .add_node("Provenance", "test:new declaration", None, BTreeMap::new());
    candidate
        .graph
        .add_edge("has_provenance", &root, &provenance, BTreeMap::new());
    candidate
        .graph
        .add_edge("has_provenance", &variant, &provenance, BTreeMap::new());
    let diff = compare_ail_core_semantics(&original, &candidate);
    assert!(
        diff.preserves_baseline(),
        "{}",
        render_ail_semantic_diff(&diff)
    );
    assert!(!diff.is_equivalent());
    assert_eq!(
        diff.lowering_validation,
        AilSemanticLoweringValidation::BaselineUnavailable
    );
    assert!(render_ail_semantic_diff(&diff).contains("structural/effective-payload-only"));
}

fn attached_payload(
    mut original: AilCore,
    owner: ail::core_model::Node,
    relation: &str,
    prefix: &str,
) -> (AilCore, AilCore) {
    let edge = original
        .graph
        .edges
        .iter_mut()
        .find(|e| e.source == owner.id && e.kind == relation)
        .unwrap();
    edge.attributes.remove("provenance");
    let target_id = edge.target.clone();
    original
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == target_id)
        .unwrap()
        .attributes
        .remove("provenance");
    original
        .graph
        .edges
        .retain(|e| !(e.source == target_id && e.kind == "has_provenance"));
    let target = original
        .graph
        .nodes
        .iter()
        .find(|n| n.id == target_id)
        .unwrap()
        .clone();
    let evidence = original.graph.add_node(
        "Provenance",
        format!("{prefix}first"),
        None,
        BTreeMap::new(),
    );
    original
        .graph
        .add_edge("has_provenance", &target, &evidence, BTreeMap::new());
    let mut candidate = original.clone();
    candidate
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == evidence.id)
        .unwrap()
        .name = format!("{prefix}second");
    (original, candidate)
}
#[test]
fn tool_and_compiler_active_evidence_changes_are_detected_in_effective_payloads() {
    let original = example("examples/refund_tool.ail");
    let owner = original
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Tool")
        .unwrap()
        .clone();
    let prefix = format!("tool:{}.call:", owner.name);
    let (original, candidate) = attached_payload(original, owner, "calls", &prefix);
    assert!(
        check_ail_core(&original).is_empty(),
        "{:?}",
        check_ail_core(&original)
    );
    assert!(
        check_ail_core(&candidate).is_empty(),
        "{:?}",
        check_ail_core(&candidate)
    );
    let diff = compare_ail_core_semantics(&original, &candidate);
    assert!(diff.changed.iter().any(|s| s.contains("effective calls")));
    reject(&original, &candidate);
    let original = example("examples/compiler_pass.ail");
    let owner = original
        .graph
        .nodes
        .iter()
        .find(|n| {
            n.kind == "Action"
                && n.attributes
                    .get("kind")
                    .is_some_and(|k| k == "CompilerPass")
        })
        .unwrap()
        .clone();
    let prefix = format!("compiler_pass:{}.read:", owner.name);
    let (original, candidate) = attached_payload(original, owner, "reads", &prefix);
    assert!(
        check_ail_core(&original).is_empty(),
        "{:?}",
        check_ail_core(&original)
    );
    assert!(
        check_ail_core(&candidate).is_empty(),
        "{:?}",
        check_ail_core(&candidate)
    );
    let diff = compare_ail_core_semantics(&original, &candidate);
    assert!(
        diff.changed.iter().any(|s| s.contains("effective reads")),
        "{}",
        render_ail_semantic_diff(&diff)
    );
    reject(&original, &candidate);
}
#[test]
fn compiler_payloads_apply_the_existing_declared_port_filter() {
    let original = example("examples/compiler_pass.ail");
    let owner = original
        .graph
        .nodes
        .iter()
        .find(|n| {
            n.kind == "Action"
                && n.attributes
                    .get("kind")
                    .is_some_and(|k| k == "CompilerPass")
        })
        .unwrap()
        .clone();
    let mut original = original;
    let target_id = original
        .graph
        .edges
        .iter()
        .find(|e| {
            e.source == owner.id
                && e.kind == "reads"
                && original
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == e.target && n.kind == "Value")
        })
        .unwrap()
        .target
        .clone();
    let target = original
        .graph
        .nodes
        .iter()
        .find(|n| n.id == target_id)
        .unwrap()
        .clone();
    let port_name = target
        .name
        .strip_prefix(&format!("{}.", owner.name))
        .unwrap_or(&target.name)
        .to_owned();
    original
        .graph
        .edges
        .iter_mut()
        .find(|e| e.source == owner.id && e.target == target_id && e.kind == "reads")
        .unwrap()
        .attributes
        .remove("provenance");
    original
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == target_id)
        .unwrap()
        .attributes
        .remove("provenance");
    original
        .graph
        .edges
        .retain(|e| !(e.source == target_id && e.kind == "has_provenance"));
    let evidence = original.graph.add_node(
        "Provenance",
        format!("compiler_pass:{}.read:{port_name}", owner.name),
        None,
        BTreeMap::new(),
    );
    original
        .graph
        .add_edge("has_provenance", &target, &evidence, BTreeMap::new());
    assert!(
        !ail_document_from_core(&original).compiler_passes[&owner.name]
            .reads
            .contains(&port_name)
    );
    let mut candidate = original.clone();
    candidate
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == evidence.id)
        .unwrap()
        .name = format!("compiler_pass:{}.read:executable payload", owner.name);
    let diff = compare_ail_core_semantics(&original, &candidate);
    assert!(
        diff.changed.iter().any(|s| s.contains("effective reads")),
        "{}",
        render_ail_semantic_diff(&diff)
    );
    reject(&original, &candidate);
}
#[test]
fn typed_expression_core_spec_roundtrip_keeps_policy_provenance_exact() {
    let original = base();
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let text = render_ail_spec_from_core(&original);
    let doc = parse_ail_package_spec_text(&package, &text).unwrap();
    let candidate = elaborate_ail_core(&package, &doc);
    assert_eq!(
        compile_ail_core_bytecode(&original).unwrap(),
        compile_ail_core_bytecode(&candidate).unwrap()
    );
    let diff = compare_ail_core_semantics(&original, &candidate);
    // Reconstructed parentheses preserve instructions but alter the policy
    // provenance attribute, which this conservative contract keeps exact.
    assert!(!diff.is_equivalent());
    assert!(diff.changed.iter().any(|s| s.contains("provenance")));
}

#[test]
fn malformed_provenance_relations_are_not_treated_as_evidence() {
    let original = base();
    let thing = original
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Thing")
        .unwrap()
        .clone();
    let field = original
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Field")
        .unwrap()
        .clone();
    let evidence = original
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Provenance")
        .unwrap()
        .clone();
    let mut malformed = original.clone();
    malformed
        .graph
        .add_edge("has_field", &thing, &evidence, BTreeMap::new());
    let diff = compare_ail_core_semantics(&malformed, &malformed);
    assert!(
        !diff.invalid.is_empty(),
        "{}",
        render_ail_semantic_diff(&diff)
    );
    let mut malformed = original.clone();
    malformed
        .graph
        .add_edge("has_provenance", &thing, &field, BTreeMap::new());
    let diff = compare_ail_core_semantics(&malformed, &malformed);
    assert!(
        !diff.invalid.is_empty(),
        "{}",
        render_ail_semantic_diff(&diff)
    );
}

fn with_nominal_declaration(original: &AilCore, name: &str) -> AilCore {
    let mut candidate = original.clone();
    let declaration = candidate
        .graph
        .add_node("Type", name, None, BTreeMap::new());
    let provenance =
        candidate
            .graph
            .add_node("Provenance", format!("type:{name}"), None, BTreeMap::new());
    candidate
        .graph
        .add_edge("has_provenance", &declaration, &provenance, BTreeMap::new());
    assert!(
        check_ail_core(&candidate).is_empty(),
        "{name}: {:?}",
        check_ail_core(&candidate)
    );
    candidate
}
#[test]
fn all_builtin_type_families_are_reserved_for_implicit_nominal_additions() {
    let original = base();
    let heads = [
        "Int",
        "Bool",
        "Float",
        "Text",
        "Bytes",
        "Json",
        "List",
        "Map",
        "Set",
        "Option",
        "Result",
        "Secret",
        "State",
        "Unit",
        "Handle",
        "Pointer",
        "Resource",
        "Never",
        "Time",
        "Duration",
        "UInt8",
        "UInt16",
        "UInt32",
        "UInt64",
        "CInt",
        "CChar",
        "Void",
        "Decimal",
        "Money",
        "Buffer",
        "Device",
        "AIL-Core graph",
        "permission inference policy",
        "Diagnostic",
        "Nullable",
        "NonNull",
        "Callback",
    ];
    let mut missed = Vec::new();
    for name in heads {
        let candidate = with_nominal_declaration(&original, name);
        let diff = compare_ail_core_semantics(&original, &candidate);
        assert!(!diff.is_equivalent());
        assert!(
            diff.invalid.is_empty(),
            "{}",
            render_ail_semantic_diff(&diff)
        );
        if diff.preserves_baseline()
            || !diff
                .preservation_violations
                .iter()
                .any(|s| s.contains("nominal type head collision"))
        {
            missed.push(name);
        }
    }
    assert!(
        missed.is_empty(),
        "builtin heads falsely admitted or not identified as collisions: {missed:?}"
    );
}
#[test]
fn nominal_additions_cannot_collide_with_existing_thing_types() {
    let original = base();
    let candidate = with_nominal_declaration(&original, "Data");
    let diff = compare_ail_core_semantics(&original, &candidate);
    assert!(
        diff.invalid.is_empty(),
        "{}",
        render_ail_semantic_diff(&diff)
    );
    assert!(!diff.is_equivalent());
    assert!(
        !diff.preserves_baseline(),
        "{}",
        render_ail_semantic_diff(&diff)
    );
    assert!(
        diff.preservation_violations
            .iter()
            .any(|s| s.contains("nominal type head collision"))
    );
    // An unrelated nominal name still belongs to a disjoint declaration namespace.
    assert!(
        compare_ail_core_semantics(
            &original,
            &with_nominal_declaration(&original, "Independent")
        )
        .preserves_baseline()
    );
}

#[test]
fn shared_type_vocabulary_preserves_field_checker_acceptance_rules() {
    let original = core("The application Audit manages types.\nA Data has:\n- value: Int\n");
    let accepted = [
        "Bytes",
        "Text",
        "Time",
        "Duration",
        "Bool",
        "Json",
        "Int",
        "Float",
        "UInt8",
        "UInt16",
        "UInt32",
        "UInt64",
        "CInt",
        "CChar",
        "Void",
        "Decimal",
        "Money",
        "Buffer",
        "Device",
        "AIL-Core graph",
        "permission inference policy",
        "Diagnostic",
        "T",
        "Data",
        "State<Ready, Closed>",
        "Callback<arbitrary signature>",
        "Nullable<Int>",
        "NonNull<Text>",
        "Option<Int>",
        "List<Text>",
        "Secret<Text>",
        "Pointer<UInt8>",
        "Option<Nullable<NonNull<Int>>>",
    ];
    for name in accepted {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field")
            .unwrap()
            .type_name = Some(name.into());
        assert!(
            check_ail_core(&candidate).is_empty(),
            "{name}: {:?}",
            check_ail_core(&candidate)
        );
    }
    for name in [
        "Unknown",
        "Nullable",
        "NonNull",
        "Callback",
        "State<>",
        "NonNull<Unknown>",
        "Nullable<>",
        "Callback<unclosed",
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field")
            .unwrap()
            .type_name = Some(name.into());
        assert!(
            check_ail_core(&candidate)
                .iter()
                .any(|s| s.contains("AIL-TYPE-001")),
            "{name}: {:?}",
            check_ail_core(&candidate)
        );
    }
}
