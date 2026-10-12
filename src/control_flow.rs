//! Structured action control flow, represented and checked as typed Core nodes.
use super::*;
use crate::bool_expression::BoolExpr;
use crate::condition::{IntComparison, IntCondition};
use crate::expression::IntExpr;
use crate::float_expression::FloatCondition;

const MAX_CONTROL_DEPTH: usize = 64;
const MAX_CONTROL_STEPS: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AilControl {
    If(IntCondition),
    IfFloat(FloatCondition),
    IfBoolean(BoolExpr),
    Else,
    EndIf,
    While(IntCondition),
    WhileFloat(FloatCondition),
    WhileBoolean(BoolExpr),
    EndWhile,
    Break,
    Continue,
    Try,
    Recover {
        errors: String,
        key: String,
    },
    EndTry,
    Raise {
        category: String,
    },
    /// Retains malformed graph positions for safe inspection before checking.
    Invalid(String),
}
impl AilControl {
    fn kind(&self) -> &'static str {
        match self {
            Self::If(_) => "if",
            Self::IfFloat(_) => "if_float",
            Self::IfBoolean(_) => "if_boolean",
            Self::Else => "else",
            Self::EndIf => "end_if",
            Self::While(_) => "while",
            Self::WhileFloat(_) => "while_float",
            Self::WhileBoolean(_) => "while_boolean",
            Self::EndWhile => "end_while",
            Self::Break => "break",
            Self::Continue => "continue",
            Self::Try => "try",
            Self::Recover { .. } => "recover",
            Self::EndTry => "end_try",
            Self::Raise { .. } => "raise",
            Self::Invalid(_) => "invalid",
        }
    }
    pub(super) fn condition(&self) -> Option<&IntCondition> {
        match self {
            Self::If(c) | Self::While(c) => Some(c),
            _ => None,
        }
    }
    pub(super) fn float_condition(&self) -> Option<&FloatCondition> {
        match self {
            Self::IfFloat(c) | Self::WhileFloat(c) => Some(c),
            _ => None,
        }
    }
    pub(super) fn boolean_condition(&self) -> Option<&BoolExpr> {
        match self {
            Self::IfBoolean(c) | Self::WhileBoolean(c) => Some(c),
            _ => None,
        }
    }
    pub(super) fn source(&self) -> String {
        match self {
            Self::If(c) => format!("if {}:", c.to_source()),
            Self::IfFloat(c) => format!("if floating {}:", c.to_source()),
            Self::IfBoolean(c) => format!("if boolean {}:", c.to_source()),
            Self::While(c) => format!("while {}:", c.to_source()),
            Self::WhileFloat(c) => format!("while floating {}:", c.to_source()),
            Self::WhileBoolean(c) => format!("while boolean {}:", c.to_source()),
            Self::Else => "otherwise:".into(),
            Self::EndIf => "end if".into(),
            Self::EndWhile => "end while".into(),
            Self::Break => "break".into(),
            Self::Continue => "continue".into(),
            Self::Try => "try:".into(),
            Self::Recover { errors, key } => {
                format!("recover {} into {key}:", errors.replace(';', " or "))
            }
            Self::EndTry => "end try".into(),
            Self::Raise { category } => format!("the system raises error {category}"),
            Self::Invalid(error) => format!("invalid control: {error}"),
        }
    }
}
fn error(message: impl Display) -> String {
    format!("AIL-CONTROL-001: {message}")
}
pub(super) fn parse_bullet(bullet: &str) -> Option<Result<AilControl, String>> {
    let text = trim_sentence(bullet);
    if raise_bullet(&text) {
        return Some((|| {
            let category = text
                .strip_prefix("the system raises error ")
                .or_else(|| text.strip_prefix("the function raises error "))
                .ok_or_else(|| error("expected raises error CATEGORY"))?;
            recovery::validate_raise(category).map_err(error)?;
            Ok(AilControl::Raise {
                category: category.into(),
            })
        })());
    }
    let control = match text.as_str() {
        "otherwise:" => AilControl::Else,
        "end if" => AilControl::EndIf,
        "end while" => AilControl::EndWhile,
        "break" => AilControl::Break,
        "continue" => AilControl::Continue,
        "try:" => AilControl::Try,
        "end try" => AilControl::EndTry,
        _ if recovery_bullet(&text) => {
            return Some((|| {
                let rest = text
                    .strip_prefix("recover ")
                    .and_then(|s| s.strip_suffix(':'))
                    .ok_or_else(|| error("invalid recovery delimiter"))?;
                let (errors, key) = rest
                    .rsplit_once(" into ")
                    .ok_or_else(|| error("recovery requires categories and a destination"))?;
                let errors = recovery::source_errors(errors).map_err(error)?;
                if !matches!(IntExpr::parse(key), Ok(IntExpr::Field(parsed)) if parsed == key) {
                    return Err(error("recovery destination must be a field reference"));
                }
                Ok(AilControl::Recover {
                    errors,
                    key: key.into(),
                })
            })());
        }
        _ => {
            let (condition, is_loop) = if let Some(c) = text.strip_prefix("while ") {
                (c, true)
            } else if let Some(c) = text.strip_prefix("if ") {
                // Preserve historical failure clauses; numeric comparisons are executable.
                if c != "boolean"
                    && !c.starts_with("boolean ")
                    && !c.ends_with(':')
                    && !c.contains(['<', '>', '=', '!'])
                {
                    return None;
                }
                (c, false)
            } else if text.starts_with("otherwise")
                || text.starts_with("end if")
                || text.starts_with("end while")
            {
                return Some(Err(error("invalid control delimiter")));
            } else {
                return None;
            };
            let Some(condition) = condition.strip_suffix(':') else {
                return Some(Err(error("if and while headers must end with ':'")));
            };
            if let Some(condition) = condition.strip_prefix("boolean ") {
                return Some(BoolExpr::parse(condition).map(|c| {
                    if is_loop {
                        AilControl::WhileBoolean(c)
                    } else {
                        AilControl::IfBoolean(c)
                    }
                }));
            }
            if let Some(condition) = condition.strip_prefix("floating ") {
                return Some(
                    FloatCondition::parse(condition)
                        .map(|c| {
                            if is_loop {
                                AilControl::WhileFloat(c)
                            } else {
                                AilControl::IfFloat(c)
                            }
                        })
                        .map_err(error),
                );
            }
            return Some(
                IntCondition::parse(condition)
                    .map(|c| {
                        if is_loop {
                            AilControl::While(c)
                        } else {
                            AilControl::If(c)
                        }
                    })
                    .map_err(error),
            );
        }
    };
    Some(Ok(control))
}
pub(super) fn raise_bullet(bullet: &str) -> bool {
    let text = trim_sentence(bullet);
    text.strip_prefix("the system ")
        .or_else(|| text.strip_prefix("the function "))
        .and_then(|s| s.strip_prefix("raises"))
        .is_some_and(|tail| {
            tail.is_empty()
                || tail.starts_with(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, '_' | '.'))
        })
}
pub(super) fn recovery_bullet(bullet: &str) -> bool {
    let text = trim_sentence(bullet);
    ["try", "recover", "end try"].iter().any(|prefix| {
        text.strip_prefix(prefix).is_some_and(|tail| {
            tail.is_empty()
                || tail
                    .chars()
                    .next()
                    .is_some_and(|c| !c.is_ascii_alphanumeric() && !matches!(c, '_' | '.'))
        })
    })
}

