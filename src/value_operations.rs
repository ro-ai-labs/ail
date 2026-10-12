//! Typed immutable construction and inspection of user-defined variants.
use super::*;
use crate::expression::IntExpr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AilValueOperation {
    Construct {
        type_name: String,
        variant: String,
        arguments: BTreeMap<String, IntExpr>,
        key: String,
    },
    Test {
        type_name: String,
        variant: String,
        source: String,
        key: String,
    },
    Get {
        type_name: String,
        variant: String,
        field: String,
        source: String,
        key: String,
    },
    Invalid(String),
}
fn error(message: impl Display) -> String {
    format!("AIL-VALUE-001: {message}")
}
impl AilValueOperation {
    pub(super) fn type_name(&self) -> &str {
        match self {
            Self::Construct { type_name, .. }
            | Self::Test { type_name, .. }
            | Self::Get { type_name, .. } => type_name,
            Self::Invalid(_) => "invalid",
        }
    }
    pub(super) fn source(&self) -> String {
        match self {
            Self::Construct {
                type_name,
                variant,
                arguments,
                key,
            } => format!(
                "the system constructs {type_name} variant {variant} with {} into {key}",
                if arguments.is_empty() {
                    "none".into()
                } else {
                    arguments
                        .iter()
                        .map(|(n, e)| format!("{n} = {}", e.to_source()))
                        .collect::<Vec<_>>()
                        .join("; ")
                }
            ),
            Self::Test {
                type_name,
                variant,
                source,
                key,
            } => format!("the system tests {type_name} variant {variant} of {source} into {key}"),
            Self::Get {
                type_name,
                variant,
                field,
                source,
                key,
            } => format!(
                "the system gets field {field} of {type_name} variant {variant} from {source} into {key}"
            ),
            Self::Invalid(e) => format!("invalid variant operation: {e}"),
        }
    }
}
pub(super) fn parse_bullet(text: &str) -> Option<Result<AilValueOperation, String>> {
    let text = trim_sentence(text);
    let parse = || -> Result<AilValueOperation, String> {
        if let Some(rest) = text.strip_prefix("the system constructs ") {
            let (ty, rest) = rest
                .split_once(" variant ")
                .ok_or_else(|| error("expected type and variant"))?;
            let call = integer_function::parse_call(rest).map_err(error)?;
            return Ok(AilValueOperation::Construct {
                type_name: value_types::parse(ty)?.to_string(),
                variant: call.target,
                arguments: call.arguments,
                key: call.key,
            });
        }
        let (rest, get) = if let Some(r) = text.strip_prefix("the system tests ") {
            (r, false)
        } else {
            (
                text.strip_prefix("the system gets field ")
                    .ok_or_else(|| error("invalid operation"))?,
                true,
            )
        };
        let (field, rest) = if get {
            let (f, r) = rest
                .split_once(" of ")
                .ok_or_else(|| error("expected field of type"))?;
            (Some(f), r)
        } else {
            (None, rest)
        };
        let (ty, rest) = rest
            .split_once(" variant ")
            .ok_or_else(|| error("expected type and variant"))?;
        let (variant, rest) = rest
            .split_once(if get { " from " } else { " of " })
            .ok_or_else(|| error("expected source field"))?;
        let (source, key) = rest
            .split_once(" into ")
            .ok_or_else(|| error("expected destination"))?;
        let type_name = value_types::parse(ty)?.to_string();
        Ok(if let Some(field) = field {
            AilValueOperation::Get {
                type_name,
                variant: variant.into(),
                field: field.into(),
                source: source.into(),
                key: key.into(),
            }
        } else {
            AilValueOperation::Test {
                type_name,
                variant: variant.into(),
                source: source.into(),
                key: key.into(),
            }
        })
    };
    (text.starts_with("the system constructs ")
        || text.starts_with("the system tests ")
        || text.starts_with("the system gets field "))
    .then(parse)
}
fn raw(op: &AilValueOperation) -> Result<AilBytecodeInstruction, String> {
    Ok(match op {
        AilValueOperation::Construct {
            type_name,
            variant,
            arguments,
            key,
        } => AilBytecodeInstruction::new(
            "VALUE_CONSTRUCT",
            &[
                ("type", type_name.clone()),
                ("variant", variant.clone()),
                ("arguments", integer_function::encode_arguments(arguments)),
                ("key", key.clone()),
            ],
        ),
        AilValueOperation::Test {
            type_name,
            variant,
            source,
            key,
        } => AilBytecodeInstruction::new(
            "VALUE_TEST",
            &[
                ("type", type_name.clone()),
                ("variant", variant.clone()),
                ("source", source.clone()),
                ("key", key.clone()),
            ],
        ),
        AilValueOperation::Get {
            type_name,
            variant,
            field,
            source,
            key,
        } => AilBytecodeInstruction::new(
            "VALUE_GET",
            &[
                ("type", type_name.clone()),
                ("variant", variant.clone()),
                ("field", field.clone()),
                ("source", source.clone()),
                ("key", key.clone()),
            ],
        ),
        AilValueOperation::Invalid(e) => return Err(error(e)),
    })
}
pub(super) fn decode(i: &AilBytecodeInstruction) -> Result<AilValueOperation, String> {
    let o = &i.operands;
    let type_name = o["type"].clone();
    let variant = o["variant"].clone();
    let key = o["key"].clone();
    Ok(match i.opcode.as_str() {
        "VALUE_CONSTRUCT" => AilValueOperation::Construct {
            type_name,
            variant,
            key,
            arguments: integer_function::decode_arguments(&o["arguments"])?,
        },
        "VALUE_TEST" => AilValueOperation::Test {
            type_name,
            variant,
            key,
            source: o["source"].clone(),
        },
        "VALUE_GET" => AilValueOperation::Get {
            type_name,
            variant,
            key,
            source: o["source"].clone(),
            field: o["field"].clone(),
        },
        _ => return Err(error("unknown variant operation")),
    })
}
pub(super) fn verify(i: &AilBytecodeInstruction, schemas: &AilValueSchemas) -> Result<(), String> {
    let names = ail_bytecode_required_operands(&i.opcode).ok_or_else(|| error("unknown opcode"))?;
    if i.operands.len() != names.len() || names.iter().any(|n| !i.operands.contains_key(*n)) {
        return Err(error("operation requires exactly its defined operands"));
    }
    let o = &i.operands;
    value_types::indices(
        schemas,
        &o["type"],
        &o["variant"],
        o.get("field").map(String::as_str),
    )?;
    for name in ["key", "source"] {
        if let Some(key) = o.get(name)
            && !matches!(IntExpr::parse(key),Ok(IntExpr::Field(k)) if k==*key)
        {
            return Err(error("invalid field operand"));
        }
    }
    if i.opcode == "VALUE_CONSTRUCT" {
        let args = integer_function::decode_arguments(&o["arguments"])?;
        let payloads = &schemas[&o["type"]][&o["variant"]];
        if args.keys().ne(payloads.keys())
            || integer_function::encode_arguments(&args) != o["arguments"]
        {
            return Err(error(
                "constructor arguments must be sorted, complete and unique",
            ));
        }
        for (name, e) in args {
            if payloads[&name] != "Int" && !matches!(e, IntExpr::Field(_)) {
                return Err(error("non-Int payload requires a field"));
            }
        }
    }
    Ok(())
}
pub(super) fn instruction(
    op: &AilValueOperation,
    document: &AilDocument,
) -> Result<AilBytecodeInstruction, String> {
    let i = raw(op)?;
    let schemas = value_types::from_document(document)?;
    verify(&i, &schemas)?;
    for (key, kind) in fields(&i, &schemas)? {
        if runtime_field_type_name(document, &key) != Some(kind.as_str()) {
            return Err(error("operand must have the exact declared type"));
        }
    }
    Ok(i)
}
pub(super) fn fields(
    i: &AilBytecodeInstruction,
    schemas: &AilValueSchemas,
) -> Result<BTreeMap<String, String>, String> {
    verify(i, schemas)?;
    let o = &i.operands;
    let ty = &o["type"];
    let payloads = &schemas[ty][&o["variant"]];
    let mut out = BTreeMap::new();
    let mut add = |key: String, kind: String| -> Result<(), String> {
        if out.insert(key, kind.clone()).is_some_and(|old| old != kind) {
            return Err(error("conflicting operand types"));
        }
        Ok(())
    };
    add(
        o["key"].clone(),
        match i.opcode.as_str() {
            "VALUE_CONSTRUCT" => ty.clone(),
            "VALUE_TEST" => "Bool".into(),
            _ => payloads[&o["field"]].clone(),
        },
    )?;
    if let Some(source) = o.get("source") {
        add(source.clone(), ty.clone())?;
    }
    if i.opcode == "VALUE_CONSTRUCT" {
        for (name, e) in integer_function::decode_arguments(&o["arguments"])? {
            for key in e.fields() {
                add(key, payloads[&name].clone())?;
            }
        }
    }
    Ok(out)
}
fn edge(graph: &mut Graph, kind: &str, from: &Node, to: &Node, attrs: BTreeMap<String, String>) {
    let mut e = Edge::new(kind, from, to, attrs);
    e.ordinal = graph.edges.len();
    graph.edges.push(e);
}
pub(super) fn build(
    graph: &mut Graph,
    document: &AilDocument,
    owner: &Node,
    node: &Node,
    op: &AilValueOperation,
    local: bool,
) -> Result<(), String> {
    let i = instruction(op, document)?;
    for (name, value) in &i.operands {
        if matches!(name.as_str(), "key" | "source") {
            let f = integer_function::destination(graph, owner, value, local);
            edge(graph, "variant_operand", node, &f, attr(&[("name", name)]));
        } else if name == "arguments" {
            let schemas = value_types::from_document(document)?;
            for (name, e) in integer_function::decode_arguments(value)? {
                let kind = &schemas[&i.operands["type"]][&i.operands["variant"]][&name];
                let f = if kind == "Int" {
                    integer_function::build_expression(
                        graph,
                        owner,
                        &format!("{}.argument.{name}", node.name),
                        &e,
                        local,
                    )
                } else if let IntExpr::Field(key) = e {
                    integer_function::destination(graph, owner, &key, local)
                } else {
                    return Err(error("non-Int payload requires field"));
                };
                edge(
                    graph,
                    "variant_argument",
                    node,
                    &f,
                    attr(&[("name", &name)]),
                );
            }
        } else {
            graph
                .nodes
                .iter_mut()
                .find(|n| n.id == node.id)
                .unwrap()
                .attributes
                .insert(name.clone(), value.clone());
        }
    }
    graph
        .nodes
        .iter_mut()
        .find(|n| n.id == node.id)
        .unwrap()
        .attributes
        .insert("opcode".into(), i.opcode);
    Ok(())
}
pub(super) fn elaborate(graph: &mut Graph, document: &AilDocument) {
    for a in document.actions.values() {
        let Some(owner) = graph.find_node("Action", &a.name).cloned() else {
            continue;
        };
        for (index, op) in a.value_operations.iter().enumerate() {
            let n = graph.add_node(
                "VariantOperation",
                format!("{}.variant{index}", a.name),
                None,
                BTreeMap::new(),
            );
            attach_provenance(graph, &n, format!("action:{}.variant:{index}", a.name));
            if let Err(e) = build(graph, document, &owner, &n, op, false) {
                graph
                    .nodes
                    .iter_mut()
                    .find(|v| v.id == n.id)
                    .unwrap()
                    .attributes
                    .insert("error".into(), e);
            }
            action_order::append(graph, "variant_step", &owner, &n, BTreeMap::new(), index);
        }
    }
}
pub(super) fn decode_core(
    core: &AilCore,
    node: &Node,
    owner: Option<&Node>,
    schemas: &AilValueSchemas,
) -> Result<AilValueOperation, String> {
    if node.type_name.is_some()
        || node.attributes.keys().any(|k| {
            !(matches!(k.as_str(), "opcode" | "type" | "variant" | "field")
                || owner.is_some() && matches!(k.as_str(), "function_owner" | "step_kind"))
        })
    {
        return Err(error("unexpected operation metadata"));
    }
    value_types::check_provenance(core, node)?;
    let opcode = node
        .attributes
        .get("opcode")
        .ok_or_else(|| error("missing operation kind"))?;
    let mut operands = node.attributes.clone();
    operands.remove("opcode");
    // Function step bookkeeping is owned and checked by value_function.
    for n in ["function_owner", "step_kind"] {
        operands.remove(n);
    }
    let mut args = BTreeMap::new();
    for e in core
        .graph
        .edges
        .iter()
        .filter(|e| e.source == node.id && e.kind != "has_provenance")
    {
        if !matches!(e.kind.as_str(), "variant_operand" | "variant_argument")
            || e.attributes.len() != 1
        {
            return Err(error("unexpected operation edge"));
        }
        let name = e
            .attributes
            .get("name")
            .ok_or_else(|| error("operand requires name"))?;
        let target = core
            .graph
            .nodes
            .iter()
            .find(|n| n.id == e.target)
            .ok_or_else(|| error("missing operand"))?;
        let mut pending = vec![target];
        let mut seen = BTreeSet::new();
        while let Some(operand) = pending.pop() {
            if !seen.insert(&operand.id) {
                continue;
            }
            value_types::check_provenance(core, operand)?;
            for child in core.graph.edges.iter().filter(|e| {
                e.source == operand.id
                    && matches!(
                        e.kind.as_str(),
                        "left_operand" | "right_operand" | "expression_field"
                    )
            }) {
                pending.push(
                    core.graph
                        .nodes
                        .iter()
                        .find(|n| n.id == child.target)
                        .ok_or_else(|| error("missing expression operand"))?,
                );
            }
        }
        if e.kind == "variant_operand" {
            if !matches!(name.as_str(), "key" | "source")
                || operands
                    .insert(
                        name.clone(),
                        value_function::scoped_field(
                            core,
                            target,
                            owner,
                            target.type_name.as_deref().unwrap_or(""),
                        )?,
                    )
                    .is_some()
            {
                return Err(error("duplicate field operand"));
            }
        } else {
            let kind = operands
                .get("type")
                .and_then(|t| schemas.get(t))
                .and_then(|v| operands.get("variant").and_then(|n| v.get(n)))
                .and_then(|f| f.get(name))
                .ok_or_else(|| error("unknown constructor argument"))?;
            let expr = if kind == "Int" {
                value_function::scoped_expression(core, target, owner)?
            } else {
                IntExpr::Field(value_function::scoped_field(core, target, owner, kind)?)
            };
            if args.insert(name.clone(), expr).is_some() {
                return Err(error("duplicate constructor argument"));
            }
        }
    }
    if opcode == "VALUE_CONSTRUCT" {
        if operands
            .insert(
                "arguments".into(),
                integer_function::encode_arguments(&args),
            )
            .is_some()
        {
            return Err(error("constructor arguments cannot be metadata"));
        }
    } else if !args.is_empty() {
        return Err(error("unexpected constructor arguments"));
    }
    let i = AilBytecodeInstruction {
        opcode: opcode.clone(),
        operands,
    };
    verify(&i, schemas)?;
    // Exact destination/source types are also checked, not merely supported types.
    for (key, kind) in fields(&i, schemas)? {
        let matches = core
            .graph
            .nodes
            .iter()
            .filter(|n| {
                n.kind == "Field"
                    && expression_core::field_key(n) == key
                    && (owner.is_none()
                        || n.attributes.get("function_owner") == owner.map(|o| &o.name))
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 || matches[0].type_name.as_deref() != Some(&kind) {
            return Err(error("operand field type mismatch"));
        }
    }
    decode(&i)
}
pub(super) fn from_core(core: &AilCore, owner: &Node) -> Vec<AilValueOperation> {
    let schemas = match value_types::from_core(core) {
        Ok(s) => s,
        Err(e) => return vec![AilValueOperation::Invalid(e)],
    };
    core.graph
        .edges
        .iter()
        .filter(|e| e.source == owner.id && e.kind == "variant_step")
        .filter_map(|e| core.graph.nodes.iter().find(|n| n.id == e.target))
        .map(|n| decode_core(core, n, None, &schemas).unwrap_or_else(AilValueOperation::Invalid))
        .collect()
}
pub(super) fn check(core: &AilCore) -> Vec<AilDiagnostic> {
    let check = || -> Result<(), String> {
        let schemas = value_types::from_core(core)?;
        for edge in core
            .graph
            .edges
            .iter()
            .filter(|e| matches!(e.kind.as_str(), "variant_operand" | "variant_argument"))
        {
            let source = core
                .graph
                .nodes
                .iter()
                .find(|n| n.id == edge.source)
                .ok_or_else(|| error("missing operand owner"))?;
            if !(source.kind == "VariantOperation"
                || source.kind == "ValueFunctionStep"
                    && source
                        .attributes
                        .get("step_kind")
                        .is_some_and(|s| s == "operation")
                    && source.attributes.get("opcode").is_some_and(|s| {
                        matches!(s.as_str(), "VALUE_CONSTRUCT" | "VALUE_TEST" | "VALUE_GET")
                    }))
            {
                return Err(error("orphan variant operand"));
            }
        }
        for node in core
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind == "VariantOperation")
        {
            let owners = core
                .graph
                .edges
                .iter()
                .filter(|e| e.target == node.id && e.kind == "variant_step")
                .collect::<Vec<_>>();
            if owners.len() != 1
                || !core
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == owners[0].source && n.kind == "Action")
                || !matches!(core.package.profile.as_str(), "Application" | "C interop")
            {
                return Err(error("operation requires one executable action owner"));
            }
            decode_core(core, node, None, &schemas)?;
        }
        Ok(())
    };
    check()
        .err()
        .map(|e| vec![AilDiagnostic::error("AIL-VALUE-001", e)])
        .unwrap_or_default()
}
pub(super) fn namespace(
    op: &AilValueOperation,
    alias: &str,
    declarations: &BTreeSet<String>,
    local: bool,
) -> AilValueOperation {
    let mut op = op.clone();
    let qualify = |t: &str| value_types::qualify(t, alias, declarations, &BTreeSet::new());
    let key = |k: &mut String| {
        if !local {
            *k = format!("{}.{}", runtime_subject_key(alias), k)
        }
    };
    match &mut op {
        AilValueOperation::Construct {
            type_name,
            arguments,
            key: k,
            ..
        } => {
            *type_name = qualify(type_name);
            key(k);
            if !local {
                for e in arguments.values_mut() {
                    *e = namespace_expr(e, alias);
                }
            }
        }
        AilValueOperation::Test {
            type_name,
            source,
            key: k,
            ..
        }
        | AilValueOperation::Get {
            type_name,
            source,
            key: k,
            ..
        } => {
            *type_name = qualify(type_name);
            key(source);
            key(k)
        }
        _ => {}
    }
    op
}
fn namespace_expr(e: &IntExpr, alias: &str) -> IntExpr {
    match e {
        IntExpr::Field(k) => IntExpr::Field(format!("{}.{}", runtime_subject_key(alias), k)),
        IntExpr::Literal(v) => IntExpr::Literal(*v),
        IntExpr::Binary { op, left, right } => IntExpr::Binary {
            op: *op,
            left: Box::new(namespace_expr(left, alias)),
            right: Box::new(namespace_expr(right, alias)),
        },
    }
}
pub(super) fn accesses<'a>(core: &'a AilCore, kind: &str) -> Vec<(&'a Edge, &'a str)> {
    let mut out = Vec::new();
    for step in core.graph.edges.iter().filter(|e| e.kind == "variant_step") {
        for e in core.graph.edges.iter().filter(|e| e.source == step.target) {
            if e.kind == "variant_operand"
                && e.attributes.get("name").is_some_and(|n| {
                    (kind == "writes" && n == "key") || (kind == "reads" && n == "source")
                })
            {
                out.push((e, step.source.as_str()));
            }
            if kind == "reads" && e.kind == "variant_argument" {
                if core
                    .graph
                    .nodes
                    .iter()
                    .any(|n| n.id == e.target && n.kind == "Field")
                {
                    out.push((e, step.source.as_str()));
                } else {
                    let mut pending = vec![e.target.as_str()];
                    let mut seen = BTreeSet::new();
                    while let Some(id) = pending.pop() {
                        if !seen.insert(id) {
                            continue;
                        }
                        for x in core.graph.edges.iter().filter(|e| e.source == id) {
                            if x.kind == "expression_field" {
                                out.push((x, step.source.as_str()));
                            } else if matches!(x.kind.as_str(), "left_operand" | "right_operand") {
                                pending.push(&x.target);
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

fn status(code: u64) -> Result<(), String> {
    match code {
        0 => Ok(()),
        1 => Err("AIL-RUNTIME-VALUE: invalid variant value".into()),
        2 => Err("AIL-RUNTIME-VALUE-LIMIT: variant value limit exceeded".into()),
        _ => Err("AIL-RUNTIME-VALUE-IO: invalid variant engine request".into()),
    }
}
fn request(schema: &[u8], scratch: &mut [u8]) -> crate::value_engine::ValueRequest {
    crate::value_engine::ValueRequest {
        kind: 0,
        schema: schema.as_ptr(),
        schema_len: schema.len() as u64,
        type_index: 0,
        variant_index: 0,
        field_index: 0,
        input: std::ptr::null(),
        input_len: 0,
        args: std::ptr::null(),
        arg_count: 0,
        output: std::ptr::null_mut(),
        output_capacity: 0,
        scratch: scratch.as_mut_ptr(),
        scratch_capacity: scratch.len() as u64,
    }
}
fn run(q: &mut crate::value_engine::ValueRequest) -> Result<(String, u64), String> {
    let producing = !matches!(q.kind, 0 | 3);
    let mut output = if producing {
        vec![0; 8_388_608]
    } else {
        Vec::new()
    };
    q.output = output.as_mut_ptr();
    q.output_capacity = output.len() as u64;
    let mut result = crate::value_engine::ValueResult {
        output_len: 0,
        scalar: 0,
        result_kind: 0,
    };
    // All borrowed descriptors remain live through this synchronous call.
    status(unsafe { crate::value_engine::run(q, &mut result) })?;
    let value = if q.kind == 3 {
        (result.scalar != 0).to_string()
    } else {
        String::from_utf8(output[..result.output_len as usize].to_vec())
            .map_err(|_| "AIL-RUNTIME-VALUE-IO: engine returned invalid text")?
    };
    Ok((value, result.result_kind))
}
pub(super) fn validate(schemas: &AilValueSchemas, kind: &str, value: &str) -> Result<(), String> {
    let schema = value_types::encode(schemas)?;
    let mut scratch = vec![0; 8_388_608];
    let mut q = request(&schema, &mut scratch);
    q.type_index = value_types::type_code(schemas, kind)?
        .checked_sub(9)
        .ok_or_else(|| error("expected custom type"))? as u64;
    q.input = value.as_ptr();
    q.input_len = value.len() as u64;
    run(&mut q).map(|_| ())
}
fn reserve(size: usize, list: &mut usize) -> Result<(), String> {
    *list = list
        .checked_add(size.max(1))
        .filter(|n| *n <= 67_108_864)
        .ok_or("AIL-RUNTIME-VALUE-LIMIT: variant allocation budget exceeded")?;
    Ok(())
}
pub(super) fn returned(
    schemas: &AilValueSchemas,
    kind: &str,
    value: &str,
    list: &mut usize,
) -> Result<String, String> {
    let schema = value_types::encode(schemas)?;
    let mut scratch = vec![0; 8_388_608];
    let mut q = request(&schema, &mut scratch);
    q.kind = 1;
    q.type_index = (value_types::type_code(schemas, kind)? - 9) as u64;
    q.input = value.as_ptr();
    q.input_len = value.len() as u64;
    let (value, _) = run(&mut q)?;
    reserve(value.len(), list)?;
    Ok(value)
}
pub(super) fn execute(
    i: &AilBytecodeInstruction,
    schemas: &AilValueSchemas,
    state: &mut BTreeMap<String, String>,
    text: &mut usize,
    list: &mut usize,
) -> Result<(), String> {
    verify(i, schemas)?;
    let o = &i.operands;
    let schema = value_types::encode(schemas)?;
    let mut scratch = vec![0; 8_388_608];
    let mut q = request(&schema, &mut scratch);
    let (t, v, f) = value_types::indices(
        schemas,
        &o["type"],
        &o["variant"],
        o.get("field").map(String::as_str),
    )?;
    q.type_index = t as u64;
    q.variant_index = v as u64;
    q.field_index = f.unwrap_or(0) as u64;
    let mut args = Vec::new();
    let source;
    if i.opcode == "VALUE_CONSTRUCT" {
        for (name, e) in integer_function::decode_arguments(&o["arguments"])? {
            let ty = &schemas[&o["type"]][&o["variant"]][&name];
            let value = if ty == "Int" {
                std::borrow::Cow::Owned(e.eval(state)?.to_string())
            } else {
                let IntExpr::Field(k) = e else {
                    unreachable!("checked field")
                };
                std::borrow::Cow::Borrowed(
                    state
                        .get(&k)
                        .ok_or("AIL-RUNTIME-VALUE: missing constructor payload")?
                        .as_str(),
                )
            };
            let mut check = request(&schema, &mut scratch);
            check.type_index = value_types::type_code(schemas, ty)? as u64;
            check.input = value.as_ptr();
            check.input_len = value.len() as u64;
            status(unsafe { crate::value_engine::validate_argument(&check) })?;
            args.push(value);
        }
        q.kind = 2;
    } else {
        source = state
            .get(&o["source"])
            .ok_or("AIL-RUNTIME-VALUE: missing variant value")?;
        q.input = source.as_ptr();
        q.input_len = source.len() as u64;
        q.kind = if i.opcode == "VALUE_TEST" { 3 } else { 4 };
    }
    let spans = args
        .iter()
        .map(|a| crate::value_engine::ValueSpan {
            data: a.as_ptr(),
            len: a.len() as u64,
        })
        .collect::<Vec<_>>();
    q.args = spans.as_ptr();
    q.arg_count = spans.len() as u64;
    let (value, _) = run(&mut q)?;
    let value = match i.opcode.as_str() {
        "VALUE_TEST" => value,
        "VALUE_CONSTRUCT" => {
            reserve(value.len(), list)?;
            value
        }
        _ => {
            let kind = &schemas[&o["type"]][&o["variant"]][&o["field"]];
            if schemas.contains_key(kind) {
                reserve(value.len(), list)?;
                value
            } else if kind == "Int" {
                value
            } else {
                value_vm::returned(kind, &value, schemas, text, list)?
            }
        }
    };
    drop(spans);
    drop(args);
    state.insert(o["key"].clone(), value);
    Ok(())
}
