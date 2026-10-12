//! Executable integer functions, with explicitly owned local fields in Core.
use super::*;
use crate::condition::{IntComparison, IntCondition};
use crate::expression::{IntExpr, IntOperator};

const MAX_VARIABLES: usize = 256;
const MAX_BODY_STEPS: usize = 4096;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AilIntegerFunction {
    pub locals: BTreeMap<String, String>,
    pub body: Vec<IntegerStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegerStep {
    Compute { key: String, expression: IntExpr },
    Call(AilIntegerCall),
    Return(IntExpr),
    Control(AilControl),
    Trace(String),
    ValueOperation(ValueOperation),
    Invalid(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AilIntegerCall {
    pub target: String,
    pub arguments: BTreeMap<String, IntExpr>,
    pub key: String,
}

fn error(message: impl Display) -> String {
    format!("AIL-FUNCTION-001: {message}")
}

fn identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn field_reference(key: &str) -> Result<(), String> {
    if matches!(IntExpr::parse(key)?, IntExpr::Field(parsed) if parsed == key) {
        Ok(())
    } else {
        Err(error("destination must be a field reference"))
    }
}

pub(super) fn parse_call(text: &str) -> Result<AilIntegerCall, String> {
    let text = trim_sentence(text);
    let (invocation, key) = text
        .rsplit_once(" into ")
        .ok_or_else(|| error("call requires 'into field'"))?;
    let (target, arguments) = invocation
        .split_once(" with ")
        .ok_or_else(|| error("call requires 'with name = expression' or 'with none'"))?;
    if target.is_empty() || target.trim() != target || target.contains([';', '=', '\n']) {
        return Err(error("invalid function target"));
    }
    field_reference(key)?;
    let mut parsed = BTreeMap::new();
    if arguments != "none" {
        for argument in arguments.split(';') {
            let (name, expression) = argument
                .split_once('=')
                .ok_or_else(|| error("expected named argument"))?;
            let name = name.trim();
            if !identifier(name) || parsed.contains_key(name) {
                return Err(error("invalid or duplicate argument name"));
            }
            parsed.insert(name.to_string(), IntExpr::parse(expression.trim())?);
        }
    }
    if parsed.len() > MAX_VARIABLES {
        return Err(error("too many arguments"));
    }
    Ok(AilIntegerCall {
        target: target.into(),
        arguments: parsed,
        key: key.into(),
    })
}

pub(super) fn parse_bullet(function: &mut AilIntegerFunction, bullet: &str) -> Result<(), String> {
    if function.body.len() >= MAX_BODY_STEPS {
        return Err(error("too many function body steps"));
    }
    let text = trim_sentence(bullet);
    if control_flow::raise_bullet(&text) && !text.starts_with("the function ") {
        return Err(error("function raises require the function subject"));
    }
    let step = if let Some(control) = control_flow::parse_bullet(&text) {
        IntegerStep::Control(control?)
    } else if let Some(expression) = text.strip_prefix("the function returns ") {
        IntegerStep::Return(IntExpr::parse(expression)?)
    } else if let Some(call) = text.strip_prefix("the function calls ") {
        IntegerStep::Call(parse_call(call)?)
    } else if let Some(operation) = value_function::parse_operation(&text) {
        IntegerStep::ValueOperation(operation.map_err(error)?)
    } else if let Some(compute) = text.strip_prefix("the function computes ") {
        let (key, expression) = compute
            .split_once(" as ")
            .ok_or_else(|| error("expected 'computes local.name as expression'"))?;
        field_reference(key)?;
        IntegerStep::Compute {
            key: key.into(),
            expression: IntExpr::parse(expression)?,
        }
    } else if let Some(trace) = text.strip_prefix("the function records a trace event named ") {
        if trace.is_empty() {
            return Err(error("trace event name is empty"));
        }
        IntegerStep::Trace(trace.into())
    } else {
        return Err(error(format!(
            "unsupported executable function statement: {text}"
        )));
    };
    function.body.push(step);
    Ok(())
}

pub(super) fn encode_arguments(arguments: &BTreeMap<String, IntExpr>) -> String {
    arguments
        .iter()
        .map(|(name, expression)| format!("{name}={}", expression.to_postfix()))
        .collect::<Vec<_>>()
        .join(";")
}

pub(super) fn decode_arguments(text: &str) -> Result<BTreeMap<String, IntExpr>, String> {
    let mut arguments = BTreeMap::new();
    if !text.is_empty() {
        for item in text.split(';') {
            let (name, expression) = item
                .split_once('=')
                .ok_or_else(|| error("malformed encoded argument"))?;
            if !identifier(name) || arguments.contains_key(name) || arguments.len() >= MAX_VARIABLES
            {
                return Err(error("invalid, duplicate, or excessive argument names"));
            }
            arguments.insert(name.into(), IntExpr::from_postfix(expression)?);
        }
    }
    Ok(arguments)
}

fn call_source(call: &AilIntegerCall) -> String {
    let arguments = if call.arguments.is_empty() {
        "none".into()
    } else {
        call.arguments
            .iter()
            .map(|(name, expression)| format!("{name} = {}", expression.to_source()))
            .collect::<Vec<_>>()
            .join("; ")
    };
    format!("{} with {arguments} into {}", call.target, call.key)
}

pub(super) fn render(function: &AilIntegerFunction) -> Vec<String> {
    let mut lines = Vec::new();
    if !function.locals.is_empty() {
        lines.push("The function stores:".into());
        for (name, type_name) in &function.locals {
            lines.push(format!("- {name}: {type_name}"));
        }
        lines.push(String::new());
    }
    lines.push("The function executes:".into());
    for step in &function.body {
        let text = match step {
            IntegerStep::Compute { key, expression } => {
                format!("the function computes {key} as {}", expression.to_source())
            }
            IntegerStep::Call(call) => format!("the function calls {}", call_source(call)),
            IntegerStep::Return(expression) => {
                format!("the function returns {}", expression.to_source())
            }
            IntegerStep::Control(AilControl::Raise { category }) => {
                format!("the function raises error {category}")
            }
            IntegerStep::Control(control) => control.source(),
            IntegerStep::Trace(trace) => {
                format!("the function records a trace event named {trace}")
            }
            IntegerStep::ValueOperation(operation) => operation.source(),
            IntegerStep::Invalid(message) => format!("invalid function statement: {message}"),
        };
        lines.push(format!("- {text}"));
    }
    lines
}

fn signature(function: &AilFunction) -> Result<BTreeSet<String>, String> {
    let integer = function.integer.as_ref().ok_or_else(|| {
        error(format!(
            "{} is not an executable integer function",
            function.name
        ))
    })?;
    if function.inputs.len() + integer.locals.len() > MAX_VARIABLES {
        return Err(error("function exceeds 256 parameters and locals"));
    }
    if function.outputs.len() != 1
        || function
            .outputs
            .values()
            .any(|v| v.type_name != "Int" || !identifier(&v.name))
    {
        return Err(error("integer function requires exactly one Int output"));
    }
    if !function.branches.is_empty()
        || !function.calls.is_empty()
        || !function.returns.is_empty()
        || !function.traces.is_empty()
    {
        return Err(error(
            "executable function cannot contain legacy descriptive statements",
        ));
    }
    let mut names = BTreeSet::new();
    for (name, value) in &function.inputs {
        if !identifier(name) || &value.name != name || value.type_name != "Int" {
            return Err(error("function parameters must be named Int values"));
        }
        names.insert(format!("local.{name}"));
    }
    for (name, type_name) in &integer.locals {
        if !identifier(name) || type_name != "Int" || !names.insert(format!("local.{name}")) {
            return Err(error("locals must be distinct named Int values"));
        }
    }
    Ok(names)
}

fn local_expression(expression: &IntExpr, names: &BTreeSet<String>) -> Result<(), String> {
    IntExpr::from_postfix(&expression.to_postfix())?;
    for field in expression.fields() {
        if !names.contains(&field) {
            return Err(error(format!(
                "undeclared or nonlocal function field {field}"
            )));
        }
    }
    Ok(())
}

fn validate_call(call: &AilIntegerCall, document: &AilDocument) -> Result<(), String> {
    let target = document
        .functions
        .get(&call.target)
        .ok_or_else(|| error(format!("unknown integer function {}", call.target)))?;
    signature(target)?;
    if call.arguments.keys().ne(target.inputs.keys()) {
        return Err(error(format!(
            "arguments do not match function {} parameters",
            call.target
        )));
    }
    Ok(())
}

pub(super) fn compile(
    function: &AilFunction,
    document: &AilDocument,
) -> Result<AilBytecodeAction, String> {
    if value_function::classify(document).contains(&function.name) {
        return value_function::compile(function, document);
    }
    let names = signature(function)?;
    let integer = function.integer.as_ref().unwrap();
    if integer.body.len() > MAX_BODY_STEPS {
        return Err(error("too many function body steps"));
    }
    let mut instructions = vec![AilBytecodeInstruction::new(
        "INTEGER_FUNCTION",
        &[(
            "parameters",
            function
                .inputs
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(";"),
        )],
    )];
    let mut controls = control_flow::Emitter::default();
    for step in &integer.body {
        match step {
            IntegerStep::Compute { key, expression } => {
                if !names.contains(key) {
                    return Err(error(format!("undeclared local destination {key}")));
                }
                local_expression(expression, &names)?;
                instructions.push(AilBytecodeInstruction::new(
                    "EVAL_INT",
                    &[
                        ("key", key.clone()),
                        ("expression", expression.to_postfix()),
                    ],
                ));
            }
            IntegerStep::Call(call) => {
                validate_call(call, document)?;
                if !names.contains(&call.key) {
                    return Err(error(format!("undeclared local destination {}", call.key)));
                }
                for expression in call.arguments.values() {
                    local_expression(expression, &names)?;
                }
                instructions.push(AilBytecodeInstruction::new(
                    "CALL_INT",
                    &[
                        ("target", call.target.clone()),
                        ("arguments", encode_arguments(&call.arguments)),
                        ("key", call.key.clone()),
                    ],
                ));
            }
            IntegerStep::Return(expression) => {
                local_expression(expression, &names)?;
                instructions.push(AilBytecodeInstruction::new(
                    "RETURN_INT",
                    &[("expression", expression.to_postfix())],
                ));
            }
            IntegerStep::Control(control) => {
                if matches!(
                    control,
                    AilControl::Try | AilControl::Recover { .. } | AilControl::EndTry
                ) {
                    return Err(error(
                        "recovery requires a value function with a declared Text destination",
                    ));
                }
                if let Some(condition) = control.condition() {
                    local_expression(&condition.left, &names)?;
                    local_expression(&condition.right, &names)?;
                }
                controls.emit(control, &mut instructions)?;
            }
            IntegerStep::Trace(trace) => instructions.push(AilBytecodeInstruction::new(
                "EMIT_TRACE",
                &[("event", trace.clone())],
            )),
            IntegerStep::ValueOperation(_) => {
                return Err(error("value operation requires a general function"));
            }
            IntegerStep::Invalid(message) => return Err(error(message)),
        }
    }
    controls.finish()?;
    let action = AilBytecodeAction {
        name: function.name.clone(),
        instructions,
    };
    recovery::check_raise_termination(&action).map_err(error)?;
    Ok(action)
}
pub(super) fn owned_node(
    graph: &mut Graph,
    kind: &str,
    name: &str,
    type_name: Option<String>,
    attributes: BTreeMap<String, String>,
) -> Node {
    let node = graph.add_node(kind, name, type_name, attributes);
    attach_provenance(graph, &node, format!("integer:{name}"));
    node
}

pub(super) fn destination(graph: &mut Graph, owner: &Node, key: &str, local: bool) -> Node {
    let found = graph
        .nodes
        .iter()
        .find(|node| {
            node.kind == "Field"
                && expression_core::field_key(node) == key
                && if local {
                    node.attributes.get("function_owner") == Some(&owner.name)
                } else {
                    !node.attributes.contains_key("function_owner")
                }
        })
        .cloned();
    found.unwrap_or_else(|| {
        owned_node(
            graph,
            "InvalidFunctionField",
            &format!("{}.invalid.{key}", owner.name),
            None,
            attr(&[("key", key)]),
        )
    })
}

pub(super) fn build_expression(
    graph: &mut Graph,
    owner: &Node,
    name: &str,
    expression: &IntExpr,
    local: bool,
) -> Node {
    if !local {
        return expression_core::build(graph, owner, name, expression);
    }
    let mut attrs = attr(&[("function_owner", &owner.name)]);
    match expression {
        IntExpr::Literal(value) => {
            attrs.insert("expression_kind".into(), "literal".into());
            attrs.insert("value".into(), value.to_string());
        }
        IntExpr::Field(_) => {
            attrs.insert("expression_kind".into(), "field".into());
        }
        IntExpr::Binary { op, .. } => {
            attrs.insert("expression_kind".into(), "binary".into());
            attrs.insert(
                "operator".into(),
                match op {
                    IntOperator::Add => "+",
                    IntOperator::Subtract => "-",
                    IntOperator::Multiply => "*",
                    IntOperator::Divide => "/",
                    IntOperator::Remainder => "%",
                }
                .into(),
            );
        }
    }
    let node = owned_node(graph, "IntegerExpression", name, Some("Int".into()), attrs);
    match expression {
        IntExpr::Field(key) => {
            let target = destination(graph, owner, key, true);
            graph.add_edge("expression_field", &node, &target, BTreeMap::new());
        }
        IntExpr::Binary { left, right, .. } => {
            let left = build_expression(graph, owner, &format!("{name}.left"), left, true);
            let right = build_expression(graph, owner, &format!("{name}.right"), right, true);
            graph.add_edge("left_operand", &node, &left, BTreeMap::new());
            graph.add_edge("right_operand", &node, &right, BTreeMap::new());
        }
        IntExpr::Literal(_) => {}
    }
    node
}

fn elaborate_call(
    graph: &mut Graph,
    owner: &Node,
    node: &Node,
    call: &AilIntegerCall,
    local: bool,
) {
    let target = graph
        .find_node("Function", &call.target)
        .cloned()
        .unwrap_or_else(|| {
            owned_node(
                graph,
                "InvalidFunctionTarget",
                &call.target,
                None,
                BTreeMap::new(),
            )
        });
    graph.add_edge("call_target", node, &target, BTreeMap::new());
    let target = destination(graph, owner, &call.key, local);
    graph.add_edge("call_destination", node, &target, BTreeMap::new());
    for (name, expression) in &call.arguments {
        let root = build_expression(
            graph,
            owner,
            &format!("{}.argument.{name}", node.name),
            expression,
            local,
        );
        graph.add_edge("call_argument", node, &root, attr(&[("name", name)]));
    }
}

pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    let general = value_function::classify(document);
    // All function declarations already exist, enabling forward and mutual calls.
    for function in document
        .functions
        .values()
        .filter(|f| f.integer.is_some() && !general.contains(&f.name))
    {
        if let Some(node) = graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Function" && n.name == function.name)
        {
            node.attributes
                .insert("executable_integer".into(), "true".into());
        }
    }
    for function in document
        .functions
        .values()
        .filter(|f| !general.contains(&f.name))
    {
        let Some(integer) = &function.integer else {
            continue;
        };
        let Some(owner) = graph.find_node("Function", &function.name).cloned() else {
            continue;
        };
        for (name, type_name, edge_kind) in function
            .inputs
            .iter()
            .map(|(name, value)| (name, &value.type_name, "function_parameter"))
            .chain(
                integer
                    .locals
                    .iter()
                    .map(|(name, type_name)| (name, type_name, "function_local")),
            )
        {
            let field = owned_node(
                graph,
                "Field",
                &format!("{}.$local.{name}", owner.name),
                Some(type_name.clone()),
                attr(&[("function_owner", &owner.name), ("local_name", name)]),
            );
            graph.add_edge(edge_kind, &owner, &field, BTreeMap::new());
        }
        for (index, step) in integer.body.iter().enumerate() {
            let kind = match step {
                IntegerStep::Compute { .. } => "compute",
                IntegerStep::Call(_) => "call",
                IntegerStep::Return(_) => "return",
                IntegerStep::Control(_) => "control",
                IntegerStep::Trace(_) => "trace",
                IntegerStep::ValueOperation(_) | IntegerStep::Invalid(_) => "invalid",
            };
            let mut attrs = attr(&[("step_kind", kind), ("function_owner", &owner.name)]);
            match step {
                IntegerStep::Control(control) => {
                    let kind = match control {
                        AilControl::If(_) => "if",
                        AilControl::Else => "else",
                        AilControl::EndIf => "end_if",
                        AilControl::While(_) => "while",
                        AilControl::EndWhile => "end_while",
                        AilControl::Break => "break",
                        AilControl::Continue => "continue",
                        AilControl::Raise { .. } => "raise",
                        AilControl::Try
                        | AilControl::Recover { .. }
                        | AilControl::EndTry
                        | AilControl::Invalid(_)
                        | AilControl::IfBoolean(_)
                        | AilControl::WhileBoolean(_)
                        | AilControl::IfFloat(_)
                        | AilControl::WhileFloat(_) => "invalid",
                    };
                    attrs.insert("control_kind".into(), kind.into());
                    if let AilControl::Raise { category } = control {
                        attrs.insert("error_category".into(), category.clone());
                    }
                }
                IntegerStep::Trace(trace) => {
                    attrs.insert("event".into(), trace.clone());
                }
                IntegerStep::Invalid(message) => {
                    attrs.insert("error".into(), message.clone());
                }
                _ => {}
            }
            let node = owned_node(
                graph,
                "FunctionStep",
                &format!("{}.step{index}", owner.name),
                None,
                attrs,
            );
            graph.add_edge(
                "function_step",
                &owner,
                &node,
                attr(&[("step_index", &index.to_string())]),
            );
            match step {
                IntegerStep::Compute { key, expression } => {
                    let field = destination(graph, &owner, key, true);
                    graph.add_edge("function_destination", &node, &field, BTreeMap::new());
                    let root = build_expression(
                        graph,
                        &owner,
                        &format!("{}.value", node.name),
                        expression,
                        true,
                    );
                    graph.add_edge("function_expression", &node, &root, BTreeMap::new());
                }
                IntegerStep::Return(expression) => {
                    let root = build_expression(
                        graph,
                        &owner,
                        &format!("{}.value", node.name),
                        expression,
                        true,
                    );
                    graph.add_edge("function_expression", &node, &root, BTreeMap::new());
                }
                IntegerStep::Call(call) => elaborate_call(graph, &owner, &node, call, true),
                IntegerStep::Control(control) => {
                    if let Some(condition) = control.condition() {
                        let comparison = owned_node(
                            graph,
                            "IntegerComparison",
                            &format!("{}.condition", node.name),
                            Some("Bool".into()),
                            attr(&[("comparison", condition.comparison.token())]),
                        );
                        let left = build_expression(
                            graph,
                            &owner,
                            &format!("{}.left", node.name),
                            &condition.left,
                            true,
                        );
                        let right = build_expression(
                            graph,
                            &owner,
                            &format!("{}.right", node.name),
                            &condition.right,
                            true,
                        );
                        graph.add_edge("comparison_left", &comparison, &left, BTreeMap::new());
                        graph.add_edge("comparison_right", &comparison, &right, BTreeMap::new());
                        graph.add_edge("function_condition", &node, &comparison, BTreeMap::new());
                    }
                }
                _ => {}
            }
        }
    }
    for action in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &action.name).cloned() else {
            continue;
        };
        for (index, call) in action.integer_calls.iter().enumerate() {
            if general.contains(&call.target) {
                value_function::elaborate_action_call(graph, document, &owner, call, index);
                continue;
            }
            let node = owned_node(
                graph,
                "ValueCall",
                &format!("{}.integer_call{index}", owner.name),
                None,
                BTreeMap::new(),
            );
            elaborate_call(graph, &owner, &node, call, false);
            action_order::append(
                graph,
                "integer_call_step",
                &owner,
                &node,
                BTreeMap::new(),
                index,
            );
        }
    }
}
fn node_by_id<'a>(core: &'a AilCore, id: &str) -> Result<&'a Node, String> {
    core.graph
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or_else(|| error("missing function graph endpoint"))
}

