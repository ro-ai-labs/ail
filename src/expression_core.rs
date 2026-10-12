//! Typed expression graph construction and validation. Provenance is explanatory,
//! never executable: lowering reconstructs expressions from nodes and edges.
use super::*;
use crate::expression::{IntExpr, IntOperator, MAX_EXPRESSION_DEPTH, MAX_EXPRESSION_NODES};

pub(super) fn assignment(
    document: &AilDocument,
    write: &str,
) -> Option<Result<(String, IntExpr), String>> {
    if let Some(rest) = write.strip_prefix("computes ") {
        return Some((|| {
            let (key, source) = rest
                .split_once(" as ")
                .ok_or("expected 'computes field as expression'")?;
            if !matches!(IntExpr::parse(key)?, IntExpr::Field(parsed) if parsed == key) {
                return Err("destination must be a field reference".into());
            }
            Ok((key.to_string(), IntExpr::parse(source)?))
        })());
    }
    let (rest, op) = write
        .strip_prefix("increments ")
        .map(|s| (s, IntOperator::Add))
        .or_else(|| {
            write
                .strip_prefix("decrements ")
                .map(|s| (s, IntOperator::Subtract))
        })?;
    // Preserve the established literal opcode, but never accept only a numeric prefix.
    if field_integer_delta_assignment(document, write).is_some() {
        return None;
    }
    Some((|| {
        let (target, source) = rest
            .rsplit_once(" by ")
            .ok_or("expected an arithmetic operand after 'by'")?;
        let key = referenced_runtime_field_key(document, target)
            .ok_or("unknown arithmetic destination")?;
        Ok((
            key.clone(),
            IntExpr::Binary {
                op,
                left: Box::new(IntExpr::Field(key)),
                right: Box::new(IntExpr::parse(source)?),
            },
        ))
    })())
}

