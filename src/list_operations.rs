//! Typed flat lists: source, checked Core, and bounded JSON runtime values.
use super::*;
use crate::expression::IntExpr;
use crate::float_expression::FloatExpr;

const MAX_ITEMS: usize = 65_536;
const MAX_BYTES: usize = 1_048_576;
const MAX_ALLOCATED: usize = 67_108_864;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilListValue {
    Integer(IntExpr),
    Floating(FloatExpr),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilListOperation {
    Empty {
        key: String,
    },
    Length {
        key: String,
        source: String,
    },
    Get {
        key: String,
        source: String,
        index: IntExpr,
    },
    Append {
        key: String,
        source: String,
        value: AilListValue,
    },
    Set {
        key: String,
        source: String,
        index: IntExpr,
        value: AilListValue,
    },
    Concat {
        key: String,
        source: String,
        other: String,
    },
    Slice {
        key: String,
        source: String,
        start: IntExpr,
        end: IntExpr,
    },
    Reverse {
        key: String,
        source: String,
    },
    Sort {
        key: String,
        source: String,
        descending: bool,
    },
    Invalid(String),
}

impl AilListValue {
    fn source(&self) -> String {
        match self {
            Self::Integer(e) => format!("integer {}", e.to_source()),
            Self::Floating(e) => format!("floating {}", e.to_source()),
            Self::Text(key) => format!("text {key}"),
        }
    }
}
impl AilListOperation {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Empty { key } => format!("the system creates an empty list into {key}"),
            Self::Length { key, source } => {
                format!("the system counts items in list {source} into {key}")
            }
            Self::Get { key, source, index } => format!(
                "the system gets item {} from list {source} into {key}",
                index.to_source()
            ),
            Self::Append { key, source, value } => format!(
                "the system appends {} to list {source} into {key}",
                value.source()
            ),
            Self::Set {
                key,
                source,
                index,
                value,
            } => format!(
                "the system sets item {} in list {source} to {} into {key}",
                index.to_source(),
                value.source()
            ),
            Self::Concat { key, source, other } => {
                format!("the system concatenates list {source} and list {other} into {key}")
            }
            Self::Slice {
                key,
                source,
                start,
                end,
            } => format!(
                "the system slices list {source} from {} to {} into {key}",
                start.to_source(),
                end.to_source()
            ),
            Self::Reverse { key, source } => {
                format!("the system reverses list {source} into {key}")
            }
            Self::Sort {
                key,
                source,
                descending,
            } => format!(
                "the system sorts list {source} in {} order into {key}",
                if *descending {
                    "descending"
                } else {
                    "ascending"
                }
            ),
            Self::Invalid(message) => format!("invalid list operation: {message}"),
        }
    }
    fn key(&self) -> Result<&str, String> {
        match self {
            Self::Empty { key }
            | Self::Length { key, .. }
            | Self::Get { key, .. }
            | Self::Append { key, .. }
            | Self::Set { key, .. }
            | Self::Concat { key, .. }
            | Self::Slice { key, .. }
            | Self::Reverse { key, .. }
            | Self::Sort { key, .. } => Ok(key),
            Self::Invalid(message) => Err(error(message)),
        }
    }
    fn list_source(&self) -> Option<&str> {
        match self {
            Self::Length { source, .. }
            | Self::Get { source, .. }
            | Self::Append { source, .. }
            | Self::Set { source, .. }
            | Self::Concat { source, .. }
            | Self::Slice { source, .. }
            | Self::Reverse { source, .. }
            | Self::Sort { source, .. } => Some(source),
            _ => None,
        }
    }
    fn index(&self) -> Option<&IntExpr> {
        match self {
            Self::Get { index, .. } | Self::Set { index, .. } => Some(index),
            _ => None,
        }
    }
    fn value(&self) -> Option<&AilListValue> {
        match self {
            Self::Append { value, .. } | Self::Set { value, .. } => Some(value),
            _ => None,
        }
    }
    fn kind(&self) -> &'static str {
        match self {
            Self::Empty { .. } => "empty",
            Self::Length { .. } => "length",
            Self::Get { .. } => "get",
            Self::Append { .. } => "append",
            Self::Set { .. } => "set",
            Self::Concat { .. } => "concat",
            Self::Slice { .. } => "slice",
            Self::Reverse { .. } => "reverse",
            Self::Sort { .. } => "sort",
            Self::Invalid(_) => "invalid",
        }
    }
}
fn error(message: impl Display) -> String {
    format!("AIL-LIST-001: {message}")
}
fn field(key: &str) -> Result<(), String> {
    if matches!(IntExpr::parse(key),Ok(IntExpr::Field(value)) if value==key) {
        Ok(())
    } else {
        Err(error(format!("expected field reference {key}")))
    }
}
fn parse_value(text: &str) -> Result<AilListValue, String> {
    if let Some(expression) = text.strip_prefix("integer ") {
        Ok(AilListValue::Integer(IntExpr::parse(expression)?))
    } else if let Some(expression) = text.strip_prefix("floating ") {
        Ok(AilListValue::Floating(
            FloatExpr::parse(expression).map_err(error)?,
        ))
    } else if let Some(key) = text.strip_prefix("text ") {
        field(key)?;
        Ok(AilListValue::Text(key.into()))
    } else {
        Err(error(
            "list item requires an explicit integer, floating or text value",
        ))
    }
}
pub(super) fn parse_bullet(text: &str) -> Option<Result<AilListOperation, String>> {
    let text = trim_sentence(text);
    let (kind, rest) =
        if let Some(rest) = text.strip_prefix("the system creates an empty list into ") {
            ("empty", rest)
        } else if let Some(rest) = text.strip_prefix("the system counts items in list ") {
            ("length", rest)
        } else if let Some(rest) = text.strip_prefix("the system gets item ") {
            ("get", rest)
        } else if let Some(rest) = text.strip_prefix("the system appends ") {
            ("append", rest)
        } else if let Some(rest) = text.strip_prefix("the system sets item ") {
            ("set", rest)
        } else if let Some(rest) = text.strip_prefix("the system concatenates list ") {
            ("concat", rest)
        } else if let Some(rest) = text.strip_prefix("the system slices list ") {
            ("slice", rest)
        } else if let Some(rest) = text.strip_prefix("the system reverses list ") {
            ("reverse", rest)
        } else if let Some(rest) = text.strip_prefix("the system sorts list ") {
            ("sort", rest)
        } else {
            return [
                "the system creates an empty list",
                "the system counts items",
                "the system gets item",
                "the system appends",
                "the system sets item",
                "the system concatenates",
                "the system slices list",
                "the system reverses",
                "the system sorts",
            ]
            .iter()
            .any(|prefix| {
                text == *prefix
                    || text
                        .strip_prefix(prefix)
                        .is_some_and(|rest| rest.starts_with(' '))
            })
            .then(|| Err(error("malformed list operation")));
        };
    Some((|| {
        let result = if kind == "empty" {
            AilListOperation::Empty { key: rest.into() }
        } else {
            let (operands, key) = rest
                .rsplit_once(" into ")
                .ok_or_else(|| error("list operation requires 'into field'"))?;
            match kind {
                "length" => AilListOperation::Length {
                    key: key.into(),
                    source: operands.into(),
                },
                "get" => {
                    let (index, source) = operands
                        .split_once(" from list ")
                        .ok_or_else(|| error("get requires 'index from list field'"))?;
                    AilListOperation::Get {
                        key: key.into(),
                        source: source.into(),
                        index: IntExpr::parse(index)?,
                    }
                }
                "append" => {
                    let (value, source) = operands
                        .split_once(" to list ")
                        .ok_or_else(|| error("append requires 'value to list field'"))?;
                    AilListOperation::Append {
                        key: key.into(),
                        source: source.into(),
                        value: parse_value(value)?,
                    }
                }
                "set" => {
                    let (index, rest) = operands
                        .split_once(" in list ")
                        .ok_or_else(|| error("set requires 'index in list field'"))?;
                    let (source, value) = rest
                        .split_once(" to ")
                        .ok_or_else(|| error("set requires an item value"))?;
                    AilListOperation::Set {
                        key: key.into(),
                        source: source.into(),
                        index: IntExpr::parse(index)?,
                        value: parse_value(value)?,
                    }
                }
                "concat" => {
                    let (source, other) = operands
                        .split_once(" and list ")
                        .ok_or_else(|| error("concat requires 'list field and list field'"))?;
                    field(other)?;
                    AilListOperation::Concat {
                        key: key.into(),
                        source: source.into(),
                        other: other.into(),
                    }
                }
                "slice" => {
                    let (source, bounds) = operands
                        .split_once(" from ")
                        .ok_or_else(|| error("slice requires start and end bounds"))?;
                    let (start, end) = bounds
                        .split_once(" to ")
                        .ok_or_else(|| error("slice requires an end bound"))?;
                    AilListOperation::Slice {
                        key: key.into(),
                        source: source.into(),
                        start: IntExpr::parse(start).map_err(error)?,
                        end: IntExpr::parse(end).map_err(error)?,
                    }
                }
                "reverse" => AilListOperation::Reverse {
                    key: key.into(),
                    source: operands.into(),
                },
                "sort" => {
                    let (source, order) = operands
                        .split_once(" in ")
                        .ok_or_else(|| error("sort requires an explicit direction"))?;
                    let descending = match order {
                        "ascending order" => false,
                        "descending order" => true,
                        _ => return Err(error("sort direction must be ascending or descending")),
                    };
                    AilListOperation::Sort {
                        key: key.into(),
                        source: source.into(),
                        descending,
                    }
                }
                _ => unreachable!(),
            }
        };
        field(result.key()?)?;
        if let Some(source) = result.list_source() {
            field(source)?;
        }
        Ok(result)
    })())
}
fn target(graph: &mut Graph, key: &str, provenance: &str) -> Node {
    if let Some(node) = graph.nodes.iter().find(|n| {
        n.kind == "Field"
            && !n.attributes.contains_key("function_owner")
            && !n.attributes.contains_key("local_name")
            && expression_core::field_key(n) == key
    }) {
        return node.clone();
    }
    let node = graph.add_node("Effect", key, None, BTreeMap::new());
    attach_provenance(graph, &node, provenance);
    node
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, operation) in action.list_operations.iter().enumerate() {
            let name = format!("{}.list{index}", owner.name);
            let element = element_type(operation, document).unwrap_or("invalid");
            let mut attributes =
                attr(&[("list_kind", operation.kind()), ("element_type", element)]);
            if let AilListOperation::Sort { descending, .. } = operation {
                attributes.insert(
                    "sort_order".into(),
                    if *descending {
                        "descending"
                    } else {
                        "ascending"
                    }
                    .into(),
                );
            }
            let node = graph.add_node("ListOperation", &name, None, attributes);
            attach_provenance(graph, &node, format!("action:{}.list:{index}", owner.name));
            if let Ok(key) = operation.key() {
                let field = target(graph, key, &name);
                graph.add_edge("list_destination", &node, &field, BTreeMap::new());
            }
            if let Some(source) = operation.list_source() {
                let field = target(graph, source, &name);
                graph.add_edge("list_source", &node, &field, BTreeMap::new());
            }
            if let Some(index) = operation.index() {
                let root = expression_core::build(graph, &owner, &format!("{name}.index"), index);
                graph.add_edge("list_index", &node, &root, BTreeMap::new());
            }
            if let AilListOperation::Concat { other, .. } = operation {
                let field = target(graph, other, &name);
                graph.add_edge("list_other", &node, &field, BTreeMap::new());
            }
            if let AilListOperation::Slice { start, end, .. } = operation {
                for (role, expression) in [("list_start", start), ("list_end", end)] {
                    let root = expression_core::build(
                        graph,
                        &owner,
                        &format!("{name}.{role}"),
                        expression,
                    );
                    graph.add_edge(role, &node, &root, BTreeMap::new());
                }
            }
            if let Some(value) = operation.value() {
                let root = match value {
                    AilListValue::Integer(e) => {
                        expression_core::build(graph, &owner, &format!("{name}.value"), e)
                    }
                    AilListValue::Floating(e) => float_operations::build_expression(
                        graph,
                        &owner,
                        &format!("{name}.value"),
                        e,
                        false,
                    ),
                    AilListValue::Text(key) => target(graph, key, &name),
                };
                graph.add_edge("list_value", &node, &root, BTreeMap::new());
            }
            action_order::append(graph, "list_step", &owner, &node, BTreeMap::new(), index);
        }
    }
}
fn child<'a>(core: &'a AilCore, node: &Node, kind: &str) -> Result<&'a Node, String> {
    let mut edges = core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == kind);
    let edge = edges
        .next()
        .ok_or_else(|| error(format!("{} requires {kind}", node.name)))?;
    if edges.next().is_some() {
        return Err(error(format!("{} has duplicate {kind} edges", node.name)));
    }
    if !edge.attributes.is_empty() {
        return Err(error(format!("unexpected {kind} operand attributes")));
    }
    core.graph
        .nodes
        .iter()
        .find(|n| n.id == edge.target)
        .ok_or_else(|| error("missing list operand endpoint"))
}
fn global_field(core: &AilCore, node: &Node, expected: &str) -> Result<String, String> {
    float_operations::scoped_field(core, node, None, expected).map_err(error)
}
fn expression(core: &AilCore, root: &Node) -> Result<IntExpr, String> {
    checked_expression(core, root, None)
}

