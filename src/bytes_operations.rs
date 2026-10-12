//! Explicit bounded byte values, typed Core operations and VM adapters.
use super::*;
use crate::expression::IntExpr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilBytesOperation {
    Length {
        source: String,
        key: String,
    },
    Get {
        source: String,
        index: IntExpr,
        key: String,
    },
    Slice {
        source: String,
        start: IntExpr,
        end: IntExpr,
        key: String,
    },
    Concat {
        left: String,
        right: String,
        key: String,
    },
    Set {
        source: String,
        index: IntExpr,
        value: IntExpr,
        key: String,
    },
    FromUtf8 {
        source: String,
        key: String,
    },
    ToUtf8 {
        source: String,
        key: String,
    },
    FromHex {
        source: String,
        key: String,
    },
    ToHex {
        source: String,
        key: String,
    },
    Compare {
        left: String,
        right: String,
        key: String,
    },
    Invalid(String),
}
const MAX_RAW: usize = 1_048_576;
const MAX_TRANSPORT: usize = 4 + 2 * MAX_RAW;
const MAX_ALLOCATED: usize = 67_108_864;
const KINDS: [&str; 10] = [
    "length",
    "get",
    "slice",
    "concat",
    "set",
    "from_utf8",
    "to_utf8",
    "from_hex",
    "to_hex",
    "compare",
];
const OPCODES: [&str; 10] = [
    "BYTES_LENGTH",
    "BYTES_GET",
    "BYTES_SLICE",
    "BYTES_CONCAT",
    "BYTES_SET",
    "BYTES_FROM_UTF8",
    "BYTES_TO_UTF8",
    "BYTES_FROM_HEX",
    "BYTES_TO_HEX",
    "BYTES_COMPARE",
];
// Protocol operand name, Core role, static field type (or an Int expression).
fn schema(kind: usize) -> Vec<(&'static str, &'static str, &'static str)> {
    let mut fields = if matches!(kind, 3 | 9) {
        vec![
            ("left", "bytes_source", "Bytes"),
            ("right", "bytes_other", "Bytes"),
        ]
    } else {
        vec![(
            "source",
            "bytes_source",
            if matches!(kind, 5 | 7) {
                "Text"
            } else {
                "Bytes"
            },
        )]
    };
    match kind {
        1 => fields.push(("index", "bytes_index", "expression")),
        2 => fields.extend([
            ("start", "bytes_start", "expression"),
            ("end", "bytes_end", "expression"),
        ]),
        4 => fields.extend([
            ("index", "bytes_index", "expression"),
            ("value", "bytes_value", "expression"),
        ]),
        _ => {}
    }
    fields.push((
        "key",
        "bytes_destination",
        match kind {
            0 | 1 | 9 => "Int",
            6 | 8 => "Text",
            _ => "Bytes",
        },
    ));
    fields
}
fn error(message: impl Display) -> String {
    format!("AIL-BYTES-001: {message}")
}
fn field(value: &str) -> Result<(), String> {
    if matches!(IntExpr::parse(value),Ok(IntExpr::Field(key)) if key==value) {
        Ok(())
    } else {
        Err(error("byte operand requires a field reference"))
    }
}
impl AilBytesOperation {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Length { source, key } => {
                format!("the system counts bytes in {source} into {key}")
            }
            Self::Get { source, index, key } => format!(
                "the system gets byte {} from bytes {source} into {key}",
                index.to_source()
            ),
            Self::Slice {
                source,
                start,
                end,
                key,
            } => format!(
                "the system slices bytes {source} from {} to {} into {key}",
                start.to_source(),
                end.to_source()
            ),
            Self::Concat { left, right, key } => {
                format!("the system joins bytes {left} and {right} into {key}")
            }
            Self::Set {
                source,
                index,
                value,
                key,
            } => format!(
                "the system sets byte {} in bytes {source} to {} into {key}",
                index.to_source(),
                value.to_source()
            ),
            Self::FromUtf8 { source, key } => {
                format!("the system encodes text {source} as UTF-8 bytes into {key}")
            }
            Self::ToUtf8 { source, key } => {
                format!("the system decodes bytes {source} as UTF-8 text into {key}")
            }
            Self::FromHex { source, key } => {
                format!("the system decodes hexadecimal text {source} into bytes {key}")
            }
            Self::ToHex { source, key } => {
                format!("the system encodes bytes {source} as hexadecimal text into {key}")
            }
            Self::Compare { left, right, key } => {
                format!("the system compares bytes {left} with {right} into {key}")
            }
            Self::Invalid(message) => format!("invalid bytes operation: {message}"),
        }
    }
}
fn boundary(text: &str, prefix: &str) -> bool {
    text.strip_prefix(prefix).is_some_and(|tail| {
        tail.chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_')
    })
}
pub(super) fn recognized_bullet(text: &str) -> bool {
    let text = trim_sentence(text);
    text.strip_prefix("the system ")
        .or_else(|| text.strip_prefix("the function "))
        .is_some_and(|body| {
            [
                "counts bytes",
                "gets byte",
                "slices bytes",
                "joins bytes",
                "sets byte",
                "encodes text",
                "decodes bytes",
                "decodes hexadecimal",
                "encodes bytes",
                "compares bytes",
            ]
            .iter()
            .any(|prefix| boundary(body, prefix))
        })
}
pub(super) fn parse_bullet(text: &str) -> Option<Result<AilBytesOperation, String>> {
    let text = trim_sentence(text);
    let body = text.strip_prefix("the system ")?;
    if !recognized_bullet(&text) {
        return None;
    }
    Some((|| {
        fn split<'a>(s: &'a str, delimiter: &str) -> Result<(&'a str, &'a str), String> {
            s.split_once(delimiter)
                .ok_or_else(|| error("malformed byte operation"))
        }
        let (body, key) = split(
            body,
            if body.starts_with("decodes hexadecimal ") {
                " into bytes "
            } else {
                " into "
            },
        )?;
        let mut operands = BTreeMap::from([("key".into(), key.into())]);
        let (kind, source) = if let Some(rest) = body.strip_prefix("counts bytes in ") {
            (0, rest)
        } else if let Some(rest) = body.strip_prefix("gets byte ") {
            let (index, source) = split(rest, " from bytes ")?;
            operands.insert(
                "index".into(),
                IntExpr::parse(index).map_err(error)?.to_postfix(),
            );
            (1, source)
        } else if let Some(rest) = body.strip_prefix("slices bytes ") {
            let (source, bounds) = split(rest, " from ")?;
            let (start, end) = split(bounds, " to ")?;
            operands.insert(
                "start".into(),
                IntExpr::parse(start).map_err(error)?.to_postfix(),
            );
            operands.insert(
                "end".into(),
                IntExpr::parse(end).map_err(error)?.to_postfix(),
            );
            (2, source)
        } else if let Some(rest) = body.strip_prefix("joins bytes ") {
            let (left, right) = split(rest, " and ")?;
            operands.insert("right".into(), right.into());
            (3, left)
        } else if let Some(rest) = body.strip_prefix("sets byte ") {
            let (index, rest) = split(rest, " in bytes ")?;
            let (source, value) = split(rest, " to ")?;
            operands.insert(
                "index".into(),
                IntExpr::parse(index).map_err(error)?.to_postfix(),
            );
            operands.insert(
                "value".into(),
                IntExpr::parse(value).map_err(error)?.to_postfix(),
            );
            (4, source)
        } else if let Some(rest) = body
            .strip_prefix("encodes text ")
            .and_then(|s| s.strip_suffix(" as UTF-8 bytes"))
        {
            (5, rest)
        } else if let Some(rest) = body
            .strip_prefix("decodes bytes ")
            .and_then(|s| s.strip_suffix(" as UTF-8 text"))
        {
            (6, rest)
        } else if let Some(rest) = body.strip_prefix("decodes hexadecimal text ") {
            (7, rest)
        } else if let Some(rest) = body
            .strip_prefix("encodes bytes ")
            .and_then(|s| s.strip_suffix(" as hexadecimal text"))
        {
            (8, rest)
        } else if let Some(rest) = body.strip_prefix("compares bytes ") {
            let (left, right) = split(rest, " with ")?;
            operands.insert("right".into(), right.into());
            (9, left)
        } else {
            return Err(error("malformed byte operation"));
        };
        operands.insert(
            if matches!(kind, 3 | 9) {
                "left"
            } else {
                "source"
            }
            .into(),
            source.into(),
        );
        from_instruction(&AilBytecodeInstruction {
            opcode: OPCODES[kind].into(),
            operands,
        })
    })())
}
fn from_instruction(i: &AilBytecodeInstruction) -> Result<AilBytesOperation, String> {
    verify(i)?;
    let get = |name: &str| i.operands[name].clone();
    let expr = |name: &str| IntExpr::from_postfix(&i.operands[name]).map_err(error);
    let key = get("key");
    Ok(match i.opcode.as_str() {
        "BYTES_LENGTH" => AilBytesOperation::Length {
            source: get("source"),
            key,
        },
        "BYTES_GET" => AilBytesOperation::Get {
            source: get("source"),
            index: expr("index")?,
            key,
        },
        "BYTES_SLICE" => AilBytesOperation::Slice {
            source: get("source"),
            start: expr("start")?,
            end: expr("end")?,
            key,
        },
        "BYTES_CONCAT" => AilBytesOperation::Concat {
            left: get("left"),
            right: get("right"),
            key,
        },
        "BYTES_SET" => AilBytesOperation::Set {
            source: get("source"),
            index: expr("index")?,
            value: expr("value")?,
            key,
        },
        "BYTES_FROM_UTF8" => AilBytesOperation::FromUtf8 {
            source: get("source"),
            key,
        },
        "BYTES_TO_UTF8" => AilBytesOperation::ToUtf8 {
            source: get("source"),
            key,
        },
        "BYTES_FROM_HEX" => AilBytesOperation::FromHex {
            source: get("source"),
            key,
        },
        "BYTES_TO_HEX" => AilBytesOperation::ToHex {
            source: get("source"),
            key,
        },
        "BYTES_COMPARE" => AilBytesOperation::Compare {
            left: get("left"),
            right: get("right"),
            key,
        },
        _ => unreachable!("verified opcode"),
    })
}
pub(super) fn instruction(op: &AilBytesOperation) -> Result<AilBytecodeInstruction, String> {
    let mut operands = BTreeMap::new();
    let (kind, source, key) = match op {
        AilBytesOperation::Length { source, key } => (0, source, key),
        AilBytesOperation::Get { source, index, key } => {
            operands.insert("index".into(), index.to_postfix());
            (1, source, key)
        }
        AilBytesOperation::Slice {
            source,
            start,
            end,
            key,
        } => {
            operands.insert("start".into(), start.to_postfix());
            operands.insert("end".into(), end.to_postfix());
            (2, source, key)
        }
        AilBytesOperation::Concat { left, right, key } => {
            operands.insert("right".into(), right.clone());
            (3, left, key)
        }
        AilBytesOperation::Set {
            source,
            index,
            value,
            key,
        } => {
            operands.insert("index".into(), index.to_postfix());
            operands.insert("value".into(), value.to_postfix());
            (4, source, key)
        }
        AilBytesOperation::FromUtf8 { source, key } => (5, source, key),
        AilBytesOperation::ToUtf8 { source, key } => (6, source, key),
        AilBytesOperation::FromHex { source, key } => (7, source, key),
        AilBytesOperation::ToHex { source, key } => (8, source, key),
        AilBytesOperation::Compare { left, right, key } => {
            operands.insert("right".into(), right.clone());
            (9, left, key)
        }
        AilBytesOperation::Invalid(message) => return Err(error(message)),
    };
    operands.insert(
        if matches!(kind, 3 | 9) {
            "left"
        } else {
            "source"
        }
        .into(),
        source.clone(),
    );
    operands.insert("key".into(), key.clone());
    let i = AilBytecodeInstruction {
        opcode: OPCODES[kind].into(),
        operands,
    };
    verify(&i)?;
    Ok(i)
}
pub(super) fn verify(i: &AilBytecodeInstruction) -> Result<(), String> {
    let kind = OPCODES
        .iter()
        .position(|opcode| *opcode == i.opcode)
        .ok_or_else(|| error("unknown byte instruction"))?;
    let schema = schema(kind);
    if i.operands.len() != schema.len() {
        return Err(error("byte instruction has incorrect operands"));
    }
    for (name, _, ty) in schema {
        let value = i
            .operands
            .get(name)
            .ok_or_else(|| error("missing byte operand"))?;
        if ty == "expression" {
            IntExpr::from_postfix(value).map_err(error)?;
        } else {
            field(value)?;
        }
    }
    Ok(())
}
pub(super) fn check_source(op: &AilBytesOperation, document: &AilDocument) -> Result<(), String> {
    let i = instruction(op)?;
    let kind = OPCODES
        .iter()
        .position(|opcode| *opcode == i.opcode)
        .unwrap();
    for (name, _, ty) in schema(kind) {
        let fields = if ty == "expression" {
            IntExpr::from_postfix(&i.operands[name])
                .map_err(error)?
                .fields()
        } else {
            BTreeSet::from([i.operands[name].clone()])
        };
        for key in fields {
            if runtime_field_type_name(document, &key)
                != Some(if ty == "expression" { "Int" } else { ty })
            {
                return Err(error(
                    "byte operand requires a declared field of its exact type",
                ));
            }
        }
    }
    Ok(())
}
pub(super) fn namespace(op: &AilBytesOperation, alias: &str) -> AilBytesOperation {
    let Ok(mut i) = instruction(op) else {
        return op.clone();
    };
    let prefix = runtime_subject_key(alias);
    let kind = OPCODES
        .iter()
        .position(|opcode| *opcode == i.opcode)
        .unwrap();
    for (name, _, ty) in schema(kind) {
        let value = &i.operands[name];
        let value = if ty == "expression" {
            value
                .split_ascii_whitespace()
                .map(|token| {
                    token
                        .strip_prefix("f:")
                        .map_or_else(|| token.to_string(), |key| format!("f:{prefix}.{key}"))
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            format!("{prefix}.{value}")
        };
        i.operands.insert(name.into(), value);
    }
    from_instruction(&i).unwrap_or_else(AilBytesOperation::Invalid)
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, op) in action.bytes_operations.iter().enumerate() {
            let i = instruction(op).ok();
            let kind = i
                .as_ref()
                .and_then(|i| OPCODES.iter().position(|opcode| *opcode == i.opcode));
            let name = format!("{}.bytes{index}", owner.name);
            let node = graph.add_node(
                "BytesOperation",
                &name,
                None,
                attr(&[("bytes_kind", kind.map_or("invalid", |k| KINDS[k]))]),
            );
            attach_provenance(graph, &node, format!("action:{}.bytes:{index}", owner.name));
            if let (Some(i), Some(kind)) = (i, kind) {
                for (operand, role, ty) in schema(kind) {
                    let value = &i.operands[operand];
                    let target = if ty == "expression" {
                        expression_core::build(
                            graph,
                            &owner,
                            &format!("{name}.{operand}"),
                            &IntExpr::from_postfix(value).expect("verified expression"),
                        )
                    } else {
                        graph
                            .nodes
                            .iter()
                            .find(|n| {
                                n.kind == "Field"
                                    && !n.attributes.contains_key("function_owner")
                                    && !n.attributes.contains_key("local_name")
                                    && expression_core::field_key(n) == *value
                            })
                            .cloned()
                            .unwrap_or_else(|| {
                                let target = graph.add_node("Effect", value, None, BTreeMap::new());
                                attach_provenance(graph, &target, format!("{name}.{operand}"));
                                target
                            })
                    };
                    graph.add_edge(role, &node, &target, BTreeMap::new());
                }
            }
            action_order::append(graph, "bytes_step", &owner, &node, BTreeMap::new(), index);
        }
    }
}
fn unique_node<'a>(core: &'a AilCore, id: &str) -> Result<&'a Node, String> {
    let mut nodes = core.graph.nodes.iter().filter(|n| n.id == id);
    let node = nodes
        .next()
        .ok_or_else(|| error("missing byte graph participant"))?;
    if nodes.next().is_some() {
        return Err(error("byte graph participants require unique identities"));
    }
    Ok(node)
}
fn participant(core: &AilCore, node: &Node) -> Result<(), String> {
    unique_node(core, &node.id)?;
    let mut targets = BTreeSet::new();
    for edge in core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == "has_provenance")
    {
        if !edge.attributes.is_empty()
            || !targets.insert(&edge.target)
            || unique_node(core, &edge.target)?.kind != "Provenance"
        {
            return Err(error("invalid byte participant provenance"));
        }
    }
    Ok(())
}
fn checked_field(
    core: &AilCore,
    node: &Node,
    owner: Option<&Node>,
    ty: &str,
) -> Result<String, String> {
    participant(core, node)?;
    let key = float_operations::scoped_field(core, node, owner, ty).map_err(error)?;
    for edge in core.graph.edges.iter().filter(|e| {
        e.target == node.id
            && matches!(
                e.kind.as_str(),
                "has_field" | "value_parameter" | "value_local"
            )
    }) {
        participant(core, unique_node(core, &edge.source)?)?;
    }
    Ok(key)
}
fn child<'a>(core: &'a AilCore, node: &Node, role: &str) -> Result<&'a Node, String> {
    let mut edges = core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == role);
    let edge = edges
        .next()
        .ok_or_else(|| error("missing byte operand edge"))?;
    if edges.next().is_some() || !edge.attributes.is_empty() {
        return Err(error("duplicate or attributed byte operand edge"));
    }
    unique_node(core, &edge.target)
}
fn expression(
    core: &AilCore,
    root: &Node,
    owner: Option<&Node>,
    parent: &Node,
    role: &str,
) -> Result<IntExpr, String> {
    let mut pending = vec![(root, 1usize, parent.id.as_str(), role, false)];
    let mut visited = BTreeSet::new();
    let mut tokens = Vec::new();
    let mut bytes = 0usize;
    while let Some((node, depth, parent, role, done)) = pending.pop() {
        if done {
            tokens.push(node.attributes["operator"].clone());
            continue;
        }
        if !visited.insert(&node.id)
            || visited.len() > crate::expression::MAX_EXPRESSION_NODES
            || depth > crate::expression::MAX_EXPRESSION_DEPTH
        {
            return Err(error("byte expression exceeds tree bounds or shares nodes"));
        }
        participant(core, node)?;
        if node.kind != "IntegerExpression" || node.type_name.as_deref() != Some("Int") {
            return Err(error(
                "byte numeric operand requires typed IntegerExpression",
            ));
        }
        let incoming = core
            .graph
            .edges
            .iter()
            .filter(|e| e.target == node.id)
            .collect::<Vec<_>>();
        if incoming.len() != 1 || incoming[0].source != parent || incoming[0].kind != role {
            return Err(error(
                "byte expression requires exactly one expected parent",
            ));
        }
        let edge = incoming[0];
        if role == "value_expression_operand" {
            if edge.attributes.len() != 1 || !edge.attributes.contains_key("name") {
                return Err(error("invalid local byte operand metadata"));
            }
        } else if !edge.attributes.is_empty() {
            return Err(error("unexpected byte expression edge attributes"));
        }
        let kind = node
            .attributes
            .get("expression_kind")
            .map(String::as_str)
            .unwrap_or("");
        let (mut attrs, roles, token) = match kind {
            "literal" => (
                vec!["expression_kind", "value"],
                vec![],
                format!(
                    "i:{}",
                    node.attributes
                        .get("value")
                        .ok_or_else(|| error("missing byte bound value"))?
                ),
            ),
            "field" => (
                vec!["expression_kind"],
                vec!["expression_field"],
                format!(
                    "f:{}",
                    checked_field(core, child(core, node, "expression_field")?, owner, "Int")?
                ),
            ),
            "binary" => (
                vec!["expression_kind", "operator"],
                vec!["left_operand", "right_operand"],
                node.attributes
                    .get("operator")
                    .ok_or_else(|| error("missing byte bound operator"))?
                    .clone(),
            ),
            _ => return Err(error("invalid byte expression kind")),
        };
        if let Some(owner) = owner {
            attrs.push("function_owner");
            if node.attributes.get("function_owner") != Some(&owner.name) {
                return Err(error("byte expression belongs to another function"));
            }
        }
        if node.attributes.len() != attrs.len()
            || attrs.iter().any(|a| !node.attributes.contains_key(*a))
        {
            return Err(error("unexpected byte expression metadata"));
        }
        for edge in core.graph.edges.iter().filter(|e| e.source == node.id) {
            if edge.kind != "has_provenance" && !roles.contains(&edge.kind.as_str()) {
                return Err(error("unexpected byte expression edge"));
            }
        }
        bytes = bytes.saturating_add(token.len()).saturating_add(1);
        if bytes > crate::expression::MAX_EXPRESSION_BYTES + 1 {
            return Err(error("byte expression exceeds raw input byte limit"));
        }
        if kind == "literal" {
            crate::expression::parse_integer(&node.attributes["value"]).map_err(error)?;
        } else if kind == "binary" && !matches!(token.as_str(), "+" | "-" | "*" | "/" | "%") {
            return Err(error("invalid byte expression operator"));
        }
        if kind == "binary" {
            pending.push((node, depth, parent, role, true));
            pending.push((
                child(core, node, "right_operand")?,
                depth + 1,
                &node.id,
                "right_operand",
                false,
            ));
            pending.push((
                child(core, node, "left_operand")?,
                depth + 1,
                &node.id,
                "left_operand",
                false,
            ));
        } else {
            tokens.push(token);
        }
    }
    IntExpr::from_postfix(&tokens.join(" ")).map_err(error)
}
pub(super) fn decode_local_expression(
    core: &AilCore,
    root: &Node,
    function: &Node,
    step: &Node,
    role: &str,
) -> Result<IntExpr, String> {
    participant(core, function)?;
    participant(core, step)?;
    let opcode = step
        .attributes
        .get("opcode")
        .map(String::as_str)
        .unwrap_or("");
    let valid = matches!(
        (opcode, role),
        ("BYTES_GET", "index")
            | ("BYTES_SLICE", "start" | "end")
            | ("BYTES_SET", "index" | "value")
    );
    if !valid
        || step.kind != "ValueFunctionStep"
        || step.attributes.get("function_owner") != Some(&function.name)
    {
        return Err(error("invalid local byte expression owner"));
    }
    let edge = core
        .graph
        .edges
        .iter()
        .find(|e| {
            e.source == step.id && e.target == root.id && e.kind == "value_expression_operand"
        })
        .ok_or_else(|| error("missing local byte expression edge"))?;
    if edge.attributes.get("name").map(String::as_str) != Some(role) {
        return Err(error("incorrect local byte operand role"));
    }
    expression(core, root, Some(function), step, "value_expression_operand")
}