pub(super) fn inside_block(action: &AilAction) -> bool {
    action.controls.iter().fold(0usize, |depth, c| match c {
        AilControl::If(_)
        | AilControl::While(_)
        | AilControl::IfFloat(_)
        | AilControl::WhileFloat(_)
        | AilControl::IfBoolean(_)
        | AilControl::WhileBoolean(_)
        | AilControl::Try => depth + 1,
        AilControl::EndIf | AilControl::EndWhile | AilControl::EndTry => depth.saturating_sub(1),
        _ => depth,
    }) > 0
}
pub(super) fn executable_bullet(bullet: &str) -> bool {
    [
        "the system computes ",
        "the system changes ",
        "Changes ",
        "the system increments ",
        "the system decrements ",
        "the system deletes ",
        "the system removes ",
        "the system creates ",
        "the system calls ",
        "the system repeats ",
    ]
    .iter()
    .any(|prefix| bullet.starts_with(prefix))
}
pub(super) fn elaborate(graph: &mut Graph, action: &Node, controls: &[AilControl]) {
    for (index, control) in controls.iter().enumerate() {
        let name = format!("{}.control{index}", action.name);
        let mut attributes = attr(&[("control_kind", control.kind())]);
        if let AilControl::Recover { errors, .. } = control {
            attributes.insert("recovery_errors".into(), errors.clone());
        }
        if let AilControl::Raise { category } = control {
            attributes.insert("error_category".into(), category.clone());
        }
        let node = graph.add_node("Control", &name, None, attributes);
        attach_provenance(
            graph,
            &node,
            format!("action:{}.control:{index}", action.name),
        );
        if let Some(condition) = control.condition() {
            let comparison = graph.add_node(
                "IntegerComparison",
                format!("{name}.condition"),
                Some("Bool".into()),
                attr(&[("comparison", condition.comparison.token())]),
            );
            attach_provenance(
                graph,
                &comparison,
                format!("action:{}.condition:{index}", action.name),
            );
            let left =
                expression_core::build(graph, action, &format!("{name}.left"), &condition.left);
            let right =
                expression_core::build(graph, action, &format!("{name}.right"), &condition.right);
            graph.add_edge("comparison_left", &comparison, &left, BTreeMap::new());
            graph.add_edge("comparison_right", &comparison, &right, BTreeMap::new());
            graph.add_edge("condition", &node, &comparison, BTreeMap::new());
        }
        if let Some(condition) = control.float_condition() {
            let comparison = float_operations::build_condition(
                graph,
                action,
                &format!("{name}.condition"),
                condition,
                false,
            );
            graph.add_edge("condition", &node, &comparison, BTreeMap::new());
        }
        if let Some(condition) = control.boolean_condition() {
            let root = bool_operations::build_expression(
                graph,
                action,
                &format!("{name}.condition"),
                condition,
                false,
            );
            graph.add_edge("condition", &node, &root, BTreeMap::new());
        }
        if let AilControl::Recover { key, .. } = control {
            let destination = graph
                .nodes
                .iter()
                .find(|field| {
                    field.kind == "Field"
                        && expression_core::field_key(field) == *key
                        && !field.attributes.contains_key("function_owner")
                })
                .cloned()
                .unwrap_or_else(|| graph.add_node("Effect", key, None, BTreeMap::new()));
            graph.add_edge("recovery_destination", &node, &destination, BTreeMap::new());
        }
        action_order::append(graph, "control_step", action, &node, BTreeMap::new(), index);
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
    if edges.next().is_some() {
        return Err(error(format!("{} has duplicate {kind} edges", node.name)));
    }
    core.graph
        .nodes
        .iter()
        .find(|n| n.id == edge.target)
        .ok_or_else(|| error("missing control operand"))
}
fn decode_recovery(core: &AilCore, node: &Node, kind: &str) -> Result<AilControl, String> {
    if !matches!(core.package.profile.as_str(), "Application" | "C interop") {
        return Err(error("recovery requires Application or C interop profile"));
    }
    let attrs: &[&str] = if kind == "recover" {
        &["control_kind", "recovery_errors"]
    } else {
        &["control_kind"]
    };
    if node.type_name.is_some()
        || node.attributes.len() != attrs.len()
        || attrs.iter().any(|key| !node.attributes.contains_key(*key))
    {
        return Err(error("unexpected recovery control metadata"));
    }
    let owners: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|edge| edge.target == node.id)
        .collect();
    if owners.len() != 1
        || owners[0].kind != "control_step"
        || !owners[0].attributes.contains_key("step_index")
        || owners[0].attributes.iter().any(|(key, value)| {
            !matches!(key.as_str(), "step_index" | "operation_index")
                || value.parse::<usize>().is_err()
        })
        || !core.graph.nodes.iter().any(|owner| {
            owner.id == owners[0].source
                && owner.kind == "Action"
                && owner
                    .attributes
                    .get("kind")
                    .is_none_or(|kind| kind != "CompilerPass")
        })
    {
        return Err(error(
            "recovery control requires one ordered executable action owner",
        ));
    }
    for edge in core.graph.edges.iter().filter(|e| e.source == node.id) {
        if edge.kind != "has_provenance"
            && !(kind == "recover"
                && edge.kind == "recovery_destination"
                && edge.attributes.is_empty())
        {
            return Err(error("unexpected recovery control edge or metadata"));
        }
    }
    Ok(match kind {
        "try" => AilControl::Try,
        "end_try" => AilControl::EndTry,
        "recover" => {
            let errors = &node.attributes["recovery_errors"];
            recovery::mask(errors).map_err(error)?;
            let field = child(core, node, "recovery_destination")?;
            let key = float_operations::scoped_field(core, field, None, "Text").map_err(error)?;
            AilControl::Recover {
                errors: errors.clone(),
                key,
            }
        }
        _ => return Err(error("invalid recovery control")),
    })
}