fn outgoing<'a>(core: &'a AilCore, node: &Node, kind: &str) -> Vec<&'a Edge> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == kind)
        .collect()
}

fn child<'a>(core: &'a AilCore, node: &Node, kind: &str) -> Result<&'a Node, String> {
    let edges = outgoing(core, node, kind);
    if edges.len() != 1 {
        return Err(error(format!(
            "{} requires exactly one {kind} edge",
            node.name
        )));
    }
    node_by_id(core, &edges[0].target)
}

fn executable(node: &Node) -> bool {
    node.kind == "Function"
        && node
            .attributes
            .get("executable_integer")
            .is_some_and(|v| v == "true")
}

fn scoped_field(core: &AilCore, field: &Node, owner: Option<&Node>) -> Result<String, String> {
    if field.kind != "Field" || field.type_name.as_deref() != Some("Int") {
        return Err(error("integer function field must be a declared Int"));
    }
    if let Some(owner) = owner {
        let name = field
            .attributes
            .get("local_name")
            .ok_or_else(|| error("function cannot access a global field"))?;
        if !identifier(name) || field.attributes.get("function_owner") != Some(&owner.name) {
            return Err(error(
                "local field belongs to another function or has an invalid name",
            ));
        }
        let declarations: Vec<_> = core
            .graph
            .edges
            .iter()
            .filter(|e| {
                e.target == field.id
                    && matches!(e.kind.as_str(), "function_parameter" | "function_local")
            })
            .collect();
        if declarations.len() != 1 || declarations[0].source != owner.id {
            return Err(error(
                "local field must have exactly one declaration in its owning function",
            ));
        }
    } else if field.attributes.contains_key("function_owner")
        || field.attributes.contains_key("local_name")
    {
        return Err(error("action cannot access function local storage"));
    }
    let key = expression_core::field_key(field);
    field_reference(&key)?;
    Ok(key)
}

