//! Checked Unicode text operations and their typed Core representation.
use super::*;
use crate::expression::IntExpr;

const MAX_TEXT_BYTES: usize = 1_048_576;
const MAX_ALLOCATED_BYTES: usize = 67_108_864;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilTextOperation {
    Join {
        key: String,
        left: String,
        right: String,
    },
    Length {
        key: String,
        source: String,
    },
    Slice {
        key: String,
        source: String,
        start: IntExpr,
        end: IntExpr,
    },
    Find {
        key: String,
        source: String,
        needle: String,
    },
    Split {
        key: String,
        source: String,
        separator: String,
    },
    JoinList {
        key: String,
        source: String,
        separator: String,
    },
    Replace {
        key: String,
        source: String,
        needle: String,
        replacement: String,
    },
    Trim {
        key: String,
        source: String,
    },
    Invalid(String),
}

impl AilTextOperation {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Join { key, left, right } => {
                format!("the system joins text {left} and {right} into {key}")
            }
            Self::Length { key, source } => {
                format!("the system counts characters in {source} into {key}")
            }
            Self::Slice {
                key,
                source,
                start,
                end,
            } => format!(
                "the system slices text {source} from {} to {} into {key}",
                start.to_source(),
                end.to_source()
            ),
            Self::Find {
                key,
                source,
                needle,
            } => format!("the system finds text {needle} in {source} into {key}"),
            Self::Split {
                key,
                source,
                separator,
            } => format!("the system splits text {source} by {separator} into {key}"),
            Self::JoinList {
                key,
                source,
                separator,
            } => format!("the system joins list {source} with {separator} into {key}"),
            Self::Replace {
                key,
                source,
                needle,
                replacement,
            } => format!(
                "the system replaces text {needle} in {source} with {replacement} into {key}"
            ),
            Self::Trim { key, source } => format!("the system trims text {source} into {key}"),
            Self::Invalid(message) => format!("invalid text operation: {message}"),
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Join { .. } => "join",
            Self::Length { .. } => "length",
            Self::Slice { .. } => "slice",
            Self::Find { .. } => "find",
            Self::Split { .. } => "split",
            Self::JoinList { .. } => "join_list",
            Self::Replace { .. } => "replace",
            Self::Trim { .. } => "trim",
            Self::Invalid(_) => "invalid",
        }
    }

    fn fields(&self) -> Vec<(&str, &str, &str)> {
        match self {
            Self::Join { key, left, right } => vec![
                ("text_source", left, "Text"),
                ("text_other", right, "Text"),
                ("text_destination", key, "Text"),
            ],
            Self::Length { key, source } => vec![
                ("text_source", source, "Text"),
                ("text_destination", key, "Int"),
            ],
            Self::Slice { key, source, .. } => vec![
                ("text_source", source, "Text"),
                ("text_destination", key, "Text"),
            ],
            Self::Find {
                key,
                source,
                needle,
            } => vec![
                ("text_source", source, "Text"),
                ("text_other", needle, "Text"),
                ("text_destination", key, "Int"),
            ],
            Self::Split {
                key,
                source,
                separator,
            } => vec![
                ("text_source", source, "Text"),
                ("text_other", separator, "Text"),
                ("text_destination", key, "List<Text>"),
            ],
            Self::JoinList {
                key,
                source,
                separator,
            } => vec![
                ("text_source", source, "List<Text>"),
                ("text_other", separator, "Text"),
                ("text_destination", key, "Text"),
            ],
            Self::Replace {
                key,
                source,
                needle,
                replacement,
            } => vec![
                ("text_source", source, "Text"),
                ("text_other", needle, "Text"),
                ("text_replacement", replacement, "Text"),
                ("text_destination", key, "Text"),
            ],
            Self::Trim { key, source } => vec![
                ("text_source", source, "Text"),
                ("text_destination", key, "Text"),
            ],
            Self::Invalid(_) => Vec::new(),
        }
    }
}

fn error(message: impl Display) -> String {
    format!("AIL-TEXT-001: {message}")
}

fn field(key: &str) -> Result<(), String> {
    if matches!(IntExpr::parse(key), Ok(IntExpr::Field(parsed)) if parsed == key) {
        Ok(())
    } else {
        Err(error(format!("expected a field reference, got {key}")))
    }
}