pub(super) fn field_key(node: &Node) -> String {
    if node.attributes.contains_key("function_owner")
        && let Some(name) = node.attributes.get("local_name")
    {
        return format!("local.{name}");
    }
    node.name
        .rsplit_once('.')
        .map(|(owner, field)| runtime_field_key(owner, field))
        .unwrap_or_else(|| node.name.clone())
}
fn field(graph: &Graph, key: &str) -> Option<Node> {
    graph
        .nodes
        .iter()
        .find(|n| n.kind == "Field" && field_key(n) == key)
        .cloned()
}
fn target(graph: &mut Graph, key: &str) -> Node {
    field(graph, key).unwrap_or_else(|| graph.add_node("Effect", key, None, BTreeMap::new()))
}
fn append_edge(
    graph: &mut Graph,
    kind: &str,
    source: &Node,
    target: &Node,
    attrs: BTreeMap<String, String>,
) {
    let mut edge = Edge::new(kind, source, target, attrs);
    edge.ordinal = graph.edges.len();
    graph.edges.push(edge);
}
pub(super) fn build(graph: &mut Graph, action: &Node, name: &str, expr: &IntExpr) -> Node {
    let attrs = match expr {
        IntExpr::Literal(value) => attr(&[
            ("expression_kind", "literal"),
            ("value", &value.to_string()),
        ]),
        IntExpr::Field(_) => attr(&[("expression_kind", "field")]),
        IntExpr::Binary { op, .. } => attr(&[
            ("expression_kind", "binary"),
            (
                "operator",
                match op {
                    IntOperator::Add => "+",
                    IntOperator::Subtract => "-",
                    IntOperator::Multiply => "*",
                    IntOperator::Divide => "/",
                    IntOperator::Remainder => "%",
                },
            ),
        ]),
    };
    let node = graph.add_node("IntegerExpression", name, Some("Int".into()), attrs);
    attach_provenance(
        graph,
        &node,
        format!("action:{}.expression:{name}", action.name),
    );
    match expr {
        IntExpr::Literal(_) => {}
        IntExpr::Field(key) => {
            let operand = target(graph, key);
            graph.add_edge("expression_field", &node, &operand, BTreeMap::new());
            graph.add_edge(
                "reads",
                action,
                &operand,
                attr(&[("inferred_expression", "true")]),
            );
        }
        IntExpr::Binary { left, right, .. } => {
            let left = build(graph, action, &format!("{name}.left"), left);
            let right = build(graph, action, &format!("{name}.right"), right);
            graph.add_edge("left_operand", &node, &left, BTreeMap::new());
            graph.add_edge("right_operand", &node, &right, BTreeMap::new());
        }
    }
    node
}
pub(super) fn elaborate_write(
    graph: &mut Graph,
    document: &AilDocument,
    action: &Node,
    index: usize,
    write: &str,
) -> bool {
    let Some(parsed) = assignment(document, write) else {
        return false;
    };
    let name = format!("{}.write{index}", action.name);
    let (destination, root) = match parsed {
        Ok((key, expression)) => (
            target(graph, &key),
            build(graph, action, &name, &expression),
        ),
        Err(error) => (
            target(graph, write),
            graph.add_node(
                "IntegerExpression",
                name,
                Some("Int".into()),
                attr(&[("expression_kind", "invalid"), ("error", &error)]),
            ),
        ),
    };
    let provenance = format!("action:{}.write:{write}", action.name);
    append_edge(
        graph,
        "writes",
        action,
        &destination,
        attr(&[
            ("expression", &core_node_label(&root)),
            ("provenance", &provenance),
            ("write_index", &index.to_string()),
        ]),
    );
    true
}
pub(super) fn elaborate_outputs(graph: &mut Graph, action: &Node, outputs: &[String]) {
    for key in outputs {
        let destination = target(graph, key);
        graph.add_edge("returns_field", action, &destination, BTreeMap::new());
        graph.add_edge(
            "reads",
            action,
            &destination,
            attr(&[("inferred_expression", "true")]),
        );
    }
}
fn child<'a>(core: &'a AilCore, node: &Node, kind: &str) -> Result<&'a Node, String> {
    let edges: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == kind)
        .collect();
    if edges.len() != 1 {
        return Err(format!("{} requires exactly one {kind} edge", node.name));
    }
    core.graph
        .nodes
        .iter()
        .find(|n| n.id == edges[0].target)
        .ok_or_else(|| "expression edge references missing node".into())
}
fn decode(core: &AilCore, id: &str, depth: usize, count: &mut usize) -> Result<IntExpr, String> {
    *count += 1;
    if depth > MAX_EXPRESSION_DEPTH || *count > MAX_EXPRESSION_NODES {
        return Err("expression graph exceeds depth or node limit (possibly cyclic)".into());
    }
    let node = core
        .graph
        .nodes
        .iter()
        .find(|n| n.id == id || core_node_label(n) == id)
        .ok_or("missing expression node")?;
    if node.kind != "IntegerExpression" || node.type_name.as_deref() != Some("Int") {
        return Err("expression node must have kind IntegerExpression and type Int".into());
    }
    let kind = node
        .attributes
        .get("expression_kind")
        .map(String::as_str)
        .unwrap_or("");
    let allowed: &[&str] = match kind {
        "literal" => &[],
        "field" => &["expression_field"],
        "binary" => &["left_operand", "right_operand"],
        _ => {
            return Err(node
                .attributes
                .get("error")
                .cloned()
                .unwrap_or_else(|| "invalid expression kind".into()));
        }
    };
    if core.graph.edges.iter().any(|e| {
        e.source == node.id
            && matches!(
                e.kind.as_str(),
                "left_operand" | "right_operand" | "expression_field"
            )
            && !allowed.contains(&e.kind.as_str())
    }) {
        return Err("unexpected operand edge on expression".into());
    }
    match kind {
        "literal" => Ok(IntExpr::Literal(crate::expression::parse_integer(
            node.attributes
                .get("value")
                .ok_or("missing literal value")?,
        )?)),
        "field" => {
            let field = child(core, node, "expression_field")?;
            if field.kind != "Field" || field.type_name.as_deref() != Some("Int") {
                return Err(format!(
                    "expression field {} must be a declared Int",
                    field.name
                ));
            }
            let key = field_key(field);
            if !matches!(IntExpr::parse(&key)?, IntExpr::Field(_)) {
                return Err("invalid runtime field key".into());
            }
            Ok(IntExpr::Field(key))
        }
        "binary" => {
            let op = match node.attributes.get("operator").map(String::as_str) {
                Some("+") => IntOperator::Add,
                Some("-") => IntOperator::Subtract,
                Some("*") => IntOperator::Multiply,
                Some("/") => IntOperator::Divide,
                Some("%") => IntOperator::Remainder,
                _ => return Err("invalid integer operator".into()),
            };
            Ok(IntExpr::Binary {
                op,
                left: Box::new(decode(
                    core,
                    &child(core, node, "left_operand")?.id,
                    depth + 1,
                    count,
                )?),
                right: Box::new(decode(
                    core,
                    &child(core, node, "right_operand")?.id,
                    depth + 1,
                    count,
                )?),
            })
        }
        _ => unreachable!(),
    }
}
pub(super) fn decode_root(core: &AilCore, id: &str) -> Result<IntExpr, String> {
    let expression = decode(core, id, 1, &mut 0)?;
    IntExpr::parse(&expression.to_source())?;
    Ok(expression)
}
pub(super) fn write_text(core: &AilCore, edge: &Edge, destination: &Node) -> Option<String> {
    edge.attributes.get("expression").map(|root| {
        format!(
            "computes {} as {}",
            field_key(destination),
            decode_root(core, root)
                .map(|e| e.to_source())
                .unwrap_or_else(|_| "invalid".into())
        )
    })
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut errors = Vec::new();
    for edge in &core.graph.edges {
        let result = if edge.kind == "writes" {
            if let Some(root) = edge.attributes.get("expression") {
                let destination = core.graph.nodes.iter().find(|n| n.id == edge.target);
                if !destination
                    .is_some_and(|n| n.kind == "Field" && n.type_name.as_deref() == Some("Int"))
                {
                    Err("integer expression destination must be a declared Int field".into())
                } else {
                    decode_root(core, root).map(|_| ())
                }
            } else if edge.attributes.get("provenance").is_some_and(|s| {
                s.split_once(".write:")
                    .is_some_and(|(_, s)| s.starts_with("computes "))
            }) {
                Err("computed write is missing its typed expression".into())
            } else if edge
                .attributes
                .get("provenance")
                .and_then(|p| p.split_once(".write:"))
                .is_some_and(|(_, w)| w.starts_with("increments ") || w.starts_with("decrements "))
            {
                if core.graph.nodes.iter().any(|n| {
                    n.id == edge.target
                        && n.kind == "Field"
                        && n.type_name.as_deref() == Some("Int")
                }) {
                    Ok(())
                } else {
                    Err("arithmetic destination must be a declared Int field".into())
                }
            } else {
                Ok(())
            }
        } else if edge.kind == "returns_field" {
            if core
                .graph
                .nodes
                .iter()
                .any(|n| n.id == edge.source && n.kind == "Action")
                && core
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == edge.target && n.kind == "Field")
            {
                Ok(())
            } else {
                Err("returned value must be a declared field on an action".into())
            }
        } else {
            Ok(())
        };
        if let Err(message) = result {
            errors.push(
                AilDiagnostic::error("AIL-EXPR-001", message)
                    .with_affected_graph_item(format!("edge:{}", edge.id))
                    .with_source_provenance(edge.attributes.get("provenance").cloned()),
            );
        }
    }
    errors
}