fn scoped_expression(core: &AilCore, root: &Node, owner: Option<&Node>) -> Result<IntExpr, String> {
    let expression = expression_core::decode_root(core, &root.id)?;
    let mut pending = vec![root];
    let mut visited = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if !visited.insert(node.id.clone()) {
            continue;
        }
        for edge in core.graph.edges.iter().filter(|e| e.source == node.id) {
            match edge.kind.as_str() {
                "expression_field" => {
                    scoped_field(core, node_by_id(core, &edge.target)?, owner)?;
                }
                "left_operand" | "right_operand" => pending.push(node_by_id(core, &edge.target)?),
                "has_provenance" => {}
                _ => return Err(error("unexpected edge on integer expression")),
            }
        }
    }
    Ok(expression)
}

fn graph_parameters(
    core: &AilCore,
    function: &Node,
) -> Result<BTreeMap<String, AilFunctionValue>, String> {
    let mut result = BTreeMap::new();
    for edge in outgoing(core, function, "has_input") {
        let node = node_by_id(core, &edge.target)?;
        let name = node
            .name
            .strip_prefix(&format!("{}.", function.name))
            .ok_or_else(|| error("input is not owned by its function"))?;
        if node.kind != "Input"
            || node.type_name.as_deref() != Some("Int")
            || !identifier(name)
            || result.contains_key(name)
        {
            return Err(error("function inputs must be distinct named Int values"));
        }
        result.insert(
            name.into(),
            AilFunctionValue {
                name: name.into(),
                type_name: "Int".into(),
                provenance: String::new(),
            },
        );
    }
    if result.len() > MAX_VARIABLES {
        return Err(error("too many function parameters"));
    }
    Ok(result)
}