/// Raise carries no data operands; validate all participating identities before
/// decoding it, including provenance and the complete incoming ownership set.
pub(super) fn validate_raise_node(
    core: &AilCore,
    node: &Node,
    owner: Option<&Node>,
) -> Result<String, String> {
    fn unique<'a>(core: &'a AilCore, id: &str) -> Result<&'a Node, String> {
        let mut nodes = core.graph.nodes.iter().filter(|n| n.id == id);
        let node = nodes
            .next()
            .ok_or_else(|| error("missing raise participant"))?;
        if nodes.next().is_some() {
            return Err(error("raise participants require unique identities"));
        }
        Ok(node)
    }
    fn participant(core: &AilCore, node: &Node) -> Result<(), String> {
        unique(core, &node.id)?;
        let mut seen = BTreeSet::new();
        for edge in core
            .graph
            .edges
            .iter()
            .filter(|e| e.source == node.id && e.kind == "has_provenance")
        {
            let provenance = unique(core, &edge.target)?;
            if !edge.attributes.is_empty()
                || !seen.insert(&edge.target)
                || provenance.kind != "Provenance"
                || provenance.type_name.is_some()
            {
                return Err(error("invalid raise participant provenance"));
            }
        }
        Ok(())
    }
    if !matches!(core.package.profile.as_str(), "Application" | "C interop") {
        return Err(error("raise requires Application or C interop profile"));
    }
    participant(core, node)?;
    let attrs: &[&str] = if owner.is_some() {
        &[
            "function_owner",
            "step_kind",
            "control_kind",
            "error_category",
        ]
    } else {
        &["control_kind", "error_category"]
    };
    if node.type_name.is_some()
        || node.attributes.len() != attrs.len()
        || attrs.iter().any(|key| !node.attributes.contains_key(*key))
        || node.attributes["control_kind"] != "raise"
    {
        return Err(error("unexpected raise control metadata"));
    }
    let incoming: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect();
    if incoming.len() != 1 {
        return Err(error("raise requires exactly one ordered owner"));
    }
    let edge = incoming[0];
    let actual_owner = unique(core, &edge.source)?;
    participant(core, actual_owner)?;
    let relation = match owner {
        None if node.kind == "Control"
            && actual_owner.kind == "Action"
            && actual_owner
                .attributes
                .get("kind")
                .is_none_or(|k| k != "CompilerPass") =>
        {
            "control_step"
        }
        Some(owner)
            if actual_owner.id == owner.id
                && owner.kind == "Function"
                && node.attributes.get("function_owner") == Some(&owner.name)
                && node.attributes["step_kind"] == "control" =>
        {
            match node.kind.as_str() {
                "FunctionStep"
                    if owner
                        .attributes
                        .get("executable_integer")
                        .is_some_and(|v| v == "true") =>
                {
                    "function_step"
                }
                "ValueFunctionStep"
                    if owner
                        .attributes
                        .get("executable_value")
                        .is_some_and(|v| v == "true") =>
                {
                    "value_function_step"
                }
                _ => return Err(error("raise requires executable function ownership")),
            }
        }
        _ => return Err(error("invalid raise owner")),
    };
    if edge.kind != relation
        || !edge.attributes.contains_key("step_index")
        || edge.attributes.iter().any(|(k, v)| {
            !(k == "step_index" || relation == "control_step" && k == "operation_index")
                || v.parse::<usize>().is_err()
        })
    {
        return Err(error("invalid ordered raise owner edge"));
    }
    if core
        .graph
        .edges
        .iter()
        .any(|e| e.source == node.id && e.kind != "has_provenance")
    {
        return Err(error("raise cannot have operand edges"));
    }
    let category = &node.attributes["error_category"];
    recovery::validate_raise(category).map_err(error)?;
    Ok(category.clone())
}

