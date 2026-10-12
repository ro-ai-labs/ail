//! Semantic ordering of state writes and action calls.
use super::*;

pub(super) fn steps(action: &AilAction) -> Vec<AilActionStep> {
    let mut result = action.steps.clone();
    for step in (0..action.calls.len())
        .map(AilActionStep::Call)
        .chain((0..action.repeated_calls.len()).map(AilActionStep::Repeat))
        .chain((0..action.writes.len()).map(AilActionStep::Write))
        .chain((0..action.controls.len()).map(AilActionStep::Control))
        .chain((0..action.integer_calls.len()).map(AilActionStep::IntegerCall))
        .chain((0..action.bytes_operations.len()).map(AilActionStep::Bytes))
        .chain((0..action.text_operations.len()).map(AilActionStep::Text))
        .chain((0..action.list_operations.len()).map(AilActionStep::List))
        .chain((0..action.literal_assignments.len()).map(AilActionStep::Literal))
        .chain((0..action.directory_operations.len()).map(AilActionStep::Directory))
        .chain((0..action.process_operations.len()).map(AilActionStep::Process))
        .chain((0..action.standard_stream_operations.len()).map(AilActionStep::StandardStream))
        .chain((0..action.file_operations.len()).map(AilActionStep::File))
        .chain((0..action.json_conversions.len()).map(AilActionStep::Json))
        .chain((0..action.json_accesses.len()).map(AilActionStep::JsonAccess))
        .chain((0..action.json_mutations.len()).map(AilActionStep::JsonMutation))
        .chain((0..action.float_operations.len()).map(AilActionStep::Float))
        .chain((0..action.value_operations.len()).map(AilActionStep::Variant))
        .chain((0..action.boolean_operations.len()).map(AilActionStep::Boolean))
    {
        if !result.contains(&step) {
            result.push(step);
        }
    }
    result
}
pub(super) fn append(
    graph: &mut Graph,
    kind: &str,
    source: &Node,
    target: &Node,
    mut attrs: BTreeMap<String, String>,
    index: usize,
) {
    attrs.insert("operation_index".into(), index.to_string());
    let mut edge = Edge::new(kind, source, target, attrs);
    edge.ordinal = graph.edges.len();
    graph.edges.push(edge);
}
pub(super) fn record_core_order(graph: &mut Graph, source: &Node, action: &AilAction) {
    let order = steps(action);
    for kind in [
        "writes",
        "calls",
        "repeats",
        "control_step",
        "integer_call_step",
        "bytes_step",
        "text_step",
        "list_step",
        "literal_step",
        "directory_step",
        "process_step",
        "stream_step",
        "file_step",
        "json_step",
        "json_access_step",
        "json_mutation_step",
        "float_step",
        "boolean_step",
        "variant_step",
    ] {
        for (index, edge) in graph
            .edges
            .iter_mut()
            .filter(|e| e.source == source.id && e.kind == kind)
            .enumerate()
        {
            let step = match kind {
                "writes" => AilActionStep::Write(index),
                "calls" => AilActionStep::Call(index),
                "repeats" => AilActionStep::Repeat(index),
                "control_step" => AilActionStep::Control(index),
                "bytes_step" => AilActionStep::Bytes(index),
                "text_step" => AilActionStep::Text(index),
                "list_step" => AilActionStep::List(index),
                "literal_step" => AilActionStep::Literal(index),
                "directory_step" => AilActionStep::Directory(index),
                "process_step" => AilActionStep::Process(index),
                "stream_step" => AilActionStep::StandardStream(index),
                "file_step" => AilActionStep::File(index),
                "json_step" => AilActionStep::Json(index),
                "json_access_step" => AilActionStep::JsonAccess(index),
                "json_mutation_step" => AilActionStep::JsonMutation(index),
                "float_step" => AilActionStep::Float(index),
                "boolean_step" => AilActionStep::Boolean(index),
                "variant_step" => AilActionStep::Variant(index),
                _ => AilActionStep::IntegerCall(index),
            };
            if let Some(position) = order.iter().position(|s| *s == step) {
                edge.attributes
                    .insert("step_index".into(), position.to_string());
                if let Some(target) = graph.nodes.iter().find(|n| n.id == edge.target) {
                    edge.id = Edge::new(&edge.kind, source, target, edge.attributes.clone()).id;
                }
            }
        }
    }
}
pub(super) fn from_core(core: &AilCore, source: &Node) -> Vec<AilActionStep> {
    let mut indexed = Vec::new();
    for kind in [
        "calls",
        "repeats",
        "writes",
        "control_step",
        "integer_call_step",
        "bytes_step",
        "text_step",
        "list_step",
        "literal_step",
        "directory_step",
        "process_step",
        "stream_step",
        "file_step",
        "json_step",
        "json_access_step",
        "json_mutation_step",
        "float_step",
        "boolean_step",
        "variant_step",
    ] {
        for (index, edge) in core
            .graph
            .edges
            .iter()
            .filter(|e| e.source == source.id && e.kind == kind)
            .enumerate()
        {
            let Some(position) = edge
                .attributes
                .get("step_index")
                .and_then(|s| s.parse::<usize>().ok())
            else {
                continue;
            };
            let step = match kind {
                "writes" => AilActionStep::Write(index),
                "calls" => AilActionStep::Call(index),
                "repeats" => AilActionStep::Repeat(index),
                "control_step" => AilActionStep::Control(index),
                "bytes_step" => AilActionStep::Bytes(index),
                "text_step" => AilActionStep::Text(index),
                "list_step" => AilActionStep::List(index),
                "literal_step" => AilActionStep::Literal(index),
                "directory_step" => AilActionStep::Directory(index),
                "process_step" => AilActionStep::Process(index),
                "stream_step" => AilActionStep::StandardStream(index),
                "file_step" => AilActionStep::File(index),
                "json_step" => AilActionStep::Json(index),
                "json_access_step" => AilActionStep::JsonAccess(index),
                "json_mutation_step" => AilActionStep::JsonMutation(index),
                "float_step" => AilActionStep::Float(index),
                "boolean_step" => AilActionStep::Boolean(index),
                "variant_step" => AilActionStep::Variant(index),
                _ => AilActionStep::IntegerCall(index),
            };
            indexed.push((position, step));
        }
    }
    indexed.sort_by_key(|(position, _)| *position);
    indexed.into_iter().map(|(_, step)| step).collect()
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut errors = Vec::new();
    for action in core.graph.nodes.iter().filter(|n| n.kind == "Action") {
        let edges = core
            .graph
            .edges
            .iter()
            .filter(|e| {
                e.source == action.id
                    && matches!(
                        e.kind.as_str(),
                        "writes"
                            | "calls"
                            | "repeats"
                            | "control_step"
                            | "integer_call_step"
                            | "bytes_step"
                            | "text_step"
                            | "list_step"
                            | "literal_step"
                            | "directory_step"
                            | "process_step"
                            | "stream_step"
                            | "file_step"
                            | "json_step"
                            | "json_access_step"
                            | "json_mutation_step"
                            | "float_step"
                            | "boolean_step"
                            | "variant_step"
                    )
            })
            .collect::<Vec<_>>();
        for edge in &edges {
            let target = core.graph.nodes.iter().find(|n| n.id == edge.target);
            let invalid = edge.attributes.contains_key("inferred_expression")
                || (edge.kind == "repeats"
                    && edge
                        .attributes
                        .get("count")
                        .and_then(|s| s.parse::<usize>().ok())
                        .is_none_or(|count| count == 0))
                || (matches!(edge.kind.as_str(), "calls" | "repeats")
                    && target.is_none_or(|n| n.kind != "Action"))
                || (edge.kind == "control_step" && target.is_none_or(|n| n.kind != "Control"));
            let invalid = invalid
                || (edge.kind == "variant_step"
                    && target.is_none_or(|n| n.kind != "VariantOperation"))
                || (edge.kind == "boolean_step"
                    && target.is_none_or(|n| n.kind != "BooleanOperation"))
                || (edge.kind == "float_step" && target.is_none_or(|n| n.kind != "FloatOperation"))
                || (edge.kind == "list_step" && target.is_none_or(|n| n.kind != "ListOperation"))
                || (edge.kind == "json_mutation_step"
                    && target.is_none_or(|n| n.kind != "JsonMutation"))
                || (edge.kind == "json_access_step"
                    && target.is_none_or(|n| n.kind != "JsonAccess"))
                || (edge.kind == "json_step" && target.is_none_or(|n| n.kind != "JsonConversion"))
                || (edge.kind == "directory_step"
                    && target.is_none_or(|n| n.kind != "DirectoryOperation"))
                || (edge.kind == "process_step"
                    && target.is_none_or(|n| n.kind != "ProcessOperation"))
                || (edge.kind == "stream_step"
                    && target.is_none_or(|n| n.kind != "StandardStreamOperation"))
                || (edge.kind == "file_step" && target.is_none_or(|n| n.kind != "FileOperation"))
                || (edge.kind == "literal_step"
                    && target.is_none_or(|n| n.kind != "LiteralAssignment"))
                || (edge.kind == "bytes_step" && target.is_none_or(|n| n.kind != "BytesOperation"))
                || (edge.kind == "text_step" && target.is_none_or(|n| n.kind != "TextOperation"))
                || (edge.kind == "integer_call_step"
                    && target.is_none_or(|n| {
                        !matches!(n.kind.as_str(), "ValueCall" | "ValueFunctionCall")
                    }));
            if invalid {
                errors.push(
                    AilDiagnostic::error(
                        "AIL-EXPR-002",
                        format!(
                            "invalid {} operation edge on action {}",
                            edge.kind, action.name
                        ),
                    )
                    .with_affected_graph_item(format!("edge:{}", edge.id)),
                );
            }
        }
        if edges
            .iter()
            .all(|e| !e.attributes.contains_key("step_index"))
        {
            continue;
        }
        let indices = edges
            .iter()
            .filter_map(|e| {
                e.attributes
                    .get("step_index")
                    .and_then(|s| s.parse::<usize>().ok())
            })
            .collect::<BTreeSet<_>>();
        if indices != (0..edges.len()).collect() {
            errors.push(AilDiagnostic::error("AIL-EXPR-002",format!("action {} requires unique contiguous step_index values on writes, calls, repeats, and control steps",action.name)));
        }
    }
    errors
}
