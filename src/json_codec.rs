//! Explicit conversion between bounded JSON text and executable typed values.
use super::*;
use crate::expression::{IntExpr, parse_integer};

const MAX_BYTES: usize = 1_048_576;
const MAX_ALLOCATED: usize = 67_108_864;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilJsonConversion {
    Parse { source: String, key: String },
    Format { source: String, key: String },
    Invalid(String),
}
impl AilJsonConversion {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Parse { source, key } => {
                format!("the system parses JSON text {source} into {key}")
            }
            Self::Format { source, key } => {
                format!("the system formats {source} as JSON text into {key}")
            }
            Self::Invalid(message) => format!("invalid JSON conversion: {message}"),
        }
    }
    fn parts(&self) -> Result<(&str, &str, &str), String> {
        match self {
            Self::Parse { source, key } => Ok(("parse", source, key)),
            Self::Format { source, key } => Ok(("format", source, key)),
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
        Err(error("JSON operand must be a field reference"))
    }
}

pub(super) fn parse_bullet(text: &str) -> Option<Result<AilJsonConversion, String>> {
    let text = trim_sentence(text);
    let parsed = if let Some(rest) = text.strip_prefix("the system parses JSON text ") {
        rest.split_once(" into ")
            .map(|(source, key)| AilJsonConversion::Parse {
                source: source.into(),
                key: key.into(),
            })
    } else if let Some(rest) = text.strip_prefix("the system formats ") {
        rest.split_once(" as JSON text into ")
            .map(|(source, key)| AilJsonConversion::Format {
                source: source.into(),
                key: key.into(),
            })
    } else {
        return (text.starts_with("the system parses JSON") || text == "the system formats")
            .then(|| Err(error("malformed JSON conversion")));
    };
    Some(
        parsed
            .ok_or_else(|| error("malformed JSON conversion"))
            .and_then(|op| {
                let (_, source, key) = op.parts()?;
                field(source)?;
                field(key)?;
                Ok(op)
            }),
    )
}

pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, op) in action.json_conversions.iter().enumerate() {
            let (kind, source, key) = op.parts().unwrap_or(("invalid", "", ""));
            let typed_key = if kind == "parse" { key } else { source };
            let type_name = runtime_field_type_name(document, typed_key).unwrap_or("invalid");
            let node = graph.add_node(
                "JsonConversion",
                format!("{}.json{index}", owner.name),
                None,
                attr(&[("json_kind", kind), ("value_type", type_name)]),
            );
            attach_provenance(graph, &node, format!("action:{}.json:{index}", owner.name));
            if kind != "invalid" {
                for (edge_kind, key) in [("json_source", source), ("json_destination", key)] {
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
                                format!("action:{}.json:{index}.{edge_kind}", owner.name),
                            );
                            target
                        });
                    graph.add_edge(edge_kind, &node, &target, BTreeMap::new());
                }
            }
            action_order::append(graph, "json_step", &owner, &node, BTreeMap::new(), index);
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
        .ok_or_else(|| error("JSON conversion has a missing operand"))?;
    if edges.next().is_some() || !edge.attributes.is_empty() {
        return Err(error(
            "JSON conversion requires one edge per operand without attributes",
        ));
    }
    let target = core
        .graph
        .nodes
        .iter()
        .find(|n| n.id == edge.target)
        .ok_or_else(|| error("JSON operand target is missing"))?;
    if target.kind != "Field"
        || target.type_name.as_deref() != Some(type_name)
        || target.attributes.contains_key("function_owner")
        || target.attributes.contains_key("local_name")
    {
        return Err(error(
            "JSON operand must be a global field of the required type",
        ));
    }
    let key = expression_core::field_key(target);
    field(&key)?;
    Ok(key)
}
fn decode_node(core: &AilCore, node: &Node) -> Result<AilJsonConversion, String> {
    if node.kind != "JsonConversion"
        || node.type_name.is_some()
        || node
            .attributes
            .keys()
            .map(String::as_str)
            .ne(["json_kind", "value_type"])
    {
        return Err(error(
            "JSON node requires exactly json_kind and value_type attributes and no type_name",
        ));
    }
    let type_name = &node.attributes["value_type"];
    if !supported(type_name) {
        return Err(error("unsupported JSON conversion type"));
    }
    let (source_type, key_type) = match node.attributes["json_kind"].as_str() {
        "parse" => ("Text", type_name.as_str()),
        "format" => (type_name.as_str(), "Text"),
        _ => return Err(error("json_kind must be parse or format")),
    };
    if core.graph.edges.iter().any(|e| {
        (e.source == node.id
            && !matches!(
                e.kind.as_str(),
                "json_source" | "json_destination" | "has_provenance"
            ))
            || (e.target == node.id && e.kind != "json_step")
    }) {
        return Err(error("unexpected edge on JSON conversion"));
    }
    let source = operand(core, node, "json_source", source_type)?;
    let key = operand(core, node, "json_destination", key_type)?;
    Ok(if node.attributes["json_kind"] == "parse" {
        AilJsonConversion::Parse { source, key }
    } else {
        AilJsonConversion::Format { source, key }
    })
}
pub(super) fn from_core(core: &AilCore, action: &Node) -> Vec<AilJsonConversion> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == action.id && e.kind == "json_step")
        .map(|edge| {
            core.graph
                .nodes
                .iter()
                .find(|n| n.id == edge.target)
                .ok_or_else(|| error("missing JSON conversion node"))
                .and_then(|node| decode_node(core, node))
                .unwrap_or_else(AilJsonConversion::Invalid)
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
    for node in core
        .graph
        .nodes
        .iter()
        .filter(|n| n.kind == "JsonConversion")
    {
        let result = (|| {
            if !matches!(core.package.profile.as_str(), "Application" | "C interop") {
                return Err(error(
                    "JSON conversion requires Application or C interop profile",
                ));
            }
            let owners: Vec<_> = core
                .graph
                .edges
                .iter()
                .filter(|e| e.target == node.id && e.kind == "json_step")
                .collect();
            if owners.len() != 1
                || !core
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == owners[0].source && executable_action(n))
            {
                return Err(error(
                    "JSON conversion requires exactly one executable action owner",
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
            "json_step" => {
                source.is_some_and(executable_action)
                    && target.is_some_and(|n| n.kind == "JsonConversion")
                    && edge
                        .attributes
                        .get("step_index")
                        .is_some_and(|v| v.parse::<usize>().is_ok())
                    && edge.attributes.iter().all(|(k, v)| {
                        matches!(k.as_str(), "step_index" | "operation_index")
                            && v.parse::<usize>().is_ok()
                    })
            }
            "json_source" | "json_destination" => {
                source.is_some_and(|n| n.kind == "JsonConversion")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error("AIL-JSON-001", format!("invalid {} JSON edge", edge.kind))
                    .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn namespace(op: &AilJsonConversion, alias: &str) -> AilJsonConversion {
    let prefix = runtime_subject_key(alias);
    match op {
        AilJsonConversion::Parse { source, key } => AilJsonConversion::Parse {
            source: format!("{prefix}.{source}"),
            key: format!("{prefix}.{key}"),
        },
        AilJsonConversion::Format { source, key } => AilJsonConversion::Format {
            source: format!("{prefix}.{source}"),
            key: format!("{prefix}.{key}"),
        },
        AilJsonConversion::Invalid(message) => AilJsonConversion::Invalid(message.clone()),
    }
}
pub(super) fn instruction(
    op: &AilJsonConversion,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let (kind, source, key) = op.parts()?;
    field(source)?;
    field(key)?;
    let (typed, text_key) = if kind == "parse" {
        (key, source)
    } else {
        (source, key)
    };
    let type_name = runtime_field_type_name(document, typed)
        .filter(|t| supported(t))
        .ok_or_else(|| error("JSON value operand must have a supported declared type"))?;
    if runtime_field_type_name(document, text_key) != Some("Text") {
        return Err(error("JSON text operand must be declared Text"));
    }
    let opcode = if kind == "parse" {
        "JSON_PARSE"
    } else {
        "JSON_FORMAT"
    };
    Ok(AilBytecodeInstruction::new(
        opcode,
        &[
            ("source", source.into()),
            ("key", key.into()),
            ("type", type_name.into()),
        ],
    ))
}
pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    if !matches!(instruction.opcode.as_str(), "JSON_PARSE" | "JSON_FORMAT")
        || instruction
            .operands
            .keys()
            .map(String::as_str)
            .ne(["key", "source", "type"])
    {
        return Err(error(
            "JSON instruction requires exactly key, source, and type operands",
        ));
    }
    if !supported(&instruction.operands["type"]) {
        return Err(error("unsupported JSON conversion type"));
    }
    field(&instruction.operands["source"])?;
    field(&instruction.operands["key"])
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let edge_kind = match kind {
        "reads" => "json_source",
        "writes" => "json_destination",
        _ => return Vec::new(),
    };
    let mut result = Vec::new();
    for step in core.graph.edges.iter().filter(|e| e.kind == "json_step") {
        for edge in core
            .graph
            .edges
            .iter()
            .filter(|e| e.source == step.target && e.kind == edge_kind)
        {
            result.push((edge, step.source.as_str()));
        }
    }
    result
}

fn list_value(input: &str, type_name: &str, parsing: bool) -> Result<String, String> {
    let result = if type_name == "List<Float>" {
        list_operations::canonical_float_json(input, parsing)
    } else {
        list_operations::canonical_literal(
            input,
            if type_name == "List<Int>" {
                "Int"
            } else {
                "Text"
            },
        )
    };
    result.map_err(|error| {
        if error.starts_with("AIL-RUNTIME-LIST-LIMIT:") {
            limit("list size or item count exceeded")
        } else {
            runtime_error("invalid typed JSON list")
        }
    })
}
fn copy_text(input: &str) -> Result<String, String> {
    let mut value = String::new();
    value
        .try_reserve_exact(input.len())
        .map_err(|_| limit("cannot allocate JSON result"))?;
    value.push_str(input);
    Ok(value)
}
fn encode_text(input: &str) -> Result<String, String> {
    let encoded_bytes = 2 + input
        .chars()
        .map(|c| match c {
            '"' | '\\' | '\n' | '\r' | '\t' => 2,
            c if c.is_control() => 6,
            c => c.len_utf8(),
        })
        .sum::<usize>();
    if encoded_bytes > MAX_BYTES {
        return Err(limit("encoded JSON result exceeds 1048576 bytes"));
    }
    // Keep exactly the same escaping policy as literals and canonical list values.
    Ok(json_string(input))
}
fn reserve(bytes: usize, allocated: &mut usize) -> Result<(), String> {
    let total = allocated
        .checked_add(bytes.max(1))
        .filter(|n| *n <= MAX_ALLOCATED)
        .ok_or_else(|| limit("JSON allocation budget exceeded"))?;
    *allocated = total;
    Ok(())
}
pub(super) fn parse_runtime_value(type_name: &str, input: &str) -> Result<String, String> {
    if input.len() > MAX_BYTES {
        return Err(limit("conversion input exceeds 1048576 bytes"));
    }
    match type_name {
        "Float" => structured_json::floating_number(input),
        "Json" => structured_json::canonical(input),
        "List<Int>" | "List<Float>" | "List<Text>" => list_value(input, type_name, true),
        _ => literal_assignment::decode(type_name, input)
            .map_err(|_| runtime_error("invalid typed JSON value")),
    }
}
pub(super) fn format_runtime_value(type_name: &str, input: &str) -> Result<String, String> {
    if input.len() > MAX_BYTES {
        return Err(limit("conversion input exceeds 1048576 bytes"));
    }
    match type_name {
        "Float" => {
            let bits = crate::float_number::parse(input)
                .map_err(|_| runtime_error("invalid Float transport"))?;
            if !f64::from_bits(bits).is_finite() {
                return Err(runtime_error("JSON Float must be finite"));
            }
            float_operations::runtime_format(bits)
                .map_err(|_| runtime_error("invalid Float transport"))
        }
        "Json" => structured_json::canonical(input),
        "List<Int>" | "List<Float>" | "List<Text>" => list_value(input, type_name, false),
        "Int" => parse_integer(input)
            .map(|value| value.to_string())
            .map_err(|_| runtime_error("invalid integer transport")),
        "Bool" if matches!(input, "true" | "false") => copy_text(input),
        "Bool" => Err(runtime_error("invalid Boolean transport")),
        "Text" => encode_text(input),
        _ => Err(runtime_error("unsupported JSON value type")),
    }
}
pub(super) fn reserve_result(
    type_name: &str,
    parsing: bool,
    bytes: usize,
    text_allocated: &mut usize,
    list_allocated: &mut usize,
) -> Result<(), String> {
    if bytes > MAX_BYTES {
        return Err(limit("JSON result exceeds 1048576 bytes"));
    }
    if !parsing || type_name == "Text" {
        reserve(bytes, text_allocated)?;
    } else if matches!(
        type_name,
        "List<Int>" | "List<Float>" | "List<Text>" | "Json"
    ) {
        reserve(bytes, list_allocated)?;
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
    let input = state
        .get(&instruction.operands["source"])
        .ok_or_else(|| runtime_error("missing conversion source"))?;
    if input.len() > MAX_BYTES {
        return Err(limit("conversion input exceeds 1048576 bytes"));
    }
    let type_name = instruction.operands["type"].as_str();
    let parsing = instruction.opcode == "JSON_PARSE";
    let value = if parsing {
        parse_runtime_value(type_name, input)?
    } else {
        format_runtime_value(type_name, input)?
    };
    reserve_result(
        type_name,
        parsing,
        value.len(),
        text_allocated,
        list_allocated,
    )?;
    state.insert(instruction.operands["key"].clone(), value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floating_json_bridge_is_finite_and_preserves_signed_zero() {
        for (input, expected) in [
            ("1.5", "1.5e0"),
            ("-0", "-0e0"),
            ("5e-324", "5e-324"),
            ("1e-9999", "0e0"),
        ] {
            assert_eq!(parse_runtime_value("Float", input).unwrap(), expected);
            assert_eq!(format_runtime_value("Float", expected).unwrap(), expected);
            assert_eq!(
                literal_assignment::decode("Float", input).unwrap(),
                expected
            );
        }
        for input in [
            "1e400", "nan", "inf", "null", "true", "\"1.5\"", "+1.5", "01.5", "1.", ".5",
        ] {
            assert!(
                parse_runtime_value("Float", input)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON:"),
                "{input}"
            );
        }
        for input in ["nan", "inf", "-inf", "1e400", "invalid"] {
            assert!(
                format_runtime_value("Float", input)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON:"),
                "{input}"
            );
        }
        let (mut text, mut lists) = (17, 23);
        reserve_result("Float", true, 20, &mut text, &mut lists).unwrap();
        assert_eq!((text, lists), (17, 23));
    }
    #[test]
    fn floating_json_checks_canonical_source_limit_before_type() {
        let input = format!("\"{}\"", "\u{7f}".repeat(200_000));
        assert!(
            parse_runtime_value("Float", &input)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
    }
    #[test]
    fn recursive_json_preserves_number_tokens_and_uses_collection_budget() {
        let raw =
            "{\"items\":[null,true,{\"n\":1.2300e+999,\"big\":9223372036854775808}],\"zero\":-0}";
        let parse = AilBytecodeInstruction::new(
            "JSON_PARSE",
            &[
                ("source", "data.raw".into()),
                ("key", "data.value".into()),
                ("type", "Json".into()),
            ],
        );
        let format = AilBytecodeInstruction::new(
            "JSON_FORMAT",
            &[
                ("source", "data.value".into()),
                ("key", "output.text".into()),
                ("type", "Json".into()),
            ],
        );
        let mut state = BTreeMap::from([("data.raw".into(), raw.into())]);
        let (mut text, mut lists) = (0, 0);
        execute(&parse, &mut state, &mut text, &mut lists).unwrap();
        assert_eq!(state["data.value"], raw);
        assert_eq!((text, lists), (0, raw.len()));
        execute(&format, &mut state, &mut text, &mut lists).unwrap();
        assert_eq!(state["output.text"], raw);
        assert_eq!((text, lists), (raw.len(), raw.len()));
    }

    fn core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Json manages values.\n\nA Data has:\n- raw: Text\n- values: List<Int>\n\nAn Output has:\n- text: Text\n\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Converted\n";
        let mut document = parse_ail_package_spec_text(&package, source).unwrap();
        let action = document.actions.get_mut("Run").unwrap();
        action.json_conversions = vec![
            AilJsonConversion::Parse {
                source: "data.raw".into(),
                key: "data.values".into(),
            },
            AilJsonConversion::Format {
                source: "data.values".into(),
                key: "output.text".into(),
            },
        ];
        action.steps.insert(0, AilActionStep::Json(1));
        action.steps.insert(0, AilActionStep::Json(0));
        elaborate_ail_core(&package, &document)
    }
    #[test]
    fn saved_core_preserves_conversions_and_rejects_ignored_semantics() {
        let original = core_fixture();
        assert_eq!(
            original
                .graph
                .nodes
                .iter()
                .filter(|n| n.kind == "JsonConversion")
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
        let operations = from_core(&saved, action);
        assert_eq!(operations.len(), 2);
        assert!(
            operations
                .iter()
                .any(|op| op.source() == "the system parses JSON text data.raw into data.values")
        );
        for kind in ["json_step", "json_source", "json_destination"] {
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
            ("json_kind", "unknown"),
            ("value_type", "Decimal"),
            ("hidden", "ignored"),
        ] {
            let mut candidate = original.clone();
            candidate
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == "JsonConversion")
                .unwrap()
                .attributes
                .insert(attribute.into(), value.into());
            assert!(!check(&candidate).is_empty(), "{attribute}");
        }
        let mut candidate = original;
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "JsonConversion")
            .unwrap()
            .type_name = Some("Text".into());
        assert!(!check(&candidate).is_empty());
    }
    #[test]
    fn core_checks_profiles_global_types_and_derived_secret_access() {
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
        let mut local = original.clone();
        local
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field")
            .unwrap()
            .attributes
            .insert("function_owner".into(), "hidden".into());
        assert!(!check(&local).is_empty());
        let mut mismatch = original.clone();
        mismatch
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name == "Data.raw")
            .unwrap()
            .type_name = Some("Int".into());
        assert!(!check(&mismatch).is_empty());
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
    fn command(opcode: &str, type_name: &str) -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(
            opcode,
            &[
                ("key", "output.value".into()),
                ("source", "data.value".into()),
                ("type", type_name.into()),
            ],
        )
    }
    fn convert(
        opcode: &str,
        type_name: &str,
        input: &str,
    ) -> Result<(String, usize, usize), String> {
        let mut state = BTreeMap::from([("data.value".into(), input.into())]);
        let mut text = 0;
        let mut list = 0;
        execute(
            &command(opcode, type_name),
            &mut state,
            &mut text,
            &mut list,
        )?;
        Ok((state.remove("output.value").unwrap(), text, list))
    }

    #[test]
    fn parses_all_five_types_and_charges_only_result_storage() {
        for (type_name, input, expected, text, list) in [
            ("Int", " \t-0\r\n", "0", 0, 0),
            ("Int", "-9223372036854775808", "-9223372036854775808", 0, 0),
            ("Bool", " false ", "false", 0, 0),
            ("Text", r#""\ud83d\ude42\u0000é""#, "🙂\0é", 7, 0),
            ("Text", "\"\"", "", 1, 0),
            ("List<Int>", " [1, -0, -2] ", "[1,0,-2]", 0, 8),
            (
                "List<Text>",
                r#"["é","\u0085"]"#,
                "[\"é\",\"\\u0085\"]",
                0,
                15,
            ),
        ] {
            assert_eq!(
                convert("JSON_PARSE", type_name, input).unwrap(),
                (expected.into(), text, list),
                "{type_name}"
            );
        }
    }

    #[test]
    fn formats_transports_with_canonical_bytes_and_only_text_budget() {
        for (type_name, input, expected) in [
            ("Int", "+00012", "12"),
            ("Int", "-000", "0"),
            ("Bool", "true", "true"),
            (
                "Text",
                "é\0\n\t\r\"\\\u{85}",
                "\"é\\u0000\\n\\t\\r\\\"\\\\\\u0085\"",
            ),
            ("List<Int>", " [1,-0, -2] ", "[1,0,-2]"),
            (
                "List<Text>",
                r#"["\ud83d\ude42", "\b"]"#,
                "[\"🙂\",\"\\u0008\"]",
            ),
        ] {
            assert_eq!(
                convert("JSON_FORMAT", type_name, input).unwrap(),
                (expected.into(), expected.len(), 0),
                "{type_name}"
            );
        }
    }

    #[test]
    fn strict_json_and_runtime_type_errors_do_not_reveal_input() {
        for (opcode, type_name, input) in [
            ("JSON_PARSE", "Int", "+1"),
            ("JSON_PARSE", "Int", "01"),
            ("JSON_PARSE", "Int", "1.0"),
            ("JSON_PARSE", "Int", "1e0"),
            ("JSON_PARSE", "Int", "9223372036854775808"),
            ("JSON_PARSE", "Int", "\u{85}1"),
            ("JSON_PARSE", "Bool", "True"),
            ("JSON_PARSE", "Text", "null"),
            ("JSON_PARSE", "Text", "{}"),
            ("JSON_PARSE", "Text", r#""\ud800""#),
            ("JSON_PARSE", "Text", r#""\udc00""#),
            ("JSON_PARSE", "Text", "\"sensitive text\" trailing"),
            ("JSON_PARSE", "Text", "\"secret\ntext\""),
            ("JSON_PARSE", "List<Int>", "[1,]"),
            ("JSON_PARSE", "List<Int>", "[[1]]"),
            ("JSON_PARSE", "List<Text>", "[1]"),
            ("JSON_FORMAT", "Int", " 1"),
            ("JSON_FORMAT", "Int", "1.0"),
            ("JSON_FORMAT", "Int", "9223372036854775808"),
            ("JSON_FORMAT", "Bool", " true"),
            ("JSON_FORMAT", "Bool", "TRUE"),
            ("JSON_FORMAT", "List<Int>", "[+1]"),
        ] {
            let error = convert(opcode, type_name, input).unwrap_err();
            assert!(
                error.starts_with("AIL-RUNTIME-JSON:"),
                "{opcode} {type_name}: {error}"
            );
            assert!(!error.contains("sensitive") && !error.contains("secret"));
        }
    }

    #[test]
    fn bounds_cover_input_result_canonical_expansion_and_item_count() {
        for opcode in ["JSON_PARSE", "JSON_FORMAT"] {
            assert!(
                convert(opcode, "Int", &"x".repeat(MAX_BYTES + 1))
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON-LIMIT:")
            );
        }
        let exact = format!("\"{}\"", "x".repeat(MAX_BYTES - 2));
        assert_eq!(
            convert("JSON_PARSE", "Text", &exact).unwrap().1,
            MAX_BYTES - 2
        );
        assert_eq!(
            convert("JSON_FORMAT", "Text", &"x".repeat(MAX_BYTES - 2))
                .unwrap()
                .1,
            MAX_BYTES
        );
        assert!(
            convert("JSON_FORMAT", "Text", &"x".repeat(MAX_BYTES - 1))
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        let expanded = format!("[\"{}\"]", "\u{85}".repeat(200_000));
        for opcode in ["JSON_PARSE", "JSON_FORMAT"] {
            assert!(
                convert(opcode, "List<Text>", &expanded)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON-LIMIT:")
            );
            let malformed = expanded.clone() + " trailing";
            assert!(
                convert(opcode, "List<Text>", &malformed)
                    .unwrap_err()
                    .starts_with("AIL-RUNTIME-JSON:")
            );
            assert!(
                convert(
                    opcode,
                    "List<Int>",
                    &format!("[{}]", vec!["0"; 65_536].join(","))
                )
                .is_ok()
            );
            assert!(
                convert(
                    opcode,
                    "List<Int>",
                    &format!("[{}]", vec!["0"; 65_537].join(","))
                )
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
            );
        }
    }

    #[test]
    fn in_place_conversion_preserves_copies_and_failed_budget_is_atomic() {
        let mut instruction = command("JSON_PARSE", "Text");
        instruction
            .operands
            .insert("key".into(), "data.value".into());
        let mut state = BTreeMap::from([
            ("data.value".into(), "\"old\"".into()),
            ("data.copy".into(), "\"old\"".into()),
        ]);
        let mut text = MAX_ALLOCATED - 3;
        let mut list = MAX_ALLOCATED;
        execute(&instruction, &mut state, &mut text, &mut list).unwrap();
        assert_eq!(state["data.value"], "old");
        assert_eq!(state["data.copy"], "\"old\"");
        assert_eq!(text, MAX_ALLOCATED);
        assert_eq!(list, MAX_ALLOCATED);
        instruction.opcode = "JSON_FORMAT".into();
        let before = state.clone();
        assert!(
            execute(&instruction, &mut state, &mut text, &mut list)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert_eq!(state, before);
        assert_eq!(text, MAX_ALLOCATED);
        assert_eq!(list, MAX_ALLOCATED);
        let mut instruction = command("JSON_PARSE", "List<Int>");
        state.insert("data.value".into(), "[]".into());
        list = MAX_ALLOCATED - 2;
        execute(&instruction, &mut state, &mut text, &mut list).unwrap();
        assert_eq!(list, MAX_ALLOCATED);
        assert_eq!(text, MAX_ALLOCATED);
        instruction.operands.insert("type".into(), "Int".into());
        state.insert("data.value".into(), "1".into());
        execute(&instruction, &mut state, &mut text, &mut list).unwrap();
        assert_eq!(state["output.value"], "1");
    }

    #[test]
    fn source_namespaces_only_fields_and_protocol_is_exact() {
        for (source, expected) in [
            (
                "the system parses JSON text data.raw into data.values",
                "the system parses JSON text pkg.data.raw into pkg.data.values",
            ),
            (
                "the system formats data.values as JSON text into output.text",
                "the system formats pkg.data.values as JSON text into pkg.output.text",
            ),
        ] {
            let op = parse_bullet(source).unwrap().unwrap();
            assert_eq!(namespace(&op, "pkg").source(), expected);
        }
        for text in [
            "the system parses JSON text x into output.text",
            "the system formats data.value as JSON text",
            "the system parses JSON text",
        ] {
            assert!(parse_bullet(text).unwrap().is_err());
        }
        for opcode in ["JSON_PARSE", "JSON_FORMAT"] {
            assert!(verify(&command(opcode, "Text")).is_ok());
            let mut invalid = command(opcode, "Text");
            invalid.operands.insert("hidden".into(), "ignored".into());
            assert!(verify(&invalid).is_err());
            assert!(verify(&command(opcode, "List<Bool>")).is_err());
            assert!(verify(&command(opcode, "Secret<Text>")).is_err());
        }
    }

    #[test]
    fn missing_malformed_and_oversize_values_leave_destination_and_budgets_unchanged() {
        let instruction = command("JSON_PARSE", "Text");
        let mut state = BTreeMap::from([("output.value".into(), "original".into())]);
        let mut text = 7;
        let mut list = 11;
        let error = execute(&instruction, &mut state, &mut text, &mut list).unwrap_err();
        assert!(error.starts_with("AIL-RUNTIME-JSON:"));
        for input in ["\"hidden\" trailing".to_string(), "x".repeat(MAX_BYTES + 1)] {
            state.insert("data.value".into(), input);
            let before = state.clone();
            assert!(execute(&instruction, &mut state, &mut text, &mut list).is_err());
            assert_eq!(state, before);
            assert_eq!((text, list), (7, 11));
        }
        let mut malformed = command("JSON_FORMAT", "Bool");
        malformed
            .operands
            .insert("type".into(), "List<Bool>".into());
        assert!(
            execute(&malformed, &mut state, &mut text, &mut list)
                .unwrap_err()
                .starts_with("AIL-JSON-001:")
        );
        assert_eq!(state["output.value"], "original");
    }

    #[test]
    fn canonical_result_and_shared_budget_limits_do_not_charge_source_bytes() {
        let spaced_empty = format!("[]{}", " ".repeat(MAX_BYTES - 2));
        assert_eq!(
            convert("JSON_PARSE", "List<Int>", &spaced_empty).unwrap(),
            ("[]".into(), 0, 2)
        );
        assert_eq!(
            convert("JSON_FORMAT", "List<Int>", &spaced_empty).unwrap(),
            ("[]".into(), 2, 0)
        );
        let mut state = BTreeMap::from([
            ("data.value".into(), "[]".into()),
            ("output.value".into(), "old".into()),
        ]);
        let mut text = MAX_ALLOCATED - 2;
        let mut list = MAX_ALLOCATED;
        execute(
            &command("JSON_FORMAT", "List<Int>"),
            &mut state,
            &mut text,
            &mut list,
        )
        .unwrap();
        assert_eq!((text, list), (MAX_ALLOCATED, MAX_ALLOCATED));
        assert_eq!(state["output.value"], "[]");
        state.insert("output.value".into(), "unchanged".into());
        assert!(
            execute(
                &command("JSON_PARSE", "List<Int>"),
                &mut state,
                &mut text,
                &mut list
            )
            .unwrap_err()
            .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert_eq!(state["output.value"], "unchanged");
        assert_eq!((text, list), (MAX_ALLOCATED, MAX_ALLOCATED));
    }

    #[test]
    fn direct_document_lowering_infers_value_type_and_rejects_wrong_text_operands() {
        let core = core_fixture();
        let document = ail_document_from_core(&core);
        for operation in &document.actions["Run"].json_conversions {
            let lowered = instruction(operation, &document).unwrap();
            assert_eq!(lowered.operands["type"], "List<Int>");
            assert!(verify(&lowered).is_ok());
        }
        for operation in [
            AilJsonConversion::Parse {
                source: "data.values".into(),
                key: "data.raw".into(),
            },
            AilJsonConversion::Format {
                source: "data.raw".into(),
                key: "data.values".into(),
            },
            AilJsonConversion::Parse {
                source: "data.raw".into(),
                key: "missing.field".into(),
            },
        ] {
            assert!(instruction(&operation, &document).is_err());
        }
    }
}