pub(super) fn decode_slice_bound(
    core: &AilCore,
    root: &Node,
    owner: &Node,
    role: &str,
) -> Result<IntExpr, String> {
    checked_expression(core, root, Some((owner, role)))
}

fn checked_expression(
    core: &AilCore,
    root: &Node,
    local_bound: Option<(&Node, &str)>,
) -> Result<IntExpr, String> {
    // Check raw tokens and ownership before decoding can normalize literals.
    let mut pending = vec![(root, 1usize, None::<&str>)];
    let mut visited = BTreeSet::new();
    let mut bytes = 0usize;
    while let Some((node, depth, parent)) = pending.pop() {
        if !visited.insert(&node.id) {
            return Err(error("list integer expression cannot be shared"));
        }
        if visited.len() > crate::expression::MAX_EXPRESSION_NODES
            || depth > crate::expression::MAX_EXPRESSION_DEPTH
        {
            return Err(error("list integer expression exceeds tree limits"));
        }
        if node.kind != "IntegerExpression" || node.type_name.as_deref() != Some("Int") {
            return Err(error("list bound requires a typed integer expression"));
        }
        let incoming: Vec<_> = core
            .graph
            .edges
            .iter()
            .filter(|e| e.target == node.id)
            .collect();
        let valid_parent = incoming.len() == 1 && {
            let edge = incoming[0];
            if let Some(parent) = parent {
                edge.source == parent
                    && edge.attributes.is_empty()
                    && matches!(edge.kind.as_str(), "left_operand" | "right_operand")
            } else if let Some((owner, role)) = local_bound {
                matches!(role, "start" | "end")
                    && edge.kind == "value_expression_operand"
                    && edge.attributes.len() == 1
                    && edge.attributes.get("name").is_some_and(|name| name == role)
                    && core.graph.nodes.iter().any(|n| {
                        n.id == edge.source
                            && n.kind == "ValueFunctionStep"
                            && n.attributes.get("function_owner") == Some(&owner.name)
                            && n.attributes
                                .get("opcode")
                                .is_some_and(|op| op == "LIST_SLICE")
                    })
            } else {
                edge.attributes.is_empty()
                    && matches!(
                        edge.kind.as_str(),
                        "list_index" | "list_value" | "list_start" | "list_end"
                    )
                    && core
                        .graph
                        .nodes
                        .iter()
                        .any(|n| n.id == edge.source && n.kind == "ListOperation")
            }
        };
        if !valid_parent {
            return Err(error(
                "list integer expression requires exactly one typed parent",
            ));
        }
        let token_bytes = match node.attributes.get("expression_kind").map(String::as_str) {
            Some("literal") => node
                .attributes
                .get("value")
                .ok_or_else(|| error("missing list integer literal"))?
                .len()
                .saturating_add(2),
            Some("field") => {
                expression_core::field_key(child(core, node, "expression_field")?).len() + 2
            }
            Some("binary") => node
                .attributes
                .get("operator")
                .ok_or_else(|| error("missing list integer operator"))?
                .len(),
            _ => return Err(error("invalid list integer expression")),
        };
        // Count one separator per token, allowing the unused final separator.
        bytes = bytes.saturating_add(token_bytes).saturating_add(1);
        if bytes > crate::expression::MAX_EXPRESSION_BYTES + 1 {
            return Err(error("list integer expression exceeds input byte limit"));
        }
        for edge in core.graph.edges.iter().filter(|e| e.source == node.id) {
            match edge.kind.as_str() {
                "expression_field" => {
                    float_operations::scoped_field(
                        core,
                        child(core, node, "expression_field")?,
                        local_bound.map(|(owner, _)| owner),
                        "Int",
                    )
                    .map_err(error)?;
                }
                "left_operand" | "right_operand" => {
                    pending.push((child(core, node, &edge.kind)?, depth + 1, Some(&node.id)));
                }
                "has_provenance" => {}
                _ => return Err(error("unexpected edge on list integer expression")),
            }
        }
    }
    value_function::scoped_expression(core, root, local_bound.map(|(owner, _)| owner))
        .map_err(error)
}

