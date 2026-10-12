//! Typed executable function signatures and scoped instruction validation.
use super::*;
use crate::expression::IntExpr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Signature {
    pub parameters: BTreeMap<String, String>,
    pub locals: BTreeMap<String, String>,
    pub result: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum OperandKind {
    Field(String),
    Expression,
    FloatExpression,
    BooleanExpression,
}

pub(super) fn supported(kind: &str) -> bool {
    matches!(
        kind,
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

pub(super) fn is_function(action: &AilBytecodeAction) -> bool {
    action
        .instructions
        .first()
        .is_some_and(|i| i.opcode == "VALUE_FUNCTION")
}

fn error(message: impl Display) -> String {
    format!("AIL-FUNCTION-001: {message}")
}

pub(super) fn identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn exact(i: &AilBytecodeInstruction, names: &[&str]) -> Result<(), String> {
    if i.operands.len() != names.len() || names.iter().any(|name| !i.operands.contains_key(*name)) {
        return Err(error(
            "function instruction requires exactly its defined operands",
        ));
    }
    Ok(())
}

pub(super) fn encode_types(types: &BTreeMap<String, String>) -> String {
    types
        .iter()
        .map(|(name, kind)| format!("{name}:{kind}"))
        .collect::<Vec<_>>()
        .join(";")
}

pub(super) fn decode_types(
    input: &str,
    schemas: &AilValueSchemas,
) -> Result<BTreeMap<String, String>, String> {
    let mut types: BTreeMap<String, String> = BTreeMap::new();
    if !input.is_empty() {
        for entry in input.split(';') {
            let (name, kind) = entry
                .split_once(':')
                .ok_or_else(|| error("invalid typed declaration"))?;
            if !identifier(name)
                || !value_types::supported(schemas, kind)
                || types.len() >= 256
                || types
                    .last_key_value()
                    .is_some_and(|(previous, _)| previous.as_str() >= name)
            {
                return Err(error(
                    "typed declarations must be sorted unique supported names (maximum 256)",
                ));
            }
            types.insert(name.to_string(), kind.to_string());
        }
    }
    Ok(types)
}

pub(super) fn signature(
    action: &AilBytecodeAction,
    schemas: &AilValueSchemas,
) -> Result<Signature, String> {
    let marker = action
        .instructions
        .first()
        .filter(|i| i.opcode == "VALUE_FUNCTION")
        .ok_or_else(|| error("expected a value function signature"))?;
    exact(marker, &["parameters", "locals", "result"])?;
    let parameters = decode_types(&marker.operands["parameters"], schemas)?;
    let locals = decode_types(&marker.operands["locals"], schemas)?;
    let result = marker.operands["result"].clone();
    if !value_types::supported(schemas, &result)
        || parameters.len() + locals.len() > 256
        || parameters.keys().any(|name| locals.contains_key(name))
    {
        return Err(error(
            "function requires supported result and at most 256 distinct parameters/locals",
        ));
    }
    Ok(Signature {
        parameters,
        locals,
        result,
    })
}

impl Signature {
    pub(super) fn fields(&self) -> BTreeMap<String, String> {
        self.parameters
            .iter()
            .chain(&self.locals)
            .map(|(name, kind)| (format!("local.{name}"), kind.clone()))
            .collect()
    }
}

pub(super) fn signatures(
    program: &AilBytecodeProgram,
) -> Result<BTreeMap<String, Signature>, String> {
    let mut result = BTreeMap::new();
    for (name, action) in &program.actions {
        if !is_function(action) && !integer_vm::is_function(action) {
            continue;
        }
        if name != &action.name {
            return Err(error("function map key differs from its name"));
        }
        let sig = if is_function(action) {
            signature(action, &program.type_schemas)?
        } else {
            let marker = &action.instructions[0];
            exact(marker, &["parameters"])?;
            let parameters = if marker.operands["parameters"].is_empty() {
                BTreeMap::new()
            } else {
                let text = marker.operands["parameters"]
                    .split(';')
                    .map(|name| format!("{name}:Int"))
                    .collect::<Vec<_>>()
                    .join(";");
                decode_types(&text, &program.type_schemas)?
            };
            Signature {
                parameters,
                locals: BTreeMap::new(),
                result: "Int".into(),
            }
        };
        result.insert(name.clone(), sig);
    }
    Ok(result)
}

pub(super) fn operand_schema(
    i: &AilBytecodeInstruction,
    schemas: &AilValueSchemas,
) -> Result<BTreeMap<String, OperandKind>, String> {
    let mut schema = BTreeMap::new();
    let mut add = |name: &str, kind: &str| {
        schema.insert(name.into(), OperandKind::Field(kind.into()));
    };
    match i.opcode.as_str() {
        "VALUE_CONSTRUCT" | "VALUE_TEST" | "VALUE_GET" => {
            value_operations::verify(i, schemas)?;
            let ty = &i.operands["type"];
            add(
                "key",
                match i.opcode.as_str() {
                    "VALUE_CONSTRUCT" => ty,
                    "VALUE_TEST" => "Bool",
                    _ => &schemas[ty][&i.operands["variant"]][&i.operands["field"]],
                },
            );
            if i.operands.contains_key("source") {
                add("source", ty);
            }
        }
        "RAISE" => recovery::verify_raise(i)?,
        "TRY_BEGIN" => {
            exact(i, &["errors", "key"])?;
            recovery::mask(&i.operands["errors"])?;
            add("key", "Text");
        }
        "RECOVER" | "TRY_END" => exact(i, &[])?,
        "EVAL_BOOL" => {
            bool_operations::verify(i)?;
            add("key", "Bool");
            schema.insert("expression".into(), OperandKind::BooleanExpression);
        }
        "BRANCH_BOOL" => {
            bool_operations::verify(i)?;
            schema.insert("expression".into(), OperandKind::BooleanExpression);
        }
        "EVAL_FLOAT" => {
            float_operations::verify(i)?;
            add("key", "Float");
            schema.insert("expression".into(), OperandKind::FloatExpression);
        }
        "INT_TO_FLOAT" | "FLOAT_TO_INT" => {
            float_operations::verify(i)?;
            add(
                "source",
                if i.opcode == "INT_TO_FLOAT" {
                    "Int"
                } else {
                    "Float"
                },
            );
            add(
                "key",
                if i.opcode == "INT_TO_FLOAT" {
                    "Float"
                } else {
                    "Int"
                },
            );
        }
        "BRANCH_FLOAT" => {
            float_operations::verify(i)?;
            schema.insert("left".into(), OperandKind::FloatExpression);
            schema.insert("right".into(), OperandKind::FloatExpression);
        }
        "EVAL_INT" => {
            exact(i, &["key", "expression"])?;
            add("key", "Int");
            schema.insert("expression".into(), OperandKind::Expression);
        }
        "BRANCH_INT" => {
            exact(i, &["left", "right", "comparison", "label"])?;
            crate::condition::IntCondition::from_postfix(
                &i.operands["left"],
                &i.operands["comparison"],
                &i.operands["right"],
            )?;
            schema.insert("left".into(), OperandKind::Expression);
            schema.insert("right".into(), OperandKind::Expression);
        }
        "BYTES_LENGTH" | "BYTES_GET" | "BYTES_SLICE" | "BYTES_CONCAT" | "BYTES_SET"
        | "BYTES_FROM_UTF8" | "BYTES_TO_UTF8" | "BYTES_FROM_HEX" | "BYTES_TO_HEX"
        | "BYTES_COMPARE" => {
            bytes_operations::verify(i)?;
            add(
                "key",
                match i.opcode.as_str() {
                    "BYTES_LENGTH" | "BYTES_GET" | "BYTES_COMPARE" => "Int",
                    "BYTES_TO_UTF8" | "BYTES_TO_HEX" => "Text",
                    _ => "Bytes",
                },
            );
            for name in ["source", "left", "right"] {
                if i.operands.contains_key(name) {
                    add(
                        name,
                        if matches!(i.opcode.as_str(), "BYTES_FROM_UTF8" | "BYTES_FROM_HEX") {
                            "Text"
                        } else {
                            "Bytes"
                        },
                    );
                }
            }
            for name in ["index", "value", "start", "end"] {
                if i.operands.contains_key(name) {
                    schema.insert(name.into(), OperandKind::Expression);
                }
            }
        }
        "ASSIGN_LITERAL" => {
            literal_assignment::verify(i)?;
            add("key", &i.operands["type"]);
        }
        "TEXT_JOIN" | "TEXT_LENGTH" | "TEXT_SLICE" | "TEXT_FIND" | "TEXT_SPLIT"
        | "TEXT_JOIN_LIST" | "TEXT_REPLACE" | "TEXT_TRIM" => {
            text_operations::verify(i)?;
            add(
                "key",
                if matches!(i.opcode.as_str(), "TEXT_LENGTH" | "TEXT_FIND") {
                    "Int"
                } else if i.opcode == "TEXT_SPLIT" {
                    "List<Text>"
                } else {
                    "Text"
                },
            );
            for name in [
                "left",
                "right",
                "source",
                "needle",
                "separator",
                "replacement",
            ] {
                if i.operands.contains_key(name) {
                    add(
                        name,
                        if name == "source" && i.opcode == "TEXT_JOIN_LIST" {
                            "List<Text>"
                        } else {
                            "Text"
                        },
                    );
                }
            }
            for name in ["start", "end"] {
                if i.operands.contains_key(name) {
                    schema.insert(name.into(), OperandKind::Expression);
                }
            }
        }
        "LIST_EMPTY" | "LIST_LENGTH" | "LIST_GET" | "LIST_APPEND" | "LIST_SET" | "LIST_CONCAT"
        | "LIST_SLICE" | "LIST_REVERSE" | "LIST_SORT" => {
            list_operations::verify(i)?;
            let element = &i.operands["type"];
            let list = format!("List<{element}>");
            add(
                "key",
                match i.opcode.as_str() {
                    "LIST_LENGTH" => "Int",
                    "LIST_GET" => element,
                    _ => &list,
                },
            );
            for name in ["source", "other"] {
                if i.operands.contains_key(name) {
                    add(name, &list);
                }
            }
            if i.operands.contains_key("value") && element == "Text" {
                add("value", "Text");
            }
            for name in ["index", "start", "end"] {
                if i.operands.contains_key(name) {
                    schema.insert(name.into(), OperandKind::Expression);
                }
            }
            if i.operands.contains_key("value") && element == "Int" {
                schema.insert("value".into(), OperandKind::Expression);
            }
            if i.operands.contains_key("value") && element == "Float" {
                schema.insert("value".into(), OperandKind::FloatExpression);
            }
        }
        "JSON_PARSE" | "JSON_FORMAT" => {
            json_codec::verify(i)?;
            let kind = &i.operands["type"];
            add(
                "source",
                if i.opcode == "JSON_PARSE" {
                    "Text"
                } else {
                    kind
                },
            );
            add(
                "key",
                if i.opcode == "JSON_PARSE" {
                    kind
                } else {
                    "Text"
                },
            );
        }
        "JSON_GET" | "JSON_COUNT" | "JSON_KIND" | "JSON_KEYS" => {
            structured_json::verify(i)?;
            add("source", "Json");
            add(
                "key",
                match i.opcode.as_str() {
                    "JSON_COUNT" => "Int",
                    "JSON_KIND" => "Text",
                    "JSON_KEYS" => "List<Text>",
                    _ => &i.operands["type"],
                },
            );
            if i.opcode == "JSON_GET" {
                add("path", "Text");
            }
        }
        "JSON_SET" | "JSON_REMOVE" => {
            json_mutation::verify(i)?;
            add("source", "Json");
            add("key", "Json");
            add("path", "Text");
            if i.opcode == "JSON_SET" {
                add("value", &i.operands["type"]);
            }
        }
        _ => return Err(error("instruction is not a local value operation")),
    }
    Ok(schema)
}

pub(super) fn instruction_fields(
    i: &AilBytecodeInstruction,
    schemas: &AilValueSchemas,
) -> Result<BTreeMap<String, String>, String> {
    let mut fields = BTreeMap::new();
    for (key, kind) in instruction_constraints(i, schemas)? {
        if fields
            .insert(key, kind.clone())
            .is_some_and(|previous| previous != kind)
        {
            return Err(error("one field is used with conflicting types"));
        }
    }
    Ok(fields)
}

fn instruction_constraints(
    i: &AilBytecodeInstruction,
    schemas: &AilValueSchemas,
) -> Result<Vec<(String, String)>, String> {
    if matches!(
        i.opcode.as_str(),
        "VALUE_CONSTRUCT" | "VALUE_TEST" | "VALUE_GET"
    ) {
        return Ok(value_operations::fields(i, schemas)?.into_iter().collect());
    }
    let mut fields = Vec::new();
    let mut add = |key: String, kind: String| -> Result<(), String> {
        if !matches!(IntExpr::parse(&key), Ok(IntExpr::Field(parsed)) if parsed == key) {
            return Err(error("invalid field operand"));
        }
        fields.push((key, kind));
        Ok(())
    };
    for (name, kind) in operand_schema(i, schemas)? {
        match kind {
            OperandKind::Field(kind) => add(i.operands[&name].clone(), kind)?,
            OperandKind::BooleanExpression => {
                for (field, kind) in
                    crate::bool_expression::BoolExpr::from_postfix(&i.operands[&name])?
                        .typed_fields()?
                {
                    add(field, kind)?;
                }
            }
            OperandKind::FloatExpression => {
                for field in
                    crate::float_expression::FloatExpr::from_postfix(&i.operands[&name])?.fields()
                {
                    add(field, "Float".into())?;
                }
            }
            OperandKind::Expression => {
                for field in IntExpr::from_postfix(&i.operands[&name])?.fields() {
                    add(field, "Int".into())?;
                }
            }
        }
    }
    Ok(fields)
}

fn typed_expression(
    expression: &IntExpr,
    kind: &str,
    fields: Option<&BTreeMap<String, String>>,
) -> Result<(), String> {
    if kind != "Int" && !matches!(expression, IntExpr::Field(_)) {
        return Err(error("non-Int values require a bare field reference"));
    }
    for field in expression.fields() {
        if let Some(fields) = fields
            && fields.get(&field).map(String::as_str) != Some(kind)
        {
            return Err(error(
                "function operand must reference declared storage of the required type",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_call(
    i: &AilBytecodeInstruction,
    fields: Option<&BTreeMap<String, String>>,
    signatures: &BTreeMap<String, Signature>,
) -> Result<(), String> {
    exact(i, &["target", "arguments", "key"])?;
    let sig = signatures
        .get(&i.operands["target"])
        .ok_or_else(|| error("unknown executable call target"))?;
    let args = integer_function::decode_arguments(&i.operands["arguments"])?;
    if args.keys().ne(sig.parameters.keys()) {
        return Err(error("named arguments do not match function signature"));
    }
    for (name, expression) in &args {
        typed_expression(expression, &sig.parameters[name], fields)?;
    }
    let key = &i.operands["key"];
    if !matches!(IntExpr::parse(key),Ok(IntExpr::Field(parsed)) if parsed == *key)
        || fields.is_some_and(|fields| fields.get(key) != Some(&sig.result))
    {
        return Err(error("call destination must have the function result type"));
    }
    Ok(())
}

pub(super) fn validate_body_instruction(
    i: &AilBytecodeInstruction,
    fields: &BTreeMap<String, String>,
    result: &str,
    signatures: &BTreeMap<String, Signature>,
    schemas: &AilValueSchemas,
) -> Result<(), String> {
    match i.opcode.as_str() {
        "CALL_INT" | "CALL_VALUE" => validate_call(i, Some(fields), signatures),
        "RETURN_VALUE" => {
            exact(i, &["expression", "type"])?;
            if i.operands["type"] != result {
                return Err(error("return type differs from function signature"));
            }
            typed_expression(
                &IntExpr::from_postfix(&i.operands["expression"])?,
                result,
                Some(fields),
            )
        }
        "LABEL" => exact(i, &["name"]),
        "JUMP" => exact(i, &["label"]),
        "EMIT_TRACE" => exact(i, &["event"]),
        _ => {
            for (key, kind) in instruction_fields(i, schemas)? {
                if fields.get(&key) != Some(&kind) {
                    return Err(error(
                        "local operation references undeclared or incorrectly typed storage",
                    ));
                }
            }
            Ok(())
        }
    }
}

pub(super) fn verify(program: &AilBytecodeProgram) -> Vec<String> {
    let signatures = match signatures(program) {
        Ok(signatures) => signatures,
        Err(e) => return vec![format!("AILBC011 {e}")],
    };
    let mut errors = verify_custom_action_fields(program, &signatures);
    for action in program.actions.values() {
        let general = is_function(action);
        let signature = signatures.get(&action.name);
        let fields = signature.map(Signature::fields).unwrap_or_default();
        if general && action.instructions.len() > 16_385 {
            errors.push(format!(
                "AILBC011 function {} exceeds 16384 body instructions",
                action.name
            ));
        }
        if general && let Err(e) = recovery::check_raise_termination(action) {
            errors.push(format!("AILBC011 function {}: {e}", action.name));
        }
        for (index, i) in action.instructions.iter().enumerate() {
            let checked = (|| {
                if (general
                    || matches!(
                        i.opcode.as_str(),
                        "VALUE_FUNCTION" | "CALL_VALUE" | "RETURN_VALUE"
                    ))
                    && !matches!(program.profile.as_str(), "Application" | "C interop")
                {
                    return Err(error(
                        "value functions require Application or C interop profile",
                    ));
                }
                if i.opcode == "VALUE_FUNCTION" {
                    return if general && index == 0 {
                        Ok(())
                    } else {
                        Err(error("value function signature must be first"))
                    };
                }
                if i.opcode == "RETURN_VALUE" && !general {
                    return Err(error("RETURN_VALUE requires a value function"));
                }
                if matches!(i.opcode.as_str(), "CALL_VALUE" | "CALL_INT") {
                    let target = program
                        .actions
                        .get(i.operands.get("target").map(String::as_str).unwrap_or(""));
                    let target_matches = target.is_some_and(|target| {
                        if i.opcode == "CALL_VALUE" {
                            is_function(target)
                        } else {
                            integer_vm::is_function(target)
                        }
                    });
                    if !target_matches {
                        return Err(error("call opcode does not match executable target kind"));
                    }
                }
                if matches!(i.opcode.as_str(), "CALL_ACTION" | "REPEAT_ACTION")
                    && i.operands
                        .get("target")
                        .is_some_and(|target| signatures.contains_key(target))
                {
                    return Err(error("executable functions require a value call"));
                }
                if general {
                    validate_body_instruction(
                        i,
                        &fields,
                        &signature.unwrap().result,
                        &signatures,
                        &program.type_schemas,
                    )
                } else if i.opcode == "CALL_VALUE" {
                    validate_call(i, None, &signatures)
                } else {
                    Ok(())
                }
            })();
            if let Err(e) = checked {
                errors.push(format!(
                    "AILBC011 procedure {} instruction {index}: {e}",
                    action.name
                ));
            }
        }
    }
    errors
}

// Action storage is shared across calls. Preserve the nominal type information
// supplied by variant operations and function contracts even in authored bytecode.
// Historical builtin-only action storage remains dynamically interpreted.
fn verify_custom_action_fields(
    program: &AilBytecodeProgram,
    signatures: &BTreeMap<String, Signature>,
) -> Vec<String> {
    if program.type_schemas.is_empty() {
        return Vec::new();
    }
    let mut constraints = BTreeMap::<String, BTreeSet<String>>::new();
    let mut errors = Vec::new();
    for action in program
        .actions
        .values()
        .filter(|action| !is_function(action) && !integer_vm::is_function(action))
    {
        for i in &action.instructions {
            let mut add = |key: String, kind: String| {
                constraints.entry(key).or_default().insert(kind);
            };
            if matches!(i.opcode.as_str(), "CALL_INT" | "CALL_VALUE") {
                if validate_call(i, None, signatures).is_err() {
                    continue; // The normal call verifier reports malformed calls.
                }
                let signature = &signatures[&i.operands["target"]];
                for (name, expression) in
                    integer_function::decode_arguments(&i.operands["arguments"]).unwrap()
                {
                    for key in expression.fields() {
                        add(key, signature.parameters[&name].clone());
                    }
                }
                add(i.operands["key"].clone(), signature.result.clone());
            } else {
                match instruction_constraints(i, &program.type_schemas) {
                    Ok(fields) => {
                        for (key, kind) in fields {
                            add(key, kind);
                        }
                    }
                    Err(e)
                        if matches!(
                            i.opcode.as_str(),
                            "VALUE_CONSTRUCT" | "VALUE_TEST" | "VALUE_GET"
                        ) =>
                    {
                        errors.push(format!("AILBC024 action {}: {e}", action.name));
                    }
                    Err(_) => {} // Other operation families retain their own verification.
                }
            }
        }
    }
    for (key, kinds) in constraints {
        if kinds.len() > 1
            && kinds
                .iter()
                .any(|kind| program.type_schemas.contains_key(kind))
        {
            errors.push(format!(
                "AILBC024 action field {key} has conflicting custom value types"
            ));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    fn signature(action: &AilBytecodeAction) -> Result<Signature, String> {
        super::signature(action, &AilValueSchemas::new())
    }
    fn validate_body_instruction(
        i: &AilBytecodeInstruction,
        fields: &BTreeMap<String, String>,
        result: &str,
        signatures: &BTreeMap<String, Signature>,
    ) -> Result<(), String> {
        super::validate_body_instruction(i, fields, result, signatures, &AilValueSchemas::new())
    }

    fn instruction(opcode: &str, pairs: &[(&str, &str)]) -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(
            opcode,
            &pairs
                .iter()
                .map(|(k, v)| (*k, (*v).into()))
                .collect::<Vec<_>>(),
        )
    }

    fn function(parameters: &str, locals: &str, result: &str) -> AilBytecodeAction {
        AilBytecodeAction {
            name: "transform".into(),
            instructions: vec![instruction(
                "VALUE_FUNCTION",
                &[
                    ("parameters", parameters),
                    ("locals", locals),
                    ("result", result),
                ],
            )],
        }
    }

    #[test]
    fn signatures_require_exact_sorted_disjoint_supported_declarations() {
        let valid = signature(&function(
            "data:Json;flag:Bool",
            "count:Int;names:List<Text>",
            "Json",
        ))
        .unwrap();
        assert_eq!(valid.parameters["data"], "Json");
        assert_eq!(valid.locals["names"], "List<Text>");
        let floating = signature(&function("a:Float", "result:Float", "Float")).unwrap();
        assert_eq!(floating.parameters["a"], "Float");
        assert_eq!(floating.result, "Float");
        for (params, locals, result) in [
            ("z:Int;a:Int", "", "Int"),
            ("a:Int;a:Int", "", "Int"),
            ("a:Int", "a:Text", "Int"),
            ("a:Decimal", "", "Int"),
            ("local.a:Int", "", "Int"),
            ("", "", "List<Bool>"),
            ("a:Int;", "", "Int"),
            ("a:Int", " a:Int", "Int"),
        ] {
            assert!(
                signature(&function(params, locals, result)).is_err(),
                "{params}/{locals}/{result}"
            );
        }
        let mut extra = function("", "", "Text");
        extra.instructions[0]
            .operands
            .insert("hidden".into(), "ignored".into());
        assert!(signature(&extra).is_err());
        let declarations = (0..257)
            .map(|n| format!("p{n:03}:Int"))
            .collect::<Vec<_>>()
            .join(";");
        assert!(signature(&function(&declarations, "", "Int")).is_err());
    }

    #[test]
    fn scoped_operations_reject_type_confusion_and_hidden_global_reads() {
        let fields = BTreeMap::from([
            ("local.doc".into(), "Json".into()),
            ("local.path".into(), "Text".into()),
            ("local.n".into(), "Int".into()),
        ]);
        let mut set = instruction(
            "JSON_SET",
            &[
                ("source", "local.doc"),
                ("path", "local.path"),
                ("value", "local.n"),
                ("key", "local.doc"),
                ("type", "Int"),
            ],
        );
        validate_body_instruction(&set, &fields, "Json", &BTreeMap::new()).unwrap();
        set.operands.insert("value".into(), "local.path".into());
        assert!(validate_body_instruction(&set, &fields, "Json", &BTreeMap::new()).is_err());
        let expression = instruction(
            "EVAL_INT",
            &[("key", "local.n"), ("expression", "f:global.value i:1 +")],
        );
        assert!(validate_body_instruction(&expression, &fields, "Json", &BTreeMap::new()).is_err());
        for op in [
            instruction(
                "FILE_READ_TEXT",
                &[("resource", "in"), ("key", "local.path")],
            ),
            instruction("OUTPUT_FIELD", &[("key", "local.doc")]),
            instruction("RETURN_INT", &[("expression", "i:0")]),
        ] {
            assert!(validate_body_instruction(&op, &fields, "Json", &BTreeMap::new()).is_err());
        }
    }

    #[test]
    fn calls_and_returns_check_exact_declared_types_and_bare_value_fields() {
        let signatures = BTreeMap::from([(
            "transform".into(),
            signature(&function("data:Json;times:Int", "", "Json")).unwrap(),
        )]);
        let fields = BTreeMap::from([
            ("local.doc".into(), "Json".into()),
            ("local.n".into(), "Int".into()),
        ]);
        let mut call = instruction(
            "CALL_VALUE",
            &[
                ("target", "transform"),
                ("arguments", "data=f:local.doc;times=f:local.n i:1 +"),
                ("key", "local.doc"),
            ],
        );
        validate_body_instruction(&call, &fields, "Json", &signatures).unwrap();
        call.operands.insert(
            "arguments".into(),
            "data=f:local.doc i:1 +;times=i:1".into(),
        );
        assert!(validate_body_instruction(&call, &fields, "Json", &signatures).is_err());
        let good = instruction(
            "RETURN_VALUE",
            &[("expression", "f:local.doc"), ("type", "Json")],
        );
        validate_body_instruction(&good, &fields, "Json", &signatures).unwrap();
        let wrong = instruction(
            "RETURN_VALUE",
            &[("expression", "f:local.n"), ("type", "Json")],
        );
        assert!(validate_body_instruction(&wrong, &fields, "Json", &signatures).is_err());
    }
}