fn decode(core: &AilCore, node: &Node) -> Result<AilControl, String> {
    if node.kind != "Control" {
        return Err(error("control_step must target a Control node"));
    }
    let kind = node
        .attributes
        .get("control_kind")
        .map(String::as_str)
        .unwrap_or("");
    if kind == "raise" {
        let category = validate_raise_node(core, node, None)?;
        return Ok(AilControl::Raise { category });
    }
    if matches!(kind, "try" | "recover" | "end_try") {
        return decode_recovery(core, node, kind);
    }
    if !matches!(
        kind,
        "if" | "while" | "if_float" | "while_float" | "if_boolean" | "while_boolean"
    ) && core
        .graph
        .edges
        .iter()
        .any(|e| e.source == node.id && e.kind == "condition")
    {
        return Err(error("delimiter must not have a condition"));
    }
    Ok(match kind {
        "if" | "while" => {
            let comparison = child(core, node, "condition")?;
            let condition = decode_condition(core, comparison)?;
            if kind == "if" {
                AilControl::If(condition)
            } else {
                AilControl::While(condition)
            }
        }
        "if_float" | "while_float" => {
            if node.type_name.is_some() || node.attributes.len() != 1 {
                return Err(error(
                    "floating control requires only its control_kind attribute",
                ));
            }
            let edges: Vec<_> = core
                .graph
                .edges
                .iter()
                .filter(|e| e.source == node.id && e.kind == "condition")
                .collect();
            if edges.len() != 1 || !edges[0].attributes.is_empty() {
                return Err(error(
                    "floating control requires one unannotated condition edge",
                ));
            }
            let condition =
                float_operations::decode_condition(core, child(core, node, "condition")?, None)?;
            if kind == "if_float" {
                AilControl::IfFloat(condition)
            } else {
                AilControl::WhileFloat(condition)
            }
        }
        "if_boolean" | "while_boolean" => {
            let condition =
                bool_operations::decode_expression(core, child(core, node, "condition")?, None)?;
            if kind == "if_boolean" {
                AilControl::IfBoolean(condition)
            } else {
                AilControl::WhileBoolean(condition)
            }
        }
        "else" => AilControl::Else,
        "end_if" => AilControl::EndIf,
        "end_while" => AilControl::EndWhile,
        "break" => AilControl::Break,
        "continue" => AilControl::Continue,
        _ => return Err(error("unknown control kind")),
    })
}
fn decode_condition(core: &AilCore, comparison: &Node) -> Result<IntCondition, String> {
    if comparison.kind != "IntegerComparison" || comparison.type_name.as_deref() != Some("Bool") {
        return Err(error("condition must be an IntegerComparison of type Bool"));
    }
    let left = expression_core::decode_root(core, &child(core, comparison, "comparison_left")?.id)?;
    let right =
        expression_core::decode_root(core, &child(core, comparison, "comparison_right")?.id)?;
    let op = IntComparison::from_token(
        comparison
            .attributes
            .get("comparison")
            .ok_or_else(|| error("missing comparison operator"))?,
    )?;
    IntCondition::from_postfix(&left.to_postfix(), op.token(), &right.to_postfix())
}
pub(super) fn from_core(core: &AilCore, action: &Node) -> Vec<AilControl> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == action.id && e.kind == "control_step")
        .map(|e| {
            core.graph
                .nodes
                .iter()
                .find(|n| n.id == e.target)
                .ok_or_else(|| error("missing control node"))
                .and_then(|n| decode(core, n))
                .unwrap_or_else(AilControl::Invalid)
        })
        .collect()
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    for edge in &core.graph.edges {
        let source = core.graph.nodes.iter().find(|n| n.id == edge.source);
        let target = core.graph.nodes.iter().find(|n| n.id == edge.target);
        let valid = match edge.kind.as_str() {
            "control_step" => {
                source.is_some_and(|n| {
                    n.kind == "Action"
                        && n.attributes
                            .get("kind")
                            .is_none_or(|kind| kind != "CompilerPass")
                }) && target.is_some_and(|n| n.kind == "Control")
                    && edge.attributes.contains_key("step_index")
            }
            "recovery_destination" => {
                source.is_some_and(|n| {
                    n.kind == "Control"
                        && n.attributes
                            .get("control_kind")
                            .is_some_and(|kind| kind == "recover")
                }) && target
                    .is_some_and(|n| n.kind == "Field" && n.type_name.as_deref() == Some("Text"))
                    && edge.attributes.is_empty()
            }
            "condition" => {
                source.is_some_and(|n| {
                    n.kind == "Control"
                        && n.attributes.get("control_kind").is_some_and(|s| {
                            matches!(
                                s.as_str(),
                                "if" | "while"
                                    | "if_float"
                                    | "while_float"
                                    | "if_boolean"
                                    | "while_boolean"
                            )
                        })
                }) && target.is_some_and(|n| {
                    matches!(
                        n.kind.as_str(),
                        "IntegerComparison" | "FloatComparison" | "BooleanExpression"
                    )
                })
            }
            "comparison_left" | "comparison_right" => {
                source.is_some_and(|n| n.kind == "IntegerComparison")
                    && target.is_some_and(|n| n.kind == "IntegerExpression")
                    || source.is_some_and(|n| n.kind == "FloatComparison")
                        && target.is_some_and(|n| n.kind == "FloatExpression")
            }
            _ => true,
        };
        let permitted = match source.map(|n| n.kind.as_str()) {
            Some("Control") => matches!(
                edge.kind.as_str(),
                "condition" | "recovery_destination" | "has_provenance"
            ),
            Some("IntegerComparison" | "FloatComparison") => matches!(
                edge.kind.as_str(),
                "comparison_left" | "comparison_right" | "has_provenance"
            ),
            _ => true,
        };
        if !valid || !permitted {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-CONTROL-001",
                    format!(
                        "invalid {} control-flow edge or missing step_index",
                        edge.kind
                    ),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    for node in &core.graph.nodes {
        let result = match node.kind.as_str() {
            "Control" => decode(core, node).map(|_| ()),
            "IntegerComparison" => decode_condition(core, node).map(|_| ()),
            _ => Ok(()),
        };
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-CONTROL-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for action in core.graph.nodes.iter().filter(|n| n.kind == "Action") {
        let controls = from_core(core, action);
        let mut emitter = Emitter::default();
        let mut instructions = Vec::new();
        for step in action_order::from_core(core, action) {
            if let AilActionStep::Control(index) = step {
                let result = controls
                    .get(index)
                    .ok_or_else(|| error("invalid control index"))
                    .and_then(|c| emitter.emit(c, &mut instructions));
                if let Err(message) = result {
                    diagnostics.push(
                        AilDiagnostic::error("AIL-CONTROL-001", message)
                            .with_affected_graph_item(format!("node:{}", action.id)),
                    );
                    break;
                }
            }
        }
        if let Err(message) = emitter.finish() {
            diagnostics.push(
                AilDiagnostic::error("AIL-CONTROL-001", message)
                    .with_affected_graph_item(format!("node:{}", action.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    if kind != "writes" {
        return Vec::new();
    }
    core.graph
        .edges
        .iter()
        .filter(|step| step.kind == "control_step")
        .flat_map(|step| {
            core.graph.edges.iter().filter_map(move |edge| {
                (edge.source == step.target && edge.kind == "recovery_destination")
                    .then_some((edge, step.source.as_str()))
            })
        })
        .collect()
}

pub(super) fn namespace(control: &AilControl, alias: &str) -> AilControl {
    if let AilControl::Recover { errors, key } = control {
        return AilControl::Recover {
            errors: errors.clone(),
            key: format!("{}.{key}", runtime_subject_key(alias)),
        };
    }
    if let Some(condition) = control.boolean_condition() {
        let prefix = runtime_subject_key(alias);
        let condition = condition.map_fields(|key| format!("{prefix}.{key}"));
        return if matches!(control, AilControl::IfBoolean(_)) {
            AilControl::IfBoolean(condition)
        } else {
            AilControl::WhileBoolean(condition)
        };
    }
    if let Some(condition) = control.float_condition() {
        let prefix = runtime_subject_key(alias);
        let condition = FloatCondition {
            left: condition.left.map_fields(|k| format!("{prefix}.{k}")),
            comparison: condition.comparison,
            right: condition.right.map_fields(|k| format!("{prefix}.{k}")),
        };
        return if matches!(control, AilControl::IfFloat(_)) {
            AilControl::IfFloat(condition)
        } else {
            AilControl::WhileFloat(condition)
        };
    }
    let Some(condition) = control.condition() else {
        return control.clone();
    };
    let prefix = runtime_subject_key(alias);
    let qualify = |expression: &crate::expression::IntExpr| {
        expression
            .to_postfix()
            .split_whitespace()
            .map(|token| {
                token
                    .strip_prefix("f:")
                    .map(|key| format!("f:{prefix}.{key}"))
                    .unwrap_or_else(|| token.to_string())
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    match IntCondition::from_postfix(
        &qualify(&condition.left),
        condition.comparison.token(),
        &qualify(&condition.right),
    ) {
        Ok(condition) => {
            if matches!(control, AilControl::If(_)) {
                AilControl::If(condition)
            } else {
                AilControl::While(condition)
            }
        }
        Err(error) => AilControl::Invalid(error),
    }
}
#[derive(Default)]
pub(super) struct Emitter {
    frames: Vec<Frame>,
    next: usize,
    controls: usize,
}
enum Frame {
    Try {
        begin: usize,
        has_recover: bool,
    },
    If {
        otherwise: String,
        end: String,
        has_else: bool,
    },
    While {
        head: String,
        end: String,
    },
}
fn label(out: &mut Vec<AilBytecodeInstruction>, name: &str) {
    out.push(AilBytecodeInstruction::new(
        "LABEL",
        &[("name", name.to_string())],
    ));
}
fn jump(out: &mut Vec<AilBytecodeInstruction>, name: &str) {
    out.push(AilBytecodeInstruction::new(
        "JUMP",
        &[("label", name.to_string())],
    ));
}
fn branch(out: &mut Vec<AilBytecodeInstruction>, control: &AilControl, label: &str) {
    if let Some(condition) = control.boolean_condition() {
        out.push(AilBytecodeInstruction::new(
            "BRANCH_BOOL",
            &[
                ("expression", condition.to_postfix()),
                ("label", label.into()),
            ],
        ));
        return;
    }
    if let Some(condition) = control.float_condition() {
        out.push(AilBytecodeInstruction::new(
            "BRANCH_FLOAT",
            &[
                ("left", condition.left.to_postfix()),
                ("comparison", condition.comparison.token().into()),
                ("right", condition.right.to_postfix()),
                ("label", label.into()),
            ],
        ));
        return;
    }
    let condition = control.condition().expect("branch control has a condition");
    out.push(AilBytecodeInstruction::new(
        "BRANCH_INT",
        &[
            ("left", condition.left.to_postfix()),
            ("comparison", condition.comparison.token().into()),
            ("right", condition.right.to_postfix()),
            ("label", label.into()),
        ],
    ));
}
impl Emitter {
    pub(super) fn emit(
        &mut self,
        control: &AilControl,
        out: &mut Vec<AilBytecodeInstruction>,
    ) -> Result<(), String> {
        self.controls += 1;
        if self.controls > MAX_CONTROL_STEPS {
            return Err(error("too many control statements"));
        }
        match control {
            AilControl::If(_)
            | AilControl::While(_)
            | AilControl::IfFloat(_)
            | AilControl::WhileFloat(_)
            | AilControl::IfBoolean(_)
            | AilControl::WhileBoolean(_) => {
                if self.frames.len() >= MAX_CONTROL_DEPTH {
                    return Err(error("control nesting exceeds 64 blocks"));
                }
                let id = self.next;
                self.next += 1;
                let body = format!("__control_{id}_body");
                let end = format!("__control_{id}_end");
                if matches!(
                    control,
                    AilControl::If(_) | AilControl::IfFloat(_) | AilControl::IfBoolean(_)
                ) {
                    let otherwise = format!("__control_{id}_otherwise");
                    branch(out, control, &body);
                    jump(out, &otherwise);
                    label(out, &body);
                    self.frames.push(Frame::If {
                        otherwise,
                        end,
                        has_else: false,
                    });
                } else {
                    let head = format!("__control_{id}_head");
                    label(out, &head);
                    branch(out, control, &body);
                    jump(out, &end);
                    label(out, &body);
                    self.frames.push(Frame::While { head, end });
                }
            }
            AilControl::Raise { category } => {
                recovery::validate_raise(category).map_err(error)?;
                out.push(AilBytecodeInstruction::new(
                    "RAISE",
                    &[("category", category.clone())],
                ));
            }
            AilControl::Try => {
                if self.frames.len() >= MAX_CONTROL_DEPTH {
                    return Err(error("control nesting exceeds 64 blocks"));
                }
                self.frames.push(Frame::Try {
                    begin: out.len(),
                    has_recover: false,
                });
                out.push(AilBytecodeInstruction::new("TRY_BEGIN", &[]));
            }
            AilControl::Recover { errors, key } => {
                recovery::mask(errors).map_err(error)?;
                if !matches!(IntExpr::parse(key), Ok(IntExpr::Field(parsed)) if parsed == *key) {
                    return Err(error("recovery destination must be a field reference"));
                }
                let Some(Frame::Try { begin, has_recover }) = self.frames.last_mut() else {
                    return Err(error("recover requires an open try block"));
                };
                if *has_recover {
                    return Err(error("try block must have exactly one recover clause"));
                }
                let instruction = out
                    .get_mut(*begin)
                    .filter(|i| i.opcode == "TRY_BEGIN")
                    .ok_or_else(|| error("missing try instruction while lowering recovery"))?;
                instruction.operands = BTreeMap::from([
                    ("errors".into(), errors.clone()),
                    ("key".into(), key.clone()),
                ]);
                *has_recover = true;
                out.push(AilBytecodeInstruction::new("RECOVER", &[]));
            }
            AilControl::EndTry => match self.frames.last() {
                Some(Frame::Try {
                    has_recover: true, ..
                }) => {
                    self.frames.pop();
                    out.push(AilBytecodeInstruction::new("TRY_END", &[]));
                }
                _ => return Err(error("end try requires a matching recovery clause")),
            },
            AilControl::Else => match self.frames.last_mut() {
                Some(Frame::If {
                    otherwise,
                    end,
                    has_else,
                }) if !*has_else => {
                    jump(out, end);
                    label(out, otherwise);
                    *has_else = true;
                }
                _ => return Err(error("otherwise requires an unmatched if block")),
            },
            AilControl::EndIf => match self.frames.pop() {
                Some(Frame::If {
                    otherwise,
                    end,
                    has_else,
                }) => label(out, if has_else { &end } else { &otherwise }),
                _ => return Err(error("end if does not match the open block")),
            },
            AilControl::EndWhile => match self.frames.pop() {
                Some(Frame::While { head, end }) => {
                    jump(out, &head);
                    label(out, &end);
                }
                _ => return Err(error("end while does not match the open block")),
            },
            AilControl::Break | AilControl::Continue => {
                let Some(Frame::While { head, end }) = self
                    .frames
                    .iter()
                    .rev()
                    .find(|f| matches!(f, Frame::While { .. }))
                else {
                    return Err(error("break and continue require an enclosing while block"));
                };
                jump(
                    out,
                    if matches!(control, AilControl::Break) {
                        end
                    } else {
                        head
                    },
                );
            }
            AilControl::Invalid(message) => return Err(error(message)),
        }
        Ok(())
    }
    pub(super) fn finish(&self) -> Result<(), String> {
        if self.frames.is_empty() {
            Ok(())
        } else {
            Err(error("unclosed control block"))
        }
    }
}

pub(super) fn check_document(action: &AilAction, document: &AilDocument) -> Result<(), String> {
    for control in &action.controls {
        if let AilControl::Recover { errors, key } = control {
            recovery::mask(errors).map_err(error)?;
            if runtime_field_type_name(document, key) != Some("Text") {
                return Err(error("recovery destination must be a declared Text field"));
            }
        }
        if let Some(condition) = control.boolean_condition() {
            condition.validate()?;
            for (field, kind) in condition.typed_fields()? {
                if runtime_field_type_name(document, &field) != Some(kind.as_str()) {
                    return Err(format!(
                        "AIL-BOOL-001: predicate operand {field} must be a declared {kind}"
                    ));
                }
            }
        }
        if let Some(condition) = control.float_condition() {
            FloatCondition::from_postfix(
                &condition.left.to_postfix(),
                condition.comparison.token(),
                &condition.right.to_postfix(),
            )?;
            for field in condition.fields() {
                if runtime_field_type_name(document, &field) != Some("Float") {
                    return Err(format!(
                        "AIL-FLOAT-001: comparison operand {field} must be a declared Float"
                    ));
                }
            }
        }
        if let Some(condition) = control.condition() {
            IntCondition::from_postfix(
                &condition.left.to_postfix(),
                condition.comparison.token(),
                &condition.right.to_postfix(),
            )?;
            for field in condition.fields() {
                if runtime_field_type_name(document, &field) != Some("Int") {
                    return Err(error(format!(
                        "comparison operand {field} must be a declared Int"
                    )));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod floating_tests {
    use super::*;
    #[test]
    fn boolean_controls_emit_typed_branches_and_share_nested_loop_targets() {
        let mut emitter = Emitter::default();
        let mut code = Vec::new();
        for source in [
            "while boolean not data.done:",
            "if floating data.score > 0:",
            "if boolean data.enabled or false:",
            "continue",
            "otherwise:",
            "break",
            "end if",
            "end if",
            "end while",
        ] {
            let control = parse_bullet(source).unwrap().unwrap();
            assert_eq!(parse_bullet(&control.source()).unwrap().unwrap(), control);
            emitter.emit(&control, &mut code).unwrap();
        }
        emitter.finish().unwrap();
        assert_eq!(code.iter().filter(|i| i.opcode == "BRANCH_BOOL").count(), 2);
        assert!(
            code.iter()
                .any(|i| i.opcode == "JUMP" && i.operands["label"] == "__control_0_head")
        );
        assert!(parse_bullet("if boolean data.enabled").unwrap().is_err());
        assert!(parse_bullet("if booleanized validation fails").is_none());
        assert!(parse_bullet("if boolean.enabled is missing").is_none());
    }
    #[test]
    fn mixed_integer_and_floating_controls_share_nesting_and_loop_targets() {
        let mut emitter = Emitter::default();
        let mut code = Vec::new();
        for source in [
            "while floating data.value > 0:",
            "if data.count == 1:",
            "continue",
            "otherwise:",
            "if floating data.value != nan:",
            "break",
            "end if",
            "end if",
            "end while",
        ] {
            let control = parse_bullet(source).unwrap().unwrap();
            assert_eq!(parse_bullet(&control.source()).unwrap().unwrap(), control);
            emitter.emit(&control, &mut code).unwrap();
        }
        emitter.finish().unwrap();
        assert_eq!(
            code.iter().filter(|i| i.opcode == "BRANCH_FLOAT").count(),
            2
        );
        assert_eq!(code.iter().filter(|i| i.opcode == "BRANCH_INT").count(), 1);
        assert!(
            code.iter()
                .any(|i| i.opcode == "JUMP" && i.operands["label"] == "__control_0_head")
        );
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    #[test]
    fn recovery_source_roundtrips_and_canonicalizes_filters() {
        for (source, canonical) in [
            ("try:.", "try:"),
            (
                "recover INT or DIVZERO into data.error:",
                "recover DIVZERO or INT into data.error:",
            ),
            (
                "recover any into local.error:.",
                "recover any into local.error:",
            ),
            ("end try.", "end try"),
        ] {
            let control = parse_bullet(source)
                .expect("recognized recovery syntax")
                .unwrap();
            assert_eq!(control.source(), canonical);
            assert_eq!(parse_bullet(&control.source()).unwrap().unwrap(), control);
        }
    }

    #[test]
    fn recovery_rejects_malformed_forms_without_claiming_identifiers() {
        for source in [
            "try",
            "try extra:",
            "try::",
            "recover",
            "recover INT into data.error",
            "recover INT or INT into data.error:",
            "recover any or INT into data.error:",
            "recover IO into data.error:",
            "recover INT;JSON into data.error:",
            "recover INT into nope:",
            "recover INT into data.error trailing:",
            "end try:",
            "end try extra",
        ] {
            assert!(parse_bullet(source).is_some_and(|r| r.is_err()), "{source}");
        }
        for source in [
            "trying something",
            "recovery happened",
            "try_count",
            "recover.item",
            "end trying",
        ] {
            assert!(parse_bullet(source).is_none(), "{source}");
        }
    }

    #[test]
    fn recovery_emitter_patches_begin_across_nested_if_and_loop() {
        let mut emitter = Emitter::default();
        let mut instructions = Vec::new();
        for source in [
            "while data.count > 0:",
            "try:",
            "if data.count == 2:",
            "continue",
            "end if",
            "try:",
            "break",
            "recover JSON into data.other:",
            "end try",
            "recover INT or DIVZERO into data.error:",
            "break",
            "end try",
            "end while",
        ] {
            emitter
                .emit(
                    &parse_bullet(source).expect("recognized control").unwrap(),
                    &mut instructions,
                )
                .unwrap();
        }
        emitter.finish().unwrap();
        let begins: Vec<_> = instructions
            .iter()
            .filter(|i| i.opcode == "TRY_BEGIN")
            .collect();
        assert_eq!(begins.len(), 2);
        assert_eq!(
            begins[0].operands,
            BTreeMap::from([
                ("errors".into(), "DIVZERO;INT".into()),
                ("key".into(), "data.error".into())
            ])
        );
        assert_eq!(begins[1].operands["errors"], "JSON");
        assert_eq!(begins[1].operands["key"], "data.other");
        assert_eq!(
            instructions
                .iter()
                .filter(|i| i.opcode == "RECOVER")
                .count(),
            2
        );
        assert_eq!(
            instructions
                .iter()
                .filter(|i| i.opcode == "TRY_END")
                .count(),
            2
        );
    }
}

#[cfg(test)]
mod recovery_core_tests {
    use super::*;

    fn fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Recovery manages data.\n\nA Data has:\n- number: Int\n- error: Text\n- other: Text\n\nAction: Run.\nWhen Run happens:\n- try:\n- the system computes data.number as 1 / 0\n- recover INT or DIVZERO into data.error:\n- the system computes data.number as -1\n- end try\n- the system records a trace event named Done\n";
        elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, source).unwrap(),
        )
    }
    fn routes(core: &AilCore) -> [AilCore; 2] {
        [
            core.clone(),
            parse_ail_core_text(&render_ail_core(core)).unwrap(),
        ]
    }
    fn recover(core: &AilCore) -> String {
        core.graph
            .nodes
            .iter()
            .find(|n| {
                n.attributes
                    .get("control_kind")
                    .is_some_and(|k| k == "recover")
            })
            .unwrap()
            .id
            .clone()
    }
    fn rejects(core: &AilCore) {
        for (route, core) in ["direct", "saved"].into_iter().zip(routes(core)) {
            assert!(
                !check(&core).is_empty(),
                "malformed recovery Core accepted through {route}"
            );
        }
    }
    #[test]
    fn recovery_core_roundtrips_preserve_roles_filters_namespace_and_accesses() {
        let original = fixture();
        for core in routes(&original) {
            assert!(check(&core).is_empty(), "{:?}", check(&core));
            let lowered = compile_ail_core_bytecode(&core).unwrap();
            let begin = lowered.actions["Run"]
                .instructions
                .iter()
                .find(|i| i.opcode == "TRY_BEGIN")
                .unwrap();
            assert_eq!(begin.operands["errors"], "DIVZERO;INT");
            assert_eq!(begin.operands["key"], "data.error");
            let action = core.graph.find_node("Action", "Run").unwrap();
            let controls = from_core(&core, action);
            assert_eq!(controls.len(), 3);
            assert_eq!(
                controls[1].source(),
                "recover DIVZERO or INT into data.error:"
            );
            assert_eq!(
                namespace(&controls[1], "Child").source(),
                "recover DIVZERO or INT into child.data.error:"
            );
            let mut without_descriptions = core.clone();
            without_descriptions
                .graph
                .edges
                .retain(|edge| edge.kind != "writes");
            assert_eq!(accesses(&without_descriptions, "writes").len(), 1);
            assert!(accesses(&without_descriptions, "reads").is_empty());
        }
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = render_ail_spec_from_core(&original);
        let reparsed = elaborate_ail_core(
            &package,
            &parse_ail_package_spec_text(&package, &source).unwrap(),
        );
        assert!(check(&reparsed).is_empty());
    }
    #[test]
    fn recovery_core_rejects_duplicate_operands_owners_and_foreign_parents() {
        for relation in ["recovery_destination", "control_step", "observes"] {
            let mut core = fixture();
            let id = recover(&core);
            let mut edge = core
                .graph
                .edges
                .iter()
                .find(|e| {
                    if relation == "recovery_destination" {
                        e.source == id && e.kind == relation
                    } else {
                        e.target == id && e.kind == "control_step"
                    }
                })
                .unwrap()
                .clone();
            edge.id = format!("duplicate-{relation}");
            edge.kind = relation.into();
            edge.ordinal = core.graph.edges.len();
            core.graph.edges.push(edge);
            rejects(&core);
        }
    }
    #[test]
    fn recovery_core_rejects_noncanonical_filters_and_ignored_metadata() {
        for (key, value) in [
            ("recovery_errors", "INT;DIVZERO"),
            ("recovery_errors", "any;INT"),
            ("recovery_errors", "IO"),
            ("ignored", "true"),
        ] {
            let mut core = fixture();
            let id = recover(&core);
            core.graph
                .nodes
                .iter_mut()
                .find(|n| n.id == id)
                .unwrap()
                .attributes
                .insert(key.into(), value.into());
            rejects(&core);
        }
        let mut core = fixture();
        let id = recover(&core);
        core.graph
            .edges
            .iter_mut()
            .find(|e| e.source == id && e.kind == "recovery_destination")
            .unwrap()
            .attributes
            .insert("ignored".into(), "true".into());
        rejects(&core);
        let mut core = fixture();
        let id = recover(&core);
        core.graph
            .nodes
            .iter_mut()
            .find(|n| n.id == id)
            .unwrap()
            .type_name = Some("Text".into());
        rejects(&core);
    }
    #[test]
    fn recovery_core_requires_genuine_global_text_and_executable_owners() {
        for case in ["type", "orphan", "local", "compiler", "profile"] {
            let mut core = fixture();
            let id = recover(&core);
            let target = core
                .graph
                .edges
                .iter()
                .find(|e| e.source == id && e.kind == "recovery_destination")
                .unwrap()
                .target
                .clone();
            match case {
                "type" => {
                    core.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == target)
                        .unwrap()
                        .type_name = Some("Int".into())
                }
                "orphan" => core
                    .graph
                    .edges
                    .retain(|e| !(e.kind == "has_field" && e.target == target)),
                "local" => {
                    core.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == target)
                        .unwrap()
                        .attributes
                        .insert("function_owner".into(), "fake".into());
                }
                "compiler" => {
                    core.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.kind == "Action")
                        .unwrap()
                        .attributes
                        .insert("kind".into(), "CompilerPass".into());
                }
                _ => core.package.profile = "System".into(),
            }
            rejects(&core);
        }
    }
    #[test]
    fn recovery_secret_writes_remain_checked_without_descriptive_edges() {
        let mut core = fixture();
        core.graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "Field" && node.name == "Data.error")
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        core.graph
            .edges
            .retain(|edge| !matches!(edge.kind.as_str(), "reads" | "writes"));
        for core in routes(&core) {
            let diagnostics = check_ail_core(&core);
            assert!(
                diagnostics.iter().any(|message| message.contains("AIL002")),
                "{diagnostics:?}"
            );
        }
    }

    #[test]
    fn recovery_nesting_requires_one_clause_and_obeys_existing_limits() {
        for sources in [
            vec!["recover any into data.error:"],
            vec!["end try"],
            vec!["try:", "end try"],
            vec![
                "try:",
                "recover any into data.error:",
                "recover INT into data.other:",
                "end try",
            ],
            vec![
                "try:",
                "if data.number == 0:",
                "recover any into data.error:",
                "end if",
                "end try",
            ],
            vec!["try:", "recover any into data.error:"],
        ] {
            let mut emitter = Emitter::default();
            let mut code = Vec::new();
            let emitted = sources.into_iter().try_for_each(|source| {
                emitter.emit(&parse_bullet(source).unwrap().unwrap(), &mut code)
            });
            assert!(emitted.and_then(|_| emitter.finish()).is_err());
        }
        let mut emitter = Emitter::default();
        let mut code = Vec::new();
        for _ in 0..64 {
            emitter.emit(&AilControl::Try, &mut code).unwrap();
        }
        assert!(emitter.emit(&AilControl::Try, &mut code).is_err());
        for _ in 0..64 {
            emitter
                .emit(
                    &AilControl::Recover {
                        errors: "any".into(),
                        key: "data.error".into(),
                    },
                    &mut code,
                )
                .unwrap();
            emitter.emit(&AilControl::EndTry, &mut code).unwrap();
        }
        emitter.finish().unwrap();
    }
}