fn decode_call(
    core: &AilCore,
    node: &Node,
    owner: Option<&Node>,
) -> Result<AilIntegerCall, String> {
    let target = child(core, node, "call_target")?;
    if !executable(target) {
        return Err(error(
            "value call must target an executable integer function",
        ));
    }
    let parameters = graph_parameters(core, target)?;
    let key = scoped_field(core, child(core, node, "call_destination")?, owner)?;
    let mut arguments = BTreeMap::new();
    for edge in outgoing(core, node, "call_argument") {
        let name = edge
            .attributes
            .get("name")
            .ok_or_else(|| error("call argument lacks a name"))?;
        if !identifier(name) || arguments.contains_key(name) {
            return Err(error("invalid or duplicate call argument"));
        }
        arguments.insert(
            name.clone(),
            scoped_expression(core, node_by_id(core, &edge.target)?, owner)?,
        );
    }
    if arguments.keys().ne(parameters.keys()) {
        return Err(error("call arguments do not match target parameters"));
    }
    Ok(AilIntegerCall {
        target: target.name.clone(),
        arguments,
        key,
    })
}

fn decode_condition(core: &AilCore, node: &Node, owner: &Node) -> Result<IntCondition, String> {
    let comparison = child(core, node, "function_condition")?;
    if comparison.kind != "IntegerComparison" || comparison.type_name.as_deref() != Some("Bool") {
        return Err(error(
            "function condition requires a Bool integer comparison",
        ));
    }
    let left = scoped_expression(
        core,
        child(core, comparison, "comparison_left")?,
        Some(owner),
    )?;
    let right = scoped_expression(
        core,
        child(core, comparison, "comparison_right")?,
        Some(owner),
    )?;
    let op = IntComparison::from_token(
        comparison
            .attributes
            .get("comparison")
            .ok_or_else(|| error("comparison lacks operator"))?,
    )?;
    IntCondition::from_postfix(&left.to_postfix(), op.token(), &right.to_postfix())
}