fn decode(core: &AilCore, node: &Node) -> Result<AilListOperation, String> {
    if node.kind != "ListOperation" || node.type_name.is_some() {
        return Err(error("list_step must target ListOperation"));
    }
    let kind = node
        .attributes
        .get("list_kind")
        .map(String::as_str)
        .unwrap_or("");
    let required: &[&str] = if kind == "sort" {
        &["list_kind", "element_type", "sort_order"]
    } else {
        &["list_kind", "element_type"]
    };
    if node.attributes.len() != required.len()
        || required
            .iter()
            .any(|key| !node.attributes.contains_key(*key))
    {
        return Err(error("unexpected list operation attribute"));
    }
    let owners: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect();
    if owners.len() != 1
        || owners[0].kind != "list_step"
        || !valid_step_attributes(owners[0])
        || !core
            .graph
            .nodes
            .iter()
            .any(|n| n.id == owners[0].source && executable_action(n))
    {
        return Err(error(
            "list operation requires exactly one executable action owner",
        ));
    }
    let element = node
        .attributes
        .get("element_type")
        .map(String::as_str)
        .unwrap_or("");
    if !matches!(element, "Int" | "Float" | "Text") {
        return Err(error(
            "list operation needs element_type Int, Float or Text",
        ));
    }
    let allowed: &[&str] = match kind {
        "empty" => &["list_destination"],
        "length" => &["list_source", "list_destination"],
        "get" => &["list_source", "list_destination", "list_index"],
        "append" => &["list_source", "list_destination", "list_value"],
        "set" => &[
            "list_source",
            "list_destination",
            "list_index",
            "list_value",
        ],
        "concat" => &["list_source", "list_other", "list_destination"],
        "slice" => &["list_source", "list_start", "list_end", "list_destination"],
        "reverse" | "sort" => &["list_source", "list_destination"],
        _ => return Err(error("invalid list operation kind")),
    };
    if core.graph.edges.iter().any(|e| {
        e.source == node.id && e.kind != "has_provenance" && !allowed.contains(&e.kind.as_str())
    }) {
        return Err(error("unexpected edge on list operation"));
    }
    let key = global_field(
        core,
        child(core, node, "list_destination")?,
        match kind {
            "length" => "Int",
            "get" => element,
            _ => list_type(element),
        },
    )?;
    if kind == "empty" {
        return Ok(AilListOperation::Empty { key });
    }
    let source = global_field(core, child(core, node, "list_source")?, list_type(element))?;
    let value = || -> Result<AilListValue, String> {
        let node = child(core, node, "list_value")?;
        if element == "Int" {
            Ok(AilListValue::Integer(expression(core, node)?))
        } else if element == "Float" {
            Ok(AilListValue::Floating(
                float_operations::decode_expression(core, node, None).map_err(error)?,
            ))
        } else {
            Ok(AilListValue::Text(global_field(core, node, "Text")?))
        }
    };
    Ok(match kind {
        "length" => AilListOperation::Length { key, source },
        "get" => AilListOperation::Get {
            key,
            source,
            index: expression(core, child(core, node, "list_index")?)?,
        },
        "append" => AilListOperation::Append {
            key,
            source,
            value: value()?,
        },
        "set" => AilListOperation::Set {
            key,
            source,
            index: expression(core, child(core, node, "list_index")?)?,
            value: value()?,
        },
        "concat" => AilListOperation::Concat {
            key,
            source,
            other: global_field(core, child(core, node, "list_other")?, list_type(element))?,
        },
        "slice" => AilListOperation::Slice {
            key,
            source,
            start: expression(core, child(core, node, "list_start")?)?,
            end: expression(core, child(core, node, "list_end")?)?,
        },
        "reverse" => AilListOperation::Reverse { key, source },
        "sort" => AilListOperation::Sort {
            key,
            source,
            descending: match node.attributes["sort_order"].as_str() {
                "ascending" => false,
                "descending" => true,
                _ => return Err(error("sort direction must be ascending or descending")),
            },
        },
        _ => unreachable!(),
    })
}
pub(super) fn from_core(core: &AilCore, action: &Node) -> Vec<AilListOperation> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == action.id && e.kind == "list_step")
        .map(|e| {
            core.graph
                .nodes
                .iter()
                .find(|n| n.id == e.target)
                .ok_or_else(|| error("missing list operation"))
                .and_then(|n| decode(core, n))
                .unwrap_or_else(AilListOperation::Invalid)
        })
        .collect()
}
fn executable_action(node: &Node) -> bool {
    node.kind == "Action"
        && node
            .attributes
            .get("kind")
            .is_none_or(|kind| kind != "CompilerPass")
}
fn valid_step_attributes(edge: &Edge) -> bool {
    edge.attributes
        .get("step_index")
        .is_some_and(|s| s.parse::<usize>().is_ok())
        && edge.attributes.iter().all(|(key, value)| {
            matches!(key.as_str(), "step_index" | "operation_index")
                && value.parse::<usize>().is_ok()
        })
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    for node in core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind == "ListOperation")
    {
        let result = (|| {
            if !matches!(core.package.profile.as_str(), "Application" | "C interop") {
                return Err(error(
                    "list operations require Application or C interop profile",
                ));
            }
            let owners: Vec<_> = core
                .graph
                .edges
                .iter()
                .filter(|e| e.target == node.id)
                .collect();
            if owners.len() != 1
                || owners[0].kind != "list_step"
                || !core
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == owners[0].source && executable_action(n))
            {
                return Err(error(
                    "list operation requires exactly one executable action owner",
                ));
            }
            decode(core, node).map(|_| ())
        })();
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-LIST-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for edge in &core.graph.edges {
        let source = core.graph.nodes.iter().find(|n| n.id == edge.source);
        let target = core.graph.nodes.iter().find(|n| n.id == edge.target);
        let operation = source.is_some_and(|n| n.kind == "ListOperation");
        let valid = match edge.kind.as_str() {
            "list_step" => {
                source.is_some_and(executable_action)
                    && target.is_some_and(|n| n.kind == "ListOperation")
                    && valid_step_attributes(edge)
            }
            "list_source" | "list_other" | "list_destination" => {
                operation && target.is_some_and(|n| n.kind == "Field") && edge.attributes.is_empty()
            }
            "list_index" | "list_start" | "list_end" => {
                operation
                    && target.is_some_and(|n| n.kind == "IntegerExpression")
                    && edge.attributes.is_empty()
            }
            "list_value" => {
                operation
                    && edge.attributes.is_empty()
                    && target.is_some_and(|n| {
                        matches!(
                            n.kind.as_str(),
                            "IntegerExpression" | "FloatExpression" | "Field"
                        )
                    })
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error("AIL-LIST-001", format!("invalid {} list edge", edge.kind))
                    .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn namespace(operation: &AilListOperation, alias: &str) -> AilListOperation {
    fn qualify(expression: &IntExpr, prefix: &str) -> IntExpr {
        match expression {
            IntExpr::Literal(n) => IntExpr::Literal(*n),
            IntExpr::Field(key) => IntExpr::Field(format!("{prefix}.{key}")),
            IntExpr::Binary { op, left, right } => IntExpr::Binary {
                op: *op,
                left: Box::new(qualify(left, prefix)),
                right: Box::new(qualify(right, prefix)),
            },
        }
    }
    let prefix = runtime_subject_key(alias);
    let key = |key: &str| format!("{prefix}.{key}");
    let value = |value: &AilListValue| match value {
        AilListValue::Integer(e) => AilListValue::Integer(qualify(e, &prefix)),
        AilListValue::Floating(e) => AilListValue::Floating(e.map_fields(key)),
        AilListValue::Text(s) => AilListValue::Text(key(s)),
    };
    match operation {
        AilListOperation::Empty { key: k } => AilListOperation::Empty { key: key(k) },
        AilListOperation::Length { key: k, source } => AilListOperation::Length {
            key: key(k),
            source: key(source),
        },
        AilListOperation::Get {
            key: k,
            source,
            index,
        } => AilListOperation::Get {
            key: key(k),
            source: key(source),
            index: qualify(index, &prefix),
        },
        AilListOperation::Append {
            key: k,
            source,
            value: v,
        } => AilListOperation::Append {
            key: key(k),
            source: key(source),
            value: value(v),
        },
        AilListOperation::Set {
            key: k,
            source,
            index,
            value: v,
        } => AilListOperation::Set {
            key: key(k),
            source: key(source),
            index: qualify(index, &prefix),
            value: value(v),
        },
        AilListOperation::Concat {
            key: k,
            source,
            other,
        } => AilListOperation::Concat {
            key: key(k),
            source: key(source),
            other: key(other),
        },
        AilListOperation::Slice {
            key: k,
            source,
            start,
            end,
        } => AilListOperation::Slice {
            key: key(k),
            source: key(source),
            start: qualify(start, &prefix),
            end: qualify(end, &prefix),
        },
        AilListOperation::Reverse { key: k, source } => AilListOperation::Reverse {
            key: key(k),
            source: key(source),
        },
        AilListOperation::Sort {
            key: k,
            source,
            descending,
        } => AilListOperation::Sort {
            key: key(k),
            source: key(source),
            descending: *descending,
        },
        AilListOperation::Invalid(message) => AilListOperation::Invalid(message.clone()),
    }
}
fn list_type(element: &str) -> &'static str {
    match element {
        "Int" => "List<Int>",
        "Float" => "List<Float>",
        "Text" => "List<Text>",
        _ => "invalid",
    }
}
fn element_type(
    operation: &AilListOperation,
    document: &AilDocument,
) -> Result<&'static str, String> {
    let key = operation.list_source().unwrap_or(operation.key()?);
    match runtime_field_type_name(document, key) {
        Some("List<Int>") => Ok("Int"),
        Some("List<Float>") => Ok("Float"),
        Some("List<Text>") => Ok("Text"),
        _ => Err(error(format!(
            "{key} must be a declared List<Int>, List<Float> or List<Text> field"
        ))),
    }
}
fn destination_type<'a>(operation: &AilListOperation, element: &'a str) -> &'a str {
    match operation {
        AilListOperation::Length { .. } => "Int",
        AilListOperation::Get { .. } => element,
        _ => list_type(element),
    }
}
pub(super) fn check_source(
    operation: &AilListOperation,
    document: &AilDocument,
) -> Result<(), String> {
    let element = element_type(operation, document)?;
    field(operation.key()?)?;
    if runtime_field_type_name(document, operation.key()?)
        != Some(destination_type(operation, element))
    {
        return Err(error("list destination type does not match operation"));
    }
    let mut expressions = Vec::new();
    if let Some(source) = operation.list_source() {
        field(source)?;
    }
    if let Some(index) = operation.index() {
        expressions.push(index);
    }
    if let AilListOperation::Concat { other, .. } = operation {
        field(other)?;
        if runtime_field_type_name(document, other) != Some(list_type(element)) {
            return Err(error("concat operands must have the same list type"));
        }
    }
    if let AilListOperation::Slice { start, end, .. } = operation {
        expressions.extend([start, end]);
    }
    match operation.value() {
        Some(AilListValue::Integer(value)) if element == "Int" => expressions.push(value),
        Some(AilListValue::Floating(value)) if element == "Float" => {
            FloatExpr::from_postfix(&value.to_postfix()).map_err(error)?;
            for key in value.fields() {
                if runtime_field_type_name(document, &key) != Some("Float") {
                    return Err(error(format!("{key} must be a declared Float field")));
                }
            }
        }
        Some(AilListValue::Text(key)) if element == "Text" => {
            field(key)?;
            if runtime_field_type_name(document, key) != Some("Text") {
                return Err(error("text item must name a declared Text field"));
            }
        }
        Some(_) => return Err(error("item value type does not match list element type")),
        None => {}
    }
    for expression in expressions {
        IntExpr::from_postfix(&expression.to_postfix())?;
        for key in expression.fields() {
            if runtime_field_type_name(document, &key) != Some("Int") {
                return Err(error(format!("{key} must be a declared Int field")));
            }
        }
    }
    Ok(())
}
pub(super) fn instruction(
    operation: &AilListOperation,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    check_source(operation, document)?;
    let opcode = match operation.kind() {
        "empty" => "LIST_EMPTY",
        "length" => "LIST_LENGTH",
        "get" => "LIST_GET",
        "append" => "LIST_APPEND",
        "set" => "LIST_SET",
        "concat" => "LIST_CONCAT",
        "slice" => "LIST_SLICE",
        "reverse" => "LIST_REVERSE",
        "sort" => "LIST_SORT",
        _ => return Err(error("invalid list operation")),
    };
    let mut result = AilBytecodeInstruction::new(
        opcode,
        &[
            ("key", operation.key()?.into()),
            ("type", element_type(operation, document)?.into()),
        ],
    );
    if let Some(source) = operation.list_source() {
        result.operands.insert("source".into(), source.into());
    }
    if let Some(index) = operation.index() {
        result.operands.insert("index".into(), index.to_postfix());
    }
    match operation {
        AilListOperation::Concat { other, .. } => {
            result.operands.insert("other".into(), other.clone());
        }
        AilListOperation::Slice { start, end, .. } => {
            result.operands.insert("start".into(), start.to_postfix());
            result.operands.insert("end".into(), end.to_postfix());
        }
        AilListOperation::Sort { descending, .. } => {
            result.operands.insert(
                "order".into(),
                if *descending {
                    "descending"
                } else {
                    "ascending"
                }
                .into(),
            );
        }
        _ => {}
    }
    if let Some(value) = operation.value() {
        result.operands.insert(
            "value".into(),
            match value {
                AilListValue::Integer(e) => e.to_postfix(),
                AilListValue::Floating(e) => e.to_postfix(),
                AilListValue::Text(key) => key.clone(),
            },
        );
    }
    verify(&result)?;
    Ok(result)
}
pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    let expected: &[&str] = match instruction.opcode.as_str() {
        "LIST_EMPTY" => &["key", "type"],
        "LIST_LENGTH" => &["key", "source", "type"],
        "LIST_GET" => &["index", "key", "source", "type"],
        "LIST_APPEND" => &["key", "source", "type", "value"],
        "LIST_SET" => &["index", "key", "source", "type", "value"],
        "LIST_CONCAT" => &["key", "other", "source", "type"],
        "LIST_SLICE" => &["end", "key", "source", "start", "type"],
        "LIST_REVERSE" => &["key", "source", "type"],
        "LIST_SORT" => &["key", "order", "source", "type"],
        _ => return Err(error("unknown list opcode")),
    };
    if instruction
        .operands
        .keys()
        .map(String::as_str)
        .ne(expected.iter().copied())
    {
        return Err(error("list opcode requires exact operands"));
    }
    let element = instruction.operands["type"].as_str();
    if !matches!(element, "Int" | "Float" | "Text") {
        return Err(error("list element type must be Int, Float or Text"));
    }
    for (name, value) in &instruction.operands {
        match name.as_str() {
            "type" => {}
            "index" | "start" | "end" => {
                IntExpr::from_postfix(value)?;
            }
            "order" if matches!(value.as_str(), "ascending" | "descending") => {}
            "order" => return Err(error("sort direction must be ascending or descending")),
            "value" if element == "Int" => {
                IntExpr::from_postfix(value)?;
            }
            "value" if element == "Float" => {
                FloatExpr::from_postfix(value).map_err(error)?;
            }
            _ => field(value)?,
        }
    }
    Ok(())
}
#[derive(Debug, PartialEq, Eq)]
enum Item {
    Integer(i64),
    Floating(u64),
    Text(String),
}
fn runtime_error(message: impl Display) -> String {
    format!("AIL-RUNTIME-LIST: {message}")
}
fn limit(message: impl Display) -> String {
    format!("AIL-RUNTIME-LIST-LIMIT: {message}")
}
fn whitespace(parser: &mut AilJsonParser<'_>) {
    while parser
        .chars
        .peek()
        .is_some_and(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
    {
        parser.chars.next();
    }
}
fn integer_token(parser: &mut AilJsonParser<'_>) -> Result<i64, String> {
    let mut token = String::new();
    if parser.consume_char('-') {
        token.push('-');
    }
    match parser.chars.peek().copied() {
        Some('0') => {
            parser.chars.next();
            token.push('0');
            if parser.chars.peek().is_some_and(char::is_ascii_digit) {
                return Err(runtime_error("leading zero in integer item"));
            }
        }
        Some('1'..='9') => {
            while parser.chars.peek().is_some_and(char::is_ascii_digit) {
                token.push(parser.chars.next().unwrap());
            }
        }
        _ => return Err(runtime_error("expected a strict JSON integer item")),
    }
    token
        .parse::<i64>()
        .map_err(|_| runtime_error("integer item outside i64 range"))
}
fn parse_list(text: &str, element: &str) -> Result<Vec<Item>, String> {
    parse_list_mode(text, element, false)
}

fn floating_token(parser: &mut AilJsonParser<'_>, strict_json: bool) -> Result<u64, String> {
    let mut token = String::new();
    while parser
        .chars
        .peek()
        .is_some_and(|c| !matches!(c, ',' | ']' | ' ' | '\t' | '\r' | '\n'))
    {
        token.push(parser.chars.next().unwrap());
    }
    let result = if strict_json {
        crate::float_number::parse_json(&token)
    } else {
        crate::float_number::parse(&token)
    };
    result.map_err(|problem| match problem {
        crate::float_number::Error::Limit => limit("floating list item exceeds input limit"),
        _ => runtime_error("invalid floating list item"),
    })
}

fn parse_list_mode(text: &str, element: &str, strict_json: bool) -> Result<Vec<Item>, String> {
    if text.len() > MAX_BYTES {
        return Err(limit("encoded list exceeds 1048576 bytes"));
    }
    let mut parser = AilJsonParser::new(text);
    whitespace(&mut parser);
    if !parser.consume_char('[') {
        return Err(runtime_error("expected a flat JSON array"));
    }
    let mut result = Vec::new();
    whitespace(&mut parser);
    if !parser.consume_char(']') {
        loop {
            if result.len() >= MAX_ITEMS {
                return Err(limit("list exceeds 65536 items"));
            }
            whitespace(&mut parser);
            let value = if element == "Int" {
                Item::Integer(integer_token(&mut parser)?)
            } else if element == "Float" {
                Item::Floating(floating_token(&mut parser, strict_json)?)
            } else {
                let value = parser
                    .parse_string()
                    .map_err(|_| runtime_error("invalid text list item"))?;
                if value.len() > MAX_BYTES {
                    return Err(limit("text list item exceeds 1048576 bytes"));
                }
                Item::Text(value)
            };
            result.push(value);
            whitespace(&mut parser);
            if parser.consume_char(']') {
                break;
            }
            if !parser.consume_char(',') {
                return Err(runtime_error("expected comma or closing bracket"));
            }
        }
    }
    whitespace(&mut parser);
    if !parser.is_finished() {
        return Err(runtime_error("unexpected content after JSON array"));
    }
    Ok(result)
}
fn canonical(items: &[Item]) -> Result<String, String> {
    if items.len() > MAX_ITEMS {
        return Err(limit("list exceeds 65536 items"));
    }
    let mut result = String::from("[");
    for (index, item) in items.iter().enumerate() {
        let floating = if let Item::Floating(bits) = item {
            Some(crate::float_expression::format_bits(*bits))
        } else {
            None
        };
        let bytes = match item {
            Item::Integer(n) => n.to_string().len(),
            Item::Floating(_) => floating.as_ref().unwrap().len(),
            Item::Text(text) => {
                2 + text
                    .chars()
                    .map(|c| match c {
                        '"' | '\\' | '\n' | '\r' | '\t' => 2,
                        c if c.is_control() => 6,
                        c => c.len_utf8(),
                    })
                    .sum::<usize>()
            }
        };
        if result.len() + usize::from(index != 0) + bytes + 1 > MAX_BYTES {
            return Err(limit("encoded result exceeds 1048576 bytes"));
        }
        if index != 0 {
            result.push(',');
        }
        match item {
            Item::Integer(n) => result.push_str(&n.to_string()),
            Item::Floating(_) => result.push_str(floating.as_ref().unwrap()),
            Item::Text(text) => result.push_str(&json_string(text)),
        }
    }
    result.push(']');
    Ok(result)
}

/// Validate borrowed list input without requiring its canonical output to fit.
pub(super) fn validate_runtime_value(text: &str, element: &str) -> Result<(), String> {
    if !matches!(element, "Int" | "Float" | "Text") {
        return Err(runtime_error(
            "list element type must be Int, Float or Text",
        ));
    }
    parse_list(text, element).map(|_| ())
}

/// Validate a bounded flat list and return its canonical runtime transport.
pub(super) fn canonical_literal(text: &str, element: &str) -> Result<String, String> {
    if !matches!(element, "Int" | "Float" | "Text") {
        return Err(runtime_error(
            "list element type must be Int, Float or Text",
        ));
    }
    canonical(&parse_list(text, element)?)
}

/// Decode a borrowed List<Text>, without imposing a canonical re-encoding bound.
pub(super) fn parse_text_items(text: &str) -> Result<Vec<String>, String> {
    parse_list(text, "Text")
        .map_err(|problem| {
            if problem.starts_with("AIL-RUNTIME-LIST-LIMIT:") {
                limit("text list exceeds input limits")
            } else {
                runtime_error("invalid text list")
            }
        })
        .map(|items| {
            items
                .into_iter()
                .map(|item| match item {
                    Item::Text(text) => text,
                    _ => unreachable!("Text parser produces only text items"),
                })
                .collect()
        })
}

/// Own one canonical List<Text> result; validation failures do not charge it.
pub(super) fn make_text_list(items: &[String], allocated: &mut usize) -> Result<String, String> {
    if items.len() > MAX_ITEMS {
        return Err(limit("list exceeds 65536 items"));
    }
    if items.iter().any(|item| item.len() > MAX_BYTES) {
        return Err(limit("text list item exceeds 1048576 bytes"));
    }
    let result = canonical(&items.iter().cloned().map(Item::Text).collect::<Vec<_>>())?;
    let total = allocated
        .checked_add(result.len())
        .filter(|n| *n <= MAX_ALLOCATED)
        .ok_or_else(|| limit("list allocation budget exceeded"))?;
    *allocated = total;
    Ok(result)
}

/// Cross the strict finite JSON boundary without imposing recursive Json limits.
pub(super) fn canonical_float_json(text: &str, parsing: bool) -> Result<String, String> {
    let items = parse_list_mode(text, "Float", parsing)?;
    if items
        .iter()
        .any(|item| matches!(item, Item::Floating(bits) if !f64::from_bits(*bits).is_finite()))
    {
        return Err(runtime_error("JSON floating list items must be finite"));
    }
    canonical(&items)
}

fn list_index(value: i64, length: usize) -> Result<usize, String> {
    let index = if value < 0 {
        (length as i64).saturating_add(value)
    } else {
        value
    };
    if index < 0 || index >= length as i64 {
        return Err("AIL-RUNTIME-INDEX: list index outside valid range".into());
    }
    Ok(index as usize)
}

fn slice_bound(value: i64, length: usize) -> usize {
    if value < 0 {
        (length as i64)
            .saturating_add(value)
            .clamp(0, length as i64) as usize
    } else {
        value.min(length as i64) as usize
    }
}

fn compare_items(left: &Item, right: &Item, descending: bool) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let order = match (left, right) {
        (Item::Integer(left), Item::Integer(right)) => left.cmp(right),
        (Item::Text(left), Item::Text(right)) => left.cmp(right),
        (Item::Floating(left), Item::Floating(right)) => {
            let (left, right) = (f64::from_bits(*left), f64::from_bits(*right));
            match (left.is_nan(), right.is_nan()) {
                (true, true) => return Ordering::Equal,
                (true, false) => return Ordering::Greater,
                (false, true) => return Ordering::Less,
                (false, false) => left.partial_cmp(&right).unwrap(),
            }
        }
        _ => unreachable!("typed list contains only one element kind"),
    };
    if descending { order.reverse() } else { order }
}

