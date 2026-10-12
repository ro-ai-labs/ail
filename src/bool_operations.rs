//! Boolean computations and explicitly owned predicate trees in checked Core.
use super::*;
use crate::bool_expression::{BoolExpr, TextOperand};
use crate::condition::{IntComparison, IntCondition};
use crate::expression::{MAX_EXPRESSION_BYTES, MAX_EXPRESSION_DEPTH, MAX_EXPRESSION_NODES};
use crate::float_expression::FloatCondition;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilBooleanOperation {
    Compute { key: String, expression: BoolExpr },
    Invalid(String),
}
fn error(message: impl Display) -> String {
    format!("AIL-BOOL-001: {message}")
}
fn field(key: &str) -> Result<(), String> {
    if crate::float_expression::field_key(key) {
        Ok(())
    } else {
        Err(error("expected a dotted Boolean operand field"))
    }
}
impl AilBooleanOperation {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Compute { key, expression } => format!(
                "the system computes boolean {key} as {}",
                expression.to_source()
            ),
            Self::Invalid(message) => format!("invalid Boolean operation: {message}"),
        }
    }
}
pub(super) fn parse_bullet(text: &str) -> Option<Result<AilBooleanOperation, String>> {
    let text = trim_sentence(text);
    let Some(rest) = text.strip_prefix("the system computes boolean ") else {
        return text
            .eq("the system computes boolean")
            .then(|| Err(error("malformed Boolean computation")));
    };
    Some((|| {
        let (key, expression) = rest
            .split_once(" as ")
            .ok_or_else(|| error("Boolean compute requires 'key as expression'"))?;
        field(key)?;
        Ok(AilBooleanOperation::Compute {
            key: key.into(),
            expression: BoolExpr::parse(expression)?,
        })
    })())
}
pub(super) fn instruction(
    op: &AilBooleanOperation,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let AilBooleanOperation::Compute { key, expression } = op else {
        return Err(error("invalid Boolean computation"));
    };
    if runtime_field_type_name(document, key) != Some("Bool") {
        return Err(error("Boolean destination must be declared Bool"));
    }
    expression.validate()?;
    for (key, kind) in expression.typed_fields()? {
        if runtime_field_type_name(document, &key) != Some(kind.as_str()) {
            return Err(error(format!("{key} must be a declared {kind} field")));
        }
    }
    let instruction = AilBytecodeInstruction::new(
        "EVAL_BOOL",
        &[
            ("key", key.clone()),
            ("expression", expression.to_postfix()),
        ],
    );
    verify(&instruction)?;
    Ok(instruction)
}
pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    let names = match instruction.opcode.as_str() {
        "EVAL_BOOL" => ["expression", "key"],
        "BRANCH_BOOL" => ["expression", "label"],
        _ => return Err(error("unknown Boolean opcode")),
    };
    if instruction.operands.keys().map(String::as_str).ne(names) {
        return Err(error("Boolean opcode requires exact operands"));
    }
    BoolExpr::from_postfix(&instruction.operands["expression"])?;
    if instruction.opcode == "EVAL_BOOL" {
        field(&instruction.operands["key"])?;
    } else if instruction.operands["label"].is_empty() {
        return Err(error("Boolean branch requires a label"));
    }
    Ok(())
}
pub(super) fn execute(
    instruction: &AilBytecodeInstruction,
    state: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    verify(instruction)?;
    if instruction.opcode != "EVAL_BOOL" {
        return Err(error("Boolean branches execute through control flow"));
    }
    let result = BoolExpr::from_postfix(&instruction.operands["expression"])?.eval(state)?;
    state.insert(instruction.operands["key"].clone(), result.to_string());
    Ok(())
}
pub(super) fn namespace(op: &AilBooleanOperation, alias: &str) -> AilBooleanOperation {
    let prefix = runtime_subject_key(alias);
    match op {
        AilBooleanOperation::Compute { key, expression } => AilBooleanOperation::Compute {
            key: format!("{prefix}.{key}"),
            expression: expression.map_fields(|key| format!("{prefix}.{key}")),
        },
        AilBooleanOperation::Invalid(message) => AilBooleanOperation::Invalid(message.clone()),
    }
}
fn node(
    graph: &mut Graph,
    kind: &str,
    name: &str,
    type_name: Option<String>,
    attributes: BTreeMap<String, String>,
) -> Node {
    let node = graph.add_node(kind, name, type_name, attributes);
    attach_provenance(graph, &node, format!("boolean:{name}"));
    node
}
fn target(graph: &mut Graph, owner: &Node, key: &str, local: bool) -> Node {
    graph
        .nodes
        .iter()
        .find(|n| {
            n.kind == "Field"
                && expression_core::field_key(n) == key
                && if local {
                    n.attributes.get("function_owner") == Some(&owner.name)
                } else {
                    !n.attributes.contains_key("function_owner")
                        && !n.attributes.contains_key("local_name")
                }
        })
        .cloned()
        .unwrap_or_else(|| node(graph, "Effect", key, None, BTreeMap::new()))
}
fn build_text(
    graph: &mut Graph,
    owner: &Node,
    name: &str,
    operand: &TextOperand,
    local: bool,
) -> Node {
    let mut attributes = match operand {
        TextOperand::Literal(value) => attr(&[("operand_kind", "literal"), ("value", value)]),
        TextOperand::Field(_) => attr(&[("operand_kind", "field")]),
    };
    if local {
        attributes.insert("function_owner".into(), owner.name.clone());
    }
    let root = node(graph, "TextOperand", name, Some("Text".into()), attributes);
    if let TextOperand::Field(key) = operand {
        let field = target(graph, owner, key, local);
        graph.add_edge("text_field", &root, &field, BTreeMap::new());
    }
    root
}
pub(super) fn build_expression(
    graph: &mut Graph,
    owner: &Node,
    name: &str,
    expression: &BoolExpr,
    local: bool,
) -> Node {
    let (kind, comparison) = match expression {
        BoolExpr::Literal(_) => ("literal", None),
        BoolExpr::Field(_) => ("field", None),
        BoolExpr::Not(_) => ("not", None),
        BoolExpr::And { .. } => ("and", None),
        BoolExpr::Or { .. } => ("or", None),
        BoolExpr::CompareInt(c) => ("integer", Some(c.comparison)),
        BoolExpr::CompareFloat(c) => ("floating", Some(c.comparison)),
        BoolExpr::CompareText { comparison, .. } => ("text", Some(*comparison)),
        BoolExpr::CompareBool { comparison, .. } => ("boolean", Some(*comparison)),
    };
    let mut attributes = attr(&[("expression_kind", kind)]);
    if local {
        attributes.insert("function_owner".into(), owner.name.clone());
    }
    if let Some(comparison) = comparison {
        attributes.insert("comparison".into(), comparison.token().into());
    }
    if let BoolExpr::Literal(value) = expression {
        attributes.insert("value".into(), value.to_string());
    }
    let root = node(
        graph,
        "BooleanExpression",
        name,
        Some("Bool".into()),
        attributes,
    );
    match expression {
        BoolExpr::Literal(_) => {}
        BoolExpr::Field(key) => {
            let field = target(graph, owner, key, local);
            graph.add_edge("boolean_field", &root, &field, BTreeMap::new());
        }
        BoolExpr::Not(operand) => {
            let operand =
                build_expression(graph, owner, &format!("{name}.operand"), operand, local);
            graph.add_edge("boolean_operand", &root, &operand, BTreeMap::new());
        }
        _ => {
            let left_name = format!("{name}.left");
            let right_name = format!("{name}.right");
            let (left, right) = match expression {
                BoolExpr::And { left, right }
                | BoolExpr::Or { left, right }
                | BoolExpr::CompareBool { left, right, .. } => (
                    build_expression(graph, owner, &left_name, left, local),
                    build_expression(graph, owner, &right_name, right, local),
                ),
                BoolExpr::CompareInt(c) => (
                    integer_function::build_expression(graph, owner, &left_name, &c.left, local),
                    integer_function::build_expression(graph, owner, &right_name, &c.right, local),
                ),
                BoolExpr::CompareFloat(c) => (
                    float_operations::build_expression(graph, owner, &left_name, &c.left, local),
                    float_operations::build_expression(graph, owner, &right_name, &c.right, local),
                ),
                BoolExpr::CompareText { left, right, .. } => (
                    build_text(graph, owner, &left_name, left, local),
                    build_text(graph, owner, &right_name, right, local),
                ),
                _ => unreachable!(),
            };
            graph.add_edge("boolean_left", &root, &left, BTreeMap::new());
            graph.add_edge("boolean_right", &root, &right, BTreeMap::new());
        }
    }
    root
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, op) in action.boolean_operations.iter().enumerate() {
            let name = format!("{}.boolean{index}", owner.name);
            let root = node(
                graph,
                "BooleanOperation",
                &name,
                None,
                attr(&[("boolean_operation_kind", "compute")]),
            );
            if let AilBooleanOperation::Compute { key, expression } = op {
                let field = target(graph, &owner, key, false);
                let expression = build_expression(
                    graph,
                    &owner,
                    &format!("{name}.expression"),
                    expression,
                    false,
                );
                graph.add_edge("boolean_destination", &root, &field, BTreeMap::new());
                graph.add_edge("boolean_expression", &root, &expression, BTreeMap::new());
            }
            action_order::append(graph, "boolean_step", &owner, &root, BTreeMap::new(), index);
        }
    }
}
fn by_id<'a>(core: &'a AilCore, id: &str) -> Result<&'a Node, String> {
    core.graph
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or_else(|| error("missing Boolean graph endpoint"))
}
fn child<'a>(core: &'a AilCore, node: &Node, relation: &str) -> Result<&'a Node, String> {
    let edges: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == relation)
        .collect();
    if edges.len() != 1 || !edges[0].attributes.is_empty() {
        return Err(error("Boolean operand requires one unannotated edge"));
    }
    by_id(core, &edges[0].target)
}
fn attrs(node: &Node, names: &[&str], owner: Option<&Node>) -> Result<(), String> {
    if node.attributes.len() != names.len() + usize::from(owner.is_some())
        || names
            .iter()
            .any(|name| !node.attributes.contains_key(*name))
        || owner.is_some_and(|owner| node.attributes.get("function_owner") != Some(&owner.name))
    {
        return Err(error(
            "Boolean tree attributes must exactly match its type and owner",
        ));
    }
    Ok(())
}
fn edges(core: &AilCore, node: &Node, names: &[&str]) -> Result<(), String> {
    if core.graph.edges.iter().any(|e| {
        e.source == node.id && e.kind != "has_provenance" && !names.contains(&e.kind.as_str())
    }) {
        return Err(error("unexpected Boolean tree edge"));
    }
    for name in names {
        child(core, node, name)?;
    }
    Ok(())
}
fn executable(node: &Node) -> bool {
    node.kind == "Action"
        && node
            .attributes
            .get("kind")
            .is_none_or(|k| k != "CompilerPass")
}
fn action_owner(core: &AilCore, node: &Node, relation: &str) -> Result<(), String> {
    let incoming: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect();
    if incoming.len() != 1
        || incoming[0].kind != relation
        || !executable(by_id(core, &incoming[0].source)?)
        || incoming[0]
            .attributes
            .get("step_index")
            .is_none_or(|v| v.parse::<usize>().is_err())
        || !incoming[0].attributes.iter().all(|(k, v)| {
            matches!(k.as_str(), "step_index" | "operation_index") && v.parse::<usize>().is_ok()
        })
    {
        return Err(error(
            "Boolean operation requires exactly one ordered executable action owner",
        ));
    }
    Ok(())
}
fn parent(core: &AilCore, node: &Node, owner: Option<&Node>) -> Result<(), String> {
    let incoming: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect();
    if incoming.len() != 1 {
        return Err(error("predicate tree node requires exactly one parent"));
    }
    let edge = incoming[0];
    let source = by_id(core, &edge.source)?;
    if let Some(owner) = owner.filter(|_| source.kind == "ValueFunctionStep") {
        let declarations: Vec<_> = core
            .graph
            .edges
            .iter()
            .filter(|e| e.target == source.id)
            .collect();
        if declarations.len() != 1
            || declarations[0].kind != "value_function_step"
            || declarations[0].source != owner.id
            || declarations[0].attributes.len() != 1
            || declarations[0]
                .attributes
                .get("step_index")
                .is_none_or(|v| v.parse::<usize>().is_err())
        {
            return Err(error(
                "predicate step requires exactly one owning function declaration",
            ));
        }
    }
    let boolean = node.kind == "BooleanExpression";
    let valid = match edge.kind.as_str() {
        "boolean_left" | "boolean_right" => {
            source.kind == "BooleanExpression"
                && source.attributes.get("function_owner") == owner.map(|o| &o.name)
                && edge.attributes.is_empty()
                && match source.attributes.get("expression_kind").map(String::as_str) {
                    Some("and" | "or" | "boolean") => boolean,
                    Some("integer") => node.kind == "IntegerExpression",
                    Some("floating") => node.kind == "FloatExpression",
                    Some("text") => node.kind == "TextOperand",
                    _ => false,
                }
        }
        "boolean_operand" => {
            boolean
                && source.kind == "BooleanExpression"
                && source.attributes.get("expression_kind").map(String::as_str) == Some("not")
                && source.attributes.get("function_owner") == owner.map(|o| &o.name)
                && edge.attributes.is_empty()
        }
        "left_operand" | "right_operand" | "unary_operand" => {
            matches!(node.kind.as_str(), "IntegerExpression" | "FloatExpression")
                && source.kind == node.kind
                && source.attributes.get("function_owner") == owner.map(|o| &o.name)
                && edge.attributes.is_empty()
        }
        "boolean_expression" => {
            boolean
                && owner.is_none()
                && source.kind == "BooleanOperation"
                && edge.attributes.is_empty()
        }
        "condition" => {
            if !boolean
                || owner.is_some()
                || source.kind != "Control"
                || !edge.attributes.is_empty()
            {
                false
            } else {
                attrs(source, &["control_kind"], None)?;
                edges(core, source, &["condition"])?;
                action_owner(core, source, "control_step")?;
                source.type_name.is_none()
                    && matches!(
                        source.attributes["control_kind"].as_str(),
                        "if_boolean" | "while_boolean"
                    )
            }
        }
        "value_condition" => {
            boolean
                && owner.is_some_and(|o| {
                    source.kind == "ValueFunctionStep"
                        && source.attributes.get("function_owner") == Some(&o.name)
                })
                && matches!(
                    source.attributes.get("control_kind").map(String::as_str),
                    Some("if_boolean" | "while_boolean")
                )
                && edge.attributes.is_empty()
        }
        "value_expression_operand" => {
            boolean
                && owner.is_some_and(|o| {
                    source.kind == "ValueFunctionStep"
                        && source.attributes.get("function_owner") == Some(&o.name)
                })
                && source.attributes.get("opcode").map(String::as_str) == Some("EVAL_BOOL")
                && edge.attributes.len() == 1
                && edge.attributes.get("name").map(String::as_str) == Some("expression")
        }
        _ => false,
    };
    if !valid {
        return Err(error("invalid predicate tree parent"));
    }
    Ok(())
}
#[derive(Default)]
struct Budget<'a> {
    raw_text_bytes: Option<&'a BTreeMap<String, usize>>,
    nodes: usize,
    bytes: usize,
}
impl Budget<'_> {
    fn node(&mut self, depth: usize, bytes: usize) -> Result<(), String> {
        self.nodes += 1;
        self.bytes = self.bytes.saturating_add(bytes);
        if depth > MAX_EXPRESSION_DEPTH
            || self.nodes > MAX_EXPRESSION_NODES
            || self.bytes > MAX_EXPRESSION_BYTES + 1
        {
            return Err(error(
                "predicate graph exceeds aggregate byte, node or depth bounds",
            ));
        }
        Ok(())
    }
}
fn numeric_tree(
    core: &AilCore,
    root: &Node,
    owner: Option<&Node>,
    depth: usize,
    budget: &mut Budget<'_>,
    float: bool,
) -> Result<(), String> {
    parent(core, root, owner)?;
    if root.kind
        != if float {
            "FloatExpression"
        } else {
            "IntegerExpression"
        }
        || root.type_name.as_deref() != Some(if float { "Float" } else { "Int" })
    {
        return Err(error("incorrect numeric predicate operand type"));
    }
    let kind = root
        .attributes
        .get("expression_kind")
        .map(String::as_str)
        .unwrap_or("");
    match kind {
        "literal" => {
            attrs(root, &["expression_kind", "value"], owner)?;
            edges(core, root, &[])?;
            budget.node(depth, root.attributes["value"].len().saturating_add(3))?;
        }
        "field" => {
            attrs(root, &["expression_kind"], owner)?;
            edges(core, root, &["expression_field"])?;
            let key = float_operations::scoped_field(
                core,
                child(core, root, "expression_field")?,
                owner,
                if float { "Float" } else { "Int" },
            )
            .map_err(error)?;
            budget.node(depth, key.len().saturating_add(4))?;
        }
        "binary" => {
            attrs(root, &["expression_kind", "operator"], owner)?;
            edges(core, root, &["left_operand", "right_operand"])?;
            budget.node(depth, root.attributes["operator"].len().saturating_add(2))?;
            numeric_tree(
                core,
                child(core, root, "left_operand")?,
                owner,
                depth + 1,
                budget,
                float,
            )?;
            numeric_tree(
                core,
                child(core, root, "right_operand")?,
                owner,
                depth + 1,
                budget,
                float,
            )?;
        }
        "negate" if float => {
            attrs(root, &["expression_kind"], owner)?;
            edges(core, root, &["unary_operand"])?;
            budget.node(depth, 5)?;
            numeric_tree(
                core,
                child(core, root, "unary_operand")?,
                owner,
                depth + 1,
                budget,
                true,
            )?;
        }
        _ => return Err(error("unknown numeric predicate operand")),
    }
    Ok(())
}
fn text_tree(
    core: &AilCore,
    root: &Node,
    owner: Option<&Node>,
    depth: usize,
    budget: &mut Budget<'_>,
) -> Result<TextOperand, String> {
    parent(core, root, owner)?;
    if root.kind != "TextOperand" || root.type_name.as_deref() != Some("Text") {
        return Err(error("expected typed TextOperand"));
    }
    match root.attributes.get("operand_kind").map(String::as_str) {
        Some("literal") => {
            attrs(root, &["operand_kind", "value"], owner)?;
            edges(core, root, &[])?;
            let value = &root.attributes["value"];
            let encoded = 2 + value
                .chars()
                .map(|c| match c {
                    '"' | '\\' | '\n' | '\r' | '\t' => 2,
                    c if c.is_control() => 6,
                    c => c.len_utf8(),
                })
                .sum::<usize>();
            let encoded = budget
                .raw_text_bytes
                .and_then(|sizes| sizes.get(&root.id))
                .copied()
                .unwrap_or(0)
                .max(encoded);
            budget.node(depth, encoded.saturating_add(3))?;
            Ok(TextOperand::Literal(value.clone()))
        }
        Some("field") => {
            attrs(root, &["operand_kind"], owner)?;
            edges(core, root, &["text_field"])?;
            let key = float_operations::scoped_field(
                core,
                child(core, root, "text_field")?,
                owner,
                "Text",
            )
            .map_err(error)?;
            budget.node(depth, key.len().saturating_add(4))?;
            Ok(TextOperand::Field(key))
        }
        _ => Err(error("invalid TextOperand kind")),
    }
}
fn decode_tree(
    core: &AilCore,
    root: &Node,
    owner: Option<&Node>,
    depth: usize,
    budget: &mut Budget<'_>,
) -> Result<BoolExpr, String> {
    parent(core, root, owner)?;
    if root.kind != "BooleanExpression" || root.type_name.as_deref() != Some("Bool") {
        return Err(error("expected typed BooleanExpression"));
    }
    let kind = root
        .attributes
        .get("expression_kind")
        .map(String::as_str)
        .unwrap_or("");
    match kind {
        "literal" => {
            attrs(root, &["expression_kind", "value"], owner)?;
            edges(core, root, &[])?;
            budget.node(depth, root.attributes["value"].len().saturating_add(3))?;
            Ok(BoolExpr::Literal(match root.attributes["value"].as_str() {
                "true" => true,
                "false" => false,
                _ => return Err(error("Boolean literal must be true or false")),
            }))
        }
        "field" => {
            attrs(root, &["expression_kind"], owner)?;
            edges(core, root, &["boolean_field"])?;
            let key = float_operations::scoped_field(
                core,
                child(core, root, "boolean_field")?,
                owner,
                "Bool",
            )
            .map_err(error)?;
            budget.node(depth, key.len().saturating_add(4))?;
            Ok(BoolExpr::Field(key))
        }
        "not" => {
            attrs(root, &["expression_kind"], owner)?;
            edges(core, root, &["boolean_operand"])?;
            budget.node(depth, 4)?;
            Ok(BoolExpr::Not(Box::new(decode_tree(
                core,
                child(core, root, "boolean_operand")?,
                owner,
                depth + 1,
                budget,
            )?)))
        }
        "and" | "or" => {
            attrs(root, &["expression_kind"], owner)?;
            edges(core, root, &["boolean_left", "boolean_right"])?;
            budget.node(depth, kind.len() + 1)?;
            let left = Box::new(decode_tree(
                core,
                child(core, root, "boolean_left")?,
                owner,
                depth + 1,
                budget,
            )?);
            let right = Box::new(decode_tree(
                core,
                child(core, root, "boolean_right")?,
                owner,
                depth + 1,
                budget,
            )?);
            Ok(if kind == "and" {
                BoolExpr::And { left, right }
            } else {
                BoolExpr::Or { left, right }
            })
        }
        "integer" | "floating" | "text" | "boolean" => {
            attrs(root, &["expression_kind", "comparison"], owner)?;
            edges(core, root, &["boolean_left", "boolean_right"])?;
            let comparison =
                IntComparison::from_token(&root.attributes["comparison"]).map_err(error)?;
            budget.node(depth, comparison.token().len() + 6)?;
            let left = child(core, root, "boolean_left")?;
            let right = child(core, root, "boolean_right")?;
            Ok(match kind {
                "integer" => {
                    numeric_tree(core, left, owner, depth + 1, budget, false)?;
                    numeric_tree(core, right, owner, depth + 1, budget, false)?;
                    BoolExpr::CompareInt(IntCondition {
                        left: value_function::scoped_expression(core, left, owner)
                            .map_err(error)?,
                        comparison,
                        right: value_function::scoped_expression(core, right, owner)
                            .map_err(error)?,
                    })
                }
                "floating" => {
                    numeric_tree(core, left, owner, depth + 1, budget, true)?;
                    numeric_tree(core, right, owner, depth + 1, budget, true)?;
                    BoolExpr::CompareFloat(FloatCondition {
                        left: float_operations::decode_expression(core, left, owner)
                            .map_err(error)?,
                        comparison,
                        right: float_operations::decode_expression(core, right, owner)
                            .map_err(error)?,
                    })
                }
                "text" => BoolExpr::CompareText {
                    left: text_tree(core, left, owner, depth + 1, budget)?,
                    comparison,
                    right: text_tree(core, right, owner, depth + 1, budget)?,
                },
                _ => BoolExpr::CompareBool {
                    left: Box::new(decode_tree(core, left, owner, depth + 1, budget)?),
                    comparison,
                    right: Box::new(decode_tree(core, right, owner, depth + 1, budget)?),
                },
            })
        }
        _ => Err(error("invalid Boolean expression kind")),
    }
}
/// Check saved-Core spelling sizes before discarded JSON escapes can shrink
/// the predicate budget. Semantic graph validation remains in the Core checker.
pub(super) fn check_raw_text_transport(
    core: &AilCore,
    raw_text_bytes: &BTreeMap<String, usize>,
) -> Result<(), String> {
    if raw_text_bytes.is_empty() {
        return Ok(());
    }
    for root in core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind == "BooleanExpression")
    {
        let owner = root
            .attributes
            .get("function_owner")
            .and_then(|name| core.graph.find_node("Function", name));
        let mut budget = Budget {
            raw_text_bytes: Some(raw_text_bytes),
            ..Default::default()
        };
        let _ = decode_tree(core, root, owner, 1, &mut budget);
        if budget.bytes > MAX_EXPRESSION_BYTES + 1 {
            return Err(error(
                "predicate graph exceeds aggregate raw transport byte bound",
            ));
        }
    }
    Ok(())
}
pub(super) fn decode_expression(
    core: &AilCore,
    root: &Node,
    owner: Option<&Node>,
) -> Result<BoolExpr, String> {
    if owner.is_some_and(|o| {
        o.kind != "Function"
            || o.attributes.get("executable_value").map(String::as_str) != Some("true")
    }) {
        return Err(error(
            "predicate local owner must be an executable value function",
        ));
    }
    let expression = decode_tree(core, root, owner, 1, &mut Budget::default())?;
    expression.validate()?;
    Ok(expression)
}
fn decode(core: &AilCore, root: &Node) -> Result<AilBooleanOperation, String> {
    if root.kind != "BooleanOperation" || root.type_name.is_some() {
        return Err(error("expected untyped BooleanOperation"));
    }
    attrs(root, &["boolean_operation_kind"], None)?;
    if root.attributes["boolean_operation_kind"] != "compute" {
        return Err(error("invalid Boolean operation kind"));
    }
    edges(core, root, &["boolean_destination", "boolean_expression"])?;
    action_owner(core, root, "boolean_step")?;
    Ok(AilBooleanOperation::Compute {
        key: float_operations::scoped_field(
            core,
            child(core, root, "boolean_destination")?,
            None,
            "Bool",
        )
        .map_err(error)?,
        expression: decode_expression(core, child(core, root, "boolean_expression")?, None)?,
    })
}
pub(super) fn from_core(core: &AilCore, action: &Node) -> Vec<AilBooleanOperation> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == action.id && e.kind == "boolean_step")
        .map(|e| {
            by_id(core, &e.target)
                .and_then(|n| decode(core, n))
                .unwrap_or_else(AilBooleanOperation::Invalid)
        })
        .collect()
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut reachable = BTreeSet::new();
    for node in core.graph.nodes.iter().filter(|n| {
        matches!(
            n.kind.as_str(),
            "BooleanOperation" | "BooleanExpression" | "TextOperand"
        )
    }) {
        let result = (|| {
            if !matches!(core.package.profile.as_str(), "Application" | "C interop") {
                return Err(error(
                    "Boolean execution requires Application or C interop profile",
                ));
            }
            if node.kind == "BooleanOperation" {
                return decode(core, node).map(|_| ());
            }
            let owner = node
                .attributes
                .get("function_owner")
                .map(|name| {
                    core.graph
                        .find_node("Function", name)
                        .ok_or_else(|| error("missing predicate function owner"))
                })
                .transpose()?;
            parent(core, node, owner)?;
            let edge = core
                .graph
                .edges
                .iter()
                .find(|e| e.target == node.id)
                .unwrap();
            if by_id(core, &edge.source)?.kind != "BooleanExpression" {
                decode_expression(core, node, owner)?;
                let mut pending = vec![node.id.as_str()];
                while let Some(id) = pending.pop() {
                    if !reachable.insert(id.to_string()) {
                        continue;
                    }
                    for edge in core.graph.edges.iter().filter(|e| {
                        e.source == id
                            && matches!(
                                e.kind.as_str(),
                                "boolean_left" | "boolean_right" | "boolean_operand"
                            )
                    }) {
                        pending.push(edge.target.as_str());
                    }
                }
            }
            Ok(())
        })();
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-BOOL-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for node in core.graph.nodes.iter().filter(|n| {
        matches!(n.kind.as_str(), "BooleanExpression" | "TextOperand") && !reachable.contains(&n.id)
    }) {
        diagnostics.push(
            AilDiagnostic::error("AIL-BOOL-001", "predicate tree is orphaned or cyclic")
                .with_affected_graph_item(format!("node:{}", node.id)),
        );
    }
    for edge in &core.graph.edges {
        let source = by_id(core, &edge.source).ok();
        let target = by_id(core, &edge.target).ok();
        let valid = match edge.kind.as_str() {
            "boolean_step" => {
                source.is_some_and(executable)
                    && target.is_some_and(|n| n.kind == "BooleanOperation")
                    && edge
                        .attributes
                        .get("step_index")
                        .is_some_and(|v| v.parse::<usize>().is_ok())
                    && edge.attributes.iter().all(|(k, v)| {
                        matches!(k.as_str(), "step_index" | "operation_index")
                            && v.parse::<usize>().is_ok()
                    })
            }
            "boolean_destination" => {
                source.is_some_and(|n| n.kind == "BooleanOperation")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            "boolean_expression" => {
                source.is_some_and(|n| n.kind == "BooleanOperation")
                    && target.is_some_and(|n| n.kind == "BooleanExpression")
                    && edge.attributes.is_empty()
            }
            "boolean_field" => {
                source.is_some_and(|n| n.kind == "BooleanExpression")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            "text_field" => {
                source.is_some_and(|n| n.kind == "TextOperand")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            "boolean_left" | "boolean_right" | "boolean_operand" => {
                source.is_some_and(|n| n.kind == "BooleanExpression")
                    && target.is_some_and(|n| {
                        matches!(
                            n.kind.as_str(),
                            "BooleanExpression"
                                | "TextOperand"
                                | "IntegerExpression"
                                | "FloatExpression"
                        )
                    })
                    && edge.attributes.is_empty()
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-BOOL-001",
                    format!("invalid {} Boolean edge", edge.kind),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let mut result = Vec::new();
    for step in core
        .graph
        .edges
        .iter()
        .filter(|e| matches!(e.kind.as_str(), "boolean_step" | "control_step"))
    {
        let mut pending = vec![step.target.as_str()];
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            for edge in core.graph.edges.iter().filter(|e| e.source == id) {
                if kind == "writes" && edge.kind == "boolean_destination" {
                    result.push((edge, step.source.as_str()));
                }
                if kind != "reads" {
                    continue;
                }
                match edge.kind.as_str() {
                    "boolean_field" | "text_field" | "expression_field" => {
                        result.push((edge, step.source.as_str()))
                    }
                    "boolean_expression" | "condition" | "boolean_left" | "boolean_right"
                    | "boolean_operand" | "left_operand" | "right_operand" | "unary_operand" => {
                        pending.push(edge.target.as_str())
                    }
                    _ => {}
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(expression: &str) -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = format!(
            "The application Predicates manages records.\n\nA Data has:\n- enabled: Bool\n- count: Int\n- score: Float\n- kind: Text\n\nAn Output has:\n- selected: Bool\n\nAction: Run.\nWhen Run happens:\n- the system computes boolean output.selected as {expression}\n"
        );
        elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, &source).unwrap(),
        )
    }
    fn mixed() -> AilCore {
        fixture(
            "data.enabled and integer data.count > 0 and floating -data.score <= 1.5 or text data.kind == \"and or ()\\u0000\"",
        )
    }
    #[test]
    fn source_protocol_and_lazy_atomic_execution_preserve_quoted_contents() {
        let source = "the system computes boolean output.selected as false and integer data.missing / 0 > 0 or text data.kind == \"and or ()\\u0000\"";
        let op = parse_bullet(source).unwrap().unwrap();
        assert_eq!(parse_bullet(&op.source()).unwrap().unwrap(), op);
        let AilBooleanOperation::Compute { expression, .. } = op else {
            unreachable!()
        };
        let code = AilBytecodeInstruction::new(
            "EVAL_BOOL",
            &[
                ("key", "output.selected".into()),
                ("expression", expression.to_postfix()),
            ],
        );
        let mut state = BTreeMap::from([("data.kind".into(), "and or ()\0".into())]);
        execute(&code, &mut state).unwrap();
        assert_eq!(state["output.selected"], "true");
        state.remove("data.kind");
        assert!(
            execute(&code, &mut state)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT:")
        );
        assert_eq!(state["output.selected"], "true");
        let mut extra = code;
        extra.operands.insert("ignored".into(), "false".into());
        assert!(verify(&extra).is_err());
        assert!(
            parse_bullet("the system computes boolean output.selected")
                .unwrap()
                .is_err()
        );
        let original = parse_bullet(
            "the system computes boolean output.selected as text data.kind == \"data.kind\"",
        )
        .unwrap()
        .unwrap();
        let renamed = namespace(&original, "Child").source();
        assert!(renamed.contains("child.data.kind"));
        assert!(renamed.contains("\"data.kind\""));
    }
    #[test]
    fn core_roundtrip_rejects_duplicates_stale_metadata_shared_trees_and_wrong_types() {
        let original = mixed();
        assert!(check(&original).is_empty(), "{:?}", check(&original));
        assert!(
            float_operations::check(&original).is_empty(),
            "{:?}",
            float_operations::check(&original)
        );
        let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        assert!(check(&saved).is_empty(), "{:?}", check(&saved));
        assert_eq!(
            from_core(
                &original,
                original.graph.find_node("Action", "Run").unwrap()
            ),
            from_core(&saved, saved.graph.find_node("Action", "Run").unwrap())
        );
        for relation in [
            "boolean_step",
            "boolean_destination",
            "boolean_expression",
            "boolean_left",
            "boolean_right",
            "boolean_field",
            "text_field",
            "expression_field",
        ] {
            let mut candidate = original.clone();
            // This fixture's Text side has one field and one literal.
            let edge = candidate
                .graph
                .edges
                .iter()
                .find(|e| e.kind == relation)
                .unwrap()
                .clone();
            candidate.graph.edges.push(edge);
            assert!(
                !check(&parse_ail_core_text(&render_ail_core(&candidate)).unwrap()).is_empty(),
                "duplicate {relation}"
            );
        }
        for node_kind in [
            "BooleanOperation",
            "BooleanExpression",
            "TextOperand",
            "IntegerExpression",
            "FloatExpression",
        ] {
            let mut candidate = original.clone();
            candidate
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == node_kind)
                .unwrap()
                .attributes
                .insert("hidden".into(), "true".into());
            assert!(!check(&candidate).is_empty(), "metadata {node_kind}");
        }
        let mut wrong = original;
        wrong
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "TextOperand")
            .unwrap()
            .type_name = Some("Bool".into());
        assert!(!check(&wrong).is_empty());
    }
    #[test]
    fn skipped_predicate_operands_remain_secret_checked_and_require_real_global_storage() {
        let original = fixture(
            "false and (data.enabled or integer data.count > 0 or floating -data.score > 0 or text data.kind == \"x\")",
        );
        let mut secret = original.clone();
        secret
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        for node in secret.graph.nodes.iter_mut().filter(|n| n.kind == "Field") {
            node.attributes.insert("secret".into(), "true".into());
        }
        assert_eq!(accesses(&secret, "reads").len(), 4);
        assert!(
            check_secret_read_protection(&secret)
                .iter()
                .any(|d| d.code == "AIL005")
        );
        assert!(
            check_secret_write_protection(&secret)
                .iter()
                .any(|d| d.code == "AIL002")
        );
        for relation in [
            "boolean_destination",
            "boolean_field",
            "text_field",
            "expression_field",
        ] {
            let mut candidate = original.clone();
            let index = candidate
                .graph
                .edges
                .iter()
                .position(|e| e.kind == relation)
                .unwrap();
            let mut forged = by_id(&candidate, &candidate.graph.edges[index].target)
                .unwrap()
                .clone();
            forged.id.push_str("_forged");
            candidate.graph.edges[index].target = forged.id.clone();
            candidate.graph.nodes.push(forged);
            assert!(!check(&candidate).is_empty(), "forged {relation}");
        }
    }
    #[test]
    fn core_enforces_aggregate_raw_numeric_bytes_before_normalization() {
        for predicate in ["integer 1 == 1", "floating 1 == 1"] {
            let mut core = fixture(predicate);
            for node in core.graph.nodes.iter_mut().filter(|n| {
                matches!(n.kind.as_str(), "IntegerExpression" | "FloatExpression")
                    && n.attributes.get("expression_kind").map(String::as_str) == Some("literal")
            }) {
                node.attributes
                    .insert("value".into(), format!("{}1", "0".repeat(33_000)));
            }
            assert!(
                !check(&core).is_empty(),
                "accepted raw aggregate {predicate}"
            );
        }
        let mut core = fixture("text \"x\" == \"y\"");
        for node in core
            .graph
            .nodes
            .iter_mut()
            .filter(|n| n.kind == "TextOperand")
        {
            node.attributes
                .insert("value".into(), "\u{85}".repeat(5_500));
        }
        assert!(!check(&core).is_empty());
    }
    #[test]
    fn core_rejects_orphan_cycles_profiles_and_foreign_function_owners() {
        let original = mixed();
        for profile in ["Compiler", "System", "UI", "AgentTool"] {
            let mut core = original.clone();
            core.package.profile = profile.into();
            assert!(!check(&core).is_empty());
        }
        let mut compiler = original.clone();
        compiler
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Action")
            .unwrap()
            .attributes
            .insert("kind".into(), "CompilerPass".into());
        assert!(!check(&compiler).is_empty());
        let mut orphan = original.clone();
        orphan
            .graph
            .edges
            .retain(|e| e.kind != "boolean_expression");
        assert!(!check(&orphan).is_empty());
        let mut foreign = original.clone();
        foreign
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "BooleanExpression")
            .unwrap()
            .attributes
            .insert("function_owner".into(), "Missing".into());
        assert!(!check(&foreign).is_empty());
        let mut cyclic = fixture("not true");
        let root_id = cyclic
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "BooleanExpression" && n.attributes["expression_kind"] == "not")
            .unwrap()
            .id
            .clone();
        cyclic
            .graph
            .edges
            .retain(|e| e.kind != "boolean_expression");
        cyclic
            .graph
            .edges
            .iter_mut()
            .find(|e| e.kind == "boolean_operand")
            .unwrap()
            .target = root_id;
        assert!(!check(&cyclic).is_empty());
    }

    #[test]
    fn function_predicates_own_all_local_types_and_boolean_controls() {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Predicates manages records.\n\nFunction: Select.\nThe function needs:\n- flag: Bool\n- count: Int\n- score: Float\n- text: Text\nThe function produces:\n- result: Bool\nThe function stores:\n- result: Bool\nThe function executes:\n- if boolean local.flag:\n- the function computes boolean local.result as integer local.count > 0 and floating -local.score < 2 and text local.text != \"\"\n- otherwise:\n- the function returns local.flag\n- end if\n- the function returns local.result\n";
        let document = parse_ail_package_spec_text(&package, source).unwrap();
        let original = elaborate_ail_core(&package, &document);
        assert!(check(&original).is_empty(), "{:?}", check(&original));
        assert!(
            float_operations::check(&original).is_empty(),
            "{:?}",
            float_operations::check(&original)
        );
        assert!(
            value_function::check(&original).is_empty(),
            "{:?}",
            value_function::check(&original)
        );
        for kind in [
            "BooleanExpression",
            "IntegerExpression",
            "FloatExpression",
            "TextOperand",
        ] {
            let mut foreign = original.clone();
            foreign
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == kind)
                .unwrap()
                .attributes
                .insert("function_owner".into(), "Other".into());
            assert!(!check(&foreign).is_empty(), "foreign {kind}");
        }
        let mut orphan = original;
        orphan
            .graph
            .edges
            .retain(|e| e.kind != "value_function_step");
        assert!(!check(&orphan).is_empty());
    }

    #[test]
    fn embedded_numeric_trees_share_boolean_node_and_depth_budgets() {
        let mut core = fixture("true");
        let owner = core.graph.find_node("Action", "Run").unwrap().clone();
        let operation = core
            .graph
            .find_node("BooleanOperation", "Run.boolean0")
            .unwrap()
            .clone();
        let edge = core
            .graph
            .edges
            .iter_mut()
            .find(|e| e.kind == "boolean_expression")
            .unwrap();
        let old = edge.target.clone();
        core.graph
            .edges
            .retain(|e| e.kind != "boolean_expression" && e.source != old);
        core.graph.nodes.retain(|n| n.id != old);
        let mut expression = BoolExpr::CompareInt(
            IntCondition::parse("1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 == 1").unwrap(),
        );
        for _ in 0..120 {
            expression = BoolExpr::Not(Box::new(expression));
        }
        let root = build_expression(&mut core.graph, &owner, "deep", &expression, false);
        core.graph
            .add_edge("boolean_expression", &operation, &root, BTreeMap::new());
        assert!(!check(&core).is_empty());
        // Both balanced numeric subtrees individually fit, but together exceed 1024 nodes.
        fn balanced(leaves: usize) -> crate::expression::IntExpr {
            use crate::expression::{IntExpr, IntOperator};
            if leaves == 1 {
                return IntExpr::Literal(1);
            }
            IntExpr::Binary {
                op: IntOperator::Add,
                left: Box::new(balanced(leaves / 2)),
                right: Box::new(balanced(leaves - leaves / 2)),
            }
        }
        let expression = BoolExpr::CompareInt(IntCondition {
            left: balanced(300),
            comparison: IntComparison::Equal,
            right: balanced(300),
        });
        let mut core = fixture("true");
        let old = core
            .graph
            .edges
            .iter()
            .find(|e| e.kind == "boolean_expression")
            .unwrap()
            .target
            .clone();
        core.graph
            .edges
            .retain(|e| e.kind != "boolean_expression" && e.source != old);
        core.graph.nodes.retain(|n| n.id != old);
        let root = build_expression(&mut core.graph, &owner, "wide", &expression, false);
        core.graph
            .add_edge("boolean_expression", &operation, &root, BTreeMap::new());
        assert!(!check(&core).is_empty());
    }
}