fn decode_step(core: &AilCore, node: &Node, owner: &Node) -> Result<IntegerStep, String> {
    if node
        .attributes
        .get("control_kind")
        .is_some_and(|kind| kind == "raise")
    {
        control_flow::validate_raise_node(core, node, Some(owner)).map_err(error)?;
    }
    if node.kind != "FunctionStep" || node.attributes.get("function_owner") != Some(&owner.name) {
        return Err(error("function step must belong to its enclosing function"));
    }
    let kind = node
        .attributes
        .get("step_kind")
        .map(String::as_str)
        .unwrap_or("");
    let allowed: &[&str] = match kind {
        "compute" => &["function_destination", "function_expression"],
        "return" => &["function_expression"],
        "call" => &["call_target", "call_destination", "call_argument"],
        "control" => &["function_condition"],
        "trace" => &[],
        _ => return Err(error("invalid function step kind")),
    };
    if core.graph.edges.iter().any(|e| {
        e.source == node.id && e.kind != "has_provenance" && !allowed.contains(&e.kind.as_str())
    }) {
        return Err(error("unexpected edge on function step"));
    }
    Ok(match kind {
        "compute" => IntegerStep::Compute {
            key: scoped_field(
                core,
                child(core, node, "function_destination")?,
                Some(owner),
            )?,
            expression: scoped_expression(
                core,
                child(core, node, "function_expression")?,
                Some(owner),
            )?,
        },
        "return" => IntegerStep::Return(scoped_expression(
            core,
            child(core, node, "function_expression")?,
            Some(owner),
        )?),
        "call" => IntegerStep::Call(decode_call(core, node, Some(owner))?),
        "trace" => IntegerStep::Trace(
            node.attributes
                .get("event")
                .filter(|s| !s.is_empty())
                .cloned()
                .ok_or_else(|| error("trace lacks event name"))?,
        ),
        "control" => {
            let control_kind = node
                .attributes
                .get("control_kind")
                .map(String::as_str)
                .unwrap_or("");
            if !matches!(control_kind, "if" | "while")
                && !outgoing(core, node, "function_condition").is_empty()
            {
                return Err(error("function control delimiter cannot have a condition"));
            }
            IntegerStep::Control(match control_kind {
                "raise" => AilControl::Raise {
                    category: control_flow::validate_raise_node(core, node, Some(owner))
                        .map_err(error)?,
                },
                "if" => AilControl::If(decode_condition(core, node, owner)?),
                "while" => AilControl::While(decode_condition(core, node, owner)?),
                "else" => AilControl::Else,
                "end_if" => AilControl::EndIf,
                "end_while" => AilControl::EndWhile,
                "break" => AilControl::Break,
                "continue" => AilControl::Continue,
                _ => return Err(error("invalid function control kind")),
            })
        }
        _ => unreachable!(),
    })
}