pub(super) fn parse_bullet(text: &str) -> Option<Result<AilTextOperation, String>> {
    let text = trim_sentence(text);
    let (kind, rest) = if let Some(rest) = text.strip_prefix("the system joins text ") {
        ("join", rest)
    } else if let Some(rest) = text.strip_prefix("the system counts characters in ") {
        ("length", rest)
    } else if let Some(rest) = text.strip_prefix("the system slices text ") {
        ("slice", rest)
    } else if let Some(rest) = text.strip_prefix("the system finds text ") {
        ("find", rest)
    } else if let Some(rest) = text.strip_prefix("the system splits text ") {
        ("split", rest)
    } else if let Some(rest) = text.strip_prefix("the system joins list ") {
        ("join_list", rest)
    } else if let Some(rest) = text.strip_prefix("the system replaces text ") {
        ("replace", rest)
    } else if let Some(rest) = text.strip_prefix("the system trims text ") {
        ("trim", rest)
    } else {
        return [
            "the system joins",
            "the system counts characters",
            "the system slices",
            "the system finds",
            "the system splits",
            "the system replaces",
            "the system trims",
        ]
        .iter()
        .any(|prefix| {
            text == *prefix
                || text
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with(' '))
        })
        .then(|| Err(error("malformed text operation")));
    };
    Some((|| {
        let (operands, key) = rest
            .rsplit_once(" into ")
            .ok_or_else(|| error("text operation requires 'into field'"))?;
        let operation = match kind {
            "join" => {
                let (left, right) = operands
                    .split_once(" and ")
                    .ok_or_else(|| error("join requires two fields separated by 'and'"))?;
                AilTextOperation::Join {
                    key: key.into(),
                    left: left.into(),
                    right: right.into(),
                }
            }
            "length" => AilTextOperation::Length {
                key: key.into(),
                source: operands.into(),
            },
            "slice" => {
                let (source, bounds) = operands
                    .split_once(" from ")
                    .ok_or_else(|| error("slice requires 'from expression to expression'"))?;
                let (start, end) = bounds
                    .split_once(" to ")
                    .ok_or_else(|| error("slice requires an end bound"))?;
                AilTextOperation::Slice {
                    key: key.into(),
                    source: source.into(),
                    start: IntExpr::parse(start)?,
                    end: IntExpr::parse(end)?,
                }
            }
            "find" => {
                let (needle, source) = operands
                    .split_once(" in ")
                    .ok_or_else(|| error("find requires 'needle in source'"))?;
                AilTextOperation::Find {
                    key: key.into(),
                    source: source.into(),
                    needle: needle.into(),
                }
            }
            "split" | "join_list" => {
                let (source, separator) = operands
                    .split_once(if kind == "split" { " by " } else { " with " })
                    .ok_or_else(|| error("split or list join requires a separator field"))?;
                if kind == "split" {
                    AilTextOperation::Split {
                        key: key.into(),
                        source: source.into(),
                        separator: separator.into(),
                    }
                } else {
                    AilTextOperation::JoinList {
                        key: key.into(),
                        source: source.into(),
                        separator: separator.into(),
                    }
                }
            }
            "replace" => {
                let (needle, rest) = operands
                    .split_once(" in ")
                    .ok_or_else(|| error("replace requires 'needle in source'"))?;
                let (source, replacement) = rest
                    .split_once(" with ")
                    .ok_or_else(|| error("replace requires a replacement field"))?;
                AilTextOperation::Replace {
                    key: key.into(),
                    source: source.into(),
                    needle: needle.into(),
                    replacement: replacement.into(),
                }
            }
            "trim" => AilTextOperation::Trim {
                key: key.into(),
                source: operands.into(),
            },
            _ => unreachable!(),
        };
        for (_, key, _) in operation.fields() {
            field(key)?;
        }
        Ok(operation)
    })())
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, operation) in action.text_operations.iter().enumerate() {
            let name = format!("{}.text{index}", owner.name);
            let mut attrs = attr(&[("text_kind", operation.kind())]);
            if let AilTextOperation::Invalid(message) = operation {
                attrs.insert("error".into(), message.clone());
            }
            let node = graph.add_node("TextOperation", &name, None, attrs);
            attach_provenance(graph, &node, format!("action:{}.text:{index}", owner.name));
            for (kind, key, _) in operation.fields() {
                let target = graph
                    .nodes
                    .iter()
                    .find(|n| {
                        n.kind == "Field"
                            && !n.attributes.contains_key("function_owner")
                            && !n.attributes.contains_key("local_name")
                            && expression_core::field_key(n) == key
                    })
                    .cloned()
                    .unwrap_or_else(|| graph.add_node("Effect", key, None, BTreeMap::new()));
                attach_provenance(
                    graph,
                    &target,
                    format!("action:{}.text:{index}.{kind}", owner.name),
                );
                graph.add_edge(kind, &node, &target, BTreeMap::new());
            }
            if let AilTextOperation::Slice { start, end, .. } = operation {
                let start = expression_core::build(graph, &owner, &format!("{name}.start"), start);
                let end = expression_core::build(graph, &owner, &format!("{name}.end"), end);
                graph.add_edge("text_start", &node, &start, BTreeMap::new());
                graph.add_edge("text_end", &node, &end, BTreeMap::new());
            }
            action_order::append(graph, "text_step", &owner, &node, BTreeMap::new(), index);
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
        .ok_or_else(|| error(format!("{} requires a {kind} edge", node.name)))?;
    if edges.next().is_some() || !edge.attributes.is_empty() {
        return Err(error(format!("{} has duplicate {kind} edges", node.name)));
    }
    core.graph
        .nodes
        .iter()
        .find(|n| n.id == edge.target)
        .ok_or_else(|| error("missing text operand node"))
}

fn global_field(core: &AilCore, node: &Node, expected: &str) -> Result<String, String> {
    float_operations::scoped_field(core, node, None, expected).map_err(|_| {
        error(format!(
            "text operand {} must be a uniquely declared global {expected} field",
            node.name
        ))
    })
}

fn bound(core: &AilCore, root: &Node) -> Result<IntExpr, String> {
    let expression = value_function::scoped_expression(core, root, None).map_err(error)?;
    let mut pending = vec![root];
    let mut visited = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if !visited.insert(&node.id) {
            return Err(error("text slice bound expressions cannot be shared"));
        }
        let incoming: Vec<_> = core
            .graph
            .edges
            .iter()
            .filter(|e| e.target == node.id)
            .collect();
        if incoming.len() != 1
            || !incoming[0].attributes.is_empty()
            || !matches!(
                incoming[0].kind.as_str(),
                "text_start" | "text_end" | "left_operand" | "right_operand"
            )
        {
            return Err(error("text slice bound requires exactly one typed parent"));
        }
        for edge in core.graph.edges.iter().filter(|e| e.source == node.id) {
            match edge.kind.as_str() {
                "expression_field" => {
                    global_field(core, child(core, node, "expression_field")?, "Int")?;
                }
                "left_operand" | "right_operand" => pending.push(child(core, node, &edge.kind)?),
                "has_provenance" => {}
                _ => return Err(error("unexpected edge on text slice bound")),
            }
        }
    }
    Ok(expression)
}

fn decode(core: &AilCore, node: &Node) -> Result<AilTextOperation, String> {
    if node.kind != "TextOperation"
        || node.type_name.is_some()
        || node.attributes.len() != 1
        || !node.attributes.contains_key("text_kind")
    {
        return Err(error("text_step must target a TextOperation"));
    }
    let incoming: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect();
    if incoming.len() != 1
        || incoming[0].kind != "text_step"
        || !core
            .graph
            .nodes
            .iter()
            .any(|n| n.id == incoming[0].source && executable_action(n))
        || !valid_step_attributes(incoming[0])
    {
        return Err(error(
            "text operation requires exactly one executable action owner",
        ));
    }
    let kind = node
        .attributes
        .get("text_kind")
        .map(String::as_str)
        .unwrap_or("");
    let allowed: &[&str] = match kind {
        "join" | "find" | "split" | "join_list" => {
            &["text_source", "text_other", "text_destination"]
        }
        "length" | "trim" => &["text_source", "text_destination"],
        "replace" => &[
            "text_source",
            "text_other",
            "text_replacement",
            "text_destination",
        ],
        "slice" => &["text_source", "text_destination", "text_start", "text_end"],
        _ => return Err(error("invalid text operation kind")),
    };
    if core.graph.edges.iter().any(|e| {
        e.source == node.id && e.kind != "has_provenance" && !allowed.contains(&e.kind.as_str())
    }) {
        return Err(error("unexpected edge on text operation"));
    }
    let source = global_field(
        core,
        child(core, node, "text_source")?,
        if kind == "join_list" {
            "List<Text>"
        } else {
            "Text"
        },
    )?;
    let key = global_field(
        core,
        child(core, node, "text_destination")?,
        if matches!(kind, "length" | "find") {
            "Int"
        } else if kind == "split" {
            "List<Text>"
        } else {
            "Text"
        },
    )?;
    Ok(match kind {
        "join" => AilTextOperation::Join {
            key,
            left: source,
            right: global_field(core, child(core, node, "text_other")?, "Text")?,
        },
        "length" => AilTextOperation::Length { key, source },
        "slice" => AilTextOperation::Slice {
            key,
            source,
            start: bound(core, child(core, node, "text_start")?)?,
            end: bound(core, child(core, node, "text_end")?)?,
        },
        "find" => AilTextOperation::Find {
            key,
            source,
            needle: global_field(core, child(core, node, "text_other")?, "Text")?,
        },
        "split" => AilTextOperation::Split {
            key,
            source,
            separator: global_field(core, child(core, node, "text_other")?, "Text")?,
        },
        "join_list" => AilTextOperation::JoinList {
            key,
            source,
            separator: global_field(core, child(core, node, "text_other")?, "Text")?,
        },
        "replace" => AilTextOperation::Replace {
            key,
            source,
            needle: global_field(core, child(core, node, "text_other")?, "Text")?,
            replacement: global_field(core, child(core, node, "text_replacement")?, "Text")?,
        },
        "trim" => AilTextOperation::Trim { key, source },
        _ => unreachable!(),
    })
}

