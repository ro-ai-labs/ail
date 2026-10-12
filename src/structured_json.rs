//! Bounded recursive JSON values, lossless number tokens, and typed pointer access.
use super::*;
use crate::expression::IntExpr;

const MAX_BYTES: usize = 1_048_576;
const MAX_VALUES: usize = 65_536;
const MAX_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilJsonAccess {
    Get {
        source: String,
        path: String,
        key: String,
    },
    Count {
        source: String,
        key: String,
    },
    Kind {
        source: String,
        key: String,
    },
    Keys {
        source: String,
        key: String,
    },
    Invalid(String),
}
impl AilJsonAccess {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Get { source, path, key } => {
                format!("the system gets JSON value at path {path} from {source} into {key}")
            }
            Self::Count { source, key } => {
                format!("the system counts entries in JSON {source} into {key}")
            }
            Self::Kind { source, key } => {
                format!("the system gets JSON kind from {source} into {key}")
            }
            Self::Keys { source, key } => {
                format!("the system gets JSON keys from {source} into {key}")
            }
            Self::Invalid(message) => format!("invalid JSON access: {message}"),
        }
    }
    fn parts(&self) -> Result<(&str, &str, Option<&str>, &str), String> {
        match self {
            Self::Get { source, path, key } => Ok(("get", source, Some(path), key)),
            Self::Count { source, key } => Ok(("count", source, None, key)),
            Self::Kind { source, key } => Ok(("kind", source, None, key)),
            Self::Keys { source, key } => Ok(("keys", source, None, key)),
            Self::Invalid(message) => Err(error(message)),
        }
    }
}
fn error(message: impl Display) -> String {
    format!("AIL-JSON-001: {message}")
}
fn runtime_error(message: &str) -> String {
    format!("AIL-RUNTIME-JSON: {message}")
}
fn limit(message: &str) -> String {
    format!("AIL-RUNTIME-JSON-LIMIT: {message}")
}
fn path_error(message: &str) -> String {
    format!("AIL-RUNTIME-JSON-PATH: {message}")
}
fn supported(type_name: &str) -> bool {
    matches!(
        type_name,
        "Int" | "Float" | "Bool" | "Text" | "List<Int>" | "List<Float>" | "List<Text>" | "Json"
    )
}
fn field(key: &str) -> Result<(), String> {
    if matches!(IntExpr::parse(key),Ok(IntExpr::Field(parsed)) if parsed==key) {
        Ok(())
    } else {
        Err(error("JSON access operand must be a field reference"))
    }
}