fn decode_function(core: &AilCore, owner: &Node) -> Result<AilIntegerFunction, String> {
    let mut function = AilIntegerFunction::default();
    for edge in outgoing(core, owner, "function_local") {
        let node = node_by_id(core, &edge.target)?;
        let key = scoped_field(core, node, Some(owner))?;
        let name = key
            .strip_prefix("local.")
            .ok_or_else(|| error("invalid local field"))?;
        if function.locals.insert(name.into(), "Int".into()).is_some() {
            return Err(error("duplicate local declaration"));
        }
    }
    let mut steps = BTreeMap::new();
    for edge in outgoing(core, owner, "function_step") {
        let index = edge
            .attributes
            .get("step_index")
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or_else(|| error("function step lacks a valid index"))?;
        if index >= MAX_BODY_STEPS
            || steps
                .insert(index, node_by_id(core, &edge.target)?)
                .is_some()
        {
            return Err(error("duplicate or excessive function step index"));
        }
    }
    for (expected, (index, node)) in steps.into_iter().enumerate() {
        if index != expected {
            return Err(error("function step indices must be contiguous"));
        }
        function.body.push(decode_step(core, node, owner)?);
    }
    Ok(function)
}

pub(super) fn from_core_function(core: &AilCore, node: &Node) -> Option<AilIntegerFunction> {
    if value_function::is_executable(node) {
        return value_function::from_core_function(core, node);
    }
    if !executable(node) {
        return None;
    }
    Some(
        decode_function(core, node).unwrap_or_else(|message| AilIntegerFunction {
            locals: BTreeMap::new(),
            body: vec![IntegerStep::Invalid(message)],
        }),
    )
}

pub(super) fn calls_from_core(core: &AilCore, node: &Node) -> Vec<AilIntegerCall> {
    outgoing(core, node, "integer_call_step")
        .into_iter()
        .map(|edge| {
            node_by_id(core, &edge.target)
                .and_then(|node| {
                    if node.kind == "ValueFunctionCall" {
                        value_function::decode_action_call(core, node)
                    } else {
                        decode_call(core, node, None)
                    }
                })
                .unwrap_or_else(|message| AilIntegerCall {
                    target: format!("invalid: {message}"),
                    ..AilIntegerCall::default()
                })
        })
        .collect()
}

fn check_function(core: &AilCore, owner: &Node) -> Result<(), String> {
    let inputs = graph_parameters(core, owner)?;
    let outputs = outgoing(core, owner, "has_output");
    if outputs.len() != 1 {
        return Err(error("integer function requires exactly one Int output"));
    }
    let output = node_by_id(core, &outputs[0].target)?;
    if output.kind != "Output"
        || output.type_name.as_deref() != Some("Int")
        || !output
            .name
            .strip_prefix(&format!("{}.", owner.name))
            .is_some_and(identifier)
    {
        return Err(error("integer function requires a named Int output"));
    }
    let mut parameter_names = BTreeSet::new();
    for edge in outgoing(core, owner, "function_parameter") {
        let key = scoped_field(core, node_by_id(core, &edge.target)?, Some(owner))?;
        if !parameter_names.insert(key) {
            return Err(error("duplicate function parameter storage"));
        }
    }
    let expected: BTreeSet<_> = inputs.keys().map(|n| format!("local.{n}")).collect();
    if parameter_names != expected {
        return Err(error("function parameter storage does not match inputs"));
    }
    let function = decode_function(core, owner)?;
    let mut names = parameter_names;
    for name in function.locals.keys() {
        if !names.insert(format!("local.{name}")) {
            return Err(error("function local duplicates parameter"));
        }
    }
    if names.len() > MAX_VARIABLES {
        return Err(error("function exceeds variable limit"));
    }
    let mut controls = control_flow::Emitter::default();
    let mut instructions = Vec::new();
    for step in &function.body {
        if let IntegerStep::Control(control) = step {
            controls.emit(control, &mut instructions)?;
        }
    }
    controls.finish()?;
    for edge in core.graph.edges.iter().filter(|e| e.source == owner.id) {
        if !matches!(
            edge.kind.as_str(),
            "has_input"
                | "has_output"
                | "function_parameter"
                | "function_local"
                | "function_step"
                | "has_provenance"
                | "has_termination_bound"
                | "has_termination_measure"
        ) {
            return Err(error(
                "executable function cannot contain legacy statements or unrelated edges",
            ));
        }
    }
    Ok(())
}

