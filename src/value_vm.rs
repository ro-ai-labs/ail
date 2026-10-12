//! Typed functions with isolated local frames and borrowed argument transports.
use super::*;
use crate::condition::IntCondition;
use crate::expression::IntExpr;
use std::borrow::Cow;

enum Step {
    Nop,
    Raise(String),
    Eval(String, IntExpr),
    Literal(String, String),
    Call {
        target: String,
        arguments: BTreeMap<String, IntExpr>,
        key: String,
        integer: bool,
    },
    Return(IntExpr, String),
    Branch(IntCondition, usize),
    FloatBranch(crate::float_expression::FloatCondition, usize),
    BooleanBranch(crate::bool_expression::BoolExpr, usize),
    BooleanEval(String, crate::bool_expression::BoolExpr),
    Jump(usize),
    Trace(String),
    Operation(AilBytecodeInstruction, BTreeSet<String>),
}
struct Routine {
    signature: value_protocol::Signature,
    steps: Vec<Step>,
    recovery: recovery::Layout,
}
pub(super) struct Machine {
    schemas: AilValueSchemas,
    routines: BTreeMap<String, Routine>,
    integers: integer_vm::Machine,
}

fn project<V: AsRef<str>>(
    state: &BTreeMap<String, V>,
    fields: impl IntoIterator<Item = String>,
) -> BTreeMap<String, String> {
    fields
        .into_iter()
        .filter_map(|key| {
            state
                .get(&key)
                .map(|value| (key, value.as_ref().to_string()))
        })
        .collect()
}
fn eval<V: AsRef<str>>(expression: &IntExpr, state: &BTreeMap<String, V>) -> Result<i64, String> {
    expression.eval(&project(state, expression.fields()))
}
fn read<'a, V: AsRef<str>>(
    expression: &IntExpr,
    kind: &str,
    state: &'a BTreeMap<String, V>,
) -> Result<&'a str, String> {
    let IntExpr::Field(key) = expression else {
        return Err("AIL-FUNCTION-001: non-Int argument requires a field".into());
    };
    let family = match kind {
        "Bytes" => "BYTES",
        "Text" => "TEXT",
        "Float" => "FLOAT",
        "List<Int>" | "List<Float>" | "List<Text>" => "LIST",
        _ if !value_protocol::supported(kind) => "VALUE",
        _ => "JSON",
    };
    state
        .get(key)
        .map(AsRef::as_ref)
        .ok_or_else(|| format!("AIL-RUNTIME-{family}: missing function value"))
}
fn validate(kind: &str, value: &str, schemas: &AilValueSchemas) -> Result<(), String> {
    match kind {
        "Float" => float_operations::runtime_parse(value).map(|_| ()),
        "Text" if value.len() > 1_048_576 => {
            Err("AIL-RUNTIME-TEXT-LIMIT: text argument exceeds 1048576 bytes".into())
        }
        "Bytes" => bytes_operations::runtime_validate(value),
        "Text" => Ok(()),
        "Bool" if value.len() > 1_048_576 => {
            Err("AIL-RUNTIME-JSON-LIMIT: Boolean transport exceeds 1048576 bytes".into())
        }
        "Bool" if matches!(value, "true" | "false") => Ok(()),
        "Bool" => Err("AIL-RUNTIME-JSON: invalid Boolean transport".into()),
        "List<Int>" => list_operations::validate_runtime_value(value, "Int"),
        "List<Float>" => list_operations::validate_runtime_value(value, "Float"),
        "List<Text>" => list_operations::validate_runtime_value(value, "Text"),
        "Json" => structured_json::canonical(value).map(|_| ()),
        _ if schemas.contains_key(kind) => value_operations::validate(schemas, kind, value),
        _ => Err("AIL-FUNCTION-001: unsupported function value type".into()),
    }
}
pub(super) fn returned(
    kind: &str,
    value: &str,
    schemas: &AilValueSchemas,
    text: &mut usize,
    list: &mut usize,
) -> Result<String, String> {
    match kind {
        "Float" => float_operations::runtime_format(float_operations::runtime_parse(value)?),
        "Bytes" => {
            bytes_operations::runtime_validate(value)?;
            bytes_operations::reserve_result(value.len(), list)?;
            Ok(value.to_string())
        }
        "Text" => {
            text_operations::reserve_result(value.len(), text)?;
            Ok(value.to_string())
        }
        "Bool" => {
            validate(kind, value, schemas)?;
            Ok(value.to_string())
        }
        "List<Int>" | "List<Float>" | "List<Text>" => {
            let value = list_operations::canonical_literal(
                value,
                match kind {
                    "List<Int>" => "Int",
                    "List<Float>" => "Float",
                    _ => "Text",
                },
            )?;
            let total = list
                .checked_add(value.len().max(1))
                .filter(|n| *n <= 67_108_864)
                .ok_or("AIL-RUNTIME-LIST-LIMIT: list allocation budget exceeded")?;
            *list = total;
            Ok(value)
        }
        "Json" => {
            let value = structured_json::canonical(value)?;
            json_codec::reserve_result(kind, true, value.len(), text, list)?;
            Ok(value)
        }
        _ if schemas.contains_key(kind) => value_operations::returned(schemas, kind, value, list),
        _ => Err("AIL-FUNCTION-001: unsupported function result type".into()),
    }
}
impl Machine {
    pub(super) fn new(program: &AilBytecodeProgram) -> Result<Self, String> {
        let errors = verify_ail_bytecode(program);
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }
        let mut routines = BTreeMap::new();
        for action in program
            .actions
            .values()
            .filter(|a| value_protocol::is_function(a))
        {
            let signature = value_protocol::signature(action, &program.type_schemas)?;
            let (labels, errors) = ail_bytecode_action_labels(action);
            if !errors.is_empty() {
                return Err(errors.join("\n"));
            }
            let recovery = recovery::Layout::analyze(action)?;
            let mut steps = Vec::new();
            for (pc, i) in action.instructions.iter().enumerate() {
                let op = |name: &str| ail_bytecode_operand(i, name);
                let target = || {
                    labels
                        .get(op("label"))
                        .copied()
                        .ok_or("AILBC011 unknown function label")
                };
                steps.push(match i.opcode.as_str() {
                    "VALUE_FUNCTION" | "LABEL" | "TRY_BEGIN" | "TRY_END" => Step::Nop,
                    "RECOVER" => Step::Jump(
                        recovery
                            .recovering(pc)
                            .ok_or("AILBC019: missing recovery scope")?
                            .end
                            + 1,
                    ),
                    "EVAL_INT" => {
                        Step::Eval(op("key").into(), IntExpr::from_postfix(op("expression"))?)
                    }
                    "RAISE" => Step::Raise(op("category").into()),
                    "ASSIGN_LITERAL" => {
                        Step::Literal(op("key").into(), literal_assignment::runtime_value(i)?)
                    }
                    "CALL_INT" | "CALL_VALUE" => Step::Call {
                        target: op("target").into(),
                        arguments: integer_function::decode_arguments(op("arguments"))?,
                        key: op("key").into(),
                        integer: i.opcode == "CALL_INT",
                    },
                    "RETURN_VALUE" => {
                        Step::Return(IntExpr::from_postfix(op("expression"))?, op("type").into())
                    }
                    "EVAL_BOOL" => Step::BooleanEval(
                        op("key").into(),
                        crate::bool_expression::BoolExpr::from_postfix(op("expression"))?,
                    ),
                    "BRANCH_BOOL" => Step::BooleanBranch(
                        crate::bool_expression::BoolExpr::from_postfix(op("expression"))?,
                        target()?,
                    ),
                    "BRANCH_FLOAT" => Step::FloatBranch(
                        crate::float_expression::FloatCondition::from_postfix(
                            op("left"),
                            op("comparison"),
                            op("right"),
                        )?,
                        target()?,
                    ),
                    "BRANCH_INT" => Step::Branch(
                        IntCondition::from_postfix(op("left"), op("comparison"), op("right"))?,
                        target()?,
                    ),
                    "JUMP" => Step::Jump(target()?),
                    "EMIT_TRACE" => Step::Trace(op("event").into()),
                    _ => {
                        let mut reads = BTreeSet::new();
                        for (role, kind) in
                            value_protocol::operand_schema(i, &program.type_schemas)?
                        {
                            if role == "key" {
                                continue;
                            }
                            match kind {
                                value_protocol::OperandKind::Field(_) => {
                                    reads.insert(op(&role).into());
                                }
                                value_protocol::OperandKind::BooleanExpression => {
                                    reads.extend(
                                        crate::bool_expression::BoolExpr::from_postfix(op(&role))?
                                            .typed_fields()?
                                            .into_keys(),
                                    );
                                }
                                value_protocol::OperandKind::FloatExpression => {
                                    reads.extend(
                                        crate::float_expression::FloatExpr::from_postfix(op(
                                            &role,
                                        ))?
                                        .fields(),
                                    );
                                }
                                value_protocol::OperandKind::Expression => {
                                    reads.extend(IntExpr::from_postfix(op(&role))?.fields())
                                }
                            }
                        }
                        if i.opcode == "VALUE_CONSTRUCT" {
                            for expression in
                                integer_function::decode_arguments(&i.operands["arguments"])?
                                    .values()
                            {
                                reads.extend(expression.fields());
                            }
                        }
                        Step::Operation(i.clone(), reads)
                    }
                });
            }
            routines.insert(
                action.name.clone(),
                Routine {
                    signature,
                    steps,
                    recovery,
                },
            );
        }
        Ok(Self {
            schemas: program.type_schemas.clone(),
            routines,
            integers: integer_vm::Machine::new(program)?,
        })
    }
    pub(super) fn call(
        &self,
        name: &str,
        arguments: &BTreeMap<String, IntExpr>,
        state: &BTreeMap<String, String>,
        trace: &mut Vec<String>,
        text: &mut usize,
        list: &mut usize,
    ) -> Result<String, String> {
        self.invoke(name, arguments, state, 0, trace, text, list)
    }
    #[allow(clippy::too_many_arguments)]
    fn invoke<V: AsRef<str>>(
        &self,
        name: &str,
        arguments: &BTreeMap<String, IntExpr>,
        state: &BTreeMap<String, V>,
        depth: usize,
        trace: &mut Vec<String>,
        text: &mut usize,
        list: &mut usize,
    ) -> Result<String, String> {
        let routine = self
            .routines
            .get(name)
            .ok_or("AIL-FUNCTION-001: unknown value function")?;
        if arguments.keys().ne(routine.signature.parameters.keys()) {
            return Err("AIL-FUNCTION-001: named arguments do not match function signature".into());
        }
        let mut frame = BTreeMap::<String, Cow<'_, str>>::new();
        for (name, kind) in &routine.signature.parameters {
            let expression = &arguments[name];
            let value = if kind == "Int" {
                Cow::Owned(eval(expression, state)?.to_string())
            } else {
                let value = read(expression, kind, state)?;
                validate(kind, value, &self.schemas)?;
                Cow::Borrowed(value)
            };
            frame.insert(format!("local.{name}"), value);
        }
        if depth >= 64 {
            return Err("AIL-RUNTIME-DEPTH: value function call depth exceeded".into());
        }
        let mut fuel = routine.steps.len().saturating_mul(1024).max(1024);
        let mut ip = 0;
        while let Some(step) = routine.steps.get(ip) {
            fuel = fuel
                .checked_sub(1)
                .ok_or("AIL bytecode execution step limit exceeded")?;
            let mut next = ip + 1;
            let result = (|| -> Result<Option<String>, String> {
                match step {
                    Step::Nop => {}
                    Step::Raise(category) => return Err(recovery::raised_error(category)),
                    Step::Eval(key, expression) => {
                        let value = eval(expression, &frame)?;
                        frame.insert(key.clone(), Cow::Owned(value.to_string()));
                    }
                    Step::Literal(key, value) => {
                        frame.insert(key.clone(), Cow::Borrowed(value));
                    }
                    Step::Call {
                        target,
                        arguments,
                        key,
                        integer,
                    } => {
                        let value = if *integer {
                            let arguments = arguments
                                .values()
                                .map(|expression| eval(expression, &frame))
                                .collect::<Result<Vec<_>, _>>()?;
                            self.integers
                                .call_at_depth(target, &arguments, depth + 1, trace)?
                                .to_string()
                        } else {
                            self.invoke(target, arguments, &frame, depth + 1, trace, text, list)?
                        };
                        frame.insert(key.clone(), Cow::Owned(value));
                    }
                    Step::Return(expression, kind) => {
                        return if kind == "Int" {
                            Ok(Some(eval(expression, &frame)?.to_string()))
                        } else {
                            returned(
                                kind,
                                read(expression, kind, &frame)?,
                                &self.schemas,
                                text,
                                list,
                            )
                            .map(Some)
                        };
                    }
                    Step::BooleanEval(key, expression) => {
                        let value = expression.eval_borrowed(&frame)?;
                        frame.insert(key.clone(), Cow::Owned(value.to_string()));
                    }
                    Step::BooleanBranch(expression, target) => {
                        if expression.eval_borrowed(&frame)? {
                            next = *target;
                            return Ok(None);
                        }
                    }
                    Step::FloatBranch(condition, target) => {
                        if condition.eval(&project(&frame, condition.fields()))? {
                            next = *target;
                            return Ok(None);
                        }
                    }
                    Step::Branch(condition, target) => {
                        if condition.eval(&project(&frame, condition.fields()))? {
                            next = *target;
                            return Ok(None);
                        }
                    }
                    Step::Jump(target) => {
                        next = *target;
                        return Ok(None);
                    }
                    Step::Trace(event) => trace.push(format!("trace {event}")),
                    Step::Operation(i, reads) => {
                        let mut local = project(&frame, reads.iter().cloned());
                        match i.opcode.as_str() {
                            "VALUE_CONSTRUCT" | "VALUE_TEST" | "VALUE_GET" => {
                                value_operations::execute(i, &self.schemas, &mut local, text, list)?
                            }
                            "BYTES_LENGTH" | "BYTES_GET" | "BYTES_SLICE" | "BYTES_CONCAT"
                            | "BYTES_SET" | "BYTES_FROM_UTF8" | "BYTES_TO_UTF8"
                            | "BYTES_FROM_HEX" | "BYTES_TO_HEX" | "BYTES_COMPARE" => {
                                bytes_operations::execute(i, &mut local, text, list)?
                            }
                            "EVAL_BOOL" => bool_operations::execute(i, &mut local)?,
                            "EVAL_FLOAT" | "INT_TO_FLOAT" | "FLOAT_TO_INT" => {
                                float_operations::execute(i, &mut local)?
                            }
                            "TEXT_JOIN" | "TEXT_LENGTH" | "TEXT_SLICE" | "TEXT_FIND"
                            | "TEXT_SPLIT" | "TEXT_JOIN_LIST" | "TEXT_REPLACE" | "TEXT_TRIM" => {
                                text_operations::execute(i, &mut local, text, list)?
                            }
                            "LIST_EMPTY" | "LIST_LENGTH" | "LIST_GET" | "LIST_APPEND"
                            | "LIST_SET" | "LIST_CONCAT" | "LIST_SLICE" | "LIST_REVERSE"
                            | "LIST_SORT" => list_operations::execute(i, &mut local, text, list)?,
                            "JSON_PARSE" | "JSON_FORMAT" => {
                                json_codec::execute(i, &mut local, text, list)?
                            }
                            "JSON_GET" | "JSON_COUNT" | "JSON_KIND" | "JSON_KEYS" => {
                                structured_json::execute(i, &mut local, text, list)?
                            }
                            "JSON_SET" | "JSON_REMOVE" => {
                                json_mutation::execute(i, &mut local, text, list)?
                            }
                            _ => return Err("AIL-FUNCTION-001: unsupported local operation".into()),
                        }
                        let key = &i.operands["key"];
                        frame.insert(
                            key.clone(),
                            Cow::Owned(
                                local
                                    .remove(key)
                                    .ok_or("AIL-FUNCTION-001: local operation has no result")?,
                            ),
                        );
                    }
                }
                Ok(None)
            })();
            match result {
                Ok(Some(value)) => return Ok(value),
                Ok(None) => {}
                Err(message) => {
                    if let Some(category) = recovery::category(&message)
                        && let Some(handler) = routine.recovery.matching(ip, category)
                    {
                        frame.insert(
                            handler.key.clone(),
                            Cow::Borrowed(recovery::code(category).unwrap()),
                        );
                        trace.push(format!("recovered {category} into {}", handler.key));
                        next = handler.recover + 1;
                    } else {
                        return Err(message);
                    }
                }
            }
            ip = next;
        }
        Err("AIL-RUNTIME-RETURN: value function ended without a return value".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn i(opcode: &str, values: &[(&str, &str)]) -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(
            opcode,
            &values
                .iter()
                .map(|(k, v)| (*k, (*v).into()))
                .collect::<Vec<_>>(),
        )
    }
    fn program(
        params: &str,
        locals: &str,
        result: &str,
        body: Vec<AilBytecodeInstruction>,
    ) -> AilBytecodeProgram {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        let document = parse_ail_package_document(&package).unwrap();
        let mut program =
            compile_ail_core_bytecode(&elaborate_ail_core(&package, &document)).unwrap();
        let mut instructions = vec![i(
            "VALUE_FUNCTION",
            &[
                ("parameters", params),
                ("locals", locals),
                ("result", result),
            ],
        )];
        instructions.extend(body);
        program.actions.insert(
            "transform".into(),
            AilBytecodeAction {
                name: "transform".into(),
                instructions,
            },
        );
        program
    }
    #[test]
    fn construction_rejects_malformed_integer_dependencies_without_panicking() {
        for bad in [
            i("EVAL_INT", &[("expression", "i:1")]),
            i("JUMP", &[("label", "absent")]),
        ] {
            let mut program = program(
                "",
                "",
                "Int",
                vec![i("RETURN_VALUE", &[("expression", "i:1"), ("type", "Int")])],
            );
            program.actions.insert(
                "broken".into(),
                AilBytecodeAction {
                    name: "broken".into(),
                    instructions: vec![i("INTEGER_FUNCTION", &[("parameters", "")]), bad],
                },
            );
            assert!(Machine::new(&program).is_err());
        }
    }
    #[test]
    fn borrowed_parameters_and_one_return_copy_use_the_shared_budget() {
        let program = program(
            "value:Text",
            "",
            "Text",
            vec![i(
                "RETURN_VALUE",
                &[("expression", "f:local.value"), ("type", "Text")],
            )],
        );
        let machine = Machine::new(&program).unwrap();
        let args = integer_function::decode_arguments("value=f:data.value").unwrap();
        let state = BTreeMap::from([("data.value".into(), "Zoë 🙂".into())]);
        let (mut text, mut list) = (67_108_864 - "Zoë 🙂".len(), 17);
        let mut trace = Vec::new();
        assert_eq!(
            machine
                .call("transform", &args, &state, &mut trace, &mut text, &mut list)
                .unwrap(),
            "Zoë 🙂"
        );
        assert_eq!((text, list), (67_108_864, 17));
        assert!(trace.is_empty());
        assert!(
            machine
                .call("transform", &args, &state, &mut trace, &mut text, &mut list)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT-LIMIT:")
        );
        assert_eq!((text, list), (67_108_864, 17));
    }
    #[test]
    fn unused_parameters_are_validated_in_name_order() {
        let program = program(
            "a:Bool;z:Text",
            "",
            "Int",
            vec![i("RETURN_VALUE", &[("expression", "i:7"), ("type", "Int")])],
        );
        let machine = Machine::new(&program).unwrap();
        let args = integer_function::decode_arguments("a=f:data.flag;z=f:data.text").unwrap();
        let mut state = BTreeMap::from([
            ("data.flag".into(), "True".into()),
            ("data.text".into(), "x".repeat(1_048_577)),
        ]);
        let (mut text, mut list) = (0, 0);
        let mut trace = Vec::new();
        assert!(
            machine
                .call("transform", &args, &state, &mut trace, &mut text, &mut list)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-JSON:")
        );
        state.insert("data.flag".into(), "true".into());
        assert!(
            machine
                .call("transform", &args, &state, &mut trace, &mut text, &mut list)
                .unwrap_err()
                .starts_with("AIL-RUNTIME-TEXT-LIMIT:")
        );
        assert_eq!((text, list), (0, 0));
    }
    #[test]
    fn lists_can_be_inspected_before_canonical_expansion() {
        let input = format!("[\"{}\"]", "\u{7f}".repeat(174_763));
        let program = program(
            "items:List<Text>",
            "count:Int",
            "Int",
            vec![
                i(
                    "LIST_LENGTH",
                    &[
                        ("source", "local.items"),
                        ("key", "local.count"),
                        ("type", "Text"),
                    ],
                ),
                i(
                    "RETURN_VALUE",
                    &[("expression", "f:local.count"), ("type", "Int")],
                ),
            ],
        );
        let machine = Machine::new(&program).unwrap();
        let args = integer_function::decode_arguments("items=f:data.items").unwrap();
        let (mut text, mut list) = (0, 0);
        let mut trace = Vec::new();
        assert_eq!(
            machine
                .call(
                    "transform",
                    &args,
                    &BTreeMap::from([("data.items".into(), input)]),
                    &mut trace,
                    &mut text,
                    &mut list
                )
                .unwrap(),
            "1"
        );
        assert_eq!((text, list), (0, 0));
        assert!(trace.is_empty());
    }
    #[test]
    fn recursive_value_frames_keep_arguments_and_charge_each_successful_return_once() {
        let program = program(
            "n:Int;value:Text",
            "result:Text",
            "Text",
            vec![
                i(
                    "BRANCH_INT",
                    &[
                        ("left", "f:local.n"),
                        ("comparison", "<="),
                        ("right", "i:0"),
                        ("label", "done"),
                    ],
                ),
                i(
                    "CALL_VALUE",
                    &[
                        ("target", "transform"),
                        ("arguments", "n=f:local.n i:1 -;value=f:local.value"),
                        ("key", "local.result"),
                    ],
                ),
                i(
                    "RETURN_VALUE",
                    &[("expression", "f:local.result"), ("type", "Text")],
                ),
                i("LABEL", &[("name", "done")]),
                i(
                    "RETURN_VALUE",
                    &[("expression", "f:local.value"), ("type", "Text")],
                ),
            ],
        );
        let machine = Machine::new(&program).unwrap();
        let args = integer_function::decode_arguments("n=f:data.n;value=f:data.value").unwrap();
        for (depth, pass) in [(63, true), (64, false)] {
            let state = BTreeMap::from([
                ("data.n".into(), depth.to_string()),
                ("data.value".into(), "x".into()),
            ]);
            let (mut text, mut list) = (0, 0);
            let mut trace = Vec::new();
            let result = machine.call("transform", &args, &state, &mut trace, &mut text, &mut list);
            if pass {
                assert_eq!(result.unwrap(), "x");
                assert_eq!((text, list), (64, 0));
            } else {
                assert!(result.unwrap_err().starts_with("AIL-RUNTIME-DEPTH:"));
                assert_eq!((text, list), (0, 0));
            }
        }
    }
}