// Number text is borrowed verbatim. Objects retain ordered member pairs, including
// duplicates; decoding keys here permits exact Unicode lookup and ambiguity checks.
enum Value<'a> {
    Null,
    Bool(bool),
    Number(&'a str),
    Text(String),
    Array(Vec<Value<'a>>),
    Object(Vec<(String, Value<'a>)>),
}
struct Parser<'a> {
    input: &'a str,
    position: usize,
    values: usize,
}
impl<'a> Parser<'a> {
    fn byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position).copied()
    }
    fn whitespace(&mut self) {
        while matches!(self.byte(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.position += 1;
        }
    }
    fn take(&mut self, byte: u8) -> bool {
        if self.byte() == Some(byte) {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn word(&mut self, word: &str) -> Result<(), String> {
        if self.input[self.position..].starts_with(word) {
            self.position += word.len();
            Ok(())
        } else {
            Err(runtime_error("invalid JSON value"))
        }
    }
    fn string(&mut self) -> Result<String, String> {
        let start = self.position;
        if !self.take(b'"') {
            return Err(runtime_error("expected JSON string"));
        }
        // Scan only this token before invoking the existing strict string decoder.
        // Decoding the remaining input for every string would be quadratic.
        while let Some(byte) = self.byte() {
            self.position += 1;
            if byte == b'"' {
                return literal_assignment::decode("Text", &self.input[start..self.position])
                    .map_err(|_| runtime_error("invalid JSON string"));
            }
            if byte == b'\\' {
                if self.byte().is_none() {
                    break;
                }
                self.position += 1;
            }
        }
        Err(runtime_error("unterminated JSON string"))
    }
    fn number(&mut self) -> Result<Value<'a>, String> {
        let start = self.position;
        self.take(b'-');
        if !self.take(b'0') {
            if !matches!(self.byte(), Some(b'1'..=b'9')) {
                return Err(runtime_error("invalid JSON number"));
            }
            while matches!(self.byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        if self.take(b'.') {
            if !matches!(self.byte(), Some(b'0'..=b'9')) {
                return Err(runtime_error("invalid JSON number"));
            }
            while matches!(self.byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        if matches!(self.byte(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.byte(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.byte(), Some(b'0'..=b'9')) {
                return Err(runtime_error("invalid JSON number"));
            }
            while matches!(self.byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        Ok(Value::Number(&self.input[start..self.position]))
    }
    fn value(&mut self, depth: usize) -> Result<Value<'a>, String> {
        self.whitespace();
        self.values += 1;
        if self.values > MAX_VALUES {
            return Err(limit("JSON value count exceeds 65536"));
        }
        match self.byte() {
            Some(b'n') => {
                self.word("null")?;
                Ok(Value::Null)
            }
            Some(b't') => {
                self.word("true")?;
                Ok(Value::Bool(true))
            }
            Some(b'f') => {
                self.word("false")?;
                Ok(Value::Bool(false))
            }
            Some(b'"') => self.string().map(Value::Text),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b'[' | b'{') if depth >= MAX_DEPTH => {
                Err(limit("JSON nesting exceeds 64 containers"))
            }
            Some(b'[') => {
                self.position += 1;
                self.whitespace();
                let mut values = Vec::new();
                if self.take(b']') {
                    return Ok(Value::Array(values));
                }
                loop {
                    values.push(self.value(depth + 1)?);
                    self.whitespace();
                    if self.take(b']') {
                        break;
                    }
                    if !self.take(b',') {
                        return Err(runtime_error("invalid JSON array delimiter"));
                    }
                }
                Ok(Value::Array(values))
            }
            Some(b'{') => {
                self.position += 1;
                self.whitespace();
                let mut members = Vec::new();
                if self.take(b'}') {
                    return Ok(Value::Object(members));
                }
                loop {
                    self.whitespace();
                    let key = self.string()?;
                    self.whitespace();
                    if !self.take(b':') {
                        return Err(runtime_error("invalid JSON object delimiter"));
                    }
                    members.push((key, self.value(depth + 1)?));
                    self.whitespace();
                    if self.take(b'}') {
                        break;
                    }
                    if !self.take(b',') {
                        return Err(runtime_error("invalid JSON object delimiter"));
                    }
                }
                Ok(Value::Object(members))
            }
            _ => Err(runtime_error("invalid JSON value")),
        }
    }
}
fn parse(input: &str) -> Result<Value<'_>, String> {
    if input.len() > MAX_BYTES {
        return Err(limit("JSON input exceeds 1048576 bytes"));
    }
    let mut parser = Parser {
        input,
        position: 0,
        values: 0,
    };
    let value = parser.value(0)?;
    parser.whitespace();
    if parser.position != input.len() {
        return Err(runtime_error("trailing JSON data"));
    }
    Ok(value)
}
fn string_bytes(text: &str) -> usize {
    2 + text
        .chars()
        .map(|c| match c {
            '"' | '\\' | '\n' | '\r' | '\t' => 2,
            c if c.is_control() => 6,
            c => c.len_utf8(),
        })
        .sum::<usize>()
}
fn encoded_size(value: &Value<'_>) -> Result<usize, String> {
    // Count after parsing all grammar. No canonical expansion is allocated until
    // its complete size is known to be within the encoded-byte bound.
    let bytes = match value {
        Value::Null | Value::Bool(true) => 4,
        Value::Bool(false) => 5,
        Value::Number(number) => number.len(),
        Value::Text(text) => string_bytes(text),
        Value::Array(values) => {
            let mut bytes = 2 + values.len().saturating_sub(1);
            for value in values {
                bytes += encoded_size(value)?;
            }
            bytes
        }
        Value::Object(members) => {
            let mut bytes = 2 + members.len().saturating_sub(1);
            for (key, value) in members {
                bytes += string_bytes(key) + 1 + encoded_size(value)?;
            }
            bytes
        }
    };
    if bytes > MAX_BYTES {
        Err(limit("canonical JSON exceeds 1048576 bytes"))
    } else {
        Ok(bytes)
    }
}
fn encode_into(value: &Value<'_>, output: &mut String) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(number) => output.push_str(number),
        Value::Text(text) => output.push_str(&json_string(text)),
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                encode_into(value, output);
            }
            output.push(']');
        }
        Value::Object(members) => {
            output.push('{');
            for (index, (key, value)) in members.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                output.push_str(&json_string(key));
                output.push(':');
                encode_into(value, output);
            }
            output.push('}');
        }
    }
}
fn encode(value: &Value<'_>) -> Result<String, String> {
    let bytes = encoded_size(value)?;
    let mut output = String::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| limit("cannot allocate JSON result"))?;
    encode_into(value, &mut output);
    Ok(output)
}
pub(super) fn canonical(input: &str) -> Result<String, String> {
    encode(&parse(input)?)
}

// A prepared mutation retains a validated source and target. Runtime callers can
// defer even fetching the value operand until source/path errors have been ruled
// out, without reparsing the source or charging an intermediate result.
pub(super) struct PreparedMutation<'a> {
    root: Value<'a>,
    ancestors: Vec<usize>,
    target: MutationTarget,
    setting: bool,
}
enum MutationTarget {
    Root,
    Object { key: String, index: Option<usize> },
    Array { index: usize, append: bool },
}
fn member_index(members: &[(String, Value<'_>)], key: &str) -> Result<Option<usize>, String> {
    let mut matching = members
        .iter()
        .enumerate()
        .filter(|(_, (name, _))| name == key);
    let index = matching.next().map(|(index, _)| index);
    if matching.next().is_some() {
        return Err(path_error("JSON member is ambiguous"));
    }
    Ok(index)
}
fn array_index(token: &str, length: usize) -> Result<usize, String> {
    if token.is_empty()
        || (token.len() > 1 && token.starts_with('0'))
        || !token.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(path_error("invalid JSON array index"));
    }
    token
        .parse::<usize>()
        .ok()
        .filter(|index| *index < length)
        .ok_or_else(|| path_error("JSON array index is out of range"))
}
pub(super) fn prepare_mutation<'a>(
    input: &'a str,
    path: Option<&str>,
    setting: bool,
) -> Result<PreparedMutation<'a>, String> {
    let root = parse(input)?;
    encoded_size(&root)?;
    let path = path.ok_or_else(|| runtime_error("missing JSON mutation path"))?;
    validate_path(path)?;
    if path.is_empty() {
        if !setting {
            return Err(path_error("cannot remove the JSON root"));
        }
        return Ok(PreparedMutation {
            root,
            ancestors: Vec::new(),
            target: MutationTarget::Root,
            setting,
        });
    }
    let mut ancestors = Vec::new();
    let mut parent = &root;
    let mut segments = path[1..].split('/').peekable();
    while let Some(token) = segments.next() {
        let key = token.replace("~1", "/").replace("~0", "~");
        if segments.peek().is_none() {
            let target = match parent {
                Value::Object(members) => {
                    let index = member_index(members, &key)?;
                    if !setting && index.is_none() {
                        return Err(path_error("JSON member does not exist"));
                    }
                    MutationTarget::Object { key, index }
                }
                Value::Array(values) => {
                    let append = setting && key == "-";
                    let index = if append {
                        values.len()
                    } else {
                        array_index(&key, values.len())?
                    };
                    MutationTarget::Array { index, append }
                }
                _ => return Err(path_error("JSON mutation parent must be a container")),
            };
            return Ok(PreparedMutation {
                root,
                ancestors,
                target,
                setting,
            });
        }
        let index = match parent {
            Value::Object(members) => member_index(members, &key)?
                .ok_or_else(|| path_error("JSON member does not exist"))?,
            Value::Array(values) => array_index(&key, values.len())?,
            _ => return Err(path_error("cannot traverse a scalar JSON value")),
        };
        ancestors.push(index);
        parent = match parent {
            Value::Object(members) => &members[index].1,
            Value::Array(values) => &values[index],
            _ => unreachable!("validated container"),
        };
    }
    unreachable!("nonempty path contains a final token")
}
fn check_structure(value: &Value<'_>, depth: usize, count: &mut usize) -> Result<(), String> {
    *count += 1;
    if *count > MAX_VALUES {
        return Err(limit("JSON value count exceeds 65536"));
    }
    if matches!(value, Value::Array(_) | Value::Object(_)) && depth >= MAX_DEPTH {
        return Err(limit("JSON nesting exceeds 64 containers"));
    }
    match value {
        Value::Array(values) => {
            for value in values {
                check_structure(value, depth + 1, count)?;
            }
        }
        Value::Object(members) => {
            for (_, value) in members {
                check_structure(value, depth + 1, count)?;
            }
        }
        _ => {}
    }
    Ok(())
}
impl PreparedMutation<'_> {
    pub(super) fn finish(self, replacement: Option<(&str, &str)>) -> Result<String, String> {
        if replacement.is_some() != self.setting {
            return Err(runtime_error(
                "mutation replacement does not match operation",
            ));
        }
        let formatted = replacement
            .map(|(kind, input)| json_codec::format_runtime_value(kind, input))
            .transpose()?;
        let replacement = formatted.as_deref().map(parse).transpose()?;
        // Declare root after formatted: borrowed numeric tokens in the inserted
        // tree remain live through encoding and are never stored as runtime state.
        let mut root = self.root;
        if matches!(self.target, MutationTarget::Root) {
            root = replacement.expect("prepared root set has a replacement");
        } else {
            let mut parent = &mut root;
            for index in self.ancestors {
                parent = match parent {
                    Value::Object(members) => &mut members[index].1,
                    Value::Array(values) => &mut values[index],
                    _ => unreachable!("prepared parent path"),
                };
            }
            match (self.target, parent, replacement) {
                (MutationTarget::Object { key, index }, Value::Object(members), Some(value)) => {
                    if let Some(index) = index {
                        members[index].1 = value;
                    } else {
                        members.push((key, value));
                    }
                }
                (
                    MutationTarget::Object {
                        index: Some(index), ..
                    },
                    Value::Object(members),
                    None,
                ) => {
                    members.remove(index);
                }
                (MutationTarget::Array { index, append }, Value::Array(values), Some(value)) => {
                    if append {
                        values.push(value);
                    } else {
                        values[index] = value;
                    }
                }
                (MutationTarget::Array { index, .. }, Value::Array(values), None) => {
                    values.remove(index);
                }
                _ => unreachable!("prepared target and operation"),
            }
        }
        check_structure(&root, 0, &mut 0)?;
        encode(&root)
    }
}
pub(super) fn mutate(
    input: &str,
    path: &str,
    replacement: Option<(&str, &str)>,
) -> Result<String, String> {
    prepare_mutation(input, Some(path), replacement.is_some())?.finish(replacement)
}