pub(super) fn from_core(core: &AilCore, action: &Node) -> Vec<AilTextOperation> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == action.id && e.kind == "text_step")
        .map(|e| {
            core.graph
                .nodes
                .iter()
                .find(|n| n.id == e.target)
                .ok_or_else(|| error("missing text operation"))
                .and_then(|n| decode(core, n))
                .unwrap_or_else(AilTextOperation::Invalid)
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
        .is_some_and(|v| v.parse::<usize>().is_ok())
        && edge.attributes.iter().all(|(k, v)| {
            matches!(k.as_str(), "step_index" | "operation_index") && v.parse::<usize>().is_ok()
        })
}

/// Derive access rules from executable operands, including loaded Core that has
/// no descriptive reads/writes edges. Retain the original edge for diagnostics.
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let mut accesses = Vec::new();
    for step in core.graph.edges.iter().filter(|e| e.kind == "text_step") {
        let mut expressions = Vec::new();
        for edge in core.graph.edges.iter().filter(|e| e.source == step.target) {
            match (kind, edge.kind.as_str()) {
                ("reads", "text_source" | "text_other" | "text_replacement")
                | ("writes", "text_destination") => {
                    accesses.push((edge, step.source.as_str()));
                }
                ("reads", "text_start" | "text_end") => expressions.push(edge.target.as_str()),
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
                    "expression_field" => accesses.push((edge, step.source.as_str())),
                    "left_operand" | "right_operand" => expressions.push(edge.target.as_str()),
                    _ => {}
                }
            }
        }
    }
    accesses
}

pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    for node in core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind == "TextOperation")
    {
        let result = (|| {
            if !matches!(core.package.profile.as_str(), "Application" | "C interop") {
                return Err(error(
                    "text operations require the Application or C interop profile",
                ));
            }
            let owners: Vec<_> = core
                .graph
                .edges
                .iter()
                .filter(|e| e.target == node.id && e.kind == "text_step")
                .collect();
            if owners.len() != 1
                || !core
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == owners[0].source && executable_action(n))
            {
                return Err(error(
                    "text operation requires exactly one executable action owner",
                ));
            }
            decode(core, node).map(|_| ())
        })();
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-TEXT-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for edge in &core.graph.edges {
        let source = core.graph.nodes.iter().find(|n| n.id == edge.source);
        let target = core.graph.nodes.iter().find(|n| n.id == edge.target);
        let text_source = source.is_some_and(|n| n.kind == "TextOperation");
        let valid = match edge.kind.as_str() {
            "text_step" => {
                source.is_some_and(executable_action)
                    && target.is_some_and(|n| n.kind == "TextOperation")
                    && valid_step_attributes(edge)
            }
            "text_source" | "text_other" | "text_replacement" | "text_destination" => {
                text_source
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            "text_start" | "text_end" => {
                text_source
                    && target.is_some_and(|n| n.kind == "IntegerExpression")
                    && edge.attributes.is_empty()
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-TEXT-001",
                    format!("invalid {} text operation edge", edge.kind),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn namespace(operation: &AilTextOperation, alias: &str) -> AilTextOperation {
    fn qualify(expression: &IntExpr, prefix: &str) -> IntExpr {
        match expression {
            IntExpr::Literal(value) => IntExpr::Literal(*value),
            IntExpr::Field(key) => IntExpr::Field(format!("{prefix}.{key}")),
            IntExpr::Binary { op, left, right } => IntExpr::Binary {
                op: *op,
                left: Box::new(qualify(left, prefix)),
                right: Box::new(qualify(right, prefix)),
            },
        }
    }
    let prefix = runtime_subject_key(alias);
    let key = |field: &str| format!("{prefix}.{field}");
    match operation {
        AilTextOperation::Join {
            key: destination,
            left,
            right,
        } => AilTextOperation::Join {
            key: key(destination),
            left: key(left),
            right: key(right),
        },
        AilTextOperation::Length {
            key: destination,
            source,
        } => AilTextOperation::Length {
            key: key(destination),
            source: key(source),
        },
        AilTextOperation::Slice {
            key: destination,
            source,
            start,
            end,
        } => AilTextOperation::Slice {
            key: key(destination),
            source: key(source),
            start: qualify(start, &prefix),
            end: qualify(end, &prefix),
        },
        AilTextOperation::Find {
            key: destination,
            source,
            needle,
        } => AilTextOperation::Find {
            key: key(destination),
            source: key(source),
            needle: key(needle),
        },
        AilTextOperation::Split {
            key: destination,
            source,
            separator,
        } => AilTextOperation::Split {
            key: key(destination),
            source: key(source),
            separator: key(separator),
        },
        AilTextOperation::JoinList {
            key: destination,
            source,
            separator,
        } => AilTextOperation::JoinList {
            key: key(destination),
            source: key(source),
            separator: key(separator),
        },
        AilTextOperation::Replace {
            key: destination,
            source,
            needle,
            replacement,
        } => AilTextOperation::Replace {
            key: key(destination),
            source: key(source),
            needle: key(needle),
            replacement: key(replacement),
        },
        AilTextOperation::Trim {
            key: destination,
            source,
        } => AilTextOperation::Trim {
            key: key(destination),
            source: key(source),
        },
        AilTextOperation::Invalid(message) => AilTextOperation::Invalid(message.clone()),
    }
}

pub(super) fn check_source(
    operation: &AilTextOperation,
    document: &AilDocument,
) -> Result<(), String> {
    instruction(operation)?;
    for (_, key, expected) in operation.fields() {
        if runtime_field_type_name(document, key) != Some(expected) {
            return Err(error(format!("{key} must be a declared {expected} field")));
        }
    }
    if let AilTextOperation::Slice { start, end, .. } = operation {
        for key in start.fields().into_iter().chain(end.fields()) {
            if runtime_field_type_name(document, &key) != Some("Int") {
                return Err(error(format!(
                    "slice bound {key} must be a declared Int field"
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn instruction(operation: &AilTextOperation) -> Result<AilBytecodeInstruction, String> {
    let result = match operation {
        AilTextOperation::Join { key, left, right } => AilBytecodeInstruction::new(
            "TEXT_JOIN",
            &[
                ("key", key.clone()),
                ("left", left.clone()),
                ("right", right.clone()),
            ],
        ),
        AilTextOperation::Length { key, source } => AilBytecodeInstruction::new(
            "TEXT_LENGTH",
            &[("key", key.clone()), ("source", source.clone())],
        ),
        AilTextOperation::Slice {
            key,
            source,
            start,
            end,
        } => AilBytecodeInstruction::new(
            "TEXT_SLICE",
            &[
                ("key", key.clone()),
                ("source", source.clone()),
                ("start", start.to_postfix()),
                ("end", end.to_postfix()),
            ],
        ),
        AilTextOperation::Find {
            key,
            source,
            needle,
        } => AilBytecodeInstruction::new(
            "TEXT_FIND",
            &[
                ("key", key.clone()),
                ("source", source.clone()),
                ("needle", needle.clone()),
            ],
        ),
        AilTextOperation::Split {
            key,
            source,
            separator,
        }
        | AilTextOperation::JoinList {
            key,
            source,
            separator,
        } => AilBytecodeInstruction::new(
            if matches!(operation, AilTextOperation::Split { .. }) {
                "TEXT_SPLIT"
            } else {
                "TEXT_JOIN_LIST"
            },
            &[
                ("key", key.clone()),
                ("source", source.clone()),
                ("separator", separator.clone()),
            ],
        ),
        AilTextOperation::Replace {
            key,
            source,
            needle,
            replacement,
        } => AilBytecodeInstruction::new(
            "TEXT_REPLACE",
            &[
                ("key", key.clone()),
                ("source", source.clone()),
                ("needle", needle.clone()),
                ("replacement", replacement.clone()),
            ],
        ),
        AilTextOperation::Trim { key, source } => AilBytecodeInstruction::new(
            "TEXT_TRIM",
            &[("key", key.clone()), ("source", source.clone())],
        ),
        AilTextOperation::Invalid(message) => return Err(error(message)),
    };
    verify(&result)?;
    Ok(result)
}

pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    let expected: &[&str] = match instruction.opcode.as_str() {
        "TEXT_JOIN" => &["key", "left", "right"],
        "TEXT_LENGTH" => &["key", "source"],
        "TEXT_SLICE" => &["end", "key", "source", "start"],
        "TEXT_FIND" => &["key", "needle", "source"],
        "TEXT_SPLIT" | "TEXT_JOIN_LIST" => &["key", "separator", "source"],
        "TEXT_REPLACE" => &["key", "needle", "replacement", "source"],
        "TEXT_TRIM" => &["key", "source"],
        _ => return Err(error("unknown text opcode")),
    };
    if instruction
        .operands
        .keys()
        .map(String::as_str)
        .ne(expected.iter().copied())
    {
        return Err(error(format!(
            "{} requires exactly operands {}",
            instruction.opcode,
            expected.join(", ")
        )));
    }
    for (name, value) in &instruction.operands {
        if matches!(name.as_str(), "start" | "end") {
            IntExpr::from_postfix(value)?;
        } else {
            field(value)?;
        }
    }
    Ok(())
}
pub(super) fn read_text<'a>(
    state: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, String> {
    let text = state
        .get(key)
        .ok_or_else(|| format!("AIL-RUNTIME-TEXT: missing text field {key}"))?;
    if text.len() > MAX_TEXT_BYTES {
        return Err(format!(
            "AIL-RUNTIME-TEXT-LIMIT: text field {key} exceeds 1048576 bytes"
        ));
    }
    Ok(text)
}

pub(super) fn reserve_result(bytes: usize, allocated: &mut usize) -> Result<(), String> {
    if bytes > MAX_TEXT_BYTES {
        return Err("AIL-RUNTIME-TEXT-LIMIT: text result exceeds 1048576 bytes".into());
    }
    let total = allocated
        .checked_add(bytes.max(1))
        .filter(|total| *total <= MAX_ALLOCATED_BYTES)
        .ok_or("AIL-RUNTIME-TEXT-LIMIT: text allocation budget exceeded")?;
    *allocated = total;
    Ok(())
}

fn slice_bound(bound: i64, length: usize) -> usize {
    let length = length as i64;
    if bound < 0 {
        length.saturating_add(bound).clamp(0, length) as usize
    } else {
        bound.min(length) as usize
    }
}

// KMP builds its prefix table once, then advances monotonically through the
// source. Resetting matched to zero after a match gives nonoverlapping matches.
// UTF-8 has self-synchronizing code units, so complete valid string matches
// always begin and end on scalar boundaries.
struct Matches<'a> {
    source: &'a [u8],
    needle: &'a [u8],
    prefix: Vec<usize>,
    position: usize,
    matched: usize,
}
impl<'a> Matches<'a> {
    fn new(source: &'a str, needle: &'a str) -> Self {
        debug_assert!(!needle.is_empty());
        let needle = needle.as_bytes();
        let mut prefix = vec![0; needle.len()];
        let mut matched = 0;
        for index in 1..needle.len() {
            while matched != 0 && needle[index] != needle[matched] {
                matched = prefix[matched - 1];
            }
            if needle[index] == needle[matched] {
                matched += 1;
            }
            prefix[index] = matched;
        }
        Self {
            source: source.as_bytes(),
            needle,
            prefix,
            position: 0,
            matched: 0,
        }
    }
}
impl Iterator for Matches<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        while self.position < self.source.len() {
            let byte = self.source[self.position];
            self.position += 1;
            while self.matched != 0 && byte != self.needle[self.matched] {
                self.matched = self.prefix[self.matched - 1];
            }
            if byte == self.needle[self.matched] {
                self.matched += 1;
            }
            if self.matched == self.needle.len() {
                self.matched = 0;
                return Some(self.position - self.needle.len());
            }
        }
        None
    }
}

fn text_buffer(bytes: usize) -> Result<String, String> {
    if bytes > MAX_TEXT_BYTES {
        return Err("AIL-RUNTIME-TEXT-LIMIT: text result exceeds 1048576 bytes".into());
    }
    let mut result = String::new();
    result
        .try_reserve_exact(bytes)
        .map_err(|_| "AIL-RUNTIME-TEXT-LIMIT: text allocation failed")?;
    Ok(result)
}

fn trim_character(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{001c}'..='\u{0020}' | '\u{0085}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}'..='\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
}

pub(super) fn execute(
    instruction: &AilBytecodeInstruction,
    state: &mut BTreeMap<String, String>,
    allocated: &mut usize,
    list_allocated: &mut usize,
) -> Result<(), String> {
    verify(instruction)?;
    let operand = |name: &str| instruction.operands[name].as_str();
    let result = match instruction.opcode.as_str() {
        "TEXT_JOIN" => {
            let left = read_text(state, operand("left"))?;
            let right = read_text(state, operand("right"))?;
            reserve_result(left.len() + right.len(), allocated)?;
            let mut result = String::with_capacity(left.len() + right.len());
            result.push_str(left);
            result.push_str(right);
            result
        }
        "TEXT_LENGTH" => read_text(state, operand("source"))?
            .chars()
            .count()
            .to_string(),
        "TEXT_SLICE" => {
            let source = read_text(state, operand("source"))?;
            let start = IntExpr::from_postfix(operand("start"))?.eval(state)?;
            let end = IntExpr::from_postfix(operand("end"))?.eval(state)?;
            let length = source.chars().count();
            let start = slice_bound(start, length);
            let end = slice_bound(end, length).max(start);
            let start_byte = source
                .char_indices()
                .nth(start)
                .map_or(source.len(), |(offset, _)| offset);
            let end_byte = source
                .char_indices()
                .nth(end)
                .map_or(source.len(), |(offset, _)| offset);
            let result = &source[start_byte..end_byte];
            reserve_result(result.len(), allocated)?;
            result.to_string()
        }
        "TEXT_FIND" => {
            let source = read_text(state, operand("source"))?;
            let needle = read_text(state, operand("needle"))?;
            source
                .find(needle)
                .map(|offset| source[..offset].chars().count().to_string())
                .unwrap_or_else(|| "-1".into())
        }
        "TEXT_SPLIT" => {
            let source = read_text(state, operand("source"))?;
            let separator = read_text(state, operand("separator"))?;
            if separator.is_empty() {
                return Err("AIL-RUNTIME-TEXT: split separator must not be empty".into());
            }
            let mut items = Vec::new();
            let mut start = 0;
            for offset in Matches::new(source, separator) {
                // A match adds one item plus the eventual trailing item.
                if items.len() == 65_535 {
                    return Err("AIL-RUNTIME-LIST-LIMIT: list exceeds 65536 items".into());
                }
                items.push(source[start..offset].to_string());
                start = offset + separator.len();
            }
            items.push(source[start..].to_string());
            list_operations::make_text_list(&items, list_allocated)?
        }
        "TEXT_JOIN_LIST" => {
            let source = state
                .get(operand("source"))
                .ok_or("AIL-RUNTIME-LIST: missing list field")?;
            let items = list_operations::parse_text_items(source)?;
            let separator = read_text(state, operand("separator"))?;
            let bytes = items.iter().map(String::len).sum::<usize>()
                + separator.len() * items.len().saturating_sub(1);
            let mut result = text_buffer(bytes)?;
            for (index, item) in items.iter().enumerate() {
                if index != 0 {
                    result.push_str(separator);
                }
                result.push_str(item);
            }
            reserve_result(bytes, allocated)?;
            result
        }
        "TEXT_REPLACE" => {
            let source = read_text(state, operand("source"))?;
            let needle = read_text(state, operand("needle"))?;
            let replacement = read_text(state, operand("replacement"))?;
            let result = if needle.is_empty() {
                let boundaries = source.chars().count() + 1;
                let mut result = text_buffer(source.len() + boundaries * replacement.len())?;
                result.push_str(replacement);
                for ch in source.chars() {
                    result.push(ch);
                    result.push_str(replacement);
                }
                result
            } else {
                let offsets: Vec<_> = Matches::new(source, needle).collect();
                let bytes =
                    source.len() - offsets.len() * needle.len() + offsets.len() * replacement.len();
                let mut result = text_buffer(bytes)?;
                let mut start = 0;
                for offset in offsets {
                    result.push_str(&source[start..offset]);
                    result.push_str(replacement);
                    start = offset + needle.len();
                }
                result.push_str(&source[start..]);
                result
            };
            reserve_result(result.len(), allocated)?;
            result
        }
        "TEXT_TRIM" => {
            let source = read_text(state, operand("source"))?;
            let trimmed = source.trim_matches(trim_character);
            let mut result = text_buffer(trimmed.len())?;
            result.push_str(trimmed);
            reserve_result(result.len(), allocated)?;
            result
        }
        _ => unreachable!("verified text opcode"),
    };
    state.insert(operand("key").into(), result);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transformations_parse_render_and_use_exact_protocols() {
        for (source, opcode) in [
            (
                "the system splits text input.value by input.separator into output.parts",
                "TEXT_SPLIT",
            ),
            (
                "the system joins list input.parts with input.separator into output.value",
                "TEXT_JOIN_LIST",
            ),
            (
                "the system replaces text input.needle in input.value with input.replacement into output.value",
                "TEXT_REPLACE",
            ),
            (
                "the system trims text input.value into output.value",
                "TEXT_TRIM",
            ),
        ] {
            let operation = parse_bullet(source)
                .expect("recognized transformation")
                .unwrap();
            assert_eq!(operation.source(), source);
            let instruction = instruction(&operation).unwrap();
            assert_eq!(instruction.opcode, opcode);
            let mut extra = instruction.clone();
            extra
                .operands
                .insert("ignored".into(), "input.value".into());
            assert!(verify(&extra).is_err());
        }
    }

    #[test]
    fn transformations_preserve_unicode_empty_parts_and_charge_separate_arenas() {
        let mut state = BTreeMap::from([
            ("input.value".into(), "🦀\0🦀é🦀".into()),
            ("input.separator".into(), "🦀".into()),
        ]);
        let (mut text, mut lists) = (0, 0);
        execute(
            &op("the system splits text input.value by input.separator into input.value"),
            &mut state,
            &mut text,
            &mut lists,
        )
        .unwrap();
        assert_eq!(state["input.value"], "[\"\",\"\\u0000\",\"é\",\"\"]");
        assert_eq!(text, 0);
        assert_eq!(lists, state["input.value"].len());
        let first_list = state["input.value"].clone();
        execute(
            &op("the system joins list input.value with input.separator into input.value"),
            &mut state,
            &mut text,
            &mut lists,
        )
        .unwrap();
        assert_eq!(state["input.value"], "🦀\0🦀é🦀");
        assert_eq!(text, state["input.value"].len());
        assert_eq!(lists, first_list.len());
    }

    #[test]
    fn replacement_uses_scalar_boundaries_and_never_searches_inserted_text() {
        for (source, needle, replacement, expected) in [
            ("é🦀e\u{301}\0", "", "/", "/é/🦀/e/\u{301}/\0/"),
            ("", "", "🦀", "🦀"),
            ("aaaaa", "aa", "aaa", "aaaaaaa"),
            ("aaa", "a", "aa", "aaaaaa"),
            ("a\0a", "\0", "é", "aéa"),
        ] {
            for key in ["input.value", "input.needle", "input.replacement"] {
                let mut state = BTreeMap::from([
                    ("input.value".into(), source.into()),
                    ("input.needle".into(), needle.into()),
                    ("input.replacement".into(), replacement.into()),
                ]);
                let (mut text, mut lists) = (11, 23);
                execute(&op(&format!("the system replaces text input.needle in input.value with input.replacement into {key}")), &mut state, &mut text, &mut lists).unwrap();
                assert_eq!(state[key], expected);
                assert_eq!(text, 11 + expected.len().max(1));
                assert_eq!(lists, 23);
            }
        }
    }

    #[test]
    fn trim_uses_all_pinned_whitespace_scalars_and_preserves_other_content() {
        let whitespace = [
            9, 10, 11, 12, 13, 28, 29, 30, 31, 32, 133, 160, 5760, 8192, 8193, 8194, 8195, 8196,
            8197, 8198, 8199, 8200, 8201, 8202, 8232, 8233, 8239, 8287, 12288,
        ];
        for scalar in whitespace {
            let ws = char::from_u32(scalar).unwrap();
            let content = format!("\0\u{200b}\u{feff}é{ws}e\u{301}");
            let mut state = BTreeMap::from([("input.value".into(), format!("{ws}{content}{ws}"))]);
            let (mut text, mut lists) = (0, 19);
            execute(
                &op("the system trims text input.value into input.value"),
                &mut state,
                &mut text,
                &mut lists,
            )
            .unwrap();
            assert_eq!(state["input.value"], content, "U+{scalar:04X}");
            assert_eq!((text, lists), (content.len(), 19));
        }
    }

    #[test]
    fn transformation_failures_validate_inputs_in_order_without_mutation() {
        let large = "x".repeat(MAX_TEXT_BYTES + 1);
        for (source, entries, expected) in [
            (
                "the system splits text input.value by input.separator into output.value",
                vec![("input.value", large.as_str())],
                "AIL-RUNTIME-TEXT-LIMIT:",
            ),
            (
                "the system splits text input.value by input.separator into output.value",
                vec![("input.value", ""), ("input.separator", "")],
                "AIL-RUNTIME-TEXT:",
            ),
            (
                "the system joins list input.value with input.separator into output.value",
                vec![("input.value", "[\"valid\",42]")],
                "AIL-RUNTIME-LIST:",
            ),
            (
                "the system joins list input.value with input.separator into output.value",
                vec![("input.value", large.as_str())],
                "AIL-RUNTIME-LIST-LIMIT:",
            ),
            (
                "the system replaces text input.needle in input.value with input.replacement into output.value",
                vec![("input.value", large.as_str())],
                "AIL-RUNTIME-TEXT-LIMIT:",
            ),
            (
                "the system replaces text input.needle in input.value with input.replacement into output.value",
                vec![("input.value", ""), ("input.needle", large.as_str())],
                "AIL-RUNTIME-TEXT-LIMIT:",
            ),
            (
                "the system replaces text input.needle in input.value with input.replacement into output.value",
                vec![("input.value", ""), ("input.needle", "")],
                "AIL-RUNTIME-TEXT:",
            ),
            (
                "the system trims text input.value into output.value",
                vec![],
                "AIL-RUNTIME-TEXT:",
            ),
        ] {
            let mut state: BTreeMap<String, String> = entries
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect();
            state.insert("output.value".into(), "preserved".into());
            let original = state.clone();
            let (mut text, mut lists) = (67_108_864, 67_108_864);
            assert!(
                execute(&op(source), &mut state, &mut text, &mut lists)
                    .unwrap_err()
                    .starts_with(expected),
                "{source}"
            );
            assert_eq!(state, original);
            assert_eq!((text, lists), (67_108_864, 67_108_864));
        }
    }

    #[test]
    fn join_borrows_noncanonical_list_and_empty_results_charge_only_their_arena() {
        let mut state = BTreeMap::from([
            (
                "input.value".into(),
                format!("[\"{}\"]", "\u{85}".repeat(400_000)),
            ),
            ("input.separator".into(), "".into()),
        ]);
        let (mut text, mut lists) = (0, 67_108_864);
        execute(
            &op("the system joins list input.value with input.separator into input.separator"),
            &mut state,
            &mut text,
            &mut lists,
        )
        .unwrap();
        assert_eq!(state["input.separator"], "\u{85}".repeat(400_000));
        assert_eq!((text, lists), (800_000, 67_108_864));
        state.insert("input.value".into(), "[]".into());
        execute(
            &op("the system joins list input.value with input.separator into input.value"),
            &mut state,
            &mut text,
            &mut lists,
        )
        .unwrap();
        assert_eq!(state["input.value"], "");
        assert_eq!((text, lists), (800_001, 67_108_864));
        state.insert("input.separator".into(), ",".into());
        lists = 67_108_860;
        text = 67_108_864;
        execute(
            &op("the system splits text input.value by input.separator into input.value"),
            &mut state,
            &mut text,
            &mut lists,
        )
        .unwrap();
        assert_eq!(state["input.value"], "[\"\"]");
        assert_eq!((text, lists), (67_108_864, 67_108_864));
    }

    #[test]
    fn split_enforces_item_and_canonical_byte_limits_before_charging() {
        for (input, separator, expected_len, success) in [
            (",".repeat(65_535), ",", 196_609, true),
            (",".repeat(65_536), ",", 0, false),
            ("\0".repeat(174_762), ",", 1_048_576, true),
            ("\0".repeat(174_763), ",", 0, false),
            ("a".repeat(1_048_572), ",", 1_048_576, true),
            ("a".repeat(1_048_573), ",", 0, false),
        ] {
            let mut state = BTreeMap::from([
                ("input.value".into(), input),
                ("input.separator".into(), separator.into()),
                ("output.value".into(), "preserved".into()),
            ]);
            let (mut text, mut lists) = (29, 0);
            let result = execute(
                &op("the system splits text input.value by input.separator into output.value"),
                &mut state,
                &mut text,
                &mut lists,
            );
            if success {
                result.unwrap();
                assert_eq!(state["output.value"].len(), expected_len);
                assert_eq!(lists, expected_len);
            } else {
                assert!(result.unwrap_err().starts_with("AIL-RUNTIME-LIST-LIMIT:"));
                assert_eq!(state["output.value"], "preserved");
                assert_eq!(lists, 0);
            }
            assert_eq!(text, 29);
        }
    }

    #[test]
    fn repetitive_matching_advances_without_overlapping_matches() {
        let needle = format!("{}b", "a".repeat(32_767));
        let source = format!("{}b", "a".repeat(1_048_575));
        assert_eq!(
            Matches::new(&source, &needle).collect::<Vec<_>>(),
            vec![1_015_808]
        );
        assert_eq!(Matches::new("aaaaa", "aa").collect::<Vec<_>>(), vec![0, 2]);
    }

    #[test]
    fn transformation_results_check_size_and_arena_limits_atomically() {
        for (source, value, needle, replacement, separator, family) in [
            (
                "the system splits text input.value by input.separator into output.value",
                "".into(),
                "",
                "".into(),
                ",".into(),
                "AIL-RUNTIME-LIST-LIMIT:",
            ),
            (
                "the system joins list input.value with input.separator into output.value",
                "[]".into(),
                "",
                "".into(),
                "".into(),
                "AIL-RUNTIME-TEXT-LIMIT:",
            ),
            (
                "the system replaces text input.needle in input.value with input.replacement into output.value",
                "".into(),
                "",
                "".into(),
                "".into(),
                "AIL-RUNTIME-TEXT-LIMIT:",
            ),
            (
                "the system trims text input.value into output.value",
                "".into(),
                "",
                "".into(),
                "".into(),
                "AIL-RUNTIME-TEXT-LIMIT:",
            ),
            (
                "the system replaces text input.needle in input.value with input.replacement into output.value",
                "a".repeat(MAX_TEXT_BYTES),
                "a",
                "aa".into(),
                "".into(),
                "AIL-RUNTIME-TEXT-LIMIT:",
            ),
            (
                "the system replaces text input.needle in input.value with input.replacement into output.value",
                "é".into(),
                "",
                "x".repeat(MAX_TEXT_BYTES),
                "".into(),
                "AIL-RUNTIME-TEXT-LIMIT:",
            ),
            (
                "the system joins list input.value with input.separator into output.value",
                "[\"\",\"\",\"\"]".into(),
                "",
                "".into(),
                "x".repeat(MAX_TEXT_BYTES),
                "AIL-RUNTIME-TEXT-LIMIT:",
            ),
        ] {
            for initial in [0, MAX_ALLOCATED_BYTES] {
                let mut state = BTreeMap::from([
                    ("input.value".into(), value.clone()),
                    ("input.needle".into(), needle.into()),
                    ("input.replacement".into(), replacement.clone()),
                    ("input.separator".into(), separator.clone()),
                    ("output.value".into(), "preserved".into()),
                ]);
                let original = state.clone();
                let (mut text, mut lists) = (initial, initial);
                let result = execute(&op(source), &mut state, &mut text, &mut lists);
                if initial == 0
                    && value.len() < MAX_TEXT_BYTES
                    && replacement.len() < MAX_TEXT_BYTES
                    && separator.len() < MAX_TEXT_BYTES
                {
                    result.unwrap();
                    if source.contains("splits") {
                        assert_eq!((text, lists), (0, 4));
                    } else {
                        assert_eq!((text, lists), (1, 0));
                    }
                } else {
                    assert!(result.unwrap_err().starts_with(family));
                    assert_eq!(state, original);
                    assert_eq!((text, lists), (initial, initial));
                }
            }
        }
        let mut state = BTreeMap::from([
            ("input.value".into(), "a".repeat(524_288)),
            ("input.needle".into(), "a".into()),
            ("input.replacement".into(), "é".into()),
        ]);
        let (mut text, mut lists) = (0, 31);
        execute(&op("the system replaces text input.needle in input.value with input.replacement into input.value"), &mut state, &mut text, &mut lists).unwrap();
        assert_eq!(state["input.value"], "é".repeat(524_288));
        assert_eq!((text, lists), (1_048_576, 31));
    }

    #[test]
    fn transformations_round_trip_typed_core_and_check_replacement_secrets() {
        let package = load_ail_package_dir("bench/R003-max/program.ail").unwrap();
        let source = "The application Texts manages text.\n\nAn Input has:\n- value: Text\n- separator: Text\n- needle: Text\n- replacement: Text\n- parts: List<Text>\n\nAction: Run.\n\nWhen Run happens:\n- the system splits text input.value by input.separator into input.parts\n- the system joins list input.parts with input.separator into input.value\n- the system replaces text input.needle in input.value with input.replacement into input.value\n- the system trims text input.value into input.value\n";
        let core = elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, source).unwrap(),
        );
        assert!(check(&core).is_empty(), "{:?}", check(&core));
        let action = core.graph.find_node("Action", "Run").unwrap();
        let original = from_core(&core, action);
        assert_eq!(original.len(), 4);
        let saved = parse_ail_core_text(&render_ail_core(&core)).unwrap();
        assert_eq!(
            from_core(&saved, saved.graph.find_node("Action", "Run").unwrap()),
            original
        );
        assert!(check(&saved).is_empty());
        for relation in [
            "text_source",
            "text_other",
            "text_replacement",
            "text_destination",
            "text_step",
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
                let saved = parse_ail_core_text(&render_ail_core(&bad)).unwrap();
                assert!(
                    !check(&saved).is_empty(),
                    "accepted {relation}, duplicate={duplicate}"
                );
            }
        }
        let mut secret = core.clone();
        secret
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        let replacement = secret
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.name == "Input.replacement")
            .unwrap();
        replacement
            .attributes
            .insert("secret".into(), "true".into());
        assert!(check_ail_core(&secret).iter().any(|d| d.contains("AIL005")));
        let mut forged = core.clone();
        let mut fake = forged
            .graph
            .nodes
            .iter()
            .find(|n| n.name == "Input.replacement")
            .unwrap()
            .clone();
        fake.id = "forged-text-replacement".into();
        forged
            .graph
            .edges
            .iter_mut()
            .find(|e| e.kind == "text_replacement")
            .unwrap()
            .target = fake.id.clone();
        forged.graph.nodes.push(fake);
        assert!(!check(&forged).is_empty());
    }

    #[test]
    fn text_core_rejects_ignored_metadata_and_forged_global_storage() {
        let initial = core_fixture();
        for mutation in 0..5 {
            let mut core = initial.clone();
            match mutation {
                0 => {
                    core.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.kind == "TextOperation")
                        .unwrap()
                        .attributes
                        .insert("ignored".into(), "true".into());
                }
                1 => {
                    core.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.kind == "TextOperation")
                        .unwrap()
                        .type_name = Some("Text".into());
                }
                2 => {
                    core.graph
                        .edges
                        .iter_mut()
                        .find(|e| e.kind == "text_source")
                        .unwrap()
                        .attributes
                        .insert("ignored".into(), "true".into());
                }
                3 => {
                    core.graph
                        .edges
                        .iter_mut()
                        .find(|e| e.kind == "text_step")
                        .unwrap()
                        .attributes
                        .insert("ignored".into(), "true".into());
                }
                _ => {
                    let source = core
                        .graph
                        .edges
                        .iter()
                        .find(|e| e.kind == "text_source")
                        .unwrap()
                        .target
                        .clone();
                    core.graph
                        .edges
                        .retain(|e| !(e.kind == "has_field" && e.target == source));
                }
            }
            assert!(!check(&core).is_empty(), "accepted mutation {mutation}");
        }
    }

    fn op(text: &str) -> AilBytecodeInstruction {
        instruction(&parse_bullet(text).expect("text statement").unwrap()).unwrap()
    }

    fn core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R003-max/program.ail").unwrap();
        let source = "The application Texts manages text.\n\nAn Input has:\n- value: Text\n- needle: Text\n- start: Int\n- end: Int\n\nAn Output has:\n- value: Text\n- length: Int\n\nAction: Run.\n\nWhen Run happens:\n- the system slices text input.value from input.start to input.end into output.value\n- the system counts characters in output.value into output.length\n- the system records a trace event named Processed\n";
        elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, source).unwrap(),
        )
    }

    #[test]
    fn typed_core_preserves_operations_and_rejects_missing_or_extra_operands() {
        let core = core_fixture();
        let action = core.graph.find_node("Action", "Run").unwrap();
        assert_eq!(from_core(&core, action).len(), 2);
        assert!(check(&core).is_empty());
        let saved = parse_ail_core_text(&render_ail_core(&core)).unwrap();
        assert_eq!(
            from_core(&saved, saved.graph.find_node("Action", "Run").unwrap()),
            from_core(&core, action)
        );
        let mut missing = core.clone();
        missing.graph.edges.retain(|e| e.kind != "text_end");
        assert!(!check(&missing).is_empty());
        let mut duplicate = core.clone();
        duplicate.graph.edges.push(
            core.graph
                .edges
                .iter()
                .find(|e| e.kind == "text_source")
                .unwrap()
                .clone(),
        );
        assert!(!check(&duplicate).is_empty());
        let loaded = parse_ail_core_text(&render_ail_core(&duplicate)).unwrap();
        assert!(!check(&loaded).is_empty());
    }

    #[test]
    fn unicode_operations_count_scalar_values_and_preserve_aliases() {
        let mut state = BTreeMap::from([
            ("input.value".into(), "aé🦀e\u{301}".into()),
            ("input.other".into(), "!".into()),
        ]);
        let mut allocated = 0;
        execute(
            &op("the system counts characters in input.value into output.length"),
            &mut state,
            &mut allocated,
            &mut 0,
        )
        .unwrap();
        assert_eq!(state["output.length"], "5");
        execute(
            &op("the system slices text input.value from 1 to -1 into input.value"),
            &mut state,
            &mut allocated,
            &mut 0,
        )
        .unwrap();
        assert_eq!(state["input.value"], "é🦀e");
        execute(
            &op("the system joins text input.value and input.other into input.value"),
            &mut state,
            &mut allocated,
            &mut 0,
        )
        .unwrap();
        assert_eq!(state["input.value"], "é🦀e!");
        assert_eq!(allocated, 15);
    }

    #[test]
    fn find_returns_character_positions_without_normalization() {
        let mut state = BTreeMap::from([
            ("input.value".into(), "🦀e\u{301}é".into()),
            ("input.needle".into(), "é".into()),
        ]);
        let mut allocated = 0;
        let find = op("the system finds text input.needle in input.value into output.position");
        execute(&find, &mut state, &mut allocated, &mut 0).unwrap();
        assert_eq!(state["output.position"], "3");
        state.insert("input.needle".into(), String::new());
        execute(&find, &mut state, &mut allocated, &mut 0).unwrap();
        assert_eq!(state["output.position"], "0");
        state.insert("input.needle".into(), "E".into());
        execute(&find, &mut state, &mut allocated, &mut 0).unwrap();
        assert_eq!(state["output.position"], "-1");
        assert_eq!(allocated, 0);
    }

    #[test]
    fn slices_clamp_extreme_negative_and_reversed_scalar_bounds() {
        for (start, end, expected) in [
            (i64::MIN, i64::MAX, "aé🦀"),
            (-2, i64::MAX, "é🦀"),
            (i64::MAX, i64::MIN, ""),
            (0, -1, "aé"),
            (9, 10, ""),
            (-99, -9, ""),
        ] {
            let instruction = op(&format!(
                "the system slices text input.value from {start} to {end} into output.value"
            ));
            let mut state = BTreeMap::from([("input.value".into(), "aé🦀".into())]);
            execute(&instruction, &mut state, &mut 0, &mut 0).unwrap();
            assert_eq!(state["output.value"], expected, "{start}..{end}");
        }
    }

    #[test]
    fn allocation_budget_counts_empty_results_and_preserves_existing_values_on_failure() {
        let instruction = op("the system joins text input.left and input.right into output.value");
        let mut state = BTreeMap::from([
            ("input.left".into(), String::new()),
            ("input.right".into(), String::new()),
        ]);
        let mut allocated = 67_108_863;
        execute(&instruction, &mut state, &mut allocated, &mut 0).unwrap();
        assert_eq!(allocated, 67_108_864);
        state.insert("output.value".into(), "preserved".into());
        assert!(
            execute(&instruction, &mut state, &mut allocated, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT-LIMIT")
        );
        assert_eq!(state["output.value"], "preserved");
        assert_eq!(allocated, 67_108_864);
        let count = op("the system counts characters in output.value into output.length");
        execute(&count, &mut state, &mut allocated, &mut 0).unwrap();
        assert_eq!(state["output.length"], "9");
        assert_eq!(allocated, 67_108_864);
    }

    #[test]
    fn operand_validation_order_precedes_destination_and_budget_mutation() {
        let oversized = "x".repeat(1_048_577);
        let cases = [
            (
                "the system joins text input.large and input.missing into output.value",
                "AIL-RUNTIME-TEXT-LIMIT",
            ),
            (
                "the system joins text input.missing and input.large into output.value",
                "AIL-RUNTIME-TEXT:",
            ),
            (
                "the system finds text input.missing in input.large into output.value",
                "AIL-RUNTIME-TEXT-LIMIT",
            ),
            (
                "the system slices text input.missing from 1 / 0 to 2 into output.value",
                "AIL-RUNTIME-TEXT:",
            ),
            (
                "the system slices text input.value from 1 / 0 to input.missing into output.value",
                "AIL-RUNTIME-DIVZERO",
            ),
        ];
        for (source, expected) in cases {
            let mut state = BTreeMap::from([
                ("input.large".into(), oversized.clone()),
                ("input.value".into(), "valid".into()),
                ("output.value".into(), "preserved".into()),
            ]);
            let mut allocated = 7;
            assert!(
                execute(&op(source), &mut state, &mut allocated, &mut 0)
                    .unwrap_err()
                    .starts_with(expected),
                "{source}"
            );
            assert_eq!(state["output.value"], "preserved");
            assert_eq!(allocated, 7);
        }
    }

    #[test]
    fn exact_operand_size_limit_and_result_size_limit_are_independent() {
        let instruction = op("the system joins text input.left and input.right into output.value");
        let mut state = BTreeMap::from([
            ("input.left".into(), "é".repeat(524_288)),
            ("input.right".into(), String::new()),
        ]);
        let mut allocated = 0;
        execute(&instruction, &mut state, &mut allocated, &mut 0).unwrap();
        assert_eq!(state["output.value"].len(), 1_048_576);
        assert_eq!(allocated, 1_048_576);
        state.insert("input.right".into(), "!".into());
        assert!(
            execute(&instruction, &mut state, &mut allocated, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT-LIMIT")
        );
        assert_eq!(state["output.value"].len(), 1_048_576);
        assert_eq!(allocated, 1_048_576);
    }

    #[test]
    fn protocol_rejects_missing_extra_malformed_and_literal_operands() {
        let original = op(
            "the system slices text input.value from input.start to input.end into output.value",
        );
        let mut missing = original.clone();
        missing.operands.remove("end");
        let mut extra = original.clone();
        extra
            .operands
            .insert("ignored".into(), "input.value".into());
        let mut literal = original.clone();
        literal.operands.insert("source".into(), "123".into());
        let mut malformed = original;
        malformed
            .operands
            .insert("start".into(), "f:input.start +".into());
        for instruction in [missing, extra, literal, malformed] {
            assert!(verify(&instruction).is_err());
        }
    }

    #[test]
    fn text_core_rejects_unsupported_owners_profiles_and_local_bounds() {
        let initial = core_fixture();
        assert!(check(&initial).is_empty());
        for profile in ["UI", "Compiler", "System", "AgentTool"] {
            let mut core = initial.clone();
            core.package.profile = profile.into();
            assert!(!check(&core).is_empty());
        }
        let mut pass = initial.clone();
        pass.graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Action")
            .unwrap()
            .attributes
            .insert("kind".into(), "CompilerPass".into());
        assert!(!check(&pass).is_empty());
        let mut local = initial;
        let bound_field = local
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name == "Input.start")
            .unwrap();
        bound_field
            .attributes
            .insert("function_owner".into(), "hidden".into());
        bound_field
            .attributes
            .insert("local_name".into(), "start".into());
        assert!(!check(&local).is_empty());
    }
}