pub(super) fn namespace_write(document: &AilDocument, write: &str, alias: &str) -> Option<String> {
    let (key, expression) = assignment(document, write)?.ok()?;
    let prefix = runtime_subject_key(alias);
    let postfix = expression
        .to_postfix()
        .split_whitespace()
        .map(|token| {
            token
                .strip_prefix("f:")
                .map(|key| format!("f:{prefix}.{key}"))
                .unwrap_or_else(|| token.to_string())
        })
        .collect::<Vec<_>>()
        .join(" ");
    let expression = IntExpr::from_postfix(&postfix).ok()?;
    Some(format!(
        "computes {prefix}.{key} as {}",
        expression.to_source()
    ))
}

pub(super) fn run_document(
    document: &AilDocument,
    action: &str,
    state: BTreeMap<String, String>,
) -> Result<AilRunResult, String> {
    for action in document.actions.values() {
        for write in &action.writes {
            if let Some((key, _)) = field_integer_delta_assignment(document, write)
                && runtime_field_type_name(document, &key) != Some("Int")
            {
                return Err(format!("AIL-EXPR-001: {key} must be a declared Int field"));
            }
            if let Some(parsed) = assignment(document, write) {
                let (key, expression) = parsed?;
                for field in expression.fields().iter().chain(std::iter::once(&key)) {
                    if runtime_field_type_name(document, field) != Some("Int") {
                        return Err(format!(
                            "AIL-EXPR-001: {field} must be a declared Int field"
                        ));
                    }
                }
            }
        }
        for field in &action.outputs {
            if runtime_field_type_name(document, field).is_none() {
                return Err(format!("AIL-EXPR-001: unknown returned field {field}"));
            }
        }
    }
    let mut program = AilBytecodeProgram {
        type_schemas: AilValueSchemas::new(),
        package_name: document.application.name.clone(),
        package_version: String::new(),
        profile: "Application".into(),
        capability_grants: Vec::new(),
        target_support: BTreeMap::new(),
        external_bindings_metadata_present: true,
        external_bindings: document.external_bindings.clone(),
        actions: document
            .actions
            .iter()
            .map(|(name, action)| {
                compile_ail_action_bytecode(document, action).map(|action| (name.clone(), action))
            })
            .collect::<Result<_, _>>()?,
        failures: document
            .failures
            .iter()
            .map(|(name, failure)| {
                (
                    name.clone(),
                    AilBytecodeFailure {
                        name: name.clone(),
                        traces: failure.traces.clone(),
                    },
                )
            })
            .collect(),
    };
    for function in document.functions.values().filter(|f| f.integer.is_some()) {
        let body = integer_function::compile(function, document)?;
        if program
            .actions
            .insert(function.name.clone(), body)
            .is_some()
        {
            return Err("AIL-FUNCTION-001: action/function name collision".into());
        }
    }
    run_ail_bytecode_action(&program, action, state)
}
