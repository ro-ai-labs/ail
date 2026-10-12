//! Explicit typed constants, retained as JSON literals through Core and bytecode.
use super::*;
use crate::expression::IntExpr;

const MAX_TEXT_BYTES: usize = 1_048_576;
const MAX_LITERAL_BYTES: usize = 6_291_458;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilLiteralAssignment {
    Assign { key: String, literal: String },
    Invalid(String),
}

impl AilLiteralAssignment {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Assign { key, literal } => format!(
                "the system assigns {} to {key}",
                canonical_untyped(literal).unwrap_or_else(|_| literal.clone())
            ),
            Self::Invalid(message) => format!("invalid literal assignment: {message}"),
        }
    }
}
fn error(message: impl Display) -> String {
    format!("AIL-LITERAL-001: {message}")
}
fn json_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}
fn supported(type_name: &str) -> bool {
    matches!(
        type_name,
        "Int"
            | "Float"
            | "Bool"
            | "Text"
            | "Bytes"
            | "List<Int>"
            | "List<Float>"
            | "List<Text>"
            | "Json"
    )
}
fn field(key: &str) -> Result<(), String> {
    if matches!(IntExpr::parse(key),Ok(IntExpr::Field(parsed)) if parsed==key) {
        Ok(())
    } else {
        Err(error("literal destination must be a field reference"))
    }
}
pub(super) fn decode(type_name: &str, literal: &str) -> Result<String, String> {
    if literal.len() > MAX_LITERAL_BYTES {
        return Err(error("raw literal exceeds 6291458 bytes"));
    }
    let text = literal.trim_matches(json_whitespace);
    match type_name {
        "Json" => structured_json::canonical(literal).map_err(error),
        "Float" => structured_json::floating_number(literal).map_err(error),
        "Int" => {
            let digits = text.strip_prefix('-').unwrap_or(text);
            if digits.is_empty()
                || !digits.bytes().all(|b| b.is_ascii_digit())
                || (digits.len() > 1 && digits.starts_with('0'))
            {
                return Err(error("Int literal must be a strict JSON integer"));
            }
            text.parse::<i64>()
                .map(|n| n.to_string())
                .map_err(|_| error("Int literal is outside i64 range"))
        }
        "Bool" if matches!(text, "true" | "false") => Ok(text.into()),
        "Bool" => Err(error("Bool literal must be true or false")),
        "Bytes" => {
            let mut parser = AilJsonParser::new(text);
            let value = parser.parse_string().map_err(error)?;
            if !parser.is_finished() {
                return Err(error("unexpected content after Bytes literal"));
            }
            bytes_operations::runtime_validate(&value).map_err(error)?;
            Ok(value)
        }
        "Text" => {
            let mut parser = AilJsonParser::new(text);
            let value = parser.parse_string().map_err(error)?;
            if !parser.is_finished() {
                return Err(error("unexpected content after text literal"));
            }
            if value.len() > MAX_TEXT_BYTES {
                return Err(error("decoded text literal exceeds 1048576 bytes"));
            }
            Ok(value)
        }
        "List<Int>" => list_operations::canonical_literal(literal, "Int").map_err(error),
        "List<Float>" => list_operations::canonical_float_json(literal, true).map_err(error),
        "List<Text>" => list_operations::canonical_literal(literal, "Text").map_err(error),
        _ => Err(error(
            "literal destination type must be Int, Float, Bool, Text, Bytes, List<Int>, List<Float>, List<Text>, or Json",
        )),
    }
}
fn canonical(type_name: &str, literal: &str) -> Result<String, String> {
    let value = decode(type_name, literal)?;
    Ok(if matches!(type_name, "Text" | "Bytes") {
        json_string(&value)
    } else {
        value
    })
}
fn canonical_untyped(literal: &str) -> Result<String, String> {
    // Preserve number tokens until the destination's declared type is known.
    // Existing Text/list literal limits can exceed the recursive Json limits.
    if let Ok(value) = structured_json::canonical(literal) {
        return Ok(value);
    }
    let text = literal.trim_matches(json_whitespace);
    let value = if text.starts_with('"') {
        canonical("Text", literal)
            .or_else(|text_error| canonical("Bytes", literal).map_err(|_| text_error))
    } else if text.starts_with('[') {
        canonical("List<Int>", literal)
            .or_else(|_| canonical("List<Text>", literal))
            .or_else(|_| canonical("List<Float>", literal))
    } else if matches!(text, "true" | "false") {
        canonical("Bool", literal)
    } else {
        canonical("Int", literal)
    }?;
    // Large escaped Text constants are valid, but Json has a smaller raw-input
    // bound. Retain this spelling until the destination type can enforce it.
    Ok(if literal.len() > MAX_TEXT_BYTES {
        literal.to_string()
    } else {
        value
    })
}
pub(super) fn parse_bullet(text: &str) -> Option<Result<AilLiteralAssignment, String>> {
    let text = trim_sentence(text);
    let Some(rest) = text.strip_prefix("the system assigns ") else {
        return text
            .starts_with("the system assigns")
            .then(|| Err(error("malformed literal assignment")));
    };
    Some((|| {
        let (literal, key) = rest
            .rsplit_once(" to ")
            .ok_or_else(|| error("expected 'assigns literal to field'"))?;
        field(key)?;
        Ok(AilLiteralAssignment::Assign {
            key: key.into(),
            literal: canonical_untyped(literal)?,
        })
    })())
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, assignment) in action.literal_assignments.iter().enumerate() {
            let (key, literal) = match assignment {
                AilLiteralAssignment::Assign { key, literal } => (Some(key), literal.as_str()),
                AilLiteralAssignment::Invalid(message) => (None, message.as_str()),
            };
            let type_name = key
                .and_then(|key| runtime_field_type_name(document, key))
                .unwrap_or("invalid");
            let literal = canonical(type_name, literal).unwrap_or_else(|_| literal.to_string());
            let node = graph.add_node(
                "LiteralAssignment",
                format!("{}.literal{index}", owner.name),
                None,
                attr(&[("value_type", type_name), ("literal", &literal)]),
            );
            attach_provenance(
                graph,
                &node,
                format!("action:{}.literal:{index}", owner.name),
            );
            if let Some(key) = key {
                let target = graph
                    .nodes
                    .iter()
                    .find(|n| {
                        n.kind == "Field"
                            && !n.attributes.contains_key("function_owner")
                            && !n.attributes.contains_key("local_name")
                            && expression_core::field_key(n) == *key
                    })
                    .cloned()
                    .unwrap_or_else(|| {
                        let target = graph.add_node("Effect", key, None, BTreeMap::new());
                        attach_provenance(
                            graph,
                            &target,
                            format!("action:{}.literal:{index}.destination", owner.name),
                        );
                        target
                    });
                graph.add_edge("literal_destination", &node, &target, BTreeMap::new());
            }
            action_order::append(graph, "literal_step", &owner, &node, BTreeMap::new(), index);
        }
    }
}
fn decode_node(core: &AilCore, node: &Node) -> Result<AilLiteralAssignment, String> {
    if node.kind != "LiteralAssignment" {
        return Err(error("literal_step must target a LiteralAssignment"));
    }
    if node.type_name.is_some() {
        return Err(error("literal assignment type belongs in value_type only"));
    }
    if node
        .attributes
        .keys()
        .map(String::as_str)
        .ne(["literal", "value_type"])
    {
        return Err(error(
            "literal node requires exactly literal and value_type attributes",
        ));
    }
    let type_name = &node.attributes["value_type"];
    if !supported(type_name) {
        return Err(error("unsupported literal value_type"));
    }
    if core.graph.edges.iter().any(|e| {
        e.source == node.id && !matches!(e.kind.as_str(), "literal_destination" | "has_provenance")
    }) {
        return Err(error("unexpected edge on literal assignment"));
    }
    let mut destinations = core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == "literal_destination");
    let edge = destinations
        .next()
        .ok_or_else(|| error("literal assignment has no destination"))?;
    if destinations.next().is_some() {
        return Err(error("literal assignment has duplicate destinations"));
    }
    if !edge.attributes.is_empty() {
        return Err(error("literal destination edge cannot have attributes"));
    }
    let target = core
        .graph
        .nodes
        .iter()
        .find(|n| n.id == edge.target)
        .ok_or_else(|| error("literal destination is missing"))?;
    if target.kind != "Field"
        || target.type_name.as_deref() != Some(type_name)
        || target.attributes.contains_key("function_owner")
        || target.attributes.contains_key("local_name")
    {
        return Err(error(
            "literal destination must be a global field matching value_type",
        ));
    }
    let key = expression_core::field_key(target);
    field(&key)?;
    Ok(AilLiteralAssignment::Assign {
        key,
        literal: canonical(type_name, &node.attributes["literal"])?,
    })
}
pub(super) fn from_core(core: &AilCore, action: &Node) -> Vec<AilLiteralAssignment> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == action.id && e.kind == "literal_step")
        .map(|edge| {
            core.graph
                .nodes
                .iter()
                .find(|n| n.id == edge.target)
                .ok_or_else(|| error("missing literal assignment node"))
                .and_then(|n| decode_node(core, n))
                .unwrap_or_else(AilLiteralAssignment::Invalid)
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
        .filter(|n| n.kind == "LiteralAssignment")
    {
        let result = (|| {
            if !matches!(core.package.profile.as_str(), "Application" | "C interop") {
                return Err(error(
                    "literal assignments require Application or C interop profile",
                ));
            }
            let owners: Vec<_> = core
                .graph
                .edges
                .iter()
                .filter(|e| e.target == node.id && e.kind == "literal_step")
                .collect();
            if owners.len() != 1
                || !core
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == owners[0].source && executable_action(n))
            {
                return Err(error(
                    "literal assignment requires exactly one executable action owner",
                ));
            }
            decode_node(core, node).map(|_| ())
        })();
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-LITERAL-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for edge in &core.graph.edges {
        let source = core.graph.nodes.iter().find(|n| n.id == edge.source);
        let target = core.graph.nodes.iter().find(|n| n.id == edge.target);
        let valid = match edge.kind.as_str() {
            "literal_step" => {
                source.is_some_and(executable_action)
                    && target.is_some_and(|n| n.kind == "LiteralAssignment")
                    && edge
                        .attributes
                        .get("step_index")
                        .is_some_and(|s| s.parse::<usize>().is_ok())
                    && edge.attributes.iter().all(|(key, value)| {
                        matches!(key.as_str(), "step_index" | "operation_index")
                            && value.parse::<usize>().is_ok()
                    })
            }
            "literal_destination" => {
                source.is_some_and(|n| n.kind == "LiteralAssignment")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-LITERAL-001",
                    format!("invalid {} literal edge", edge.kind),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn namespace(assignment: &AilLiteralAssignment, alias: &str) -> AilLiteralAssignment {
    match assignment {
        AilLiteralAssignment::Assign { key, literal } => AilLiteralAssignment::Assign {
            key: format!("{}.{key}", runtime_subject_key(alias)),
            literal: literal.clone(),
        },
        AilLiteralAssignment::Invalid(message) => AilLiteralAssignment::Invalid(message.clone()),
    }
}
pub(super) fn instruction(
    assignment: &AilLiteralAssignment,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let AilLiteralAssignment::Assign { key, literal } = assignment else {
        return Err(error("invalid literal assignment"));
    };
    field(key)?;
    let type_name = runtime_field_type_name(document, key)
        .ok_or_else(|| error(format!("unknown destination {key}")))?;
    let value = canonical(type_name, literal)?;
    Ok(AilBytecodeInstruction::new(
        "ASSIGN_LITERAL",
        &[
            ("key", key.clone()),
            ("type", type_name.into()),
            ("value", value),
        ],
    ))
}
pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    runtime_value(instruction).map(|_| ())
}
pub(super) fn runtime_value(instruction: &AilBytecodeInstruction) -> Result<String, String> {
    if instruction.opcode != "ASSIGN_LITERAL"
        || instruction
            .operands
            .keys()
            .map(String::as_str)
            .ne(["key", "type", "value"])
    {
        return Err(error(
            "ASSIGN_LITERAL requires exactly key, type, and value operands",
        ));
    }
    field(&instruction.operands["key"])?;
    decode(
        &instruction.operands["type"],
        &instruction.operands["value"],
    )
}
pub(super) fn execute(
    instruction: &AilBytecodeInstruction,
    state: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let value = runtime_value(instruction)?;
    state.insert(instruction.operands["key"].clone(), value);
    Ok(())
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    if kind != "writes" {
        return Vec::new();
    }
    let mut result = Vec::new();
    for step in core.graph.edges.iter().filter(|e| e.kind == "literal_step") {
        for edge in core
            .graph
            .edges
            .iter()
            .filter(|e| e.source == step.target && e.kind == "literal_destination")
        {
            result.push((edge, step.source.as_str()));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Constants manages constants.\n\nAn Output has:\n- value: Text\n\nAction: Run.\nWhen Run happens:\n- the system assigns \"hello\" to output.value\n- the system records a trace event named Assigned\n";
        elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, source).unwrap(),
        )
    }

    #[test]
    fn saved_core_rejects_ignored_literal_metadata() {
        let original = core_fixture();
        assert!(check(&original).is_empty(), "{:?}", check(&original));
        for (kind, attribute, value) in [
            ("literal_destination", "expression", "other.value"),
            ("literal_step", "condition", "false"),
            ("literal_step", "operation_index", "invalid"),
        ] {
            let mut core = original.clone();
            core.graph
                .edges
                .iter_mut()
                .find(|edge| edge.kind == kind)
                .unwrap()
                .attributes
                .insert(attribute.into(), value.into());
            let saved = parse_ail_core_text(&render_ail_core(&core)).unwrap();
            assert!(!check(&saved).is_empty(), "{kind} {attribute}");
        }
        let mut core = original;
        core.graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "LiteralAssignment")
            .unwrap()
            .type_name = Some("Int".into());
        let saved = parse_ail_core_text(&render_ail_core(&core)).unwrap();
        assert!(!check(&saved).is_empty(), "unexpected node type_name");
    }

    fn command(type_name: &str, literal: &str) -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(
            "ASSIGN_LITERAL",
            &[
                ("key", "output.value".into()),
                ("type", type_name.into()),
                ("value", literal.into()),
            ],
        )
    }
    #[test]
    fn literals_decode_unicode_and_canonicalize_scalar_and_flat_list_values() {
        for (type_name, literal, expected) in [
            ("Int", " -0 ", "0"),
            ("Int", "-9223372036854775808", "-9223372036854775808"),
            ("Bool", " true ", "true"),
            (
                "Text",
                r#""to output.value\n\ud83d\ude42\u0000""#,
                "to output.value\n🙂\0",
            ),
            ("List<Int>", " [1, -0, -2] ", "[1,0,-2]"),
            (
                "List<Text>",
                r#"["\u00e9", "\ud83d\ude42", "\b"]"#,
                "[\"é\",\"🙂\",\"\\u0008\"]",
            ),
        ] {
            assert_eq!(
                runtime_value(&command(type_name, literal)).unwrap(),
                expected
            );
        }
    }
    #[test]
    fn json_literal_raw_bound_cannot_be_hidden_by_untyped_normalization() {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let literal = format!("\"{}\"", "\\u0061".repeat(175_000));
        assert!(literal.len() > MAX_TEXT_BYTES);
        for type_name in ["Text", "Json"] {
            let source = format!(
                "The application Constants manages values.\n\nA Data has:\n- value: {type_name}\n\nAction: Run.\nWhen Run happens:\n- the system assigns {literal} to data.value\n- the system returns data.value\n- the system records a trace event named Assigned\n"
            );
            let document = parse_ail_package_spec_text(&package, &source).unwrap();
            let core = elaborate_ail_core(&package, &document);
            let result = compile_ail_core_bytecode(&core);
            if type_name == "Json" {
                assert!(
                    result.is_err(),
                    "oversized raw Json literal was normalized past its bound"
                );
            } else {
                let result =
                    run_ail_bytecode_action(&result.unwrap(), "Run", BTreeMap::new()).unwrap();
                assert_eq!(result.final_state["data.value"], "a".repeat(175_000));
            }
        }
    }

    #[test]
    fn destination_namespace_preserves_field_like_text_and_source_keywords() {
        let assignment =
            parse_bullet(r#"the system assigns "input.value to output.value" to output.message"#)
                .unwrap()
                .unwrap();
        assert_eq!(
            namespace(&assignment, "pkg").source(),
            r#"the system assigns "input.value to output.value" to pkg.output.message"#
        );
    }

    #[test]
    fn strict_json_rejects_mistyped_tokens_and_non_json_whitespace() {
        for (type_name, literal) in [
            ("Int", "+1"),
            ("Int", "01"),
            ("Int", "-01"),
            ("Int", "1.0"),
            ("Int", "1e0"),
            ("Int", "9223372036854775808"),
            ("Int", "-9223372036854775809"),
            ("Int", "\u{85}1"),
            ("Bool", "True"),
            ("Bool", "1"),
            ("Text", "null"),
            ("Text", "\"ok\"\u{85}"),
            ("Text", "\"ok\" trailing"),
            ("Text", r#""\ud800""#),
            ("Text", r#""\udc00""#),
            ("Text", r#""\x00""#),
            ("Text", "\"raw\nnewline\""),
            ("List<Int>", "[1,]"),
            ("List<Int>", "[[1]]"),
            ("List<Int>", "[true]"),
            ("List<Int>", "[1,\"2\"]"),
            ("List<Text>", "[1]"),
            ("List<Text>", r#"["\ud800"]"#),
            ("List<Bool>", "[]"),
            ("Decimal", "1"),
        ] {
            assert!(
                verify(&command(type_name, literal)).is_err(),
                "{type_name}: {literal}"
            );
        }
        for text in [
            "the system assigns",
            "the system assigns 1",
            "the system assigns 1 to invalid",
        ] {
            assert!(parse_bullet(text).unwrap().is_err(), "{text}");
        }
        assert!(parse_bullet("the system records a trace event named Assigned").is_none());
    }

    #[test]
    fn text_literal_checks_decoded_and_raw_limits() {
        let exact = format!("\"{}\"", "x".repeat(MAX_TEXT_BYTES));
        assert_eq!(
            runtime_value(&command("Text", &exact)).unwrap().len(),
            MAX_TEXT_BYTES
        );
        assert!(
            verify(&command(
                "Text",
                &format!("\"{}\"", "x".repeat(MAX_TEXT_BYTES + 1))
            ))
            .is_err()
        );
        let escaped = format!("\"{}\"", "\\u0000".repeat(MAX_TEXT_BYTES));
        assert_eq!(escaped.len(), MAX_LITERAL_BYTES);
        assert_eq!(
            runtime_value(&command("Text", &escaped)).unwrap().len(),
            MAX_TEXT_BYTES
        );
        assert!(verify(&command("Text", &(escaped + " "))).is_err());
    }

    #[test]
    fn list_literal_checks_input_canonical_and_item_limits() {
        let exact = format!("[\"{}\"]", "x".repeat(MAX_TEXT_BYTES - 4));
        assert_eq!(
            runtime_value(&command("List<Text>", &exact)).unwrap().len(),
            MAX_TEXT_BYTES
        );
        assert!(verify(&command("List<Text>", &(exact + " "))).is_err());
        let expanded = format!("[\"{}\"]", "\u{85}".repeat(200_000));
        assert!(expanded.len() < MAX_TEXT_BYTES);
        assert!(verify(&command("List<Text>", &expanded)).is_err());
        let exact = format!("[{}]", vec!["0"; 65_536].join(","));
        assert!(verify(&command("List<Int>", &exact)).is_ok());
        let excess = format!("[{}]", vec!["0"; 65_537].join(","));
        assert!(verify(&command("List<Int>", &excess)).is_err());
    }

    #[test]
    fn bytecode_contract_is_exact_and_failed_assignment_is_atomic() {
        let original = command("Text", "\"old\"");
        let mut state = BTreeMap::new();
        execute(&original, &mut state).unwrap();
        state.insert("output.copy".into(), state["output.value"].clone());
        execute(&command("Text", "\"new\""), &mut state).unwrap();
        assert_eq!(state["output.copy"], "old");
        for invalid in [
            {
                let mut i = original.clone();
                i.operands.remove("value");
                i
            },
            {
                let mut i = original.clone();
                i.operands.insert("ignored".into(), "true".into());
                i
            },
            {
                let mut i = original.clone();
                i.operands.insert("key".into(), "not a field".into());
                i
            },
            {
                let mut i = original.clone();
                i.opcode = "SET_STATE".into();
                i
            },
            command("Text", "false"),
        ] {
            let before = state.clone();
            assert!(verify(&invalid).is_err());
            assert!(execute(&invalid, &mut state).is_err());
            assert_eq!(state, before);
        }
    }

    #[test]
    fn saved_core_canonicalizes_literals_and_retains_duplicate_edges_for_validation() {
        let original = core_fixture();
        let mut equivalent = original.clone();
        equivalent
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "LiteralAssignment")
            .unwrap()
            .attributes
            .insert("literal".into(), r#" "\u00e9" "#.into());
        let saved = parse_ail_core_text(&render_ail_core(&equivalent)).unwrap();
        assert!(check(&saved).is_empty());
        let action = saved
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "Action")
            .unwrap();
        assert_eq!(
            from_core(&saved, action)[0].source(),
            r#"the system assigns "é" to output.value"#
        );
        for kind in ["literal_step", "literal_destination"] {
            let mut candidate = original.clone();
            candidate.graph.edges.push(
                candidate
                    .graph
                    .edges
                    .iter()
                    .find(|e| e.kind == kind)
                    .unwrap()
                    .clone(),
            );
            let saved = parse_ail_core_text(&render_ail_core(&candidate)).unwrap();
            assert_eq!(
                saved.graph.edges.iter().filter(|e| e.kind == kind).count(),
                2
            );
            assert!(!check(&saved).is_empty(), "{kind}");
        }
        for (attribute, value) in [
            ("value_type", "Int"),
            ("literal", "true"),
            ("expression", "ignored"),
        ] {
            let mut candidate = original.clone();
            candidate
                .graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == "LiteralAssignment")
                .unwrap()
                .attributes
                .insert(attribute.into(), value.into());
            assert!(!check(&candidate).is_empty(), "{attribute}");
        }
        let mut missing = original;
        missing
            .graph
            .edges
            .retain(|e| e.kind != "literal_destination");
        assert!(!check(&missing).is_empty());
    }

    #[test]
    fn literal_core_requires_executable_profile_action_and_global_destination() {
        let original = core_fixture();
        for profile in ["Compiler", "System", "UI", "AgentTool"] {
            let mut core = original.clone();
            core.package.profile = profile.into();
            assert!(!check(&core).is_empty(), "{profile}");
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
        let mut local = original;
        local
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field")
            .unwrap()
            .attributes
            .insert("function_owner".into(), "hidden".into());
        assert!(!check(&local).is_empty());
    }

    #[test]
    fn literal_secret_write_is_derived_without_descriptive_access_edges() {
        let mut core = core_fixture();
        core.graph.edges.retain(|e| e.kind != "writes");
        core.graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field")
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        assert!(accesses(&core, "reads").is_empty());
        assert_eq!(accesses(&core, "writes").len(), 1);
        assert!(
            check_secret_write_protection(&core)
                .iter()
                .any(|d| d.code == "AIL002")
        );
    }
}
