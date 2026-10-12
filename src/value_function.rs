//! Typed executable functions with explicitly owned local Core operands.
use super::*;
use crate::condition::{IntComparison, IntCondition};
use crate::expression::IntExpr;
use value_protocol::{OperandKind, Signature};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueOperation {
    Variant(AilValueOperation),
    Literal(AilLiteralAssignment),
    Bytes(AilBytesOperation),
    Text(AilTextOperation),
    List(AilListOperation),
    Json(AilJsonConversion),
    Access(AilJsonAccess),
    Mutation(AilJsonMutation),
    Float(AilFloatOperation),
    Boolean(AilBooleanOperation),
}
impl ValueOperation {
    pub(super) fn source(&self) -> String {
        let source = match self {
            Self::Variant(op) => op.source(),
            Self::Literal(op) => op.source(),
            Self::Bytes(op) => op.source(),
            Self::Text(op) => op.source(),
            Self::List(op) => op.source(),
            Self::Json(op) => op.source(),
            Self::Access(op) => op.source(),
            Self::Mutation(op) => op.source(),
            Self::Float(op) => op.source(),
            Self::Boolean(op) => op.source(),
        };
        source.replacen("the system ", "the function ", 1)
    }
    fn instruction(&self, document: &AilDocument) -> Result<AilBytecodeInstruction, String> {
        match self {
            Self::Variant(op) => value_operations::instruction(op, document),
            Self::Literal(op) => literal_assignment::instruction(op, document),
            Self::Bytes(op) => {
                bytes_operations::check_source(op, document)?;
                bytes_operations::instruction(op)
            }
            Self::Text(op) => {
                text_operations::check_source(op, document)?;
                text_operations::instruction(op)
            }
            Self::List(op) => list_operations::instruction(op, document),
            Self::Json(op) => json_codec::instruction(op, document),
            Self::Access(op) => structured_json::instruction(op, document),
            Self::Mutation(op) => json_mutation::instruction(op, document),
            Self::Float(op) => float_operations::instruction(op, document),
            Self::Boolean(op) => bool_operations::instruction(op, document),
        }
        .map_err(error)
    }
}
fn error(message: impl Display) -> String {
    format!("AIL-FUNCTION-001: {message}")
}
pub(super) fn parse_operation(text: &str) -> Option<Result<ValueOperation, String>> {
    let text = format!("the system {}", text.strip_prefix("the function ")?);
    value_operations::parse_bullet(&text)
        .map(|r| r.map(ValueOperation::Variant))
        .or_else(|| literal_assignment::parse_bullet(&text).map(|r| r.map(ValueOperation::Literal)))
        .or_else(|| list_operations::parse_bullet(&text).map(|r| r.map(ValueOperation::List)))
        .or_else(|| bytes_operations::parse_bullet(&text).map(|r| r.map(ValueOperation::Bytes)))
        .or_else(|| text_operations::parse_bullet(&text).map(|r| r.map(ValueOperation::Text)))
        .or_else(|| json_codec::parse_bullet(&text).map(|r| r.map(ValueOperation::Json)))
        .or_else(|| structured_json::parse_bullet(&text).map(|r| r.map(ValueOperation::Access)))
        .or_else(|| json_mutation::parse_bullet(&text).map(|r| r.map(ValueOperation::Mutation)))
        .or_else(|| float_operations::parse_bullet(&text).map(|r| r.map(ValueOperation::Float)))
        .or_else(|| bool_operations::parse_bullet(&text).map(|r| r.map(ValueOperation::Boolean)))
}
pub(super) fn classify(document: &AilDocument) -> BTreeSet<String> {
    let mut general: BTreeSet<String> = document
        .functions
        .values()
        .filter_map(|f| {
            let body = f.integer.as_ref()?;
            (f.inputs
                .values()
                .chain(f.outputs.values())
                .any(|v| v.type_name != "Int")
                || body.locals.values().any(|t| t != "Int")
                || body.body.iter().any(|s| {
                    matches!(
                        s,
                        IntegerStep::ValueOperation(_)
                            | IntegerStep::Control(
                                AilControl::IfFloat(_)
                                    | AilControl::WhileFloat(_)
                                    | AilControl::IfBoolean(_)
                                    | AilControl::WhileBoolean(_)
                            )
                    )
                }))
            .then(|| f.name.clone())
        })
        .collect();
    loop {
        let callers: Vec<_> = document
            .functions
            .values()
            .filter(|f| !general.contains(&f.name))
            .filter(|f| {
                f.integer.as_ref().is_some_and(|body| {
                    body.body.iter().any(
                        |s| matches!(s, IntegerStep::Call(call) if general.contains(&call.target)),
                    )
                })
            })
            .map(|f| f.name.clone())
            .collect();
        if callers.is_empty() {
            break;
        }
        general.extend(callers);
    }
    general
}
fn source_signature(function: &AilFunction, document: &AilDocument) -> Result<Signature, String> {
    let body = function
        .integer
        .as_ref()
        .ok_or_else(|| error("call target is not executable"))?;
    if function.inputs.len() + body.locals.len() > 256 || body.body.len() > 4096 {
        return Err(error("function exceeds variable or body step limit"));
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
    if function.outputs.len() != 1 {
        return Err(error("function requires exactly one typed output"));
    }
    for (name, value) in function.inputs.iter().chain(&function.outputs) {
        if name != &value.name
            || !value_protocol::identifier(name)
            || !(value_protocol::supported(&value.type_name)
                || value_types::supported(&value_types::from_document(document)?, &value.type_name))
        {
            return Err(error(
                "function parameters and result require supported named types",
            ));
        }
    }
    for (name, kind) in &body.locals {
        if !value_protocol::identifier(name)
            || !(value_protocol::supported(kind)
                || value_types::supported(&value_types::from_document(document)?, kind))
            || function.inputs.contains_key(name)
        {
            return Err(error(
                "function locals must be distinct supported named values",
            ));
        }
    }
    Ok(Signature {
        parameters: function
            .inputs
            .iter()
            .map(|(n, v)| (n.clone(), v.type_name.clone()))
            .collect(),
        locals: body.locals.clone(),
        result: function.outputs.values().next().unwrap().type_name.clone(),
    })
}
fn source_signatures(document: &AilDocument) -> Result<BTreeMap<String, Signature>, String> {
    document
        .functions
        .values()
        .filter(|f| f.integer.is_some())
        .map(|f| source_signature(f, document).map(|s| (f.name.clone(), s)))
        .collect()
}
fn local_document(signature: &Signature, original: &AilDocument) -> AilDocument {
    let fields = signature
        .parameters
        .iter()
        .chain(&signature.locals)
        .map(|(name, kind)| {
            (
                name.clone(),
                AilField {
                    name: name.clone(),
                    type_name: kind.clone(),
                    is_secret: false,
                    provenance: String::new(),
                },
            )
        })
        .collect();
    let mut document = original.clone();
    document.things.clear();
    document.things.insert(
        "Local".into(),
        AilThing {
            name: "Local".into(),
            fields,
            provenance: String::new(),
        },
    );
    document
}
fn call_instruction(call: &AilIntegerCall, general: &BTreeSet<String>) -> AilBytecodeInstruction {
    AilBytecodeInstruction::new(
        if general.contains(&call.target) {
            "CALL_VALUE"
        } else {
            "CALL_INT"
        },
        &[
            ("target", call.target.clone()),
            (
                "arguments",
                integer_function::encode_arguments(&call.arguments),
            ),
            ("key", call.key.clone()),
        ],
    )
}
pub(super) fn action_call_instruction(
    call: &AilIntegerCall,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let instruction = call_instruction(call, &classify(document));
    let fields = document
        .things
        .values()
        .flat_map(|t| {
            t.fields
                .values()
                .map(|f| (runtime_field_key(&t.name, &f.name), f.type_name.clone()))
        })
        .collect();
    value_protocol::validate_call(&instruction, Some(&fields), &source_signatures(document)?)
        .map_err(error)?;
    Ok(instruction)
}
pub(super) fn compile(
    function: &AilFunction,
    document: &AilDocument,
) -> Result<AilBytecodeAction, String> {
    let signature = source_signature(function, document)?;
    let schemas = value_types::from_document(document)?;
    let fields = signature.fields();
    let signatures = source_signatures(document)?;
    let general = classify(document);
    let scope = local_document(&signature, document);
    let mut instructions = vec![AilBytecodeInstruction::new(
        "VALUE_FUNCTION",
        &[
            (
                "parameters",
                value_protocol::encode_types(&signature.parameters),
            ),
            ("locals", value_protocol::encode_types(&signature.locals)),
            ("result", signature.result.clone()),
        ],
    )];
    let mut controls = control_flow::Emitter::default();
    for step in &function.integer.as_ref().unwrap().body {
        match step {
            IntegerStep::Compute { key, expression } => {
                instructions.push(AilBytecodeInstruction::new(
                    "EVAL_INT",
                    &[
                        ("key", key.clone()),
                        ("expression", expression.to_postfix()),
                    ],
                ))
            }
            IntegerStep::Call(call) => instructions.push(call_instruction(call, &general)),
            IntegerStep::Return(expression) => instructions.push(AilBytecodeInstruction::new(
                "RETURN_VALUE",
                &[
                    ("expression", expression.to_postfix()),
                    ("type", signature.result.clone()),
                ],
            )),
            IntegerStep::Control(control) => {
                controls.emit(control, &mut instructions).map_err(error)?
            }
            IntegerStep::Trace(event) => instructions.push(AilBytecodeInstruction::new(
                "EMIT_TRACE",
                &[("event", event.clone())],
            )),
            IntegerStep::ValueOperation(op) => instructions.push(op.instruction(&scope)?),
            IntegerStep::Invalid(message) => return Err(error(message)),
        }
    }
    controls.finish().map_err(error)?;
    for instruction in &instructions[1..] {
        value_protocol::validate_body_instruction(
            instruction,
            &fields,
            &signature.result,
            &signatures,
            &schemas,
        )
        .map_err(error)?;
    }
    let action = AilBytecodeAction {
        name: function.name.clone(),
        instructions,
    };
    recovery::check_raise_termination(&action).map_err(error)?;
    Ok(action)
}

// Graph data operands always use explicit roles. Graph::add_edge merges edges
// by endpoints, so append here when two roles may reference the same storage.
fn edge(
    graph: &mut Graph,
    kind: &str,
    source: &Node,
    target: &Node,
    attributes: BTreeMap<String, String>,
) {
    let mut e = Edge::new(kind, source, target, attributes);
    e.ordinal = graph.edges.len();
    graph.edges.push(e);
}
fn node(
    graph: &mut Graph,
    kind: &str,
    name: &str,
    type_name: Option<String>,
    attributes: BTreeMap<String, String>,
) -> Node {
    let n = graph.add_node(kind, name, type_name, attributes);
    attach_provenance(graph, &n, format!("value:{name}"));
    n
}
pub(super) fn is_executable(node: &Node) -> bool {
    node.kind == "Function"
        && node
            .attributes
            .get("executable_value")
            .is_some_and(|v| v == "true")
}
fn field_operand(graph: &mut Graph, owner: &Node, step: &Node, role: &str, key: &str, local: bool) {
    let target = integer_function::destination(graph, owner, key, local);
    edge(
        graph,
        "value_field_operand",
        step,
        &target,
        attr(&[("name", role)]),
    );
}
fn expression_operand(
    graph: &mut Graph,
    owner: &Node,
    step: &Node,
    role: &str,
    expression: &IntExpr,
    local: bool,
) {
    let root = integer_function::build_expression(
        graph,
        owner,
        &format!("{}.operand.{role}", step.name),
        expression,
        local,
    );
    edge(
        graph,
        "value_expression_operand",
        step,
        &root,
        attr(&[("name", role)]),
    );
}
fn elaborate_instruction(
    graph: &mut Graph,
    owner: &Node,
    step: &Node,
    instruction: &AilBytecodeInstruction,
) -> Result<(), String> {
    let schema = value_protocol::operand_schema(instruction, &AilValueSchemas::new())?;
    for (name, kind) in &schema {
        match kind {
            OperandKind::Field(_) => {
                field_operand(graph, owner, step, name, &instruction.operands[name], true)
            }
            OperandKind::BooleanExpression => {
                let expression =
                    crate::bool_expression::BoolExpr::from_postfix(&instruction.operands[name])?;
                let root = bool_operations::build_expression(
                    graph,
                    owner,
                    &format!("{}.operand.{name}", step.name),
                    &expression,
                    true,
                );
                edge(
                    graph,
                    "value_expression_operand",
                    step,
                    &root,
                    attr(&[("name", name)]),
                );
            }
            OperandKind::FloatExpression => {
                let expression =
                    crate::float_expression::FloatExpr::from_postfix(&instruction.operands[name])?;
                let root = float_operations::build_expression(
                    graph,
                    owner,
                    &format!("{}.operand.{name}", step.name),
                    &expression,
                    true,
                );
                edge(
                    graph,
                    "value_expression_operand",
                    step,
                    &root,
                    attr(&[("name", name)]),
                );
            }
            OperandKind::Expression => expression_operand(
                graph,
                owner,
                step,
                name,
                &IntExpr::from_postfix(&instruction.operands[name])?,
                true,
            ),
        }
    }
    let stored = graph.nodes.iter_mut().find(|n| n.id == step.id).unwrap();
    stored
        .attributes
        .insert("opcode".into(), instruction.opcode.clone());
    for (name, value) in &instruction.operands {
        if !schema.contains_key(name) {
            stored.attributes.insert(name.clone(), value.clone());
        }
    }
    Ok(())
}
fn elaborate_call(
    graph: &mut Graph,
    document: &AilDocument,
    owner: &Node,
    step: &Node,
    call: &AilIntegerCall,
    local: bool,
) {
    let target = graph
        .find_node("Function", &call.target)
        .cloned()
        .unwrap_or_else(|| {
            node(
                graph,
                "InvalidFunctionTarget",
                &call.target,
                None,
                BTreeMap::new(),
            )
        });
    edge(graph, "value_call_target", step, &target, BTreeMap::new());
    field_operand(graph, owner, step, "key", &call.key, local);
    for (name, expression) in &call.arguments {
        let kind = document
            .functions
            .get(&call.target)
            .and_then(|f| f.inputs.get(name))
            .map(|v| v.type_name.as_str());
        let target = if kind.is_some_and(|t| t != "Int") {
            match expression {
                IntExpr::Field(key) => integer_function::destination(graph, owner, key, local),
                _ => node(
                    graph,
                    "InvalidFunctionArgument",
                    &format!("{}.argument.{name}", step.name),
                    None,
                    BTreeMap::new(),
                ),
            }
        } else {
            integer_function::build_expression(
                graph,
                owner,
                &format!("{}.argument.{name}", step.name),
                expression,
                local,
            )
        };
        edge(
            graph,
            "value_call_argument",
            step,
            &target,
            attr(&[("name", name)]),
        );
    }
}
pub(super) fn elaborate_action_call(
    graph: &mut Graph,
    document: &AilDocument,
    owner: &Node,
    call: &AilIntegerCall,
    index: usize,
) {
    let step = node(
        graph,
        "ValueFunctionCall",
        &format!("{}.integer_call{index}", owner.name),
        None,
        BTreeMap::new(),
    );
    elaborate_call(graph, document, owner, &step, call, false);
    action_order::append(
        graph,
        "integer_call_step",
        owner,
        &step,
        BTreeMap::new(),
        index,
    );
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    let general = classify(document);
    for owner in graph
        .nodes
        .iter_mut()
        .filter(|n| n.kind == "Function" && general.contains(&n.name))
    {
        owner
            .attributes
            .insert("executable_value".into(), "true".into());
    }
    for function in document
        .functions
        .values()
        .filter(|f| general.contains(&f.name))
    {
        let Some(owner) = graph.find_node("Function", &function.name).cloned() else {
            continue;
        };
        let body = function.integer.as_ref().unwrap();
        for (name, kind, relation) in function
            .inputs
            .iter()
            .map(|(n, v)| (n, &v.type_name, "value_parameter"))
            .chain(body.locals.iter().map(|(n, t)| (n, t, "value_local")))
        {
            let field = node(
                graph,
                "Field",
                &format!("{}.$local.{name}", owner.name),
                Some(kind.clone()),
                attr(&[("function_owner", &owner.name), ("local_name", name)]),
            );
            edge(graph, relation, &owner, &field, BTreeMap::new());
        }
        let signature = source_signature(function, document);
        for (index, operation) in body.body.iter().enumerate() {
            let kind = match operation {
                IntegerStep::Compute { .. } => "compute",
                IntegerStep::Call(_) => "call",
                IntegerStep::Return(_) => "return",
                IntegerStep::Control(_) => "control",
                IntegerStep::Trace(_) => "trace",
                IntegerStep::ValueOperation(_) => "operation",
                IntegerStep::Invalid(_) => "invalid",
            };
            let step = node(
                graph,
                "ValueFunctionStep",
                &format!("{}.value_step{index}", owner.name),
                None,
                attr(&[("function_owner", &owner.name), ("step_kind", kind)]),
            );
            edge(
                graph,
                "value_function_step",
                &owner,
                &step,
                attr(&[("step_index", &index.to_string())]),
            );
            let result = (|| -> Result<(), String> {
                let signature = signature.as_ref().map_err(Clone::clone)?;
                match operation {
                    IntegerStep::Compute { key, expression } => elaborate_instruction(
                        graph,
                        &owner,
                        &step,
                        &AilBytecodeInstruction::new(
                            "EVAL_INT",
                            &[
                                ("key", key.clone()),
                                ("expression", expression.to_postfix()),
                            ],
                        ),
                    )?,
                    IntegerStep::ValueOperation(ValueOperation::Variant(op)) => {
                        value_operations::build(
                            graph,
                            &local_document(signature, document),
                            &owner,
                            &step,
                            op,
                            true,
                        )?
                    }
                    IntegerStep::ValueOperation(op) => elaborate_instruction(
                        graph,
                        &owner,
                        &step,
                        &op.instruction(&local_document(signature, document))?,
                    )?,
                    IntegerStep::Call(call) => {
                        elaborate_call(graph, document, &owner, &step, call, true)
                    }
                    IntegerStep::Return(expression) => {
                        graph
                            .nodes
                            .iter_mut()
                            .find(|n| n.id == step.id)
                            .unwrap()
                            .attributes
                            .insert("type".into(), signature.result.clone());
                        if signature.result == "Int" {
                            expression_operand(
                                graph,
                                &owner,
                                &step,
                                "expression",
                                expression,
                                true,
                            );
                        } else if let IntExpr::Field(key) = expression {
                            field_operand(graph, &owner, &step, "expression", key, true);
                        } else {
                            return Err(error("non-Int return requires a field"));
                        }
                    }
                    IntegerStep::Trace(event) => {
                        graph
                            .nodes
                            .iter_mut()
                            .find(|n| n.id == step.id)
                            .unwrap()
                            .attributes
                            .insert("event".into(), event.clone());
                    }
                    IntegerStep::Control(control) => {
                        let kind = match control {
                            AilControl::If(_) => "if",
                            AilControl::IfBoolean(_) => "if_boolean",
                            AilControl::WhileBoolean(_) => "while_boolean",
                            AilControl::IfFloat(_) => "if_float",
                            AilControl::WhileFloat(_) => "while_float",
                            AilControl::Else => "else",
                            AilControl::EndIf => "end_if",
                            AilControl::While(_) => "while",
                            AilControl::EndWhile => "end_while",
                            AilControl::Break => "break",
                            AilControl::Continue => "continue",
                            AilControl::Try => "try",
                            AilControl::Recover { .. } => "recover",
                            AilControl::EndTry => "end_try",
                            AilControl::Raise { .. } => "raise",
                            AilControl::Invalid(_) => "invalid",
                        };
                        graph
                            .nodes
                            .iter_mut()
                            .find(|n| n.id == step.id)
                            .unwrap()
                            .attributes
                            .insert("control_kind".into(), kind.into());
                        if let AilControl::Raise { category } = control {
                            graph
                                .nodes
                                .iter_mut()
                                .find(|n| n.id == step.id)
                                .unwrap()
                                .attributes
                                .insert("error_category".into(), category.clone());
                        }
                        if let AilControl::Recover { errors, key } = control {
                            recovery::mask(errors).map_err(error)?;
                            graph
                                .nodes
                                .iter_mut()
                                .find(|n| n.id == step.id)
                                .unwrap()
                                .attributes
                                .insert("recovery_errors".into(), errors.clone());
                            field_operand(graph, &owner, &step, "key", key, true);
                        }
                        if let Some(expression) = control.boolean_condition() {
                            let root = bool_operations::build_expression(
                                graph,
                                &owner,
                                &format!("{}.condition", step.name),
                                expression,
                                true,
                            );
                            edge(graph, "value_condition", &step, &root, BTreeMap::new());
                        }
                        if let Some(condition) = control.float_condition() {
                            let comparison = float_operations::build_condition(
                                graph,
                                &owner,
                                &format!("{}.condition", step.name),
                                condition,
                                true,
                            );
                            edge(
                                graph,
                                "value_condition",
                                &step,
                                &comparison,
                                BTreeMap::new(),
                            );
                        }
                        if let Some(condition) = control.condition() {
                            let comparison = node(
                                graph,
                                "IntegerComparison",
                                &format!("{}.condition", step.name),
                                Some("Bool".into()),
                                attr(&[
                                    ("function_owner", &owner.name),
                                    ("comparison", condition.comparison.token()),
                                ]),
                            );
                            for (role, expression) in [
                                ("comparison_left", &condition.left),
                                ("comparison_right", &condition.right),
                            ] {
                                let root = integer_function::build_expression(
                                    graph,
                                    &owner,
                                    &format!("{}.{}", comparison.name, role),
                                    expression,
                                    true,
                                );
                                edge(graph, role, &comparison, &root, BTreeMap::new());
                            }
                            edge(
                                graph,
                                "value_condition",
                                &step,
                                &comparison,
                                BTreeMap::new(),
                            );
                        }
                    }
                    IntegerStep::Invalid(message) => return Err(error(message)),
                }
                Ok(())
            })();
            if let Err(message) = result {
                graph
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == step.id)
                    .unwrap()
                    .attributes
                    .insert("error".into(), message);
            }
        }
    }
}
fn by_id<'a>(core: &'a AilCore, id: &str) -> Result<&'a Node, String> {
    core.graph
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or_else(|| error("missing value function endpoint"))
}
fn outgoing<'a>(core: &'a AilCore, node: &Node, kind: &str) -> Vec<&'a Edge> {
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind == kind)
        .collect()
}
fn exact_attributes(node: &Node, names: &[&str]) -> Result<(), String> {
    if node.attributes.len() != names.len()
        || names.iter().any(|n| !node.attributes.contains_key(*n))
    {
        Err(error(
            "value function node requires exactly its defined attributes",
        ))
    } else {
        Ok(())
    }
}
fn only_edges(core: &AilCore, node: &Node, kinds: &[&str]) -> Result<(), String> {
    if core.graph.edges.iter().any(|e| {
        e.source == node.id && e.kind != "has_provenance" && !kinds.contains(&e.kind.as_str())
    }) {
        Err(error("unexpected edge on value function node"))
    } else {
        Ok(())
    }
}
fn child<'a>(core: &'a AilCore, node: &Node, kind: &str) -> Result<&'a Node, String> {
    let edges = outgoing(core, node, kind);
    if edges.len() != 1 || !edges[0].attributes.is_empty() {
        return Err(error(
            "value function requires one unannotated operand edge",
        ));
    }
    by_id(core, &edges[0].target)
}
pub(super) fn scoped_field(
    core: &AilCore,
    field: &Node,
    owner: Option<&Node>,
    expected: &str,
) -> Result<String, String> {
    if field.kind != "Field"
        || field.type_name.as_deref() != Some(expected)
        || !(value_protocol::supported(expected)
            || value_types::supported(&value_types::from_core(core)?, expected))
    {
        return Err(error("operand field type differs from its required type"));
    }
    if let Some(owner) = owner {
        exact_attributes(field, &["function_owner", "local_name"])?;
        let name = &field.attributes["local_name"];
        if field.attributes["function_owner"] != owner.name || !value_protocol::identifier(name) {
            return Err(error("local operand belongs to another function"));
        }
        let edges: Vec<_> = core
            .graph
            .edges
            .iter()
            .filter(|e| {
                e.target == field.id
                    && matches!(
                        e.kind.as_str(),
                        "value_parameter" | "value_local" | "function_parameter" | "function_local"
                    )
            })
            .collect();
        if edges.len() != 1
            || edges[0].source != owner.id
            || !matches!(edges[0].kind.as_str(), "value_parameter" | "value_local")
            || !edges[0].attributes.is_empty()
        {
            return Err(error(
                "local field requires one declaration in its owning function",
            ));
        }
    } else if field.attributes.contains_key("function_owner")
        || field.attributes.contains_key("local_name")
    {
        return Err(error("action call cannot access function local storage"));
    }
    let key = expression_core::field_key(field);
    if !matches!(IntExpr::parse(&key),Ok(IntExpr::Field(parsed)) if parsed==key) {
        return Err(error("invalid operand field name"));
    }
    if owner.is_none() {
        let declarations: Vec<_> = core
            .graph
            .edges
            .iter()
            .filter(|e| e.kind == "has_field" && e.target == field.id)
            .collect();
        if declarations.len() != 1 || !declarations[0].attributes.is_empty() {
            return Err(error(
                "action operand requires exactly one global storage declaration",
            ));
        }
        let thing = by_id(core, &declarations[0].source)?;
        let field_name = field.name.strip_prefix(&format!("{}.", thing.name));
        if thing.kind != "Thing"
            || field_name.is_none_or(|name| name.is_empty() || name.contains('.'))
        {
            return Err(error("action operand must be declared by its owning Thing"));
        }
        // Runtime storage is addressed by normalized keys. A second Core Field
        // with that key could otherwise hide the real declaration's secrecy or
        // type while reading or writing the same runtime value.
        if core
            .graph
            .nodes
            .iter()
            .filter(|n| {
                n.kind == "Field"
                    && !n.attributes.contains_key("function_owner")
                    && !n.attributes.contains_key("local_name")
                    && expression_core::field_key(n) == key
            })
            .count()
            != 1
        {
            return Err(error(
                "action operand runtime key has competing field declarations",
            ));
        }
    }
    Ok(key)
}
pub(super) fn scoped_expression(
    core: &AilCore,
    root: &Node,
    owner: Option<&Node>,
) -> Result<IntExpr, String> {
    let expression = expression_core::decode_root(core, &root.id).map_err(error)?;
    let mut pending = vec![root];
    let mut seen = BTreeSet::new();
    while let Some(node) = pending.pop() {
        if !seen.insert(&node.id) {
            continue;
        }
        let kind = node
            .attributes
            .get("expression_kind")
            .map(String::as_str)
            .unwrap_or("");
        let (mut attrs, edges) = match kind {
            "literal" => (vec!["expression_kind", "value"], vec![]),
            "field" => (vec!["expression_kind"], vec!["expression_field"]),
            "binary" => (
                vec!["expression_kind", "operator"],
                vec!["left_operand", "right_operand"],
            ),
            _ => return Err(error("invalid expression kind")),
        };
        if let Some(owner) = owner {
            attrs.push("function_owner");
            if node.attributes.get("function_owner") != Some(&owner.name) {
                return Err(error("expression belongs to another function"));
            }
        }
        exact_attributes(node, &attrs)?;
        only_edges(core, node, &edges)?;
        for relation in edges {
            let target = child(core, node, relation)?;
            if relation == "expression_field" {
                scoped_field(core, target, owner, "Int")?;
            } else {
                pending.push(target);
            }
        }
    }
    Ok(expression)
}
fn metadata_values(
    core: &AilCore,
    owner: &Node,
    relation: &str,
    kind: &str,
) -> Result<BTreeMap<String, String>, String> {
    let mut values = BTreeMap::new();
    for edge in outgoing(core, owner, relation) {
        let node = by_id(core, &edge.target)?;
        let name = node
            .name
            .strip_prefix(&format!("{}.", owner.name))
            .filter(|s| value_protocol::identifier(s))
            .ok_or_else(|| error("function value metadata is not owned"))?;
        let type_name = node
            .type_name
            .as_deref()
            .filter(|t| {
                value_protocol::supported(t)
                    || value_types::from_core(core).is_ok_and(|s| value_types::supported(&s, t))
            })
            .ok_or_else(|| error("function value has unsupported type"))?;
        if node.kind != kind
            || !node.attributes.is_empty()
            || !edge.attributes.is_empty()
            || values.insert(name.into(), type_name.into()).is_some()
        {
            return Err(error(
                "function metadata requires distinct typed declarations",
            ));
        }
        if core
            .graph
            .edges
            .iter()
            .filter(|e| e.target == node.id && e.kind == relation)
            .count()
            != 1
        {
            return Err(error("function metadata has multiple owners"));
        }
    }
    Ok(values)
}
fn graph_signature(core: &AilCore, owner: &Node) -> Result<Signature, String> {
    let general = is_executable(owner);
    if !(general
        || owner.kind == "Function"
            && owner
                .attributes
                .get("executable_integer")
                .is_some_and(|v| v == "true"))
    {
        return Err(error("call target must be an executable function"));
    }
    let parameters = metadata_values(core, owner, "has_input", "Input")?;
    let outputs = metadata_values(core, owner, "has_output", "Output")?;
    if outputs.len() != 1 {
        return Err(error("function requires exactly one output"));
    }
    let result = outputs.values().next().unwrap().clone();
    if !general {
        if result != "Int" || parameters.values().any(|t| t != "Int") {
            return Err(error("integer target has a non-Int signature"));
        }
        return Ok(Signature {
            parameters,
            locals: BTreeMap::new(),
            result,
        });
    }
    let mut declared = BTreeMap::new();
    let mut locals = BTreeMap::new();
    for relation in ["value_parameter", "value_local"] {
        for edge in outgoing(core, owner, relation) {
            let field = by_id(core, &edge.target)?;
            let kind = field.type_name.as_deref().unwrap_or("");
            scoped_field(core, field, Some(owner), kind)?;
            let name = field.attributes["local_name"].clone();
            let target = if relation == "value_parameter" {
                &mut declared
            } else {
                &mut locals
            };
            if target.insert(name, kind.into()).is_some() {
                return Err(error("duplicate local storage declaration"));
            }
        }
    }
    if declared != parameters
        || parameters.len() + locals.len() > 256
        || locals.keys().any(|n| parameters.contains_key(n))
    {
        return Err(error(
            "parameter metadata and local storage must match distinct declarations",
        ));
    }
    Ok(Signature {
        parameters,
        locals,
        result,
    })
}
fn role<'a>(
    core: &'a AilCore,
    node: &Node,
    relation: &str,
    name: &str,
) -> Result<&'a Node, String> {
    let edges: Vec<_> = outgoing(core, node, relation)
        .into_iter()
        .filter(|e| e.attributes.get("name").is_some_and(|n| n == name))
        .collect();
    if edges.len() != 1 || edges[0].attributes.len() != 1 {
        return Err(error("named operand must have exactly one role edge"));
    }
    by_id(core, &edges[0].target)
}
fn decode_call(
    core: &AilCore,
    node: &Node,
    owner: Option<&Node>,
) -> Result<AilIntegerCall, String> {
    only_edges(
        core,
        node,
        &[
            "value_call_target",
            "value_field_operand",
            "value_call_argument",
        ],
    )?;
    let target = child(core, node, "value_call_target")?;
    if owner.is_none() && !is_executable(target) {
        return Err(error("general action call must target a value function"));
    }
    let signature = graph_signature(core, target)?;
    if outgoing(core, node, "value_field_operand").len() != 1 {
        return Err(error("call requires exactly one destination"));
    }
    let key = scoped_field(
        core,
        role(core, node, "value_field_operand", "key")?,
        owner,
        &signature.result,
    )?;
    let mut arguments = BTreeMap::new();
    for edge in outgoing(core, node, "value_call_argument") {
        let name = edge
            .attributes
            .get("name")
            .filter(|_| edge.attributes.len() == 1)
            .ok_or_else(|| error("call argument requires exactly a name"))?;
        let kind = signature
            .parameters
            .get(name)
            .ok_or_else(|| error("unknown call argument"))?;
        let target = by_id(core, &edge.target)?;
        let expression = if kind == "Int" {
            scoped_expression(core, target, owner)?
        } else {
            IntExpr::Field(scoped_field(core, target, owner, kind)?)
        };
        if arguments.insert(name.clone(), expression).is_some() {
            return Err(error("duplicate call argument"));
        }
    }
    if arguments.keys().ne(signature.parameters.keys()) {
        return Err(error("call argument names do not match parameters"));
    }
    Ok(AilIntegerCall {
        target: target.name.clone(),
        arguments,
        key,
    })
}
pub(super) fn decode_action_call(core: &AilCore, node: &Node) -> Result<AilIntegerCall, String> {
    if node.kind != "ValueFunctionCall" || node.type_name.is_some() {
        return Err(error("expected untyped ValueFunctionCall node"));
    }
    exact_attributes(node, &[])?;
    decode_call(core, node, None)
}
fn decode_instruction(
    core: &AilCore,
    node: &Node,
    owner: &Node,
) -> Result<AilBytecodeInstruction, String> {
    only_edges(
        core,
        node,
        &["value_field_operand", "value_expression_operand"],
    )?;
    let opcode = node
        .attributes
        .get("opcode")
        .cloned()
        .ok_or_else(|| error("local operation has no opcode"))?;
    if opcode.starts_with("BYTES_") {
        bytes_operations::validate_local_operation(core, node, owner)?;
    }
    let mut operands: BTreeMap<String, String> = node
        .attributes
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "function_owner" | "step_kind" | "opcode"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for edge in core.graph.edges.iter().filter(|e| {
        e.source == node.id
            && matches!(
                e.kind.as_str(),
                "value_field_operand" | "value_expression_operand"
            )
    }) {
        let name = edge
            .attributes
            .get("name")
            .filter(|_| edge.attributes.len() == 1)
            .ok_or_else(|| error("operation operand requires exactly a role"))?;
        let target = by_id(core, &edge.target)?;
        let value = if edge.kind == "value_field_operand" {
            scoped_field(
                core,
                target,
                Some(owner),
                target.type_name.as_deref().unwrap_or(""),
            )?
        } else if opcode == "LIST_SLICE" && matches!(name.as_str(), "start" | "end") {
            list_operations::decode_slice_bound(core, target, owner, name)?.to_postfix()
        } else if matches!(opcode.as_str(), "BYTES_GET" | "BYTES_SET" | "BYTES_SLICE")
            && matches!(name.as_str(), "index" | "value" | "start" | "end")
        {
            bytes_operations::decode_local_expression(core, target, owner, node, name)?.to_postfix()
        } else if target.kind == "BooleanExpression" {
            bool_operations::decode_expression(core, target, Some(owner))?.to_postfix()
        } else if target.kind == "FloatExpression" {
            float_operations::decode_expression(core, target, Some(owner))?.to_postfix()
        } else {
            scoped_expression(core, target, Some(owner))?.to_postfix()
        };
        if operands.insert(name.clone(), value).is_some() {
            return Err(error(
                "duplicate operation operand or operand hidden in metadata",
            ));
        }
    }
    let instruction = AilBytecodeInstruction { opcode, operands };
    // Variant steps are dispatched separately by decode_step; these operation
    // contracts contain only builtins and do not need the custom registry.
    let schema =
        value_protocol::operand_schema(&instruction, &AilValueSchemas::new()).map_err(error)?;
    for (name, kind) in schema {
        match kind {
            OperandKind::Field(kind) => {
                scoped_field(
                    core,
                    role(core, node, "value_field_operand", &name)?,
                    Some(owner),
                    &kind,
                )?;
            }
            OperandKind::BooleanExpression => {
                bool_operations::decode_expression(
                    core,
                    role(core, node, "value_expression_operand", &name)?,
                    Some(owner),
                )?;
            }
            OperandKind::FloatExpression => {
                float_operations::decode_expression(
                    core,
                    role(core, node, "value_expression_operand", &name)?,
                    Some(owner),
                )?;
            }
            OperandKind::Expression => {
                scoped_expression(
                    core,
                    role(core, node, "value_expression_operand", &name)?,
                    Some(owner),
                )?;
            }
        }
    }
    Ok(instruction)
}
fn operation(instruction: &AilBytecodeInstruction) -> Result<ValueOperation, String> {
    let o = &instruction.operands;
    let key = o
        .get("key")
        .cloned()
        .ok_or_else(|| error("local operation has no destination"))?;
    let source = || o["source"].clone();
    let index = || IntExpr::from_postfix(&o["index"]);
    let value = || {
        if o["type"] == "Int" {
            IntExpr::from_postfix(&o["value"]).map(AilListValue::Integer)
        } else if o["type"] == "Float" {
            crate::float_expression::FloatExpr::from_postfix(&o["value"])
                .map(AilListValue::Floating)
        } else {
            Ok(AilListValue::Text(o["value"].clone()))
        }
    };
    Ok(match instruction.opcode.as_str() {
        "EVAL_BOOL" => ValueOperation::Boolean(AilBooleanOperation::Compute {
            key,
            expression: crate::bool_expression::BoolExpr::from_postfix(&o["expression"])?,
        }),
        "EVAL_FLOAT" => ValueOperation::Float(AilFloatOperation::Compute {
            key,
            expression: crate::float_expression::FloatExpr::from_postfix(&o["expression"])?,
        }),
        "INT_TO_FLOAT" => ValueOperation::Float(AilFloatOperation::IntToFloat {
            key,
            source: source(),
        }),
        "FLOAT_TO_INT" => ValueOperation::Float(AilFloatOperation::FloatToInt {
            key,
            source: source(),
        }),
        "ASSIGN_LITERAL" => ValueOperation::Literal(AilLiteralAssignment::Assign {
            key,
            literal: o["value"].clone(),
        }),
        "BYTES_LENGTH" => ValueOperation::Bytes(AilBytesOperation::Length {
            key,
            source: o["source"].clone(),
        }),
        "BYTES_GET" => ValueOperation::Bytes(AilBytesOperation::Get {
            key,
            source: o["source"].clone(),
            index: IntExpr::from_postfix(&o["index"])?,
        }),
        "BYTES_SLICE" => ValueOperation::Bytes(AilBytesOperation::Slice {
            key,
            source: o["source"].clone(),
            start: IntExpr::from_postfix(&o["start"])?,
            end: IntExpr::from_postfix(&o["end"])?,
        }),
        "BYTES_CONCAT" => ValueOperation::Bytes(AilBytesOperation::Concat {
            key,
            left: o["left"].clone(),
            right: o["right"].clone(),
        }),
        "BYTES_SET" => ValueOperation::Bytes(AilBytesOperation::Set {
            key,
            source: o["source"].clone(),
            index: IntExpr::from_postfix(&o["index"])?,
            value: IntExpr::from_postfix(&o["value"])?,
        }),
        "BYTES_FROM_UTF8" => ValueOperation::Bytes(AilBytesOperation::FromUtf8 {
            key,
            source: o["source"].clone(),
        }),
        "BYTES_TO_UTF8" => ValueOperation::Bytes(AilBytesOperation::ToUtf8 {
            key,
            source: o["source"].clone(),
        }),
        "BYTES_FROM_HEX" => ValueOperation::Bytes(AilBytesOperation::FromHex {
            key,
            source: o["source"].clone(),
        }),
        "BYTES_TO_HEX" => ValueOperation::Bytes(AilBytesOperation::ToHex {
            key,
            source: o["source"].clone(),
        }),
        "BYTES_COMPARE" => ValueOperation::Bytes(AilBytesOperation::Compare {
            key,
            left: o["left"].clone(),
            right: o["right"].clone(),
        }),
        "TEXT_SPLIT" => ValueOperation::Text(AilTextOperation::Split {
            key,
            source: source(),
            separator: o["separator"].clone(),
        }),
        "TEXT_JOIN_LIST" => ValueOperation::Text(AilTextOperation::JoinList {
            key,
            source: source(),
            separator: o["separator"].clone(),
        }),
        "TEXT_REPLACE" => ValueOperation::Text(AilTextOperation::Replace {
            key,
            source: source(),
            needle: o["needle"].clone(),
            replacement: o["replacement"].clone(),
        }),
        "TEXT_TRIM" => ValueOperation::Text(AilTextOperation::Trim {
            key,
            source: source(),
        }),
        "TEXT_JOIN" => ValueOperation::Text(AilTextOperation::Join {
            key,
            left: o["left"].clone(),
            right: o["right"].clone(),
        }),
        "TEXT_LENGTH" => ValueOperation::Text(AilTextOperation::Length {
            key,
            source: source(),
        }),
        "TEXT_SLICE" => ValueOperation::Text(AilTextOperation::Slice {
            key,
            source: source(),
            start: IntExpr::from_postfix(&o["start"])?,
            end: IntExpr::from_postfix(&o["end"])?,
        }),
        "TEXT_FIND" => ValueOperation::Text(AilTextOperation::Find {
            key,
            source: source(),
            needle: o["needle"].clone(),
        }),
        "LIST_CONCAT" => ValueOperation::List(AilListOperation::Concat {
            key,
            source: source(),
            other: o["other"].clone(),
        }),
        "LIST_SLICE" => ValueOperation::List(AilListOperation::Slice {
            key,
            source: source(),
            start: IntExpr::from_postfix(&o["start"])?,
            end: IntExpr::from_postfix(&o["end"])?,
        }),
        "LIST_REVERSE" => ValueOperation::List(AilListOperation::Reverse {
            key,
            source: source(),
        }),
        "LIST_SORT" => ValueOperation::List(AilListOperation::Sort {
            key,
            source: source(),
            descending: o["order"] == "descending",
        }),
        "LIST_EMPTY" => ValueOperation::List(AilListOperation::Empty { key }),
        "LIST_LENGTH" => ValueOperation::List(AilListOperation::Length {
            key,
            source: source(),
        }),
        "LIST_GET" => ValueOperation::List(AilListOperation::Get {
            key,
            source: source(),
            index: index()?,
        }),
        "LIST_APPEND" => ValueOperation::List(AilListOperation::Append {
            key,
            source: source(),
            value: value()?,
        }),
        "LIST_SET" => ValueOperation::List(AilListOperation::Set {
            key,
            source: source(),
            index: index()?,
            value: value()?,
        }),
        "JSON_PARSE" => ValueOperation::Json(AilJsonConversion::Parse {
            key,
            source: source(),
        }),
        "JSON_FORMAT" => ValueOperation::Json(AilJsonConversion::Format {
            key,
            source: source(),
        }),
        "JSON_GET" => ValueOperation::Access(AilJsonAccess::Get {
            key,
            source: source(),
            path: o["path"].clone(),
        }),
        "JSON_COUNT" => ValueOperation::Access(AilJsonAccess::Count {
            key,
            source: source(),
        }),
        "JSON_KIND" => ValueOperation::Access(AilJsonAccess::Kind {
            key,
            source: source(),
        }),
        "JSON_KEYS" => ValueOperation::Access(AilJsonAccess::Keys {
            key,
            source: source(),
        }),
        "JSON_SET" => ValueOperation::Mutation(AilJsonMutation::Set {
            key,
            source: source(),
            path: o["path"].clone(),
            value: o["value"].clone(),
        }),
        "JSON_REMOVE" => ValueOperation::Mutation(AilJsonMutation::Remove {
            key,
            source: source(),
            path: o["path"].clone(),
        }),
        _ => return Err(error("unsupported function-local operation")),
    })
}
fn decode_condition(core: &AilCore, node: &Node, owner: &Node) -> Result<IntCondition, String> {
    let comparison = child(core, node, "value_condition")?;
    if comparison.kind != "IntegerComparison"
        || comparison.type_name.as_deref() != Some("Bool")
        || comparison.attributes.get("function_owner") != Some(&owner.name)
    {
        return Err(error("condition must be an owned integer comparison"));
    }
    exact_attributes(comparison, &["function_owner", "comparison"])?;
    only_edges(core, comparison, &["comparison_left", "comparison_right"])?;
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
    let comparison = IntComparison::from_token(&comparison.attributes["comparison"])?;
    Ok(IntCondition {
        left,
        right,
        comparison,
    })
}
fn decode_step(
    core: &AilCore,
    node: &Node,
    owner: &Node,
    result: &str,
) -> Result<IntegerStep, String> {
    if node
        .attributes
        .get("control_kind")
        .is_some_and(|kind| kind == "raise")
    {
        control_flow::validate_raise_node(core, node, Some(owner)).map_err(error)?;
    }
    if node.kind != "ValueFunctionStep"
        || node.type_name.is_some()
        || node.attributes.get("function_owner") != Some(&owner.name)
    {
        return Err(error("step must belong to its enclosing value function"));
    }
    let kind = node
        .attributes
        .get("step_kind")
        .map(String::as_str)
        .unwrap_or("");
    Ok(match kind {
        "compute" | "operation" => {
            if kind == "operation"
                && node.attributes.get("opcode").is_some_and(|o| {
                    matches!(o.as_str(), "VALUE_CONSTRUCT" | "VALUE_TEST" | "VALUE_GET")
                })
            {
                return Ok(IntegerStep::ValueOperation(ValueOperation::Variant(
                    value_operations::decode_core(
                        core,
                        node,
                        Some(owner),
                        &value_types::from_core(core)?,
                    )?,
                )));
            }
            let instruction = decode_instruction(core, node, owner)?;
            if kind == "compute" {
                if instruction.opcode != "EVAL_INT" {
                    return Err(error("compute step requires EVAL_INT"));
                }
                IntegerStep::Compute {
                    key: instruction.operands["key"].clone(),
                    expression: IntExpr::from_postfix(&instruction.operands["expression"])?,
                }
            } else {
                IntegerStep::ValueOperation(operation(&instruction)?)
            }
        }
        "call" => {
            exact_attributes(node, &["function_owner", "step_kind"])?;
            IntegerStep::Call(decode_call(core, node, Some(owner))?)
        }
        "return" => {
            exact_attributes(node, &["function_owner", "step_kind", "type"])?;
            if node.attributes["type"] != result {
                return Err(error("return type differs from output declaration"));
            }
            let relation = if result == "Int" {
                "value_expression_operand"
            } else {
                "value_field_operand"
            };
            only_edges(core, node, &[relation])?;
            if outgoing(core, node, relation).len() != 1 {
                return Err(error("return requires exactly one operand"));
            }
            let target = role(core, node, relation, "expression")?;
            IntegerStep::Return(if result == "Int" {
                scoped_expression(core, target, Some(owner))?
            } else {
                IntExpr::Field(scoped_field(core, target, Some(owner), result)?)
            })
        }
        "trace" => {
            exact_attributes(node, &["function_owner", "step_kind", "event"])?;
            only_edges(core, node, &[])?;
            if node.attributes["event"].is_empty() {
                return Err(error("trace requires a name"));
            }
            IntegerStep::Trace(node.attributes["event"].clone())
        }
        "control" => {
            let kind = node
                .attributes
                .get("control_kind")
                .map(String::as_str)
                .unwrap_or("");
            exact_attributes(
                node,
                if kind == "raise" {
                    &[
                        "function_owner",
                        "step_kind",
                        "control_kind",
                        "error_category",
                    ]
                } else if kind == "recover" {
                    &[
                        "function_owner",
                        "step_kind",
                        "control_kind",
                        "recovery_errors",
                    ]
                } else {
                    &["function_owner", "step_kind", "control_kind"]
                },
            )?;
            only_edges(
                core,
                node,
                if matches!(
                    kind,
                    "if" | "while" | "if_float" | "while_float" | "if_boolean" | "while_boolean"
                ) {
                    &["value_condition"]
                } else if kind == "recover" {
                    &["value_field_operand"]
                } else {
                    &[]
                },
            )?;
            IntegerStep::Control(match kind {
                "raise" => AilControl::Raise {
                    category: control_flow::validate_raise_node(core, node, Some(owner))
                        .map_err(error)?,
                },
                "try" => AilControl::Try,
                "end_try" => AilControl::EndTry,
                "recover" => {
                    let errors = &node.attributes["recovery_errors"];
                    recovery::mask(errors).map_err(error)?;
                    if outgoing(core, node, "value_field_operand").len() != 1 {
                        return Err(error("recovery requires one local destination"));
                    }
                    let key = scoped_field(
                        core,
                        role(core, node, "value_field_operand", "key")?,
                        Some(owner),
                        "Text",
                    )?;
                    AilControl::Recover {
                        errors: errors.clone(),
                        key,
                    }
                }
                "if_boolean" => AilControl::IfBoolean(bool_operations::decode_expression(
                    core,
                    child(core, node, "value_condition")?,
                    Some(owner),
                )?),
                "while_boolean" => AilControl::WhileBoolean(bool_operations::decode_expression(
                    core,
                    child(core, node, "value_condition")?,
                    Some(owner),
                )?),
                "if_float" => AilControl::IfFloat(float_operations::decode_condition(
                    core,
                    child(core, node, "value_condition")?,
                    Some(owner),
                )?),
                "while_float" => AilControl::WhileFloat(float_operations::decode_condition(
                    core,
                    child(core, node, "value_condition")?,
                    Some(owner),
                )?),
                "if" => AilControl::If(decode_condition(core, node, owner)?),
                "while" => AilControl::While(decode_condition(core, node, owner)?),
                "else" => AilControl::Else,
                "end_if" => AilControl::EndIf,
                "end_while" => AilControl::EndWhile,
                "break" => AilControl::Break,
                "continue" => AilControl::Continue,
                _ => return Err(error("unknown control kind")),
            })
        }
        _ => return Err(error("invalid value function step")),
    })
}
fn decode_function(core: &AilCore, owner: &Node) -> Result<AilIntegerFunction, String> {
    if !is_executable(owner) || owner.type_name.is_some() {
        return Err(error("expected untyped executable value Function"));
    }
    exact_attributes(owner, &["label", "executable_value"])?;
    only_edges(
        core,
        owner,
        &[
            "has_input",
            "has_output",
            "value_parameter",
            "value_local",
            "value_function_step",
            "has_termination_bound",
            "has_termination_measure",
        ],
    )?;
    let signature = graph_signature(core, owner)?;
    let mut steps = BTreeMap::new();
    for edge in outgoing(core, owner, "value_function_step") {
        let index = edge
            .attributes
            .get("step_index")
            .filter(|_| edge.attributes.len() == 1)
            .and_then(|v| v.parse::<usize>().ok())
            .ok_or_else(|| error("step requires exactly one valid index"))?;
        if index >= 4096 || steps.insert(index, by_id(core, &edge.target)?).is_some() {
            return Err(error("duplicate or excessive body step"));
        }
    }
    let mut body = Vec::new();
    for (expected, (index, step)) in steps.into_iter().enumerate() {
        if index != expected {
            return Err(error("body step indices must be contiguous"));
        }
        body.push(decode_step(core, step, owner, &signature.result)?);
    }
    Ok(AilIntegerFunction {
        locals: signature.locals,
        body,
    })
}
pub(super) fn from_core_function(core: &AilCore, node: &Node) -> Option<AilIntegerFunction> {
    if !is_executable(node) {
        return None;
    }
    Some(
        decode_function(core, node).unwrap_or_else(|message| AilIntegerFunction {
            locals: BTreeMap::new(),
            body: vec![IntegerStep::Invalid(message)],
        }),
    )
}
fn executable_action(node: &Node) -> bool {
    node.kind == "Action"
        && node
            .attributes
            .get("kind")
            .is_none_or(|v| v != "CompilerPass")
}
fn step_owner<'a>(core: &'a AilCore, node: &Node, relation: &str) -> Result<&'a Node, String> {
    let incoming: Vec<_> = core
        .graph
        .edges
        .iter()
        .filter(|e| e.target == node.id)
        .collect();
    if incoming.len() != 1 || incoming[0].kind != relation {
        return Err(error("function step requires exactly one owner edge"));
    }
    by_id(core, &incoming[0].source)
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let mut diagnostics = Vec::new();
    for node in &core.graph.nodes {
        let result = (|| -> Result<(), String> {
            if node.kind == "Function"
                && node.attributes.contains_key("executable_value")
                && !is_executable(node)
            {
                return Err(error("invalid value executable marker"));
            }
            if (is_executable(node)
                || matches!(
                    node.kind.as_str(),
                    "ValueFunctionStep" | "ValueFunctionCall"
                ))
                && !matches!(core.package.profile.as_str(), "Application" | "C interop")
            {
                return Err(error(
                    "value functions require Application or C interop profile",
                ));
            }
            match node.kind.as_str() {
                "Function" if is_executable(node) => {
                    decode_function(core, node)?;
                }
                "ValueFunctionStep" => {
                    let owner = step_owner(core, node, "value_function_step")?;
                    if !is_executable(owner) {
                        return Err(error(
                            "value step owner must be an executable value function",
                        ));
                    }
                    decode_step(core, node, owner, &graph_signature(core, owner)?.result)?;
                }
                "ValueFunctionCall" => {
                    if !executable_action(step_owner(core, node, "integer_call_step")?) {
                        return Err(error("value call owner must be an executable action"));
                    }
                    decode_action_call(core, node)?;
                }
                "Field" => {
                    if let Some(owner) = node
                        .attributes
                        .get("function_owner")
                        .and_then(|name| core.graph.find_node("Function", name))
                        .filter(|n| is_executable(n))
                    {
                        scoped_field(
                            core,
                            node,
                            Some(owner),
                            node.type_name.as_deref().unwrap_or(""),
                        )?;
                    }
                }
                _ => {}
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
        let source = by_id(core, &edge.source).ok();
        let target = by_id(core, &edge.target).ok();
        let step = source.is_some_and(|n| n.kind == "ValueFunctionStep");
        let call = source.is_some_and(|n| {
            n.kind == "ValueFunctionCall"
                || n.kind == "ValueFunctionStep"
                    && n.attributes.get("step_kind").is_some_and(|v| v == "call")
        });
        let named = || {
            edge.attributes.len() == 1
                && edge
                    .attributes
                    .get("name")
                    .is_some_and(|v| value_protocol::identifier(v))
        };
        let valid = match edge.kind.as_str() {
            "value_parameter" | "value_local" => {
                source.is_some_and(is_executable)
                    && target.is_some_and(|n| n.kind == "Field")
                    && edge.attributes.is_empty()
            }
            "value_function_step" => {
                source.is_some_and(is_executable)
                    && target.is_some_and(|n| n.kind == "ValueFunctionStep")
                    && edge.attributes.len() == 1
                    && edge
                        .attributes
                        .get("step_index")
                        .is_some_and(|v| v.parse::<usize>().is_ok())
            }
            "value_call_target" => {
                call && target.is_some_and(|n| {
                    is_executable(n)
                        || n.kind == "Function"
                            && n.attributes
                                .get("executable_integer")
                                .is_some_and(|v| v == "true")
                }) && edge.attributes.is_empty()
            }
            "value_call_argument" => {
                call && target
                    .is_some_and(|n| matches!(n.kind.as_str(), "Field" | "IntegerExpression"))
                    && named()
            }
            "value_field_operand" => {
                (step || call) && target.is_some_and(|n| n.kind == "Field") && named()
            }
            "value_expression_operand" => {
                step && target.is_some_and(|n| {
                    matches!(
                        n.kind.as_str(),
                        "IntegerExpression" | "FloatExpression" | "BooleanExpression"
                    )
                }) && named()
            }
            "value_condition" => {
                step && target.is_some_and(|n| {
                    matches!(
                        n.kind.as_str(),
                        "IntegerComparison" | "FloatComparison" | "BooleanExpression"
                    )
                }) && edge.attributes.is_empty()
            }
            "integer_call_step" if target.is_some_and(|n| n.kind == "ValueFunctionCall") => {
                source.is_some_and(executable_action)
                    && edge
                        .attributes
                        .get("step_index")
                        .is_some_and(|v| v.parse::<usize>().is_ok())
                    && edge.attributes.iter().all(|(k, v)| {
                        matches!(k.as_str(), "step_index" | "operation_index")
                            && v.parse::<usize>().is_ok()
                    })
            }
            _ => true,
        };
        if !valid {
            diagnostics.push(
                AilDiagnostic::error(
                    "AIL-FUNCTION-001",
                    format!("invalid {} value function edge", edge.kind),
                )
                .with_affected_graph_item(format!("edge:{}", edge.id)),
            );
        }
    }
    // Recompute the source classification instead of trusting a caller-supplied
    // executable marker. This also checks closure promotion and typed bodies.
    if diagnostics.is_empty() {
        let document = ail_document_from_core(core);
        let general = classify(&document);
        for node in core.graph.nodes.iter().filter(|n| n.kind == "Function") {
            let marked = is_executable(node);
            if marked != general.contains(&node.name) {
                diagnostics.push(
                    AilDiagnostic::error(
                        "AIL-FUNCTION-001",
                        "executable marker differs from canonical function classification",
                    )
                    .with_affected_graph_item(format!("node:{}", node.id)),
                );
            } else if marked
                && let Some(function) = document.functions.get(&node.name)
                && let Err(message) = compile(function, &document)
            {
                diagnostics.push(
                    AilDiagnostic::error("AIL-FUNCTION-001", message)
                        .with_affected_graph_item(format!("node:{}", node.id)),
                );
            }
        }
    }
    diagnostics
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let mut result = Vec::new();
    for step in core
        .graph
        .edges
        .iter()
        .filter(|e| e.kind == "integer_call_step")
    {
        if !by_id(core, &step.target).is_ok_and(|n| n.kind == "ValueFunctionCall") {
            continue;
        }
        for edge in core.graph.edges.iter().filter(|e| e.source == step.target) {
            if kind == "writes"
                && edge.kind == "value_field_operand"
                && edge.attributes.get("name").is_some_and(|v| v == "key")
            {
                result.push((edge, step.source.as_str()));
            }
            if kind != "reads" || edge.kind != "value_call_argument" {
                continue;
            }
            if by_id(core, &edge.target).is_ok_and(|n| n.kind == "Field") {
                result.push((edge, step.source.as_str()));
                continue;
            }
            let mut pending = vec![edge.target.as_str()];
            let mut seen = BTreeSet::new();
            while let Some(id) = pending.pop() {
                if !seen.insert(id) {
                    continue;
                }
                for operand in core.graph.edges.iter().filter(|e| e.source == id) {
                    match operand.kind.as_str() {
                        "expression_field" => result.push((operand, step.source.as_str())),
                        "left_operand" | "right_operand" => pending.push(operand.target.as_str()),
                        _ => {}
                    }
                }
            }
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> (AilPackage, AilDocument) {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Values manages values.\n\nA Data has:\n- number: Int\n- text: Text\n\nAn Output has:\n- number: Int\n- text: Text\n\nFunction: plain.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function executes:\n- the function returns local.n\n\nFunction: promoted.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function stores:\n- result: Int\nThe function executes:\n- the function calls general with n = local.n into local.result\n- the function returns local.result\n\nFunction: general.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function stores:\n- text: Text\n- result: Int\nThe function executes:\n- the function calls plain with n = local.n into local.result\n- the function returns local.result\n\nFunction: echo.\nThe function needs:\n- text: Text\nThe function produces:\n- result: Text\nThe function executes:\n- the function returns local.text\n\nAction: Run.\nWhen Run happens:\n- the system calls plain with n = data.number into output.number\n- the system calls echo with text = data.text into output.text\n- the system calls promoted with n = data.number into output.number\n- the system records a trace event named Values\n";
        let document = parse_ail_package_spec_text(&package, source).unwrap();
        (package, document)
    }
    #[test]
    fn classification_promotes_callers_and_preserves_the_integer_protocol() {
        let (_, document) = document();
        assert_eq!(
            classify(&document),
            BTreeSet::from(["general".into(), "promoted".into(), "echo".into()])
        );
        assert_eq!(
            integer_function::compile(&document.functions["plain"], &document)
                .unwrap()
                .instructions[0]
                .opcode,
            "INTEGER_FUNCTION"
        );
        let general = integer_function::compile(&document.functions["general"], &document).unwrap();
        assert_eq!(general.instructions[0].opcode, "VALUE_FUNCTION");
        assert_eq!(general.instructions[1].opcode, "CALL_INT");
        assert_eq!(general.instructions[2].opcode, "RETURN_VALUE");
        assert_eq!(
            integer_function::compile(&document.functions["promoted"], &document)
                .unwrap()
                .instructions[1]
                .opcode,
            "CALL_VALUE"
        );
        let call = &document.actions["Run"].integer_calls[1];
        assert_eq!(
            action_call_instruction(call, &document).unwrap().opcode,
            "CALL_VALUE"
        );
    }
    #[test]
    fn local_operations_parse_lower_and_reject_global_or_wrong_typed_fields() {
        let (_, mut document) = document();
        let function = document.functions.get_mut("echo").unwrap();
        function
            .integer
            .as_mut()
            .unwrap()
            .locals
            .insert("copy".into(), "Text".into());
        integer_function::parse_bullet(
            function.integer.as_mut().unwrap(),
            "the function joins text local.text and local.text into local.copy",
        )
        .unwrap();
        let lowered = integer_function::compile(&document.functions["echo"], &document).unwrap();
        assert!(lowered.instructions.iter().any(|i| i.opcode == "TEXT_JOIN"));
        let mut invalid = document.clone();
        integer_function::parse_bullet(
            invalid
                .functions
                .get_mut("echo")
                .unwrap()
                .integer
                .as_mut()
                .unwrap(),
            "the function joins text data.text and local.text into local.copy",
        )
        .unwrap();
        assert!(integer_function::compile(&invalid.functions["echo"], &invalid).is_err());
        let mut body = AilIntegerFunction::default();
        assert!(
            integer_function::parse_bullet(
                &mut body,
                "the system joins text local.text and local.text into local.copy"
            )
            .is_err()
        );
        assert!(
            integer_function::parse_bullet(
                &mut body,
                "the function reads text from file input into local.text"
            )
            .is_err()
        );
    }
    #[test]
    fn core_roundtrip_keeps_mixed_call_order_and_general_markers() {
        let (package, document) = document();
        let original = elaborate_ail_core(&package, &document);
        assert_eq!(
            original
                .graph
                .nodes
                .iter()
                .filter(|n| is_executable(n))
                .count(),
            3
        );
        let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
        assert!(check(&saved).is_empty(), "{:?}", check(&saved));
        assert!(
            integer_function::check(&saved).is_empty(),
            "{:?}",
            integer_function::check(&saved)
        );
        let decoded = ail_document_from_core(&saved);
        assert_eq!(
            decoded.actions["Run"].integer_calls,
            document.actions["Run"].integer_calls
        );
        assert_eq!(classify(&decoded), classify(&document));
        for name in ["echo", "general", "promoted"] {
            assert_eq!(
                integer_function::compile(&decoded.functions[name], &decoded).unwrap(),
                integer_function::compile(&document.functions[name], &document).unwrap()
            );
        }
    }
    #[test]
    fn every_local_operation_roundtrips_with_typed_roles_and_shared_fields() {
        let (package, mut document) = document();
        let function = document.functions.get_mut("general").unwrap();
        let body = function.integer.as_mut().unwrap();
        body.locals.extend(
            [
                ("flag", "Bool"),
                ("items", "List<Int>"),
                ("names", "List<Text>"),
                ("json", "Json"),
            ]
            .map(|(n, t)| (n.into(), t.into())),
        );
        body.body.clear();
        for statement in [
            "the function assigns true to local.flag",
            "the function assigns {} to local.json",
            "the function assigns \"/name\" to local.text",
            "the function joins text local.text and local.text into local.text",
            "the function counts characters in local.text into local.result",
            "the function slices text local.text from 0 to local.n into local.text",
            "the function finds text local.text in local.text into local.result",
            "the function creates an empty list into local.items",
            "the function creates an empty list into local.names",
            "the function counts items in list local.names into local.result",
            "the function appends integer local.n to list local.items into local.items",
            "the function appends text local.text to list local.names into local.names",
            "the function sets item 0 in list local.items to integer local.n into local.items",
            "the function sets item 0 in list local.names to text local.text into local.names",
            "the function gets item 0 from list local.items into local.result",
            "the function gets item 0 from list local.names into local.text",
            "the function parses JSON text local.text into local.json",
            "the function formats local.json as JSON text into local.text",
            "the function gets JSON value at path local.text from local.json into local.flag",
            "the function counts entries in JSON local.json into local.result",
            "the function sets JSON value at path local.text in local.json to local.flag into local.json",
            "the function removes JSON value at path local.text from local.json into local.json",
            "the function returns local.n",
        ] {
            integer_function::parse_bullet(body, statement)
                .unwrap_or_else(|e| panic!("{statement}: {e}"));
        }
        let original =
            integer_function::compile(&document.functions["general"], &document).unwrap();
        let core = elaborate_ail_core(&package, &document);
        let core = parse_ail_core_text(&render_ail_core(&core)).unwrap();
        assert!(check(&core).is_empty(), "{:?}", check(&core));
        let restored = ail_document_from_core(&core);
        assert_eq!(
            integer_function::compile(&restored.functions["general"], &restored).unwrap(),
            original
        );
        let rendered =
            integer_function::render(&restored.functions["general"].integer.clone().unwrap());
        assert!(
            rendered
                .iter()
                .any(|s| s.contains("joins text local.text and local.text into local.text"))
        );
    }
    #[test]
    fn checked_core_rejects_foreign_locals_duplicate_roles_and_ignored_metadata() {
        let (package, document) = document();
        let original = elaborate_ail_core(&package, &document);
        let rejected =
            |core: &AilCore| assert!(!check(core).is_empty(), "forged Core was accepted");
        for relation in [
            "value_parameter",
            "value_local",
            "value_function_step",
            "value_field_operand",
            "value_expression_operand",
            "value_call_argument",
            "value_call_target",
        ] {
            let mut core = original.clone();
            let duplicate = core
                .graph
                .edges
                .iter()
                .find(|e| e.kind == relation)
                .unwrap()
                .clone();
            core.graph.edges.push(duplicate);
            rejected(&core);
        }
        for kind in ["ValueFunctionStep", "ValueFunctionCall"] {
            let mut core = original.clone();
            core.graph
                .nodes
                .iter_mut()
                .find(|n| n.kind == kind)
                .unwrap()
                .attributes
                .insert("ignored".into(), "true".into());
            rejected(&core);
        }
        let mut core = original.clone();
        core.graph
            .nodes
            .iter_mut()
            .find(|n| {
                n.kind == "Field"
                    && n.attributes
                        .get("function_owner")
                        .is_some_and(|v| v == "echo")
            })
            .unwrap()
            .attributes
            .insert("function_owner".into(), "general".into());
        rejected(&core);
        let mut core = original.clone();
        core.graph
            .nodes
            .iter_mut()
            .find(|n| {
                n.kind == "IntegerExpression"
                    && n.attributes
                        .get("function_owner")
                        .is_some_and(|v| v == "general")
            })
            .unwrap()
            .attributes
            .insert("ignored".into(), "true".into());
        rejected(&core);
        let mut core = original.clone();
        let id = core
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "ValueFunctionStep")
            .unwrap()
            .id
            .clone();
        core.graph.edges.retain(|e| e.target != id);
        rejected(&core);
        let mut core = original.clone();
        core.graph
            .edges
            .iter_mut()
            .find(|e| e.kind == "value_field_operand")
            .unwrap()
            .attributes
            .insert("type".into(), "Int".into());
        rejected(&core);
        let mut core = original.clone();
        core.graph
            .nodes
            .iter_mut()
            .find(|n| is_executable(n))
            .unwrap()
            .attributes
            .insert("executable_integer".into(), "true".into());
        rejected(&core);
    }
    #[test]
    fn canonical_general_marker_requires_a_real_trigger_or_promoted_callee() {
        let (package, document) = document();
        let mut core = elaborate_ail_core(&package, &document);
        let field = core
            .graph
            .nodes
            .iter()
            .find(|n| {
                n.kind == "Field"
                    && n.attributes
                        .get("function_owner")
                        .is_some_and(|v| v == "general")
                    && n.attributes.get("local_name").is_some_and(|v| v == "text")
            })
            .unwrap()
            .id
            .clone();
        core.graph
            .edges
            .retain(|e| e.source != field && e.target != field);
        core.graph.nodes.retain(|n| n.id != field);
        assert!(
            check(&core)
                .iter()
                .any(|d| d.message.contains("canonical function classification"))
        );
        let mut document = document.clone();
        let echo = document.functions.get_mut("echo").unwrap();
        echo.integer.as_mut().unwrap().body = vec![IntegerStep::Return(
            IntExpr::parse("local.text + 1").unwrap(),
        )];
        assert!(integer_function::compile(&document.functions["echo"], &document).is_err());
    }
    #[test]
    fn action_call_operands_require_real_unambiguous_global_declarations() {
        let (package, document) = document();
        let original = elaborate_ail_core(&package, &document);
        for (kind, owner_name, field_name) in [
            ("value_call_argument", "Run.integer_call1", "Data.text"),
            ("value_field_operand", "Run.integer_call1", "Output.text"),
            (
                "expression_field",
                "Run.integer_call2.argument.n",
                "Data.number",
            ),
        ] {
            let mut core = original.clone();
            let original_field = core
                .graph
                .nodes
                .iter()
                .find(|n| n.kind == "Field" && n.name == field_name)
                .unwrap()
                .clone();
            let mut forged = original_field.clone();
            forged.id = "forged-global".into();
            forged.attributes.clear();
            core.graph.nodes.push(forged.clone());
            let source = core
                .graph
                .nodes
                .iter()
                .find(|n| n.name == owner_name)
                .unwrap()
                .id
                .clone();
            core.graph
                .edges
                .iter_mut()
                .find(|e| e.kind == kind && e.source == source && e.target == original_field.id)
                .unwrap()
                .target = forged.id.clone();
            assert!(!check(&core).is_empty(), "orphan {kind} field accepted");
            let declaration = core
                .graph
                .edges
                .iter()
                .find(|e| e.kind == "has_field" && e.target == original_field.id)
                .unwrap()
                .clone();
            let mut duplicate = declaration;
            duplicate.target = forged.id;
            core.graph.edges.push(duplicate);
            assert!(
                !check(&core).is_empty(),
                "runtime-key shadow {kind} accepted"
            );
        }
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    fn fixture() -> (AilPackage, AilDocument) {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let source = "The application Recovery manages data.\n\nA Data has:\n- number: Int\n- error: Text\n\nFunction: plain.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function executes:\n- the function returns 10 / local.n\n\nFunction: guarded.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function stores:\n- result: Int\n- error: Text\nThe function executes:\n- try:\n- the function calls plain with n = local.n into local.result\n- the function returns local.result\n- recover DIVZERO into local.error:\n- the function returns -1\n- end try\n\nFunction: wrapper.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function stores:\n- result: Int\nThe function executes:\n- the function calls guarded with n = local.n into local.result\n- the function returns local.result\n\nAction: Run.\nWhen Run happens:\n- the system calls wrapper with n = data.number into data.number\n- the system records a trace event named Done\n";
        let document = parse_ail_package_spec_text(&package, source).unwrap();
        (package, document)
    }
    fn recover(core: &AilCore) -> String {
        core.graph
            .nodes
            .iter()
            .find(|n| {
                n.kind == "ValueFunctionStep"
                    && n.attributes
                        .get("control_kind")
                        .is_some_and(|v| v == "recover")
            })
            .unwrap()
            .id
            .clone()
    }
    fn rejects(core: &AilCore) {
        for core in [
            core.clone(),
            parse_ail_core_text(&render_ail_core(core)).unwrap(),
        ] {
            assert!(
                !check(&core).is_empty(),
                "malformed local recovery accepted"
            );
        }
    }
    #[test]
    fn recovery_functions_promote_callers_and_preserve_integer_callees_and_roundtrips() {
        let (package, document) = fixture();
        assert_eq!(
            classify(&document),
            BTreeSet::from(["guarded".into(), "wrapper".into()])
        );
        assert_eq!(
            integer_function::compile(&document.functions["plain"], &document)
                .unwrap()
                .instructions[0]
                .opcode,
            "INTEGER_FUNCTION"
        );
        let guarded = compile(&document.functions["guarded"], &document).unwrap();
        assert_eq!(guarded.instructions[0].opcode, "VALUE_FUNCTION");
        assert_eq!(guarded.instructions[1].opcode, "TRY_BEGIN");
        assert_eq!(guarded.instructions[1].operands["key"], "local.error");
        assert_eq!(guarded.instructions[2].opcode, "CALL_INT");
        let original = elaborate_ail_core(&package, &document);
        let rendered = render_ail_spec_from_core(&original);
        for core in [
            original.clone(),
            parse_ail_core_text(&render_ail_core(&original)).unwrap(),
            elaborate_ail_core(
                &package,
                &parse_ail_package_spec_text(&package, &rendered).unwrap(),
            ),
        ] {
            assert!(check(&core).is_empty(), "{:?}", check(&core));
            let step = core
                .graph
                .nodes
                .iter()
                .find(|n| n.id == recover(&core))
                .unwrap();
            assert_eq!(step.attributes.len(), 4);
            let function = core.graph.find_node("Function", "guarded").unwrap();
            assert_eq!(
                from_core_function(&core, function),
                document.functions["guarded"].integer
            );
        }
        let renamed = integer_function::namespace_function(
            document.functions["guarded"].integer.as_ref().unwrap(),
            "Child",
            &BTreeSet::new(),
        );
        assert!(renamed.body.iter().any(|step| matches!(step, IntegerStep::Control(AilControl::Recover { key, errors }) if key == "local.error" && errors == "DIVZERO")));
    }
    #[test]
    fn recovery_local_core_rejects_metadata_types_global_fields_and_ownership_forgeries() {
        let (package, document) = fixture();
        let original = elaborate_ail_core(&package, &document);
        for case in [
            "extra",
            "filter",
            "duplicate",
            "owner",
            "type",
            "global",
            "role",
            "foreign_parent",
        ] {
            let mut core = original.clone();
            let id = recover(&core);
            let destination = core
                .graph
                .edges
                .iter()
                .find(|e| e.source == id && e.kind == "value_field_operand")
                .unwrap()
                .target
                .clone();
            match case {
                "extra" => {
                    core.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == id)
                        .unwrap()
                        .attributes
                        .insert("ignored".into(), "true".into());
                }
                "filter" => {
                    core.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == id)
                        .unwrap()
                        .attributes
                        .insert("recovery_errors".into(), "any;INT".into());
                }
                "duplicate" | "foreign_parent" => {
                    let mut edge = core
                        .graph
                        .edges
                        .iter()
                        .find(|e| {
                            if case == "duplicate" {
                                e.source == id && e.kind == "value_field_operand"
                            } else {
                                e.target == id
                            }
                        })
                        .unwrap()
                        .clone();
                    edge.id = format!("extra-{case}");
                    edge.ordinal = core.graph.edges.len();
                    core.graph.edges.push(edge);
                }
                "owner" => {
                    core.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == destination)
                        .unwrap()
                        .attributes
                        .insert("function_owner".into(), "wrapper".into());
                }
                "type" => {
                    core.graph
                        .nodes
                        .iter_mut()
                        .find(|n| n.id == destination)
                        .unwrap()
                        .type_name = Some("Int".into())
                }
                "global" => {
                    let global = core
                        .graph
                        .nodes
                        .iter()
                        .find(|n| n.kind == "Field" && n.name == "Data.error")
                        .unwrap()
                        .id
                        .clone();
                    core.graph
                        .edges
                        .iter_mut()
                        .find(|e| e.source == id && e.kind == "value_field_operand")
                        .unwrap()
                        .target = global;
                }
                _ => {
                    core.graph
                        .edges
                        .iter_mut()
                        .find(|e| e.source == id && e.kind == "value_field_operand")
                        .unwrap()
                        .attributes
                        .insert("name".into(), "source".into());
                }
            }
            rejects(&core);
        }
    }
    #[test]
    fn recovery_function_destinations_require_declared_local_text() {
        let (_, document) = fixture();
        for key in ["data.error", "local.n", "local.missing"] {
            let mut bad = document.clone();
            for step in &mut bad
                .functions
                .get_mut("guarded")
                .unwrap()
                .integer
                .as_mut()
                .unwrap()
                .body
            {
                if let IntegerStep::Control(AilControl::Recover {
                    key: destination, ..
                }) = step
                {
                    *destination = key.into();
                }
            }
            assert!(
                integer_function::compile(&bad.functions["guarded"], &bad).is_err(),
                "{key}"
            );
        }
        let mut bad = document;
        bad.functions
            .get_mut("guarded")
            .unwrap()
            .integer
            .as_mut()
            .unwrap()
            .locals
            .remove("error");
        assert!(integer_function::compile(&bad.functions["guarded"], &bad).is_err());
    }
}