fn reject_global_local_reads(core: &AilCore, root: &Node) -> Result<(), String> {
    let mut pending = vec![root];
    let mut visited = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if !visited.insert(&node.id) {
            continue;
        }
        for edge in core.graph.edges.iter().filter(|e| e.source == node.id) {
            if let Ok(target) = node_by_id(core, &edge.target) {
                match edge.kind.as_str() {
                    "expression_field"
                        if target.attributes.contains_key("function_owner")
                            || target.attributes.contains_key("local_name") =>
                    {
                        return Err(error(
                            "action expression cannot access function local storage",
                        ));
                    }
                    "left_operand" | "right_operand" | "comparison_left" | "comparison_right"
                    | "condition" => pending.push(target),
                    _ => {}
                }
            }
        }
    }
    Ok(())
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
    for node in &core.graph.nodes {
        let result = (|| -> Result<(), String> {
            if (executable(node) || node.kind == "ValueCall")
                && !matches!(core.package.profile.as_str(), "Application" | "C interop")
            {
                return Err(error(
                    "executable integer functions require the Application or C interop profile",
                ));
            }
            if node.kind == "Control" {
                reject_global_local_reads(core, node)?;
            }
            if node.kind == "Action" {
                for edge in outgoing(core, node, "writes") {
                    if let Some(label) = edge.attributes.get("expression")
                        && let Some(root) = core
                            .graph
                            .nodes
                            .iter()
                            .find(|n| n.id == *label || core_node_label(n) == *label)
                    {
                        reject_global_local_reads(core, root)?;
                    }
                }
            }
            if executable(node) {
                check_function(core, node)?;
            }
            if node.kind == "Function"
                && node.attributes.contains_key("executable_integer")
                && !executable(node)
            {
                return Err(error("invalid executable function marker"));
            }
            if node.kind == "FunctionStep" {
                let owners: Vec<_> = core
                    .graph
                    .edges
                    .iter()
                    .filter(|e| e.kind == "function_step" && e.target == node.id)
                    .collect();
                if owners.len() != 1 {
                    return Err(error("function step requires exactly one owner"));
                }
                let owner = node_by_id(core, &owners[0].source)?;
                if !executable(owner) {
                    return Err(error("function step owner must be executable"));
                }
                decode_step(core, node, owner)?;
            }
            if node.kind == "Field"
                && (node.attributes.contains_key("function_owner")
                    || node.attributes.contains_key("local_name"))
            {
                let owner = node
                    .attributes
                    .get("function_owner")
                    .and_then(|name| core.graph.find_node("Function", name))
                    .ok_or_else(|| error("local storage has no owning function"))?;
                if !value_function::is_executable(owner) {
                    if !executable(owner) {
                        return Err(error("local storage owner must be executable"));
                    }
                    scoped_field(core, node, Some(owner))?;
                }
            }
            if node.kind == "ValueCall" {
                let owners: Vec<_> = core
                    .graph
                    .edges
                    .iter()
                    .filter(|e| e.kind == "integer_call_step" && e.target == node.id)
                    .collect();
                if owners.len() != 1 || !executable_action(node_by_id(core, &owners[0].source)?) {
                    return Err(error("value call requires exactly one action owner"));
                }
                if core.graph.edges.iter().any(|e| {
                    e.source == node.id
                        && !matches!(
                            e.kind.as_str(),
                            "call_target" | "call_destination" | "call_argument" | "has_provenance"
                        )
                }) {
                    return Err(error("unexpected edge on value call"));
                }
                decode_call(core, node, None)?;
            }
            Ok(())
        })();
        if let Err(message) = result {
            diagnostics.push(
                AilDiagnostic::error("AIL-FUNCTION-001", message)
                    .with_affected_graph_item(format!("node:{}", node.id)),
            );
        }
    }
    for edge in &core.graph.edges {
        let source = core.graph.nodes.iter().find(|n| n.id == edge.source);
        let target = core.graph.nodes.iter().find(|n| n.id == edge.target);
        let step = |kind: &str| {
            source.is_some_and(|n| {
                n.kind == "FunctionStep" && n.attributes.get("step_kind").is_some_and(|s| s == kind)
            })
        };
        let call = source.is_some_and(|n| n.kind == "ValueCall") || step("call");
        let valid = match edge.kind.as_str() {
            "function_parameter" | "function_local" => {
                source.is_some_and(executable) && target.is_some_and(|n| n.kind == "Field")
            }
            "function_step" => {
                source.is_some_and(executable) && target.is_some_and(|n| n.kind == "FunctionStep")
            }
            "integer_call_step" => {
                source.is_some_and(executable_action)
                    && target.is_some_and(|n| {
                        matches!(n.kind.as_str(), "ValueCall" | "ValueFunctionCall")
                    })
                    && edge.attributes.contains_key("step_index")
            }
            "function_expression" => {
                (step("compute") || step("return"))
                    && target.is_some_and(|n| n.kind == "IntegerExpression")
            }
            "function_destination" => step("compute") && target.is_some_and(|n| n.kind == "Field"),
            "function_condition" => {
                step("control") && target.is_some_and(|n| n.kind == "IntegerComparison")
            }
            "call_target" => call && target.is_some_and(executable),
            "call_destination" => call && target.is_some_and(|n| n.kind == "Field"),
            "call_argument" => call && target.is_some_and(|n| n.kind == "IntegerExpression"),
            _ => true,
        };
        let invalid_global_local_access = source
            .is_some_and(|n| matches!(n.kind.as_str(), "Action" | "Tool"))
            && target
                .is_some_and(|n| n.kind == "Field" && n.attributes.contains_key("function_owner"));
        if !valid || invalid_global_local_access {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-FUNCTION-001",
                    format!("invalid {} function edge", edge.kind),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    diagnostics
}
pub(super) fn namespace_function(
    function: &AilIntegerFunction,
    alias: &str,
    type_names: &BTreeSet<String>,
) -> AilIntegerFunction {
    let mut result = function.clone();
    for kind in result.locals.values_mut() {
        *kind = value_types::qualify(kind, alias, type_names, &BTreeSet::new());
    }
    for step in &mut result.body {
        if let IntegerStep::ValueOperation(ValueOperation::Variant(op)) = step {
            *op = value_operations::namespace(op, alias, type_names, true);
        }
        if let IntegerStep::Call(call) = step {
            *call = namespace_call(call, alias, true);
        }
    }
    result
}

pub(super) fn namespace_call(call: &AilIntegerCall, alias: &str, local: bool) -> AilIntegerCall {
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
    let mut result = call.clone();
    result.target = qualify_name(alias, &call.target);
    if !local {
        let prefix = runtime_subject_key(alias);
        result.key = format!("{prefix}.{}", call.key);
        for expression in result.arguments.values_mut() {
            *expression = qualify(expression, &prefix);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core_fixture() -> AilCore {
        let package = load_ail_package_dir("bench/R003-max/program.ail").unwrap();
        let source = "The application Numbers manages numbers.\n\nA N has:\n- value: Int\n\nAn Out has:\n- result: Int\n\nFunction: identity.\n\nThe function needs:\n- n: Int\n\nThe function produces:\n- result: Int\n\nThe function executes:\n- the function returns local.n\n\nAction: Run.\n\nWhen Run happens:\n- the system computes out.result as n.value\n- the system calls identity with n = n.value into out.result\n- the system returns out.result\n- the system records a trace event named Calculated\n";
        let document = parse_ail_package_spec_text(&package, source).unwrap();
        elaborate_ail_core(&package, &document)
    }

    #[test]
    fn action_expression_cannot_read_a_function_local_via_core() {
        let mut core = core_fixture();
        assert!(check(&core).is_empty());
        let local_id = core
            .graph
            .nodes
            .iter()
            .find(|n| n.attributes.get("local_name").is_some_and(|v| v == "n"))
            .unwrap()
            .id
            .clone();
        let expression_id = core
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "IntegerExpression" && n.name == "Run.write0")
            .unwrap()
            .id
            .clone();
        core.graph
            .edges
            .iter_mut()
            .find(|e| e.kind == "expression_field" && e.source == expression_id)
            .unwrap()
            .target = local_id;
        assert!(
            !check(&core).is_empty(),
            "action expression borrowed a function's local storage"
        );
    }

    #[test]
    fn unsupported_profiles_cannot_silently_discard_integer_functions() {
        for profile in ["Compiler", "AgentTool", "UI", "System"] {
            let mut core = core_fixture();
            core.package.profile = profile.into();
            assert!(
                !check(&core).is_empty(),
                "{profile} accepted integer functions without a lowering path"
            );
        }
    }

    #[test]
    fn compiler_pass_cannot_silently_discard_value_call_steps() {
        let mut core = core_fixture();
        core.graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Action" && n.name == "Run")
            .unwrap()
            .attributes
            .insert("kind".into(), "CompilerPass".into());
        assert!(
            !check(&core).is_empty(),
            "compiler pass accepted an action value call"
        );
    }

    #[test]
    fn function_expression_cannot_borrow_another_functions_local() {
        let initial = core_fixture();
        let mut document = ail_document_from_core(&initial);
        let mut other = document.functions["identity"].clone();
        other.name = "other".into();
        other.label = "other".into();
        document.functions.insert(other.name.clone(), other);
        let package = load_ail_package_dir("bench/R003-max/program.ail").unwrap();
        let mut core = elaborate_ail_core(&package, &document);
        assert!(check(&core).is_empty());
        let other_local = core
            .graph
            .nodes
            .iter()
            .find(|n| {
                n.kind == "Field"
                    && n.attributes
                        .get("function_owner")
                        .is_some_and(|v| v == "other")
            })
            .unwrap()
            .id
            .clone();
        let expression = core
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "IntegerExpression" && n.name == "identity.step0.value")
            .unwrap()
            .id
            .clone();
        core.graph
            .edges
            .iter_mut()
            .find(|e| e.source == expression && e.kind == "expression_field")
            .unwrap()
            .target = other_local;
        assert!(!check(&core).is_empty());
        assert!(!check(&parse_ail_core_text(&render_ail_core(&core)).unwrap()).is_empty());
    }

    #[test]
    fn saved_core_retains_duplicate_function_edges_for_rejection() {
        for kind in [
            "function_step",
            "function_parameter",
            "function_expression",
            "call_target",
            "call_destination",
            "call_argument",
        ] {
            let mut core = core_fixture();
            let duplicate = core
                .graph
                .edges
                .iter()
                .find(|e| e.kind == kind)
                .unwrap()
                .clone();
            core.graph.edges.push(duplicate);
            assert!(!check(&core).is_empty(), "duplicate {kind} was accepted");
            let loaded = parse_ail_core_text(&render_ail_core(&core)).unwrap();
            assert!(
                !check(&loaded).is_empty(),
                "saved Core sanitized duplicate {kind}"
            );
        }
    }

    #[test]
    fn function_step_indices_must_be_present_and_contiguous() {
        for index in [None, Some("1"), Some("-1"), Some("4096")] {
            let mut core = core_fixture();
            let step = core
                .graph
                .edges
                .iter_mut()
                .find(|e| e.kind == "function_step")
                .unwrap();
            step.attributes.remove("step_index");
            if let Some(index) = index {
                step.attributes.insert("step_index".into(), index.into());
            }
            assert!(
                !check(&core).is_empty(),
                "invalid function index {index:?} was accepted"
            );
        }
    }

    #[test]
    fn calls_preserve_named_expression_arguments() {
        let call = parse_call("sum with z = local.n - 1; a = 5 into local.result").unwrap();
        assert_eq!(call.target, "sum");
        assert_eq!(call.key, "local.result");
        assert_eq!(encode_arguments(&call.arguments), "a=i:5;z=f:local.n i:1 -");
        assert_eq!(
            decode_arguments("a=i:5;z=f:local.n i:1 -").unwrap(),
            call.arguments
        );
        assert!(parse_call("sum with n = 1; n = 2 into local.result").is_err());
        assert!(decode_arguments("n=i:1;n=i:2").is_err());
    }

    #[test]
    fn executable_bodies_reject_descriptive_statements() {
        let mut function = AilIntegerFunction::default();
        assert!(
            parse_bullet(
                &mut function,
                "the function computes local.n as local.n + 1"
            )
            .is_ok()
        );
        assert!(parse_bullet(&mut function, "the function returns local.n").is_ok());
        assert!(parse_bullet(&mut function, "the function changes global state").is_err());
        assert_eq!(function.body.len(), 2);
    }
}
