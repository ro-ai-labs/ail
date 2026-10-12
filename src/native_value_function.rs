//! Typed recursive frames share the root arenas and the integer call-depth
//! counter. Parameters borrow validated caller storage; returns materialize once
//! before the frame disappears. Helpers and scratch never charge parameters.
use super::*;

pub(super) struct Target {
    label: String,
    signature: value_protocol::Signature,
    slots: BTreeMap<String, usize>,
    frame: usize,
}

pub(super) fn prepare(
    functions: &BTreeMap<String, AilBytecodeAction>,
    integers: &BTreeMap<String, FunctionTarget>,
    schemas: &AilValueSchemas,
) -> Result<BTreeMap<String, Target>, String> {
    let mut signatures = integers
        .iter()
        .map(|(name, target)| {
            (
                name.clone(),
                value_protocol::Signature {
                    parameters: target
                        .parameters
                        .iter()
                        .map(|p| (p.clone(), "Int".into()))
                        .collect(),
                    locals: BTreeMap::new(),
                    result: "Int".into(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (name, function) in functions {
        if value_protocol::is_function(function) {
            if name != &function.name || function.instructions.len() > 16_385 {
                return Err(
                    "native value function has mismatched name or too many instructions".into(),
                );
            }
            signatures.insert(name.clone(), value_protocol::signature(function, schemas)?);
        }
    }
    let mut targets = BTreeMap::new();
    for (index, (name, function)) in functions.iter().enumerate() {
        if !value_protocol::is_function(function) {
            continue;
        }
        let signature = signatures[name].clone();
        let fields = signature.fields();
        for instruction in function.instructions.iter().skip(1) {
            value_protocol::validate_body_instruction(
                instruction,
                &fields,
                &signature.result,
                &signatures,
                schemas,
            )?;
            if matches!(instruction.opcode.as_str(), "CALL_INT" | "CALL_VALUE") {
                let target = required(instruction, "target")?;
                let matches = if instruction.opcode == "CALL_INT" {
                    integers.contains_key(target)
                } else {
                    functions
                        .get(target)
                        .is_some_and(value_protocol::is_function)
                };
                if !matches {
                    return Err("native call opcode does not match function kind".into());
                }
            }
        }
        let frame = fields.len() * SLOT_SIZE + native_recovery::FUNCTION_CONTEXT;
        let slots = fields
            .into_keys()
            .enumerate()
            .map(|(i, name)| (name, i * SLOT_SIZE))
            .collect();
        targets.insert(
            name.clone(),
            Target {
                label: format!("value_function_{index}"),
                signature,
                slots,
                frame,
            },
        );
    }
    Ok(targets)
}

pub(super) fn emit_call(
    emitter: &mut Emitter,
    instruction: &AilBytecodeInstruction,
    targets: &BTreeMap<String, Target>,
) -> Result<(), String> {
    let target = targets
        .get(required(instruction, "target")?)
        .ok_or("unknown native value function")?;
    let signatures = BTreeMap::from([(
        required(instruction, "target")?.to_string(),
        target.signature.clone(),
    )]);
    value_protocol::validate_call(instruction, None, &signatures)?;
    let arguments = integer_function::decode_arguments(required(instruction, "arguments")?)?;
    // Every packet is (scalar-or-pointer, length). Validation runs in sorted
    // parameter order before the callee checks/enters the shared depth counter.
    for (name, expression) in &arguments {
        let kind = &target.signature.parameters[name];
        if kind == "Int" {
            emitter.expression(expression)?;
            emitter.code.emit(&[0x31, 0xd2]); // xor edx, edx
        } else {
            let IntExpr::Field(field) = expression else {
                unreachable!("validated bare field")
            };
            load_value(emitter, field);
            validate_borrow(emitter, kind)?;
        }
        emitter.code.emit(&[0x50, 0x52]); // push data; push length
    }
    emitter.code.emit_call_label(&target.label);
    if !arguments.is_empty() {
        emitter.code.emit(&[0x48, 0x81, 0xc4]);
        emitter
            .code
            .emit(&((arguments.len() * 16) as u32).to_le_bytes());
    }
    store_result(
        emitter,
        required(instruction, "key")?,
        &target.signature.result,
    )
}

fn load_value(emitter: &mut Emitter, field: &str) {
    emitter.load_rdi(emitter.slot(field) + POINTER);
    emitter.load_rsi(emitter.slot(field) + LENGTH);
}

// RDI/RSI transport -> RAX/RDX borrowed transport, or Bool scalar in RAX.
fn validate_borrow(emitter: &mut Emitter, kind: &str) -> Result<(), String> {
    if !value_protocol::supported(kind) {
        return native_value::boundary(emitter, kind, false);
    }
    if kind == "Float" {
        emitter.code.emit_call_label("float_parse");
        emitter.code.emit(&[0x31, 0xd2]);
        return Ok(());
    }
    if kind == "Bool" {
        emitter.code.emit_call_label("value_boolean");
        emitter.code.emit(&[0x31, 0xd2]);
        return Ok(());
    }
    emitter.code.emit(&[0x57, 0x56]); // retain original descriptor
    match kind {
        "Text" => emitter.code.emit_call_label("text_validate"),
        "Bytes" => emitter.code.emit_call_label("bytes_validate"),
        "List<Int>" | "List<Text>" | "List<Float>" => {
            emitter.code.emit_mov_edx_imm32(match kind {
                "List<Int>" => 0,
                "List<Text>" => 1,
                _ => 2,
            });
            emitter.code.emit_call_label("list_parse");
            emitter.code.emit_mov_rdi_rax();
            emitter.code.emit_call_label("list_release");
        }
        "Json" => {
            emitter.code.emit_call_label("structured_parse");
            emitter.code.emit_mov_rdi_rax();
            emitter.code.emit_call_label("structured_release");
        }
        _ => return Err("unsupported native value parameter type".into()),
    }
    emitter.code.emit(&[0x5a, 0x58]); // original length, pointer
    Ok(())
}

fn store_result(emitter: &mut Emitter, key: &str, kind: &str) -> Result<(), String> {
    if kind == "Int" {
        emitter.store_integer(key);
    } else if kind == "Float" {
        native_float::store(emitter, key);
    } else if kind == "Bool" {
        let no = format!("value_false_{}", emitter.next);
        let done = format!("value_bool_done_{}", emitter.next);
        emitter.next += 1;
        let truth = emitter.data(b"true");
        let falsehood = emitter.data(b"false");
        emitter.code.emit_test_rax_rax();
        emitter.code.emit_jcc_label(&[0x0f, 0x84], &no);
        emitter.code.emit_lea_rsi_label(&truth);
        emitter.store_rsi(emitter.slot(key) + POINTER);
        emitter.set(emitter.slot(key) + LENGTH, 4);
        emitter.code.emit_jmp_label(&done);
        emitter.code.label(no)?;
        emitter.code.emit_lea_rsi_label(&falsehood);
        emitter.store_rsi(emitter.slot(key) + POINTER);
        emitter.set(emitter.slot(key) + LENGTH, 5);
        emitter.code.label(done)?;
        emitter.set(emitter.slot(key) + NUMERIC, 0);
    } else {
        emitter.store_rax(emitter.slot(key) + POINTER);
        emitter.store_rdx(emitter.slot(key) + LENGTH);
        emitter.set(emitter.slot(key) + NUMERIC, 0);
    }
    Ok(())
}

fn emit_return(emitter: &mut Emitter, instruction: &AilBytecodeInstruction) -> Result<(), String> {
    let expression = IntExpr::from_postfix(required(instruction, "expression")?)?;
    let kind = required(instruction, "type")?;
    if kind == "Int" {
        return emitter.expression(&expression);
    }
    let IntExpr::Field(field) = expression else {
        return Err("native non-Int return must be a field".into());
    };
    load_value(emitter, &field);
    if !value_protocol::supported(kind) {
        return native_value::boundary(emitter, kind, true);
    }
    match kind {
        "Float" => emitter.code.emit_call_label("float_parse"),
        "Bool" => emitter.code.emit_call_label("value_boolean"),
        "Bytes" => {
            emitter.code.emit_call_label("bytes_validate");
            emitter.code.emit(&[0x48, 0x89, 0xf2, 0x48, 0x89, 0xfe]);
            emitter.code.emit_call_label("bytes_copy");
        }
        "Text" => {
            emitter.code.emit_call_label("text_validate");
            emitter.code.emit(&[0x48, 0x89, 0xf2, 0x48, 0x89, 0xfe]); // length rdx; bytes rsi
            emitter.code.emit_call_label("value_copy_text");
        }
        "List<Int>" | "List<Text>" | "List<Float>" => {
            emitter.code.emit_mov_edx_imm32(match kind {
                "List<Int>" => 0,
                "List<Text>" => 1,
                _ => 2,
            });
            emitter.code.emit_call_label("list_parse");
            emitter.code.emit(&[0x50]);
            emitter.code.emit_mov_rdi_rax();
            emitter.code.emit_call_label("list_encode");
            emitter.code.emit(&[0x48, 0x89, 0xc6]);
            emitter.code.emit_call_label("value_copy_list");
            emitter.code.emit(&[0x5f]);
            emitter.code.emit_call_label("list_release");
        }
        "Json" => {
            emitter.code.emit_call_label("structured_parse");
            emitter.code.emit(&[0x50]);
            emitter
                .code
                .emit(&[0x48, 0x8b, 0xb0, 0x40, 0x10, 0x30, 0x00]);
            emitter
                .code
                .emit(&[0x48, 0x8b, 0x90, 0x48, 0x10, 0x30, 0x00]);
            emitter.code.emit_call_label("json_copy_list");
            emitter.code.emit(&[0x5f]);
            emitter.code.emit_call_label("structured_release");
        }
        _ => return Err("unsupported native return type".into()),
    }
    Ok(())
}

pub(super) fn emit_function(
    emitter: &mut Emitter,
    function: &AilBytecodeAction,
    target: &Target,
    targets: &BTreeMap<String, Target>,
    integers: &BTreeMap<String, FunctionTarget>,
) -> Result<(), String> {
    let layout = recovery::Layout::analyze(function)?;
    let (labels, diagnostics) = ail_bytecode_action_labels(function);
    if !diagnostics.is_empty() {
        return Err(diagnostics.join("\n"));
    }
    emitter.slots = target.slots.clone();
    emitter.integer_frame = false;
    emitter.function_operations = true;
    emitter.code.label(&target.label)?;
    emitter.code.emit(&[0x49, 0x83, 0xff, 64]);
    emitter
        .code
        .emit_jcc_label(&[0x0f, 0x83], "error_function_depth");
    emitter
        .code
        .emit(&[0x49, 0xff, 0xc7, 0x53, 0x41, 0x56, 0x48, 0x81, 0xec]); // inc depth, save rbx/r14, sub rsp
    emitter.code.emit(&(target.frame as u32).to_le_bytes());
    emitter
        .code
        .emit(&[0x48, 0x89, 0xe3, 0x48, 0x89, 0xdf, 0x31, 0xc0, 0xb9]);
    emitter
        .code
        .emit(&((target.frame / 8) as u32).to_le_bytes());
    emitter.code.emit(&[0xf3, 0x48, 0xab, 0x49, 0xbe]);
    emitter
        .code
        .emit(&(function.instructions.len().saturating_mul(1024).max(1024) as u64).to_le_bytes());
    emitter.begin_function_context(target.frame);
    for (index, (name, kind)) in target.signature.parameters.iter().enumerate() {
        let offset = target.frame + 24 + (target.signature.parameters.len() - 1 - index) * 16;
        emitter.load_rax(offset + 8);
        emitter.load_rdx(offset);
        store_result(emitter, &format!("local.{name}"), kind)?;
    }
    let label = |index| format!("{}_instruction_{index}", target.label);
    let return_label = format!("{}_return", target.label);
    for (index, instruction) in function.instructions.iter().enumerate() {
        emitter.code.label(label(index))?;
        emitter.consume_fuel();
        let branch_target = if matches!(
            instruction.opcode.as_str(),
            "JUMP" | "BRANCH_INT" | "BRANCH_BOOL" | "BRANCH_FLOAT"
        ) {
            let target = labels
                .get(required(instruction, "label")?)
                .ok_or("unknown native value function branch label")?;
            emitter.recovery_target(&layout, index, *target, label(*target))?
        } else {
            String::new()
        };
        match instruction.opcode.as_str() {
            "VALUE_FUNCTION" | "LABEL" | "TRY_END" => {}
            "RAISE" => native_recovery::emit_raise(&mut emitter.code, instruction)?,
            "TRY_BEGIN" => emitter.begin_recovery(
                layout.beginning(index).unwrap(),
                &label(layout.beginning(index).unwrap().recover + 1),
            ),
            "RECOVER" => {
                emitter.pop_recovery(1);
                emitter
                    .code
                    .emit_jmp_label(&label(layout.recovering(index).unwrap().end + 1));
            }
            "EVAL_BOOL" => native_bool::emit_operation(emitter, instruction)?,
            "BRANCH_BOOL" => {
                native_bool::emit_branch(emitter, instruction, &branch_target)?;
            }
            "EVAL_FLOAT" | "INT_TO_FLOAT" | "FLOAT_TO_INT" => {
                native_float::emit_operation(emitter, instruction)?
            }
            "BRANCH_FLOAT" => {
                native_float::emit_branch(emitter, instruction, &branch_target)?;
            }
            "EVAL_INT" => {
                emitter.expression(&IntExpr::from_postfix(required(
                    instruction,
                    "expression",
                )?)?)?;
                emitter.store_integer(required(instruction, "key")?);
            }
            "VALUE_CONSTRUCT" | "VALUE_TEST" | "VALUE_GET" => {
                native_value::emit_operation(emitter, instruction)?
            }
            "CALL_INT" => emitter.integer_call(instruction, integers)?,
            "CALL_VALUE" => emit_call(emitter, instruction, targets)?,
            "RETURN_VALUE" => {
                emit_return(emitter, instruction)?;
                emitter.code.emit_jmp_label(&return_label);
            }
            "ASSIGN_LITERAL" => {
                let value = literal_assignment::runtime_value(instruction)?;
                if required(instruction, "type")? == "Float" {
                    native_float::literal(emitter, &value)?;
                    native_float::store(emitter, required(instruction, "key")?);
                    continue;
                }
                let data = emitter.data(value.as_bytes());
                let slot = emitter.slot(required(instruction, "key")?);
                emitter.code.emit_lea_rsi_label(&data);
                emitter.store_rsi(slot + POINTER);
                emitter.set(slot + LENGTH, value.len() as u64);
                emitter.set(slot + NUMERIC, 0);
            }
            "BYTES_LENGTH" | "BYTES_GET" | "BYTES_SLICE" | "BYTES_CONCAT" | "BYTES_SET"
            | "BYTES_FROM_UTF8" | "BYTES_TO_UTF8" | "BYTES_FROM_HEX" | "BYTES_TO_HEX"
            | "BYTES_COMPARE" => native_bytes::emit_operation(emitter, instruction)?,
            "TEXT_JOIN" | "TEXT_LENGTH" | "TEXT_SLICE" | "TEXT_FIND" | "TEXT_SPLIT"
            | "TEXT_JOIN_LIST" | "TEXT_REPLACE" | "TEXT_TRIM" => {
                native_text::emit_operation(emitter, instruction)?
            }
            "LIST_EMPTY" | "LIST_LENGTH" | "LIST_GET" | "LIST_APPEND" | "LIST_SET"
            | "LIST_CONCAT" | "LIST_SLICE" | "LIST_REVERSE" | "LIST_SORT" => {
                native_list::emit_operation(emitter, instruction)?
            }
            "JSON_PARSE" | "JSON_FORMAT" | "JSON_GET" | "JSON_COUNT" | "JSON_KIND"
            | "JSON_KEYS" => native_json::emit_operation(emitter, instruction)?,
            "JSON_SET" | "JSON_REMOVE" => {
                native_json_mutation::emit_operation(emitter, instruction)?
            }
            "JUMP" | "BRANCH_INT" => {
                if instruction.opcode == "JUMP" {
                    emitter.code.emit_jmp_label(&branch_target);
                } else {
                    let condition = crate::condition::IntCondition::from_postfix(
                        required(instruction, "left")?,
                        required(instruction, "comparison")?,
                        required(instruction, "right")?,
                    )?;
                    emitter.expression(&condition.left)?;
                    emitter.code.emit(&[0x50]);
                    emitter.expression(&condition.right)?;
                    emitter
                        .code
                        .emit(&[0x48, 0x89, 0xc1, 0x58, 0x48, 0x39, 0xc8]);
                    let opcode = match condition.comparison {
                        crate::condition::IntComparison::Equal => 0x84,
                        crate::condition::IntComparison::NotEqual => 0x85,
                        crate::condition::IntComparison::Less => 0x8c,
                        crate::condition::IntComparison::LessEqual => 0x8e,
                        crate::condition::IntComparison::Greater => 0x8f,
                        crate::condition::IntComparison::GreaterEqual => 0x8d,
                    };
                    emitter.code.emit_jcc_label(&[0x0f, opcode], &branch_target);
                }
            }
            "EMIT_TRACE" => emitter.trace(&format!("trace {}\n", required(instruction, "event")?)),
            _ => return Err("unsupported native value function instruction".into()),
        }
    }
    emitter.code.label(label(function.instructions.len()))?;
    emitter.code.emit_jmp_label("error_function_return");
    emitter.code.label(return_label)?;
    emitter.end_function_context(target.frame);
    emitter.code.emit(&[0x48, 0x89, 0xdc, 0x48, 0x81, 0xc4]);
    emitter.code.emit(&(target.frame as u32).to_le_bytes());
    emitter
        .code
        .emit(&[0x41, 0x5e, 0x5b, 0x49, 0xff, 0xcf, 0xc3]);
    emitter.function_operations = false;
    Ok(())
}

pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    // Exact Bool transport, using JSON diagnostics without allocating text.
    code.label("value_boolean")?;
    code.emit_call_label("json_text_validate");
    code.emit(&[0x48, 0x83, 0xfe, 0x04]); // len == 4
    code.emit_jcc_label(&[0x0f, 0x85], "value_boolean_false");
    code.emit(&[0x81, 0x3f, 0x74, 0x72, 0x75, 0x65]); // true
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0xb8, 1, 0, 0, 0, 0xc3]);
    code.label("value_boolean_false")?;
    code.emit(&[0x48, 0x83, 0xfe, 0x05]);
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x81, 0x3f, 0x66, 0x61, 0x6c, 0x73]); // fals
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x80, 0x7f, 0x04, 0x65]); // e
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x31, 0xc0, 0xc3]);
    code.label("value_copy_text")?;
    code.emit(&[0x56]);
    code.emit_call_label("text_allocate");
    code.emit_jmp_label("value_copy_finish");
    code.label("value_copy_list")?;
    code.emit(&[0x56]);
    code.emit_call_label("list_allocate");
    code.label("value_copy_finish")?;
    code.emit(&[
        0x5e, 0x48, 0x89, 0xc7, 0x48, 0x89, 0xd1, 0x50, 0xf3, 0xa4, 0x58, 0xc3,
    ]);
    Ok(())
}
