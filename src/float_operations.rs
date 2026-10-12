//! Typed floating computation, explicit conversions, and owned Core expressions.
use super::*;
use crate::expression::{MAX_EXPRESSION_BYTES, MAX_EXPRESSION_DEPTH, MAX_EXPRESSION_NODES};
use crate::float_expression::{FloatCondition, FloatExpr, FloatOperator};
use crate::float_number;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilFloatOperation {
    Compute { key: String, expression: FloatExpr },
    IntToFloat { source: String, key: String },
    FloatToInt { source: String, key: String },
    Invalid(String),
}
fn error(message: impl Display) -> String {
    format!("AIL-FLOAT-001: {message}")
}
fn field(key: &str) -> Result<(), String> {
    if crate::float_expression::field_key(key) {
        Ok(())
    } else {
        Err(error(
            "floating operation requires a dotted field reference",
        ))
    }
}
impl AilFloatOperation {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Compute { key, expression } => format!(
                "the system computes floating {key} as {}",
                expression.to_source()
            ),
            Self::IntToFloat { source, key } => {
                format!("the system converts integer {source} to floating {key}")
            }
            Self::FloatToInt { source, key } => {
                format!("the system converts floating {source} to integer {key}")
            }
            Self::Invalid(message) => format!("invalid floating operation: {message}"),
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Self::Compute { .. } => "compute",
            Self::IntToFloat { .. } => "int_to_float",
            Self::FloatToInt { .. } => "float_to_int",
            Self::Invalid(_) => "invalid",
        }
    }
    fn key(&self) -> Result<&str, String> {
        match self {
            Self::Compute { key, .. }
            | Self::IntToFloat { key, .. }
            | Self::FloatToInt { key, .. } => Ok(key),
            Self::Invalid(message) => Err(error(message)),
        }
    }
    fn fields(&self) -> Result<Vec<(&str, &str)>, String> {
        Ok(match self {
            Self::Compute { key, .. } => vec![(key, "Float")],
            Self::IntToFloat { source, key } => vec![(source, "Int"), (key, "Float")],
            Self::FloatToInt { source, key } => vec![(source, "Float"), (key, "Int")],
            Self::Invalid(message) => return Err(error(message)),
        })
    }
}
pub(super) fn parse_bullet(text: &str) -> Option<Result<AilFloatOperation, String>> {
    let text = trim_sentence(text);
    let parsed = if let Some(rest) = text.strip_prefix("the system computes floating ") {
        return Some((|| {
            let (key, expression) = rest
                .split_once(" as ")
                .ok_or_else(|| error("floating compute requires 'key as expression'"))?;
            field(key)?;
            Ok(AilFloatOperation::Compute {
                key: key.into(),
                expression: FloatExpr::parse(expression)?,
            })
        })());
    } else if let Some(rest) = text.strip_prefix("the system converts integer ") {
        rest.split_once(" to floating ")
            .map(|(source, key)| AilFloatOperation::IntToFloat {
                source: source.into(),
                key: key.into(),
            })
    } else if let Some(rest) = text.strip_prefix("the system converts floating ") {
        rest.split_once(" to integer ")
            .map(|(source, key)| AilFloatOperation::FloatToInt {
                source: source.into(),
                key: key.into(),
            })
    } else {
        return (text.starts_with("the system computes floating")
            || text.starts_with("the system converts integer")
            || text.starts_with("the system converts floating"))
        .then(|| Err(error("malformed floating operation")));
    };
    Some(
        parsed
            .ok_or_else(|| error("malformed floating conversion"))
            .and_then(|op| {
                for (key, _) in op.fields()? {
                    field(key)?;
                }
                Ok(op)
            }),
    )
}
fn raw_instruction(op: &AilFloatOperation) -> Result<AilBytecodeInstruction, String> {
    Ok(match op {
        AilFloatOperation::Compute { key, expression } => AilBytecodeInstruction::new(
            "EVAL_FLOAT",
            &[
                ("key", key.clone()),
                ("expression", expression.to_postfix()),
            ],
        ),
        AilFloatOperation::IntToFloat { source, key }
        | AilFloatOperation::FloatToInt { source, key } => AilBytecodeInstruction::new(
            if matches!(op, AilFloatOperation::IntToFloat { .. }) {
                "INT_TO_FLOAT"
            } else {
                "FLOAT_TO_INT"
            },
            &[("source", source.clone()), ("key", key.clone())],
        ),
        AilFloatOperation::Invalid(message) => return Err(error(message)),
    })
}
pub(super) fn instruction(
    op: &AilFloatOperation,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let instruction = raw_instruction(op)?;
    verify(&instruction)?;
    for (key, kind) in op.fields()? {
        if runtime_field_type_name(document, key) != Some(kind) {
            return Err(error(format!("{key} must be a declared {kind} field")));
        }
    }
    if let AilFloatOperation::Compute { expression, .. } = op {
        for key in expression.fields() {
            if runtime_field_type_name(document, &key) != Some("Float") {
                return Err(error("floating expression fields must be declared Float"));
            }
        }
    }
    Ok(instruction)
}
pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    let names: &[&str] = match instruction.opcode.as_str() {
        "EVAL_FLOAT" => &["key", "expression"],
        "INT_TO_FLOAT" | "FLOAT_TO_INT" => &["key", "source"],
        "BRANCH_FLOAT" => &["left", "comparison", "right", "label"],
        _ => return Err(error("unknown floating opcode")),
    };
    if instruction.operands.len() != names.len()
        || names.iter().any(|n| !instruction.operands.contains_key(*n))
    {
        return Err(error(
            "floating instruction requires exactly its defined operands",
        ));
    }
    let o = &instruction.operands;
    match instruction.opcode.as_str() {
        "EVAL_FLOAT" => {
            field(&o["key"])?;
            FloatExpr::from_postfix(&o["expression"])?;
        }
        "INT_TO_FLOAT" | "FLOAT_TO_INT" => {
            field(&o["source"])?;
            field(&o["key"])?;
        }
        "BRANCH_FLOAT" => {
            FloatCondition::from_postfix(&o["left"], &o["comparison"], &o["right"])?;
            if o["label"].is_empty() {
                return Err(error("floating branch requires a label"));
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
pub(super) fn runtime_parse(text: &str) -> Result<u64, String> {
    float_number::parse(text).map_err(|e| e.to_string())
}
pub(super) fn runtime_format(bits: u64) -> Result<String, String> {
    let mut buffer = [0u8; 32];
    let size = float_number::format(bits, &mut buffer).map_err(|e| e.to_string())?;
    Ok(std::str::from_utf8(&buffer[..size])
        .map_err(|_| "AIL-RUNTIME-FLOAT: invalid numeric format")?
        .into())
}
pub(super) fn execute(
    instruction: &AilBytecodeInstruction,
    state: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    verify(instruction)?;
    let o = &instruction.operands;
    let result = match instruction.opcode.as_str() {
        "EVAL_FLOAT" => runtime_format(FloatExpr::from_postfix(&o["expression"])?.eval(state)?)?,
        "INT_TO_FLOAT" => {
            let source = state
                .get(&o["source"])
                .ok_or("AIL-RUNTIME-INT: missing integer field")?;
            runtime_format(float_number::from_integer(
                crate::expression::parse_integer(source)?,
            ))?
        }
        "FLOAT_TO_INT" => {
            let source = state
                .get(&o["source"])
                .ok_or("AIL-RUNTIME-FLOAT: missing floating field")?;
            float_number::to_integer(runtime_parse(source)?)
                .map_err(|e| e.to_string())?
                .to_string()
        }
        _ => return Err(error("floating branch must execute through control flow")),
    };
    state.insert(o["key"].clone(), result);
    Ok(())
}
pub(super) fn namespace(op: &AilFloatOperation, alias: &str) -> AilFloatOperation {
    let prefix = runtime_subject_key(alias);
    let qualify = |key: &str| format!("{prefix}.{key}");
    match op {
        AilFloatOperation::Compute { key, expression } => AilFloatOperation::Compute {
            key: qualify(key),
            expression: expression.map_fields(qualify),
        },
        AilFloatOperation::IntToFloat { source, key } => AilFloatOperation::IntToFloat {
            source: qualify(source),
            key: qualify(key),
        },
        AilFloatOperation::FloatToInt { source, key } => AilFloatOperation::FloatToInt {
            source: qualify(source),
            key: qualify(key),
        },
        AilFloatOperation::Invalid(message) => AilFloatOperation::Invalid(message.clone()),
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
    attach_provenance(graph, &node, format!("floating:{name}"));
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
pub(super) fn build_expression(
    graph: &mut Graph,
    owner: &Node,
    name: &str,
    expression: &FloatExpr,
    local: bool,
) -> Node {
    let mut attributes = match expression {
        FloatExpr::Literal(bits) => attr(&[
            ("expression_kind", "literal"),
            ("value", &crate::float_expression::format_bits(*bits)),
        ]),
        FloatExpr::Field(_) => attr(&[("expression_kind", "field")]),
        FloatExpr::Binary { op, .. } => {
            attr(&[("expression_kind", "binary"), ("operator", op.token())])
        }
        FloatExpr::Negate(_) => attr(&[("expression_kind", "negate")]),
    };
    if local {
        attributes.insert("function_owner".into(), owner.name.clone());
    }
    let root = node(
        graph,
        "FloatExpression",
        name,
        Some("Float".into()),
        attributes,
    );
    match expression {
        FloatExpr::Field(key) => {
            let field = target(graph, owner, key, local);
            graph.add_edge("expression_field", &root, &field, BTreeMap::new());
        }
        FloatExpr::Binary { left, right, .. } => {
            let left = build_expression(graph, owner, &format!("{name}.left"), left, local);
            let right = build_expression(graph, owner, &format!("{name}.right"), right, local);
            graph.add_edge("left_operand", &root, &left, BTreeMap::new());
            graph.add_edge("right_operand", &root, &right, BTreeMap::new());
        }
        FloatExpr::Negate(operand) => {
            let child = build_expression(graph, owner, &format!("{name}.negate"), operand, local);
            graph.add_edge("unary_operand", &root, &child, BTreeMap::new());
        }
        FloatExpr::Literal(_) => {}
    }
    root
}
pub(super) fn build_condition(
    graph: &mut Graph,
    owner: &Node,
    name: &str,
    condition: &FloatCondition,
    local: bool,
) -> Node {
    let mut attrs = attr(&[("comparison", condition.comparison.token())]);
    if local {
        attrs.insert("function_owner".into(), owner.name.clone());
    }
    let comparison = node(graph, "FloatComparison", name, Some("Bool".into()), attrs);
    for (role, expression) in [
        ("comparison_left", &condition.left),
        ("comparison_right", &condition.right),
    ] {
        let root = build_expression(graph, owner, &format!("{name}.{role}"), expression, local);
        graph.add_edge(role, &comparison, &root, BTreeMap::new());
    }
    comparison
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, operation) in action.float_operations.iter().enumerate() {
            let step = node(
                graph,
                "FloatOperation",
                &format!("{}.floating{index}", owner.name),
                None,
                attr(&[("float_operation_kind", operation.kind())]),
            );
            if let Ok(key) = operation.key() {
                let field = target(graph, &owner, key, false);
                graph.add_edge("float_destination", &step, &field, BTreeMap::new());
            }
            match operation {
                AilFloatOperation::Compute { expression, .. } => {
                    let root = build_expression(
                        graph,
                        &owner,
                        &format!("{}.value", step.name),
                        expression,
                        false,
                    );
                    graph.add_edge("float_expression", &step, &root, BTreeMap::new());
                }
                AilFloatOperation::IntToFloat { source, .. }
                | AilFloatOperation::FloatToInt { source, .. } => {
                    let field = target(graph, &owner, source, false);
                    graph.add_edge("float_source", &step, &field, BTreeMap::new());
                }
                AilFloatOperation::Invalid(_) => {}
            }
            action_order::append(graph, "float_step", &owner, &step, BTreeMap::new(), index);
        }
    }
}
fn by_id<'a>(core: &'a AilCore, id: &str) -> Result<&'a Node, String> {
    core.graph
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or_else(|| error("floating graph endpoint is missing"))
}
fn child<'a>(core: &'a AilCore, node: &Node, kind: &str) -> Result<&'a Node, String> {
    let edges: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == kind)
        .collect();
    if edges.len() != 1 || !edges[0].attributes.is_empty() {
        return Err(error(
            "floating operand requires exactly one unannotated edge",
        ));
    }
    by_id(core, &edges[0].target)
}
fn attrs(node: &Node, names: &[&str], owner: Option<&Node>) -> Result<(), String> {
    let expected = names.len() + usize::from(owner.is_some());
    if node.attributes.len() != expected
        || names.iter().any(|n| !node.attributes.contains_key(*n))
        || owner.is_some_and(|o| node.attributes.get("function_owner") != Some(&o.name))
    {
        return Err(error(
            "floating node attributes must exactly match its type and owner",
        ));
    }
    Ok(())
}
fn edges(core: &AilCore, node: &Node, allowed: &[&str]) -> Result<(), String> {
    if core.graph.edges.iter().any(|e| {
        e.source == node.id && e.kind != "has_provenance" && !allowed.contains(&e.kind.as_str())
    }) {
        Err(error("unexpected floating operand edge"))
    } else {
        Ok(())
    }
}
fn owner_valid(owner: Option<&Node>) -> Result<(), String> {
    if owner.is_some_and(|n| {
        n.kind != "Function"
            || n.attributes
                .get("executable_value")
                .is_none_or(|v| v != "true")
    }) {
        Err(error(
            "floating local owner must be an executable value function",
        ))
    } else {
        Ok(())
    }
}
pub(super) fn scoped_field(
    core: &AilCore,
    node: &Node,
    owner: Option<&Node>,
    kind: &str,
) -> Result<String, String> {
    if node.kind != "Field" || node.type_name.as_deref() != Some(kind) {
        return Err(error("floating operand has an incorrect field type"));
    }
    let key = expression_core::field_key(node);
    field(&key)?;
    if let Some(owner) = owner {
        owner_valid(Some(owner))?;
        attrs(node, &["local_name"], Some(owner))?;
        let name = &node.attributes["local_name"];
        if !value_protocol::identifier(name) {
            return Err(error("invalid floating local name"));
        }
        let declarations: Vec<_> = core
            .graph
            .edges
            .iter()
            .filter(|e| {
                e.target == node.id
                    && matches!(
                        e.kind.as_str(),
                        "value_parameter" | "value_local" | "function_parameter" | "function_local"
                    )
            })
            .collect();
        if declarations.len() != 1
            || declarations[0].source != owner.id
            || !matches!(
                declarations[0].kind.as_str(),
                "value_parameter" | "value_local"
            )
            || !declarations[0].attributes.is_empty()
        {
            return Err(error(
                "floating local requires exactly one owning declaration",
            ));
        }
    } else {
        if node.attributes.contains_key("function_owner")
            || node.attributes.contains_key("local_name")
        {
            return Err(error(
                "floating action cannot access function local storage",
            ));
        }
        let declarations: Vec<_> = core
            .graph
            .edges
            .iter()
            .filter(|e| e.kind == "has_field" && e.target == node.id)
            .collect();
        if declarations.len() != 1 || !declarations[0].attributes.is_empty() {
            return Err(error(
                "floating global requires exactly one storage declaration",
            ));
        }
        let thing = by_id(core, &declarations[0].source)?;
        if thing.kind != "Thing"
            || node
                .name
                .strip_prefix(&format!("{}.", thing.name))
                .is_none_or(|n| n.is_empty() || n.contains('.'))
        {
            return Err(error("floating global must belong to its declaring Thing"));
        }
        if core
            .graph
            .nodes
            .iter()
            .filter(|n| {
                n.kind == "Field"
                    && !n.attributes.contains_key("function_owner")
                    && !n.attributes.contains_key("local_name")
                    && expression_core::field_key(n) == key
            })
            .count()
            != 1
        {
            return Err(error(
                "floating global runtime key has competing declarations",
            ));
        }
    }
    Ok(key)
}
fn expression_parent(core: &AilCore, node: &Node, owner: Option<&Node>) -> Result<(), String> {
    let incoming: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect();
    if incoming.len() != 1 {
        return Err(error("floating expression must have exactly one parent"));
    }
    let edge = incoming[0];
    let source = by_id(core, &edge.source)?;
    let valid = match edge.kind.as_str() {
        "left_operand" | "right_operand" | "unary_operand" => {
            source.kind == "FloatExpression" && edge.attributes.is_empty()
        }
        "comparison_left" | "comparison_right" => {
            source.kind == "FloatComparison" && edge.attributes.is_empty()
        }
        "float_expression" => {
            owner.is_none() && source.kind == "FloatOperation" && edge.attributes.is_empty()
        }
        "list_value" => {
            owner.is_none()
                && source.kind == "ListOperation"
                && source.attributes.get("element_type").map(String::as_str) == Some("Float")
                && matches!(
                    source.attributes.get("list_kind").map(String::as_str),
                    Some("append" | "set")
                )
                && edge.attributes.is_empty()
        }
        "boolean_left" | "boolean_right" => {
            source.kind == "BooleanExpression"
                && source.type_name.as_deref() == Some("Bool")
                && source.attributes.get("expression_kind").map(String::as_str) == Some("floating")
                && source.attributes.get("function_owner") == owner.map(|o| &o.name)
                && edge.attributes.is_empty()
        }
        "value_expression_operand" => {
            owner.is_some_and(|o| {
                source.kind == "ValueFunctionStep"
                    && source.attributes.get("function_owner") == Some(&o.name)
            }) && edge.attributes.len() == 1
                && edge.attributes.contains_key("name")
        }
        _ => false,
    };
    if !valid {
        return Err(error("invalid floating expression parent"));
    }
    if (source.kind == "FloatExpression" || source.kind == "FloatComparison")
        && source.attributes.get("function_owner") != owner.map(|n| &n.name)
    {
        return Err(error("floating expression parent has a different owner"));
    }
    Ok(())
}
fn decode_tree(
    core: &AilCore,
    node: &Node,
    owner: Option<&Node>,
    depth: usize,
    count: &mut usize,
    bytes: &mut usize,
) -> Result<FloatExpr, String> {
    *count += 1;
    if *count > MAX_EXPRESSION_NODES || depth > MAX_EXPRESSION_DEPTH {
        return Err(error("floating graph exceeds node or depth bounds"));
    }
    if node.kind != "FloatExpression" || node.type_name.as_deref() != Some("Float") {
        return Err(error("expected a typed FloatExpression"));
    }
    expression_parent(core, node, owner)?;
    let kind = node
        .attributes
        .get("expression_kind")
        .map(String::as_str)
        .unwrap_or("");
    Ok(match kind {
        "literal" => {
            attrs(node, &["expression_kind", "value"], owner)?;
            edges(core, node, &[])?;
            count_bytes(bytes, node.attributes["value"].len().saturating_add(3))?;
            FloatExpr::Literal(float_number::parse(&node.attributes["value"]).map_err(error)?)
        }
        "field" => {
            attrs(node, &["expression_kind"], owner)?;
            edges(core, node, &["expression_field"])?;
            let key = scoped_field(core, child(core, node, "expression_field")?, owner, "Float")?;
            count_bytes(bytes, key.len().saturating_add(3))?;
            FloatExpr::Field(key)
        }
        "binary" => {
            attrs(node, &["expression_kind", "operator"], owner)?;
            edges(core, node, &["left_operand", "right_operand"])?;
            let op = FloatOperator::from_token(&node.attributes["operator"])
                .ok_or_else(|| error("invalid floating binary operator"))?;
            count_bytes(bytes, 2)?;
            let left = decode_tree(
                core,
                child(core, node, "left_operand")?,
                owner,
                depth + 1,
                count,
                bytes,
            )?;
            let right = decode_tree(
                core,
                child(core, node, "right_operand")?,
                owner,
                depth + 1,
                count,
                bytes,
            )?;
            FloatExpr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            }
        }
        "negate" => {
            attrs(node, &["expression_kind"], owner)?;
            edges(core, node, &["unary_operand"])?;
            count_bytes(bytes, 4)?;
            FloatExpr::Negate(Box::new(decode_tree(
                core,
                child(core, node, "unary_operand")?,
                owner,
                depth + 1,
                count,
                bytes,
            )?))
        }
        _ => return Err(error("unknown floating expression kind")),
    })
}
// Count the raw postfix representation before decimal normalization can shrink
// it. Each token includes one trailing space; the complete tree has one spare.
fn count_bytes(bytes: &mut usize, additional: usize) -> Result<(), String> {
    *bytes = bytes.saturating_add(additional);
    if *bytes > MAX_EXPRESSION_BYTES + 1 {
        Err(error("floating graph exceeds the expression byte budget"))
    } else {
        Ok(())
    }
}
pub(super) fn decode_expression(
    core: &AilCore,
    root: &Node,
    owner: Option<&Node>,
) -> Result<FloatExpr, String> {
    owner_valid(owner)?;
    let expression = decode_tree(core, root, owner, 1, &mut 0, &mut 0)?;
    FloatExpr::from_postfix(&expression.to_postfix())?;
    Ok(expression)
}
pub(super) fn decode_condition(
    core: &AilCore,
    root: &Node,
    owner: Option<&Node>,
) -> Result<FloatCondition, String> {
    owner_valid(owner)?;
    if root.kind != "FloatComparison" || root.type_name.as_deref() != Some("Bool") {
        return Err(error("expected FloatComparison with Bool type"));
    }
    attrs(root, &["comparison"], owner)?;
    edges(core, root, &["comparison_left", "comparison_right"])?;
    let incoming: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == root.id)
        .collect();
    if incoming.len() != 1 || !incoming[0].attributes.is_empty() {
        return Err(error(
            "floating comparison requires exactly one owning control",
        ));
    }
    let parent = by_id(core, &incoming[0].source)?;
    if !(if let Some(owner) = owner {
        incoming[0].kind == "value_condition"
            && parent.kind == "ValueFunctionStep"
            && parent.attributes.get("function_owner") == Some(&owner.name)
    } else {
        incoming[0].kind == "condition" && parent.kind == "Control"
    }) {
        return Err(error("invalid floating comparison owner"));
    }
    if owner.is_none() {
        let steps: Vec<_> = core
            .graph
            .edges
            .iter()
            .filter(|e| e.target == parent.id)
            .collect();
        if steps.len() != 1
            || steps[0].kind != "control_step"
            || !executable_action(by_id(core, &steps[0].source)?)
            || steps[0]
                .attributes
                .get("step_index")
                .is_none_or(|v| v.parse::<usize>().is_err())
            || !steps[0].attributes.iter().all(|(k, v)| {
                matches!(k.as_str(), "step_index" | "operation_index") && v.parse::<usize>().is_ok()
            })
        {
            return Err(error(
                "floating control requires exactly one executable action owner",
            ));
        }
    }
    let mut count = 1;
    let mut bytes = root.attributes["comparison"].len().saturating_add(1);
    let left = decode_tree(
        core,
        child(core, root, "comparison_left")?,
        owner,
        2,
        &mut count,
        &mut bytes,
    )?;
    let right = decode_tree(
        core,
        child(core, root, "comparison_right")?,
        owner,
        2,
        &mut count,
        &mut bytes,
    )?;
    FloatCondition::from_postfix(
        &left.to_postfix(),
        &root.attributes["comparison"],
        &right.to_postfix(),
    )
}
fn decode(core: &AilCore, node: &Node) -> Result<AilFloatOperation, String> {
    if node.kind != "FloatOperation" || node.type_name.is_some() {
        return Err(error("expected untyped FloatOperation node"));
    }
    attrs(node, &["float_operation_kind"], None)?;
    let kind = node.attributes["float_operation_kind"].as_str();
    let allowed: &[&str] = match kind {
        "compute" => &["float_destination", "float_expression"],
        "int_to_float" | "float_to_int" => &["float_source", "float_destination"],
        _ => return Err(error("unknown floating operation kind")),
    };
    edges(core, node, allowed)?;
    let key = scoped_field(
        core,
        child(core, node, "float_destination")?,
        None,
        if kind == "float_to_int" {
            "Int"
        } else {
            "Float"
        },
    )?;
    Ok(match kind {
        "compute" => AilFloatOperation::Compute {
            key,
            expression: decode_expression(core, child(core, node, "float_expression")?, None)?,
        },
        "int_to_float" => AilFloatOperation::IntToFloat {
            key,
            source: scoped_field(core, child(core, node, "float_source")?, None, "Int")?,
        },
        "float_to_int" => AilFloatOperation::FloatToInt {
            key,
            source: scoped_field(core, child(core, node, "float_source")?, None, "Float")?,
        },
        _ => unreachable!(),
    })
}
pub(super) fn from_core(core: &AilCore, action: &Node) -> Vec<AilFloatOperation> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.kind == "float_step" && e.source == action.id)
        .map(|e| {
            by_id(core, &e.target)
                .and_then(|n| decode(core, n))
                .unwrap_or_else(AilFloatOperation::Invalid)
        })
        .collect()
}
fn executable_action(node: &Node) -> bool {
    node.kind == "Action"
        && node
            .attributes
            .get("kind")
            .is_none_or(|v| v != "CompilerPass")
}
fn owned(core: &AilCore, node: &Node) -> Result<Option<Node>, String> {
    node.attributes
        .get("function_owner")
        .map(|name| {
            core.graph
                .find_node("Function", name)
                .cloned()
                .ok_or_else(|| error("floating node owner is missing"))
        })
        .transpose()
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut reachable = BTreeSet::new();
    for node in core.graph.nodes.iter().filter(|n| {
        matches!(
            n.kind.as_str(),
            "FloatOperation" | "FloatExpression" | "FloatComparison"
        )
    }) {
        let result = (|| -> Result<(), String> {
            if !matches!(core.package.profile.as_str(), "Application" | "C interop") {
                return Err(error(
                    "floating execution requires Application or C interop profile",
                ));
            }
            match node.kind.as_str() {
                "FloatOperation" => {
                    let parents: Vec<_> = core
                        .graph
                        .edges
                        .iter()
                        .filter(|e| e.target == node.id)
                        .collect();
                    if parents.len() != 1
                        || parents[0].kind != "float_step"
                        || !executable_action(by_id(core, &parents[0].source)?)
                    {
                        return Err(error(
                            "floating operation requires exactly one executable action owner",
                        ));
                    }
                    decode(core, node)?;
                }
                "FloatComparison" => {
                    let owner = owned(core, node)?;
                    decode_condition(core, node, owner.as_ref())?;
                }
                "FloatExpression" => {
                    let owner = owned(core, node)?;
                    expression_parent(core, node, owner.as_ref())?;
                    let parent = core
                        .graph
                        .edges
                        .iter()
                        .find(|e| e.target == node.id)
                        .unwrap();
                    if by_id(core, &parent.source)?.kind != "FloatExpression" {
                        decode_expression(core, node, owner.as_ref())?;
                        let mut pending = vec![node.id.as_str()];
                        while let Some(id) = pending.pop() {
                            if !reachable.insert(id.to_string()) {
                                continue;
                            }
                            for edge in core.graph.edges.iter().filter(|e| {
                                e.source == id
                                    && matches!(
                                        e.kind.as_str(),
                                        "left_operand" | "right_operand" | "unary_operand"
                                    )
                            }) {
                                pending.push(edge.target.as_str());
                            }
                        }
                    }
                }
                _ => unreachable!(),
            }
            Ok(())
        })();
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-FLOAT-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for node in core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind == "FloatExpression" && !reachable.contains(&n.id))
    {
        diagnostics.push(
            AilDiagnostic::error("AIL-FLOAT-001", "floating expression is orphaned or cyclic")
                .with_affected_graph_item(format!("node:{}", node.id)),
        );
    }
    for edge in &core.graph.edges {
        let source = by_id(core, &edge.source).ok();
        let target = by_id(core, &edge.target).ok();
        let valid = match edge.kind.as_str() {
            "float_step" => {
                source.is_some_and(executable_action)
                    && target.is_some_and(|n| n.kind == "FloatOperation")
                    && edge
                        .attributes
                        .get("step_index")
                        .is_some_and(|v| v.parse::<usize>().is_ok())
                    && edge.attributes.iter().all(|(k, v)| {
                        matches!(k.as_str(), "step_index" | "operation_index")
                            && v.parse::<usize>().is_ok()
                    })
            }
            "float_destination" | "float_source" => {
                source.is_some_and(|n| n.kind == "FloatOperation")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            "float_expression" => {
                source.is_some_and(|n| n.kind == "FloatOperation")
                    && target.is_some_and(|n| n.kind == "FloatExpression")
                    && edge.attributes.is_empty()
            }
            "unary_operand" => {
                source.is_some_and(|n| n.kind == "FloatExpression")
                    && target.is_some_and(|n| n.kind == "FloatExpression")
                    && edge.attributes.is_empty()
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-FLOAT-001",
                    format!("invalid {} floating edge", edge.kind),
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
        .filter(|e| matches!(e.kind.as_str(), "float_step" | "control_step"))
    {
        let mut pending = vec![step.target.as_str()];
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            for edge in core.graph.edges.iter().filter(|e| e.source == id) {
                if kind == "writes" && edge.kind == "float_destination" {
                    result.push((edge, step.source.as_str()));
                }
                if kind != "reads" {
                    continue;
                }
                match edge.kind.as_str() {
                    "float_source" | "expression_field"
                        if step.kind == "float_step"
                            || by_id(core, &edge.source)
                                .is_ok_and(|n| n.kind == "FloatExpression") =>
                    {
                        result.push((edge, step.source.as_str()))
                    }
                    "float_expression" | "condition" | "comparison_left" | "comparison_right"
                    | "left_operand" | "right_operand" | "unary_operand" => {
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
    #[test]
    fn float_operation_source_and_conversion_runtime_are_explicit() {
        let operation =
            parse_bullet("the system computes floating output.value as input.value / 2")
                .unwrap()
                .unwrap();
        assert!(matches!(operation, AilFloatOperation::Compute { .. }));
        let mut state = BTreeMap::from([("input.value".into(), "7".into())]);
        let convert = AilBytecodeInstruction::new(
            "INT_TO_FLOAT",
            &[
                ("source", "input.value".into()),
                ("key", "output.value".into()),
            ],
        );
        execute(&convert, &mut state).unwrap();
        assert_eq!(state["output.value"], "7e0");
        state.insert("input.value".into(), "9223372036854775808".into());
        let convert = AilBytecodeInstruction::new(
            "FLOAT_TO_INT",
            &[
                ("source", "input.value".into()),
                ("key", "output.value".into()),
            ],
        );
        assert!(
            execute(&convert, &mut state)
                .unwrap_err()
                .contains("AIL-RUNTIME-FLOAT")
        );
        assert_eq!(state["output.value"], "7e0");
    }
    fn core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Floating manages values.\n\nA Data has:\n- value: Float\n- other: Float\n- count: Int\n\nAn Output has:\n- value: Float\n- whole: Int\n\nFunction: transform.\nThe function needs:\n- value: Float\nThe function produces:\n- result: Float\nThe function stores:\n- result: Float\nThe function executes:\n- the function computes floating local.result as -(local.value * 2)\n- if floating local.result != nan:\n- the function returns local.result\n- end if\n- the function returns local.value\n\nAction: Run.\nWhen Run happens:\n- the system computes floating output.value as data.value + -(data.other)\n- the system converts integer data.count to floating output.value\n- the system converts floating output.value to integer output.whole\n- if floating output.value > 0:\n- while data.count > 0:\n- break\n- end while\n- end if\n- the system calls transform with value = data.value into output.value\n- the system records a trace event named Floating\n";
        elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, source).unwrap(),
        )
    }
    #[test]
    fn floating_core_roundtrip_preserves_local_and_global_typed_trees() {
        let core = core_fixture();
        assert!(check(&core).is_empty(), "{:?}", check(&core));
        assert!(
            control_flow::check(&core).is_empty(),
            "{:?}",
            control_flow::check(&core)
        );
        let document = ail_document_from_core(&core);
        let saved = parse_ail_core_text(&render_ail_core(&core)).unwrap();
        assert!(check(&saved).is_empty(), "{:?}", check(&saved));
        let restored = ail_document_from_core(&saved);
        assert_eq!(
            document.actions["Run"].float_operations,
            restored.actions["Run"].float_operations
        );
        assert_eq!(
            document.functions["transform"].integer,
            restored.functions["transform"].integer
        );
        assert_eq!(restored.actions["Run"].float_operations.len(), 3);
    }
    #[test]
    fn floating_core_rejects_extra_metadata_shared_trees_and_foreign_fields() {
        let original = core_fixture();
        for relation in [
            "float_step",
            "float_destination",
            "float_source",
            "float_expression",
            "unary_operand",
            "comparison_left",
        ] {
            let mut core = original.clone();
            let duplicate = core
                .graph
                .edges
                .iter()
                .find(|e| e.kind == relation)
                .unwrap()
                .clone();
            core.graph.edges.push(duplicate);
            let saved = parse_ail_core_text(&render_ail_core(&core)).unwrap();
            assert!(!check(&saved).is_empty(), "accepted duplicate {relation}");
        }
        for kind in ["FloatOperation", "FloatExpression", "FloatComparison"] {
            let mut core = original.clone();
            core.graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == kind)
                .unwrap()
                .attributes
                .insert("ignored".into(), "true".into());
            assert!(
                !check(&core).is_empty(),
                "accepted extra metadata on {kind}"
            );
        }
        let mut core = original.clone();
        let global = core
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "Field" && n.name == "Data.value")
            .unwrap()
            .id
            .clone();
        let local = core
            .graph
            .nodes
            .iter()
            .find(|n| {
                n.kind == "Field"
                    && n.attributes
                        .get("function_owner")
                        .is_some_and(|v| v == "transform")
            })
            .unwrap()
            .id
            .clone();
        core.graph
            .edges
            .iter_mut()
            .find(|e| e.kind == "expression_field" && e.target == global)
            .unwrap()
            .target = local;
        assert!(!check(&core).is_empty());
        let mut core = original.clone();
        let original_field = core
            .graph
            .nodes
            .iter()
            .find(|n| n.id == global)
            .unwrap()
            .clone();
        let mut forged = original_field.clone();
        forged.id = "forged-global".into();
        forged.attributes.clear();
        core.graph.nodes.push(forged.clone());
        core.graph
            .edges
            .iter_mut()
            .find(|e| e.kind == "expression_field" && e.target == global)
            .unwrap()
            .target = forged.id;
        assert!(!check(&core).is_empty());
    }
    #[test]
    fn floating_controls_require_executable_owners_and_derive_secret_reads() {
        let original = core_fixture();
        let mut core = original.clone();
        let comparison = core
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "FloatComparison" && !n.attributes.contains_key("function_owner"))
            .unwrap()
            .id
            .clone();
        let control = core
            .graph
            .edges
            .iter()
            .find(|e| e.kind == "condition" && e.target == comparison)
            .unwrap()
            .source
            .clone();
        core.graph.edges.retain(|e| e.target != control);
        assert!(!check(&core).is_empty(), "orphan floating control accepted");
        let mut core = original.clone();
        let secret = core
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name == "Data.other")
            .unwrap();
        secret.attributes.insert("secret".into(), "true".into());
        core.graph.edges.retain(|e| e.kind != "reads");
        assert!(check_ail_core(&core).iter().any(|d| d.contains("AIL005")));
        let mut core = original;
        core.package.profile = "AgentTool".into();
        assert!(!check(&core).is_empty());
    }
    #[test]
    fn floating_core_applies_byte_bounds_before_literal_canonicalization() {
        let mut core = core_fixture();
        core.graph
            .nodes
            .iter_mut()
            .find(|n| {
                n.kind == "FloatExpression"
                    && n.attributes
                        .get("expression_kind")
                        .is_some_and(|v| v == "literal")
            })
            .unwrap()
            .attributes
            .insert("value".into(), "0".repeat(65_537));
        assert!(
            !check(&core).is_empty(),
            "oversized raw floating literal accepted"
        );
        let mut core = core_fixture();
        let comparison = core
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "FloatComparison" && n.attributes.contains_key("function_owner"))
            .unwrap()
            .id
            .clone();
        let operands: Vec<_> = core
            .graph
            .edges
            .iter()
            .filter(|e| {
                e.source == comparison
                    && matches!(e.kind.as_str(), "comparison_left" | "comparison_right")
            })
            .map(|e| e.target.clone())
            .collect();
        for id in operands {
            core.graph
                .edges
                .retain(|e| e.source != id || e.kind == "has_provenance");
            let node = core.graph.nodes.iter_mut().find(|n| n.id == id).unwrap();
            node.attributes
                .insert("expression_kind".into(), "literal".into());
            node.attributes.insert("value".into(), "0".repeat(33_000));
        }
        assert!(
            !check(&core).is_empty(),
            "combined floating condition byte limit ignored"
        );
    }
}