/// Add Bytes-specific identity, provenance and tree checks to the common typed
/// function decoder, without changing older operation families.
pub(super) fn validate_local_operation(
    core: &AilCore,
    step: &Node,
    owner: &Node,
) -> Result<(), String> {
    participant(core, step)?;
    participant(core, owner)?;
    let kind = step
        .attributes
        .get("opcode")
        .and_then(|opcode| OPCODES.iter().position(|op| *op == opcode))
        .ok_or_else(|| error("invalid local Bytes opcode"))?;
    for (name, _, ty) in schema(kind) {
        let role = if ty == "expression" {
            "value_expression_operand"
        } else {
            "value_field_operand"
        };
        let mut edges = core.graph.edges.iter().filter(|e| {
            e.source == step.id
                && e.kind == role
                && e.attributes.get("name").is_some_and(|n| n == name)
        });
        let edge = edges
            .next()
            .ok_or_else(|| error("missing local byte operand"))?;
        if edges.next().is_some() || edge.attributes.len() != 1 {
            return Err(error("invalid local byte operand metadata"));
        }
        let node = unique_node(core, &edge.target)?;
        if ty == "expression" {
            decode_local_expression(core, node, owner, step, name)?;
        } else {
            checked_field(core, node, Some(owner), ty)?;
        }
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
fn valid_step(edge: &Edge) -> bool {
    edge.attributes.contains_key("step_index")
        && edge.attributes.iter().all(|(k, v)| {
            matches!(k.as_str(), "step_index" | "operation_index") && v.parse::<usize>().is_ok()
        })
}
fn decode(core: &AilCore, node: &Node) -> Result<AilBytesOperation, String> {
    participant(core, node)?;
    if node.kind != "BytesOperation" || node.type_name.is_some() || node.attributes.len() != 1 {
        return Err(error("byte operation requires only bytes_kind metadata"));
    }
    let kind = node
        .attributes
        .get("bytes_kind")
        .and_then(|s| KINDS.iter().position(|k| *k == s))
        .ok_or_else(|| error("unknown bytes_kind"))?;
    let incoming = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect::<Vec<_>>();
    if incoming.len() != 1 || incoming[0].kind != "bytes_step" || !valid_step(incoming[0]) {
        return Err(error(
            "byte operation requires one ordered executable owner",
        ));
    }
    let owner = unique_node(core, &incoming[0].source)?;
    participant(core, owner)?;
    if !executable(owner) {
        return Err(error("byte operation owner must be executable Action"));
    }
    let schema = schema(kind);
    let mut operands = BTreeMap::new();
    for edge in core.graph.edges.iter().filter(|e| e.source == node.id) {
        if edge.kind != "has_provenance" && !schema.iter().any(|(_, role, _)| *role == edge.kind) {
            return Err(error("unexpected byte operation edge"));
        }
    }
    for (name, role, ty) in schema {
        let target = child(core, node, role)?;
        let value = if ty == "expression" {
            expression(core, target, None, node, role)?.to_postfix()
        } else {
            checked_field(core, target, None, ty)?
        };
        operands.insert(name.into(), value);
    }
    from_instruction(&AilBytecodeInstruction {
        opcode: OPCODES[kind].into(),
        operands,
    })
}
pub(super) fn from_core(core: &AilCore, owner: &Node) -> Vec<AilBytesOperation> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == owner.id && e.kind == "bytes_step")
        .map(|e| {
            unique_node(core, &e.target)
                .and_then(|n| decode(core, n))
                .unwrap_or_else(AilBytesOperation::Invalid)
        })
        .collect()
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut result = Vec::new();
    for node in core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind == "BytesOperation")
    {
        let check = if matches!(core.package.profile.as_str(), "Application" | "C interop") {
            decode(core, node).map(|_| ())
        } else {
            Err(error(
                "Bytes operations require Application or C interop profile",
            ))
        };
        if let Err(message) = check {
            result.push(
                AilDiagnostic::error("AIL-BYTES-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for edge in &core.graph.edges {
        let source = unique_node(core, &edge.source).ok();
        let target = unique_node(core, &edge.target).ok();
        let valid = match edge.kind.as_str() {
            "bytes_step" => {
                source.is_some_and(executable)
                    && target.is_some_and(|n| n.kind == "BytesOperation")
                    && valid_step(edge)
            }
            "bytes_source" | "bytes_other" | "bytes_destination" => {
                source.is_some_and(|n| n.kind == "BytesOperation")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            "bytes_index" | "bytes_start" | "bytes_end" | "bytes_value" => {
                source.is_some_and(|n| n.kind == "BytesOperation")
                    && target.is_some_and(|n| n.kind == "IntegerExpression")
                    && edge.attributes.is_empty()
            }
            _ => true,
        };
        if !valid {
            result.push(
                AilDiagnostic::error("AIL-BYTES-001", "invalid byte structural edge")
                    .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    result
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let mut result = Vec::new();
    for step in core.graph.edges.iter().filter(|e| e.kind == "bytes_step") {
        let mut pending = Vec::new();
        for edge in core.graph.edges.iter().filter(|e| e.source == step.target) {
            match (kind, edge.kind.as_str()) {
                ("reads", "bytes_source" | "bytes_other") | ("writes", "bytes_destination") => {
                    result.push((edge, step.source.as_str()))
                }
                ("reads", "bytes_index" | "bytes_start" | "bytes_end" | "bytes_value") => {
                    pending.push(edge.target.as_str())
                }
                _ => {}
            }
        }
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            for edge in core.graph.edges.iter().filter(|e| e.source == id) {
                match edge.kind.as_str() {
                    "expression_field" => result.push((edge, step.source.as_str())),
                    "left_operand" | "right_operand" => pending.push(edge.target.as_str()),
                    _ => {}
                }
            }
        }
    }
    result
}
fn status(code: u64) -> Result<(), String> {
    Err(match code {
        0 => return Ok(()),
        1 => "AIL-RUNTIME-BYTES: invalid or missing byte value",
        2 => "AIL-RUNTIME-BYTES-LIMIT: byte value exceeds allocation or size limit",
        3 => "AIL-RUNTIME-TEXT: invalid or missing text value",
        4 => "AIL-RUNTIME-TEXT-LIMIT: text value exceeds size limit",
        5 => "AIL-RUNTIME-INDEX: byte index is out of range",
        _ => "AIL-RUNTIME-IO: byte engine request failed",
    }
    .into())
}
pub(super) fn runtime_validate(value: &str) -> Result<(), String> {
    crate::byte_engine::validate_transport(value.as_bytes())
        .map(|_| ())
        .map_err(|s| status(s).unwrap_err())
}
fn request(
    kind: usize,
    left: Option<&[u8]>,
    right: Option<&[u8]>,
) -> crate::byte_engine::ByteRequest {
    crate::byte_engine::ByteRequest {
        kind: kind as u64,
        left: left.map_or(std::ptr::null(), |s| s.as_ptr()),
        left_len: left.map_or(0, |s| s.len() as u64),
        right: right.map_or(std::ptr::null(), |s| s.as_ptr()),
        right_len: right.map_or(0, |s| s.len() as u64),
        first: 0,
        second: 0,
        output: std::ptr::null_mut(),
        output_capacity: 0,
    }
}
fn run(
    request: &mut crate::byte_engine::ByteRequest,
    capacity: usize,
) -> Result<(Vec<u8>, i64), String> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(capacity)
        .map_err(|_| "AIL-RUNTIME-BYTES-LIMIT: byte scratch allocation failed".to_string())?;
    output.resize(capacity, 0);
    request.output = output.as_mut_ptr();
    request.output_capacity = capacity as u64;
    let mut result = crate::byte_engine::ByteResult {
        output_len: 0,
        scalar: 0,
    };
    // Inputs are borrowed for the call and output is a disjoint owned buffer.
    status(unsafe { crate::byte_engine::run(request, &mut result) })?;
    output.truncate(result.output_len as usize);
    Ok((output, result.scalar))
}
pub(super) fn runtime_from_raw(value: &[u8]) -> Result<String, String> {
    let mut request = request(10, Some(value), None);
    // Validate before calculating or allocating the encoded result.
    status(unsafe { crate::byte_engine::validate(&request) })?;
    let (output, _) = run(&mut request, 4 + 2 * value.len())?;
    String::from_utf8(output)
        .map_err(|_| "AIL-RUNTIME-IO: byte engine produced invalid transport".into())
}
pub(super) fn runtime_to_raw(value: &str) -> Result<Vec<u8>, String> {
    let mut request = request(11, Some(value.as_bytes()), None);
    status(unsafe { crate::byte_engine::validate(&request) })?;
    run(&mut request, (value.len() - 4) / 2).map(|r| r.0)
}
pub(super) fn reserve_result(length: usize, allocated: &mut usize) -> Result<(), String> {
    if !(4..=MAX_TRANSPORT).contains(&length)
        || allocated
            .checked_add(length)
            .is_none_or(|total| total > MAX_ALLOCATED)
    {
        return status(2);
    }
    *allocated += length;
    Ok(())
}
pub(super) fn execute(
    i: &AilBytecodeInstruction,
    state: &mut BTreeMap<String, String>,
    text: &mut usize,
    list: &mut usize,
) -> Result<(), String> {
    verify(i)?;
    let kind = OPCODES.iter().position(|op| *op == i.opcode).unwrap();
    let left = state
        .get(
            &i.operands[if matches!(kind, 3 | 9) {
                "left"
            } else {
                "source"
            }],
        )
        .map(|s| s.as_bytes());
    let right = if matches!(kind, 3 | 9) {
        state.get(&i.operands["right"]).map(|s| s.as_bytes())
    } else {
        None
    };
    let mut request = request(kind, left, right);
    status(unsafe { crate::byte_engine::validate(&request) })?;
    for (name, _, ty) in schema(kind) {
        if ty == "expression" {
            let value = IntExpr::from_postfix(&i.operands[name])
                .map_err(error)?
                .eval(state)?;
            if matches!(name, "index" | "start") {
                request.first = value;
            } else {
                request.second = value;
            }
        }
    }
    let length = left.unwrap().len();
    let capacity = match kind {
        0 | 1 | 9 => 0,
        2 | 4 => length,
        3 => (length + right.unwrap().len() - 4).min(MAX_TRANSPORT),
        5 => 4 + 2 * length,
        6 => (length - 4) / 2,
        7 => length + 4,
        8 => length - 4,
        _ => unreachable!(),
    };
    let (output, scalar) = run(&mut request, capacity)?;
    let value = if matches!(kind, 0 | 1 | 9) {
        scalar.to_string()
    } else {
        let value = String::from_utf8(output)
            .map_err(|_| "AIL-RUNTIME-IO: byte engine returned invalid text".to_string())?;
        if matches!(kind, 6 | 8) {
            text_operations::reserve_result(value.len(), text)?;
        } else {
            reserve_result(value.len(), list)?;
        }
        value
    };
    state.insert(i.operands["key"].clone(), value);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn i(opcode: &str, values: &[(&str, &str)]) -> AilBytecodeInstruction {
        AilBytecodeInstruction {
            opcode: opcode.into(),
            operands: attr(values),
        }
    }
    #[test]
    fn all_source_forms_roundtrip_and_have_exact_protocol() {
        for source in [
            "counts bytes in data.a into data.n",
            "gets byte -1 from bytes data.a into data.n",
            "slices bytes data.a from -3 to data.n + 1 into data.b",
            "joins bytes data.a and data.b into data.a",
            "sets byte data.n in bytes data.a to 255 into data.b",
            "encodes text data.t as UTF-8 bytes into data.b",
            "decodes bytes data.a as UTF-8 text into data.t",
            "decodes hexadecimal text data.t into bytes data.b",
            "encodes bytes data.a as hexadecimal text into data.t",
            "compares bytes data.a with data.b into data.n",
        ] {
            let source = format!("the system {source}");
            let op = parse_bullet(&source)
                .expect("recognized Bytes form")
                .unwrap();
            assert_eq!(parse_bullet(&op.source()).unwrap().unwrap(), op);
            let mut instruction = instruction(&op).unwrap();
            verify(&instruction).unwrap();
            instruction
                .operands
                .insert("ignored".into(), "data.a".into());
            assert!(verify(&instruction).is_err());
        }
    }
    #[test]
    fn all_bytes_and_empty_transport_are_distinct_from_missing() {
        let raw = (0..=255).collect::<Vec<u8>>();
        let encoded = runtime_from_raw(&raw).unwrap();
        assert_eq!(runtime_to_raw(&encoded).unwrap(), raw);
        assert_eq!(runtime_from_raw(&[]).unwrap(), "hex:");
        for bad in ["", "hex:0", "hex:FF", " hex:00", "hex:00\n", "HEX:00"] {
            assert!(
                runtime_validate(bad)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-BYTES:")
            );
        }
    }
    #[test]
    fn input_then_all_expressions_then_bounds_and_atomic_commit() {
        let mut state = attr(&[("data.a", "hex:00"), ("data.out", "sentinel")]);
        let mut text = 17;
        let mut list = 23;
        let op = i(
            "BYTES_SET",
            &[
                ("source", "data.a"),
                ("index", "i:9"),
                ("value", "i:1 i:0 /"),
                ("key", "data.out"),
            ],
        );
        assert!(
            execute(&op, &mut state, &mut text, &mut list)
                .unwrap_err()
                .contains("DIVZERO")
        );
        assert_eq!(state["data.out"], "sentinel");
        assert_eq!((text, list), (17, 23));
        state.insert("data.a".into(), "secret bad input".into());
        let err = execute(&op, &mut state, &mut text, &mut list).unwrap_err();
        assert!(err.starts_with("AIL-RUNTIME-BYTES:"));
        assert!(!err.contains("secret"));
    }
    #[test]
    fn aliases_and_two_arenas_charge_only_successful_result() {
        let mut state = attr(&[("data.a", "hex:00ff"), ("data.b", "hex:80")]);
        let mut text = 0;
        let mut list = 0;
        execute(
            &i(
                "BYTES_CONCAT",
                &[("left", "data.a"), ("right", "data.b"), ("key", "data.a")],
            ),
            &mut state,
            &mut text,
            &mut list,
        )
        .unwrap();
        assert_eq!(state["data.a"], "hex:00ff80");
        assert_eq!((text, list), (0, 10));
        execute(
            &i("BYTES_TO_HEX", &[("source", "data.a"), ("key", "data.t")]),
            &mut state,
            &mut text,
            &mut list,
        )
        .unwrap();
        assert_eq!((text, list), (6, 10));
        list = 67_108_861;
        let before = state.clone();
        assert!(
            execute(
                &i(
                    "BYTES_SLICE",
                    &[
                        ("source", "data.a"),
                        ("start", "i:9"),
                        ("end", "i:0"),
                        ("key", "data.a")
                    ]
                ),
                &mut state,
                &mut text,
                &mut list
            )
            .unwrap_err()
            .contains("BYTES-LIMIT")
        );
        assert_eq!(state, before);
        assert_eq!((text, list), (6, 67_108_861));
    }
    fn core(body: &str) -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = format!(
            "The application Bytes manages bytes.\n\nA Data has:\n- a: Bytes\n- b: Bytes\n- n: Int\n- t: Text\n\nAction: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Done\n"
        );
        elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, &source).unwrap(),
        )
    }
    fn rejects(core: &AilCore) {
        assert!(!check(core).is_empty(), "accepted malformed direct Core");
        if let Ok(saved) = parse_ail_core_text(&render_ail_core(core)) {
            assert!(!check(&saved).is_empty(), "accepted malformed saved Core");
        }
    }
    #[test]
    fn core_roundtrips_qualified_fields_and_derives_all_static_accesses() {
        let original =
            core("- the system sets byte data.n in bytes data.a to data.n + 1 into data.b");
        assert!(check(&original).is_empty(), "{:?}", check(&original));
        let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        assert!(check(&saved).is_empty());
        let mut structural = saved.clone();
        structural
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        assert_eq!(accesses(&structural, "reads").len(), 3);
        assert_eq!(accesses(&structural, "writes").len(), 1);
        let action = structural
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "Action" && n.name == "Run")
            .unwrap();
        let op = from_core(&structural, action).remove(0);
        let namespaced = namespace(&op, "Child");
        let i = instruction(&namespaced).unwrap();
        assert_eq!(i.operands["source"], "child.data.a");
        assert_eq!(i.operands["value"], "f:child.data.n i:1 +");
    }
    #[test]
    fn core_rejects_every_ambiguous_participant_and_provenance_identity() {
        let original = core("- the system sets byte data.n in bytes data.a to 255 into data.b");
        assert!(check(&original).is_empty());
        for node in original.graph.nodes.iter().filter(|n| {
            matches!(
                n.kind.as_str(),
                "Action"
                    | "Thing"
                    | "Field"
                    | "BytesOperation"
                    | "IntegerExpression"
                    | "Provenance"
            )
        }) {
            // Only provenance reachable from byte participants is part of this contract.
            if node.kind == "Provenance"
                && !original.graph.edges.iter().any(|e| {
                    e.target == node.id
                        && original.graph.nodes.iter().any(|n| {
                            n.id == e.source
                                && matches!(
                                    n.kind.as_str(),
                                    "Action"
                                        | "Thing"
                                        | "Field"
                                        | "BytesOperation"
                                        | "IntegerExpression"
                                )
                        })
                })
            {
                continue;
            }
            // data.t is intentionally unused and not a participant.
            if node.kind == "Field" && expression_core::field_key(node) == "data.t" {
                continue;
            }
            if node.kind == "Provenance"
                && original.graph.edges.iter().any(|e| {
                    e.target == node.id
                        && original
                            .graph
                            .nodes
                            .iter()
                            .any(|n| n.id == e.source && expression_core::field_key(n) == "data.t")
                })
            {
                continue;
            }
            let mut bad = original.clone();
            bad.graph.nodes.push(node.clone());
            rejects(&bad);
        }
    }
    #[test]
    fn core_rejects_sharing_foreign_parents_hidden_tokens_and_raw_overflow() {
        let original = core("- the system slices bytes data.a from 0 + 1 to 2 + 3 into data.b");
        for mutation in [
            "shared-root",
            "shared-descendant",
            "foreign-parent",
            "operator-tokens",
            "raw-overflow",
        ] {
            let mut bad = original.clone();
            let root = bad
                .graph
                .edges
                .iter()
                .find(|e| e.kind == "bytes_start")
                .unwrap()
                .target
                .clone();
            match mutation {
                "shared-root" => {
                    bad.graph
                        .edges
                        .iter_mut()
                        .find(|e| e.kind == "bytes_end")
                        .unwrap()
                        .target = root
                }
                "shared-descendant" => {
                    let left = bad
                        .graph
                        .edges
                        .iter()
                        .find(|e| e.source == root && e.kind == "left_operand")
                        .unwrap()
                        .target
                        .clone();
                    bad.graph
                        .edges
                        .iter_mut()
                        .find(|e| e.source == root && e.kind == "right_operand")
                        .unwrap()
                        .target = left;
                }
                "foreign-parent" => {
                    let mut extra = bad
                        .graph
                        .edges
                        .iter()
                        .find(|e| e.kind == "bytes_start")
                        .unwrap()
                        .clone();
                    extra.id = "forged-parent".into();
                    extra.source = bad
                        .graph
                        .nodes
                        .iter()
                        .find(|n| n.kind == "Action")
                        .unwrap()
                        .id
                        .clone();
                    bad.graph.edges.push(extra);
                }
                "operator-tokens" => {
                    bad.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == root)
                        .unwrap()
                        .attributes
                        .insert("operator".into(), "i:2 + +".into());
                }
                _ => {
                    let literal = bad
                        .graph
                        .nodes
                        .iter_mut()
                        .find(|n| {
                            n.kind == "IntegerExpression"
                                && n.attributes
                                    .get("expression_kind")
                                    .is_some_and(|v| v == "literal")
                        })
                        .unwrap();
                    literal
                        .attributes
                        .insert("value".into(), "0".repeat(65_535));
                }
            }
            rejects(&bad);
        }
    }
    #[test]
    fn exact_raw_postfix_bound_is_checked_before_literal_normalization() {
        let mut candidate = core("- the system gets byte 0 from bytes data.a into data.n");
        let index = candidate
            .graph
            .nodes
            .iter()
            .position(|n| n.kind == "IntegerExpression")
            .unwrap();
        candidate.graph.nodes[index]
            .attributes
            .insert("value".into(), "0".repeat(65_534));
        assert!(check(&candidate).is_empty());
        let saved = parse_ail_core_text(&render_ail_core(&candidate)).unwrap();
        assert!(check(&saved).is_empty());
        candidate.graph.nodes[index]
            .attributes
            .insert("value".into(), "0".repeat(65_535));
        rejects(&candidate);
    }
    #[test]
    fn malformed_reserved_forms_are_rejected_without_claiming_text_or_lists() {
        for body in [
            "counts bytes",
            "gets byte",
            "slices bytes",
            "joins bytes",
            "sets byte",
            "encodes text",
            "decodes bytes",
            "decodes hexadecimal",
            "encodes bytes",
            "compares bytes",
        ] {
            assert!(
                parse_bullet(&format!("the system {body}"))
                    .unwrap()
                    .is_err()
            );
            assert!(recognized_bullet(&format!("the function {body}")));
        }
        for body in [
            "counts characters in data.t into data.n",
            "slices list data.a from 0 to 1 into data.b",
            "joins text data.t and data.t into data.t",
        ] {
            assert!(parse_bullet(&format!("the system {body}")).is_none());
        }
    }
    #[test]
    fn local_byte_trees_enforce_raw_limits_parent_scope_and_participant_identity() {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Bytes manages bytes.\n\nA Data has:\n- a: Bytes\n- b: Bytes\n\nFunction: window.\nThe function needs:\n- raw: Bytes\nThe function produces:\n- result: Bytes\nThe function stores:\n- result: Bytes\nThe function executes:\n- the function slices bytes local.raw from 0 to 1 into local.result\n- the function returns local.result\n\nAction: Run.\nWhen Run happens:\n- the system calls window with raw = data.a into data.b\n- the system records a trace event named Done\n";
        let original = elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, source).unwrap(),
        );
        assert!(
            check_ail_core(&original).is_empty(),
            "{:?}",
            check_ail_core(&original)
        );
        let root = original
            .graph
            .edges
            .iter()
            .find(|e| {
                e.kind == "value_expression_operand"
                    && e.attributes.get("name").is_some_and(|n| n == "start")
            })
            .unwrap()
            .target
            .clone();
        let mut exact = original.clone();
        exact
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.id == root)
            .unwrap()
            .attributes
            .insert("value".into(), "0".repeat(65_534));
        compile_ail_core_bytecode(&exact).unwrap();
        compile_ail_core_bytecode(&parse_ail_core_text(&render_ail_core(&exact)).unwrap()).unwrap();
        for mutation in ["raw", "shared", "owner", "identity", "provenance"] {
            let mut bad = original.clone();
            match mutation {
                "raw" => {
                    bad.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == root)
                        .unwrap()
                        .attributes
                        .insert("value".into(), "0".repeat(65_535));
                }
                "shared" => {
                    bad.graph
                        .edges
                        .iter_mut()
                        .find(|e| {
                            e.kind == "value_expression_operand"
                                && e.attributes.get("name").is_some_and(|n| n == "end")
                        })
                        .unwrap()
                        .target = root.clone()
                }
                "owner" => {
                    bad.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == root)
                        .unwrap()
                        .attributes
                        .insert("function_owner".into(), "foreign".into());
                }
                "identity" => bad.graph.nodes.push(
                    original
                        .graph
                        .nodes
                        .iter()
                        .find(|n| n.id == root)
                        .unwrap()
                        .clone(),
                ),
                _ => {
                    let edge = original
                        .graph
                        .edges
                        .iter()
                        .find(|e| e.source == root && e.kind == "has_provenance")
                        .unwrap();
                    bad.graph.nodes.push(
                        original
                            .graph
                            .nodes
                            .iter()
                            .find(|n| n.id == edge.target)
                            .unwrap()
                            .clone(),
                    );
                }
            }
            assert!(
                compile_ail_core_bytecode(&bad).is_err(),
                "accepted {mutation}"
            );
            if let Ok(saved) = parse_ail_core_text(&render_ail_core(&bad)) {
                assert!(
                    compile_ail_core_bytecode(&saved).is_err(),
                    "saved accepted {mutation}"
                );
            }
        }
    }
    #[test]
    fn utf8_hex_and_result_boundaries_preserve_atomicity() {
        let raw = vec![255; MAX_RAW];
        let canonical = runtime_from_raw(&raw).unwrap();
        assert_eq!(canonical.len(), MAX_TRANSPORT);
        assert_eq!(runtime_to_raw(&canonical).unwrap(), raw);
        assert!(
            runtime_from_raw(&vec![0; MAX_RAW + 1])
                .unwrap_err()
                .contains("BYTES-LIMIT")
        );
        let mut state = attr(&[("data.a", &canonical), ("data.out", "preserved")]);
        let mut text = 0;
        let mut list = 0;
        for (opcode, category) in [("BYTES_TO_HEX", "TEXT-LIMIT"), ("BYTES_TO_UTF8", "TEXT:")] {
            assert!(
                execute(
                    &i(opcode, &[("source", "data.a"), ("key", "data.out")]),
                    &mut state,
                    &mut text,
                    &mut list
                )
                .unwrap_err()
                .contains(category)
            );
            assert_eq!(state["data.out"], "preserved");
            assert_eq!((text, list), (0, 0));
        }
        state.insert("data.t".into(), "λ\0🙂".into());
        execute(
            &i(
                "BYTES_FROM_UTF8",
                &[("source", "data.t"), ("key", "data.a")],
            ),
            &mut state,
            &mut text,
            &mut list,
        )
        .unwrap();
        execute(
            &i("BYTES_TO_UTF8", &[("source", "data.a"), ("key", "data.t")]),
            &mut state,
            &mut text,
            &mut list,
        )
        .unwrap();
        assert_eq!(state["data.t"], "λ\0🙂");
        assert_eq!((text, list), (7, 18));
        state.insert("data.t".into(), "Aa00FF".into());
        execute(
            &i("BYTES_FROM_HEX", &[("source", "data.t"), ("key", "data.a")]),
            &mut state,
            &mut text,
            &mut list,
        )
        .unwrap();
        assert_eq!(state["data.a"], "hex:aa00ff");
    }
}