// Stable bottom-up mergesort over indices. Scratch is two O(n) descriptor
// vectors; text payloads are compared in place and never copied by the sort.
// Invert the sorted permutation into scratch, then move items with O(n) swaps.
fn sort_items(items: &mut [Item], descending: bool) -> Result<(), String> {
    if items.len() < 2 {
        return Ok(());
    }
    let mut order = Vec::new();
    let mut scratch = Vec::new();
    for buffer in [&mut order, &mut scratch] {
        buffer
            .try_reserve_exact(items.len())
            .map_err(|_| limit("list sort scratch allocation failed"))?;
    }
    order.extend(0..items.len());
    scratch.resize(items.len(), 0);
    let mut width = 1;
    while width < items.len() {
        for start in (0..items.len()).step_by(width * 2) {
            let middle = (start + width).min(items.len());
            let end = (start + width * 2).min(items.len());
            let (mut left, mut right) = (start, middle);
            for target in &mut scratch[start..end] {
                if left < middle
                    && (right == end
                        || compare_items(&items[order[left]], &items[order[right]], descending)
                            != std::cmp::Ordering::Greater)
                {
                    *target = order[left];
                    left += 1;
                } else {
                    *target = order[right];
                    right += 1;
                }
            }
        }
        std::mem::swap(&mut order, &mut scratch);
        width *= 2;
    }
    for (destination, source) in order.into_iter().enumerate() {
        scratch[source] = destination;
    }
    for index in 0..items.len() {
        while scratch[index] != index {
            let other = scratch[index];
            items.swap(index, other);
            scratch.swap(index, other);
        }
    }
    Ok(())
}
pub(super) fn execute(
    instruction: &AilBytecodeInstruction,
    state: &mut BTreeMap<String, String>,
    text_allocated: &mut usize,
    list_allocated: &mut usize,
) -> Result<(), String> {
    verify(instruction)?;
    let operand = |name: &str| instruction.operands[name].as_str();
    let element = operand("type");
    let opcode = instruction.opcode.as_str();
    let mut items = if opcode == "LIST_EMPTY" {
        Vec::new()
    } else {
        let source = state
            .get(operand("source"))
            .ok_or_else(|| runtime_error(format!("missing list field {}", operand("source"))))?;
        parse_list(source, element)?
    };
    match opcode {
        "LIST_CONCAT" => {
            let source = state
                .get(operand("other"))
                .ok_or_else(|| runtime_error("missing second list field"))?;
            let mut other = parse_list(source, element)?;
            if items.len() + other.len() > MAX_ITEMS {
                return Err(limit("list result exceeds 65536 items"));
            }
            items
                .try_reserve_exact(other.len())
                .map_err(|_| limit("list concat scratch allocation failed"))?;
            items.append(&mut other);
        }
        "LIST_SLICE" => {
            let start = IntExpr::from_postfix(operand("start"))?.eval(state)?;
            let end = IntExpr::from_postfix(operand("end"))?.eval(state)?;
            let start = slice_bound(start, items.len());
            let end = slice_bound(end, items.len()).max(start);
            items.truncate(end);
            items.drain(..start);
        }
        "LIST_REVERSE" => items.reverse(),
        "LIST_SORT" => sort_items(&mut items, operand("order") == "descending")?,
        _ => {}
    }
    let index = if matches!(opcode, "LIST_GET" | "LIST_SET") {
        Some(list_index(
            IntExpr::from_postfix(operand("index"))?.eval(state)?,
            items.len(),
        )?)
    } else {
        None
    };
    if matches!(opcode, "LIST_APPEND" | "LIST_SET") {
        let value = if element == "Int" {
            Item::Integer(IntExpr::from_postfix(operand("value"))?.eval(state)?)
        } else if element == "Float" {
            Item::Floating(FloatExpr::from_postfix(operand("value"))?.eval(state)?)
        } else {
            Item::Text(text_operations::read_text(state, operand("value"))?.to_string())
        };
        if let Some(index) = index {
            items[index] = value;
        } else {
            items.push(value);
        }
    }
    let result = match opcode {
        "LIST_LENGTH" => items.len().to_string(),
        "LIST_GET" => match &items[index.unwrap()] {
            Item::Integer(n) => n.to_string(),
            Item::Floating(bits) => crate::float_expression::format_bits(*bits),
            Item::Text(text) => {
                text_operations::reserve_result(text.len(), text_allocated)?;
                text.clone()
            }
        },
        _ => {
            let result = canonical(&items)?;
            let total = list_allocated
                .checked_add(result.len())
                .filter(|total| *total <= MAX_ALLOCATED)
                .ok_or_else(|| limit("list allocation budget exceeded"))?;
            *list_allocated = total;
            result
        }
    };
    state.insert(operand("key").into(), result);
    Ok(())
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let mut result = Vec::new();
    for step in core.graph.edges.iter().filter(|e| e.kind == "list_step") {
        let mut expressions = Vec::new();
        for edge in core.graph.edges.iter().filter(|e| e.source == step.target) {
            match (kind, edge.kind.as_str()) {
                ("reads", "list_source" | "list_other") | ("writes", "list_destination") => {
                    result.push((edge, step.source.as_str()))
                }
                ("reads", "list_index" | "list_start" | "list_end") => {
                    expressions.push(edge.target.as_str())
                }
                ("reads", "list_value") => {
                    if core
                        .graph
                        .nodes
                        .iter()
                        .any(|n| n.id == edge.target && n.kind == "Field")
                    {
                        result.push((edge, step.source.as_str()));
                    } else {
                        expressions.push(edge.target.as_str());
                    }
                }
                _ => {}
            }
        }
        let mut visited = BTreeSet::new();
        while let Some(id) = expressions.pop() {
            if !visited.insert(id) {
                continue;
            }
            for edge in core.graph.edges.iter().filter(|e| e.source == id) {
                match edge.kind.as_str() {
                    "expression_field" => result.push((edge, step.source.as_str())),
                    "left_operand" | "right_operand" | "unary_operand" => {
                        expressions.push(edge.target.as_str())
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
    fn transformations_recognize_source_and_preserve_explicit_sort_direction() {
        for source in [
            "the system concatenates list data.left and list data.right into data.result",
            "the system slices list data.values from data.start to -1 into data.result",
            "the system reverses list data.values into data.result",
            "the system sorts list data.values in ascending order into data.result",
            "the system sorts list data.values in descending order into data.result",
        ] {
            let operation = parse_bullet(source)
                .expect("recognized list transformation")
                .unwrap();
            assert_eq!(operation.source(), source);
        }
    }

    #[test]
    fn list_integer_core_rejects_hidden_metadata_and_shared_or_foreign_parents() {
        let original = core_fixture();
        for mutation in 0..3 {
            let mut core = original.clone();
            let id = core
                .graph
                .nodes
                .iter()
                .find(|n| n.kind == "IntegerExpression")
                .unwrap()
                .id
                .clone();
            match mutation {
                0 => {
                    core.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == id)
                        .unwrap()
                        .attributes
                        .insert("hidden".into(), "true".into());
                }
                1 => {
                    core.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == id)
                        .unwrap()
                        .attributes
                        .insert("function_owner".into(), "Foreign".into());
                }
                _ => {
                    let mut edge = core
                        .graph
                        .edges
                        .iter()
                        .find(|e| e.target == id)
                        .unwrap()
                        .clone();
                    edge.kind = "reads".into();
                    core.graph.edges.push(edge);
                }
            }
            assert!(
                !check(&core).is_empty(),
                "accepted integer expression mutation {mutation}"
            );
        }
    }

    #[test]
    fn stable_float_sort_keeps_nan_last_and_preserves_signed_zero_order_both_ways() {
        for (order, expected) in [
            ("ascending", "[-inf,-2e0,-0e0,0e0,-0e0,0e0,2e0,inf,nan,nan]"),
            (
                "descending",
                "[inf,2e0,-0e0,0e0,-0e0,0e0,-2e0,-inf,nan,nan]",
            ),
        ] {
            let mut state = BTreeMap::from([(
                "input.values".into(),
                "[nan,-0,2,0,-inf,-0,inf,-2,nan,0]".into(),
            )]);
            let (mut text, mut lists) = (17, 19);
            execute(
                &command(
                    "LIST_SORT",
                    "Float",
                    &[("source", "input.values"), ("order", order)],
                ),
                &mut state,
                &mut text,
                &mut lists,
            )
            .unwrap();
            assert_eq!(state["output.value"], expected);
            assert_eq!((text, lists), (17, 19 + expected.len()));
        }
    }

    #[test]
    fn transformations_preserve_all_element_types_and_operand_aliases() {
        for (element, first, other, joined, reversed, sliced) in [
            ("Int", "[3,-2,0]", "[7]", "[3,-2,0,7]", "[0,-2,3]", "[-2,0]"),
            (
                "Float",
                "[-0,nan,+001]",
                "[-inf]",
                "[-0e0,nan,1e0,-inf]",
                "[1e0,nan,-0e0]",
                "[nan,1e0]",
            ),
            (
                "Text",
                "[\"é\",\"\\u0000\",\"é\"]",
                "[\"🦀\"]",
                "[\"é\",\"\\u0000\",\"é\",\"🦀\"]",
                "[\"é\",\"\\u0000\",\"é\"]",
                "[\"\\u0000\",\"é\"]",
            ),
        ] {
            for (opcode, arguments, expected) in [
                ("LIST_CONCAT", vec![("other", "input.other")], joined),
                ("LIST_REVERSE", vec![], reversed),
                (
                    "LIST_SLICE",
                    vec![("start", "i:1"), ("end", "i:999")],
                    sliced,
                ),
            ] {
                for key in ["input.values", "input.other", "output.value"] {
                    let mut state = BTreeMap::from([
                        ("input.values".into(), first.into()),
                        ("input.other".into(), other.into()),
                    ]);
                    let mut operands = vec![("source", "input.values"), ("key", key)];
                    operands.extend(arguments.clone());
                    let (mut text, mut lists) = (MAX_ALLOCATED, 7);
                    execute(
                        &command(opcode, element, &operands),
                        &mut state,
                        &mut text,
                        &mut lists,
                    )
                    .unwrap();
                    assert_eq!(state[key], expected, "{element} {opcode} {key}");
                    assert_eq!((text, lists), (MAX_ALLOCATED, 7 + expected.len()));
                    if key != "input.values" {
                        assert_eq!(state["input.values"], first);
                    }
                    if key != "input.other" {
                        assert_eq!(state["input.other"], other);
                    }
                }
            }
        }
    }

    #[test]
    fn numeric_and_unicode_sorting_is_directional_without_normalization() {
        for (element, input, order, expected) in [
            (
                "Int",
                "[9223372036854775807,0,-9223372036854775808,-1,1]",
                "ascending",
                "[-9223372036854775808,-1,0,1,9223372036854775807]",
            ),
            (
                "Int",
                "[9223372036854775807,0,-9223372036854775808,-1,1]",
                "descending",
                "[9223372036854775807,1,0,-1,-9223372036854775808]",
            ),
            (
                "Text",
                "[\"é\",\"é\",\"e\",\"🦀\",\"\",\"\\u0000\",\"é\"]",
                "ascending",
                "[\"\",\"\\u0000\",\"e\",\"é\",\"é\",\"é\",\"🦀\"]",
            ),
            (
                "Text",
                "[\"é\",\"é\",\"e\",\"🦀\",\"\",\"\\u0000\",\"é\"]",
                "descending",
                "[\"🦀\",\"é\",\"é\",\"é\",\"e\",\"\\u0000\",\"\"]",
            ),
        ] {
            let mut state = BTreeMap::from([("input.values".into(), input.into())]);
            execute(
                &command(
                    "LIST_SORT",
                    element,
                    &[("source", "input.values"), ("order", order)],
                ),
                &mut state,
                &mut 0,
                &mut 0,
            )
            .unwrap();
            assert_eq!(state["output.value"], expected);
        }
    }

    #[test]
    fn list_slices_clamp_extremes_and_evaluate_source_then_both_bounds() {
        for (start, end, expected) in [
            (i64::MIN, i64::MAX, "[1,2,3]"),
            (-2, i64::MAX, "[2,3]"),
            (0, -1, "[1,2]"),
            (i64::MAX, i64::MIN, "[]"),
            (-99, -98, "[]"),
            (2, 1, "[]"),
        ] {
            let mut state = BTreeMap::from([("input.values".into(), "[1,2,3]".into())]);
            let (mut text, mut lists) = (11, 13);
            execute(
                &command(
                    "LIST_SLICE",
                    "Int",
                    &[
                        ("source", "input.values"),
                        ("start", &format!("i:{start}")),
                        ("end", &format!("i:{end}")),
                    ],
                ),
                &mut state,
                &mut text,
                &mut lists,
            )
            .unwrap();
            assert_eq!(state["output.value"], expected);
            assert_eq!((text, lists), (11, 13 + expected.len()));
        }
        for (input, start, end, family) in [
            (
                "[1,2,invalid]",
                "i:1 i:0 /",
                "f:input.missing",
                "AIL-RUNTIME-LIST:",
            ),
            ("[]", "i:1 i:0 /", "f:input.missing", "AIL-RUNTIME-DIVZERO:"),
            ("[]", "f:input.missing", "i:1 i:0 /", "AIL-RUNTIME-INT:"),
            (
                "[]",
                "i:9223372036854775807",
                "i:1 i:0 /",
                "AIL-RUNTIME-DIVZERO:",
            ),
            (
                "[1]",
                "i:9223372036854775807 i:1 +",
                "i:1 i:0 /",
                "AIL-RUNTIME-OVERFLOW:",
            ),
        ] {
            let mut state = BTreeMap::from([
                ("input.values".into(), input.into()),
                ("output.value".into(), "preserved".into()),
            ]);
            let (mut text, mut lists) = (MAX_ALLOCATED, MAX_ALLOCATED);
            let error = execute(
                &command(
                    "LIST_SLICE",
                    "Int",
                    &[("source", "input.values"), ("start", start), ("end", end)],
                ),
                &mut state,
                &mut text,
                &mut lists,
            )
            .unwrap_err();
            assert!(error.starts_with(family), "{error}, expected {family}");
            assert_eq!(state["output.value"], "preserved");
            assert_eq!((text, lists), (MAX_ALLOCATED, MAX_ALLOCATED));
        }
    }

    #[test]
    fn concat_validates_both_whole_lists_before_result_limits_without_input_leaks() {
        let maximum = format!("[{}]", vec!["0"; MAX_ITEMS].join(","));
        for (element, first, second, family) in [
            (
                "Int",
                maximum.as_str(),
                Some("[0,invalid]"),
                "AIL-RUNTIME-LIST:",
            ),
            ("Int", maximum.as_str(), None, "AIL-RUNTIME-LIST:"),
            (
                "Int",
                maximum.as_str(),
                Some("[0]"),
                "AIL-RUNTIME-LIST-LIMIT:",
            ),
            ("Text", "[]", Some("[\"\\Z\"]"), "AIL-RUNTIME-LIST:"),
        ] {
            let mut state = BTreeMap::from([
                ("input.values".into(), first.into()),
                ("output.value".into(), "preserved".into()),
            ]);
            if let Some(second) = second {
                state.insert("input.other".into(), second.into());
            }
            let (mut text, mut lists) = (17, 19);
            let error = execute(
                &command(
                    "LIST_CONCAT",
                    element,
                    &[("source", "input.values"), ("other", "input.other")],
                ),
                &mut state,
                &mut text,
                &mut lists,
            )
            .unwrap_err();
            assert!(error.starts_with(family), "{error}");
            assert!(!error.contains('Z'));
            assert_eq!(state["output.value"], "preserved");
            assert_eq!((text, lists), (17, 19));
        }
    }

    #[test]
    fn slices_borrow_canonical_overflow_inputs_while_whole_results_fail_atomically() {
        for (element, input, expected) in [
            (
                "Text",
                format!("[\"{}\",\"ok\"]", "\u{85}".repeat(200_000)),
                "[\"ok\"]",
            ),
            (
                "Float",
                format!("[{}]", vec!["12345678901234"; MAX_ITEMS].join(",")),
                "[1.2345678901234e13]",
            ),
        ] {
            for opcode in ["LIST_SLICE", "LIST_REVERSE", "LIST_SORT", "LIST_CONCAT"] {
                let mut state = BTreeMap::from([
                    ("input.values".into(), input.clone()),
                    ("input.other".into(), "[]".into()),
                    ("output.value".into(), "preserved".into()),
                ]);
                let mut operands = vec![("source", "input.values")];
                match opcode {
                    "LIST_SLICE" => {
                        operands.extend([("start", "i:-1"), ("end", "i:9223372036854775807")])
                    }
                    "LIST_SORT" => operands.push(("order", "ascending")),
                    "LIST_CONCAT" => operands.push(("other", "input.other")),
                    _ => {}
                }
                let (mut text, mut lists) = (MAX_ALLOCATED, 17);
                let result = execute(
                    &command(opcode, element, &operands),
                    &mut state,
                    &mut text,
                    &mut lists,
                );
                if opcode == "LIST_SLICE" {
                    result.unwrap();
                    assert_eq!(state["output.value"], expected);
                    assert_eq!(lists, 17 + expected.len());
                } else {
                    assert!(result.unwrap_err().starts_with("AIL-RUNTIME-LIST-LIMIT:"));
                    assert_eq!(state["output.value"], "preserved");
                    assert_eq!(lists, 17);
                }
                assert_eq!(text, MAX_ALLOCATED);
            }
        }
    }

    #[test]
    fn maximum_sorts_handle_permuted_numeric_and_common_prefix_text_inputs() {
        let permutation: Vec<_> = (0..MAX_ITEMS)
            .map(|i| (i * 40503 + 17) % MAX_ITEMS)
            .collect();
        for element in ["Int", "Text"] {
            let input = format!(
                "[{}]",
                permutation
                    .iter()
                    .map(|n| if element == "Int" {
                        n.to_string()
                    } else {
                        format!("\"prefix.{n:05}\"")
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let mut state = BTreeMap::from([("input.values".into(), input)]);
            let started = std::time::Instant::now();
            execute(
                &command(
                    "LIST_SORT",
                    element,
                    &[("source", "input.values"), ("order", "ascending")],
                ),
                &mut state,
                &mut 0,
                &mut 0,
            )
            .unwrap();
            let sorted = parse_list(&state["output.value"], element).unwrap();
            assert_eq!(sorted.len(), MAX_ITEMS);
            for (index, item) in sorted.iter().enumerate() {
                match item {
                    Item::Integer(value) => assert_eq!(*value, index as i64),
                    Item::Text(value) => assert_eq!(value, &format!("prefix.{index:05}")),
                    _ => unreachable!(),
                }
            }
            assert!(started.elapsed() < std::time::Duration::from_secs(10));
        }
    }

    #[test]
    fn transformation_empty_and_unchanged_results_charge_one_list_copy() {
        for (opcode, operands) in [
            ("LIST_CONCAT", vec![("other", "input.other")]),
            ("LIST_SLICE", vec![("start", "i:0"), ("end", "i:99")]),
            ("LIST_REVERSE", vec![]),
            ("LIST_SORT", vec![("order", "descending")]),
        ] {
            for input in ["[]", "[0]"] {
                let mut state = BTreeMap::from([
                    ("input.values".into(), input.into()),
                    ("input.other".into(), "[]".into()),
                    ("output.value".into(), "preserved".into()),
                ]);
                let mut args = vec![("source", "input.values")];
                args.extend(operands.clone());
                let instruction = command(opcode, "Int", &args);
                let (mut text, mut lists) = (MAX_ALLOCATED, MAX_ALLOCATED - input.len());
                execute(&instruction, &mut state, &mut text, &mut lists).unwrap();
                assert_eq!(state["output.value"], input);
                assert_eq!((text, lists), (MAX_ALLOCATED, MAX_ALLOCATED));
                state.insert("output.value".into(), "preserved".into());
                assert!(
                    execute(&instruction, &mut state, &mut text, &mut lists)
                        .unwrap_err()
                        .starts_with("AIL-RUNTIME-LIST-LIMIT:")
                );
                assert_eq!(state["output.value"], "preserved");
                assert_eq!((text, lists), (MAX_ALLOCATED, MAX_ALLOCATED));
            }
        }
    }

    fn transformation_core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R003-max/program.ail").unwrap();
        let source = "The application Lists manages lists.\n\nA Data has:\n- left: List<Int>\n- right: List<Int>\n- result: List<Int>\n- start: Int\n- end: Int\n\nAction: Run.\nWhen Run happens:\n- the system concatenates list data.left and list data.right into data.result\n- the system slices list data.result from data.start + 1 to data.end - 1 into data.result\n- the system reverses list data.result into data.result\n- the system sorts list data.result in descending order into data.result\n- the system records a trace event named Ordered\n";
        elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, source).unwrap(),
        )
    }

    #[test]
    fn transformation_core_checks_exact_roles_sort_metadata_and_numeric_scope() {
        let core = transformation_core_fixture();
        assert!(check(&core).is_empty(), "{:?}", check(&core));
        let action = core.graph.find_node("Action", "Run").unwrap();
        let original = from_core(&core, action);
        assert_eq!(original.len(), 4);
        let saved = parse_ail_core_text(&render_ail_core(&core)).unwrap();
        assert!(check(&saved).is_empty());
        assert_eq!(
            from_core(&saved, saved.graph.find_node("Action", "Run").unwrap()),
            original
        );
        for relation in [
            "list_source",
            "list_other",
            "list_start",
            "list_end",
            "list_destination",
            "list_step",
        ] {
            for duplicate in [false, true] {
                let mut bad = core.clone();
                let edge = bad
                    .graph
                    .edges
                    .iter_mut()
                    .find(|e| e.kind == relation)
                    .unwrap();
                if duplicate {
                    let copied = edge.clone();
                    bad.graph.edges.push(copied);
                } else {
                    edge.attributes.insert("ignored".into(), "true".into());
                }
                assert!(
                    !check(&parse_ail_core_text(&render_ail_core(&bad)).unwrap()).is_empty(),
                    "accepted {relation} duplicate={duplicate}"
                );
            }
        }
        for sort_order in [None, Some("reverse"), Some("Ascending")] {
            let mut bad = core.clone();
            let node = bad
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.attributes.get("list_kind").is_some_and(|k| k == "sort"))
                .unwrap();
            if let Some(order) = sort_order {
                node.attributes.insert("sort_order".into(), order.into());
            } else {
                node.attributes.remove("sort_order");
            }
            assert!(!check(&bad).is_empty());
        }
        let mut bad = core.clone();
        let node = bad
            .graph
            .nodes
            .iter_mut()
            .find(|n| {
                n.attributes
                    .get("list_kind")
                    .is_some_and(|k| k == "reverse")
            })
            .unwrap();
        node.attributes
            .insert("sort_order".into(), "ascending".into());
        assert!(!check(&bad).is_empty());
        let mut raw = core;
        raw.graph
            .nodes
            .iter_mut()
            .find(|n| {
                n.kind == "IntegerExpression"
                    && n.attributes
                        .get("expression_kind")
                        .is_some_and(|k| k == "literal")
            })
            .unwrap()
            .attributes
            .insert("value".into(), "0".repeat(65_536));
        assert!(!check(&raw).is_empty());
    }

    #[test]
    fn transformation_accesses_include_both_lists_and_both_bound_expressions() {
        for key in ["Data.left", "Data.right", "Data.start", "Data.end"] {
            let mut core = transformation_core_fixture();
            core.graph
                .edges
                .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
            core.graph
                .nodes
                .iter_mut()
                .find(|n| n.name == key)
                .unwrap()
                .attributes
                .insert("secret".into(), "true".into());
            assert!(
                check_ail_core(&core).iter().any(|d| d.contains("AIL005")),
                "missing secret read: {key}"
            );
        }
    }

    #[test]
    fn transformation_protocol_and_source_reject_ignored_order_and_malformed_forms() {
        for source in [
            "the system concatenates",
            "the system concatenates list data.a into data.b",
            "the system slices list data.a from 0 into data.b",
            "the system reverses list",
            "the system sorts list data.a into data.b",
            "the system sorts list data.a in reverse order into data.b",
        ] {
            assert!(parse_bullet(source).unwrap().is_err(), "accepted {source}");
        }
        for source in [
            "the system slices text data.a from 0 to 1 into data.b",
            "the system sorts_other data.a",
            "the system reverses_data",
            "the system slices lists data.a",
        ] {
            assert!(parse_bullet(source).is_none(), "claimed unrelated {source}");
        }
        let original = command(
            "LIST_SORT",
            "Int",
            &[("source", "input.values"), ("order", "ascending")],
        );
        for mutation in 0..4 {
            let mut instruction = original.clone();
            match mutation {
                0 => {
                    instruction.operands.remove("order");
                }
                1 => {
                    instruction
                        .operands
                        .insert("order".into(), "reverse".into());
                }
                2 => {
                    instruction
                        .operands
                        .insert("ignored".into(), "input.values".into());
                }
                _ => {
                    instruction.operands.insert("source".into(), "[]".into());
                }
            }
            assert!(verify(&instruction).is_err());
        }
        let original = parse_bullet(
            "the system slices list data.values from data.start to data.end into data.result",
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            namespace(&original, "Child").source(),
            "the system slices list child.data.values from child.data.start to child.data.end into child.data.result"
        );
    }

    #[test]
    fn text_codec_borrows_large_noncanonical_lists_but_checks_owned_results_atomically() {
        let input = format!("[\"{}\"]", "\u{85}".repeat(400_000));
        let items = parse_text_items(&input).unwrap();
        assert_eq!(items, vec!["\u{85}".repeat(400_000)]);
        let mut allocated = 17;
        assert!(
            make_text_list(&items, &mut allocated)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT:")
        );
        assert_eq!(allocated, 17);
        let items = vec!["\0é\"\\".into(), String::new()];
        let output = make_text_list(&items, &mut allocated).unwrap();
        assert_eq!(output, "[\"\\u0000é\\\"\\\\\",\"\"]");
        assert_eq!(allocated, 17 + output.len());
        allocated = MAX_ALLOCATED - output.len() + 1;
        let previous = allocated;
        assert!(
            make_text_list(&items, &mut allocated)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT:")
        );
        assert_eq!(allocated, previous);
        for invalid in [
            "[\"valid\",1]",
            "[\"valid\",\"\\ud800\"]",
            "[\"valid\"] extra",
        ] {
            assert!(
                parse_text_items(invalid)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-LIST:")
            );
        }
        let message = parse_text_items("[\"\\Z\"]").unwrap_err();
        assert!(
            !message.contains('Z'),
            "diagnostic exposed input content: {message}"
        );
    }

    fn command(opcode: &str, type_name: &str, operands: &[(&str, &str)]) -> AilBytecodeInstruction {
        let mut result = AilBytecodeInstruction::new(
            opcode,
            &[("type", type_name.into()), ("key", "output.value".into())],
        );
        for (key, value) in operands {
            result.operands.insert((*key).into(), (*value).into());
        }
        result
    }

    #[test]
    fn floating_lists_normalize_runtime_tokens_and_preserve_ieee_values() {
        assert_eq!(
            canonical_literal(
                " \t[+001,.5,1.,-0,5e-324,+nan,-nan,+inf,-inf,1e9999]\n",
                "Float"
            )
            .unwrap(),
            "[1e0,5e-1,1e0,-0e0,5e-324,nan,nan,inf,-inf,inf]"
        );
        for input in [
            "[1 2]",
            "[1e +2]",
            "[NaN]",
            "[Infinity]",
            "[true]",
            "[null]",
            "[\"1\"]",
            "[[1]]",
            "[{}]",
            "[1,]",
            "[,1]",
            "[1,,2]",
            "[1]x",
            "[1;2]",
            "[1\u{a0}]",
            "[1/*x*/]",
        ] {
            assert!(
                canonical_literal(input, "Float")
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-LIST:"),
                "accepted {input:?}"
            );
        }
    }

    #[test]
    fn floating_updates_evaluate_expressions_preserve_aliases_and_charge_only_lists() {
        let mut state = BTreeMap::from([
            ("input.values".into(), "[+001,-0,nan]".into()),
            ("input.value".into(), "2.5".into()),
        ]);
        let (mut text_budget, mut list_budget) = (17, 19);
        execute(
            &command(
                "LIST_APPEND",
                "Float",
                &[
                    ("source", "input.values"),
                    ("value", "f:input.value d:2 * neg"),
                ],
            ),
            &mut state,
            &mut text_budget,
            &mut list_budget,
        )
        .unwrap();
        assert_eq!(state["output.value"], "[1e0,-0e0,nan,-5e0]");
        assert_eq!(state["input.values"], "[+001,-0,nan]");
        assert_eq!(
            (text_budget, list_budget),
            (17, 19 + state["output.value"].len())
        );
        let before = list_budget;
        execute(
            &command(
                "LIST_GET",
                "Float",
                &[("source", "input.values"), ("index", "i:-2")],
            ),
            &mut state,
            &mut text_budget,
            &mut list_budget,
        )
        .unwrap();
        assert_eq!(state["output.value"], "-0e0");
        assert_eq!((text_budget, list_budget), (17, before));
        let mut set = command(
            "LIST_SET",
            "Float",
            &[
                ("source", "input.values"),
                ("index", "i:0"),
                ("value", "d:inf d:inf -"),
            ],
        );
        set.operands.insert("key".into(), "input.values".into());
        execute(&set, &mut state, &mut text_budget, &mut list_budget).unwrap();
        assert_eq!(state["input.values"], "[nan,-0e0,nan]");
    }

    #[test]
    fn floating_list_value_markers_round_trip_without_integer_coercion() {
        let source = "the system sets item -1 in list data.values to floating -input.value / .5 into data.values";
        let operation = parse_bullet(source).unwrap().unwrap();
        assert_eq!(
            parse_bullet(&operation.source()).unwrap().unwrap(),
            operation
        );
        assert!(
            parse_bullet("the system appends 1.5 to list data.values into data.values")
                .unwrap()
                .is_err()
        );
        assert!(
            verify(&command(
                "LIST_APPEND",
                "Float",
                &[("source", "data.values"), ("value", "i:1")]
            ))
            .is_err()
        );
    }

    #[test]
    fn floating_json_boundary_is_strict_finite_and_allows_full_flat_item_count() {
        assert_eq!(
            canonical_float_json("[-0,1.25e2,5e-324,-1e-9999]", true).unwrap(),
            "[-0e0,1.25e2,5e-324,-0e0]"
        );
        assert_eq!(
            canonical_float_json("[+001,.5,1.,-0]", false).unwrap(),
            "[1e0,5e-1,1e0,-0e0]"
        );
        for token in [
            "+1", "01", "-01", ".5", "1.", "inf", "-inf", "nan", "1e9999",
        ] {
            assert!(
                canonical_float_json(&format!("[{token}]"), true)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-LIST:"),
                "JSON accepted {token}"
            );
        }
        for token in ["inf", "+inf", "-inf", "nan", "+nan", "-nan", "1e9999"] {
            assert!(
                canonical_float_json(&format!("[0,{token}]"), false)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-LIST:"),
                "format accepted {token}"
            );
        }
        let full = format!("[{}]", vec!["0"; MAX_ITEMS].join(","));
        assert_eq!(
            canonical_float_json(&full, true)
                .unwrap()
                .matches(',')
                .count(),
            MAX_ITEMS - 1
        );
        let excess = format!("[{},0]", &full[1..full.len() - 1]);
        assert!(
            canonical_float_json(&excess, true)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT:")
        );
        let nonfinite_excess = format!("[nan,{}]", &full[1..full.len() - 1]);
        assert!(
            canonical_float_json(&nonfinite_excess, false)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT:")
        );
        assert!(
            canonical_float_json(&nonfinite_excess, true)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST:")
        );
        let expansion_with_nonfinite =
            format!("[{},nan]", vec!["1000000000000001"; 60_000].join(","));
        assert!(
            canonical_float_json(&expansion_with_nonfinite, false)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST:")
        );
    }

    #[test]
    fn floating_result_exact_byte_and_shared_allocation_limits_are_atomic() {
        let large = "1.7976931348623157e308";
        let exact = format!("[{},-0e0]", vec![large; 45_590].join(","));
        assert_eq!(exact.len(), MAX_BYTES);
        assert_eq!(canonical_literal(&exact, "Float").unwrap(), exact);
        let mut state = BTreeMap::from([
            ("input.values".into(), exact),
            ("output.value".into(), "untouched".into()),
        ]);
        let (mut text_budget, mut list_budget) = (7, MAX_ALLOCATED - MAX_BYTES);
        let replace = command(
            "LIST_SET",
            "Float",
            &[
                ("source", "input.values"),
                ("index", "i:-1"),
                ("value", "d:-0"),
            ],
        );
        execute(&replace, &mut state, &mut text_budget, &mut list_budget).unwrap();
        assert_eq!(state["output.value"].len(), MAX_BYTES);
        assert_eq!((text_budget, list_budget), (7, MAX_ALLOCATED));
        state.insert("output.value".into(), "untouched".into());
        assert!(
            execute(&replace, &mut state, &mut text_budget, &mut list_budget)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT:")
        );
        let oversized = command(
            "LIST_SET",
            "Float",
            &[
                ("source", "input.values"),
                ("index", "i:-1"),
                ("value", "d:1e308"),
            ],
        );
        let mut fresh_budget = 0;
        assert!(
            execute(&oversized, &mut state, &mut text_budget, &mut fresh_budget)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT:")
        );
        assert_eq!(state["output.value"], "untouched");
        assert_eq!(
            (text_budget, list_budget, fresh_budget),
            (7, MAX_ALLOCATED, 0)
        );
    }

    #[test]
    fn floating_list_failures_preserve_priority_destination_and_budgets() {
        for (source, index, value, family) in [
            (
                "[1,bad]",
                "i:1 i:0 /",
                "f:input.missing",
                "AIL-RUNTIME-LIST:",
            ),
            ("[1]", "i:1 i:0 /", "f:input.missing", "AIL-RUNTIME-DIVZERO"),
            ("[1]", "i:2", "f:input.missing", "AIL-RUNTIME-INDEX"),
            ("[1]", "i:0", "f:input.missing", "AIL-RUNTIME-FLOAT:"),
            ("[1]", "i:0", "d:1 d:-0 /", "AIL-RUNTIME-DIVZERO"),
        ] {
            let mut state = BTreeMap::from([
                ("input.values".into(), source.into()),
                ("output.value".into(), "untouched".into()),
            ]);
            let (mut text_budget, mut list_budget) = (13, 29);
            let instruction = command(
                "LIST_SET",
                "Float",
                &[
                    ("source", "input.values"),
                    ("index", index),
                    ("value", value),
                ],
            );
            assert!(
                execute(&instruction, &mut state, &mut text_budget, &mut list_budget)
                    .unwrap_err()
                    .starts_with(family)
            );
            assert_eq!(state["output.value"], "untouched");
            assert_eq!((text_budget, list_budget), (13, 29));
        }
        let full = format!("[{}]", vec!["0"; MAX_ITEMS].join(","));
        let mut state = BTreeMap::from([("input.values".into(), full)]);
        let invalid = command(
            "LIST_APPEND",
            "Float",
            &[("source", "input.values"), ("value", "f:input.missing")],
        );
        assert!(
            execute(&invalid, &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-FLOAT:")
        );
        let valid = command(
            "LIST_APPEND",
            "Float",
            &[("source", "input.values"), ("value", "d:1")],
        );
        assert!(
            execute(&valid, &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT:")
        );
    }

    #[test]
    fn floating_borrow_and_get_allow_canonical_expansion_without_arena_charges() {
        // Each short input expands when normalized to scientific notation.
        let input = format!("[{}]", vec!["1000000000000001"; 60_000].join(","));
        assert!(input.len() < MAX_BYTES);
        validate_runtime_value(&input, "Float").unwrap();
        assert!(
            canonical_literal(&input, "Float")
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT:")
        );
        let mut state = BTreeMap::from([("input.values".into(), input)]);
        let (mut text_budget, mut list_budget) = (MAX_ALLOCATED, MAX_ALLOCATED);
        execute(
            &command(
                "LIST_GET",
                "Float",
                &[("source", "input.values"), ("index", "i:0")],
            ),
            &mut state,
            &mut text_budget,
            &mut list_budget,
        )
        .unwrap();
        assert_eq!(state["output.value"], "1.000000000000001e15");
        execute(
            &command("LIST_LENGTH", "Float", &[("source", "input.values")]),
            &mut state,
            &mut text_budget,
            &mut list_budget,
        )
        .unwrap();
        assert_eq!(state["output.value"], "60000");
        assert_eq!((text_budget, list_budget), (MAX_ALLOCATED, MAX_ALLOCATED));
        assert!(
            execute(
                &command("LIST_EMPTY", "Float", &[]),
                &mut state,
                &mut text_budget,
                &mut list_budget
            )
            .unwrap_err()
            .starts_with("AIL-RUNTIME-LIST-LIMIT:")
        );
        assert_eq!(state["output.value"], "60000");
        let exact = format!("[0]{}", " ".repeat(MAX_BYTES - 3));
        validate_runtime_value(&exact, "Float").unwrap();
        assert!(
            validate_runtime_value(&(exact + " "), "Float")
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT:")
        );
    }

    #[test]
    fn integer_lists_validate_all_items_and_canonicalize_updates() {
        let mut state = BTreeMap::from([(
            "input.values".into(),
            " [ -0, 2, -9223372036854775808 ] ".into(),
        )]);
        let mut text_budget = 0;
        let mut list_budget = 0;
        execute(
            &command(
                "LIST_GET",
                "Int",
                &[("source", "input.values"), ("index", "i:-1")],
            ),
            &mut state,
            &mut text_budget,
            &mut list_budget,
        )
        .unwrap();
        assert_eq!(state["output.value"], "-9223372036854775808");
        execute(
            &command(
                "LIST_APPEND",
                "Int",
                &[("source", "input.values"), ("value", "i:3 i:4 +")],
            ),
            &mut state,
            &mut text_budget,
            &mut list_budget,
        )
        .unwrap();
        assert_eq!(state["output.value"], "[0,2,-9223372036854775808,7]");
        assert_eq!(list_budget, 28);
        assert_eq!(text_budget, 0);
    }

    #[test]
    fn text_lists_decode_surrogates_and_emit_canonical_unicode_escapes() {
        let mut state = BTreeMap::from([
            (
                "input.values".into(),
                r#"["\ud83e\udd80","\b\f\u0085","é"]"#.into(),
            ),
            ("input.text".into(), "\t\n".into()),
        ]);
        let mut text_budget = 0;
        let mut list_budget = 0;
        execute(
            &command(
                "LIST_GET",
                "Text",
                &[("source", "input.values"), ("index", "i:0")],
            ),
            &mut state,
            &mut text_budget,
            &mut list_budget,
        )
        .unwrap();
        assert_eq!(state["output.value"], "🦀");
        assert_eq!(text_budget, 4);
        execute(
            &command(
                "LIST_APPEND",
                "Text",
                &[("source", "input.values"), ("value", "input.text")],
            ),
            &mut state,
            &mut text_budget,
            &mut list_budget,
        )
        .unwrap();
        assert_eq!(
            state["output.value"],
            "[\"🦀\",\"\\u0008\\u000c\\u0085\",\"é\",\"\\t\\n\"]"
        );
        assert_eq!(text_budget, 4);
    }

    #[test]
    fn flat_parser_rejects_non_json_numbers_mixed_values_and_recursive_containers() {
        for input in [
            "",
            "null",
            "[+1]",
            "[01]",
            "[-01]",
            "[1.0]",
            "[1e0]",
            "[9223372036854775808]",
            "[-9223372036854775809]",
            "[1,]",
            "[,1]",
            "[1,,2]",
            "[true]",
            "[null]",
            "[{}]",
            "[[]]",
            "[\"1\"]",
            "[1,\"bad\"]",
            "[1] trailing",
            "\u{a0}[1]",
            "[1,\u{a0}2]",
        ] {
            assert!(parse_list(input, "Int").is_err(), "accepted {input:?}");
        }
        for input in [
            "[1]",
            "[false]",
            "[[]]",
            "[{}]",
            "[\"x\",]",
            "[\"\\q\"]",
            "[\"\\ud800\"]",
            "[\"\\udc00\"]",
            "[\"\\ud800\\ud800\"]",
            "[\"line\nfeed\"]",
        ] {
            assert!(parse_list(input, "Text").is_err(), "accepted {input:?}");
        }
        assert!(parse_list(&"[".repeat(10_000), "Int").is_err());
        assert_eq!(
            parse_list(" \t\r\n[0,-0,-1,9223372036854775807]\n", "Int").unwrap(),
            vec![
                Item::Integer(0),
                Item::Integer(0),
                Item::Integer(-1),
                Item::Integer(i64::MAX)
            ]
        );
    }

    #[test]
    fn source_validation_and_index_range_precede_item_evaluation_atomically() {
        for (input, index, value, expected) in [
            ("[1,\"bad\"]", "i:0", "i:1 i:0 /", "AIL-RUNTIME-LIST:"),
            ("[1]", "i:1 i:0 /", "i:3", "AIL-RUNTIME-DIVZERO"),
            (
                "[1]",
                "i:-9223372036854775808",
                "i:1 i:0 /",
                "AIL-RUNTIME-INDEX",
            ),
            ("[]", "i:0", "i:1 i:0 /", "AIL-RUNTIME-INDEX"),
            ("[1]", "i:-1", "i:1 i:0 /", "AIL-RUNTIME-DIVZERO"),
        ] {
            let instruction = command(
                "LIST_SET",
                "Int",
                &[
                    ("source", "input.values"),
                    ("index", index),
                    ("value", value),
                ],
            );
            let mut state = BTreeMap::from([
                ("input.values".into(), input.into()),
                ("output.value".into(), "preserved".into()),
            ]);
            let (mut text_budget, mut list_budget) = (7, 11);
            assert!(
                execute(&instruction, &mut state, &mut text_budget, &mut list_budget)
                    .unwrap_err()
                    .starts_with(expected)
            );
            assert_eq!(state["output.value"], "preserved");
            assert_eq!((text_budget, list_budget), (7, 11));
        }
    }

    #[test]
    fn list_and_text_allocation_budgets_are_separate_and_fail_atomically() {
        let mut state = BTreeMap::from([
            ("input.values".into(), "[\"\"]".into()),
            ("output.value".into(), "preserved".into()),
        ]);
        let (mut text_budget, mut list_budget) = (67_108_863, 67_108_862);
        execute(
            &command("LIST_EMPTY", "Text", &[]),
            &mut state,
            &mut text_budget,
            &mut list_budget,
        )
        .unwrap();
        assert_eq!(state["output.value"], "[]");
        assert_eq!((text_budget, list_budget), (67_108_863, 67_108_864));
        let get = command(
            "LIST_GET",
            "Text",
            &[("source", "input.values"), ("index", "i:0")],
        );
        execute(&get, &mut state, &mut text_budget, &mut list_budget).unwrap();
        assert_eq!(state["output.value"], "");
        assert_eq!((text_budget, list_budget), (67_108_864, 67_108_864));
        state.insert("output.value".into(), "preserved".into());
        assert!(
            execute(&get, &mut state, &mut text_budget, &mut list_budget)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT-LIMIT")
        );
        assert!(
            execute(
                &command("LIST_EMPTY", "Int", &[]),
                &mut state,
                &mut text_budget,
                &mut list_budget
            )
            .unwrap_err()
            .starts_with("AIL-RUNTIME-LIST-LIMIT")
        );
        assert_eq!(state["output.value"], "preserved");
        assert_eq!((text_budget, list_budget), (67_108_864, 67_108_864));
    }

    #[test]
    fn list_item_and_encoded_byte_limits_cover_inputs_and_canonical_results() {
        let full = format!("[{}]", vec!["0"; 65_536].join(","));
        assert_eq!(parse_list(&full, "Int").unwrap().len(), 65_536);
        let mut state = BTreeMap::from([
            ("input.values".into(), full),
            ("output.value".into(), "preserved".into()),
        ]);
        let invalid = command(
            "LIST_APPEND",
            "Int",
            &[("source", "input.values"), ("value", "i:1 i:0 /")],
        );
        assert!(
            execute(&invalid, &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-DIVZERO")
        );
        let valid = command(
            "LIST_APPEND",
            "Int",
            &[("source", "input.values"), ("value", "i:1")],
        );
        assert!(
            execute(&valid, &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT")
        );
        assert_eq!(state["output.value"], "preserved");
        let maximum = format!("[\"{}\"]", "x".repeat(1_048_572));
        assert!(parse_list(&maximum, "Text").is_ok());
        assert!(
            parse_list(&(maximum + " "), "Text")
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT")
        );
        assert!(
            canonical(&[Item::Text("\u{85}".repeat(200_000))])
                .unwrap_err()
                .starts_with("AIL-RUNTIME-LIST-LIMIT")
        );
    }

    #[test]
    fn read_only_operations_accept_valid_inputs_whose_canonical_form_exceeds_result_limit() {
        let source = format!("[\"{}\"]", "\u{85}".repeat(200_000));
        let mut state = BTreeMap::from([("input.values".into(), source.clone())]);
        let (mut text_budget, mut list_budget) = (0, 0);
        execute(
            &command("LIST_LENGTH", "Text", &[("source", "input.values")]),
            &mut state,
            &mut text_budget,
            &mut list_budget,
        )
        .unwrap();
        assert_eq!(state["output.value"], "1");
        execute(
            &command(
                "LIST_GET",
                "Text",
                &[("source", "input.values"), ("index", "i:0")],
            ),
            &mut state,
            &mut text_budget,
            &mut list_budget,
        )
        .unwrap();
        assert_eq!(state["output.value"].len(), 400_000);
        assert_eq!(state["input.values"], source);
        assert_eq!((text_budget, list_budget), (400_000, 0));
        let append = command(
            "LIST_APPEND",
            "Text",
            &[("source", "input.values"), ("value", "input.missing")],
        );
        assert!(
            execute(&append, &mut state, &mut text_budget, &mut list_budget)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT:")
        );
    }

    fn core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Lists manages lists.\n\nA Data has:\n- numbers: List<Int>\n- names: List<Text>\n\nAn Input has:\n- value: Int\n- index: Int\n- text: Text\n\nAn Output has:\n- value: Int\n\nAction: Run.\n\nWhen Run happens:\n- the system appends integer input.value + 1 to list data.numbers into data.numbers\n- the system gets item input.index from list data.numbers into output.value\n- the system appends text input.text to list data.names into data.names\n- the system records a trace event named Processed\n";
        elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, source).unwrap(),
        )
    }

    fn floating_core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Lists manages lists.\n\nA Data has:\n- values: List<Float>\n- value: Float\n- index: Int\n\nAction: Run.\n\nWhen Run happens:\n- the system appends floating -data.value + .5 to list data.values into data.values\n- the system gets item data.index from list data.values into data.value\n";
        elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, source).unwrap(),
        )
    }

    #[test]
    fn floating_core_checks_typed_owned_expression_trees_and_unary_secret_reads() {
        let original = floating_core_fixture();
        assert!(check(&original).is_empty(), "{:?}", check(&original));
        assert!(
            float_operations::check(&original).is_empty(),
            "{:?}",
            float_operations::check(&original)
        );
        let loaded = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        assert_eq!(
            from_core(
                &original,
                original.graph.find_node("Action", "Run").unwrap()
            ),
            from_core(&loaded, loaded.graph.find_node("Action", "Run").unwrap())
        );
        for kind in [
            "list_step",
            "list_source",
            "list_destination",
            "list_value",
            "list_index",
            "unary_operand",
        ] {
            let mut core = original.clone();
            core.graph.edges.push(
                core.graph
                    .edges
                    .iter()
                    .find(|e| e.kind == kind)
                    .unwrap()
                    .clone(),
            );
            let saved = parse_ail_core_text(&render_ail_core(&core)).unwrap();
            assert!(
                !check(&saved).is_empty() || !float_operations::check(&saved).is_empty(),
                "duplicate {kind}"
            );
        }
        for kind in [
            "list_step",
            "list_source",
            "list_destination",
            "list_value",
            "list_index",
        ] {
            let mut core = original.clone();
            core.graph
                .edges
                .iter_mut()
                .find(|e| e.kind == kind)
                .unwrap()
                .attributes
                .insert("ignored_semantics".into(), "1".into());
            let saved = parse_ail_core_text(&render_ail_core(&core)).unwrap();
            assert!(!check(&saved).is_empty(), "ignored {kind} metadata");
        }
        let mut typed = original.clone();
        typed
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "ListOperation")
            .unwrap()
            .type_name = Some("List<Float>".into());
        assert!(!check(&typed).is_empty());
        let mut core = original;
        core.graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        let float_field = core
            .graph
            .find_node("Field", "Data.value")
            .unwrap()
            .id
            .clone();
        assert!(
            accesses(&core, "reads")
                .iter()
                .any(|(edge, _)| edge.target == float_field)
        );
    }

    #[test]
    fn list_core_rejects_forged_global_storage_on_all_operand_roles() {
        let original = core_fixture();
        for kind in [
            "list_source",
            "list_destination",
            "expression_field",
            "list_value",
        ] {
            let mut core = original.clone();
            let edge_index = core
                .graph
                .edges
                .iter()
                .position(|e| {
                    e.kind == kind
                        && core
                            .graph
                            .nodes
                            .iter()
                            .any(|n| n.id == e.target && n.kind == "Field")
                })
                .unwrap();
            let mut forged = core
                .graph
                .nodes
                .iter()
                .find(|n| n.id == core.graph.edges[edge_index].target)
                .unwrap()
                .clone();
            forged.id.push_str("_forged");
            forged.attributes.remove("secret");
            core.graph.edges[edge_index].target = forged.id.clone();
            core.graph.nodes.push(forged);
            assert!(!check(&core).is_empty(), "accepted forged {kind}");
        }
    }

    #[test]
    fn typed_core_round_trip_rejects_duplicate_edges_and_inconsistent_element_types() {
        let original = core_fixture();
        assert!(check(&original).is_empty());
        let loaded = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        assert_eq!(
            from_core(
                &original,
                original.graph.find_node("Action", "Run").unwrap()
            ),
            from_core(&loaded, loaded.graph.find_node("Action", "Run").unwrap())
        );
        for kind in [
            "list_step",
            "list_source",
            "list_destination",
            "list_index",
            "list_value",
        ] {
            let mut core = original.clone();
            core.graph.edges.push(
                core.graph
                    .edges
                    .iter()
                    .find(|e| e.kind == kind)
                    .unwrap()
                    .clone(),
            );
            assert!(
                !check(&parse_ail_core_text(&render_ail_core(&core)).unwrap()).is_empty(),
                "duplicate {kind}"
            );
        }
        let mut core = original;
        core.graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "ListOperation")
            .unwrap()
            .attributes
            .insert("element_type".into(), "Text".into());
        assert!(!check(&core).is_empty());
    }

    #[test]
    fn list_accesses_include_text_and_integer_expression_reads_without_descriptive_edges() {
        let mut core = core_fixture();
        core.graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        let names = |kind| {
            accesses(&core, kind)
                .into_iter()
                .map(|(e, owner)| {
                    assert_eq!(owner, core.graph.find_node("Action", "Run").unwrap().id);
                    core.graph
                        .nodes
                        .iter()
                        .find(|n| n.id == e.target)
                        .unwrap()
                        .name
                        .clone()
                })
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(
            names("reads"),
            [
                "Data.numbers",
                "Data.names",
                "Input.value",
                "Input.index",
                "Input.text"
            ]
            .into_iter()
            .map(str::to_string)
            .collect()
        );
        assert_eq!(
            names("writes"),
            ["Data.numbers", "Data.names", "Output.value"]
                .into_iter()
                .map(str::to_string)
                .collect()
        );
    }
}
