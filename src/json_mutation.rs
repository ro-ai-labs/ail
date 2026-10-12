//! Immutable typed set/remove operations over bounded recursive JSON values.
use super::*;
use crate::expression::IntExpr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilJsonMutation {
    Set {
        source: String,
        path: String,
        value: String,
        key: String,
    },
    Remove {
        source: String,
        path: String,
        key: String,
    },
    Invalid(String),
}
impl AilJsonMutation {
    pub(super) fn source(&self) -> String {
        match self {
            Self::Set {
                source,
                path,
                value,
                key,
            } => format!(
                "the system sets JSON value at path {path} in {source} to {value} into {key}"
            ),
            Self::Remove { source, path, key } => {
                format!("the system removes JSON value at path {path} from {source} into {key}")
            }
            Self::Invalid(message) => format!("invalid JSON mutation: {message}"),
        }
    }
    fn parts(&self) -> Result<(&str, &str, &str, Option<&str>, &str), String> {
        match self {
            Self::Set {
                source,
                path,
                value,
                key,
            } => Ok(("set", source, path, Some(value), key)),
            Self::Remove { source, path, key } => Ok(("remove", source, path, None, key)),
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
        Err(error("JSON mutation operand must be a field reference"))
    }
}
pub(super) fn parse_bullet(text: &str) -> Option<Result<AilJsonMutation, String>> {
    let text = trim_sentence(text);
    let parsed = if let Some(rest) = text.strip_prefix("the system sets JSON value at path ") {
        rest.split_once(" in ").and_then(|(path, rest)| {
            rest.split_once(" to ").and_then(|(source, rest)| {
                rest.split_once(" into ")
                    .map(|(value, key)| AilJsonMutation::Set {
                        source: source.into(),
                        path: path.into(),
                        value: value.into(),
                        key: key.into(),
                    })
            })
        })
    } else if let Some(rest) = text.strip_prefix("the system removes JSON value at path ") {
        rest.split_once(" from ").and_then(|(path, rest)| {
            rest.split_once(" into ")
                .map(|(source, key)| AilJsonMutation::Remove {
                    source: source.into(),
                    path: path.into(),
                    key: key.into(),
                })
        })
    } else {
        return (text.starts_with("the system sets JSON")
            || text.starts_with("the system removes JSON"))
        .then(|| Err(error("malformed JSON mutation")));
    };
    Some(
        parsed
            .ok_or_else(|| error("malformed JSON mutation"))
            .and_then(|op| {
                let (_, source, path, value, key) = op.parts()?;
                field(source)?;
                field(path)?;
                field(key)?;
                if let Some(value) = value {
                    field(value)?;
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
        for (index, op) in action.json_mutations.iter().enumerate() {
            let (kind, source, path, value, key) =
                op.parts().unwrap_or(("invalid", "", "", None, ""));
            let mut attributes = attr(&[("json_mutation_kind", kind)]);
            if let Some(value) = value {
                attributes.insert(
                    "value_type".into(),
                    runtime_field_type_name(document, value)
                        .unwrap_or("invalid")
                        .into(),
                );
            }
            let node = graph.add_node(
                "JsonMutation",
                format!("{}.json_mutation{index}", owner.name),
                None,
                attributes,
            );
            attach_provenance(
                graph,
                &node,
                format!("action:{}.json_mutation:{index}", owner.name),
            );
            if kind != "invalid" {
                let mut fields = vec![
                    ("json_mutation_source", source),
                    ("json_mutation_path", path),
                    ("json_mutation_destination", key),
                ];
                if let Some(value) = value {
                    fields.push(("json_mutation_value", value));
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
                                format!("action:{}.json_mutation:{index}.{edge_kind}", owner.name),
                            );
                            target
                        });
                    graph.add_edge(edge_kind, &node, &target, BTreeMap::new());
                }
            }
            action_order::append(
                graph,
                "json_mutation_step",
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
        .ok_or_else(|| error("JSON mutation has a missing operand"))?;
    if edges.next().is_some() || !edge.attributes.is_empty() {
        return Err(error(
            "JSON mutation requires one edge per operand without attributes",
        ));
    }
    let target = core
        .graph
        .nodes
        .iter()
        .find(|n| n.id == edge.target)
        .ok_or_else(|| error("JSON mutation operand target is missing"))?;
    if target.kind != "Field"
        || target.type_name.as_deref() != Some(type_name)
        || target.attributes.contains_key("function_owner")
        || target.attributes.contains_key("local_name")
    {
        return Err(error(
            "JSON mutation operand must be a global field of the required type",
        ));
    }
    let key = expression_core::field_key(target);
    field(&key)?;
    Ok(key)
}
fn decode_node(core: &AilCore, node: &Node) -> Result<AilJsonMutation, String> {
    if node.kind != "JsonMutation" || node.type_name.is_some() {
        return Err(error(
            "JSON mutation requires a JsonMutation node without type_name",
        ));
    }
    let kind = node
        .attributes
        .get("json_mutation_kind")
        .map(String::as_str)
        .ok_or_else(|| error("JSON mutation kind is missing"))?;
    let expected: &[&str] = match kind {
        "set" => &["json_mutation_kind", "value_type"],
        "remove" => &["json_mutation_kind"],
        _ => return Err(error("JSON mutation kind must be set or remove")),
    };
    if node
        .attributes
        .keys()
        .map(String::as_str)
        .ne(expected.iter().copied())
    {
        return Err(error(
            "JSON mutation requires exactly its defined attributes",
        ));
    }
    if kind == "set" && !supported(&node.attributes["value_type"]) {
        return Err(error("unsupported JSON mutation value type"));
    }
    if core.graph.edges.iter().any(|e| {
        (e.source == node.id
            && !matches!(
                e.kind.as_str(),
                "json_mutation_source"
                    | "json_mutation_destination"
                    | "json_mutation_path"
                    | "has_provenance"
            )
            && !(kind == "set" && e.kind == "json_mutation_value"))
            || (e.target == node.id && e.kind != "json_mutation_step")
    }) {
        return Err(error("unexpected edge on JSON mutation"));
    }
    let source = operand(core, node, "json_mutation_source", "Json")?;
    let path = operand(core, node, "json_mutation_path", "Text")?;
    let key = operand(core, node, "json_mutation_destination", "Json")?;
    Ok(if kind == "set" {
        AilJsonMutation::Set {
            source,
            path,
            value: operand(
                core,
                node,
                "json_mutation_value",
                &node.attributes["value_type"],
            )?,
            key,
        }
    } else {
        AilJsonMutation::Remove { source, path, key }
    })
}
pub(super) fn from_core(core: &AilCore, action: &Node) -> Vec<AilJsonMutation> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == action.id && e.kind == "json_mutation_step")
        .map(|edge| {
            core.graph
                .nodes
                .iter()
                .find(|n| n.id == edge.target)
                .ok_or_else(|| error("missing JSON mutation node"))
                .and_then(|node| decode_node(core, node))
                .unwrap_or_else(AilJsonMutation::Invalid)
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
    for node in core.graph.nodes.iter().filter(|n| n.kind == "JsonMutation") {
        let result = (|| {
            if !matches!(core.package.profile.as_str(), "Application" | "C interop") {
                return Err(error(
                    "JSON mutation requires Application or C interop profile",
                ));
            }
            let owners: Vec<_> = core
                .graph
                .edges
                .iter()
                .filter(|e| e.target == node.id && e.kind == "json_mutation_step")
                .collect();
            if owners.len() != 1
                || !core
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == owners[0].source && executable_action(n))
            {
                return Err(error(
                    "JSON mutation requires exactly one executable action owner",
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
            "json_mutation_step" => {
                source.is_some_and(executable_action)
                    && target.is_some_and(|n| n.kind == "JsonMutation")
                    && edge
                        .attributes
                        .get("step_index")
                        .is_some_and(|v| v.parse::<usize>().is_ok())
                    && edge.attributes.iter().all(|(k, v)| {
                        matches!(k.as_str(), "step_index" | "operation_index")
                            && v.parse::<usize>().is_ok()
                    })
            }
            "json_mutation_source"
            | "json_mutation_destination"
            | "json_mutation_path"
            | "json_mutation_value" => {
                source.is_some_and(|n| n.kind == "JsonMutation")
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-JSON-001",
                    format!("invalid {} JSON mutation edge", edge.kind),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn namespace(op: &AilJsonMutation, alias: &str) -> AilJsonMutation {
    let prefix = runtime_subject_key(alias);
    match op {
        AilJsonMutation::Set {
            source,
            path,
            value,
            key,
        } => AilJsonMutation::Set {
            source: format!("{prefix}.{source}"),
            path: format!("{prefix}.{path}"),
            value: format!("{prefix}.{value}"),
            key: format!("{prefix}.{key}"),
        },
        AilJsonMutation::Remove { source, path, key } => AilJsonMutation::Remove {
            source: format!("{prefix}.{source}"),
            path: format!("{prefix}.{path}"),
            key: format!("{prefix}.{key}"),
        },
        AilJsonMutation::Invalid(message) => AilJsonMutation::Invalid(message.clone()),
    }
}
pub(super) fn instruction(
    op: &AilJsonMutation,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let (_, source, path, value, key) = op.parts()?;
    for (key, type_name) in [(source, "Json"), (path, "Text"), (key, "Json")] {
        field(key)?;
        if runtime_field_type_name(document, key) != Some(type_name) {
            return Err(error(
                "JSON mutation requires declared Json source/destination and Text path",
            ));
        }
    }
    let mut instruction = AilBytecodeInstruction::new(
        if value.is_some() {
            "JSON_SET"
        } else {
            "JSON_REMOVE"
        },
        &[
            ("source", source.into()),
            ("path", path.into()),
            ("key", key.into()),
        ],
    );
    if let Some(value) = value {
        field(value)?;
        let kind = runtime_field_type_name(document, value)
            .filter(|t| supported(t))
            .ok_or_else(|| error("JSON mutation value must have a supported declared type"))?;
        instruction.operands.insert("value".into(), value.into());
        instruction.operands.insert("type".into(), kind.into());
    }
    Ok(instruction)
}
pub(super) fn verify(instruction: &AilBytecodeInstruction) -> Result<(), String> {
    let expected: &[&str] = match instruction.opcode.as_str() {
        "JSON_SET" => &["key", "path", "source", "type", "value"],
        "JSON_REMOVE" => &["key", "path", "source"],
        _ => return Err(error("unknown JSON mutation opcode")),
    };
    if instruction
        .operands
        .keys()
        .map(String::as_str)
        .ne(expected.iter().copied())
    {
        return Err(error(
            "JSON mutation instruction requires exactly its defined operands",
        ));
    }
    if instruction.opcode == "JSON_SET" {
        if !supported(&instruction.operands["type"]) {
            return Err(error("unsupported JSON mutation type"));
        }
        field(&instruction.operands["value"])?;
    }
    for key in ["source", "path", "key"] {
        field(&instruction.operands[key])?;
    }
    Ok(())
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let mut result = Vec::new();
    for step in core
        .graph
        .edges
        .iter()
        .filter(|e| e.kind == "json_mutation_step")
    {
        for edge in core.graph.edges.iter().filter(|e| {
            e.source == step.target
                && match kind {
                    "reads" => matches!(
                        e.kind.as_str(),
                        "json_mutation_source" | "json_mutation_path" | "json_mutation_value"
                    ),
                    "writes" => e.kind == "json_mutation_destination",
                    _ => false,
                }
        }) {
            result.push((edge, step.source.as_str()));
        }
    }
    result
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
        .ok_or_else(|| runtime_error("missing JSON mutation source"))?;
    let path = state.get(&instruction.operands["path"]).map(String::as_str);
    let result = if instruction.opcode == "JSON_SET" {
        let prepared = structured_json::prepare_mutation(input, path, true)?;
        let value = state
            .get(&instruction.operands["value"])
            .ok_or_else(|| runtime_error("missing JSON mutation value"))?;
        prepared.finish(Some((&instruction.operands["type"], value)))?
    } else if let Some(path) = path {
        structured_json::mutate(input, path, None)?
    } else {
        // Preparation reports any malformed/oversize source before the absent
        // path, following the same priority as set without fetching a value.
        structured_json::prepare_mutation(input, None, false)?.finish(None)?
    };
    json_codec::reserve_result("Json", true, result.len(), text_allocated, list_allocated)?;
    state.insert(instruction.operands["key"].clone(), result);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command(setting: bool, kind: &str) -> AilBytecodeInstruction {
        let mut instruction = AilBytecodeInstruction::new(
            if setting { "JSON_SET" } else { "JSON_REMOVE" },
            &[
                ("key", "out.value".into()),
                ("source", "data.document".into()),
                ("path", "data.path".into()),
            ],
        );
        if setting {
            instruction.operands.insert("type".into(), kind.into());
            instruction
                .operands
                .insert("value".into(), "data.value".into());
        }
        instruction
    }
    #[test]
    fn runtime_validation_priority_precedes_even_missing_value_and_preserves_state() {
        let mut state = BTreeMap::from([("out.value".into(), "unchanged".into())]);
        let mut text = 7;
        let mut list = 11;
        for (source, path, expected) in [
            (None, None, "AIL-RUNTIME-JSON:"),
            (Some("broken"), None, "AIL-RUNTIME-JSON:"),
            (Some("{}"), None, "AIL-RUNTIME-JSON:"),
            (Some("{}"), Some("/missing/child"), "AIL-RUNTIME-JSON-PATH:"),
            (Some("[0]"), Some("/1"), "AIL-RUNTIME-JSON-PATH:"),
            (Some("{}"), Some("/new"), "AIL-RUNTIME-JSON:"),
        ] {
            state.remove("data.document");
            state.remove("data.path");
            if let Some(source) = source {
                state.insert("data.document".into(), source.into());
            }
            if let Some(path) = path {
                state.insert("data.path".into(), path.into());
            }
            let before = state.clone();
            assert!(
                execute(&command(true, "Bool"), &mut state, &mut text, &mut list)
                    .unwrap_err()
                    .starts_with(expected)
            );
            assert_eq!(state, before);
            assert_eq!((text, list), (7, 11));
        }
        state.insert("data.value".into(), "True".into());
        state.insert("data.path".into(), "/bad~2".into());
        assert!(
            execute(&command(true, "Bool"), &mut state, &mut text, &mut list)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-PATH:")
        );
    }
    #[test]
    fn missing_operands_do_not_hide_source_or_path_limits() {
        for setting in [false, true] {
            for source in [
                "x".repeat(1_048_577),
                format!("\"{}\"", "\u{85}".repeat(200_000)),
            ] {
                let mut state = BTreeMap::from([
                    ("data.document".into(), source),
                    ("out.value".into(), "preserved".into()),
                ]);
                let before = state.clone();
                let mut text = 19;
                let mut list = 23;
                assert!(
                    execute(&command(setting, "Bool"), &mut state, &mut text, &mut list)
                        .unwrap_err()
                        .starts_with("AIL-RUNTIME-JSON-LIMIT:")
                );
                assert_eq!(state, before);
                assert_eq!((text, list), (19, 23));
            }
        }
        let mut state = BTreeMap::from([
            ("data.document".into(), "{}".into()),
            ("data.path".into(), format!("/{}", "a".repeat(1_048_576))),
        ]);
        assert!(
            execute(&command(true, "Bool"), &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        state.insert("data.path".into(), "/missing/child".into());
        state.insert("data.value".into(), "x".repeat(1_048_577));
        assert!(
            execute(&command(true, "Bool"), &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-PATH:")
        );
        state.insert("data.path".into(), "/valid".into());
        assert!(
            execute(&command(true, "Bool"), &mut state, &mut 0, &mut 0)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
    }

    #[test]
    fn runtime_aliases_are_immutable_and_only_final_json_uses_shared_list_budget() {
        for destination in ["data.document", "data.path", "data.value", "out.value"] {
            let source = "{\"x\":-0}";
            let mut state = BTreeMap::from([
                ("data.document".into(), source.into()),
                ("data.copy".into(), source.into()),
                ("data.path".into(), "/added".into()),
                ("data.value".into(), "🙂".into()),
            ]);
            let mut instruction = command(true, "Text");
            instruction
                .operands
                .insert("key".into(), destination.into());
            let expected = "{\"x\":-0,\"added\":\"🙂\"}";
            let mut text = 67_108_864;
            let mut list = 67_108_864 - expected.len();
            execute(&instruction, &mut state, &mut text, &mut list).unwrap();
            assert_eq!(state[destination], expected);
            assert_eq!(state["data.copy"], source);
            assert_eq!((text, list), (67_108_864, 67_108_864));
        }
        let mut state = BTreeMap::from([
            ("data.document".into(), "{\"a\":1}".into()),
            ("data.path".into(), "/a".into()),
            ("out.value".into(), "unchanged".into()),
        ]);
        let mut text = 5;
        let mut list = 67_108_863;
        let before = state.clone();
        assert!(
            execute(&command(false, ""), &mut state, &mut text, &mut list)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON-LIMIT:")
        );
        assert_eq!(state, before);
        assert_eq!((text, list), (5, 67_108_863));
        list = 67_108_862;
        execute(&command(false, ""), &mut state, &mut text, &mut list).unwrap();
        assert_eq!(state["out.value"], "{}");
        assert_eq!((text, list), (5, 67_108_864));
    }
    #[test]
    fn source_and_protocol_are_exact_and_imports_qualify_all_fields() {
        for (source, want) in [
            (
                "the system sets JSON value at path data.path in data.document to data.value into output.document",
                "the system sets JSON value at path pkg.data.path in pkg.data.document to pkg.data.value into pkg.output.document",
            ),
            (
                "the system removes JSON value at path data.path from data.document into output.document",
                "the system removes JSON value at path pkg.data.path from pkg.data.document into pkg.output.document",
            ),
        ] {
            let op = parse_bullet(source).unwrap().unwrap();
            assert_eq!(namespace(&op, "pkg").source(), want);
        }
        for source in [
            "the system sets JSON value",
            "the system removes JSON value at path invalid from data.document into output.document",
        ] {
            assert!(parse_bullet(source).unwrap().is_err());
        }
        assert!(verify(&command(true, "Json")).is_ok());
        assert!(verify(&command(false, "")).is_ok());
        let mut invalid = command(false, "");
        invalid.operands.insert("type".into(), "Int".into());
        assert!(verify(&invalid).is_err());
        assert!(verify(&command(true, "Decimal")).is_err());
        let mut invalid = command(true, "Text");
        invalid.operands.remove("value");
        assert!(verify(&invalid).is_err());
    }
    fn core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Json manages values.\n\nA Data has:\n- document: Json\n- path: Text\n- value: Int\n\nAn Output has:\n- document: Json\n\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Mutated\n";
        let mut document = parse_ail_package_spec_text(&package, source).unwrap();
        let action = document.actions.get_mut("Run").unwrap();
        action.json_mutations = vec![
            AilJsonMutation::Set {
                source: "data.document".into(),
                path: "data.path".into(),
                value: "data.value".into(),
                key: "output.document".into(),
            },
            AilJsonMutation::Remove {
                source: "data.document".into(),
                path: "data.path".into(),
                key: "output.document".into(),
            },
        ];
        action.steps.insert(0, AilActionStep::JsonMutation(1));
        action.steps.insert(0, AilActionStep::JsonMutation(0));
        elaborate_ail_core(&package, &document)
    }
    #[test]
    fn core_roundtrip_rejects_duplicate_edges_extra_attrs_and_remove_value_semantics() {
        let original = core_fixture();
        assert_eq!(
            original
                .graph
                .nodes
                .iter()
                .filter(|n| n.kind == "JsonMutation")
                .count(),
            2
        );
        let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        assert!(check(&saved).is_empty(), "{:?}", check(&saved));
        assert_eq!(
            from_core(
                &saved,
                saved
                    .graph
                    .nodes
                    .iter()
                    .find(|n| n.kind == "Action")
                    .unwrap()
            )
            .len(),
            2
        );
        for kind in [
            "json_mutation_step",
            "json_mutation_source",
            "json_mutation_destination",
            "json_mutation_path",
            "json_mutation_value",
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
        let mut invalid = original.clone();
        invalid
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "JsonMutation")
            .unwrap()
            .type_name = Some("Json".into());
        assert!(!check(&invalid).is_empty());
        let mut invalid = original.clone();
        invalid
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "JsonMutation" && n.attributes["json_mutation_kind"] == "remove")
            .unwrap()
            .attributes
            .insert("value_type".into(), "Int".into());
        assert!(!check(&invalid).is_empty());
        let mut invalid = original.clone();
        let remove = invalid
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "JsonMutation" && n.attributes["json_mutation_kind"] == "remove")
            .unwrap()
            .id
            .clone();
        invalid
            .graph
            .edges
            .iter_mut()
            .find(|e| e.kind == "json_mutation_value")
            .unwrap()
            .source = remove;
        assert!(!check(&invalid).is_empty());
    }
    #[test]
    fn core_enforces_global_types_owners_profiles_and_operand_secret_accesses() {
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
        for field_name in [
            "Data.document",
            "Data.path",
            "Data.value",
            "Output.document",
        ] {
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
        assert_eq!(accesses(&secret, "reads").len(), 5);
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
}