fn validate_path(path: &str) -> Result<(), String> {
    if path.len() > MAX_BYTES {
        return Err(limit("JSON path exceeds 1048576 bytes"));
    }
    if !path.is_empty() && !path.starts_with('/') {
        return Err(path_error("JSON path must be empty or begin with slash"));
    }
    let mut index = 0;
    let bytes = path.as_bytes();
    while index < bytes.len() {
        if bytes[index] == b'~' {
            if !matches!(bytes.get(index + 1), Some(b'0' | b'1')) {
                return Err(path_error("invalid JSON path escape"));
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(())
}
fn select<'v, 'a>(mut value: &'v Value<'a>, path: &str) -> Result<&'v Value<'a>, String> {
    validate_path(path)?;
    if path.is_empty() {
        return Ok(value);
    }
    for token in path[1..].split('/') {
        // Replacing ~1 before ~0 implements a single decode: ~01 remains ~1.
        let decoded;
        let token = if token.contains('~') {
            decoded = token.replace("~1", "/").replace("~0", "~");
            decoded.as_str()
        } else {
            token
        };
        value = match value {
            Value::Object(members) => {
                let mut matching = members.iter().filter(|(key, _)| key == token);
                let (_, selected) = matching
                    .next()
                    .ok_or_else(|| path_error("JSON member does not exist"))?;
                if matching.next().is_some() {
                    return Err(path_error("JSON member is ambiguous"));
                }
                selected
            }
            Value::Array(values) => {
                if token.is_empty()
                    || (token.len() > 1 && token.starts_with('0'))
                    || !token.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(path_error("invalid JSON array index"));
                }
                let index = token
                    .parse::<usize>()
                    .map_err(|_| path_error("JSON array index is out of range"))?;
                values
                    .get(index)
                    .ok_or_else(|| path_error("JSON array index is out of range"))?
            }
            _ => return Err(path_error("cannot traverse a scalar JSON value")),
        };
    }
    Ok(value)
}

pub(super) fn parse_bullet(text: &str) -> Option<Result<AilJsonAccess, String>> {
    let text = trim_sentence(text);
    let parsed = if let Some(rest) = text.strip_prefix("the system gets JSON value at path ") {
        rest.split_once(" from ").and_then(|(path, rest)| {
            rest.split_once(" into ")
                .map(|(source, key)| AilJsonAccess::Get {
                    source: source.into(),
                    path: path.into(),
                    key: key.into(),
                })
        })
    } else if let Some(rest) = text.strip_prefix("the system counts entries in JSON ") {
        rest.split_once(" into ")
            .map(|(source, key)| AilJsonAccess::Count {
                source: source.into(),
                key: key.into(),
            })
    } else if let Some(rest) = text.strip_prefix("the system gets JSON kind from ") {
        rest.split_once(" into ")
            .map(|(source, key)| AilJsonAccess::Kind {
                source: source.into(),
                key: key.into(),
            })
    } else if let Some(rest) = text.strip_prefix("the system gets JSON keys from ") {
        rest.split_once(" into ")
            .map(|(source, key)| AilJsonAccess::Keys {
                source: source.into(),
                key: key.into(),
            })
    } else {
        return (text.starts_with("the system gets JSON")
            || text.starts_with("the system counts entries in JSON"))
        .then(|| Err(error("malformed JSON access")));
    };
    Some(
        parsed
            .ok_or_else(|| error("malformed JSON access"))
            .and_then(|op| {
                let (_, source, path, key) = op.parts()?;
                field(source)?;
                field(key)?;
                if let Some(path) = path {
                    field(path)?;
                }
                Ok(op)
            }),
    )
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, op) in action.json_accesses.iter().enumerate() {
            let (kind, source, path, key) = op.parts().unwrap_or(("invalid", "", None, ""));
            let type_name = runtime_field_type_name(document, key).unwrap_or("invalid");
            let node = graph.add_node(
                "JsonAccess",
                format!("{}.json_access{index}", owner.name),
                None,
                attr(&[("json_access_kind", kind), ("value_type", type_name)]),
            );
            attach_provenance(
                graph,
                &node,
                format!("action:{}.json_access:{index}", owner.name),
            );
            if kind != "invalid" {
                let mut fields = vec![
                    ("json_access_source", source),
                    ("json_access_destination", key),
                ];
                if let Some(path) = path {
                    fields.push(("json_access_path", path));
                }
                for (edge_kind, key) in fields {
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
                        .unwrap_or_else(|| {
                            let target = graph.add_node("Effect", key, None, BTreeMap::new());
                            attach_provenance(
                                graph,
                                &target,
                                format!("action:{}.json_access:{index}.{edge_kind}", owner.name),
                            );
                            target
                        });
                    graph.add_edge(edge_kind, &node, &target, BTreeMap::new());
                }
            }
            action_order::append(
                graph,
                "json_access_step",
                &owner,
                &node,
                BTreeMap::new(),
                index,
            );
        }
    }
}
fn operand(core: &AilCore, node: &Node, kind: &str, type_name: &str) -> Result<String, String> {
    let mut edges = core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == kind);
    let edge = edges
        .next()
        .ok_or_else(|| error("JSON access has a missing operand"))?;
    if edges.next().is_some() || !edge.attributes.is_empty() {
        return Err(error(
            "JSON access requires one edge per operand without attributes",
        ));
    }
    let target = core
        .graph
        .nodes
        .iter()
        .find(|n| n.id == edge.target)
        .ok_or_else(|| error("JSON operand target is missing"))?;
    float_operations::scoped_field(core, target, None, type_name).map_err(error)
}
fn decode_node(core: &AilCore, node: &Node) -> Result<AilJsonAccess, String> {
    if node.kind != "JsonAccess"
        || node.type_name.is_some()
        || node
            .attributes
            .keys()
            .map(String::as_str)
            .ne(["json_access_kind", "value_type"])
    {
        return Err(error(
            "JSON access requires exactly json_access_kind and value_type attributes and no type_name",
        ));
    }
    let type_name = &node.attributes["value_type"];
    let kind = node.attributes["json_access_kind"].as_str();
    if !supported(type_name)
        || !matches!(kind, "get" | "count" | "kind" | "keys")
        || (kind == "count" && type_name != "Int")
        || (kind == "kind" && type_name != "Text")
        || (kind == "keys" && type_name != "List<Text>")
    {
        return Err(error("invalid JSON access kind or destination type"));
    }
    if core.graph.edges.iter().any(|e| {
        (e.source == node.id
            && !matches!(
                e.kind.as_str(),
                "json_access_source" | "json_access_destination" | "has_provenance"
            )
            && !(kind == "get" && e.kind == "json_access_path"))
            || (e.target == node.id && e.kind != "json_access_step")
    }) {
        return Err(error("unexpected edge on JSON access"));
    }
    let source = operand(core, node, "json_access_source", "Json")?;
    let key = operand(core, node, "json_access_destination", type_name)?;
    Ok(match kind {
        "get" => AilJsonAccess::Get {
            source,
            path: operand(core, node, "json_access_path", "Text")?,
            key,
        },
        "kind" => AilJsonAccess::Kind { source, key },
        "keys" => AilJsonAccess::Keys { source, key },
        _ => AilJsonAccess::Count { source, key },
    })
}
pub(super) fn from_core(core: &AilCore, action: &Node) -> Vec<AilJsonAccess> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == action.id && e.kind == "json_access_step")
        .map(|edge| {
            core.graph
                .nodes
                .iter()
                .find(|n| n.id == edge.target)
                .ok_or_else(|| error("missing JSON access node"))
                .and_then(|node| decode_node(core, node))
                .unwrap_or_else(AilJsonAccess::Invalid)
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
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    for node in core.graph.nodes.iter().filter(|n| n.kind == "JsonAccess") {
        let result = (|| {
            if !matches!(core.package.profile.as_str(), "Application" | "C interop") {
                return Err(error(
                    "JSON access requires Application or C interop profile",
                ));
            }
            let owners: Vec<_> = core
                .graph
                .edges
                .iter()
                .filter(|e| e.target == node.id && e.kind == "json_access_step")
                .collect();
            if owners.len() != 1
                || !core
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == owners[0].source && executable_action(n))
            {
                return Err(error(
                    "JSON access requires exactly one executable action owner",
                ));
            }
            decode_node(core, node).map(|_| ())
        })();
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-JSON-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for edge in &core.graph.edges {
        let source = core.graph.nodes.iter().find(|n| n.id == edge.source);
        let target = core.graph.nodes.iter().find(|n| n.id == edge.target);
        let valid = match edge.kind.as_str() {
            "json_access_step" => {
                source.is_some_and(executable_action)
                    && target.is_some_and(|n| n.kind == "JsonAccess")
                    && edge
                        .attributes
                        .get("step_index")
                        .is_some_and(|v| v.parse::<usize>().is_ok())
                    && edge.attributes.iter().all(|(k, v)| {
                        matches!(k.as_str(), "step_index" | "operation_index")
                            && v.parse::<usize>().is_ok()
                    })
            }
            "json_access_source" | "json_access_destination" | "json_access_path" => {
                source.is_some_and(|n| n.kind == "JsonAccess")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-JSON-001",
                    format!("invalid {} JSON access edge", edge.kind),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn namespace(op: &AilJsonAccess, alias: &str) -> AilJsonAccess {
    let prefix = runtime_subject_key(alias);
    match op {
        AilJsonAccess::Get { source, path, key } => AilJsonAccess::Get {
            source: format!("{prefix}.{source}"),
            path: format!("{prefix}.{path}"),
            key: format!("{prefix}.{key}"),
        },
        AilJsonAccess::Count { source, key } => AilJsonAccess::Count {
            source: format!("{prefix}.{source}"),
            key: format!("{prefix}.{key}"),
        },
        AilJsonAccess::Kind { source, key } => AilJsonAccess::Kind {
            source: format!("{prefix}.{source}"),
            key: format!("{prefix}.{key}"),
        },
        AilJsonAccess::Keys { source, key } => AilJsonAccess::Keys {
            source: format!("{prefix}.{source}"),
            key: format!("{prefix}.{key}"),
        },
        AilJsonAccess::Invalid(message) => AilJsonAccess::Invalid(message.clone()),
    }
}
pub(super) fn instruction(
    op: &AilJsonAccess,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let (kind, source, path, key) = op.parts()?;
    field(source)?;
    field(key)?;
    if runtime_field_type_name(document, source) != Some("Json") {
        return Err(error("JSON access source must be declared Json"));
    }
    let type_name = runtime_field_type_name(document, key)
        .filter(|t| supported(t))
        .ok_or_else(|| error("JSON access destination must have a supported declared type"))?;
    if let Some(path) = path {
        field(path)?;
        if runtime_field_type_name(document, path) != Some("Text") {
            return Err(error("JSON path must be declared Text"));
        }
        Ok(AilBytecodeInstruction::new(
            "JSON_GET",
            &[
                ("source", source.into()),
                ("path", path.into()),
                ("key", key.into()),
                ("type", type_name.into()),
            ],
        ))
    } else if let Some(opcode) = match (kind, type_name) {
        ("count", "Int") => Some("JSON_COUNT"),
        ("kind", "Text") => Some("JSON_KIND"),
        ("keys", "List<Text>") => Some("JSON_KEYS"),
        _ => None,
    } {
        Ok(AilBytecodeInstruction::new(
            opcode,
            &[("source", source.into()), ("key", key.into())],
        ))
    } else {
        Err(error("JSON access destination has an incorrect type"))
    }
}
pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    let expected: &[&str] = match instruction.opcode.as_str() {
        "JSON_GET" => &["key", "path", "source", "type"],
        "JSON_COUNT" | "JSON_KIND" | "JSON_KEYS" => &["key", "source"],
        _ => return Err(error("unknown JSON access opcode")),
    };
    if instruction
        .operands
        .keys()
        .map(String::as_str)
        .ne(expected.iter().copied())
    {
        return Err(error(
            "JSON access instruction requires exactly its defined operands",
        ));
    }
    if instruction.opcode == "JSON_GET" {
        if !supported(&instruction.operands["type"]) {
            return Err(error("unsupported JSON access type"));
        }
        field(&instruction.operands["path"])?;
    }
    field(&instruction.operands["source"])?;
    field(&instruction.operands["key"])
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let mut result = Vec::new();
    for step in core
        .graph
        .edges
        .iter()
        .filter(|e| e.kind == "json_access_step")
    {
        for edge in core.graph.edges.iter().filter(|e| {
            e.source == step.target
                && match kind {
                    "reads" => matches!(e.kind.as_str(), "json_access_source" | "json_access_path"),
                    "writes" => e.kind == "json_access_destination",
                    _ => false,
                }
        }) {
            result.push((edge, step.source.as_str()));
        }
    }
    result
}
/// Decode a strict JSON number to a finite scalar Float without arena charges.
pub(super) fn floating_number(input: &str) -> Result<String, String> {
    let value = parse(input)?;
    encoded_size(&value)?;
    let Value::Number(number) = value else {
        return Err(runtime_error("Float requires a JSON number"));
    };
    let bits = crate::float_number::parse(number)
        .map_err(|_| runtime_error("invalid floating JSON number"))?;
    if !f64::from_bits(bits).is_finite() {
        return Err(runtime_error("JSON Float must be finite"));
    }
    float_operations::runtime_format(bits)
        .map_err(|_| runtime_error("cannot format floating JSON number"))
}

pub(super) fn execute(
    instruction: &AilBytecodeInstruction,
    state: &mut BTreeMap<String, String>,
    text_allocated: &mut usize,
    list_allocated: &mut usize,
) -> Result<(), String> {
    verify(instruction)?;
    let input = state
        .get(&instruction.operands["source"])
        .ok_or_else(|| runtime_error("missing JSON access source"))?;
    let value = parse(input)?;
    // Whole-source canonical size is required even if the selected result is tiny.
    // This validation does not consume a result allocation or need a source copy.
    encoded_size(&value)?;
    let result = if instruction.opcode == "JSON_GET" {
        let path = state
            .get(&instruction.operands["path"])
            .ok_or_else(|| runtime_error("missing JSON path"))?;
        let selected = encode(select(&value, path)?)?;
        let type_name = &instruction.operands["type"];
        let result = json_codec::parse_runtime_value(type_name, &selected)?;
        json_codec::reserve_result(
            type_name,
            true,
            result.len(),
            text_allocated,
            list_allocated,
        )?;
        result
    } else if instruction.opcode == "JSON_KIND" {
        let result = match &value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::Text(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };
        json_codec::reserve_result("Text", true, result.len(), text_allocated, list_allocated)?;
        result.to_string()
    } else if instruction.opcode == "JSON_KEYS" {
        let Value::Object(members) = value else {
            return Err(runtime_error("JSON keys requires an object"));
        };
        let keys = Value::Array(
            members
                .into_iter()
                .map(|(key, _)| Value::Text(key))
                .collect(),
        );
        let result = encode(&keys)?;
        json_codec::reserve_result(
            "List<Text>",
            true,
            result.len(),
            text_allocated,
            list_allocated,
        )?;
        result
    } else {
        match &value {
            Value::Array(values) => values.len(),
            Value::Object(members) => members.len(),
            _ => return Err(runtime_error("JSON count requires an array or object")),
        }
        .to_string()
    };
    state.insert(instruction.operands["key"].clone(), result);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn get(type_name: &str) -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(
            "JSON_GET",
            &[
                ("key", "out.value".into()),
                ("source", "data.json".into()),
                ("path", "data.path".into()),
                ("type", type_name.into()),
            ],
        )
    }
    fn count() -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(
            "JSON_COUNT",
            &[("key", "out.value".into()), ("source", "data.json".into())],
        )
    }
    fn inspect(opcode: &str) -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(
            opcode,
            &[("key", "out.value".into()), ("source", "data.json".into())],
        )
    }

    #[test]
    fn inspection_kind_classifies_all_roots_without_numeric_conversion() {
        for (input, expected) in [
            ("null", "null"),
            ("true", "boolean"),
            ("false", "boolean"),
            ("-0.0000E+999999999", "number"),
            ("\"\u{85}\"", "string"),
            ("[1,true,null]", "array"),
            ("{\"x\":1,\"x\":2}", "object"),
        ] {
            let mut state = BTreeMap::from([("data.json".into(), input.into())]);
            let (mut text, mut list) = (11, 17);
            execute(&inspect("JSON_KIND"), &mut state, &mut text, &mut list).unwrap();
            assert_eq!(state["out.value"], expected);
            assert_eq!(state["data.json"], input);
            assert_eq!((text, list), (11 + expected.len(), 17));
        }
    }

    #[test]
    fn inspection_keys_preserve_decoded_pairs_order_and_all_unicode_names() {
        let input =
            r#"{"":0,"dup":1,"\u0064up":2,"\u0000":3,"é":4,"e\u0301":5,"/~~":6,"\u0085":7,"🦀":8}"#;
        let expected =
            "[\"\",\"dup\",\"dup\",\"\\u0000\",\"é\",\"e\u{301}\",\"/~~\",\"\\u0085\",\"🦀\"]";
        let mut state = BTreeMap::from([
            ("data.json".into(), input.into()),
            ("data.copy".into(), input.into()),
        ]);
        let mut op = inspect("JSON_KEYS");
        op.operands.insert("key".into(), "data.json".into());
        let (mut text, mut list) = (23, 29);
        execute(&op, &mut state, &mut text, &mut list).unwrap();
        assert_eq!(state["data.json"], expected);
        assert_eq!(state["data.copy"], input);
        assert_eq!((text, list), (23, 29 + expected.len()));
        state.insert("data.json".into(), "{}".into());
        execute(&op, &mut state, &mut text, &mut list).unwrap();
        assert_eq!(state["data.json"], "[]");
        assert_eq!((text, list), (23, 31 + expected.len()));
    }

    #[test]
    fn inspection_validates_complete_source_before_kind_type_and_allocation() {
        let oversized_canonical = format!("{{\"large\":\"{}\"}}", "\u{85}".repeat(200_000));
        let deep = format!("{}0{}", "[".repeat(65), "]".repeat(65));
        let many = format!("[{}]", vec!["null"; MAX_VALUES].join(","));
        let oversized_raw = format!("null{}", " ".repeat(MAX_BYTES - 3));
        for (input, family) in [
            ("{\"a\":[1,broken]}".to_string(), "AIL-RUNTIME-JSON:"),
            ("{} false".into(), "AIL-RUNTIME-JSON:"),
            (oversized_canonical, "AIL-RUNTIME-JSON-LIMIT:"),
            (deep, "AIL-RUNTIME-JSON-LIMIT:"),
            (many, "AIL-RUNTIME-JSON-LIMIT:"),
            (oversized_raw, "AIL-RUNTIME-JSON-LIMIT:"),
        ] {
            for opcode in ["JSON_KIND", "JSON_KEYS"] {
                let mut state = BTreeMap::from([
                    ("data.json".into(), input.clone()),
                    ("out.value".into(), "preserved".into()),
                ]);
                let (mut text, mut list) = (67_108_864, 67_108_864);
                assert!(
                    execute(&inspect(opcode), &mut state, &mut text, &mut list)
                        .unwrap_err()
                        .starts_with(family),
                    "{opcode}"
                );
                assert_eq!(state["out.value"], "preserved");
                assert_eq!((text, list), (67_108_864, 67_108_864));
            }
        }
        for input in ["null", "false", "-1e99999", "\"text\"", "[]"] {
            let mut state = BTreeMap::from([("data.json".into(), input.into())]);
            assert!(
                execute(
                    &inspect("JSON_KEYS"),
                    &mut state,
                    &mut 67_108_864,
                    &mut 67_108_864
                )
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON:")
            );
        }
    }

    #[test]
    fn inspection_final_results_charge_their_arena_once_and_fail_atomically() {
        for (opcode, input, output, text_charge, list_charge) in [
            ("JSON_KIND", "false", "boolean", 7, 0),
            ("JSON_KEYS", "{}", "[]", 0, 2),
            ("JSON_KEYS", "{\"a\":1,\"a\":2}", "[\"a\",\"a\"]", 0, 9),
        ] {
            let mut state = BTreeMap::from([("data.json".into(), input.into())]);
            let (mut text, mut list) = (67_108_864 - text_charge, 67_108_864 - list_charge);
            execute(&inspect(opcode), &mut state, &mut text, &mut list).unwrap();
            assert_eq!(state["out.value"], output);
            assert_eq!((text, list), (67_108_864, 67_108_864));
            state.insert("out.value".into(), "preserved".into());
            assert!(
                execute(&inspect(opcode), &mut state, &mut text, &mut list)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON-LIMIT:")
            );
            assert_eq!(state["out.value"], "preserved");
            assert_eq!((text, list), (67_108_864, 67_108_864));
        }
    }

    #[test]
    fn inspection_keys_allow_65535_members_including_duplicates() {
        let input = format!("{{{}}}", vec!["\"\":0"; MAX_VALUES - 1].join(","));
        let mut state = BTreeMap::from([("data.json".into(), input)]);
        let (mut text, mut list) = (0, 0);
        execute(&inspect("JSON_KEYS"), &mut state, &mut text, &mut list).unwrap();
        assert_eq!(
            state["out.value"],
            format!("[{}]", vec!["\"\""; MAX_VALUES - 1].join(","))
        );
        assert_eq!((text, list), (0, state["out.value"].len()));
    }

    #[test]
    fn inspection_source_and_protocol_are_explicit_and_exact() {
        for kind in ["kind", "keys"] {
            let source = format!("the system gets JSON {kind} from data.document into out.value");
            let op = parse_bullet(&source).unwrap().unwrap();
            assert_eq!(op.source(), source);
            assert_eq!(
                namespace(&op, "Child").source(),
                format!(
                    "the system gets JSON {kind} from child.data.document into child.out.value"
                )
            );
            assert!(
                parse_bullet(&format!("the system gets JSON {kind} from data.document"))
                    .unwrap()
                    .is_err()
            );
            assert!(
                parse_bullet(&format!("the system gets JSON {kind}"))
                    .unwrap()
                    .is_err()
            );
        }
        for opcode in ["JSON_KIND", "JSON_KEYS"] {
            let op = inspect(opcode);
            assert!(verify(&op).is_ok());
            for name in ["path", "type", "ignored"] {
                let mut extra = op.clone();
                extra.operands.insert(name.into(), "data.path".into());
                assert!(verify(&extra).is_err());
            }
        }
    }

    #[test]
    fn core_access_operands_require_unique_genuine_global_storage() {
        for role in [
            "json_access_source",
            "json_access_destination",
            "json_access_path",
        ] {
            let mut core = core_fixture();
            let index = core
                .graph
                .edges
                .iter()
                .position(|e| e.kind == role)
                .unwrap();
            let mut forged = core
                .graph
                .nodes
                .iter()
                .find(|n| n.id == core.graph.edges[index].target)
                .unwrap()
                .clone();
            forged.id.push_str("_forged");
            forged.attributes.remove("secret");
            core.graph.edges[index].target = forged.id.clone();
            core.graph.nodes.push(forged);
            assert!(!check(&core).is_empty(), "accepted forged {role}");
        }
    }

    #[test]
    fn inspection_core_roundtrips_fixed_types_and_rejects_extra_paths_or_ownership() {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Json manages records.\n\nA Data has:\n- document: Json\n\nAn Output has:\n- kind: Text\n- keys: List<Text>\n\nAction: Run.\nWhen Run happens:\n- the system gets JSON kind from data.document into output.kind\n- the system gets JSON keys from data.document into output.keys\n";
        let document = parse_ail_package_spec_text(&package, source).unwrap();
        let original = elaborate_ail_core(&package, &document);
        assert!(check(&original).is_empty(), "{:?}", check(&original));
        let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        assert_eq!(
            from_core(&saved, saved.graph.find_node("Action", "Run").unwrap()),
            document.actions["Run"].json_accesses
        );
        for op in &document.actions["Run"].json_accesses {
            let code = instruction(op, &document).unwrap();
            assert!(matches!(code.opcode.as_str(), "JSON_KIND" | "JSON_KEYS"));
            assert_eq!(code.operands.len(), 2);
        }
        for kind in ["kind", "keys"] {
            let operation_id = original
                .graph
                .nodes
                .iter()
                .find(|n| n.kind == "JsonAccess" && n.attributes["json_access_kind"] == kind)
                .unwrap()
                .id
                .clone();
            for role in [
                "json_access_step",
                "json_access_source",
                "json_access_destination",
            ] {
                let mut duplicate = original.clone();
                let edge = duplicate
                    .graph
                    .edges
                    .iter()
                    .find(|e| {
                        e.kind == role && (e.source == operation_id || e.target == operation_id)
                    })
                    .unwrap()
                    .clone();
                duplicate.graph.edges.push(edge);
                assert!(
                    !check(&parse_ail_core_text(&render_ail_core(&duplicate)).unwrap()).is_empty()
                );
            }
            let mut extra_path = original.clone();
            let mut edge = extra_path
                .graph
                .edges
                .iter()
                .find(|e| e.source == operation_id && e.kind == "json_access_destination")
                .unwrap()
                .clone();
            edge.id.push_str("_path");
            edge.kind = "json_access_path".into();
            extra_path.graph.edges.push(edge);
            assert!(!check(&extra_path).is_empty());
            let mut wrong_type = original.clone();
            wrong_type
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.id == operation_id)
                .unwrap()
                .attributes
                .insert("value_type".into(), "Json".into());
            assert!(!check(&wrong_type).is_empty());
            let mut orphan = original.clone();
            orphan
                .graph
                .edges
                .retain(|e| !(e.kind == "json_access_step" && e.target == operation_id));
            assert!(!check(&orphan).is_empty());
        }
        for profile in ["Compiler", "System", "UI", "AgentTool"] {
            let mut invalid = original.clone();
            invalid.package.profile = profile.into();
            assert!(!check(&invalid).is_empty());
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
        let mut secret = original;
        secret
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        for node in secret.graph.nodes.iter_mut().filter(|n| n.kind == "Field") {
            node.attributes.insert("secret".into(), "true".into());
        }
        assert_eq!(accesses(&secret, "reads").len(), 2);
        assert_eq!(accesses(&secret, "writes").len(), 2);
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
    }
    fn lookup(source: &str, path: &str, type_name: &str) -> Result<(String, usize, usize), String> {
        let mut state = BTreeMap::from([
            ("data.json".into(), source.into()),
            ("data.path".into(), path.into()),
        ]);
        let mut text = 0;
        let mut list = 0;
        execute(&get(type_name), &mut state, &mut text, &mut list)?;
        Ok((state["out.value"].clone(), text, list))
    }

    fn core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Json manages values.\n\nA Data has:\n- document: Json\n- path: Text\n\nAn Output has:\n- value: Json\n- count: Int\n\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Accessed\n";
        let mut document = parse_ail_package_spec_text(&package, source).unwrap();
        let action = document.actions.get_mut("Run").unwrap();
        action.json_accesses = vec![
            AilJsonAccess::Get {
                source: "data.document".into(),
                path: "data.path".into(),
                key: "output.value".into(),
            },
            AilJsonAccess::Count {
                source: "data.document".into(),
                key: "output.count".into(),
            },
        ];
        action.steps.insert(0, AilActionStep::JsonAccess(1));
        action.steps.insert(0, AilActionStep::JsonAccess(0));
        elaborate_ail_core(&package, &document)
    }
    #[test]
    fn core_roundtrip_checks_exact_ownership_operands_and_metadata() {
        let original = core_fixture();
        assert_eq!(
            original
                .graph
                .nodes
                .iter()
                .filter(|n| n.kind == "JsonAccess")
                .count(),
            2
        );
        let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        assert!(check(&saved).is_empty(), "{:?}", check(&saved));
        let action = saved
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "Action")
            .unwrap();
        assert_eq!(from_core(&saved, action).len(), 2);
        for kind in [
            "json_access_step",
            "json_access_source",
            "json_access_destination",
            "json_access_path",
        ] {
            let mut duplicate = original.clone();
            duplicate.graph.edges.push(
                duplicate
                    .graph
                    .edges
                    .iter()
                    .find(|e| e.kind == kind)
                    .unwrap()
                    .clone(),
            );
            assert!(
                !check(&parse_ail_core_text(&render_ail_core(&duplicate)).unwrap()).is_empty(),
                "duplicate {kind}"
            );
            let mut extra = original.clone();
            extra
                .graph
                .edges
                .iter_mut()
                .find(|e| e.kind == kind)
                .unwrap()
                .attributes
                .insert("hidden".into(), "ignored".into());
            assert!(!check(&extra).is_empty(), "extra {kind}");
        }
        for (attribute, value) in [
            ("json_access_kind", "unknown"),
            ("value_type", "Decimal"),
            ("hidden", "ignored"),
        ] {
            let mut candidate = original.clone();
            candidate
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == "JsonAccess")
                .unwrap()
                .attributes
                .insert(attribute.into(), value.into());
            assert!(!check(&candidate).is_empty(), "{attribute}");
        }
        let mut typed = original.clone();
        typed
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "JsonAccess")
            .unwrap()
            .type_name = Some("Json".into());
        assert!(!check(&typed).is_empty());
        let mut count_path = original.clone();
        let count_id = count_path
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "JsonAccess" && n.attributes["json_access_kind"] == "count")
            .unwrap()
            .id
            .clone();
        count_path
            .graph
            .edges
            .iter_mut()
            .find(|e| e.kind == "json_access_path")
            .unwrap()
            .source = count_id;
        assert!(!check(&count_path).is_empty());
    }
    #[test]
    fn core_checks_profiles_global_types_and_semantic_secret_accesses() {
        let original = core_fixture();
        for profile in ["Compiler", "System", "UI", "AgentTool"] {
            let mut candidate = original.clone();
            candidate.package.profile = profile.into();
            assert!(!check(&candidate).is_empty(), "{profile}");
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
        for field_name in ["Data.document", "Data.path", "Output.value", "Output.count"] {
            let mut candidate = original.clone();
            candidate
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == "Field" && n.name == field_name)
                .unwrap()
                .attributes
                .insert("function_owner".into(), "hidden".into());
            assert!(!check(&candidate).is_empty(), "local {field_name}");
            let mut candidate = original.clone();
            candidate
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == "Field" && n.name == field_name)
                .unwrap()
                .type_name = Some("Bool".into());
            assert!(!check(&candidate).is_empty(), "type {field_name}");
        }
        let mut secret = original;
        secret
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        for node in secret.graph.nodes.iter_mut().filter(|n| n.kind == "Field") {
            node.attributes.insert("secret".into(), "true".into());
        }
        assert_eq!(accesses(&secret, "reads").len(), 3);
        assert_eq!(accesses(&secret, "writes").len(), 2);
        assert!(
            check_secret_write_protection(&secret)
                .iter()
                .any(|d| d.code == "AIL002")
        );
        assert!(
            check_secret_read_protection(&secret)
                .iter()
                .any(|d| d.code == "AIL005")
        );
    }

    #[test]
    fn mutations_preserve_pairs_numbers_and_replace_append_remove_positions() {
        let source = r#"{"a":1.00E+2,"dup":-0,"dup":3,"arr":[null,true,"old"],"tail":9}"#;
        assert_eq!(
            mutate(source, "/arr/1", Some(("Int", "+00012"))).unwrap(),
            r#"{"a":1.00E+2,"dup":-0,"dup":3,"arr":[null,12,"old"],"tail":9}"#
        );
        assert_eq!(
            mutate(source, "/arr/-", Some(("Json", r#"{"x":1e999999}"#))).unwrap(),
            r#"{"a":1.00E+2,"dup":-0,"dup":3,"arr":[null,true,"old",{"x":1e999999}],"tail":9}"#
        );
        assert_eq!(
            mutate(source, "/arr/0", None).unwrap(),
            r#"{"a":1.00E+2,"dup":-0,"dup":3,"arr":[true,"old"],"tail":9}"#
        );
        assert_eq!(
            mutate(source, "/a", None).unwrap(),
            r#"{"dup":-0,"dup":3,"arr":[null,true,"old"],"tail":9}"#
        );
        assert_eq!(
            mutate(source, "/new", Some(("Bool", "false"))).unwrap(),
            r#"{"a":1.00E+2,"dup":-0,"dup":3,"arr":[null,true,"old"],"tail":9,"new":false}"#
        );
        assert_eq!(
            mutate(source, "", Some(("List<Int>", " [1,-0,2] "))).unwrap(),
            "[1,0,2]"
        );
        assert_eq!(
            mutate("null", "", Some(("Text", "é\0\u{85}"))).unwrap(),
            "\"é\\u0000\\u0085\""
        );
        assert_eq!(
            mutate("{}", "/-", Some(("List<Text>", r#"["\ud83d\ude42"]"#))).unwrap(),
            "{\"-\":[\"🙂\"]}"
        );
    }

    #[test]
    fn mutation_pointer_tokens_are_decoded_once_and_duplicates_fail_only_when_selected() {
        let source =
            r#"{"":{"a/b":{"~1":[1,2]}},"\u0000":1,"é":2,"e\u0301":3,"dup":1,"\u0064up":2}"#;
        let result = mutate(source, "//a~1b/~01/0", Some(("Int", "9"))).unwrap();
        assert!(result.contains(r#""~1":[9,2]"#));
        let result = mutate(source, "/\0", None).unwrap();
        assert!(!result.contains(r#""\u0000":1"#));
        let result = mutate(source, "/é", None).unwrap();
        assert!(result.contains("\"e\u{301}\":3"));
        for path in ["/dup", "/dup/x"] {
            assert!(
                mutate(source, path, Some(("Text", "valid")))
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON-PATH:")
            );
            assert!(
                mutate(source, path, None)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON-PATH:")
            );
        }
    }

    #[test]
    fn mutation_target_checks_precede_value_conversion() {
        for (source, path) in [
            ("{}", "/missing/x"),
            ("[]", "/0"),
            ("[0]", "/1"),
            ("[0]", "/00"),
            ("[0]", "/-/x"),
            ("0", "/a"),
            ("{}", "/missing/~2"),
            (r#"{"a":0,"a":1}"#, "/a"),
        ] {
            assert!(
                mutate(source, path, Some(("Bool", "malformed")))
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON-PATH:"),
                "{source} {path}"
            );
        }
        for (source, path) in [("{}", ""), ("{}", "/missing"), ("[0]", "/-")] {
            assert!(
                mutate(source, path, None)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON-PATH:")
            );
        }
        assert!(
            mutate("broken", "bad~2", Some(("Bool", "bad")))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON:")
        );
        for (kind, value) in [
            ("Bool", "True"),
            ("Int", " 1"),
            ("Int", "9223372036854775808"),
            ("List<Text>", "[1]"),
            ("Json", "[bad]"),
        ] {
            assert!(
                mutate("{}", "/a", Some((kind, value)))
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON:"),
                "{kind}"
            );
        }
    }

    #[test]
    fn mutations_check_attached_structure_and_discard_replaced_subtree_counts() {
        let maximum = format!("[{}]", vec!["0"; MAX_VALUES - 1].join(","));
        assert_eq!(
            mutate(&maximum, "", Some(("Json", &maximum))).unwrap(),
            maximum
        );
        assert_eq!(
            mutate(&maximum, "/0", Some(("Int", "1"))).unwrap().len(),
            maximum.len()
        );
        assert!(
            mutate(&maximum, "/-", Some(("Int", "1")))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert!(
            mutate("[0]", "/0", Some(("Json", &maximum)))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert!(mutate(&maximum, "/0", None).is_ok());
        let depth64 = format!("{}0{}", "[".repeat(64), "]".repeat(64));
        assert_eq!(
            mutate("null", "", Some(("Json", &depth64))).unwrap(),
            depth64
        );
        assert!(
            mutate("[0]", "/0", Some(("Json", &depth64)))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert_eq!(
            mutate(&depth64, &"/0".repeat(64), Some(("Bool", "true"))).unwrap(),
            format!("{}true{}", "[".repeat(64), "]".repeat(64))
        );
        assert!(
            mutate(&depth64, &"/0".repeat(64), Some(("Json", "[]")))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
    }

    #[test]
    fn mutations_enforce_source_value_and_result_bounds_even_when_replacing_root() {
        let oversized = format!("\"{}\"", "\u{85}".repeat(200_000));
        assert!(
            mutate(&oversized, "", Some(("Int", "0")))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert!(
            mutate("null", "", Some(("Text", &"x".repeat(MAX_BYTES))))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert!(
            mutate(
                "null",
                "",
                Some(("Int", &format!("{}1", "0".repeat(MAX_BYTES))))
            )
            .unwrap_err()
            .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        let large = format!("\"{}\"", "x".repeat(MAX_BYTES - 2));
        assert_eq!(
            mutate("null", "", Some(("Json", &large))).unwrap().len(),
            MAX_BYTES
        );
        assert!(
            mutate("{}", "/a", Some(("Json", &large)))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert_eq!(mutate(&large, "", Some(("Json", "null"))).unwrap(), "null");
    }

    #[test]
    fn canonical_preserves_recursive_pairs_and_every_valid_number_token() {
        let input = r#" { "same": [null, true, false, -0, 1.200E+0002, 999999999999999999999999999999], "\u0073ame": {"escaped": "\ud83d\ude42\u0000\u0085"} } "#;
        assert_eq!(
            canonical(input).unwrap(),
            "{\"same\":[null,true,false,-0,1.200E+0002,999999999999999999999999999999],\"same\":{\"escaped\":\"🙂\\u0000\\u0085\"}}"
        );
        for token in [
            "-0",
            "0.0",
            "-0.00e-0",
            "1E+0002",
            "123456789012345678901234567890",
            "1e999999999999999999999999999999",
        ] {
            assert_eq!(canonical(token).unwrap(), token);
        }
    }

    #[test]
    fn recursive_parser_rejects_invalid_grammar_surrogates_and_trailing_values() {
        for source in [
            "",
            "nullx",
            "true false",
            "[1,]",
            "{\"a\":1,}",
            "{1:true}",
            "{\"a\" 1}",
            "[[]",
            "+1",
            "01",
            "-01",
            "1.",
            ".1",
            "1e",
            "1e+",
            "NaN",
            "Infinity",
            "\u{85}null",
            r#""\ud800""#,
            r#""\udc00""#,
            r#""\ud800\u0000""#,
            "\"raw\nline\"",
        ] {
            let error = canonical(source).unwrap_err();
            assert!(
                error.starts_with("AIL-RUNTIME-JSON:"),
                "{source:?}: {error}"
            );
        }
    }

    #[test]
    fn parser_limits_count_values_but_not_member_names_and_check_full_grammar_first() {
        let maximum = format!("{}0{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
        assert_eq!(canonical(&maximum).unwrap(), maximum);
        assert!(
            canonical(&format!(
                "{}0{}",
                "[".repeat(MAX_DEPTH + 1),
                "]".repeat(MAX_DEPTH + 1)
            ))
            .unwrap_err()
            .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        let maximum = format!("[{}]", vec!["null"; MAX_VALUES - 1].join(","));
        assert!(canonical(&maximum).is_ok());
        assert!(
            canonical(&format!("[{}]", vec!["null"; MAX_VALUES].join(",")))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        let maximum = format!("{{{}}}", vec!["\"\":0"; MAX_VALUES - 1].join(","));
        assert!(canonical(&maximum).is_ok());
        let exact = format!("\"{}\"", "x".repeat(MAX_BYTES - 2));
        assert_eq!(canonical(&exact).unwrap().len(), MAX_BYTES);
        assert!(
            canonical(&(exact + " "))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        let expansion = format!("\"{}\"", "\u{85}".repeat(200_000));
        assert!(
            canonical(&expansion)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert!(
            canonical(&(expansion + " trailing"))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON:")
        );
    }

    #[test]
    fn pointers_decode_once_and_support_empty_unicode_and_nul_keys() {
        let source = r#"{"": {"a/b":{"~1":[-0,true,"é",[1,2],{"\u0000":"🙂"}]}}, "é":"composed", "e\u0301":"decomposed"}"#;
        for (path, kind, value, text, list) in [
            ("//a~1b/~01/0", "Int", "0", 0, 0),
            ("//a~1b/~01/1", "Bool", "true", 0, 0),
            ("//a~1b/~01/2", "Text", "é", 2, 0),
            ("//a~1b/~01/3", "List<Int>", "[1,2]", 0, 5),
            ("//a~1b/~01/4/\0", "Text", "🙂", 4, 0),
            ("/é", "Text", "composed", 8, 0),
            ("/e\u{301}", "Text", "decomposed", 10, 0),
        ] {
            assert_eq!(
                lookup(source, path, kind).unwrap(),
                (value.into(), text, list),
                "{path:?}"
            );
        }
        assert_eq!(
            lookup(" [null,1.00E+0] ", "", "Json").unwrap(),
            ("[null,1.00E+0]".into(), 0, 14)
        );
    }

    #[test]
    fn pointer_lookup_rejects_ambiguous_members_array_forms_and_scalar_traversal() {
        for (source, path) in [
            (r#"{"a":1,"\u0061":2}"#, "/a"),
            ("[1]", "/01"),
            ("[1]", "/-"),
            ("[1]", "/-1"),
            ("[1]", "/1"),
            ("[1]", "/999999999999999999999999999"),
            ("[1]", "/"),
            ("1", "/child"),
            ("{}", "/missing"),
            ("{}", "#"),
            ("{}", "a"),
            ("{}", "/~2"),
            ("{}", "/~"),
        ] {
            assert!(
                lookup(source, path, "Json")
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON-PATH:"),
                "{source} {path}"
            );
        }
        assert!(
            lookup("1.0", "", "Int")
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON:")
        );
        assert!(
            lookup("9223372036854775808", "", "Int")
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON:")
        );
        assert!(
            lookup("null", "", "Text")
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON:")
        );
    }

    #[test]
    fn count_validates_source_and_counts_immediate_pairs_including_duplicates() {
        for (source, want) in [
            ("[]", "0"),
            ("[null,{},[1,2]]", "3"),
            (r#"{"a":1,"a":2}"#, "2"),
        ] {
            let mut state = BTreeMap::from([("data.json".into(), source.into())]);
            let mut text = 9;
            let mut list = 17;
            execute(&count(), &mut state, &mut text, &mut list).unwrap();
            assert_eq!(state["out.value"], want);
            assert_eq!((text, list), (9, 17));
        }
        for source in ["null", "false", "1", "\"text\"", "[1] broken"] {
            let mut state = BTreeMap::from([("data.json".into(), source.into())]);
            assert!(
                execute(&count(), &mut state, &mut 0, &mut 0)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON:")
            );
        }
    }

    #[test]
    fn source_validation_precedes_path_and_failed_lookup_preserves_aliases_and_budgets() {
        let mut state = BTreeMap::from([
            ("data.json".into(), "[broken".into()),
            ("data.path".into(), "/bad~2".into()),
            ("out.value".into(), "unchanged".into()),
        ]);
        let mut text = 7;
        let mut list = 11;
        assert!(
            execute(&get("Text"), &mut state, &mut text, &mut list)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON:")
        );
        assert_eq!(state["out.value"], "unchanged");
        assert_eq!((text, list), (7, 11));
        state.insert("data.json".into(), "{\"a\":\"old\"}".into());
        state.insert("data.path".into(), "/a".into());
        state.insert("data.copy".into(), state["data.json"].clone());
        let mut same = get("Text");
        same.operands.insert("key".into(), "data.json".into());
        execute(&same, &mut state, &mut text, &mut list).unwrap();
        assert_eq!(state["data.json"], "old");
        assert_eq!(state["data.copy"], "{\"a\":\"old\"}");
        assert_eq!((text, list), (10, 11));
        state.insert("data.json".into(), "\"new\"".into());
        state.insert("data.path".into(), "".into());
        text = 67_108_864;
        assert!(
            execute(&same, &mut state, &mut text, &mut list)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert_eq!(state["data.json"], "\"new\"");
        assert_eq!((text, list), (67_108_864, 11));
    }

    #[test]
    fn access_validates_source_bound_then_complete_pointer_before_traversing() {
        let mut state = BTreeMap::from([("out.value".into(), "preserved".into())]);
        assert!(
            execute(&get("Json"), &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON:")
        );
        state.insert("data.json".into(), "{}".into());
        assert!(
            execute(&get("Json"), &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON:")
        );
        state.insert("data.path".into(), "/missing/~2".into());
        let error = execute(&get("Json"), &mut state, &mut 0, &mut 0).unwrap_err();
        assert!(error.starts_with("AIL-RUNTIME-JSON-PATH:") && error.contains("escape"));
        state.insert(
            "data.path".into(),
            format!("/{}", "x".repeat(MAX_BYTES - 1)),
        );
        assert!(
            execute(&get("Json"), &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-PATH:")
        );
        state.get_mut("data.path").unwrap().push('x');
        assert!(
            execute(&get("Json"), &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        state.insert(
            "data.json".into(),
            format!("{{\"large\":\"{}\",\"small\":1}}", "\u{85}".repeat(200_000)),
        );
        state.insert("data.path".into(), "/small".into());
        assert!(
            execute(&get("Int"), &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert!(
            execute(&count(), &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert_eq!(state["out.value"], "preserved");
    }

    #[test]
    fn successful_access_charges_only_final_result_and_failure_is_transactional() {
        for (source, type_name, charge_text, charge_list) in [
            ("\"\"", "Text", 1, 0),
            ("[]", "List<Int>", 0, 2),
            ("[]", "List<Text>", 0, 2),
            ("null", "Json", 0, 4),
            ("-0", "Int", 0, 0),
            ("true", "Bool", 0, 0),
        ] {
            let mut state = BTreeMap::from([
                ("data.json".into(), source.into()),
                ("data.path".into(), "".into()),
                ("out.value".into(), "unchanged".into()),
            ]);
            let mut text = 67_108_864 - charge_text;
            let mut list = 67_108_864 - charge_list;
            execute(&get(type_name), &mut state, &mut text, &mut list).unwrap();
            assert_eq!((text, list), (67_108_864, 67_108_864), "{type_name}");
            if charge_text + charge_list != 0 {
                let before = state.clone();
                assert!(
                    execute(&get(type_name), &mut state, &mut text, &mut list)
                        .unwrap_err()
                        .starts_with("AIL-RUNTIME-JSON-LIMIT:")
                );
                assert_eq!(state, before);
                assert_eq!((text, list), (67_108_864, 67_108_864));
            }
        }
        let mut alias = get("Json");
        alias.operands.insert("key".into(), "data.path".into());
        let mut state = BTreeMap::from([
            ("data.json".into(), "{\"a\":[1.00e+2]}".into()),
            ("data.path".into(), "/a".into()),
        ]);
        let mut text = 0;
        let mut list = 0;
        execute(&alias, &mut state, &mut text, &mut list).unwrap();
        assert_eq!(state["data.path"], "[1.00e+2]");
        assert_eq!(state["data.json"], "{\"a\":[1.00e+2]}");
        assert_eq!((text, list), (0, 9));
    }

    #[test]
    fn source_and_protocol_require_exact_field_operands() {
        for (source, want) in [
            (
                "the system gets JSON value at path data.path from data.document into output.value",
                "the system gets JSON value at path pkg.data.path from pkg.data.document into pkg.output.value",
            ),
            (
                "the system counts entries in JSON data.document into output.count",
                "the system counts entries in JSON pkg.data.document into pkg.output.count",
            ),
        ] {
            let op = parse_bullet(source).unwrap().unwrap();
            assert_eq!(namespace(&op, "pkg").source(), want);
        }
        assert!(verify(&get("Json")).is_ok());
        assert!(verify(&count()).is_ok());
        let mut invalid = count();
        invalid.operands.insert("path".into(), "data.path".into());
        assert!(verify(&invalid).is_err());
        assert!(verify(&get("Decimal")).is_err());
        for source in [
            "the system gets JSON value at path x from data.json into out.value",
            "the system counts entries in JSON",
            "the system gets JSON value",
        ] {
            assert!(parse_bullet(source).unwrap().is_err());
        }
    }
}
