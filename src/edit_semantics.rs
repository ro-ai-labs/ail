//! Checked structural comparison and conservative directional edit admission.
use super::*;

type Label = (String, String);
type Attributes = BTreeMap<String, String>;

/// Supplemental validation of existing symbolic compiled records for additions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AilSemanticLoweringValidation {
    #[default]
    NotRequired,
    BaselineUnavailable,
    CandidateFailed,
    ExistingRecordsPreserved,
    ExistingRecordsChanged,
}
impl AilSemanticLoweringValidation {
    fn report_label(self) -> &'static str {
        match self {
            Self::NotRequired => "not-required",
            Self::BaselineUnavailable => "baseline-unavailable (structural/effective-payload-only)",
            Self::CandidateFailed => "candidate-failed",
            Self::ExistingRecordsPreserved => "existing-records-preserved",
            Self::ExistingRecordsChanged => "existing-records-changed",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// Exact checked structural facts, with a directional declaration-addition policy.
/// This does not prove observational equivalence between different programs.
pub struct AilSemanticDiff {
    pub removed: Vec<String>,
    pub added: Vec<String>,
    pub retyped: Vec<String>,
    /// Modified attributes, metadata, local sequences, or effective payloads.
    pub changed: Vec<String>,
    /// Either input failed checking or had ambiguous identity/reference facts.
    pub invalid: Vec<String>,
    /// Additions outside declaration ownership or changing existing compilation.
    pub preservation_violations: Vec<String>,
    pub lowering_validation: AilSemanticLoweringValidation,
}
impl AilSemanticDiff {
    pub fn is_equivalent(&self) -> bool {
        self.preserves_baseline() && self.added.is_empty()
    }
    pub fn preserves_baseline(&self) -> bool {
        self.removed.is_empty()
            && self.retyped.is_empty()
            && self.changed.is_empty()
            && self.invalid.is_empty()
            && self.preservation_violations.is_empty()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeFact {
    type_name: Option<String>,
    attributes: Attributes,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Relation {
    target: Label,
    attributes: Attributes,
}
struct Projection {
    nodes: BTreeMap<Label, NodeFact>,
    relations: BTreeMap<(Label, String), Vec<Relation>>,
    payloads: BTreeMap<(Label, String), Vec<String>>,
}
fn label(n: &Node) -> Label {
    (n.kind.clone(), n.name.clone())
}
fn display(l: &Label) -> String {
    format!("{} {}", l.0, l.1)
}
fn resolve_expression<'a>(core: &'a AilCore, reference: &str) -> Result<&'a Node, String> {
    let mut matches = core
        .graph
        .nodes
        .iter()
        .filter(|n| n.id == reference || core_node_label(n) == reference);
    match (matches.next(), matches.next()) {
        (Some(n), None) if n.kind == "IntegerExpression" => Ok(n),
        (Some(_), Some(_)) => Err(format!(
            "ambiguous writes.expression reference {reference:?}"
        )),
        _ => Err(format!("invalid writes.expression reference {reference:?}")),
    }
}
fn validate(core: &AilCore, side: &str, diff: &mut AilSemanticDiff) -> bool {
    let before = diff.invalid.len();
    let mut ids = BTreeSet::new();
    let mut labels = BTreeSet::new();
    for node in &core.graph.nodes {
        if !ids.insert(&node.id) {
            diff.invalid.push(format!(
                "{side}: duplicate node ID for {}",
                display(&label(node))
            ));
        }
        if node.kind != "Provenance" && !labels.insert(label(node)) {
            diff.invalid.push(format!(
                "{side}: duplicate semantic label {}",
                display(&label(node))
            ));
        }
    }
    let by_id: BTreeMap<_, _> = core
        .graph
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n))
        .collect();
    for edge in &core.graph.edges {
        if let (Some(source), Some(target)) = (
            by_id.get(edge.source.as_str()),
            by_id.get(edge.target.as_str()),
        ) && (edge.kind == "has_provenance" && target.kind != "Provenance"
            || edge.kind != "has_provenance"
                && (source.kind == "Provenance" || target.kind == "Provenance"))
        {
            diff.invalid.push(format!(
                "{side}: malformed evidence relation {} {} -> {}",
                edge.kind,
                display(&label(source)),
                display(&label(target))
            ));
        }
    }
    for edge in core.graph.edges.iter().filter(|e| e.kind == "writes") {
        if let Some(reference) = edge.attributes.get("expression")
            && let Err(error) = resolve_expression(core, reference)
        {
            diff.invalid.push(format!("{side}: {error}"));
        }
    }
    // Check even self-comparisons. Never turn a malformed graph into equal facts.
    diff.invalid.extend(
        check_ail_core(core)
            .into_iter()
            .map(|d| format!("{side}: {}", d)),
    );
    before == diff.invalid.len()
}
fn project(core: &AilCore) -> Projection {
    let by_id: BTreeMap<_, _> = core
        .graph
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n))
        .collect();
    let nodes = core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind != "Provenance")
        .map(|n| {
            (
                label(n),
                NodeFact {
                    type_name: n.type_name.clone(),
                    attributes: n.attributes.clone(),
                },
            )
        })
        .collect();
    let mut relations: BTreeMap<(Label, String), Vec<Relation>> = BTreeMap::new();
    let mut edges: Vec<_> = core.graph.edges.iter().collect();
    edges.sort_by_key(|e| e.ordinal);
    for edge in edges {
        let source = by_id[edge.source.as_str()];
        let target = by_id[edge.target.as_str()];
        if edge.kind == "has_provenance"
            || source.kind == "Provenance"
            || target.kind == "Provenance"
        {
            continue;
        }
        let mut attributes = edge.attributes.clone();
        if edge.kind == "writes"
            && let Some(reference) = attributes.get_mut("expression")
        {
            *reference =
                core_node_label(resolve_expression(core, reference).expect("validated expression"));
        }
        relations
            .entry((label(source), edge.kind.clone()))
            .or_default()
            .push(Relation {
                target: label(target),
                attributes,
            });
    }
    // Use the compiler's canonical projection once. These specific vectors carry
    // executable provenance fallback; all other evidence can be omitted.
    let document = ail_document_from_core(core);
    let mut payloads = BTreeMap::new();
    for (name, action) in document.actions {
        payloads.insert(
            (("Action".into(), name.clone()), "reads".into()),
            action.reads,
        );
        payloads.insert((("Action".into(), name), "writes".into()), action.writes);
    }
    for (name, tool) in document.tools {
        for (role, values) in [
            ("reads", tool.reads),
            ("writes", tool.writes),
            ("calls", tool.calls),
        ] {
            payloads.insert((("Tool".into(), name.clone()), role.into()), values);
        }
    }
    for (name, pass) in document.compiler_passes {
        payloads.insert(
            (("Action".into(), name.clone()), "reads".into()),
            pass.reads,
        );
        payloads.insert((("Action".into(), name), "writes".into()), pass.writes);
    }
    for (name, failure) in document.failures {
        payloads.insert(
            (("Failure".into(), name), "handling".into()),
            failure.handling,
        );
    }
    Projection {
        nodes,
        relations,
        payloads,
    }
}
fn ordinary_root(l: &Label, fact: &NodeFact) -> bool {
    matches!(l.0.as_str(), "Function" | "Type")
        || l.0 == "Action"
            && fact
                .attributes
                .get("kind")
                .is_none_or(|k| k != "CompilerPass")
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Own,
    Reference,
    Atom,
}
// Schema roles are deliberately explicit. Reachability alone cannot authorize
// a new Failure, foreign binding, field dictionary, or executable orphan.
fn role(source: &str, relation: &str, target: &str) -> Option<Role> {
    use Role::*;
    match (source, relation, target) {
        ("Type", "contains", "Variant") | ("Variant", "has_field", "Field") => Some(Own),
        ("Function", "has_input", "Input")
        | ("Function", "has_output", "Output")
        | (
            "Function",
            "function_parameter" | "function_local" | "value_parameter" | "value_local",
            "Field",
        )
        | ("Function", "function_step", "FunctionStep")
        | ("Function", "value_function_step", "ValueFunctionStep")
        | ("Function", "contains", "Branch" | "Return")
        | ("Function", "calls", "Call")
        | ("Function", "has_termination_bound", "TerminationBound")
        | ("Function", "has_termination_measure", "TerminationMeasure") => Some(Own),
        ("Action", "control_step", "Control")
        | ("Action", "integer_call_step", "ValueCall" | "ValueFunctionCall")
        | ("Action", "bytes_step", "BytesOperation")
        | ("Action", "text_step", "TextOperation")
        | ("Action", "list_step", "ListOperation")
        | ("Action", "literal_step", "LiteralAssignment")
        | ("Action", "directory_step", "DirectoryOperation")
        | ("Action", "process_step", "ProcessOperation")
        | ("Action", "stream_step", "StandardStreamOperation")
        | ("Action", "file_step", "FileOperation")
        | ("Action", "json_step", "JsonConversion")
        | ("Action", "json_access_step", "JsonAccess")
        | ("Action", "json_mutation_step", "JsonMutation")
        | ("Action", "float_step", "FloatOperation")
        | ("Action", "boolean_step", "BooleanOperation")
        | ("Action", "variant_step", "VariantOperation") => Some(Own),
        ("Action", "calls" | "repeats", "Action")
        | ("Action", "may_fail_with", "Failure")
        | ("Action", "returns_field" | "protects_secret", "Field") => Some(Reference),
        ("Action" | "Function", "records_trace", "Trace")
        | ("Action", "requires", "Rule" | "Permission" | "Capability")
        | ("Action", "guarantees", "Guarantee")
        | ("Action", "requires_approval", "Approval")
        | ("Action", "requires_confirmation", "Confirmation")
        | ("Action", "reads" | "writes", "Effect" | "Value") => Some(Atom),
        ("Action", "reads" | "writes", "Field") => Some(Reference),
        ("FunctionStep" | "ValueCall", "call_target", "Function")
        | ("ValueFunctionStep" | "ValueFunctionCall", "value_call_target", "Function") => {
            Some(Reference)
        }
        (
            "FunctionStep" | "ValueCall",
            "call_argument" | "function_expression",
            "IntegerExpression",
        )
        | ("FunctionStep", "function_condition", "IntegerComparison") => Some(Own),
        ("FunctionStep" | "ValueCall", "call_destination" | "function_destination", "Field") => {
            Some(Reference)
        }
        (
            "ValueFunctionStep" | "ValueFunctionCall" | "VariantOperation",
            "value_expression_operand" | "value_call_argument" | "variant_argument",
            "IntegerExpression" | "FloatExpression" | "BooleanExpression",
        )
        | (
            "ValueFunctionStep",
            "value_condition",
            "IntegerComparison" | "FloatComparison" | "BooleanExpression",
        ) => Some(Own),
        (
            "ValueFunctionStep" | "ValueFunctionCall" | "VariantOperation",
            "value_field_operand" | "value_call_argument" | "variant_argument" | "variant_operand",
            "Field",
        ) => Some(Reference),
        (
            "IntegerExpression",
            "left_operand" | "right_operand" | "unary_operand",
            "IntegerExpression",
        )
        | (
            "FloatExpression",
            "left_operand" | "right_operand" | "unary_operand",
            "FloatExpression",
        )
        | (
            "BooleanExpression",
            "boolean_left" | "boolean_right" | "boolean_operand",
            "BooleanExpression" | "IntegerExpression" | "FloatExpression" | "TextOperand",
        )
        | ("IntegerComparison", "comparison_left" | "comparison_right", "IntegerExpression")
        | ("FloatComparison", "comparison_left" | "comparison_right", "FloatExpression")
        | ("Control", "condition", "IntegerComparison" | "FloatComparison" | "BooleanExpression") => {
            Some(Own)
        }
        ("IntegerExpression" | "FloatExpression", "expression_field", "Field")
        | ("BooleanExpression", "boolean_field", "Field")
        | ("TextOperand", "text_field", "Field")
        | ("Control", "recovery_destination", "Field") => Some(Reference),
        _ => operation_role(source, relation, target),
    }
}
fn operation_role(source: &str, relation: &str, target: &str) -> Option<Role> {
    // Each operation family declares its own finite operand vocabulary.
    let roles: &[&str] = match source {
        "BytesOperation" => &[
            "bytes_source",
            "bytes_other",
            "bytes_destination",
            "bytes_index",
            "bytes_start",
            "bytes_end",
            "bytes_value",
        ],
        "TextOperation" => &[
            "text_source",
            "text_other",
            "text_replacement",
            "text_destination",
            "text_start",
            "text_end",
        ],
        "ListOperation" => &[
            "list_source",
            "list_other",
            "list_destination",
            "list_index",
            "list_start",
            "list_end",
            "list_value",
        ],
        "LiteralAssignment" => &["literal_destination"],
        "DirectoryOperation" => &[
            "directory_path",
            "directory_source",
            "directory_destination",
        ],
        "ProcessOperation" => &[
            "process_arguments",
            "process_input",
            "process_timeout",
            "process_stdout",
            "process_stderr",
            "process_status",
        ],
        "StandardStreamOperation" => &["stream_source", "stream_destination"],
        "FileOperation" => &["file_source", "file_destination"],
        "JsonConversion" => &["json_source", "json_destination"],
        "JsonAccess" => &[
            "json_access_source",
            "json_access_destination",
            "json_access_path",
        ],
        "JsonMutation" => &[
            "json_mutation_source",
            "json_mutation_destination",
            "json_mutation_path",
            "json_mutation_value",
        ],
        "FloatOperation" => &["float_source", "float_destination", "float_expression"],
        "BooleanOperation" => &["boolean_destination", "boolean_expression"],
        "VariantOperation" => &["variant_operand", "variant_argument"],
        _ => &[],
    };
    if !roles.contains(&relation) {
        return None;
    }
    match target {
        "Field" => Some(Role::Reference),
        "IntegerExpression" | "FloatExpression" | "BooleanExpression" => Some(Role::Own),
        _ => None,
    }
}
fn head(name: &str) -> &str {
    name.split('<').next().unwrap_or(name).trim()
}
fn allowed_additions(
    b: &Projection,
    c: &Projection,
    core: &AilCore,
    diff: &mut AilSemanticDiff,
) -> BTreeSet<Label> {
    let new: BTreeSet<_> = c
        .nodes
        .keys()
        .filter(|l| !b.nodes.contains_key(*l))
        .cloned()
        .collect();
    let roots: BTreeSet<_> = new
        .iter()
        .filter(|l| ordinary_root(l, &c.nodes[*l]))
        .cloned()
        .collect();
    let mut reached = roots.clone();
    let mut pending: Vec<_> = roots.iter().cloned().collect();
    let applications: Vec<_> = b.nodes.keys().filter(|l| l.0 == "Application").collect();
    for root in &roots {
        if matches!(root.0.as_str(), "Action" | "Function") {
            if b.nodes
                .keys()
                .chain(roots.iter().filter(|l| *l != root))
                .any(|l| {
                    l.1 == root.1
                        && matches!(
                            l.0.as_str(),
                            "Action"
                                | "Function"
                                | "Tool"
                                | "SystemComponent"
                                | "Route"
                                | "Form"
                                | "Dashboard"
                                | "Workflow"
                        )
                })
            {
                diff.preservation_violations
                    .push(format!("runtime name collision for {}", display(root)));
            }
            if !applications.is_empty()
                && !applications.iter().any(|app| {
                    c.relations
                        .get(&((*app).clone(), "contains".into()))
                        .is_some_and(|v| v.iter().any(|r| &r.target == root))
                })
            {
                diff.preservation_violations.push(format!(
                    "new {} lacks Application.contains ownership",
                    display(root)
                ));
            }
        }
        if root.0 == "Type"
            && (is_reserved_ail_type_head(head(&root.1))
                || b.nodes
                    .keys()
                    .chain(roots.iter().filter(|l| *l != root))
                    .any(|l| {
                        matches!(l.0.as_str(), "Thing" | "Type") && head(&l.1) == head(&root.1)
                    }))
        {
            diff.preservation_violations
                .push(format!("nominal type head collision for {}", display(root)));
        }
    }
    // Only Thing-owned fields are public baseline field references. Locals and
    // variant payload fields belong to existing declarations and cannot be adopted.
    let public_fields: BTreeSet<_> = b
        .relations
        .iter()
        .filter(|((l, k), _)| l.0 == "Thing" && k == "has_field")
        .flat_map(|(_, rs)| rs.iter().map(|r| r.target.clone()))
        .collect();
    while let Some(owner) = pending.pop() {
        for ((source, kind), values) in c.relations.iter().filter(|((s, _), _)| s == &owner) {
            for value in values {
                let target = &value.target;
                let edge_role = role(&source.0, kind, &target.0);
                let target_new = new.contains(target);
                let valid = match edge_role {
                    Some(Role::Own) => target_new,
                    Some(Role::Reference) => {
                        if target_new {
                            reached.contains(target) || roots.contains(target)
                        } else {
                            target.0 != "Field" || public_fields.contains(target)
                        }
                    }
                    Some(Role::Atom) => true,
                    None => false,
                };
                // New local fields may be visited before the owning port edge.
                // Defer reachability of references until the full closure is built.
                if !(valid || edge_role == Some(Role::Reference) && target_new) {
                    diff.preservation_violations.push(format!(
                        "unauthorized new relation {kind} {} -> {}",
                        display(source),
                        display(target)
                    ));
                }
                if target_new
                    && matches!(edge_role, Some(Role::Own | Role::Atom))
                    && reached.insert(target.clone())
                {
                    pending.push(target.clone());
                }
            }
        }
        for edge in core.graph.edges.iter().filter(|e| e.kind == "writes") {
            let source = core
                .graph
                .nodes
                .iter()
                .find(|n| n.id == edge.source)
                .expect("checked source");
            if label(source) != owner {
                continue;
            }
            if let Some(reference) = edge.attributes.get("expression") {
                let target = label(resolve_expression(core, reference).expect("checked reference"));
                if !new.contains(&target) {
                    diff.preservation_violations.push(format!(
                        "new {} adopts existing expression {}",
                        display(&owner),
                        display(&target)
                    ));
                } else if reached.insert(target.clone()) {
                    pending.push(target);
                }
            }
        }
    }
    for node in new.difference(&reached) {
        diff.preservation_violations.push(format!(
            "new {} is outside permitted declaration ownership",
            display(node)
        ));
    }
    roots
}
fn edge_report(source: &Label, kind: &str, relation: &Relation) -> String {
    format!(
        "edge {kind} {} -> {} attributes {:?}",
        display(source),
        display(&relation.target),
        relation.attributes
    )
}
pub fn compare_ail_core_semantics(baseline: &AilCore, candidate: &AilCore) -> AilSemanticDiff {
    let mut diff = AilSemanticDiff::default();
    let baseline_valid = validate(baseline, "baseline", &mut diff);
    let candidate_valid = validate(candidate, "candidate", &mut diff);
    if !baseline_valid || !candidate_valid {
        diff.invalid.sort();
        diff.invalid.dedup();
        return diff;
    }
    let b = project(baseline);
    let c = project(candidate);
    if baseline.package != candidate.package {
        diff.changed.push(format!(
            "package metadata: expected {:?}; actual {:?}",
            baseline.package, candidate.package
        ));
    }
    for (l, fact) in &b.nodes {
        match c.nodes.get(l) {
            None => diff.removed.push(format!("node {}", display(l))),
            Some(actual) => {
                if fact.type_name != actual.type_name {
                    diff.retyped.push(format!(
                        "node {}: {:?} -> {:?}",
                        display(l),
                        fact.type_name,
                        actual.type_name
                    ));
                }
                if fact.attributes != actual.attributes {
                    diff.changed.push(format!(
                        "node {} attributes: expected {:?}; actual {:?}",
                        display(l),
                        fact.attributes,
                        actual.attributes
                    ));
                }
            }
        }
    }
    for l in c.nodes.keys().filter(|l| !b.nodes.contains_key(*l)) {
        diff.added.push(format!("node {}", display(l)));
    }
    let roots = allowed_additions(&b, &c, candidate, &mut diff);
    let relation_keys: BTreeSet<_> = b
        .relations
        .keys()
        .chain(c.relations.keys())
        .cloned()
        .collect();
    for (owner, kind) in relation_keys {
        let key = (owner.clone(), kind.clone());
        let before = b.relations.get(&key).cloned().unwrap_or_default();
        let after = c.relations.get(&key).cloned().unwrap_or_default();
        let frozen_after: Vec<_> = after
            .iter()
            .filter(|r| {
                !(owner.0 == "Application"
                    && kind == "contains"
                    && roots.contains(&r.target)
                    && matches!(r.target.0.as_str(), "Action" | "Function"))
            })
            .cloned()
            .collect();
        if b.nodes.contains_key(&owner) && before != frozen_after {
            diff.changed.push(format!(
                "{} {kind} sequence: expected {before:?}; actual {after:?}",
                display(&owner)
            ));
        }
        // Multiset subtraction preserves repeated identical edge occurrences.
        let mut remaining = after.clone();
        for r in &before {
            if let Some(i) = remaining.iter().position(|v| v == r) {
                remaining.remove(i);
            } else {
                diff.removed.push(edge_report(&owner, &kind, r));
            }
        }
        diff.added
            .extend(remaining.iter().map(|r| edge_report(&owner, &kind, r)));
    }
    for ((owner, role), before) in &b.payloads {
        let after = c.payloads.get(&(owner.clone(), role.clone()));
        if after != Some(before) {
            diff.changed.push(format!(
                "{} effective {role}: expected {before:?}; actual {after:?}",
                display(owner)
            ));
        }
    }
    if !roots.is_empty() {
        validate_existing_compiled_records(baseline, candidate, &mut diff);
    }
    for entries in [
        &mut diff.removed,
        &mut diff.added,
        &mut diff.retyped,
        &mut diff.changed,
        &mut diff.preservation_violations,
    ] {
        entries.sort();
    }
    diff
}
/// Supplemental protection of existing symbolic compiled records against the
/// full candidate. Baseline lowering unavailability retains only the structural
/// and effective-payload claim established by the caller's comparison.
pub(super) fn validate_existing_compiled_records(
    baseline: &AilCore,
    candidate: &AilCore,
    diff: &mut AilSemanticDiff,
) {
    match compile_ail_core_bytecode(baseline) {
        Err(_) => diff.lowering_validation = AilSemanticLoweringValidation::BaselineUnavailable,
        Ok(before) => match compile_ail_core_bytecode(candidate) {
            Err(error) => {
                diff.lowering_validation = AilSemanticLoweringValidation::CandidateFailed;
                diff.preservation_violations
                    .push(format!("candidate no longer lowers: {error}"));
            }
            Ok(after) => {
                let before_violations = diff.preservation_violations.len();
                for (name, record) in before.actions {
                    if after.actions.get(&name) != Some(&record) {
                        diff.preservation_violations
                            .push(format!("existing compiled action/function {name} changed"));
                    }
                }
                for (name, record) in before.failures {
                    if after.failures.get(&name) != Some(&record) {
                        diff.preservation_violations
                            .push(format!("existing compiled failure {name} changed"));
                    }
                }
                for (name, record) in before.type_schemas {
                    if after.type_schemas.get(&name) != Some(&record) {
                        diff.preservation_violations
                            .push(format!("existing concrete type schema {name} changed"));
                    }
                }
                diff.lowering_validation =
                    if diff.preservation_violations.len() == before_violations {
                        AilSemanticLoweringValidation::ExistingRecordsPreserved
                    } else {
                        AilSemanticLoweringValidation::ExistingRecordsChanged
                    };
            }
        },
    }
}
pub fn render_ail_semantic_diff(diff: &AilSemanticDiff) -> String {
    let mut report = "kind: AIL-Semantic-Diff\n".to_string();
    for (name, entries) in [
        ("removed", &diff.removed),
        ("added", &diff.added),
        ("retyped", &diff.retyped),
        ("changed", &diff.changed),
        ("invalid", &diff.invalid),
        ("preservation-violation", &diff.preservation_violations),
    ] {
        report.push_str(&format!("{name}-count {}\n", entries.len()));
    }
    report.push_str(&format!(
        "lowering-validation {}\n",
        diff.lowering_validation.report_label()
    ));
    report.push_str(&format!(
        "preserves-baseline {}\nsemantically-equivalent {}\n",
        diff.preserves_baseline(),
        diff.is_equivalent()
    ));
    for (name, entries) in [
        ("removed", &diff.removed),
        ("retyped", &diff.retyped),
        ("added", &diff.added),
        ("changed", &diff.changed),
        ("invalid", &diff.invalid),
        ("preservation-violation", &diff.preservation_violations),
    ] {
        for entry in entries {
            report.push_str(&format!("{name} {entry}\n"));
        }
    }
    report
}
