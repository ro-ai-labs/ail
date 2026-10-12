//! Direct x86-64 scalar execution. Values are mutable pointer/length pairs;
//! numeric writes own a decimal buffer so copies cannot alias a later write.
use super::*;
use crate::expression::{IntExpr, IntOperator};

#[path = "native_bool.rs"]
mod native_bool;
#[path = "native_bytes.rs"]
mod native_bytes;
#[path = "native_directory.rs"]
mod native_directory;
#[path = "native_file.rs"]
mod native_file;
#[path = "native_float.rs"]
mod native_float;
#[path = "native_json.rs"]
mod native_json;
#[path = "native_json_mutation.rs"]
mod native_json_mutation;
#[path = "native_list.rs"]
mod native_list;
#[path = "native_list_transform.rs"]
mod native_list_transform;
#[path = "native_process.rs"]
mod native_process;
#[path = "native_recovery.rs"]
mod native_recovery;
#[path = "native_stdio.rs"]
mod native_stdio;
#[path = "native_structured_json.rs"]
mod native_structured_json;
#[path = "native_text.rs"]
mod native_text;
#[path = "native_text_transform.rs"]
mod native_text_transform;
#[path = "native_value.rs"]
mod native_value;
#[path = "native_value_function.rs"]
mod native_value_function;

const SLOT_SIZE: usize = 64;
const POINTER: usize = 0;
const LENGTH: usize = 8;
const SELECTED: usize = 16;
const NUMERIC: usize = 24;
const BUFFER: usize = 32;

pub(super) fn legacy_stdio_initialize(code: &mut X64Code) -> Result<(), String> {
    native_stdio::legacy_initialize(code)
}

pub(super) fn legacy_stdio_skip(code: &mut X64Code, target: &str) {
    native_stdio::skip(code, target);
}

pub(super) fn legacy_process_initialize(code: &mut X64Code) -> Result<(), String> {
    native_process::validate_startup(code)
}

pub(super) fn legacy_directory_initialize(code: &mut X64Code) -> Result<(), String> {
    native_directory::validate_startup(code)
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn emit(
    action: &AilBytecodeAction,
    failures: &BTreeMap<String, AilBytecodeFailure>,
) -> Result<Vec<u8>, String> {
    emit_with_functions(action, failures, &BTreeMap::new(), &BTreeMap::new())
}

pub(super) fn emit_with_functions(
    action: &AilBytecodeAction,
    failures: &BTreeMap<String, AilBytecodeFailure>,
    functions: &BTreeMap<String, AilBytecodeAction>,
    schemas: &AilValueSchemas,
) -> Result<Vec<u8>, String> {
    if action
        .instructions
        .first()
        .is_some_and(|i| matches!(i.opcode.as_str(), "INTEGER_FUNCTION" | "VALUE_FUNCTION"))
    {
        return Err("executable function cannot execute as a root action".into());
    }
    let layout = recovery::Layout::analyze_expanded(action)?;
    let function_targets = prepare_functions(functions)?;
    let value_targets = native_value_function::prepare(functions, &function_targets, schemas)?;
    let has_bool = std::iter::once(action)
        .chain(functions.values())
        .any(|procedure| {
            procedure
                .instructions
                .iter()
                .any(|i| matches!(i.opcode.as_str(), "EVAL_BOOL" | "BRANCH_BOOL"))
        });
    let has_value = !schemas.is_empty();
    let has_bytes = has_value
        || std::iter::once(action)
            .chain(functions.values())
            .any(|procedure| {
                procedure.instructions.iter().any(|i| {
                    native_bytes::is_operation(&i.opcode)
                        || matches!(i.opcode.as_str(), "FILE_READ_BYTES" | "FILE_WRITE_BYTES")
                        || matches!(i.opcode.as_str(), "ASSIGN_LITERAL" | "RETURN_VALUE")
                            && i.operands.get("type").is_some_and(|v| v == "Bytes")
                        || i.opcode == "VALUE_FUNCTION"
                            && (i.operands.get("result").is_some_and(|v| v == "Bytes")
                                || ["parameters", "locals"].iter().any(|role| {
                                    i.operands.get(*role).is_some_and(|v| {
                                        v.split(';').any(|part| part.ends_with(":Bytes"))
                                    })
                                }))
                })
            });
    let has_float = std::iter::once(action)
        .chain(functions.values())
        .any(|procedure| {
            procedure
                .instructions
                .iter()
                .any(native_float::needs_runtime)
        });
    let has_process = action
        .instructions
        .iter()
        .any(|i| i.opcode == "PROCESS_RUN");
    let has_directory = action
        .instructions
        .iter()
        .any(|i| native_directory::is_operation(&i.opcode));
    let (labels, diagnostics) = ail_bytecode_action_labels(action);
    if !diagnostics.is_empty() {
        return Err(diagnostics.join("\n"));
    }
    let mut fields = BTreeSet::new();
    let mut expressions = BTreeMap::new();
    let mut comparisons = BTreeMap::new();
    for (index, instruction) in action.instructions.iter().enumerate() {
        if instruction.opcode.starts_with("VALUE_") {
            fields.extend(value_operations::fields(instruction, schemas)?.into_keys());
        }
        fields.extend(native_bytes::fields(instruction)?);
        fields.extend(native_bool::fields(instruction)?);
        fields.extend(native_float::fields(instruction)?);
        fields.extend(native_text::fields(instruction)?);
        fields.extend(native_list::fields(instruction)?);
        fields.extend(native_json::fields(instruction)?);
        fields.extend(native_json_mutation::fields(instruction)?);
        fields.extend(native_stdio::fields(instruction)?);
        fields.extend(native_process::fields(instruction)?);
        fields.extend(native_directory::fields(instruction)?);
        for operand in ["key", "source"] {
            if let Some(field) = instruction.operands.get(operand) {
                fields.insert(field.clone());
            }
        }
        if matches!(instruction.opcode.as_str(), "CALL_INT" | "CALL_VALUE") {
            for argument in
                integer_function::decode_arguments(required(instruction, "arguments")?)?.values()
            {
                fields.extend(argument.fields());
            }
        } else if instruction.opcode == "EVAL_INT" {
            let expression = IntExpr::from_postfix(required(instruction, "expression")?)?;
            fields.extend(expression.fields());
            expressions.insert(index, expression);
        } else if instruction.opcode == "BRANCH_INT" {
            let left = IntExpr::from_postfix(required(instruction, "left")?)?;
            let right = IntExpr::from_postfix(required(instruction, "right")?)?;
            // Jump over the taken path when the signed comparison is false.
            let skipped_opcode = match required(instruction, "comparison")? {
                "==" => 0x85, // jne
                "!=" => 0x84, // je
                "<" => 0x8d,  // jge
                "<=" => 0x8f, // jg
                ">" => 0x8e,  // jle
                ">=" => 0x8c, // jl
                comparison => {
                    return Err(format!("invalid native integer comparison '{comparison}'"));
                }
            };
            fields.extend(left.fields());
            fields.extend(right.fields());
            comparisons.insert(index, (left, right, skipped_opcode));
        }
    }
    let slots = fields
        .into_iter()
        .enumerate()
        .map(|(index, key)| (key, index * SLOT_SIZE))
        .collect::<BTreeMap<_, _>>();
    let grant_base = slots
        .len()
        .checked_mul(SLOT_SIZE)
        .ok_or("native scalar state is too large")?;
    let file_grants = native_file::grants(action, grant_base)?;
    let process_grants = native_process::grants(action, grant_base + file_grants.len() * 8)?;
    let directory_grants = native_directory::grants(
        action,
        grant_base + (file_grants.len() + process_grants.len()) * 8,
    )?;
    let frame = file_grants
        .len()
        .checked_add(process_grants.len() + directory_grants.len())
        .ok_or("native scalar state is too large")?
        .checked_mul(8)
        .and_then(|n| grant_base.checked_add(n))
        .and_then(|n| n.checked_add(native_recovery::ROOT_BYTES + 8))
        .filter(|n| *n <= i32::MAX as usize)
        .ok_or("native scalar state is too large")?;
    let mut emitter = Emitter {
        code: X64Code::default(),
        data: Vec::new(),
        next: 0,
        slots,
        selection_flag: frame - native_recovery::ROOT_BYTES - 8,
        recovery_jumps: Vec::new(),
        integer_frame: false,
        function_operations: false,
        value_schemas: schemas.clone(),
        value_schema: String::new(),
        value_schema_len: 0,
    };
    if has_value {
        let encoded = value_types::encode(schemas)?;
        emitter.value_schema_len = encoded.len();
        emitter.value_schema = emitter.data(&encoded);
    }
    let fuel = match action.instructions.first() {
        Some(instruction) if instruction.opcode == "NATIVE_FUEL_ROOT" => fuel_limit(instruction)?,
        _ => action.instructions.len().saturating_mul(1024).max(1024),
    };
    emitter.initialize(frame, fuel)?;
    native_stdio::initialize(&mut emitter.code)?;
    native_file::initialize(&mut emitter, &file_grants);
    native_process::initialize(&mut emitter, &process_grants)?;
    native_directory::initialize(&mut emitter, &directory_grants)?;
    for (index, instruction) in action.instructions.iter().enumerate() {
        emitter.code.label(format!("instruction_{index}"))?;
        let consumes_fuel = match instruction.opcode.as_str() {
            "NATIVE_FUEL_ROOT"
            | "NATIVE_FUEL_SCOPE_BEGIN"
            | "NATIVE_FUEL_SCOPE_END"
            | "NATIVE_OUTPUT_SCOPE_BEGIN"
            | "NATIVE_OUTPUT_SCOPE_END" => false,
            "NATIVE_TRACE_LINE" => instruction
                .operands
                .get("fuel")
                .is_none_or(|fuel| fuel != "0"),
            _ => true,
        };
        if consumes_fuel {
            emitter.consume_fuel();
        }
        let key = || required(instruction, "key");
        let branch_target = if matches!(
            instruction.opcode.as_str(),
            "JUMP" | "BRANCH_INT" | "BRANCH_BOOL" | "BRANCH_FLOAT" | "BRANCH_FIELD_EQUALS"
        ) {
            let target = labels
                .get(required(instruction, "label")?)
                .ok_or("unknown native branch label")?;
            emitter.recovery_target(&layout, index, *target, format!("instruction_{target}"))?
        } else {
            String::new()
        };
        match instruction.opcode.as_str() {
            "STDIN_READ_TEXT" | "STDOUT_WRITE_TEXT" | "STDERR_WRITE_TEXT" => {
                native_stdio::emit_operation(&mut emitter, instruction)?;
            }
            "EVAL_BOOL" => native_bool::emit_operation(&mut emitter, instruction)?,
            "DIRECTORY_LIST_NAMES" | "DIRECTORY_READ_TEXT" | "DIRECTORY_WRITE_TEXT" => {
                native_directory::emit_operation(&mut emitter, instruction, &directory_grants)?;
            }
            "PROCESS_RUN" => {
                native_process::emit_operation(&mut emitter, instruction, &process_grants)?
            }
            "BRANCH_BOOL" => {
                native_bool::emit_branch(&mut emitter, instruction, &branch_target)?;
            }
            "EVAL_FLOAT" | "INT_TO_FLOAT" | "FLOAT_TO_INT" => {
                native_float::emit_operation(&mut emitter, instruction)?
            }
            "BRANCH_FLOAT" => {
                native_float::emit_branch(&mut emitter, instruction, &branch_target)?;
            }
            "TEXT_JOIN" | "TEXT_LENGTH" | "TEXT_SLICE" | "TEXT_FIND" | "TEXT_SPLIT"
            | "TEXT_JOIN_LIST" | "TEXT_REPLACE" | "TEXT_TRIM" => {
                native_text::emit_operation(&mut emitter, instruction)?;
            }
            "LIST_EMPTY" | "LIST_LENGTH" | "LIST_GET" | "LIST_APPEND" | "LIST_SET"
            | "LIST_CONCAT" | "LIST_SLICE" | "LIST_REVERSE" | "LIST_SORT" => {
                native_list::emit_operation(&mut emitter, instruction)?;
            }
            "BYTES_LENGTH" | "BYTES_GET" | "BYTES_SLICE" | "BYTES_CONCAT" | "BYTES_SET"
            | "BYTES_FROM_UTF8" | "BYTES_TO_UTF8" | "BYTES_FROM_HEX" | "BYTES_TO_HEX"
            | "BYTES_COMPARE" => native_bytes::emit_operation(&mut emitter, instruction)?,
            "FILE_READ_BYTES" | "FILE_WRITE_BYTES" | "FILE_READ_TEXT" | "FILE_WRITE_TEXT" => {
                native_file::emit_operation(&mut emitter, instruction, &file_grants)?;
            }
            "JSON_SET" | "JSON_REMOVE" => {
                native_json_mutation::emit_operation(&mut emitter, instruction)?
            }
            "JSON_PARSE" | "JSON_FORMAT" | "JSON_GET" | "JSON_COUNT" | "JSON_KIND"
            | "JSON_KEYS" => native_json::emit_operation(&mut emitter, instruction)?,
            "VALUE_CONSTRUCT" | "VALUE_TEST" | "VALUE_GET" => {
                native_value::emit_operation(&mut emitter, instruction)?
            }
            "CALL_INT" => emitter.integer_call(instruction, &function_targets)?,
            "CALL_VALUE" => {
                native_value_function::emit_call(&mut emitter, instruction, &value_targets)?
            }
            "EVAL_INT" => {
                emitter.expression(&expressions[&index])?;
                emitter.store_integer(key()?);
                emitter.trace(&format!("computed {}\n", key()?));
            }
            "ADD_INT_FIELD" => {
                let delta = required(instruction, "delta")?
                    .parse::<i64>()
                    .map_err(|_| "native ADD_INT_FIELD delta must be a signed 64-bit integer")?;
                emitter.integer(key()?);
                emitter.code.emit(&[0x48, 0xb9]); // mov rcx, imm64
                emitter.code.emit(&delta.to_le_bytes());
                emitter.code.emit(&[0x48, 0x01, 0xc8]); // add rax, rcx
                emitter.code.emit_jcc_label(&[0x0f, 0x80], "error_overflow");
                emitter.store_integer(key()?);
                emitter.trace_value(&format!("add {} by {delta} -> ", key()?), key()?);
            }
            "ASSIGN_LITERAL" => {
                let value = literal_assignment::runtime_value(instruction)?;
                if required(instruction, "type")? == "Float" {
                    native_float::literal(&mut emitter, &value)?;
                    native_float::store(&mut emitter, key()?);
                    emitter.trace(&format!("assigned literal {}\n", key()?));
                    continue;
                }
                let label = emitter.data(value.as_bytes());
                emitter.code.emit_lea_rsi_label(&label);
                emitter.store_rsi(emitter.slot(key()?) + POINTER);
                emitter.set(emitter.slot(key()?) + LENGTH, value.len() as u64);
                emitter.set(emitter.slot(key()?) + NUMERIC, 0);
                emitter.trace(&format!("assigned literal {}\n", key()?));
            }
            "SET_FIELD" => {
                let value = required(instruction, "value")?;
                let label = emitter.data(value.as_bytes());
                emitter.code.emit_lea_rsi_label(&label);
                emitter.store_rsi(emitter.slot(key()?) + POINTER);
                emitter.set(emitter.slot(key()?) + LENGTH, value.len() as u64);
                emitter.set(emitter.slot(key()?) + NUMERIC, 0);
                emitter.trace(&format!("write {}={value}\n", key()?));
            }
            "COPY_FIELD" => emitter.copy(key()?, required(instruction, "source")?, index)?,
            "OUTPUT_FIELD" => {
                emitter.set(emitter.slot(key()?) + SELECTED, 1);
                emitter.set(emitter.selection_flag, 1);
            }
            "NATIVE_OUTPUT_SCOPE_BEGIN" => emitter.begin_output_scope(),
            "NATIVE_OUTPUT_SCOPE_END" => {
                // RETURN_SUCCESS consumes fuel before the VM validates selected
                // outputs, so preserve that error order after removing a return.
                match instruction.operands.get("return_cost").map(String::as_str) {
                    None | Some("0") => {}
                    Some("1") => emitter.consume_fuel(),
                    _ => return Err("native output scope return_cost must be 0 or 1".to_string()),
                }
                emitter.end_output_scope()?;
            }
            "NATIVE_FUEL_ROOT" => {}
            "NATIVE_FUEL_SCOPE_BEGIN" => emitter.begin_fuel_scope(fuel_limit(instruction)?),
            "NATIVE_FUEL_SCOPE_END" => {
                match required(instruction, "return_cost")? {
                    "0" => {}
                    "1" => emitter.consume_fuel(),
                    _ => return Err("native fuel scope return_cost must be 0 or 1".to_string()),
                }
                emitter.end_fuel_scope();
            }
            "TRY_BEGIN" => emitter.begin_recovery(
                layout.beginning(index).unwrap(),
                &format!(
                    "instruction_{}",
                    layout.beginning(index).unwrap().recover + 1
                ),
            ),
            "RECOVER" => {
                emitter.pop_recovery(1);
                emitter.code.emit_jmp_label(&format!(
                    "instruction_{}",
                    layout.recovering(index).unwrap().end + 1
                ));
            }
            "TRY_END" => {}
            "RAISE" => native_recovery::emit_raise(&mut emitter.code, instruction)?,
            "RETURN_SUCCESS" => {
                emitter.pop_recovery(layout.protected(index).len());
                emitter.code.emit_jmp_label("success");
            }
            "LABEL" => {}
            "BRANCH_INT" => {
                let label = required(instruction, "label")?;
                let (left, right, skipped_opcode) = &comparisons[&index];
                emitter.expression(left)?;
                emitter.code.emit(&[0x50]); // push rax
                emitter.expression(right)?;
                emitter.code.emit(&[
                    0x48, 0x89, 0xc1, // mov rcx, rax
                    0x58, // pop rax
                    0x48, 0x39, 0xc8, // cmp rax, rcx
                ]);
                // Signed Jcc uses SF/OF together, so comparing MIN with MAX
                // remains valid even when their arithmetic difference overflows.
                let skipped = format!("branch_skipped_{index}");
                emitter
                    .code
                    .emit_jcc_label(&[0x0f, *skipped_opcode], &skipped);
                emitter.trace(&format!("branch {label} taken\n"));
                emitter.code.emit_jmp_label(&branch_target);
                emitter.code.label(skipped)?;
                emitter.trace(&format!("branch {label} skipped\n"));
            }
            "JUMP" | "BRANCH_FIELD_EQUALS" => {
                let label = required(instruction, "label")?;
                if instruction.opcode == "JUMP" {
                    emitter.trace(&format!("jump {label}\n"));
                    emitter.code.emit_jmp_label(&branch_target);
                } else {
                    let skipped = format!("branch_skipped_{index}");
                    emitter.compare_value(key()?, required(instruction, "value")?);
                    emitter.code.emit(&[0x85, 0xc0]);
                    emitter.code.emit_jcc_label(&[0x0f, 0x85], &skipped);
                    emitter.trace(&format!("branch {label} taken\n"));
                    emitter.code.emit_jmp_label(&branch_target);
                    emitter.code.label(skipped)?;
                    emitter.trace(&format!("branch {label} skipped\n"));
                }
            }
            "REQUIRE_EXISTS"
            | "REQUIRE_FIELD_NOT_EQUALS"
            | "REQUIRE_FIELD_IN"
            | "REQUIRE_FIELD_AFTER" => {
                let failed = format!("require_failed_{index}");
                let passed = format!("require_passed_{index}");
                match instruction.opcode.as_str() {
                    "REQUIRE_EXISTS" => {
                        emitter.load_rax(emitter.slot(key()?) + POINTER);
                        emitter.code.emit_test_rax_rax();
                        emitter.code.emit_jcc_label(&[0x0f, 0x84], &failed);
                    }
                    "REQUIRE_FIELD_NOT_EQUALS" => {
                        emitter.compare_value(key()?, required(instruction, "value")?);
                        emitter.code.emit(&[0x85, 0xc0]);
                        emitter.code.emit_jcc_label(&[0x0f, 0x84], &failed);
                    }
                    "REQUIRE_FIELD_IN" => {
                        for value in decode_ail_bytecode_list(required(instruction, "values")?) {
                            emitter.compare_value(key()?, &value);
                            emitter.code.emit(&[0x85, 0xc0]);
                            emitter.code.emit_jcc_label(&[0x0f, 0x84], &passed);
                        }
                        emitter.code.emit_jmp_label(&failed);
                    }
                    "REQUIRE_FIELD_AFTER" => {
                        emitter.compare_fields(required(instruction, "source")?, key()?);
                        emitter.code.emit(&[0x83, 0xf8, 0x01]); // cmp eax, 1
                        emitter.code.emit_jcc_label(&[0x0f, 0x85], &failed);
                    }
                    _ => unreachable!(),
                }
                emitter.code.emit_jmp_label(&passed);
                emitter.code.label(failed)?;
                for (index, line) in
                    native_failure_trace_lines(&[], instruction.operands.get("failure"), failures)
                        .into_iter()
                        .enumerate()
                {
                    if index == 0 {
                        emitter.diagnostic(&line);
                    } else {
                        emitter.trace(&line);
                    }
                }
                emitter.code.emit_exit(1);
                emitter.code.label(passed)?;
                emitter.trace(&format!(
                    "rule passed: {}\n",
                    required(instruction, "rule")?
                ));
            }
            "ACTION_BEGIN" => emitter.trace(&format!(
                "action {} started\n",
                required(instruction, "action")?
            )),
            "NATIVE_TRACE_LINE" => emitter.trace(&format!("{}\n", required(instruction, "text")?)),
            "EMIT_TRACE" => emitter.trace(&format!("trace {}\n", required(instruction, "event")?)),
            "OBSERVE_RULE" => emitter.trace(&format!(
                "rule observed: {}\n",
                required(instruction, "rule")?
            )),
            "ASSERT_GUARANTEE" => emitter.trace(&format!(
                "guarantee checked: {}\n",
                required(instruction, "text")?
            )),
            "READ_FIELD" => emitter.trace(&format!("read {}\n", key()?)),
            "READ_EFFECT" => emitter.trace(&format!("read {}\n", required(instruction, "text")?)),
            "WRITE_FIELD" => emitter.trace(&format!("write {}\n", key()?)),
            "EFFECT" => emitter.trace(&format!("effect {}\n", required(instruction, "text")?)),
            "FUNCTION_BEGIN" => emitter.trace(&format!(
                "function {} started\n",
                required(instruction, "label")?
            )),
            "FUNCTION_INPUT" | "FUNCTION_OUTPUT" => emitter.trace(&format!(
                "function {} {}:{}\n",
                if instruction.opcode == "FUNCTION_INPUT" {
                    "input"
                } else {
                    "output"
                },
                required(instruction, "name")?,
                required(instruction, "type")?
            )),
            "FUNCTION_BRANCH" => emitter.trace(&format!(
                "function branch {}\n",
                required(instruction, "condition")?
            )),
            "FUNCTION_CALL" => emitter.trace(&format!(
                "function call {}\n",
                required(instruction, "target")?
            )),
            "FUNCTION_RETURN" => emitter.trace(&format!(
                "function return {}\n",
                required(instruction, "value")?
            )),
            opcode => {
                return Err(format!(
                    "unsupported native scalar opcode '{opcode}' in action '{}'",
                    action.name
                ));
            }
        }
    }
    emitter
        .code
        .label(format!("instruction_{}", action.instructions.len()))?;
    emitter.code.label("success")?;
    emitter.outputs()?;
    emitter.code.emit_exit(0);
    for (name, function) in functions {
        if let Some(target) = function_targets.get(name) {
            emitter.integer_function(function, target, &function_targets)?;
        } else {
            native_value_function::emit_function(
                &mut emitter,
                function,
                &value_targets[name],
                &value_targets,
                &function_targets,
            )?;
        }
    }
    emitter.emit_recovery_jumps()?;
    native_recovery::emit_helpers(&mut emitter)?;
    for (label, message) in [
        (
            "error_int",
            "AIL-RUNTIME-INT: missing or invalid signed decimal integer\n",
        ),
        (
            "error_overflow",
            "AIL-RUNTIME-OVERFLOW: signed 64-bit arithmetic overflow\n",
        ),
        ("error_divzero", "AIL-RUNTIME-DIVZERO: division by zero\n"),
        (
            "error_float",
            "AIL-RUNTIME-FLOAT: missing or invalid floating value or conversion\n",
        ),
        (
            "error_float_limit",
            "AIL-RUNTIME-FLOAT-LIMIT: floating input exceeds 1048576 bytes\n",
        ),
        (
            "error_output",
            "AIL-RUNTIME-OUTPUT: selected output field is missing\n",
        ),
        (
            "error_steps",
            "AIL bytecode execution step limit exceeded\n",
        ),
        (
            "error_function_depth",
            "AIL-RUNTIME-DEPTH: integer function call depth exceeded\n",
        ),
        (
            "error_function_return",
            "AIL-RUNTIME-RETURN: integer function reached the end without returning\n",
        ),
        (
            "error_text",
            "AIL-RUNTIME-TEXT: missing or invalid UTF-8 text\n",
        ),
        (
            "error_bytes",
            "AIL-RUNTIME-BYTES: missing or malformed byte value\n",
        ),
        (
            "error_bytes_limit",
            "AIL-RUNTIME-BYTES-LIMIT: byte size or allocation budget exceeded\n",
        ),
        ("error_bytes_io", "AIL-RUNTIME-IO: byte operation failed\n"),
        (
            "error_text_limit",
            "AIL-RUNTIME-TEXT-LIMIT: text size or allocation budget exceeded\n",
        ),
        (
            "error_text_allocation",
            "AIL-RUNTIME-TEXT-LIMIT: unable to allocate native text storage\n",
        ),
        (
            "error_list",
            "AIL-RUNTIME-LIST: missing or malformed typed list\n",
        ),
        (
            "error_list_limit",
            "AIL-RUNTIME-LIST-LIMIT: list size or allocation budget exceeded\n",
        ),
        (
            "error_list_allocation",
            "AIL-RUNTIME-LIST-LIMIT: unable to allocate native list storage\n",
        ),
        (
            "error_index",
            "AIL-RUNTIME-INDEX: list index is outside its range\n",
        ),
        (
            "error_capability",
            "AIL-RUNTIME-CAPABILITY: file resource grant is absent\n",
        ),
        (
            "error_file_grant",
            "AIL-RUNTIME-CAPABILITY: malformed file resource grant\n",
        ),
        (
            "error_io",
            "AIL-RUNTIME-IO: file operation failed or target is not a regular file\n",
        ),
        (
            "error_stdio_capability",
            "AIL-RUNTIME-CAPABILITY: standard-stream grant is absent\n",
        ),
        (
            "error_stdio_io",
            "AIL-RUNTIME-IO: standard-stream operation failed\n",
        ),
        (
            "error_directory_capability",
            "AIL-RUNTIME-CAPABILITY: directory resource grant is absent\n",
        ),
        (
            "error_directory_io",
            "AIL-RUNTIME-IO: directory operation failed\n",
        ),
        (
            "error_path",
            "AIL-RUNTIME-PATH: invalid or unusable directory path\n",
        ),
        (
            "error_process",
            "AIL-RUNTIME-PROCESS: invalid process invocation or child setup/exec failure\n",
        ),
        (
            "error_timeout",
            "AIL-RUNTIME-TIMEOUT: process deadline expired\n",
        ),
        (
            "error_process_limit",
            "AIL-RUNTIME-PROCESS-LIMIT: process argument count or byte limit exceeded\n",
        ),
        (
            "error_process_io",
            "AIL-RUNTIME-IO: process host operation failed\n",
        ),
        (
            "error_process_capability",
            "AIL-RUNTIME-CAPABILITY: process resource grant is absent\n",
        ),
        (
            "error_value",
            "AIL-RUNTIME-VALUE: missing or malformed variant value\n",
        ),
        (
            "error_value_limit",
            "AIL-RUNTIME-VALUE-LIMIT: variant value resource limit exceeded\n",
        ),
        (
            "error_value_io",
            "AIL-RUNTIME-VALUE-IO: variant runtime contract failure\n",
        ),
        (
            "error_json",
            "AIL-RUNTIME-JSON: missing or malformed JSON conversion value\n",
        ),
        (
            "error_json_path",
            "AIL-RUNTIME-JSON-PATH: malformed, unresolved, or ambiguous JSON pointer\n",
        ),
        (
            "error_json_limit",
            "AIL-RUNTIME-JSON-LIMIT: JSON size or allocation limit exceeded\n",
        ),
    ] {
        if let Some(category) = recovery::category(message) {
            emitter.code.label(label)?;
            emitter.code.emit(&[0xbf]); // mov edi, category index
            emitter.code.emit(
                &(recovery::CATEGORIES
                    .iter()
                    .position(|c| *c == category)
                    .unwrap() as u32)
                    .to_le_bytes(),
            );
            emitter.code.emit_jmp_label("recovery_raise");
            emitter.code.label(format!("fatal_{label}"))?;
        } else {
            emitter.code.label(label)?;
        }
        emitter.diagnostic(message);
        emitter.code.emit_exit(1);
    }
    emitter.code.label("error_bool")?;
    emitter.code.emit(&[0xbf, 4, 0, 0, 0]);
    emitter.code.emit_jmp_label("recovery_raise");
    emitter.code.label("fatal_error_bool")?;
    emitter.diagnostic("AIL-RUNTIME-BOOL: missing or malformed Boolean value\n");
    emitter.code.emit_exit(1);
    emitter.code.label("error_recovery_cleanup")?;
    emitter.diagnostic("AIL-RUNTIME-LIMIT: unable to release native temporary storage\n");
    emitter.code.emit_exit(1);
    emit_helpers(&mut emitter.code)?;
    if has_bool {
        native_bool::emit_helpers(&mut emitter.code)?;
    }
    native_text::emit_helpers(&mut emitter.code)?;
    if std::iter::once(action)
        .chain(functions.values())
        .any(|procedure| {
            procedure.instructions.iter().any(|i| {
                matches!(
                    i.opcode.as_str(),
                    "TEXT_SPLIT" | "TEXT_JOIN_LIST" | "TEXT_REPLACE" | "TEXT_TRIM"
                )
            })
        })
    {
        native_text_transform::emit_helpers(&mut emitter.code)?;
    }
    native_list::emit_helpers(&mut emitter.code, has_float)?;
    if std::iter::once(action)
        .chain(functions.values())
        .any(|procedure| {
            procedure.instructions.iter().any(|i| {
                matches!(
                    i.opcode.as_str(),
                    "LIST_CONCAT" | "LIST_SLICE" | "LIST_REVERSE" | "LIST_SORT"
                )
            })
        })
    {
        native_list_transform::emit_helpers(&mut emitter.code)?;
    }
    native_file::emit_helpers(&mut emitter.code, has_bytes)?;
    if has_bytes {
        native_bytes::emit_helpers(&mut emitter.code)?;
    }
    native_stdio::emit_helpers(&mut emitter.code)?;
    if has_process {
        native_process::emit_helpers(&mut emitter.code)?;
    }
    if has_value
        || has_float
        || !value_targets.is_empty()
        || action.instructions.iter().any(|instruction| {
            matches!(
                instruction.opcode.as_str(),
                "JSON_PARSE"
                    | "JSON_FORMAT"
                    | "JSON_GET"
                    | "JSON_COUNT"
                    | "JSON_KIND"
                    | "JSON_KEYS"
                    | "JSON_SET"
                    | "JSON_REMOVE"
            )
        })
    {
        native_json::emit_helpers(&mut emitter.code, has_float)?;
        native_structured_json::emit_helpers(&mut emitter.code)?;
        native_json_mutation::emit_helpers(&mut emitter.code)?;
    }
    if !value_targets.is_empty() {
        native_value_function::emit_helpers(&mut emitter.code)?;
    }
    if has_value {
        native_value::emit_helpers(&mut emitter.code)?;
    }
    if has_float {
        native_float::emit_helpers(&mut emitter.code)?;
    }
    for (label, bytes) in emitter.data {
        emitter.code.label(label)?;
        emitter.code.emit(&bytes);
        // Empty values need a valid unique address even without data bytes.
        emitter.code.emit(&[0]);
    }
    let code = emitter.code.finish()?;
    if has_float || has_process || has_directory || has_bytes {
        native_float_runtime::wrap(&code)
    } else {
        Ok(wrap_linux_x86_64_elf(&code))
    }
}

fn required<'a>(instruction: &'a AilBytecodeInstruction, key: &str) -> Result<&'a str, String> {
    instruction
        .operands
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("native {} is missing {key}", instruction.opcode))
}

fn fuel_limit(instruction: &AilBytecodeInstruction) -> Result<usize, String> {
    required(instruction, "limit")?
        .parse::<usize>()
        .ok()
        .filter(|limit| *limit > 0)
        .ok_or_else(|| {
            format!(
                "native {} limit must be a positive integer",
                instruction.opcode
            )
        })
}

struct FunctionTarget {
    label: String,
    parameters: Vec<String>,
    slots: BTreeMap<String, usize>,
    frame: usize,
}

fn local_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn prepare_functions(
    functions: &BTreeMap<String, AilBytecodeAction>,
) -> Result<BTreeMap<String, FunctionTarget>, String> {
    let mut targets = BTreeMap::new();
    for (index, (name, function)) in functions.iter().enumerate() {
        if value_protocol::is_function(function) {
            continue;
        }
        if name != &function.name {
            return Err("native integer function key differs from its name".into());
        }
        let marker = function
            .instructions
            .first()
            .filter(|i| i.opcode == "INTEGER_FUNCTION")
            .ok_or("native integer call target is not an INTEGER_FUNCTION")?;
        let signature = required(marker, "parameters")?;
        let mut parameters = BTreeSet::new();
        if !signature.is_empty() {
            for parameter in signature.split(';') {
                if !local_name(parameter)
                    || !parameters.insert(parameter.to_string())
                    || parameters.len() > 256
                {
                    return Err(
                        "native integer function has invalid or duplicate parameters".into(),
                    );
                }
            }
        }
        let mut fields = parameters
            .iter()
            .map(|name| format!("local.{name}"))
            .collect::<BTreeSet<_>>();
        for (position, instruction) in function.instructions.iter().enumerate() {
            match instruction.opcode.as_str() {
                "INTEGER_FUNCTION" if position == 0 => {}
                "EVAL_INT" | "RETURN_INT" => {
                    fields.extend(
                        IntExpr::from_postfix(required(instruction, "expression")?)?.fields(),
                    );
                    if instruction.opcode == "EVAL_INT" {
                        fields.insert(required(instruction, "key")?.to_string());
                    }
                }
                "CALL_INT" => {
                    fields.insert(required(instruction, "key")?.to_string());
                    for argument in
                        integer_function::decode_arguments(required(instruction, "arguments")?)?
                            .values()
                    {
                        fields.extend(argument.fields());
                    }
                }
                "BRANCH_INT" => {
                    fields.extend(
                        crate::condition::IntCondition::from_postfix(
                            required(instruction, "left")?,
                            required(instruction, "comparison")?,
                            required(instruction, "right")?,
                        )?
                        .fields(),
                    );
                }
                "LABEL" | "JUMP" | "EMIT_TRACE" | "RAISE" => {}
                opcode => {
                    return Err(format!(
                        "unsupported integer function opcode '{opcode}' in {name}"
                    ));
                }
            }
        }
        if fields.len() > 256
            || fields
                .iter()
                .any(|key| !key.strip_prefix("local.").is_some_and(local_name))
        {
            return Err("native integer functions require at most 256 local variables".into());
        }
        // Locals use the same offsets as root scalar slots, but hold raw i64
        // values and an initialized flag, never pointers into a caller frame.
        let frame = fields.len() * SLOT_SIZE + native_recovery::FUNCTION_CONTEXT;
        let slots = fields
            .into_iter()
            .enumerate()
            .map(|(index, key)| (key, index * SLOT_SIZE))
            .collect();
        targets.insert(
            name.clone(),
            FunctionTarget {
                label: format!("integer_function_{index}"),
                parameters: parameters.into_iter().collect(),
                slots,
                frame,
            },
        );
    }
    Ok(targets)
}

struct Emitter {
    code: X64Code,
    data: Vec<(String, Vec<u8>)>,
    next: usize,
    slots: BTreeMap<String, usize>,
    selection_flag: usize,
    recovery_jumps: Vec<(String, String, usize)>,
    integer_frame: bool,
    function_operations: bool,
    value_schemas: AilValueSchemas,
    value_schema: String,
    value_schema_len: usize,
}

impl Emitter {
    fn integer_call(
        &mut self,
        instruction: &AilBytecodeInstruction,
        targets: &BTreeMap<String, FunctionTarget>,
    ) -> Result<(), String> {
        let name = required(instruction, "target")?;
        let target = targets
            .get(name)
            .ok_or_else(|| format!("unknown native integer function '{name}'"))?;
        let arguments = integer_function::decode_arguments(required(instruction, "arguments")?)?;
        if arguments.keys().ne(target.parameters.iter()) {
            return Err(format!(
                "native arguments do not match integer function '{name}' parameters"
            ));
        }
        // All expressions run in the caller, in sorted name order. The result
        // destination is written only after the callee returns successfully.
        for expression in arguments.values() {
            self.expression(expression)?;
            self.code.emit(&[0x50]); // push rax
        }
        self.code.emit_call_label(&target.label);
        if !arguments.is_empty() {
            self.code.emit(&[0x48, 0x81, 0xc4]); // add rsp, argument bytes
            self.code
                .emit(&((arguments.len() * 8) as u32).to_le_bytes());
        }
        self.store_integer(required(instruction, "key")?);
        Ok(())
    }

    fn integer_function(
        &mut self,
        function: &AilBytecodeAction,
        target: &FunctionTarget,
        targets: &BTreeMap<String, FunctionTarget>,
    ) -> Result<(), String> {
        let (labels, diagnostics) = ail_bytecode_action_labels(function);
        if !diagnostics.is_empty() {
            return Err(diagnostics.join("\n"));
        }
        self.slots = target.slots.clone();
        self.integer_frame = true;
        self.code.label(&target.label)?;
        self.code.emit(&[0x49, 0x83, 0xff, 64]); // cmp r15, 64
        self.code
            .emit_jcc_label(&[0x0f, 0x83], "error_function_depth");
        self.code.emit(&[
            0x49, 0xff, 0xc7, // inc r15
            0x53, 0x41, 0x56, // push rbx; push r14
            0x48, 0x81, 0xec, // sub rsp, frame
        ]);
        self.code.emit(&(target.frame as u32).to_le_bytes());
        self.code.emit(&[
            0x48, 0x89, 0xe3, // mov rbx, rsp
            0x48, 0x89, 0xdf, // mov rdi, rbx
            0x31, 0xc0, 0xb9, // xor eax, eax; mov ecx, frame/8
        ]);
        self.code.emit(&((target.frame / 8) as u32).to_le_bytes());
        self.code.emit(&[0xf3, 0x48, 0xab, 0x49, 0xbe]); // rep stosq; mov r14, fuel
        self.code.emit(
            &(function.instructions.len().saturating_mul(1024).max(1024) as u64).to_le_bytes(),
        );
        self.begin_function_context(target.frame);
        for (index, parameter) in target.parameters.iter().enumerate() {
            // Frame, saved r14/rbx, return address, then reverse-pushed args.
            self.load_rax(target.frame + 24 + (target.parameters.len() - 1 - index) * 8);
            self.store_integer(&format!("local.{parameter}"));
        }
        let instruction_label = |index| format!("{}_instruction_{index}", target.label);
        let return_label = format!("{}_return", target.label);
        for (index, instruction) in function.instructions.iter().enumerate() {
            self.code.label(instruction_label(index))?;
            self.consume_fuel();
            match instruction.opcode.as_str() {
                "INTEGER_FUNCTION" | "LABEL" => {}
                "RAISE" => native_recovery::emit_raise(&mut self.code, instruction)?,
                "EVAL_INT" => {
                    self.expression(&IntExpr::from_postfix(required(
                        instruction,
                        "expression",
                    )?)?)?;
                    self.store_integer(required(instruction, "key")?);
                }
                "CALL_INT" => self.integer_call(instruction, targets)?,
                "RETURN_INT" => {
                    self.expression(&IntExpr::from_postfix(required(
                        instruction,
                        "expression",
                    )?)?)?;
                    self.code.emit_jmp_label(&return_label);
                }
                "BRANCH_INT" | "JUMP" => {
                    let name = required(instruction, "label")?;
                    let destination = labels
                        .get(name)
                        .ok_or_else(|| format!("unknown native integer function label '{name}'"))?;
                    if instruction.opcode == "JUMP" {
                        self.code.emit_jmp_label(&instruction_label(*destination));
                    } else {
                        let condition = crate::condition::IntCondition::from_postfix(
                            required(instruction, "left")?,
                            required(instruction, "comparison")?,
                            required(instruction, "right")?,
                        )?;
                        self.expression(&condition.left)?;
                        self.code.emit(&[0x50]);
                        self.expression(&condition.right)?;
                        self.code.emit(&[0x48, 0x89, 0xc1, 0x58, 0x48, 0x39, 0xc8]);
                        let opcode = match condition.comparison {
                            crate::condition::IntComparison::Equal => 0x84,
                            crate::condition::IntComparison::NotEqual => 0x85,
                            crate::condition::IntComparison::Less => 0x8c,
                            crate::condition::IntComparison::LessEqual => 0x8e,
                            crate::condition::IntComparison::Greater => 0x8f,
                            crate::condition::IntComparison::GreaterEqual => 0x8d,
                        };
                        self.code
                            .emit_jcc_label(&[0x0f, opcode], &instruction_label(*destination));
                    }
                }
                "EMIT_TRACE" => self.trace(&format!("trace {}\n", required(instruction, "event")?)),
                _ => unreachable!("function instruction subset checked before emission"),
            }
        }
        self.code.emit_jmp_label("error_function_return");
        self.code.label(return_label)?;
        self.end_function_context(target.frame);
        self.code.emit(&[0x48, 0x89, 0xdc, 0x48, 0x81, 0xc4]); // rsp=rbx; add rsp, frame
        self.code.emit(&(target.frame as u32).to_le_bytes());
        self.code.emit(&[0x41, 0x5e, 0x5b, 0x49, 0xff, 0xcf, 0xc3]); // pop r14; pop rbx; dec r15; ret
        Ok(())
    }

    fn consume_fuel(&mut self) {
        self.code.emit(&[0x49, 0x83, 0xee, 0x01]); // sub r14, 1
        self.code.emit_jcc_label(&[0x0f, 0x82], "error_steps");
        self.code.emit(&[0x4c, 0x8b, 0x5d, 0x30, 0x4d, 0x89, 0x33]); // r11=live fuel cell; [r11]=r14
    }
    fn slot(&self, key: &str) -> usize {
        self.slots[key]
    }
    fn data(&mut self, value: &[u8]) -> String {
        push_native_data_label(&mut self.data, &mut self.next, "scalar_data", value)
    }
    fn write(&mut self, fd: u32, value: &[u8]) {
        let label = self.data(value);
        self.code.emit_write_label(fd, &label, value.len() as u32);
    }
    fn trace(&mut self, value: &str) {
        let skip = format!("stdio_trace_skip_{}", self.next);
        self.next += 1;
        native_stdio::skip(&mut self.code, &skip);
        self.write(2, value.as_bytes());
        self.code.label(skip).expect("unique trace label");
    }
    fn diagnostic(&mut self, value: &str) {
        let label = self.data(value.as_bytes());
        self.code.emit(&[0xbf, 2, 0, 0, 0]);
        self.code.emit_lea_rsi_label(&label);
        self.code.emit_mov_edx_imm32(value.len() as u32);
        self.code.emit_call_label("stdio_diagnostic_write");
    }
    fn operation_trace(&mut self, value: &str) {
        if !self.function_operations {
            self.trace(value);
        }
    }
    fn displacement(&mut self, offset: usize) {
        self.code.emit(&(offset as u32).to_le_bytes());
    }
    fn load_rax(&mut self, offset: usize) {
        self.code.emit(&[0x48, 0x8b, 0x83]);
        self.displacement(offset);
    }
    fn load_rdi(&mut self, offset: usize) {
        self.code.emit(&[0x48, 0x8b, 0xbb]);
        self.displacement(offset);
    }
    fn load_rsi(&mut self, offset: usize) {
        self.code.emit(&[0x48, 0x8b, 0xb3]);
        self.displacement(offset);
    }
    fn load_rcx(&mut self, offset: usize) {
        self.code.emit(&[0x48, 0x8b, 0x8b]);
        self.displacement(offset);
    }
    fn load_rdx(&mut self, offset: usize) {
        self.code.emit(&[0x48, 0x8b, 0x93]);
        self.displacement(offset);
    }
    fn store_rax(&mut self, offset: usize) {
        self.code.emit(&[0x48, 0x89, 0x83]);
        self.displacement(offset);
    }
    fn store_rsi(&mut self, offset: usize) {
        self.code.emit(&[0x48, 0x89, 0xb3]);
        self.displacement(offset);
    }
    fn store_rdx(&mut self, offset: usize) {
        self.code.emit(&[0x48, 0x89, 0x93]);
        self.displacement(offset);
    }
    fn lea_rsi(&mut self, offset: usize) {
        self.code.emit(&[0x48, 0x8d, 0xb3]);
        self.displacement(offset);
    }
    fn set(&mut self, offset: usize, value: u64) {
        self.code.emit(&[0x48, 0xb8]);
        self.code.emit(&value.to_le_bytes());
        self.store_rax(offset);
    }
    fn initialize(&mut self, frame: usize, fuel: usize) -> Result<(), String> {
        self.code.emit(&[0x45, 0x31, 0xff]); // xor r15d, r15d (active integer function frames)
        self.code.emit(&[
            0x4c, 0x8b, 0x2c, 0x24, // mov r13, [rsp] (argc)
            0x49, 0xff, 0xcd, // dec r13
            0x4c, 0x8d, 0x64, 0x24, 0x10, // lea r12, [rsp+16] (argv[1])
            0x48, 0x81, 0xec, // sub rsp, frame
        ]);
        self.code.emit(&(frame as u32).to_le_bytes());
        self.code
            .emit(&[0x48, 0x89, 0xe3, 0x48, 0x89, 0xdf, 0x31, 0xc0, 0xb9]); // rbx=frame; rdi=rbx; eax=0; ecx=frame/8
        self.code.emit(&((frame / 8) as u32).to_le_bytes());
        self.code.emit(&[0xf3, 0x48, 0xab, 0x49, 0xbe]); // rep stosq; mov r14, fuel
        self.code.emit(&(fuel as u64).to_le_bytes());
        // RBP points to immutable text/list arena metadata in the root frame.
        // It is preserved by integer functions and every native value helper.
        self.code.emit(&[0x48, 0x8d, 0xab]); // lea rbp, [rbx + arena offset]
        self.code
            .emit(&((self.selection_flag + 8) as u32).to_le_bytes());
        self.code.emit(&[
            0x4c, 0x89, 0x75, 0x40, 0x48, 0x8d, 0x45, 0x40, 0x48, 0x89, 0x45, 0x30,
        ]); // root live fuel cell
        for (key, offset) in self.slots.clone() {
            let prefix = format!("{key}=");
            let label = self.data(prefix.as_bytes());
            let absent = format!("initial_absent_{offset}");
            self.code.emit_lea_rsi_label(&label);
            self.code.emit_mov_edx_imm32(prefix.len() as u32);
            self.code.emit_call_label("scalar_find_last");
            self.store_rax(offset + POINTER);
            self.code.emit_test_rax_rax();
            self.code.emit_jcc_label(&[0x0f, 0x84], &absent);
            self.code.emit_mov_rdi_rax();
            self.code.emit_call_label("scalar_strlen");
            self.store_rax(offset + LENGTH);
            self.code.label(absent)?;
        }
        Ok(())
    }
    fn integer(&mut self, key: &str) {
        if self.integer_frame {
            self.load_rax(self.slot(key) + POINTER);
            self.code.emit_test_rax_rax();
            self.code.emit_jcc_label(&[0x0f, 0x84], "error_int");
            self.load_rax(self.slot(key) + NUMERIC);
            return;
        }
        self.load_rdi(self.slot(key) + POINTER);
        self.load_rsi(self.slot(key) + LENGTH);
        self.code.emit_call_label("scalar_parse");
    }
    fn store_integer(&mut self, key: &str) {
        let slot = self.slot(key);
        if self.integer_frame {
            self.store_rax(slot + NUMERIC);
            self.set(slot + POINTER, 1);
            return;
        }
        self.code.emit_mov_rdi_rax();
        self.lea_rsi(slot + SLOT_SIZE);
        self.code.emit_call_label("scalar_render");
        self.store_rax(slot + POINTER);
        self.store_rdx(slot + LENGTH);
        self.set(slot + NUMERIC, 1);
    }
    fn expression(&mut self, expr: &IntExpr) -> Result<(), String> {
        match expr {
            IntExpr::Literal(value) => {
                self.code.emit(&[0x48, 0xb8]);
                self.code.emit(&value.to_le_bytes());
            }
            IntExpr::Field(key) => self.integer(key),
            IntExpr::Binary { op, left, right } => {
                self.expression(left)?;
                self.code.emit(&[0x50]); // push rax
                self.expression(right)?;
                self.code.emit(&[0x48, 0x89, 0xc1, 0x58]); // mov rcx, rax; pop rax
                match op {
                    IntOperator::Add => self.code.emit(&[0x48, 0x01, 0xc8]),
                    IntOperator::Subtract => self.code.emit(&[0x48, 0x29, 0xc8]),
                    IntOperator::Multiply => self.code.emit(&[0x48, 0x0f, 0xaf, 0xc1]),
                    IntOperator::Divide | IntOperator::Remainder => {
                        self.code.emit(&[0x48, 0x85, 0xc9]); // test rcx,rcx
                        self.code.emit_jcc_label(&[0x0f, 0x84], "error_divzero");
                        // x86 IDIV traps for MIN/-1; detect before issuing it.
                        let safe = format!("division_safe_{}", self.next);
                        self.next += 1;
                        self.code.emit(&[0x48, 0x83, 0xf9, 0xff]);
                        self.code.emit_jcc_label(&[0x0f, 0x85], &safe);
                        self.code.emit(&[0x48, 0xba]);
                        self.code.emit(&i64::MIN.to_le_bytes());
                        self.code.emit(&[0x48, 0x39, 0xd0]);
                        self.code.emit_jcc_label(&[0x0f, 0x84], "error_overflow");
                        self.code.label(safe)?;
                        self.code.emit(&[0x48, 0x99, 0x48, 0xf7, 0xf9]); // cqo; idiv rcx
                        if matches!(op, IntOperator::Remainder) {
                            self.code.emit(&[0x48, 0x89, 0xd0]);
                        }
                    }
                }
                if matches!(
                    op,
                    IntOperator::Add | IntOperator::Subtract | IntOperator::Multiply
                ) {
                    self.code.emit_jcc_label(&[0x0f, 0x80], "error_overflow");
                }
            }
        }
        Ok(())
    }
    fn copy(&mut self, key: &str, source: &str, index: usize) -> Result<(), String> {
        let src = self.slot(source);
        let dst = self.slot(key);
        let done = format!("copy_done_{index}");
        let direct = format!("copy_direct_{index}");
        self.load_rax(src + POINTER);
        self.code.emit_test_rax_rax();
        self.code.emit_jcc_label(&[0x0f, 0x84], &done);
        self.load_rdx(src + LENGTH);
        self.store_rdx(dst + LENGTH);
        self.load_rcx(src + NUMERIC);
        self.code.emit(&[0x48, 0x85, 0xc9]);
        self.code.emit_jcc_label(&[0x0f, 0x84], &direct);
        self.code.emit(&[0x48, 0x89, 0xc6]); // mov rsi, rax
        self.code.emit(&[0x48, 0x8d, 0xbb]);
        self.displacement(dst + BUFFER); // lea rdi, buffer
        self.code.emit(&[0x48, 0x89, 0xf8]);
        self.store_rax(dst + POINTER);
        self.code.emit(&[0x48, 0x89, 0xd1, 0xf3, 0xa4]); // rcx=len; rep movsb
        self.set(dst + NUMERIC, 1);
        self.code.emit_jmp_label(&done);
        self.code.label(direct)?;
        self.store_rax(dst + POINTER);
        self.set(dst + NUMERIC, 0);
        self.code.label(done)?;
        self.trace(&format!("write {key}\n"));
        Ok(())
    }
    fn compare_value(&mut self, key: &str, value: &str) {
        let label = self.data(value.as_bytes());
        self.load_rdi(self.slot(key) + POINTER);
        self.load_rcx(self.slot(key) + LENGTH);
        self.code.emit_lea_rsi_label(&label);
        self.code.emit_mov_edx_imm32(value.len() as u32);
        self.code.emit_call_label("scalar_compare");
    }
    fn compare_fields(&mut self, left: &str, right: &str) {
        self.load_rdi(self.slot(left) + POINTER);
        self.load_rcx(self.slot(left) + LENGTH);
        self.load_rsi(self.slot(right) + POINTER);
        self.load_rdx(self.slot(right) + LENGTH);
        self.code.emit_call_label("scalar_compare");
    }
    fn write_value(&mut self, fd: u32, key: &str) {
        self.load_rsi(self.slot(key) + POINTER);
        self.load_rdx(self.slot(key) + LENGTH);
        self.code.emit(&[0xb8, 1, 0, 0, 0, 0xbf]);
        self.code.emit(&fd.to_le_bytes());
        self.code.emit(&[0x0f, 0x05]);
    }
    fn trace_value(&mut self, prefix: &str, key: &str) {
        let skip = format!("stdio_trace_value_skip_{}", self.next);
        self.next += 1;
        native_stdio::skip(&mut self.code, &skip);
        self.write(2, prefix.as_bytes());
        self.write_value(2, key);
        self.write(2, b"\n");
        self.code.label(skip).expect("unique trace value label");
    }
    fn validate_selected(&mut self) -> Result<(), String> {
        let serial = self.next;
        self.next += 1;
        for (key, slot) in self.slots.clone() {
            let skip = format!("validate_skip_{serial}_{slot}");
            self.load_rax(slot + SELECTED);
            self.code.emit_test_rax_rax();
            self.code.emit_jcc_label(&[0x0f, 0x84], &skip);
            self.load_rax(self.slot(&key) + POINTER);
            self.code.emit_test_rax_rax();
            self.code.emit_jcc_label(&[0x0f, 0x84], "error_output");
            self.code.label(skip)?;
        }
        Ok(())
    }
    fn outputs(&mut self) -> Result<(), String> {
        // Validate the whole return selection before writing any stdout bytes.
        self.validate_selected()?;
        native_stdio::skip(&mut self.code, "outputs_done");
        for (key, slot) in self.slots.clone() {
            let present = format!("output_present_{slot}");
            let skip = format!("output_skip_{slot}");
            self.load_rax(self.selection_flag);
            self.code.emit_test_rax_rax();
            self.code.emit_jcc_label(&[0x0f, 0x84], &present);
            self.load_rax(slot + SELECTED);
            self.code.emit_test_rax_rax();
            self.code.emit_jcc_label(&[0x0f, 0x84], &skip);
            self.code.label(present)?;
            self.load_rax(slot + POINTER);
            self.code.emit_test_rax_rax();
            self.code.emit_jcc_label(&[0x0f, 0x84], &skip);
            self.write(1, format!("{key}=").as_bytes());
            self.write_value(1, &key);
            self.write(1, b"\n");
            self.code.label(skip)?;
        }
        self.load_rax(self.selection_flag);
        self.code.emit_test_rax_rax();
        self.code.emit_jcc_label(&[0x0f, 0x85], "outputs_done");
        self.unknown_outputs()?;
        self.code.label("outputs_done")?;
        Ok(())
    }
    fn unknown_outputs(&mut self) -> Result<(), String> {
        // Emit untouched argv keys that were not referenced by the program.
        // A reverse lookup suppresses earlier duplicates just like BTreeMap::insert.
        self.code.emit(&[0x45, 0x31, 0xff]); // xor r15d,r15d
        self.code.label("unknown_loop")?;
        self.code.emit(&[0x4d, 0x39, 0xef]); // cmp r15,r13
        self.code.emit_jcc_label(&[0x0f, 0x83], "outputs_done");
        self.code.emit(&[0x4b, 0x8b, 0x34, 0xfc, 0x31, 0xd2]); // rsi=[r12+r15*8]; edx=0
        self.code.emit(&[0x48, 0x89, 0xf7]); // rdi=argv item
        self.code.emit_call_label("file_grant_kind");
        self.code.emit_test_rax_rax();
        self.code.emit_jcc_label(&[0x0f, 0x85], "unknown_next");
        // Process bindings are immutable authority, never untouched state keys.
        // Startup has validated all reserved forms, including unused bindings.
        for (offset, byte) in b"--run-process=".iter().enumerate() {
            self.code.emit(&[0x80, 0x7f, offset as u8, *byte]);
            self.code
                .emit_jcc_label(&[0x0f, 0x85], "unknown_process_checked");
        }
        self.code.emit_jmp_label("unknown_next");
        self.code.label("unknown_process_checked")?;
        for (prefix, next) in [
            (b"--read-directory=".as_slice(), "unknown_directory_write"),
            (
                b"--write-directory=".as_slice(),
                "unknown_directory_checked",
            ),
        ] {
            for (offset, byte) in prefix.iter().enumerate() {
                self.code.emit(&[0x80, 0x7f, offset as u8, *byte]);
                self.code.emit_jcc_label(&[0x0f, 0x85], next);
            }
            self.code.emit_jmp_label("unknown_next");
            self.code.label(next)?;
        }
        self.code.label("unknown_key_scan")?;
        self.code.emit(&[0x8a, 0x04, 0x16, 0x84, 0xc0]); // al=[rsi+rdx]; test al,al
        self.code.emit_jcc_label(&[0x0f, 0x84], "unknown_next");
        self.code.emit(&[0x48, 0xff, 0xc2, 0x3c, 0x3d]); // ++rdx; cmp al,'='
        self.code.emit_jcc_label(&[0x0f, 0x85], "unknown_key_scan");
        self.code.emit(&[0x56, 0x52]); // preserve prefix pointer and length
        self.code.emit_call_label("scalar_find_last");
        self.code
            .emit(&[0x5a, 0x5e, 0x48, 0x8d, 0x0c, 0x16, 0x48, 0x39, 0xc8]); // pop rdx,rsi; rcx=rsi+rdx; cmp rax,rcx
        self.code.emit_jcc_label(&[0x0f, 0x85], "unknown_next");
        for (key, slot) in self.slots.clone() {
            let prefix = format!("{key}=");
            let label = self.data(prefix.as_bytes());
            let next = format!("unknown_not_{slot}");
            self.code.emit(&[0x48, 0x81, 0xfa]);
            self.code.emit(&(prefix.len() as u32).to_le_bytes());
            self.code.emit_jcc_label(&[0x0f, 0x85], &next);
            self.code
                .emit(&[0x56, 0x52, 0x48, 0x89, 0xf7, 0x48, 0x89, 0xd1]); // push rsi,rdx; rdi=rsi; rcx=rdx
            self.code.emit_lea_rsi_label(&label);
            self.code.emit_call_label("scalar_compare");
            self.code.emit(&[0x5a, 0x5e, 0x85, 0xc0]);
            self.code.emit_jcc_label(&[0x0f, 0x84], "unknown_next");
            self.code.label(next)?;
        }
        self.code.emit(&[0x48, 0x89, 0xf7, 0x56]); // mov rdi,rsi; push rsi
        self.code.emit_call_label("scalar_strlen");
        self.code.emit(&[
            0x5e, 0x48, 0x89, 0xc2, 0xb8, 1, 0, 0, 0, 0xbf, 1, 0, 0, 0, 0x0f, 0x05,
        ]);
        self.write(1, b"\n");
        self.code.label("unknown_next")?;
        self.code.emit(&[0x49, 0xff, 0xc7]);
        self.code.emit_jmp_label("unknown_loop");
        Ok(())
    }
}

fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    // The helper ABI preserves rbx (state), r12/r13 (argv), r14 (fuel), r15.
    // All strings are spans except the initial OS argv scan.
    code.label("scalar_find_last")?; // rsi=key prefix, edx=length -> rax=value pointer or 0
    code.emit(&[0x4d, 0x89, 0xe8]); // mov r8,r13
    code.label("scalar_find_loop")?;
    code.emit(&[0x4d, 0x85, 0xc0]);
    code.emit_jcc_label(&[0x0f, 0x84], "scalar_find_missing");
    code.emit(&[0x49, 0xff, 0xc8, 0x4b, 0x8b, 0x3c, 0xc4, 0x31, 0xc9]); // --r8; rdi=argv[r8]; ecx=0
    code.label("scalar_find_compare")?;
    code.emit(&[0x39, 0xd1]);
    code.emit_jcc_label(&[0x0f, 0x83], "scalar_find_found");
    code.emit(&[0x8a, 0x04, 0x0f, 0x3a, 0x04, 0x0e]);
    code.emit_jcc_label(&[0x0f, 0x85], "scalar_find_loop");
    code.emit(&[0x48, 0xff, 0xc1]);
    code.emit_jmp_label("scalar_find_compare");
    code.label("scalar_find_found")?;
    code.emit(&[0x48, 0x8d, 0x04, 0x17, 0xc3]); // lea rax,[rdi+rdx]; ret
    code.label("scalar_find_missing")?;
    code.emit(&[0x31, 0xc0, 0xc3]);

    code.label("scalar_strlen")?;
    code.emit(&[0x31, 0xc0]);
    code.label("scalar_strlen_loop")?;
    code.emit(&[0x80, 0x3c, 0x07, 0]);
    code.emit_jcc_label(&[0x0f, 0x84], "scalar_strlen_done");
    code.emit(&[0x48, 0xff, 0xc0]);
    code.emit_jmp_label("scalar_strlen_loop");
    code.label("scalar_strlen_done")?;
    code.emit(&[0xc3]);

    // Compare rdi/rcx and rsi/rdx spans lexically. A missing operand yields -1
    // so it neither equals a string nor satisfies REQUIRE_FIELD_AFTER.
    code.label("scalar_compare")?;
    code.emit(&[0x48, 0x85, 0xff]);
    code.emit_jcc_label(&[0x0f, 0x84], "scalar_compare_less");
    code.emit(&[0x48, 0x85, 0xf6]);
    code.emit_jcc_label(&[0x0f, 0x84], "scalar_compare_less");
    code.emit(&[0x45, 0x31, 0xc0]); // r8=0
    code.label("scalar_compare_loop")?;
    code.emit(&[0x49, 0x39, 0xc8]); // cmp r8,rcx
    code.emit_jcc_label(&[0x0f, 0x83], "scalar_compare_lengths");
    code.emit(&[0x49, 0x39, 0xd0]); // cmp r8,rdx
    code.emit_jcc_label(&[0x0f, 0x83], "scalar_compare_greater");
    code.emit(&[0x42, 0x8a, 0x04, 0x07, 0x42, 0x3a, 0x04, 0x06]); // al=left[r8]; cmp al,right[r8]
    code.emit_jcc_label(&[0x0f, 0x82], "scalar_compare_less");
    code.emit_jcc_label(&[0x0f, 0x87], "scalar_compare_greater");
    code.emit(&[0x49, 0xff, 0xc0]);
    code.emit_jmp_label("scalar_compare_loop");
    code.label("scalar_compare_lengths")?;
    code.emit(&[0x48, 0x39, 0xd1]);
    code.emit_jcc_label(&[0x0f, 0x82], "scalar_compare_less");
    code.emit(&[0x31, 0xc0, 0xc3]);
    code.label("scalar_compare_less")?;
    code.emit(&[0xb8, 0xff, 0xff, 0xff, 0xff, 0xc3]);
    code.label("scalar_compare_greater")?;
    code.emit(&[0xb8, 1, 0, 0, 0, 0xc3]);

    // Parse a complete signed decimal span into rax, accumulating negatively
    // so INT64_MIN requires no out-of-range positive intermediate.
    code.label("scalar_parse")?; // rdi=pointer, rsi=length
    code.emit(&[0x48, 0x85, 0xff]);
    code.emit_jcc_label(&[0x0f, 0x84], "error_int");
    code.emit(&[0x48, 0x85, 0xf6]);
    code.emit_jcc_label(&[0x0f, 0x84], "error_int");
    code.emit(&[0x31, 0xc0, 0x45, 0x31, 0xc0, 0x80, 0x3f, 0x2d]); // acc=0; sign=0; cmp [rdi],'-'
    code.emit_jcc_label(&[0x0f, 0x85], "scalar_parse_plus");
    code.emit(&[0x41, 0xb8, 1, 0, 0, 0]);
    code.emit_jmp_label("scalar_parse_sign");
    code.label("scalar_parse_plus")?;
    code.emit(&[0x80, 0x3f, 0x2b]);
    code.emit_jcc_label(&[0x0f, 0x85], "scalar_parse_digit");
    code.label("scalar_parse_sign")?;
    code.emit(&[0x48, 0xff, 0xc7, 0x48, 0xff, 0xce]);
    code.emit_jcc_label(&[0x0f, 0x84], "error_int");
    code.label("scalar_parse_digit")?;
    code.emit(&[0x0f, 0xb6, 0x0f, 0x83, 0xe9, 0x30, 0x83, 0xf9, 9]); // ecx=digit-'0'
    code.emit_jcc_label(&[0x0f, 0x87], "error_int");
    code.emit(&[0x48, 0x6b, 0xc0, 0x0a]);
    code.emit_jcc_label(&[0x0f, 0x80], "error_int");
    code.emit(&[0x48, 0x29, 0xc8]);
    code.emit_jcc_label(&[0x0f, 0x80], "error_int");
    code.emit(&[0x48, 0xff, 0xc7, 0x48, 0xff, 0xce]);
    code.emit_jcc_label(&[0x0f, 0x85], "scalar_parse_digit");
    code.emit(&[0x45, 0x85, 0xc0]);
    code.emit_jcc_label(&[0x0f, 0x85], "scalar_parse_return");
    code.emit(&[0x48, 0xf7, 0xd8]);
    code.emit_jcc_label(&[0x0f, 0x80], "error_int");
    code.label("scalar_parse_return")?;
    code.emit(&[0xc3]);

    // Format rdi backwards from rsi (end of a private 32-byte field buffer).
    // Returns rax=first byte, rdx=length; unsigned division handles MIN's magnitude.
    code.label("scalar_render")?;
    code.emit(&[
        0x48, 0x89, 0xf8, 0x49, 0x89, 0xf0, 0x45, 0x31, 0xc9, 0x48, 0x85, 0xc0,
    ]);
    code.emit_jcc_label(&[0x0f, 0x8d], "scalar_render_digits");
    code.emit(&[0x48, 0xf7, 0xd8, 0x41, 0xb9, 1, 0, 0, 0]);
    code.label("scalar_render_digits")?;
    code.emit(&[
        0x31, 0xd2, 0xb9, 10, 0, 0, 0, // edx=0; ecx=10
        0x48, 0xf7, 0xf1, // div rcx
        0x80, 0xc2, 0x30, 0x48, 0xff, 0xce, 0x88, 0x16, // dl+='0'; --rsi; [rsi]=dl
        0x48, 0x85, 0xc0,
    ]);
    code.emit_jcc_label(&[0x0f, 0x85], "scalar_render_digits");
    code.emit(&[0x45, 0x85, 0xc9]);
    code.emit_jcc_label(&[0x0f, 0x84], "scalar_render_done");
    code.emit(&[0x48, 0xff, 0xce, 0xc6, 0x06, 0x2d]);
    code.label("scalar_render_done")?;
    code.emit(&[0x48, 0x89, 0xf0, 0x4c, 0x89, 0xc2, 0x48, 0x29, 0xf2, 0xc3]);
    Ok(())
}
#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Output};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);
    static EXECUTION: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn native_explicit_raise_uses_each_existing_recovery_category() {
        for category in [
            "INT",
            "OVERFLOW",
            "DIVZERO",
            "FLOAT",
            "BOOL",
            "TEXT",
            "LIST",
            "INDEX",
            "JSON",
            "JSON-PATH",
            "PROCESS",
            "TIMEOUT",
            "PATH",
            "BYTES",
        ] {
            let output = run(
                vec![
                    instruction("TRY_BEGIN", &[("errors", category), ("key", "out.error")]),
                    instruction("RAISE", &[("category", category)]),
                    instruction("RECOVER", &[]),
                    instruction("TRY_END", &[]),
                    instruction("OUTPUT_FIELD", &[("key", "out.error")]),
                ],
                &[],
            );
            assert!(output.status.success(), "{category}: {output:?}");
            assert_eq!(
                output.stdout,
                format!("out.error=AIL-RUNTIME-{category}\n").as_bytes()
            );
        }
    }

    #[test]
    fn native_recovery_raw_opcodes_preserve_prior_writes_and_failed_destination() {
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "out.value"), ("value", "7")]),
                instruction("TRY_BEGIN", &[("errors", "DIVZERO"), ("key", "out.error")]),
                eval("out.prior", "11"),
                eval("out.value", "1 / 0"),
                eval("out.prior", "99"),
                instruction("RECOVER", &[]),
                eval("out.handled", "1"),
                instruction("TRY_END", &[]),
                instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                instruction("OUTPUT_FIELD", &[("key", "out.prior")]),
                instruction("OUTPUT_FIELD", &[("key", "out.error")]),
                instruction("OUTPUT_FIELD", &[("key", "out.handled")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"out.error=AIL-RUNTIME-DIVZERO\nout.handled=1\nout.prior=11\nout.value=7\n"
        );
        assert!(!String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-DIVZERO"));
    }

    fn recovery_begin(errors: &str, key: &str) -> AilBytecodeInstruction {
        instruction("TRY_BEGIN", &[("errors", errors), ("key", key)])
    }

    #[test]
    fn native_recovery_catches_each_data_family_without_diagnostics_or_destination_writes() {
        let cases = [
            ("INT", eval("out.value", "input.missing")),
            ("OVERFLOW", eval("out.value", "9223372036854775807 + 1")),
            ("DIVZERO", eval("out.value", "5 / 0")),
            (
                "FLOAT",
                instruction(
                    "FLOAT_TO_INT",
                    &[("key", "out.value"), ("source", "input.missing")],
                ),
            ),
            (
                "BOOL",
                instruction(
                    "EVAL_BOOL",
                    &[("key", "out.value"), ("expression", "bf:input.missing")],
                ),
            ),
            (
                "TEXT",
                instruction(
                    "TEXT_LENGTH",
                    &[("key", "out.value"), ("source", "input.missing")],
                ),
            ),
            (
                "LIST",
                instruction(
                    "LIST_LENGTH",
                    &[
                        ("key", "out.value"),
                        ("source", "input.badlist"),
                        ("type", "Int"),
                    ],
                ),
            ),
            (
                "INDEX",
                instruction(
                    "LIST_GET",
                    &[
                        ("key", "out.value"),
                        ("source", "input.list"),
                        ("type", "Int"),
                        ("index", "i:1"),
                    ],
                ),
            ),
            (
                "JSON",
                instruction(
                    "JSON_PARSE",
                    &[
                        ("key", "out.value"),
                        ("source", "input.badjson"),
                        ("type", "Json"),
                    ],
                ),
            ),
            (
                "JSON-PATH",
                json_get("out.value", "input.json", "input.path", "Int"),
            ),
        ];
        for (category, operation) in cases {
            for errors in [category, "any"] {
                let output = run(
                    vec![
                        eval("out.value", "37"),
                        recovery_begin(errors, "out.error"),
                        operation.clone(),
                        instruction("RECOVER", &[]),
                        instruction("TRY_END", &[]),
                        instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                        instruction("OUTPUT_FIELD", &[("key", "out.error")]),
                    ],
                    &[
                        "input.badlist=[1,]",
                        "input.list=[7]",
                        "input.badjson={",
                        "input.json={}",
                        "input.path=/missing",
                    ],
                );
                assert!(output.status.success(), "{category}: {output:?}");
                assert_eq!(
                    output.stdout,
                    format!("out.error=AIL-RUNTIME-{category}\nout.value=37\n").as_bytes()
                );
                assert!(
                    !String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-"),
                    "{category}: {output:?}"
                );
            }
        }
    }

    #[test]
    fn native_recovery_nested_nonmatching_and_handler_errors_reach_outer_scope() {
        for inner in ["DIVZERO", "INT"] {
            let output = run(
                vec![
                    recovery_begin("any", "out.outer"),
                    recovery_begin(inner, "out.inner"),
                    eval("out.value", "1 / 0"),
                    instruction("RECOVER", &[]),
                    eval("out.value", "input.missing"),
                    instruction("TRY_END", &[]),
                    instruction("RECOVER", &[]),
                    eval("out.handled", "7"),
                    instruction("TRY_END", &[]),
                    instruction("OUTPUT_FIELD", &[("key", "out.outer")]),
                    instruction("OUTPUT_FIELD", &[("key", "out.handled")]),
                ],
                &[],
            );
            assert!(output.status.success(), "{inner}: {output:?}");
            let expected = if inner == "DIVZERO" { "INT" } else { "DIVZERO" };
            assert_eq!(
                output.stdout,
                format!("out.handled=7\nout.outer=AIL-RUNTIME-{expected}\n").as_bytes()
            );
        }
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "out.error"), ("value", "unchanged")]),
                recovery_begin("any", "out.error"),
                eval("out.value", "9"),
                instruction("RECOVER", &[]),
                eval("out.value", "1 / 0"),
                instruction("TRY_END", &[]),
                instruction("OUTPUT_FIELD", &[("key", "out.error")]),
                instruction("OUTPUT_FIELD", &[("key", "out.value")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.error=unchanged\nout.value=9\n");
    }

    #[test]
    fn native_recovery_jump_exits_pop_handlers_only_when_taken_and_reentry_is_fresh() {
        for branch in [
            instruction("JUMP", &[("label", "outside")]),
            branch_int("1", "==", "1", "outside"),
            instruction(
                "BRANCH_BOOL",
                &[("expression", "b:true"), ("label", "outside")],
            ),
            instruction(
                "BRANCH_FLOAT",
                &[
                    ("left", "d:1"),
                    ("right", "d:1"),
                    ("comparison", "=="),
                    ("label", "outside"),
                ],
            ),
            instruction(
                "BRANCH_FIELD_EQUALS",
                &[
                    ("key", "data.value"),
                    ("value", "yes"),
                    ("label", "outside"),
                ],
            ),
        ] {
            let output = run(
                vec![
                    recovery_begin("any", "out.outer"),
                    recovery_begin("any", "out.inner"),
                    branch,
                    instruction("RECOVER", &[]),
                    eval("out.wrong", "999"),
                    instruction("TRY_END", &[]),
                    instruction("LABEL", &[("name", "outside")]),
                    eval("out.result", "1 / 0"),
                    instruction("RECOVER", &[]),
                    instruction("TRY_END", &[]),
                    instruction("OUTPUT_FIELD", &[("key", "out.outer")]),
                ],
                &["data.value=yes"],
            );
            assert!(output.status.success(), "{output:?}");
            assert_eq!(output.stdout, b"out.outer=AIL-RUNTIME-DIVZERO\n");
        }
        let output = run(
            vec![
                eval("out.count", "0"),
                instruction("LABEL", &[("name", "loop")]),
                recovery_begin("DIVZERO", "out.error"),
                branch_int("out.count", "<", "0", "outside"),
                eval("out.count", "out.count + 1"),
                eval("out.value", "1 / 0"),
                instruction("RECOVER", &[]),
                instruction("TRY_END", &[]),
                instruction("LABEL", &[("name", "outside")]),
                branch_int("out.count", "<", "100", "loop"),
                instruction("OUTPUT_FIELD", &[("key", "out.count")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.count=100\n");
    }

    #[test]
    fn native_recovery_callee_unwind_restores_depth_and_failed_call_destination() {
        let function = integer_function(
            "recursive",
            "n",
            vec![
                branch_int("local.n", "==", "0", "fail"),
                call_int("recursive", "n=f:local.n i:1 -", "local.result"),
                return_int("local.result"),
                instruction("LABEL", &[("name", "fail")]),
                return_int("1 / 0"),
            ],
        );
        let output = run_functions(
            vec![
                eval("out.value", "43"),
                eval("out.count", "0"),
                instruction("LABEL", &[("name", "again")]),
                recovery_begin("DIVZERO", "out.error"),
                call_int("recursive", "n=i:63", "out.value"),
                instruction("RECOVER", &[]),
                eval("out.count", "out.count + 1"),
                instruction("TRY_END", &[]),
                branch_int("out.count", "<", "100", "again"),
                instruction("OUTPUT_FIELD", &[("key", "out.count")]),
                instruction("OUTPUT_FIELD", &[("key", "out.value")]),
            ],
            BTreeMap::from([function]),
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.count=100\nout.value=43\n");
    }

    #[test]
    fn native_recovery_value_return_errors_are_caught_before_owned_handlers_leave() {
        let function = value_function(
            "recover_return",
            "",
            "error:Text;result:Int",
            "Int",
            vec![
                recovery_begin("DIVZERO", "local.error"),
                return_value("Int", "1 / 0"),
                instruction("RECOVER", &[]),
                return_value("Int", "7"),
                instruction("TRY_END", &[]),
            ],
        );
        let early = value_function(
            "early",
            "",
            "error:Text",
            "Int",
            vec![
                recovery_begin("any", "local.error"),
                return_value("Int", "11"),
                instruction("RECOVER", &[]),
                return_value("Int", "99"),
                instruction("TRY_END", &[]),
            ],
        );
        let output = run_functions(
            vec![
                recovery_begin("DIVZERO", "out.error"),
                call_value("recover_return", "", "out.a"),
                call_value("early", "", "out.b"),
                eval("out.c", "1 / 0"),
                instruction("RECOVER", &[]),
                instruction("TRY_END", &[]),
                instruction("OUTPUT_FIELD", &[("key", "out.a")]),
                instruction("OUTPUT_FIELD", &[("key", "out.b")]),
                instruction("OUTPUT_FIELD", &[("key", "out.error")]),
            ],
            BTreeMap::from([function, early]),
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"out.a=7\nout.b=11\nout.error=AIL-RUNTIME-DIVZERO\n"
        );
    }

    #[test]
    fn native_recovery_restores_nested_output_scopes_and_keeps_callee_writes() {
        let output = run(
            vec![
                eval("out.value", "1"),
                instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                recovery_begin("any", "data.error"),
                instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "8")]),
                instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                eval("out.value", "7"),
                instruction("OUTPUT_FIELD", &[("key", "missing.inner")]),
                instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "8")]),
                instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                eval("out.value", "11"),
                eval("out.value", "1 / 0"),
                instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
                instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "0")]),
                instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
                instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "0")]),
                instruction("RECOVER", &[]),
                eval("data.handled", "1"),
                instruction("TRY_END", &[]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.value=11\n");
    }

    #[test]
    fn native_recovery_keeps_live_fuel_instead_of_refunding_try_entry_budget() {
        let output = run(
            vec![
                instruction("NATIVE_FUEL_ROOT", &[("limit", "20")]),
                recovery_begin("DIVZERO", "data.error"),
                instruction("LABEL", &[("name", "fail_again")]),
                eval("out.value", "1 / 0"),
                instruction("RECOVER", &[]),
                instruction("TRY_END", &[]),
                instruction("JUMP", &[("label", "try_again")]),
                instruction("LABEL", &[("name", "try_again")]),
                recovery_begin("DIVZERO", "data.error"),
                eval("out.value", "1 / 0"),
                instruction("RECOVER", &[]),
                instruction("TRY_END", &[]),
                instruction("JUMP", &[("label", "try_again")]),
            ],
            &[],
        );
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("execution step limit exceeded"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-DIVZERO"));
    }

    #[test]
    fn native_recovery_float_failure_preserves_existing_numeric_buffer_and_alias() {
        let output = run(
            vec![
                float_eval("out.value", "d:12.25"),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.alias"), ("source", "out.value")],
                ),
                recovery_begin("DIVZERO;FLOAT", "out.error"),
                float_eval("out.value", "d:1 d:0 /"),
                instruction("RECOVER", &[]),
                instruction("TRY_END", &[]),
                instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                instruction("OUTPUT_FIELD", &[("key", "out.alias")]),
                instruction("OUTPUT_FIELD", &[("key", "out.error")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"out.alias=1.225e1\nout.error=AIL-RUNTIME-DIVZERO\nout.value=1.225e1\n"
        );
    }

    fn run_recovery_limited(instructions: Vec<AilBytecodeInstruction>, args: &[String]) -> Output {
        let binary = emit(
            &AilBytecodeAction {
                name: "RecoveryResources".into(),
                instructions,
            },
            &BTreeMap::new(),
        )
        .unwrap();
        let _guard = EXECUTION.lock().unwrap();
        let path = std::env::temp_dir().join(format!(
            "ail-recovery-limited-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, binary).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        // A test harness shell sets Linux resource limits before exec. The ELF
        // itself remains standalone and starts with only its ordinary arguments.
        let mut child = Command::new("sh")
            .args([
                "-c",
                "ulimit -v 196608; ulimit -n 24; exec \"$@\"",
                "recovery-test",
            ])
            .arg(&path)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if child.try_wait().unwrap().is_none() {
            child.kill().unwrap();
            child.wait().unwrap();
            std::fs::remove_file(path).unwrap();
            panic!("native recovery resource probe exceeded ten seconds");
        }
        let output = child.wait_with_output().unwrap();
        std::fs::remove_file(path).unwrap();
        output
    }

    #[test]
    fn native_recovery_reclaims_failed_scratch_and_arbitrary_successful_frees_under_limits() {
        let path = std::env::temp_dir().join(format!(
            "ail-recovery-invalid-utf8-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, [0xff, 0xfe]).unwrap();
        let mut instructions = vec![
            instruction("NATIVE_FUEL_ROOT", &[("limit", "100000")]),
            eval("out.count", "0"),
            eval("data.round", "0"),
            instruction("SET_FIELD", &[("key", "out.value"), ("value", "old")]),
            instruction("LABEL", &[("name", "again")]),
        ];
        for operation in [
            instruction(
                "LIST_LENGTH",
                &[
                    ("key", "out.value"),
                    ("source", "input.badlist"),
                    ("type", "Int"),
                ],
            ),
            instruction(
                "JSON_KIND",
                &[("key", "out.value"), ("source", "input.badjson")],
            ),
            json_get("out.value", "input.json", "input.missingpath", "Int"),
            json_set(
                "out.value",
                "input.json",
                "input.path",
                "input.badlist",
                "List<Int>",
            ),
            instruction(
                "FILE_READ_TEXT",
                &[("key", "out.value"), ("resource", "bad")],
            ),
        ] {
            instructions.extend([
                recovery_begin("any", "data.error"),
                operation,
                instruction("RECOVER", &[]),
                eval("out.count", "out.count + 1"),
                instruction("TRY_END", &[]),
            ]);
        }
        // JSON_SET frees its source and formatter while newer mappings remain.
        // Text matching/splitting exercise independent scratch allocation sizes.
        instructions.extend([
            json_set(
                "data.json",
                "input.json",
                "input.path",
                "input.goodlist",
                "List<Int>",
            ),
            instruction(
                "TEXT_FIND",
                &[
                    ("key", "data.index"),
                    ("source", "input.text"),
                    ("needle", "input.separator"),
                ],
            ),
            instruction(
                "TEXT_SPLIT",
                &[
                    ("key", "data.parts"),
                    ("source", "input.text"),
                    ("separator", "input.separator"),
                ],
            ),
            instruction(
                "TEXT_REPLACE",
                &[
                    ("key", "data.changed"),
                    ("source", "input.text"),
                    ("needle", "input.separator"),
                    ("replacement", "input.text"),
                ],
            ),
            eval("data.round", "data.round + 1"),
            branch_int("data.round", "<", "300", "again"),
            instruction("OUTPUT_FIELD", &[("key", "out.count")]),
            instruction("OUTPUT_FIELD", &[("key", "out.value")]),
            instruction("OUTPUT_FIELD", &[("key", "data.json")]),
        ]);
        let args = [
            "input.badlist=[1,]",
            "input.badjson={",
            "input.json={}",
            "input.path=/value",
            "input.missingpath=/missing",
            "input.goodlist=[7,8]",
            "input.text=a,b,c",
            "input.separator=,",
        ]
        .into_iter()
        .map(String::from)
        .chain([format!("--read-file=bad={}", path.display())])
        .collect::<Vec<_>>();
        let output = run_recovery_limited(instructions, &args);
        std::fs::remove_file(path).unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"data.json={\"value\":[7,8]}\nout.count=1500\nout.value=old\n"
        );
    }

    #[test]
    fn native_recovery_successful_arena_charges_survive_and_static_error_codes_are_free() {
        let mut instructions = vec![
            instruction("NATIVE_FUEL_ROOT", &[("limit", "100000")]),
            instruction(
                "SET_FIELD",
                &[("key", "data.seed"), ("value", &"x".repeat(1_048_576))],
            ),
            eval("data.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            recovery_begin("any", "data.error"),
            text_slice("data.copy", "data.seed", "0", "1048576"),
            eval("data.failed", "1 / 0"),
            instruction("RECOVER", &[]),
            eval("data.count", "data.count + 1"),
            instruction("TRY_END", &[]),
            branch_int("data.count", "<", "64", "again"),
            instruction(
                "TEXT_LENGTH",
                &[("key", "out.length"), ("source", "data.copy")],
            ),
            instruction("OUTPUT_FIELD", &[("key", "out.length")]),
        ];
        let output = run(instructions.clone(), &[]);
        assert!(output.status.success(), "{:?}", output.status);
        assert_eq!(output.stdout, b"out.length=1048576\n");
        instructions.extend([
            recovery_begin("any", "data.error"),
            text_slice("data.copy", "data.copy", "0", "0"),
            instruction("RECOVER", &[]),
            instruction("TRY_END", &[]),
        ]);
        let output = run(instructions, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT"));
    }

    #[test]
    fn native_recovery_any_does_not_catch_capability_io_depth_or_return_protocol_errors() {
        for (operation, args, diagnostic) in [
            (
                instruction(
                    "FILE_READ_TEXT",
                    &[("key", "out.value"), ("resource", "missing")],
                ),
                vec![],
                "AIL-RUNTIME-CAPABILITY",
            ),
            (
                instruction(
                    "FILE_READ_TEXT",
                    &[("key", "out.value"), ("resource", "missing")],
                ),
                vec!["--read-file=missing=/ail-recovery-test-absent-file"],
                "AIL-RUNTIME-IO",
            ),
        ] {
            let output = run(
                vec![
                    recovery_begin("any", "out.error"),
                    operation,
                    instruction("RECOVER", &[]),
                    instruction("TRY_END", &[]),
                ],
                &args,
            );
            assert_eq!(output.status.code(), Some(1), "{output:?}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(diagnostic),
                "{output:?}"
            );
        }
        for (function, diagnostic) in [
            (
                integer_function(
                    "f",
                    "",
                    vec![
                        call_int("f", "", "local.result"),
                        return_int("local.result"),
                    ],
                ),
                "AIL-RUNTIME-DEPTH",
            ),
            (integer_function("f", "", vec![]), "AIL-RUNTIME-RETURN"),
        ] {
            let output = run_functions(
                vec![
                    recovery_begin("any", "out.error"),
                    call_int("f", "", "out.result"),
                    instruction("RECOVER", &[]),
                    instruction("TRY_END", &[]),
                ],
                BTreeMap::from([function]),
                &[],
            );
            assert_eq!(output.status.code(), Some(1), "{output:?}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(diagnostic),
                "{output:?}"
            );
        }
    }

    #[test]
    fn native_recovery_preserves_existing_bool_argument_json_error_identity() {
        let function = value_function(
            "f",
            "value:Bool",
            "",
            "Bool",
            vec![return_value("Bool", "local.value")],
        );
        let output = run_functions(
            vec![
                recovery_begin("JSON", "out.error"),
                call_value("f", "value=f:input.bad", "out.value"),
                instruction("RECOVER", &[]),
                instruction("TRY_END", &[]),
                instruction("OUTPUT_FIELD", &[("key", "out.error")]),
            ],
            BTreeMap::from([function]),
            &["input.bad=TRUE"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.error=AIL-RUNTIME-JSON\n");
    }

    #[test]
    fn native_recovery_charges_exact_throwing_instruction_fuel_and_resumes_paused_caller() {
        for limit in [6, 7] {
            let output = run(
                vec![
                    instruction("NATIVE_FUEL_ROOT", &[("limit", &limit.to_string())]),
                    eval("out.count", "0"),
                    recovery_begin("LIST", "out.error"),
                    instruction(
                        "LIST_LENGTH",
                        &[
                            ("key", "out.count"),
                            ("source", "input.list"),
                            ("type", "Int"),
                        ],
                    ),
                    instruction("RECOVER", &[]),
                    eval("out.count", "out.count + 1"),
                    instruction("TRY_END", &[]),
                    instruction("OUTPUT_FIELD", &[("key", "out.count")]),
                    instruction("OUTPUT_FIELD", &[("key", "out.error")]),
                ],
                &["input.list=[1,]"],
            );
            assert_eq!(
                output.status.success(),
                limit == 7,
                "limit {limit}: {output:?}"
            );
            if limit == 7 {
                assert_eq!(output.stdout, b"out.count=1\nout.error=AIL-RUNTIME-LIST\n");
            } else {
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .contains("execution step limit exceeded")
                );
            }
        }
        for limit in [4, 5] {
            let output = run(
                vec![
                    instruction("NATIVE_FUEL_ROOT", &[("limit", &limit.to_string())]),
                    eval("out.value", "1"),
                    recovery_begin("DIVZERO", "data.error"),
                    instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "3")]),
                    eval("out.value", "11"),
                    eval("out.value", "1 / 0"),
                    instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "0")]),
                    instruction("RECOVER", &[]),
                    eval("data.handled", "1"),
                    instruction("TRY_END", &[]),
                    instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                ],
                &[],
            );
            assert_eq!(
                output.status.success(),
                limit == 5,
                "limit {limit}: {output:?}"
            );
            if limit == 5 {
                assert_eq!(output.stdout, b"out.value=11\n");
            } else {
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .contains("execution step limit exceeded")
                );
            }
        }
    }

    #[test]
    fn native_recovery_text_return_validation_and_value_frame_jump_cleanup() {
        let text = value_function(
            "text",
            "",
            "error:Text;missing:Text",
            "Text",
            vec![
                recovery_begin("TEXT", "local.error"),
                return_value("Text", "local.missing"),
                instruction("RECOVER", &[]),
                return_value("Text", "local.error"),
                instruction("TRY_END", &[]),
            ],
        );
        let jumping = value_function(
            "jumping",
            "",
            "inner:Text;outer:Text",
            "Text",
            vec![
                recovery_begin("any", "local.outer"),
                recovery_begin("any", "local.inner"),
                instruction(
                    "BRANCH_BOOL",
                    &[("expression", "b:true"), ("label", "outside")],
                ),
                instruction("RECOVER", &[]),
                return_value("Text", "local.inner"),
                instruction("TRY_END", &[]),
                instruction("LABEL", &[("name", "outside")]),
                return_value("Text", "local.inner"),
                instruction("RECOVER", &[]),
                return_value("Text", "local.outer"),
                instruction("TRY_END", &[]),
            ],
        );
        let output = run_functions(
            vec![
                call_value("text", "", "out.text"),
                call_value("jumping", "", "out.jump"),
                instruction("OUTPUT_FIELD", &[("key", "out.text")]),
                instruction("OUTPUT_FIELD", &[("key", "out.jump")]),
            ],
            BTreeMap::from([text, jumping]),
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"out.jump=AIL-RUNTIME-TEXT\nout.text=AIL-RUNTIME-TEXT\n"
        );
    }

    fn instruction(opcode: &str, operands: &[(&str, &str)]) -> AilBytecodeInstruction {
        AilBytecodeInstruction::new(
            opcode,
            &operands
                .iter()
                .map(|(k, v)| (*k, (*v).to_string()))
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn native_process_raw_opcode_requires_runtime_and_captures_status() {
        let output = run(
            vec![
                instruction(
                    "PROCESS_RUN",
                    &[
                        ("resource", "tool"),
                        ("arguments", "data.args"),
                        ("input", "data.input"),
                        ("timeout_ms", "data.timeout"),
                        ("stdout", "data.out"),
                        ("stderr", "data.err"),
                        ("status", "data.status"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "data.out")]),
                instruction("OUTPUT_FIELD", &[("key", "data.status")]),
            ],
            &[
                "--run-process=tool=/bin/cat",
                "data.args=[]",
                "data.input=hello",
                "data.timeout=2000",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"data.out=hello\ndata.status=0\n");
    }

    #[test]
    fn native_directory_raw_opcodes_list_read_write() {
        let root = file_test_directory();
        std::fs::write(root.join("a"), "é\0\n").unwrap();
        let output = run(
            vec![
                instruction(
                    "DIRECTORY_LIST_NAMES",
                    &[
                        ("resource", "files"),
                        ("path", "data.root"),
                        ("key", "data.names"),
                    ],
                ),
                instruction(
                    "DIRECTORY_READ_TEXT",
                    &[
                        ("resource", "files"),
                        ("path", "data.path"),
                        ("key", "data.path"),
                    ],
                ),
                instruction(
                    "DIRECTORY_WRITE_TEXT",
                    &[
                        ("resource", "files"),
                        ("path", "data.destination"),
                        ("source", "data.path"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "data.names")]),
                instruction("OUTPUT_FIELD", &[("key", "data.path")]),
            ],
            &[
                &format!("--read-directory=files={}", root.display()),
                &format!("--write-directory=files={}", root.display()),
                "data.root=",
                "data.path=a",
                "data.destination=b",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            "data.names=[\"a\"]\ndata.path=é\0\n\n".as_bytes()
        );
        assert_eq!(output.stderr, b"listed names in directory resource files\nread text from directory resource files\nwrote text to directory resource files\n");
        assert_eq!(std::fs::read(root.join("b")).unwrap(), "é\0\n".as_bytes());
        std::fs::remove_dir_all(root).unwrap();
    }

    fn directory_instruction(
        opcode: &str,
        path: &str,
        destination: &str,
    ) -> AilBytecodeInstruction {
        instruction(
            opcode,
            &[
                ("resource", "files"),
                ("path", path),
                (
                    if opcode == "DIRECTORY_WRITE_TEXT" {
                        "source"
                    } else {
                        "key"
                    },
                    destination,
                ),
            ],
        )
    }

    #[test]
    fn native_directory_authority_and_input_priority_precede_filesystem_effects() {
        use std::os::unix::ffi::OsStringExt;
        let root = file_test_directory();
        std::fs::write(root.join("file"), b"original").unwrap();
        let read = format!("--read-directory=files={}", root.display());
        let write = format!("--write-directory=files={}", root.display());
        for (opcode, args, expected) in [
            ("DIRECTORY_READ_TEXT", vec![], "CAPABILITY"),
            ("DIRECTORY_READ_TEXT", vec![write.as_str()], "CAPABILITY"),
            ("DIRECTORY_READ_TEXT", vec![read.as_str()], "TEXT"),
            (
                "DIRECTORY_WRITE_TEXT",
                vec![read.as_str(), "data.path=../file"],
                "CAPABILITY",
            ),
            (
                "DIRECTORY_WRITE_TEXT",
                vec![write.as_str(), "data.path=../file"],
                "PATH",
            ),
            (
                "DIRECTORY_WRITE_TEXT",
                vec![write.as_str(), "data.path=file"],
                "TEXT",
            ),
            (
                "DIRECTORY_READ_TEXT",
                vec![read.as_str(), "data.path=missing"],
                "PATH",
            ),
        ] {
            let output = run(
                vec![directory_instruction(opcode, "data.path", "data.result")],
                &args,
            );
            assert_eq!(output.status.code(), Some(1), "{output:?}");
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains(&format!("AIL-RUNTIME-{expected}:")),
                "{output:?}"
            );
            assert!(output.stdout.is_empty());
        }
        assert_eq!(std::fs::read(root.join("file")).unwrap(), b"original");
        let bad = std::ffi::OsString::from_vec(b"data.path=\xff".to_vec());
        let output = run_os(
            vec![directory_instruction(
                "DIRECTORY_READ_TEXT",
                "data.path",
                "data.result",
            )],
            &[read.into(), bad],
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT:"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_directory_grants_validate_every_flag_and_never_leak_in_both_emitters() {
        let root = file_test_directory();
        let valid = format!("--read-directory=unused={}", root.display());
        let action = AilBytecodeAction {
            name: "Quiet".into(),
            instructions: vec![
                instruction("SET_FIELD", &[("key", "data.value"), ("value", "effect")]),
                instruction("RETURN_SUCCESS", &[]),
            ],
        };
        let malformed = [
            "--read-directory",
            "--write-directory-x",
            "--read-directory=bad..name=/tmp",
            "--write-directory=ok=relative",
            "--read-directory==/tmp",
            "--read-directory=ok=",
        ]
        .map(str::to_string);
        for binary in [
            emit(&action, &BTreeMap::new()).unwrap(),
            emit_linux_x86_64_elf_for_action(&action, &BTreeMap::new()).unwrap(),
        ] {
            let path = root.join("probe");
            std::fs::write(&path, binary).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            for bad in malformed.iter().cloned().chain([
                format!("--read-directory={}=/tmp", "x".repeat(257)),
                format!("--write-directory=ok=/{}", "x".repeat(4095)),
            ]) {
                let output = Command::new(&path)
                    .args([bad.as_str(), &valid])
                    .output()
                    .unwrap();
                assert_eq!(output.status.code(), Some(1), "{bad}: {output:?}");
                assert!(output.stdout.is_empty(), "{output:?}");
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-CAPABILITY:")
                );
            }
            let output = Command::new(&path).args([&valid, &valid]).output().unwrap();
            assert!(output.status.success(), "{output:?}");
            assert!(!String::from_utf8_lossy(&output.stdout).contains("--read-directory"));
            assert!(
                !String::from_utf8_lossy(&output.stdout)
                    .contains(&root.to_string_lossy().to_string())
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_directory_startup_diagnostics_survive_broken_stderr() {
        use std::os::fd::OwnedFd;
        use std::os::unix::net::UnixStream;
        let root = file_test_directory();
        let action = AilBytecodeAction {
            name: "MustNotRun".into(),
            instructions: vec![
                instruction(
                    "SET_FIELD",
                    &[("key", "data.effect"), ("value", "must not execute")],
                ),
                instruction("RETURN_SUCCESS", &[]),
            ],
        };
        for binary in [
            emit(&action, &BTreeMap::new()).unwrap(),
            emit_linux_x86_64_elf_for_action(&action, &BTreeMap::new()).unwrap(),
        ] {
            let path = root.join("probe");
            std::fs::write(&path, binary).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            for args in [
                vec!["--read-directory=bad=relative"],
                vec!["--stdio", "--write-directory"],
                vec!["--read-directory", "--stdio"],
            ] {
                let (writer, reader) = UnixStream::pair().unwrap();
                drop(reader);
                let output = Command::new(&path)
                    .args(args)
                    .stderr(std::process::Stdio::from(OwnedFd::from(writer)))
                    .output()
                    .unwrap();
                assert_eq!(output.status.code(), Some(1), "{output:?}");
                assert!(output.stdout.is_empty());
            }
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_directory_duplicate_modes_are_independent_and_last_binding_wins() {
        let first = file_test_directory();
        let second = file_test_directory();
        std::fs::write(first.join("file"), b"first").unwrap();
        std::fs::write(second.join("file"), b"second").unwrap();
        let output = run(
            vec![
                directory_instruction("DIRECTORY_READ_TEXT", "data.path", "data.text"),
                directory_instruction("DIRECTORY_WRITE_TEXT", "data.path", "data.text"),
                instruction(
                    "COPY_FIELD",
                    &[("key", "data.alias"), ("source", "data.text")],
                ),
                instruction("SET_FIELD", &[("key", "data.path"), ("value", "")]),
                directory_instruction("DIRECTORY_LIST_NAMES", "data.path", "data.names"),
                instruction("OUTPUT_FIELD", &[("key", "data.alias")]),
            ],
            &[
                &format!("--read-directory=files={}", first.display()),
                &format!("--read-directory=files={}", second.display()),
                &format!("--write-directory=files={}", first.display()),
                "data.path=file",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"data.alias=second\n");
        assert_eq!(std::fs::read(first.join("file")).unwrap(), b"second");
        std::fs::remove_dir_all(first).unwrap();
        std::fs::remove_dir_all(second).unwrap();
    }

    #[test]
    fn native_directory_recovery_releases_scratch_and_keeps_prior_values() {
        let root = file_test_directory();
        std::fs::write(root.join("invalid"), vec![0xff; 1_048_576]).unwrap();
        let operations = vec![
            instruction("NATIVE_FUEL_ROOT", &[("limit", "100000")]),
            instruction("SET_FIELD", &[("key", "out.value"), ("value", "old")]),
            eval("out.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            recovery_begin("TEXT", "data.error"),
            directory_instruction("DIRECTORY_READ_TEXT", "data.path", "out.value"),
            instruction("RECOVER", &[]),
            eval("out.count", "out.count + 1"),
            instruction("TRY_END", &[]),
            recovery_begin("PATH", "data.error"),
            directory_instruction("DIRECTORY_READ_TEXT", "data.missing", "out.value"),
            instruction("RECOVER", &[]),
            instruction("TRY_END", &[]),
            branch_int("out.count", "<", "300", "again"),
            instruction("OUTPUT_FIELD", &[("key", "out.count")]),
            instruction("OUTPUT_FIELD", &[("key", "out.value")]),
        ];
        let output = run_recovery_limited(
            operations,
            &[
                format!("--read-directory=files={}", root.display()),
                "data.path=invalid".into(),
                "data.missing=missing".into(),
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.count=300\nout.value=old\n");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_directory_successful_results_charge_only_the_designated_shared_arena() {
        let root = file_test_directory();
        std::fs::create_dir(root.join("empty")).unwrap();
        std::fs::create_dir(root.join("full-list")).unwrap();
        std::fs::write(root.join("empty-text"), b"").unwrap();
        std::fs::write(root.join("full-text"), vec![b'x'; 1_048_576]).unwrap();
        // 4096*(253+3)+1 - 1 = exactly 1 MiB canonical JSON.
        for i in 0..4096 {
            std::fs::write(
                root.join("full-list").join(format!(
                    "{i:04}{}",
                    "x".repeat(if i == 0 { 248 } else { 249 })
                )),
                b"",
            )
            .unwrap();
        }
        for (opcode, path, other, other_path, empty, category) in [
            (
                "DIRECTORY_READ_TEXT",
                "full-text",
                "DIRECTORY_LIST_NAMES",
                "empty",
                "empty-text",
                "TEXT-LIMIT",
            ),
            (
                "DIRECTORY_LIST_NAMES",
                "full-list",
                "DIRECTORY_READ_TEXT",
                "empty-text",
                "empty",
                "LIST-LIMIT",
            ),
        ] {
            let code = vec![
                eval("out.count", "0"),
                instruction("LABEL", &[("name", "again")]),
                directory_instruction(opcode, "data.path", "data.result"),
                eval("out.count", "out.count + 1"),
                branch_int("out.count", "<", "64", "again"),
                directory_instruction(other, "data.other", "data.other_result"),
                instruction("OUTPUT_FIELD", &[("key", "out.count")]),
            ];
            let args = [
                format!("--read-directory=files={}", root.display()),
                format!("data.path={path}"),
                format!("data.other={other_path}"),
                format!("data.empty={empty}"),
            ];
            let args = args.iter().map(String::as_str).collect::<Vec<_>>();
            let output = run(code.clone(), &args);
            assert!(output.status.success(), "{opcode}: {output:?}");
            assert_eq!(output.stdout, b"out.count=64\n");
            let mut exhausted = code;
            exhausted.push(directory_instruction(opcode, "data.empty", "data.result"));
            let output = run(exhausted, &args);
            assert_eq!(output.status.code(), Some(1), "{opcode}: {output:?}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains(&format!("AIL-RUNTIME-{category}:"))
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    fn process_run() -> AilBytecodeInstruction {
        instruction(
            "PROCESS_RUN",
            &[
                ("resource", "tool"),
                ("arguments", "data.args"),
                ("input", "data.input"),
                ("timeout_ms", "data.timeout"),
                ("stdout", "data.out"),
                ("stderr", "data.err"),
                ("status", "data.status"),
            ],
        )
    }

    #[test]
    fn native_process_grants_never_become_automatic_state_output() {
        for operations in [vec![eval("data.count", "1")], vec![process_run()]] {
            let output = run(
                operations,
                &[
                    "--run-process=tool=/bin/true",
                    "--run-process=unused=/bin/false",
                    "data.args=[]",
                    "data.input=",
                    "data.timeout=2000",
                ],
            );
            assert!(output.status.success(), "{output:?}");
            for forbidden in ["--run-process", "/bin/true", "/bin/false"] {
                assert!(
                    !String::from_utf8_lossy(&output.stdout).contains(forbidden),
                    "{output:?}"
                );
                assert!(
                    !String::from_utf8_lossy(&output.stderr).contains(forbidden),
                    "{output:?}"
                );
            }
        }
    }

    #[test]
    fn native_process_grants_are_checked_before_operands_and_before_effects() {
        for flag in [
            "--run-process",
            "--run-process=",
            "--run-process=tool=relative",
            "--run-process=.tool=/bin/true",
            "--run-process=tool.=/bin/true",
            "--run-processX=tool=/bin/true",
        ] {
            let output = run(vec![eval("out.prior", "5")], &[flag]);
            assert!(!output.status.success(), "{flag}: {output:?}");
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).starts_with("AIL-RUNTIME-CAPABILITY:"));
        }
        let long = format!("--run-process=unused=/{}", "x".repeat(4095));
        let output = run(vec![eval("out.prior", "5")], &[&long]);
        assert!(!output.status.success());
        let output = run(vec![process_run()], &["data.args=malformed"]);
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("AIL-RUNTIME-CAPABILITY:"));
        let output = run(
            vec![
                process_run(),
                instruction("OUTPUT_FIELD", &[("key", "data.status")]),
            ],
            &[
                "--run-process=tool=/does/not/exist",
                "--run-process=tool=/bin/true",
                "data.args=[]",
                "data.input=",
                "data.timeout=2000",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"data.status=0\n");
    }

    #[test]
    fn native_process_operand_validation_priority_and_argument_limits() {
        let excessive = format!("[{}]", vec!["\"\""; 257].join(","));
        for (arguments, input, timeout, error) in [
            ("[false]", None, "bad", "LIST"),
            ("[\"\\u0000\"]", None, "bad", "PROCESS"),
            (excessive.as_str(), None, "bad", "PROCESS-LIMIT"),
            ("[]", None, "bad", "TEXT"),
            ("[]", Some(""), "bad", "INT"),
            ("[]", Some(""), "0", "PROCESS"),
            ("[]", Some(""), "60001", "PROCESS"),
        ] {
            let mut args = vec![
                "--run-process=tool=/bin/true".to_string(),
                format!("data.args={arguments}"),
                format!("data.timeout={timeout}"),
            ];
            if let Some(input) = input {
                args.push(format!("data.input={input}"));
            }
            let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
            let output = run(vec![process_run()], &refs);
            assert!(!output.status.success());
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .starts_with(&format!("AIL-RUNTIME-{error}:")),
                "{args:?}: {output:?}"
            );
        }
        let available = 65_536 - "/bin/true".len() - 2;
        for (length, success) in [(available, true), (available + 1, false)] {
            let list = format!("data.args=[\"{}\"]", "x".repeat(length));
            let output = run(
                vec![process_run()],
                &[
                    "--run-process=tool=/bin/true",
                    &list,
                    "data.input=",
                    "data.timeout=2000",
                ],
            );
            assert_eq!(
                output.status.success(),
                success,
                "length {length}: {output:?}"
            );
            if !success {
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .starts_with("AIL-RUNTIME-PROCESS-LIMIT:")
                );
            }
        }
    }

    #[test]
    fn native_process_recovery_keeps_all_destinations_and_continues_after_cleanup() {
        for (path, list, timeout, error) in [
            ("/does/not/exist", "[]", "1000", "PROCESS"),
            ("/bin/sleep", "[\"1\"]", "10", "TIMEOUT"),
            (
                "/usr/bin/python3",
                "[\"-c\",\"import os; os.write(1,b'new'); os.write(2,bytes([255]))\"]",
                "2000",
                "TEXT",
            ),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "data.out"), ("value", "old out")]),
                    instruction("SET_FIELD", &[("key", "data.err"), ("value", "old err")]),
                    eval("data.status", "99"),
                    recovery_begin(error, "data.error"),
                    process_run(),
                    instruction("RECOVER", &[]),
                    instruction("TRY_END", &[]),
                    instruction("OUTPUT_FIELD", &[("key", "data.out")]),
                    instruction("OUTPUT_FIELD", &[("key", "data.err")]),
                    instruction("OUTPUT_FIELD", &[("key", "data.status")]),
                    instruction("OUTPUT_FIELD", &[("key", "data.error")]),
                ],
                &[
                    &format!("--run-process=tool={path}"),
                    &format!("data.args={list}"),
                    "data.input=",
                    &format!("data.timeout={timeout}"),
                ],
            );
            assert!(output.status.success(), "{error}: {output:?}");
            assert_eq!(output.stdout, format!("data.err=old err\ndata.error=AIL-RUNTIME-{error}\ndata.out=old out\ndata.status=99\n").as_bytes());
        }
    }

    #[test]
    fn native_process_aliases_preserve_input_and_timeout_and_prior_captures() {
        let mut operation = process_run();
        operation
            .operands
            .insert("stdout".into(), "data.input".into());
        operation
            .operands
            .insert("status".into(), "data.timeout".into());
        let output = run(
            vec![
                operation.clone(),
                instruction(
                    "COPY_FIELD",
                    &[("key", "data.saved"), ("source", "data.input")],
                ),
                instruction("SET_FIELD", &[("key", "data.input"), ("value", "second")]),
                eval("data.timeout", "2000"),
                operation,
                instruction("OUTPUT_FIELD", &[("key", "data.input")]),
                instruction("OUTPUT_FIELD", &[("key", "data.saved")]),
                instruction("OUTPUT_FIELD", &[("key", "data.timeout")]),
            ],
            &[
                "--run-process=tool=/bin/cat",
                "data.args=[]",
                "data.input=first",
                "data.timeout=2000",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"data.input=second\ndata.saved=first\ndata.timeout=0\n"
        );
    }

    #[test]
    fn native_process_repeated_invalid_second_capture_does_not_charge_text_arena() {
        let mut operations = vec![
            eval("data.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            recovery_begin("TEXT", "data.error"),
            process_run(),
            instruction("RECOVER", &[]),
            instruction("TRY_END", &[]),
            eval("data.count", "data.count + 1"),
        ];
        operations.push(instruction(
            "BRANCH_INT",
            &[
                ("left", &IntExpr::parse("data.count").unwrap().to_postfix()),
                ("right", &IntExpr::parse("65").unwrap().to_postfix()),
                ("comparison", "<"),
                ("label", "again"),
            ],
        ));
        operations.push(instruction("OUTPUT_FIELD", &[("key", "data.count")]));
        let output = run(
            operations,
            &[
                "--run-process=tool=/usr/bin/python3",
                "data.args=[\"-c\",\"import os; os.write(1,b'x'*1048576); os.write(2,bytes([255]))\"]",
                "data.input=",
                "data.timeout=2000",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"data.count=65\n");
    }

    #[test]
    fn native_process_empty_captures_reserve_two_bytes_together() {
        for (remaining, success) in [(2, true), (1, false)] {
            let source = "x".repeat(1_048_576);
            let mut operations = vec![instruction(
                "SET_FIELD",
                &[("key", "data.source"), ("value", &source)],
            )];
            for _ in 0..63 {
                operations.push(text_slice("data.fill", "data.source", "0", "1048576"));
            }
            operations.push(text_slice(
                "data.fill",
                "data.source",
                "0",
                &(1_048_576 - remaining).to_string(),
            ));
            operations.push(process_run());
            operations.push(instruction("OUTPUT_FIELD", &[("key", "data.status")]));
            let output = run(
                operations,
                &[
                    "--run-process=tool=/bin/true",
                    "data.args=[]",
                    "data.input=",
                    "data.timeout=2000",
                ],
            );
            assert_eq!(
                output.status.success(),
                success,
                "remaining={remaining}: {output:?}"
            );
            if success {
                assert_eq!(output.stdout, b"data.status=0\n");
            } else {
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT:")
                );
            }
        }
    }

    fn run(instructions: Vec<AilBytecodeInstruction>, args: &[&str]) -> Output {
        run_os(
            instructions,
            &args
                .iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>(),
        )
    }

    fn run_os(instructions: Vec<AilBytecodeInstruction>, args: &[std::ffi::OsString]) -> Output {
        // Avoid concurrent child creation while another test has an ELF open for writing.
        let _guard = EXECUTION.lock().unwrap();
        let action = AilBytecodeAction {
            name: "Compute".to_string(),
            instructions,
        };
        let binary = emit(&action, &BTreeMap::new()).expect("native compilation");
        let path = std::env::temp_dir().join(format!(
            "ail-native-integer-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, binary).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let output = Command::new(&path).args(args).output().unwrap();
        std::fs::remove_file(path).unwrap();
        output
    }

    fn text_slice(key: &str, source: &str, start: &str, end: &str) -> AilBytecodeInstruction {
        instruction(
            "TEXT_SLICE",
            &[
                ("key", key),
                ("source", source),
                ("start", &IntExpr::parse(start).unwrap().to_postfix()),
                ("end", &IntExpr::parse(end).unwrap().to_postfix()),
            ],
        )
    }

    fn json_set(
        key: &str,
        source: &str,
        path: &str,
        value: &str,
        kind: &str,
    ) -> AilBytecodeInstruction {
        instruction(
            "JSON_SET",
            &[
                ("key", key),
                ("source", source),
                ("path", path),
                ("value", value),
                ("type", kind),
            ],
        )
    }
    fn json_remove(key: &str, source: &str, path: &str) -> AilBytecodeInstruction {
        instruction(
            "JSON_REMOVE",
            &[("key", key), ("source", source), ("path", path)],
        )
    }

    #[test]
    fn native_json_mutation_formats_all_six_types_without_losing_json_tokens() {
        for (kind, value, encoded) in [
            ("Int", "+00042", "42"),
            ("Int", "-9223372036854775808", "-9223372036854775808"),
            ("Bool", "false", "false"),
            ("Bool", "true", "true"),
            (
                "Text",
                "🦀\0\n\t\r\u{7f}\u{85}",
                "\"🦀\\u0000\\n\\t\\r\\u007f\\u0085\"",
            ),
            ("List<Int>", " [ -0, 42 ] ", "[0,42]"),
            (
                "List<Text>",
                r#"["\ud83e\udd80", "\u0000"]"#,
                "[\"🦀\",\"\\u0000\"]",
            ),
            (
                "Json",
                r#" {"a":-0.00E+999,"a":[null,true,999999999999999999999999999999]} "#,
                r#"{"a":-0.00E+999,"a":[null,true,999999999999999999999999999999]}"#,
            ),
        ] {
            for (source, path, expected) in [
                ("{}", "/new", format!("{{\"new\":{encoded}}}")),
                ("[0]", "/0", format!("[{encoded}]")),
                ("null", "", encoded.to_string()),
            ] {
                let output = run(
                    vec![
                        instruction("SET_FIELD", &[("key", "data.source"), ("value", source)]),
                        instruction("SET_FIELD", &[("key", "data.value"), ("value", value)]),
                        instruction("SET_FIELD", &[("key", "data.path"), ("value", path)]),
                        json_set("out.value", "data.source", "data.path", "data.value", kind),
                        instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                    ],
                    &[],
                );
                assert!(output.status.success(), "{kind} {path}: {output:?}");
                assert_eq!(output.stdout, format!("out.value={expected}\n").as_bytes());
            }
        }
    }

    #[test]
    fn native_json_mutation_removes_first_middle_last_and_only_members() {
        for (source, path, expected) in [
            ("[0,1,2]", "/0", "[1,2]"),
            ("[0,1,2]", "/1", "[0,2]"),
            ("[0,1,2]", "/2", "[0,1]"),
            ("[0]", "/0", "[]"),
            (r#"{"a":0,"b":1,"c":2}"#, "/a", r#"{"b":1,"c":2}"#),
            (r#"{"a":0,"b":1,"c":2}"#, "/b", r#"{"a":0,"c":2}"#),
            (r#"{"a":0,"b":1,"c":2}"#, "/c", r#"{"a":0,"b":1}"#),
            (r#"{"a":0}"#, "/a", "{}"),
            (
                r#"{"a/b":{"":0,"~":1,"\u0000":2},"d":1,"d":2}"#,
                "/a~1b/\0",
                r#"{"a/b":{"":0,"~":1},"d":1,"d":2}"#,
            ),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "data.source"), ("value", source)]),
                    instruction("SET_FIELD", &[("key", "data.path"), ("value", path)]),
                    json_remove("out.value", "data.source", "data.path"),
                    instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                ],
                &[],
            );
            assert!(output.status.success(), "{source} {path}: {output:?}");
            assert_eq!(output.stdout, format!("out.value={expected}\n").as_bytes());
        }
    }

    #[test]
    fn native_json_mutation_appends_object_keys_and_decodes_pointer_escapes_once() {
        for (source, path, expected) in [
            ("[]", "/-", "[7]"),
            ("[1]", "/-", "[1,7]"),
            ("{}", "/-", r#"{"-":7}"#),
            (
                r#"{"a":1,"d":true,"d":null}"#,
                "/a",
                r#"{"a":7,"d":true,"d":null}"#,
            ),
            (
                r#"{"a":1,"d":true,"d":null}"#,
                "/new",
                r#"{"a":1,"d":true,"d":null,"new":7}"#,
            ),
            ("{}", "/", r#"{"":7}"#),
            ("{}", "/~01", r#"{"~1":7}"#),
            ("{}", "/a~1b~0\0\n\u{7f}", "{\"a/b~\\u0000\\n\\u007f\":7}"),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "data.source"), ("value", source)]),
                    instruction("SET_FIELD", &[("key", "data.path"), ("value", path)]),
                    json_set(
                        "out.value",
                        "data.source",
                        "data.path",
                        "input.value",
                        "Int",
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                ],
                &["input.value=7"],
            );
            assert!(output.status.success(), "{source} {path:?}: {output:?}");
            assert_eq!(output.stdout, format!("out.value={expected}\n").as_bytes());
        }
    }

    #[test]
    fn native_json_mutation_validates_source_path_target_then_replacement() {
        for (source, path, expected) in [
            ("[invalid]", "/bad~2", "AIL-RUNTIME-JSON:"),
            ("[invalid]", "", "AIL-RUNTIME-JSON:"),
            ("{}", "/bad~2", "AIL-RUNTIME-JSON-PATH:"),
            ("{}", "/missing/x", "AIL-RUNTIME-JSON-PATH:"),
            ("{}", "/missing/x/~3", "AIL-RUNTIME-JSON-PATH:"),
            ("[1]", "/01", "AIL-RUNTIME-JSON-PATH:"),
            ("[1]", "/1", "AIL-RUNTIME-JSON-PATH:"),
            ("[1]", "/-1", "AIL-RUNTIME-JSON-PATH:"),
            ("[1]", "/-/x", "AIL-RUNTIME-JSON-PATH:"),
            ("[1]", "/99999999999999999999999", "AIL-RUNTIME-JSON-PATH:"),
            (r#"{"a":1,"\u0061":2}"#, "/a", "AIL-RUNTIME-JSON-PATH:"),
            ("1", "/x", "AIL-RUNTIME-JSON-PATH:"),
            ("{}", "/new", "AIL-RUNTIME-JSON:"),
            ("null", "", "AIL-RUNTIME-JSON:"),
            ("[]", "/-", "AIL-RUNTIME-JSON:"),
        ] {
            let output = run(
                vec![json_set(
                    "out.value",
                    "input.source",
                    "input.path",
                    "input.value",
                    "Bool",
                )],
                &[
                    &format!("input.source={source}"),
                    &format!("input.path={path}"),
                    "input.value=FALSE",
                ],
            );
            assert_eq!(output.status.code(), Some(1), "{source} {path}: {output:?}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(expected),
                "{source} {path}: {output:?}"
            );
        }
        for (source, path) in [
            ("{}", ""),
            ("{}", "/missing"),
            ("[]", "/-"),
            ("[1]", "/1"),
            (r#"{"a":1,"a":2}"#, "/a"),
        ] {
            let output = run(
                vec![json_remove("out.value", "input.source", "input.path")],
                &[
                    &format!("input.source={source}"),
                    &format!("input.path={path}"),
                ],
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-PATH:"));
        }
        for (kind, value) in [
            ("Int", "1.0"),
            ("Int", "9223372036854775808"),
            ("Bool", "true "),
            ("List<Int>", "[1,true]"),
            ("List<Text>", "[null]"),
            ("Json", "[1,]"),
        ] {
            let output = run(
                vec![json_set(
                    "out.value",
                    "input.source",
                    "input.path",
                    "input.value",
                    kind,
                )],
                &[
                    "input.source={}",
                    "input.path=/x",
                    &format!("input.value={value}"),
                ],
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"),
                "{kind} {value}: {output:?}"
            );
        }
    }

    #[test]
    fn native_json_mutation_missing_and_invalid_utf8_operands_use_json_errors() {
        use std::os::unix::ffi::OsStringExt;
        let operation = || {
            json_set(
                "out.value",
                "input.source",
                "input.path",
                "input.value",
                "Text",
            )
        };
        for args in [
            vec!["input.path=/x".into(), "input.value=value".into()],
            vec!["input.source={}".into(), "input.value=value".into()],
            vec!["input.source={}".into(), "input.path=/x".into()],
            vec![
                std::ffi::OsString::from_vec(b"input.source=\xff".to_vec()),
                "input.path=/x".into(),
                "input.value=value".into(),
            ],
            vec![
                "input.source={}".into(),
                std::ffi::OsString::from_vec(b"input.path=/\xff".to_vec()),
                "input.value=value".into(),
            ],
            vec![
                "input.source={}".into(),
                "input.path=/x".into(),
                std::ffi::OsString::from_vec(b"input.value=\xff".to_vec()),
            ],
        ] {
            let output = run_os(vec![operation()], &args);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"),
                "{output:?}"
            );
        }
        let output = run_os(
            vec![operation()],
            &[
                "input.source=[]".into(),
                "input.path=/1".into(),
                std::ffi::OsString::from_vec(b"input.value=\xff".to_vec()),
            ],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-PATH:"));
    }

    #[test]
    fn native_json_mutation_keeps_aliases_and_source_path_value_reads_immutable() {
        let output = run(
            vec![
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.original"), ("source", "input.source")],
                ),
                json_set(
                    "input.source",
                    "input.source",
                    "input.path",
                    "input.source",
                    "Json",
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.copy"), ("source", "input.source")],
                ),
                json_set(
                    "input.path",
                    "input.source",
                    "input.path",
                    "input.path",
                    "Text",
                ),
                json_remove("out.value", "input.path", "input.remove"),
                instruction("OUTPUT_FIELD", &[("key", "out.original")]),
                instruction("OUTPUT_FIELD", &[("key", "out.copy")]),
                instruction("OUTPUT_FIELD", &[("key", "out.value")]),
            ],
            &[
                r#"input.source={"a":1}"#,
                "input.path=/a",
                "input.remove=/a",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"out.copy={\"a\":{\"a\":1}}\nout.original={\"a\":1}\nout.value={}\n"
        );
    }

    #[test]
    fn native_json_mutation_replacement_excludes_removed_values_from_structural_limits() {
        let maximum = format!("[{}]", vec!["0"; 65_535].join(","));
        let too_many = format!("[{}]", vec!["0"; 65_536].join(","));
        let nested = |depth| format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
        for (source, path, value, kind, success) in [
            (
                maximum.clone(),
                "/0".to_string(),
                "1".to_string(),
                "Json",
                true,
            ),
            (
                maximum.clone(),
                "/-".to_string(),
                "1".to_string(),
                "Json",
                false,
            ),
            (
                "{}".to_string(),
                "".to_string(),
                maximum.clone(),
                "Json",
                true,
            ),
            ("{}".to_string(), "/x".to_string(), maximum, "Json", false),
            (
                "null".to_string(),
                "".to_string(),
                too_many,
                "List<Int>",
                false,
            ),
            (nested(64), "/0".to_string(), nested(63), "Json", true),
            (nested(64), "/0".to_string(), nested(64), "Json", false),
            (
                "null".to_string(),
                "".to_string(),
                nested(65),
                "Json",
                false,
            ),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "data.source"), ("value", &source)]),
                    instruction("SET_FIELD", &[("key", "data.path"), ("value", &path)]),
                    instruction("SET_FIELD", &[("key", "data.value"), ("value", &value)]),
                    json_set(
                        "data.source",
                        "data.source",
                        "data.path",
                        "data.value",
                        kind,
                    ),
                    instruction("SET_FIELD", &[("key", "out.done"), ("value", "yes")]),
                    instruction("OUTPUT_FIELD", &[("key", "out.done")]),
                ],
                &[],
            );
            assert_eq!(
                output.status.success(),
                success,
                "{} {path} {}: {:?}",
                source.len(),
                value.len(),
                output.status
            );
            if success {
                assert_eq!(output.stdout, b"out.done=yes\n");
            } else {
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT:")
                );
            }
        }
    }

    #[test]
    fn native_json_mutation_checks_raw_canonical_and_result_sizes_in_order() {
        let expansion = format!("\"{}\"", "\u{7f}".repeat(200_000));
        for (source, path, value, kind, expected) in [
            (
                "{}".to_string(),
                "/x".to_string(),
                "a".repeat(1_048_568),
                "Text",
                None,
            ),
            (
                "{}".to_string(),
                "/x".to_string(),
                "a".repeat(1_048_569),
                "Text",
                Some("AIL-RUNTIME-JSON-LIMIT:"),
            ),
            (
                expansion.clone(),
                "".to_string(),
                "1".to_string(),
                "Int",
                Some("AIL-RUNTIME-JSON-LIMIT:"),
            ),
            (
                format!("{expansion}bad"),
                "".to_string(),
                "1".to_string(),
                "Int",
                Some("AIL-RUNTIME-JSON:"),
            ),
            (
                "{}".to_string(),
                "/x".to_string(),
                expansion,
                "Json",
                Some("AIL-RUNTIME-JSON-LIMIT:"),
            ),
            (
                "{}".to_string(),
                "/x".to_string(),
                " ".repeat(1_048_577),
                "Int",
                Some("AIL-RUNTIME-JSON-LIMIT:"),
            ),
            (
                "{}".to_string(),
                format!("/{}", "\u{7f}".repeat(200_000)),
                "bad".to_string(),
                "Bool",
                Some("AIL-RUNTIME-JSON:"),
            ),
            (
                "{}".to_string(),
                format!("/{}", "\u{7f}".repeat(200_000)),
                "true".to_string(),
                "Bool",
                Some("AIL-RUNTIME-JSON-LIMIT:"),
            ),
            (
                "{}".to_string(),
                format!("/{}", "a".repeat(1_048_576)),
                "bad".to_string(),
                "Bool",
                Some("AIL-RUNTIME-JSON-LIMIT:"),
            ),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "data.source"), ("value", &source)]),
                    instruction("SET_FIELD", &[("key", "data.path"), ("value", &path)]),
                    instruction("SET_FIELD", &[("key", "data.value"), ("value", &value)]),
                    json_set(
                        "data.source",
                        "data.source",
                        "data.path",
                        "data.value",
                        kind,
                    ),
                    instruction(
                        "TEXT_LENGTH",
                        &[("key", "out.length"), ("source", "data.source")],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.length")]),
                ],
                &[],
            );
            if let Some(error) = expected {
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains(error),
                    "{} {} {}: {:?}",
                    source.len(),
                    path.len(),
                    value.len(),
                    output.status
                );
            } else {
                assert!(output.status.success(), "{:?}", output.status);
                assert_eq!(output.stdout, b"out.length=1048576\n");
            }
        }
    }

    #[test]
    fn native_json_mutation_reserves_only_final_json_and_shares_list_budget() {
        let value = "a".repeat(1_048_572);
        let mut instructions = vec![
            instruction("SET_FIELD", &[("key", "data.source"), ("value", "[0]")]),
            instruction("SET_FIELD", &[("key", "data.path"), ("value", "/0")]),
            instruction("SET_FIELD", &[("key", "data.value"), ("value", &value)]),
            instruction("SET_FIELD", &[("key", "data.empty"), ("value", "")]),
            eval("loop.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "2048")]),
            json_set(
                "data.result",
                "data.source",
                "data.path",
                "data.value",
                "Text",
            ),
            instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "1")]),
            eval("loop.count", "loop.count + 1"),
            branch_int("loop.count", "<", "64", "again"),
            instruction(
                "TEXT_JOIN",
                &[
                    ("key", "data.text"),
                    ("left", "data.value"),
                    ("right", "data.empty"),
                ],
            ),
            instruction(
                "JSON_COUNT",
                &[("key", "out.count"), ("source", "data.result")],
            ),
            instruction("OUTPUT_FIELD", &[("key", "out.count")]),
        ];
        let output = run(instructions.clone(), &[]);
        assert!(output.status.success(), "{:?}", output.status);
        assert_eq!(output.stdout, b"out.count=1\n");
        instructions.push(json_remove("data.result", "data.source", "data.path"));
        let output = run(instructions, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT:"));
    }

    #[test]
    fn native_json_mutation_replaces_appends_and_removes_in_order() {
        let output = run(
            vec![
                instruction(
                    "JSON_SET",
                    &[
                        ("key", "data.document"),
                        ("source", "input.raw"),
                        ("path", "input.replace"),
                        ("value", "input.value"),
                        ("type", "Int"),
                    ],
                ),
                instruction(
                    "JSON_SET",
                    &[
                        ("key", "data.document"),
                        ("source", "data.document"),
                        ("path", "input.append"),
                        ("value", "input.text"),
                        ("type", "Text"),
                    ],
                ),
                instruction(
                    "JSON_REMOVE",
                    &[
                        ("key", "out.document"),
                        ("source", "data.document"),
                        ("path", "input.remove"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.document")]),
            ],
            &[
                r#"input.raw={"a":[1,2,3],"b":-0.00E+4,"d":true,"d":null}"#,
                "input.replace=/a/1",
                "input.value=+0042",
                "input.append=/a/-",
                "input.text=🦀",
                "input.remove=/a/0",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            "out.document={\"a\":[42,3,\"🦀\"],\"b\":-0.00E+4,\"d\":true,\"d\":null}\n".as_bytes()
        );
        assert_eq!(
            output.stderr,
            b"set JSON data.document\nset JSON data.document\nremoved JSON out.document\n"
        );
    }

    fn json_get(key: &str, source: &str, path: &str, kind: &str) -> AilBytecodeInstruction {
        instruction(
            "JSON_GET",
            &[
                ("key", key),
                ("source", source),
                ("path", path),
                ("type", kind),
            ],
        )
    }

    #[test]
    fn native_structured_json_rejects_complete_malformed_grammar_and_types() {
        for source in [
            "",
            " ",
            "[",
            "{",
            "[1,]",
            "{\"a\":1,}",
            "{1:2}",
            "{\"a\" 2}",
            "{\"a\":}",
            "[1 2]",
            "[truex]",
            "null null",
            "NaN",
            "+1",
            ".1",
            "1.",
            "1e",
            "1e-",
            "-",
            "--1",
            "-01",
            "1E+-2",
            "\u{a0}null",
            "\"\u{1f}\"",
            r#""\ud800""#,
            r#""\udc00""#,
            r#"{"\ud800":1}"#,
            r#""\u00zz""#,
            r#""\x00""#,
        ] {
            let output = run(
                vec![json_get("out.value", "input.raw", "input.path", "Json")],
                &[&format!("input.raw={source}"), "input.path=/missing"],
            );
            assert_eq!(output.status.code(), Some(1), "{source:?}: {output:?}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"),
                "{source:?}: {output:?}"
            );
        }
        for (source, kind) in [
            ("1.0", "Int"),
            ("1e0", "Int"),
            ("9223372036854775808", "Int"),
            ("null", "Bool"),
            ("true", "Int"),
            ("null", "Text"),
            ("[1,true]", "List<Int>"),
            ("[[1]]", "List<Int>"),
            ("[null]", "List<Text>"),
            ("{}", "List<Text>"),
        ] {
            let output = run(
                vec![json_get("out.value", "input.raw", "input.path", kind)],
                &[&format!("input.raw={source}"), "input.path="],
            );
            assert_eq!(output.status.code(), Some(1), "{source}: {output:?}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"),
                "{output:?}"
            );
        }
        for source in ["null", "true", "1", "\"x\""] {
            let output = run(
                vec![instruction(
                    "JSON_COUNT",
                    &[("key", "out.count"), ("source", "input.raw")],
                )],
                &[&format!("input.raw={source}")],
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"));
        }
    }

    #[test]
    fn native_structured_json_pointer_syntax_unicode_empty_and_nul_keys() {
        let source = r#"{"": {"\u0000": "zero\u0000\n"}, "~1": 7, "é": 1, "é": 2, "array": [0]}"#;
        for (path, kind, expected) in [
            ("//\0", "Text", "zero\0\n"),
            ("/~01", "Int", "7"),
            ("/é", "Int", "1"),
            ("/é", "Int", "2"),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "data.document"), ("value", source)]),
                    instruction("SET_FIELD", &[("key", "data.path"), ("value", path)]),
                    json_get("out.value", "data.document", "data.path", kind),
                    instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                ],
                &[],
            );
            assert!(output.status.success(), "{path:?}: {output:?}");
            assert_eq!(output.stdout, format!("out.value={expected}\n").as_bytes());
        }
        for path in [
            "#",
            "a",
            "/~",
            "/~2",
            "/missing/~3",
            "/array/",
            "/array/-",
            "/array/-1",
            "/array/+0",
            "/array/00",
            "/array/1",
            "/array/9223372036854775808",
            "/array/9999999999999999999999999999999999999",
            "/array/0/x",
        ] {
            let output = run(
                vec![json_get("out.value", "input.raw", "input.path", "Json")],
                &[
                    &format!("input.raw={source}"),
                    &format!("input.path={path}"),
                ],
            );
            assert_eq!(output.status.code(), Some(1), "{path}: {output:?}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-PATH:"),
                "{output:?}"
            );
        }
    }

    #[test]
    fn native_json_inspection_classifies_all_kinds_without_numeric_conversion() {
        for (source, expected) in [
            ("null", "null"),
            ("false", "boolean"),
            ("true", "boolean"),
            ("-1.234500e+999999999999999999999999999", "number"),
            ("\"a\\u0000b\"", "string"),
            ("[{},0,null]", "array"),
            ("{\"a\":0,\"a\":1}", "object"),
        ] {
            let output = run(
                vec![
                    instruction("JSON_KIND", &[("key", "out.kind"), ("source", "input.doc")]),
                    instruction("OUTPUT_FIELD", &[("key", "out.kind")]),
                ],
                &[&format!("input.doc={source}")],
            );
            assert!(output.status.success(), "{source}: {output:?}");
            assert_eq!(output.stdout, format!("out.kind={expected}\n").as_bytes());
            assert_eq!(output.stderr, b"got JSON kind out.kind\n");
        }
    }

    #[test]
    fn native_json_inspection_keys_preserve_decoded_duplicates_order_and_aliases() {
        let output = run(
            vec![
                instruction(
                    "JSON_KEYS",
                    &[("key", "input.doc"), ("source", "input.doc")],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.saved"), ("source", "input.doc")],
                ),
                instruction(
                    "JSON_KIND",
                    &[("key", "input.doc"), ("source", "input.doc")],
                ),
                instruction(
                    "JSON_KEYS",
                    &[("key", "out.empty"), ("source", "input.empty")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "input.doc")]),
                instruction("OUTPUT_FIELD", &[("key", "out.saved")]),
                instruction("OUTPUT_FIELD", &[("key", "out.empty")]),
            ],
            &[
                r#"input.doc={"a":0,"\u0061":1,"":null,"\u0000":[],"\n":{},"é":true,"e\u0301":1e999,"\ud83d\ude00":false,"/~":0}"#,
                "input.empty={}",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, "input.doc=array\nout.empty=[]\nout.saved=[\"a\",\"a\",\"\",\"\\u0000\",\"\\n\",\"é\",\"é\",\"😀\",\"/~\"]\n".as_bytes());
    }

    #[test]
    fn native_json_inspection_validates_the_whole_source_before_root_type() {
        let expanded = format!("\"{}\"", "\u{7f}".repeat(200_000));
        for (source, family) in [
            ("{\"unselected\":[1,broken]}".to_string(), "JSON"),
            ("{} trailing".to_string(), "JSON"),
            (format!("{expanded} trailing"), "JSON"),
            (expanded, "JSON-LIMIT"),
            (
                format!("{}0{}", "[".repeat(65), "]".repeat(65)),
                "JSON-LIMIT",
            ),
            (format!("{}null", " ".repeat(1_048_573)), "JSON-LIMIT"),
            (
                format!("{{{}}}", vec!["\"\":0"; 65_536].join(",")),
                "JSON-LIMIT",
            ),
        ] {
            for opcode in ["JSON_KIND", "JSON_KEYS"] {
                let output = run(
                    vec![
                        instruction("SET_FIELD", &[("key", "input.doc"), ("value", &source)]),
                        instruction(opcode, &[("key", "out.result"), ("source", "input.doc")]),
                    ],
                    &[],
                );
                assert!(!output.status.success(), "{opcode}/{}", source.len());
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .contains(&format!("AIL-RUNTIME-{family}:")),
                    "{opcode}: {output:?}"
                );
            }
        }
        for source in ["null", "true", "1e99999", "\"text\"", "[]"] {
            let output = run(
                vec![instruction(
                    "JSON_KEYS",
                    &[("key", "out.keys"), ("source", "input.doc")],
                )],
                &[&format!("input.doc={source}")],
            );
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"));
        }
        use std::os::unix::ffi::OsStringExt;
        for opcode in ["JSON_KIND", "JSON_KEYS"] {
            let code = vec![instruction(
                opcode,
                &[("key", "out.result"), ("source", "input.doc")],
            )];
            for args in [
                vec![],
                vec![std::ffi::OsString::from_vec(
                    b"input.doc={\"\xff\":0}".to_vec(),
                )],
            ] {
                let output = run_os(code.clone(), &args);
                assert!(!output.status.success());
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"));
            }
        }
    }

    #[test]
    fn native_json_inspection_accepts_maximal_member_and_source_bounds() {
        let members = format!("{{{}}}", vec!["\"\":0"; 65_535].join(","));
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "input.doc"), ("value", &members)]),
                instruction(
                    "JSON_KEYS",
                    &[("key", "input.doc"), ("source", "input.doc")],
                ),
                instruction(
                    "LIST_LENGTH",
                    &[
                        ("key", "out.count"),
                        ("source", "input.doc"),
                        ("type", "Text"),
                    ],
                ),
                instruction(
                    "LIST_GET",
                    &[
                        ("key", "out.last"),
                        ("source", "input.doc"),
                        ("type", "Text"),
                        ("index", "i:-1"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.count")]),
                instruction("OUTPUT_FIELD", &[("key", "out.last")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.count=65535\nout.last=\n");
        let input = format!("{{\"{}\":0}}", "x".repeat(1_048_570));
        assert_eq!(input.len(), 1_048_576);
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "input.doc"), ("value", &input)]),
                instruction("JSON_KEYS", &[("key", "out.keys"), ("source", "input.doc")]),
                instruction(
                    "TEXT_LENGTH",
                    &[("key", "out.length"), ("source", "out.keys")],
                ),
                instruction("JSON_KIND", &[("key", "out.kind"), ("source", "input.doc")]),
                instruction("OUTPUT_FIELD", &[("key", "out.kind")]),
                instruction("OUTPUT_FIELD", &[("key", "out.length")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.kind=object\nout.length=1048574\n");
        let nested = format!("{}0{}", "[".repeat(64), "]".repeat(64));
        let output = run(
            vec![
                instruction("JSON_KIND", &[("key", "out.kind"), ("source", "input.doc")]),
                instruction("OUTPUT_FIELD", &[("key", "out.kind")]),
            ],
            &[&format!("input.doc={nested}")],
        );
        assert!(output.status.success());
        assert_eq!(output.stdout, b"out.kind=array\n");
    }

    #[test]
    fn native_json_inspection_body_only_helpers_return_owned_values_without_numeric_runtime() {
        let functions = BTreeMap::from([
            value_function(
                "keys",
                "doc:Json",
                "result:List<Text>",
                "List<Text>",
                vec![
                    instruction(
                        "JSON_KEYS",
                        &[("key", "local.result"), ("source", "local.doc")],
                    ),
                    return_value("List<Text>", "local.result"),
                ],
            ),
            value_function(
                "kind",
                "doc:Json",
                "result:Text",
                "Text",
                vec![
                    instruction(
                        "JSON_KIND",
                        &[("key", "local.result"), ("source", "local.doc")],
                    ),
                    return_value("Text", "local.result"),
                ],
            ),
        ]);
        let instructions = vec![
            call_value("keys", "doc=f:input.doc", "out.keys"),
            call_value("kind", "doc=f:input.doc", "out.kind"),
            call_value("keys", "doc=f:input.other", "data.reused"),
            instruction("OUTPUT_FIELD", &[("key", "out.keys")]),
            instruction("OUTPUT_FIELD", &[("key", "out.kind")]),
            instruction("RETURN_SUCCESS", &[]),
        ];
        let bytes = emit_with_functions(
            &AilBytecodeAction {
                name: "Inspect".into(),
                instructions: instructions.clone(),
            },
            &BTreeMap::new(),
            &functions,
            &BTreeMap::new(),
        )
        .unwrap();
        // No numeric PT_LOAD segments are necessary merely to inspect a Json
        // document with lossless numbers outside any binary64/i64 range.
        assert_eq!(u16::from_le_bytes([bytes[56], bytes[57]]), 1);
        let output = run_functions(
            instructions,
            functions,
            &[
                "input.doc={\"zero\":-0,\"big\":1e999999999999999999999}",
                "input.other={\"overwrite\":0}",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"out.keys=[\"zero\",\"big\"]\nout.kind=object\n"
        );
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn native_json_inspection_charges_exact_owned_results_and_additional_function_returns() {
        for (opcode, source, result, kind, charge, return_error) in [
            ("JSON_KIND", "null", "null", "Text", 4, "TEXT-LIMIT"),
            ("JSON_KEYS", "{}", "[]", "List<Text>", 2, "LIST-LIMIT"),
        ] {
            let functions = BTreeMap::from([value_function(
                "inspect",
                "doc:Json",
                &format!("value:{kind}"),
                kind,
                vec![
                    instruction(opcode, &[("key", "local.value"), ("source", "local.doc")]),
                    return_value(kind, "local.value"),
                ],
            )]);
            for (remaining, function, success) in [
                (charge, false, true),
                (charge - 1, false, false),
                (charge, true, false),
                (charge * 2, true, true),
            ] {
                let mut root = vec![
                    instruction("SET_FIELD", &[("key", "input.doc"), ("value", source)]),
                    instruction("SET_FIELD", &[("key", "input.empty"), ("value", "")]),
                ];
                let fill = if kind == "Text" {
                    let full = "x".repeat(1_048_576);
                    let tail = "x".repeat(1_048_576 - remaining);
                    root.push(instruction(
                        "SET_FIELD",
                        &[("key", "input.full"), ("value", &full)],
                    ));
                    root.push(instruction(
                        "SET_FIELD",
                        &[("key", "input.tail"), ("value", &tail)],
                    ));
                    instruction(
                        "TEXT_JOIN",
                        &[
                            ("key", "data.used"),
                            ("left", "input.full"),
                            ("right", "input.empty"),
                        ],
                    )
                } else {
                    let full = format!("[\"{}\"]", "x".repeat(1_048_572));
                    let tail = format!("[\"{}\"]", "x".repeat(1_048_572 - remaining));
                    root.push(instruction(
                        "SET_FIELD",
                        &[("key", "input.full"), ("value", &full)],
                    ));
                    root.push(instruction(
                        "SET_FIELD",
                        &[("key", "input.tail"), ("value", &tail)],
                    ));
                    instruction(
                        "JSON_PARSE",
                        &[
                            ("key", "data.used"),
                            ("source", "input.full"),
                            ("type", "List<Text>"),
                        ],
                    )
                };
                root.extend((0..63).map(|_| fill.clone()));
                let mut tail = fill;
                tail.operands.insert(
                    if kind == "Text" { "left" } else { "source" }.into(),
                    "input.tail".into(),
                );
                root.push(tail);
                // Exercise the other arena while this one is nearly exhausted.
                root.push(instruction(
                    "SET_FIELD",
                    &[
                        ("key", "input.other"),
                        ("value", if kind == "Text" { "{}" } else { "null" }),
                    ],
                ));
                root.push(instruction(
                    if kind == "Text" {
                        "JSON_KEYS"
                    } else {
                        "JSON_KIND"
                    },
                    &[("key", "data.other"), ("source", "input.other")],
                ));
                root.push(if function {
                    call_value("inspect", "doc=f:input.doc", "out.result")
                } else {
                    instruction(opcode, &[("key", "out.result"), ("source", "input.doc")])
                });
                root.push(instruction("OUTPUT_FIELD", &[("key", "out.result")]));
                root.push(instruction("RETURN_SUCCESS", &[]));
                let output = run_functions(root, functions.clone(), &[]);
                assert_eq!(
                    output.status.success(),
                    success,
                    "{opcode}/{remaining}/{function}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                if success {
                    assert_eq!(output.stdout, format!("out.result={result}\n").as_bytes());
                } else {
                    assert!(output.stdout.is_empty());
                    let family = if function { return_error } else { "JSON-LIMIT" };
                    assert!(
                        String::from_utf8_lossy(&output.stderr)
                            .contains(&format!("AIL-RUNTIME-{family}:"))
                    );
                }
            }
        }
    }

    #[test]
    fn native_structured_json_depth_and_total_values_include_root_not_keys() {
        for (source, count, success) in [
            (format!("{}0{}", "[".repeat(64), "]".repeat(64)), "1", true),
            (format!("{}0{}", "[".repeat(65), "]".repeat(65)), "", false),
            (format!("[{}]", vec!["0"; 65_535].join(",")), "65535", true),
            (format!("[{}]", vec!["0"; 65_536].join(",")), "", false),
            (
                format!("{{{}}}", vec!["\"\":0"; 65_535].join(",")),
                "65535",
                true,
            ),
            (
                format!("{{{}}}", vec!["\"\":0"; 65_536].join(",")),
                "",
                false,
            ),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "data.document"), ("value", &source)]),
                    instruction(
                        "JSON_COUNT",
                        &[("key", "out.count"), ("source", "data.document")],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.count")]),
                ],
                &[],
            );
            assert_eq!(
                output.status.success(),
                success,
                "size {}: {:?}",
                source.len(),
                output.status
            );
            if success {
                assert_eq!(output.stdout, format!("out.count={count}\n").as_bytes());
            } else {
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT:")
                );
            }
        }
    }

    #[test]
    fn native_structured_json_raw_canonical_and_path_byte_limits_precede_lookup() {
        let expanded = format!("[\"{}\"]", "\u{7f}".repeat(200_000));
        for (source, path, expected) in [
            (
                format!("[\"{}\"]", "a".repeat(1_048_572)),
                "".to_string(),
                None,
            ),
            (
                format!("[\"{}\"]", "a".repeat(1_048_573)),
                "".to_string(),
                Some("AIL-RUNTIME-JSON-LIMIT:"),
            ),
            (
                expanded.clone(),
                "/missing/~3".to_string(),
                Some("AIL-RUNTIME-JSON-LIMIT:"),
            ),
            (
                format!("{expanded}garbage"),
                "/~3".to_string(),
                Some("AIL-RUNTIME-JSON:"),
            ),
            (
                "[]".to_string(),
                format!("/{}", "a".repeat(1_048_575)),
                Some("AIL-RUNTIME-JSON-PATH:"),
            ),
            (
                "[]".to_string(),
                format!("/{}", "a".repeat(1_048_576)),
                Some("AIL-RUNTIME-JSON-LIMIT:"),
            ),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "data.document"), ("value", &source)]),
                    instruction("SET_FIELD", &[("key", "data.path"), ("value", &path)]),
                    json_get("data.document", "data.document", "data.path", "Json"),
                    instruction(
                        "JSON_COUNT",
                        &[("key", "out.count"), ("source", "data.document")],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.count")]),
                ],
                &[],
            );
            if let Some(error) = expected {
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains(error),
                    "{} {}: {:?}",
                    source.len(),
                    path.len(),
                    output.status
                );
            } else {
                assert!(output.status.success(), "{:?}", output.status);
                assert_eq!(output.stdout, b"out.count=1\n");
            }
        }
    }

    #[test]
    fn native_structured_json_invalid_utf8_missing_fields_and_source_precedence() {
        use std::os::unix::ffi::OsStringExt;
        for args in [
            vec![std::ffi::OsString::from("input.path=")],
            vec![std::ffi::OsString::from("input.raw={}")],
            vec![
                std::ffi::OsString::from_vec(b"input.raw=\"\xff\"".to_vec()),
                "input.path=/~3".into(),
            ],
            vec![
                "input.raw={}".into(),
                std::ffi::OsString::from_vec(b"input.path=/\xff".to_vec()),
            ],
            vec!["input.raw=bad".into(), "input.path=/~3".into()],
        ] {
            let output = run_os(
                vec![json_get("out.value", "input.raw", "input.path", "Json")],
                &args,
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"),
                "{output:?}"
            );
        }
    }

    #[test]
    fn native_structured_json_aliases_and_scratch_lifetimes_preserve_values() {
        let output = run(
            vec![
                instruction(
                    "JSON_PARSE",
                    &[
                        ("key", "data.document"),
                        ("source", "input.raw"),
                        ("type", "Json"),
                    ],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.original"), ("source", "data.document")],
                ),
                json_get("data.document", "data.document", "input.path", "Json"),
                json_get("input.path", "out.original", "input.path", "Text"),
                instruction(
                    "JSON_FORMAT",
                    &[
                        ("key", "out.formatted"),
                        ("source", "data.document"),
                        ("type", "Json"),
                    ],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.text"), ("source", "input.path")],
                ),
                instruction("SET_FIELD", &[("key", "data.document"), ("value", "null")]),
                instruction("OUTPUT_FIELD", &[("key", "out.original")]),
                instruction("OUTPUT_FIELD", &[("key", "out.formatted")]),
                instruction("OUTPUT_FIELD", &[("key", "out.text")]),
            ],
            &[r#"input.raw={"key":"🦀\u0000\n"}"#, "input.path=/key"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, "out.formatted=\"🦀\\u0000\\n\"\nout.original={\"key\":\"🦀\\u0000\\n\"}\nout.text=🦀\0\n\n".as_bytes());
    }

    #[test]
    fn native_structured_json_final_result_budget_is_shared_and_no_intermediate_is_charged() {
        let large = format!("[\"{}\"]", "a".repeat(1_048_572));
        for (opcode, other, exhausted_type) in [
            ("JSON_PARSE", "JSON_FORMAT", "Json"),
            ("JSON_FORMAT", "JSON_PARSE", "Text"),
        ] {
            let mut instructions = vec![
                instruction("SET_FIELD", &[("key", "data.large"), ("value", &large)]),
                instruction(
                    "SET_FIELD",
                    &[
                        ("key", "data.small"),
                        ("value", r#"{"i":42,"b":true,"t":"x","a":[]}"#),
                    ],
                ),
                instruction("SET_FIELD", &[("key", "data.path_i"), ("value", "/i")]),
                instruction("SET_FIELD", &[("key", "data.path_b"), ("value", "/b")]),
                instruction("SET_FIELD", &[("key", "data.path_t"), ("value", "/t")]),
                instruction("SET_FIELD", &[("key", "data.path_a"), ("value", "/a")]),
                eval("loop.count", "0"),
                instruction("LABEL", &[("name", "again")]),
                instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "2048")]),
                instruction(
                    opcode,
                    &[
                        ("key", "data.result"),
                        ("source", "data.large"),
                        ("type", "Json"),
                    ],
                ),
                instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "1")]),
                eval("loop.count", "loop.count + 1"),
                branch_int("loop.count", "<", "64", "again"),
                instruction(
                    other,
                    &[
                        ("key", "data.other"),
                        ("source", "data.large"),
                        ("type", "Json"),
                    ],
                ),
                json_get("out.i", "data.small", "data.path_i", "Int"),
                json_get("out.b", "data.small", "data.path_b", "Bool"),
                instruction(
                    "JSON_COUNT",
                    &[("key", "out.count"), ("source", "data.small")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.i")]),
                instruction("OUTPUT_FIELD", &[("key", "out.b")]),
                instruction("OUTPUT_FIELD", &[("key", "out.count")]),
            ];
            let output = run(instructions.clone(), &[]);
            assert!(output.status.success(), "{opcode}: {:?}", output.status);
            assert_eq!(output.stdout, b"out.b=true\nout.count=4\nout.i=42\n");
            instructions.push(json_get(
                "data.fail",
                "data.small",
                "data.path_t",
                exhausted_type,
            ));
            let output = run(instructions, &[]);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT:"));
        }
    }

    #[test]
    fn native_structured_json_preserves_numbers_order_and_duplicate_pairs() {
        let output = run(
            vec![
                instruction(
                    "JSON_PARSE",
                    &[
                        ("key", "data.document"),
                        ("source", "input.raw"),
                        ("type", "Json"),
                    ],
                ),
                instruction(
                    "JSON_COUNT",
                    &[("key", "out.count"), ("source", "data.document")],
                ),
                instruction(
                    "JSON_FORMAT",
                    &[
                        ("key", "out.json"),
                        ("source", "data.document"),
                        ("type", "Json"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.count")]),
                instruction("OUTPUT_FIELD", &[("key", "out.json")]),
            ],
            &[
                r#"input.raw= { "a": [null, true, -0, 1.250E+99, 9999999999999999999999999], "a": {"é":"\ud83e\udd80"} } "#,
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "out.count=2\nout.json={\"a\":[null,true,-0,1.250E+99,9999999999999999999999999],\"a\":{\"é\":\"🦀\"}}\n"
        );
    }

    #[test]
    fn native_structured_json_get_decodes_pointers_and_all_destination_types() {
        for (path, kind, expected) in [
            ("/a~1b/~0/0", "Int", "-42"),
            ("/a~1b/~0/1", "Bool", "true"),
            ("/a~1b/~0/2", "Text", "🦀"),
            ("/a~1b/~0/3", "List<Int>", "[1,0]"),
            ("/a~1b/~0/4", "List<Text>", "[\"é\",\"\\u007f\"]"),
            ("/a~1b/~0/5", "Json", "{\"\":null}"),
            ("/a~1b/~0/5/", "Json", "null"),
        ] {
            let output = run(
                vec![
                    instruction(
                        "JSON_GET",
                        &[
                            ("key", "out.value"),
                            ("source", "input.raw"),
                            ("path", "input.path"),
                            ("type", kind),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                ],
                &[
                    r#"input.raw={"a/b":{"~":[-42,true,"\ud83e\udd80",[1,-0],["é","\u007f"],{"":null}]}}"#,
                    &format!("input.path={path}"),
                ],
            );
            assert!(output.status.success(), "{path}: {output:?}");
            assert_eq!(output.stdout, format!("out.value={expected}\n").as_bytes());
        }
    }

    #[test]
    fn native_structured_json_rejects_duplicate_paths_and_malformed_numbers() {
        for (raw, path, error) in [
            (r#"{"a":1,"\u0061":2}"#, "/a", "AIL-RUNTIME-JSON-PATH:"),
            ("[1]", "/01", "AIL-RUNTIME-JSON-PATH:"),
            ("[1]", "/missing/~2", "AIL-RUNTIME-JSON-PATH:"),
            ("[1.e2]", "/missing", "AIL-RUNTIME-JSON:"),
            ("[1e+]", "", "AIL-RUNTIME-JSON:"),
            ("[01]", "", "AIL-RUNTIME-JSON:"),
        ] {
            let output = run(
                vec![instruction(
                    "JSON_GET",
                    &[
                        ("key", "out.value"),
                        ("source", "input.raw"),
                        ("path", "input.path"),
                        ("type", "Json"),
                    ],
                )],
                &[&format!("input.raw={raw}"), &format!("input.path={path}")],
            );
            assert!(!output.status.success(), "{raw}: {output:?}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(error),
                "{output:?}"
            );
        }
    }

    #[test]
    fn native_json_parses_and_formats_all_supported_types_canonically() {
        for (kind, source, decoded, encoded) in [
            ("Int", " \n-0\t ", "0", "0"),
            (
                "Int",
                "-9223372036854775808",
                "-9223372036854775808",
                "-9223372036854775808",
            ),
            ("Bool", " \rfalse\n", "false", "false"),
            ("Bool", "true", "true", "true"),
            (
                "Text",
                r#""\ud83e\udd80\u0000\n\u0085""#,
                "🦀\0\n\u{85}",
                "\"🦀\\u0000\\n\\u0085\"",
            ),
            (
                "List<Int>",
                " [ -0, -9223372036854775808, 2 ] ",
                "[0,-9223372036854775808,2]",
                "[0,-9223372036854775808,2]",
            ),
            (
                "List<Text>",
                r#"["\ud83e\udd80", "\u007f\b\f"]"#,
                "[\"🦀\",\"\\u007f\\u0008\\u000c\"]",
                "[\"🦀\",\"\\u007f\\u0008\\u000c\"]",
            ),
        ] {
            let output = run(
                vec![
                    instruction(
                        "JSON_PARSE",
                        &[
                            ("key", "data.value"),
                            ("source", "input.raw"),
                            ("type", kind),
                        ],
                    ),
                    instruction(
                        "COPY_FIELD",
                        &[("key", "out.decoded"), ("source", "data.value")],
                    ),
                    instruction(
                        "JSON_FORMAT",
                        &[
                            ("key", "out.encoded"),
                            ("source", "data.value"),
                            ("type", kind),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.decoded")]),
                    instruction("OUTPUT_FIELD", &[("key", "out.encoded")]),
                ],
                &[&format!("input.raw={source}")],
            );
            assert!(output.status.success(), "{kind}: {output:?}");
            assert_eq!(
                output.stdout,
                format!("out.decoded={decoded}\nout.encoded={encoded}\n").as_bytes(),
                "{kind}"
            );
            assert_eq!(
                output.stderr,
                b"parsed JSON data.value\nwrite out.decoded\nformatted JSON out.encoded\n"
            );
        }
    }

    #[test]
    fn native_json_strict_parsing_and_conversion_errors_use_json_categories() {
        for (kind, malformed) in [
            (
                "Int",
                vec![
                    "",
                    " ",
                    "+1",
                    "01",
                    "-01",
                    "1.0",
                    "1e0",
                    "9223372036854775808",
                    "-9223372036854775809",
                    "1null",
                    "[1]",
                    "null",
                    "١",
                ],
            ),
            (
                "Bool",
                vec![
                    "",
                    "TRUE",
                    "False",
                    "0",
                    "null",
                    "truefalse",
                    "true x",
                    "[false]",
                ],
            ),
            (
                "Text",
                vec![
                    "",
                    "null",
                    "false",
                    "[]",
                    "\"\\ud800\"",
                    "\"\\udc00\"",
                    "\"\\ud800\\u0041\"",
                    "\"\\x\"",
                    "\"raw\n\"",
                    "\"good\"false",
                ],
            ),
            (
                "List<Int>",
                vec![
                    "{}",
                    "[true]",
                    "[1,\"2\"]",
                    "[1,[]]",
                    "[1,]",
                    "[+1]",
                    "[01]",
                    "[1]false",
                ],
            ),
            (
                "List<Text>",
                vec![
                    "null",
                    "[1]",
                    "[\"ok\",null]",
                    "[\"ok\",\"\\ud800\"]",
                    "[\"ok\"]true",
                ],
            ),
        ] {
            for source in malformed {
                let output = run(
                    vec![instruction(
                        "JSON_PARSE",
                        &[
                            ("key", "out.value"),
                            ("source", "input.raw"),
                            ("type", kind),
                        ],
                    )],
                    &[&format!("input.raw={source}")],
                );
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{kind}/{source:?}: {output:?}"
                );
                assert!(output.stdout.is_empty());
                assert_eq!(
                    output.stderr,
                    b"AIL-RUNTIME-JSON: missing or malformed JSON conversion value\n"
                );
            }
            for opcode in ["JSON_PARSE", "JSON_FORMAT"] {
                let output = run(
                    vec![instruction(
                        opcode,
                        &[
                            ("key", "out.value"),
                            ("source", "input.missing"),
                            ("type", kind),
                        ],
                    )],
                    &[],
                );
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).starts_with("AIL-RUNTIME-JSON:"));
            }
        }
    }

    #[test]
    fn native_json_format_integer_accepts_transport_signs_and_bool_is_exact() {
        for (source, expected) in [
            ("+001", "1"),
            ("-000", "0"),
            ("+0009223372036854775807", "9223372036854775807"),
            ("-009223372036854775808", "-9223372036854775808"),
        ] {
            let output = run(
                vec![
                    instruction(
                        "JSON_FORMAT",
                        &[
                            ("key", "out.json"),
                            ("source", "input.number"),
                            ("type", "Int"),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.json")]),
                ],
                &[&format!("input.number={source}")],
            );
            assert!(output.status.success(), "{source}: {output:?}");
            assert_eq!(output.stdout, format!("out.json={expected}\n").as_bytes());
        }
        for (kind, values) in [
            ("Int", vec![" 1", "1 ", "1.0", "+", "9223372036854775808"]),
            ("Bool", vec![" true", "true ", "TRUE", "1", "false\n"]),
        ] {
            for source in values {
                let output = run(
                    vec![instruction(
                        "JSON_FORMAT",
                        &[
                            ("key", "out.json"),
                            ("source", "input.value"),
                            ("type", kind),
                        ],
                    )],
                    &[&format!("input.value={source}")],
                );
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{kind}/{source:?}: {output:?}"
                );
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).starts_with("AIL-RUNTIME-JSON:"));
            }
        }
    }

    #[test]
    fn native_json_rejects_invalid_utf8_for_both_modes_and_all_types() {
        use std::os::unix::ffi::OsStringExt;
        for opcode in ["JSON_PARSE", "JSON_FORMAT"] {
            for kind in ["Int", "Bool", "Text", "List<Int>", "List<Text>"] {
                let output = run_os(
                    vec![instruction(
                        opcode,
                        &[
                            ("key", "out.value"),
                            ("source", "input.value"),
                            ("type", kind),
                        ],
                    )],
                    &[std::ffi::OsString::from_vec(
                        b"input.value=\xed\xa0\x80".to_vec(),
                    )],
                );
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).starts_with("AIL-RUNTIME-JSON:"));
            }
        }
    }

    #[test]
    fn native_json_text_is_immutable_and_supports_in_place_parse_and_format() {
        let output = run(
            vec![
                instruction(
                    "JSON_PARSE",
                    &[
                        ("key", "data.text"),
                        ("source", "data.text"),
                        ("type", "Text"),
                    ],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.saved"), ("source", "data.text")],
                ),
                instruction(
                    "JSON_FORMAT",
                    &[
                        ("key", "data.text"),
                        ("source", "data.text"),
                        ("type", "Text"),
                    ],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.encoded"), ("source", "data.text")],
                ),
                instruction(
                    "JSON_PARSE",
                    &[
                        ("key", "data.text"),
                        ("source", "data.text"),
                        ("type", "Text"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.saved")]),
                instruction("OUTPUT_FIELD", &[("key", "out.encoded")]),
                instruction("OUTPUT_FIELD", &[("key", "data.text")]),
            ],
            &[r#"data.text="é\u0000\n\"\\\u009f""#],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, "data.text=é\0\n\"\\\u{9f}\nout.encoded=\"é\\u0000\\n\\\"\\\\\\u009f\"\nout.saved=é\0\n\"\\\u{9f}\n".as_bytes());
    }

    #[test]
    fn native_json_string_format_uses_the_full_one_mib_output_limit() {
        for (size, success) in [(1_048_574, true), (1_048_575, false)] {
            let value = "a".repeat(size);
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "data.text"), ("value", &value)]),
                    instruction(
                        "JSON_FORMAT",
                        &[
                            ("key", "data.json"),
                            ("source", "data.text"),
                            ("type", "Text"),
                        ],
                    ),
                    instruction(
                        "JSON_PARSE",
                        &[
                            ("key", "data.text"),
                            ("source", "data.json"),
                            ("type", "Text"),
                        ],
                    ),
                    instruction(
                        "TEXT_LENGTH",
                        &[("key", "out.length"), ("source", "data.json")],
                    ),
                    instruction(
                        "TEXT_LENGTH",
                        &[("key", "out.decoded"), ("source", "data.text")],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.length")]),
                    instruction("OUTPUT_FIELD", &[("key", "out.decoded")]),
                ],
                &[],
            );
            assert_eq!(
                output.status.success(),
                success,
                "size {size}: {:?}",
                output.status
            );
            if success {
                assert_eq!(output.stdout, b"out.decoded=1048574\nout.length=1048576\n");
            } else {
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT"));
            }
        }
        let source = format!("[\"{}\"]garbage", "\u{85}".repeat(200_000));
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "data.text"), ("value", &source)]),
                instruction(
                    "JSON_PARSE",
                    &[
                        ("key", "out.list"),
                        ("source", "data.text"),
                        ("type", "List<Text>"),
                    ],
                ),
            ],
            &[],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT"));
    }

    #[test]
    fn native_json_list_conversions_charge_only_the_final_designated_arena() {
        let large = format!("[\"{}\"]", "a".repeat(1_048_572));
        for (opcode, other) in [("JSON_PARSE", "JSON_FORMAT"), ("JSON_FORMAT", "JSON_PARSE")] {
            let mut instructions = vec![
                instruction("SET_FIELD", &[("key", "data.large"), ("value", &large)]),
                instruction("SET_FIELD", &[("key", "data.empty"), ("value", "[]")]),
                eval("loop.count", "0"),
                instruction("LABEL", &[("name", "again")]),
                instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "2048")]),
                instruction(
                    opcode,
                    &[
                        ("key", "data.result"),
                        ("source", "data.large"),
                        ("type", "List<Text>"),
                    ],
                ),
                instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "1")]),
                eval("loop.count", "loop.count + 1"),
                branch_int("loop.count", "<", "64", "again"),
                instruction(
                    other,
                    &[
                        ("key", "out.other"),
                        ("source", "data.empty"),
                        ("type", "List<Int>"),
                    ],
                ),
                instruction(
                    "JSON_PARSE",
                    &[
                        ("key", "out.number"),
                        ("source", "input.number"),
                        ("type", "Int"),
                    ],
                ),
                instruction(
                    "JSON_PARSE",
                    &[
                        ("key", "out.boolean"),
                        ("source", "input.boolean"),
                        ("type", "Bool"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.other")]),
                instruction("OUTPUT_FIELD", &[("key", "out.number")]),
                instruction("OUTPUT_FIELD", &[("key", "out.boolean")]),
            ];
            let output = run(
                instructions.clone(),
                &["input.number=42", "input.boolean=true"],
            );
            assert!(output.status.success(), "{opcode}: {:?}", output.status);
            assert_eq!(
                output.stdout,
                b"out.boolean=true\nout.number=42\nout.other=[]\n"
            );
            instructions.push(instruction(
                opcode,
                &[
                    ("key", "data.result"),
                    ("source", "data.empty"),
                    ("type", "List<Int>"),
                ],
            ));
            let output = run(instructions, &["input.number=42", "input.boolean=true"]);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT"));
        }
    }

    #[test]
    fn native_json_text_parse_charges_decoded_bytes_and_empty_values_cost_one() {
        let large = "a".repeat(1_048_576);
        let almost = "a".repeat(1_048_574);
        let mut instructions = vec![
            instruction("SET_FIELD", &[("key", "data.large"), ("value", &large)]),
            instruction("SET_FIELD", &[("key", "data.almost"), ("value", &almost)]),
            instruction("SET_FIELD", &[("key", "data.empty"), ("value", "")]),
            instruction(
                "SET_FIELD",
                &[("key", "data.json"), ("value", "\"\\u0000\"")],
            ),
            instruction(
                "SET_FIELD",
                &[("key", "data.json_empty"), ("value", "\"\"")],
            ),
            eval("loop.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            instruction(
                "TEXT_JOIN",
                &[
                    ("key", "data.text"),
                    ("left", "data.large"),
                    ("right", "data.empty"),
                ],
            ),
            eval("loop.count", "loop.count + 1"),
            branch_int("loop.count", "<", "63", "again"),
            instruction(
                "TEXT_JOIN",
                &[
                    ("key", "data.text"),
                    ("left", "data.almost"),
                    ("right", "data.empty"),
                ],
            ),
            instruction(
                "JSON_PARSE",
                &[
                    ("key", "out.nul"),
                    ("source", "data.json"),
                    ("type", "Text"),
                ],
            ),
            instruction(
                "JSON_PARSE",
                &[
                    ("key", "out.empty"),
                    ("source", "data.json_empty"),
                    ("type", "Text"),
                ],
            ),
            instruction("OUTPUT_FIELD", &[("key", "out.nul")]),
            instruction("OUTPUT_FIELD", &[("key", "out.empty")]),
        ];
        let output = run(instructions.clone(), &[]);
        assert!(output.status.success(), "{:?}", output.status);
        assert_eq!(output.stdout, b"out.empty=\nout.nul=\0\n");
        instructions.push(instruction(
            "JSON_PARSE",
            &[
                ("key", "out.empty"),
                ("source", "data.json_empty"),
                ("type", "Text"),
            ],
        ));
        let output = run(instructions, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT"));
    }

    #[test]
    fn native_json_input_and_item_limits_keep_json_diagnostics() {
        let too_large = "x".repeat(1_048_577);
        let too_many = format!("[{}0]", "0,".repeat(65_536));
        for opcode in ["JSON_PARSE", "JSON_FORMAT"] {
            for (kind, value) in [
                ("Int", too_large.as_str()),
                ("Bool", too_large.as_str()),
                ("Text", too_large.as_str()),
                ("List<Int>", too_large.as_str()),
                ("List<Int>", too_many.as_str()),
            ] {
                let output = run(
                    vec![
                        instruction("SET_FIELD", &[("key", "data.input"), ("value", value)]),
                        instruction(
                            opcode,
                            &[
                                ("key", "out.result"),
                                ("source", "data.input"),
                                ("type", kind),
                            ],
                        ),
                    ],
                    &[],
                );
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{opcode}/{kind}: {:?}",
                    output.status
                );
                assert!(output.stdout.is_empty());
                let trace = String::from_utf8_lossy(&output.stderr);
                assert!(trace.contains("AIL-RUNTIME-JSON-LIMIT"));
                assert!(
                    !trace.contains("AIL-RUNTIME-TEXT")
                        && !trace.contains("AIL-RUNTIME-LIST")
                        && !trace.contains("AIL-RUNTIME-INT")
                );
            }
        }
    }

    #[test]
    fn native_files_read_and_write_exact_utf8_bytes_with_explicit_grants() {
        let directory = std::env::temp_dir().join(format!(
            "ail-native-files-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        let source = directory.join("source.txt");
        let destination = directory.join("destination.txt");
        let content = "é\0\n🦀".as_bytes();
        std::fs::write(&source, content).unwrap();
        std::fs::write(&destination, b"longer previous contents").unwrap();
        let output = run(
            vec![
                instruction(
                    "FILE_READ_TEXT",
                    &[("resource", "input"), ("key", "data.text")],
                ),
                instruction(
                    "FILE_WRITE_TEXT",
                    &[("resource", "output"), ("source", "data.text")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "data.text")]),
            ],
            &[
                &format!("--read-file=input={}", source.display()),
                &format!("--write-file=output={}", destination.display()),
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, "data.text=é\0\n🦀\n".as_bytes());
        assert_eq!(std::fs::read(&destination).unwrap(), content);
        assert_eq!(
            output.stderr,
            b"read file resource input\nwrote file resource output\n"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    fn file_test_directory() -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "ail-native-files-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        directory
    }

    #[test]
    fn native_files_grants_are_separate_from_state_and_from_the_opposite_mode() {
        let directory = file_test_directory();
        let path = directory.join("untouched");
        std::fs::write(&path, b"original").unwrap();
        for (opcode, field, argument) in [
            ("FILE_READ_TEXT", "key", format!("input={}", path.display())),
            (
                "FILE_READ_TEXT",
                "key",
                format!("reads.input={}", path.display()),
            ),
            (
                "FILE_READ_TEXT",
                "key",
                format!("--write-file=input={}", path.display()),
            ),
            (
                "FILE_WRITE_TEXT",
                "source",
                format!("--read-file=input={}", path.display()),
            ),
        ] {
            let output = run(
                vec![instruction(
                    opcode,
                    &[("resource", "input"), (field, "data.missing")],
                )],
                &[&argument],
            );
            assert_eq!(output.status.code(), Some(1), "{output:?}");
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-CAPABILITY"));
            assert!(!String::from_utf8_lossy(&output.stderr).contains(&path.display().to_string()));
            assert_eq!(std::fs::read(&path).unwrap(), b"original");
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn native_files_validate_all_grants_and_use_the_last_duplicate_without_state_output() {
        let directory = file_test_directory();
        let path = directory.join("valid=path");
        std::fs::write(&path, b"hello").unwrap();
        let output = run(
            vec![instruction(
                "FILE_READ_TEXT",
                &[("resource", "ns.input_2"), ("key", "data.text")],
            )],
            &[
                "--read-file=ns.input_2=/absent",
                &format!("--read-file=ns.input_2={}", path.display()),
                "--write-file=unused=/absent",
                "ordinary.value=kept",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"data.text=hello\nordinary.value=kept\n");
        for argument in [
            "--read-file",
            "--write-file",
            "--read-fileX=x=y",
            "--read-file=",
            "--read-file==x",
            "--write-file=input=",
            "--read-file=input",
            "--read-file=2bad=x",
            "--read-file=a..b=x",
            "--read-file=a.=x",
            "--read-file=a-b=x",
            "--read-file=é=x",
        ] {
            let output = run(vec![instruction("RETURN_SUCCESS", &[])], &[argument]);
            assert_eq!(output.status.code(), Some(1), "{argument}: {output:?}");
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-CAPABILITY"));
        }
        let too_long = format!("--read-file={}=path", "a".repeat(257));
        let output = run(vec![instruction("RETURN_SUCCESS", &[])], &[&too_long]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn native_files_validate_write_text_before_opening_or_truncating() {
        use std::os::unix::ffi::OsStringExt;
        let directory = file_test_directory();
        let path = directory.join("untouched");
        std::fs::write(&path, b"original").unwrap();
        let grant = std::ffi::OsString::from(format!("--write-file=output={}", path.display()));
        for input in [
            None,
            Some(std::ffi::OsString::from_vec(b"data.text=\xff".to_vec())),
        ] {
            let mut args = vec![grant.clone()];
            args.extend(input);
            let output = run_os(
                vec![instruction(
                    "FILE_WRITE_TEXT",
                    &[("resource", "output"), ("source", "data.text")],
                )],
                &args,
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT:"));
            assert_eq!(std::fs::read(&path).unwrap(), b"original");
        }
        let oversized = "a".repeat(1_048_577);
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "data.text"), ("value", &oversized)]),
                instruction(
                    "FILE_WRITE_TEXT",
                    &[("resource", "output"), ("source", "data.text")],
                ),
            ],
            &[grant.to_str().unwrap()],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT"));
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn native_files_read_limits_utf8_and_empty_values_have_distinct_errors() {
        let directory = file_test_directory();
        let path = directory.join("source");
        for (bytes, expected) in [
            (vec![], Ok(0)),
            (vec![b'a'; 1_048_576], Ok(1_048_576)),
            (vec![0xff], Err("AIL-RUNTIME-TEXT:")),
            (vec![0xff; 1_048_577], Err("AIL-RUNTIME-TEXT-LIMIT")),
        ] {
            std::fs::write(&path, &bytes).unwrap();
            let output = run(
                vec![
                    instruction(
                        "FILE_READ_TEXT",
                        &[("resource", "input"), ("key", "data.text")],
                    ),
                    instruction(
                        "TEXT_LENGTH",
                        &[("key", "out.count"), ("source", "data.text")],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.count")]),
                ],
                &[&format!("--read-file=input={}", path.display())],
            );
            match expected {
                Ok(count) => {
                    assert!(output.status.success(), "{output:?}");
                    assert_eq!(output.stdout, format!("out.count={count}\n").as_bytes());
                }
                Err(error) => {
                    assert_eq!(output.status.code(), Some(1));
                    assert!(output.stdout.is_empty());
                    assert!(
                        String::from_utf8_lossy(&output.stderr).contains(error),
                        "{output:?}"
                    );
                }
            }
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn native_files_reopen_in_order_and_preserve_previous_read_values() {
        let directory = file_test_directory();
        let path = directory.join("same");
        std::fs::write(&path, b"before").unwrap();
        let output = run(
            vec![
                instruction(
                    "FILE_READ_TEXT",
                    &[("resource", "shared"), ("key", "data.current")],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.saved"), ("source", "data.current")],
                ),
                instruction(
                    "FILE_WRITE_TEXT",
                    &[("resource", "shared"), ("source", "input.new")],
                ),
                instruction(
                    "FILE_READ_TEXT",
                    &[("resource", "shared"), ("key", "data.current")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "data.current")]),
                instruction("OUTPUT_FIELD", &[("key", "out.saved")]),
            ],
            &[
                "input.new=after",
                &format!("--read-file=shared={}", path.display()),
                &format!("--write-file=shared={}", path.display()),
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"data.current=after\nout.saved=before\n");
        assert_eq!(std::fs::read(&path).unwrap(), b"after");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn native_files_reject_nonregular_or_absent_paths_with_io_errors() {
        let directory = file_test_directory();
        for path in [
            directory.clone(),
            directory.join("missing-parent").join("file"),
            std::path::PathBuf::from("/dev/null"),
        ] {
            for (opcode, mode, operand) in [
                ("FILE_READ_TEXT", "read", "key"),
                ("FILE_WRITE_TEXT", "write", "source"),
            ] {
                let output = run(
                    vec![instruction(
                        opcode,
                        &[("resource", "file"), (operand, "data.text")],
                    )],
                    &[
                        "data.text=valid",
                        &format!("--{mode}-file=file={}", path.display()),
                    ],
                );
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{opcode} {path:?}: {output:?}"
                );
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-IO"));
            }
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn native_files_reads_share_text_budget_and_later_errors_preserve_completed_writes() {
        let directory = file_test_directory();
        let source = directory.join("source");
        let empty = directory.join("empty");
        let destination = directory.join("destination");
        let content = vec![b'x'; 1_048_576];
        std::fs::write(&source, &content).unwrap();
        std::fs::write(&empty, b"").unwrap();
        let output = run(
            vec![
                eval("loop.count", "0"),
                instruction("LABEL", &[("name", "again")]),
                instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "2048")]),
                instruction(
                    "FILE_READ_TEXT",
                    &[("resource", "input"), ("key", "data.text")],
                ),
                instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "1")]),
                eval("loop.count", "loop.count + 1"),
                branch_int("loop.count", "<", "64", "again"),
                instruction(
                    "FILE_WRITE_TEXT",
                    &[("resource", "output"), ("source", "data.text")],
                ),
                instruction(
                    "FILE_READ_TEXT",
                    &[("resource", "empty"), ("key", "data.text")],
                ),
            ],
            &[
                &format!("--read-file=input={}", source.display()),
                &format!("--read-file=empty={}", empty.display()),
                &format!("--write-file=output={}", destination.display()),
            ],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT"));
        assert_eq!(std::fs::read(destination).unwrap(), content);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn native_float_lists_execute_scalar_operations_and_keep_immutable_aliases() {
        let output = run(
            vec![
                instruction(
                    "LIST_LENGTH",
                    &[
                        ("key", "out.count"),
                        ("source", "data.list"),
                        ("type", "Float"),
                    ],
                ),
                instruction(
                    "LIST_GET",
                    &[
                        ("key", "out.zero"),
                        ("source", "data.list"),
                        ("type", "Float"),
                        ("index", "i:1"),
                    ],
                ),
                instruction(
                    "LIST_SET",
                    &[
                        ("key", "data.list"),
                        ("source", "data.list"),
                        ("type", "Float"),
                        ("index", "i:-1"),
                        ("value", "f:data.value d:2e0 *"),
                    ],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.saved"), ("source", "data.list")],
                ),
                instruction(
                    "LIST_APPEND",
                    &[
                        ("key", "data.list"),
                        ("source", "data.list"),
                        ("type", "Float"),
                        ("value", "d:nan"),
                    ],
                ),
                instruction("LIST_EMPTY", &[("key", "out.empty"), ("type", "Float")]),
                instruction("OUTPUT_FIELD", &[("key", "data.list")]),
                instruction("OUTPUT_FIELD", &[("key", "out.count")]),
                instruction("OUTPUT_FIELD", &[("key", "out.zero")]),
                instruction("OUTPUT_FIELD", &[("key", "out.saved")]),
                instruction("OUTPUT_FIELD", &[("key", "out.empty")]),
            ],
            &[
                "data.list= [ +001, -0, .5, 1., +inf, -inf ] ",
                "data.value=1.25",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"data.list=[1e0,-0e0,5e-1,1e0,inf,2.5e0,nan]\nout.count=6\nout.empty=[]\nout.saved=[1e0,-0e0,5e-1,1e0,inf,2.5e0]\nout.zero=-0e0\n");
    }

    fn float_list_instruction(opcode: &str, operands: &[(&str, &str)]) -> AilBytecodeInstruction {
        let mut operands = operands.to_vec();
        operands.push(("type", "Float"));
        instruction(opcode, &operands)
    }

    #[test]
    fn native_float_lists_validate_full_transport_and_normalize_ieee_values() {
        for (token, expected) in [
            ("-nan", "nan"),
            ("+nan", "nan"),
            ("nan", "nan"),
            ("+inf", "inf"),
            ("-inf", "-inf"),
            ("1e400", "inf"),
            ("-1e-400", "-0e0"),
            ("5e-324", "5e-324"),
            ("1.7976931348623157e308", "1.7976931348623157e308"),
            (
                "1.00000000000000011102230246251565404236316680908203125",
                "1e0",
            ),
        ] {
            let output = run(
                vec![
                    float_list_instruction(
                        "LIST_GET",
                        &[
                            ("key", "out.value"),
                            ("source", "input.list"),
                            ("index", "i:-1"),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                ],
                &[&format!("input.list= [ {token} ] ")],
            );
            assert!(output.status.success(), "{token}: {output:?}");
            assert_eq!(output.stdout, format!("out.value={expected}\n").as_bytes());
        }
        let code = vec![float_list_instruction(
            "LIST_GET",
            &[
                ("key", "out.value"),
                ("source", "input.list"),
                ("index", "i:0"),
            ],
        )];
        for bad in [
            "",
            "1",
            "[1,]",
            "[,1]",
            "[1,,2]",
            "[1 2]",
            "[1e +2]",
            "[[1]]",
            "[\"1\"]",
            "[true]",
            "[null]",
            "[NaN]",
            "[Infinity]",
            "[1_0]",
            "[0x1]",
            "[1] trailing",
            "[1,broken]",
            "[1,\u{a0}2]",
            "[1\u{b}]",
        ] {
            let output = run(code.clone(), &[&format!("input.list={bad}")]);
            assert!(!output.status.success(), "{bad}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST:"),
                "{bad}: {output:?}"
            );
        }
        let output = run(code.clone(), &[]);
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST:"));
        use std::os::unix::ffi::OsStringExt;
        let output = run_os(
            code,
            &[std::ffi::OsString::from_vec(
                b"input.list=[1,\xff]".to_vec(),
            )],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST:"));
    }

    #[test]
    fn native_float_lists_order_source_index_and_value_errors() {
        for (source, index, value, args, family) in [
            ("[1,bad]", "i:1 i:0 /", "d:1 d:0 /", "", "LIST"),
            ("[1]", "i:1 i:0 /", "d:1 d:0 /", "", "DIVZERO"),
            ("[1]", "i:-9223372036854775808", "d:1 d:0 /", "", "INDEX"),
            ("[]", "i:0", "d:1 d:0 /", "", "INDEX"),
            ("[1]", "i:1", "f:input.value", "", "INDEX"),
            ("[1]", "i:0", "f:input.value", "input.value=bad", "FLOAT"),
            ("[1]", "i:0", "d:1 d:-0 /", "", "DIVZERO"),
        ] {
            let output = run(
                vec![float_list_instruction(
                    "LIST_SET",
                    &[
                        ("key", "out.value"),
                        ("source", "input.list"),
                        ("index", index),
                        ("value", value),
                    ],
                )],
                &[&format!("input.list={source}"), args],
            );
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:")),
                "{source}/{index}/{value}: {output:?}"
            );
        }
        let full = format!("[{}]", vec!["0"; 65_536].join(","));
        for (value, family) in [("f:input.value", "FLOAT"), ("d:1", "LIST-LIMIT")] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "input.list"), ("value", &full)]),
                    float_list_instruction(
                        "LIST_APPEND",
                        &[
                            ("key", "out.value"),
                            ("source", "input.list"),
                            ("value", value),
                        ],
                    ),
                ],
                &[],
            );
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:"))
            );
        }
        let huge = " ".repeat(1_048_577);
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "input.list"), ("value", "[1]")]),
                instruction("SET_FIELD", &[("key", "input.value"), ("value", &huge)]),
                float_list_instruction(
                    "LIST_APPEND",
                    &[
                        ("key", "out.value"),
                        ("source", "input.list"),
                        ("value", "f:input.value"),
                    ],
                ),
            ],
            &[],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-FLOAT-LIMIT:"));
    }

    #[test]
    fn native_float_lists_json_boundaries_use_strict_finite_numbers_and_keep_aliases() {
        let output = run(
            vec![
                instruction(
                    "JSON_PARSE",
                    &[
                        ("key", "data.list"),
                        ("source", "data.raw"),
                        ("type", "List<Float>"),
                    ],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.saved"), ("source", "data.list")],
                ),
                instruction(
                    "JSON_FORMAT",
                    &[
                        ("key", "data.list"),
                        ("source", "data.list"),
                        ("type", "List<Float>"),
                    ],
                ),
                json_set(
                    "out.doc",
                    "data.doc",
                    "data.path",
                    "data.runtime",
                    "List<Float>",
                ),
                json_get("out.values", "out.doc", "data.path", "List<Float>"),
                instruction("OUTPUT_FIELD", &[("key", "data.list")]),
                instruction("OUTPUT_FIELD", &[("key", "out.saved")]),
                instruction("OUTPUT_FIELD", &[("key", "out.doc")]),
                instruction("OUTPUT_FIELD", &[("key", "out.values")]),
            ],
            &[
                "data.raw=[-0,5e-324,1e-400]",
                "data.doc={\"huge\":1e400,\"items\":null}",
                "data.path=/items",
                "data.runtime=[+001,.5,1.]",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"data.list=[-0e0,5e-324,0e0]\nout.doc={\"huge\":1e400,\"items\":[1e0,5e-1,1e0]}\nout.saved=[-0e0,5e-324,0e0]\nout.values=[1e0,5e-1,1e0]\n");
        for opcode in ["JSON_PARSE", "JSON_FORMAT"] {
            let bads: &[&str] = if opcode == "JSON_PARSE" {
                &[
                    "[+1]", "[01]", "[.5]", "[1.]", "[nan]", "[inf]", "[1e400]", "[true]", "[[1]]",
                    "[\"1\"]",
                ]
            } else {
                &["[nan]", "[-nan]", "[inf]", "[1e400]", "[1,bad]"]
            };
            for bad in bads {
                let output = run(
                    vec![instruction(
                        opcode,
                        &[
                            ("key", "out.value"),
                            ("source", "input.list"),
                            ("type", "List<Float>"),
                        ],
                    )],
                    &[&format!("input.list={bad}")],
                );
                assert!(!output.status.success(), "{opcode}/{bad}");
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"),
                    "{output:?}"
                );
            }
        }
        for (source, path, family) in [
            ("broken", "bad", "JSON"),
            ("{}", "bad", "JSON-PATH"),
            ("{}", "/a", "JSON"),
        ] {
            let output = run(
                vec![json_set(
                    "out.value",
                    "input.doc",
                    "input.path",
                    "input.list",
                    "List<Float>",
                )],
                &[
                    &format!("input.doc={source}"),
                    &format!("input.path={path}"),
                    "input.list=[nan]",
                ],
            );
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:"))
            );
        }
    }

    #[test]
    fn native_float_lists_json_format_validates_count_before_finiteness() {
        let input = format!("[nan,{}]", vec!["0"; 65_536].join(","));
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "input.list"), ("value", &input)]),
                instruction(
                    "JSON_FORMAT",
                    &[
                        ("key", "out.value"),
                        ("source", "input.list"),
                        ("type", "List<Float>"),
                    ],
                ),
            ],
            &[],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT:"));
    }

    #[test]
    fn native_float_lists_keep_flat_item_limits_and_defer_canonical_bounds() {
        for count in [65_536, 65_537] {
            let input = format!("[{}]", vec!["0"; count].join(","));
            for json in [false, true] {
                let mut code = vec![instruction(
                    "SET_FIELD",
                    &[("key", "input.list"), ("value", &input)],
                )];
                if json {
                    code.push(instruction(
                        "JSON_PARSE",
                        &[
                            ("key", "input.list"),
                            ("source", "input.list"),
                            ("type", "List<Float>"),
                        ],
                    ));
                }
                code.push(float_list_instruction(
                    "LIST_LENGTH",
                    &[("key", "out.count"), ("source", "input.list")],
                ));
                code.push(instruction("OUTPUT_FIELD", &[("key", "out.count")]));
                let output = run(code, &[]);
                assert_eq!(output.status.success(), count == 65_536, "{output:?}");
                if count == 65_536 {
                    assert_eq!(output.stdout, b"out.count=65536\n");
                } else {
                    assert!(output.stdout.is_empty());
                    let family = if json { "JSON-LIMIT" } else { "LIST-LIMIT" };
                    assert!(
                        String::from_utf8_lossy(&output.stderr)
                            .contains(&format!("AIL-RUNTIME-{family}:"))
                    );
                }
            }
        }
        let expanded = format!("[{}]", vec!["1234567890123"; 65_536].join(","));
        for producing in [false, true] {
            let mut code = vec![instruction(
                "SET_FIELD",
                &[("key", "input.list"), ("value", &expanded)],
            )];
            code.push(if producing {
                float_list_instruction(
                    "LIST_SET",
                    &[
                        ("key", "out.value"),
                        ("source", "input.list"),
                        ("index", "i:0"),
                        ("value", "d:1"),
                    ],
                )
            } else {
                float_list_instruction(
                    "LIST_GET",
                    &[
                        ("key", "out.value"),
                        ("source", "input.list"),
                        ("index", "i:-1"),
                    ],
                )
            });
            code.push(instruction("OUTPUT_FIELD", &[("key", "out.value")]));
            let output = run(code, &[]);
            assert_eq!(output.status.success(), !producing);
            if producing {
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT:")
                );
            } else {
                assert_eq!(output.stdout, b"out.value=1.234567890123e12\n");
            }
        }
    }

    #[test]
    fn native_float_lists_function_boundaries_borrow_then_canonicalize_one_result() {
        let functions = BTreeMap::from([
            value_function(
                "identity",
                "items:List<Float>",
                "",
                "List<Float>",
                vec![return_value("List<Float>", "local.items")],
            ),
            value_function(
                "size",
                "items:List<Float>",
                "result:Int",
                "Int",
                vec![
                    float_list_instruction(
                        "LIST_LENGTH",
                        &[("key", "local.result"), ("source", "local.items")],
                    ),
                    return_value("Int", "local.result"),
                ],
            ),
            value_function(
                "unused",
                "a:List<Float>;z:Int",
                "",
                "Int",
                vec![return_value("Int", "1")],
            ),
        ]);
        let output = run_functions(
            value_root("identity", "items=f:input.list"),
            functions.clone(),
            &["input.list=[+001,.5,-0,nan]"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.result=[1e0,5e-1,-0e0,nan]\n");
        assert!(output.stderr.is_empty());
        let input = format!("[{}]", vec!["1234567890123"; 65_536].join(","));
        for (target, success) in [("size", true), ("identity", false)] {
            let mut root = vec![instruction(
                "SET_FIELD",
                &[("key", "input.list"), ("value", &input)],
            )];
            root.extend(value_root(target, "items=f:input.list"));
            let output = run_functions(root, functions.clone(), &[]);
            assert_eq!(output.status.success(), success);
            if success {
                assert_eq!(output.stdout, b"out.result=65536\n");
            } else {
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT:")
                );
            }
        }
        let output = run_functions(
            value_root("unused", "a=f:input.list;z=i:1 i:0 /"),
            functions,
            &["input.list=[bad]"],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST:"));
    }

    #[test]
    fn native_float_lists_recursive_and_body_only_operations_preserve_callers() {
        let functions = BTreeMap::from([
            value_function(
                "grow",
                "items:List<Float>;n:Int",
                "next:List<Float>;result:List<Float>",
                "List<Float>",
                vec![
                    instruction(
                        "BRANCH_INT",
                        &[
                            ("left", "f:local.n"),
                            ("comparison", "<="),
                            ("right", "i:0"),
                            ("label", "done"),
                        ],
                    ),
                    float_list_instruction(
                        "LIST_APPEND",
                        &[
                            ("key", "local.next"),
                            ("source", "local.items"),
                            ("value", "d:nan"),
                        ],
                    ),
                    call_value(
                        "grow",
                        "items=f:local.next;n=f:local.n i:1 -",
                        "local.result",
                    ),
                    return_value("List<Float>", "local.result"),
                    instruction("LABEL", &[("name", "done")]),
                    return_value("List<Float>", "local.items"),
                ],
            ),
            value_function(
                "make",
                "",
                "items:List<Float>;number:Float;raw:Text",
                "Text",
                vec![
                    float_list_instruction("LIST_EMPTY", &[("key", "local.items")]),
                    float_list_instruction(
                        "LIST_APPEND",
                        &[
                            ("key", "local.items"),
                            ("source", "local.items"),
                            ("value", "d:5e-324"),
                        ],
                    ),
                    float_list_instruction(
                        "LIST_GET",
                        &[
                            ("key", "local.number"),
                            ("source", "local.items"),
                            ("index", "i:0"),
                        ],
                    ),
                    instruction(
                        "JSON_FORMAT",
                        &[
                            ("key", "local.raw"),
                            ("source", "local.items"),
                            ("type", "List<Float>"),
                        ],
                    ),
                    return_value("Text", "local.raw"),
                ],
            ),
        ]);
        let output = run_functions(
            vec![
                call_value("grow", "items=f:input.list;n=i:3", "out.result"),
                call_value("make", "", "out.text"),
                instruction("OUTPUT_FIELD", &[("key", "out.result")]),
                instruction("OUTPUT_FIELD", &[("key", "out.text")]),
                instruction("OUTPUT_FIELD", &[("key", "input.list")]),
                instruction("RETURN_SUCCESS", &[]),
            ],
            functions,
            &["input.list=[+001,-0]"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"input.list=[+001,-0]\nout.result=[1e0,-0e0,nan,nan,nan]\nout.text=[5e-324]\n"
        );
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn native_float_lists_exact_size_returns_charge_only_shared_list_arena() {
        let mut elements = vec!["1.234567891e100"; 65_536];
        elements[0] = "1.23456789e100";
        let input = format!("[{}]", elements.join(","));
        assert_eq!(input.len(), 1_048_576);
        let functions = BTreeMap::from([
            value_function(
                "identity",
                "items:List<Float>",
                "",
                "List<Float>",
                vec![return_value("List<Float>", "local.items")],
            ),
            value_function(
                "unused",
                "items:List<Float>",
                "",
                "Int",
                vec![return_value("Int", "7")],
            ),
        ]);
        for (target, calls, success) in [
            ("unused", 65, true),
            ("identity", 64, true),
            ("identity", 65, false),
        ] {
            let mut root = vec![instruction(
                "SET_FIELD",
                &[("key", "input.list"), ("value", &input)],
            )];
            root.extend(
                (0..calls).map(|_| call_value(target, "items=f:input.list", "data.result")),
            );
            // Scalar get and list length do not reserve either arena. JSON
            // formatting uses the separate text arena after list exhaustion.
            root.push(float_list_instruction(
                "LIST_GET",
                &[
                    ("key", "out.value"),
                    ("source", "input.list"),
                    ("index", "i:0"),
                ],
            ));
            root.push(float_list_instruction(
                "LIST_LENGTH",
                &[("key", "out.count"), ("source", "input.list")],
            ));
            root.push(instruction(
                "JSON_FORMAT",
                &[
                    ("key", "data.text"),
                    ("source", "input.list"),
                    ("type", "List<Float>"),
                ],
            ));
            root.push(instruction("OUTPUT_FIELD", &[("key", "out.value")]));
            root.push(instruction("OUTPUT_FIELD", &[("key", "out.count")]));
            root.push(instruction("RETURN_SUCCESS", &[]));
            let output = run_functions(root, functions.clone(), &[]);
            assert_eq!(
                output.status.success(),
                success,
                "{target}/{calls}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            if success {
                assert_eq!(
                    output.stdout,
                    b"out.count=65536\nout.value=1.23456789e100\n"
                );
            } else {
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT:")
                );
            }
        }
    }

    #[test]
    fn native_lists_execute_integer_operations_and_preserve_copies() {
        let output = run(
            vec![
                instruction(
                    "LIST_LENGTH",
                    &[
                        ("key", "out.count"),
                        ("source", "data.source"),
                        ("type", "Int"),
                    ],
                ),
                instruction(
                    "LIST_GET",
                    &[
                        ("key", "out.minimum"),
                        ("source", "data.source"),
                        ("type", "Int"),
                        ("index", "i:-2"),
                    ],
                ),
                instruction(
                    "LIST_SET",
                    &[
                        ("key", "data.changed"),
                        ("source", "data.source"),
                        ("type", "Int"),
                        ("index", "i:2"),
                        ("value", "i:2 i:5 *"),
                    ],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.saved"), ("source", "data.changed")],
                ),
                instruction(
                    "LIST_APPEND",
                    &[
                        ("key", "data.changed"),
                        ("source", "data.changed"),
                        ("type", "Int"),
                        ("value", "i:4"),
                    ],
                ),
                instruction("LIST_EMPTY", &[("key", "out.empty"), ("type", "Int")]),
                instruction("OUTPUT_FIELD", &[("key", "data.changed")]),
                instruction("OUTPUT_FIELD", &[("key", "out.count")]),
                instruction("OUTPUT_FIELD", &[("key", "out.minimum")]),
                instruction("OUTPUT_FIELD", &[("key", "out.saved")]),
                instruction("OUTPUT_FIELD", &[("key", "out.empty")]),
            ],
            &["data.source= [ -0, -9223372036854775808, 3 ] "],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"data.changed=[0,-9223372036854775808,10,4]\nout.count=3\nout.empty=[]\nout.minimum=-9223372036854775808\nout.saved=[0,-9223372036854775808,10]\n");
    }

    #[test]
    fn native_list_transformations_raw_opcodes_preserve_aliases_and_signed_order() {
        let output = run(
            vec![
                instruction(
                    "SET_FIELD",
                    &[("key", "x.left"), ("value", "[3,-9223372036854775808,2]")],
                ),
                instruction(
                    "SET_FIELD",
                    &[("key", "x.right"), ("value", "[9223372036854775807,-1,2]")],
                ),
                instruction(
                    "LIST_CONCAT",
                    &[
                        ("key", "x.right"),
                        ("source", "x.left"),
                        ("other", "x.right"),
                        ("type", "Int"),
                    ],
                ),
                instruction(
                    "LIST_SORT",
                    &[
                        ("key", "x.sorted"),
                        ("source", "x.right"),
                        ("order", "ascending"),
                        ("type", "Int"),
                    ],
                ),
                instruction(
                    "LIST_SLICE",
                    &[
                        ("key", "x.part"),
                        ("source", "x.sorted"),
                        ("start", "i:1"),
                        ("end", "i:-1"),
                        ("type", "Int"),
                    ],
                ),
                instruction(
                    "LIST_REVERSE",
                    &[("key", "x.part"), ("source", "x.part"), ("type", "Int")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "x.right")]),
                instruction("OUTPUT_FIELD", &[("key", "x.sorted")]),
                instruction("OUTPUT_FIELD", &[("key", "x.part")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"x.part=[3,2,2,-1]\nx.right=[3,-9223372036854775808,2,9223372036854775807,-1,2]\nx.sorted=[-9223372036854775808,-1,2,2,3,9223372036854775807]\n");
    }

    fn list_transform_instruction(
        opcode: &str,
        kind: &str,
        key: &str,
        source: &str,
        extra: &[(&str, &str)],
    ) -> AilBytecodeInstruction {
        let mut operands = vec![("key", key), ("source", source), ("type", kind)];
        operands.extend_from_slice(extra);
        instruction(opcode, &operands)
    }

    #[test]
    fn native_list_transformations_sort_ieee_values_stably_in_both_directions() {
        let source = "[nan,-0.0,4.0,0.0,-inf,nan,inf,-0.0,5e-324,-5e-324]";
        for (order, expected) in [
            (
                "ascending",
                "[-inf,-5e-324,-0e0,0e0,-0e0,5e-324,4e0,inf,nan,nan]",
            ),
            (
                "descending",
                "[inf,4e0,5e-324,-0e0,0e0,-0e0,-5e-324,-inf,nan,nan]",
            ),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "x.list"), ("value", source)]),
                    list_transform_instruction(
                        "LIST_SORT",
                        "Float",
                        "out.list",
                        "x.list",
                        &[("order", order)],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.list")]),
                ],
                &[],
            );
            assert!(output.status.success(), "{order}: {output:?}");
            assert_eq!(output.stdout, format!("out.list={expected}\n").as_bytes());
        }
    }

    #[test]
    fn native_list_transformations_text_descriptors_survive_aliases_and_scratch_release() {
        let output = run(
            vec![
                instruction(
                    "SET_FIELD",
                    &[
                        ("key", "x.first"),
                        ("value", r#"["é","a\u0000b","a","é", "😀", "a\u0000"]"#),
                    ],
                ),
                instruction(
                    "SET_FIELD",
                    &[("key", "x.second"), ("value", r#"["","𐀀","\u007f"]"#)],
                ),
                list_transform_instruction(
                    "LIST_CONCAT",
                    "Text",
                    "x.second",
                    "x.first",
                    &[("other", "x.second")],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.original"), ("source", "x.second")],
                ),
                list_transform_instruction(
                    "LIST_SORT",
                    "Text",
                    "x.second",
                    "x.second",
                    &[("order", "descending")],
                ),
                list_transform_instruction("LIST_REVERSE", "Text", "out.list", "x.second", &[]),
                list_transform_instruction(
                    "LIST_SLICE",
                    "Text",
                    "x.first",
                    "x.first",
                    &[("start", "i:0"), ("end", "i:0")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.list")]),
                instruction("OUTPUT_FIELD", &[("key", "out.original")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, concat!(
            "out.list=[\"\",\"a\",\"a\\u0000\",\"a\\u0000b\",\"é\",\"\\u007f\",\"é\",\"𐀀\",\"😀\"]\n",
            "out.original=[\"é\",\"a\\u0000b\",\"a\",\"é\",\"😀\",\"a\\u0000\",\"\",\"𐀀\",\"\\u007f\"]\n"
        ).as_bytes());
    }

    #[test]
    fn native_list_transformations_slice_clamps_extremes_and_reads_both_expression_fields() {
        for (start, end, expected) in [
            (i64::MIN, i64::MAX, "[11,22,33,44]"),
            (i64::MAX, i64::MIN, "[]"),
            (-3, -1, "[22,33]"),
            (-1, 100, "[44]"),
            (-100, 2, "[11,22]"),
            (2, 2, "[]"),
            (3, 1, "[]"),
        ] {
            let output = run(
                vec![
                    instruction(
                        "SET_FIELD",
                        &[("key", "x.source"), ("value", "[11,22,33,44]")],
                    ),
                    list_transform_instruction(
                        "LIST_SLICE",
                        "Int",
                        "out.list",
                        "x.source",
                        &[
                            ("start", "f:bounds.start i:0 +"),
                            ("end", "f:bounds.end i:0 +"),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.list")]),
                ],
                &[
                    &format!("bounds.start={start}"),
                    &format!("bounds.end={end}"),
                ],
            );
            assert!(output.status.success(), "{start}:{end}: {output:?}");
            assert_eq!(output.stdout, format!("out.list={expected}\n").as_bytes());
        }
    }

    #[test]
    fn native_list_transformations_validate_inputs_and_bounds_in_contract_order() {
        for (source, start, end, family) in [
            ("[1,bad]", "i:0", "i:0", "LIST"),
            ("[1,bad]", "i:1 i:0 /", "f:x.missing", "LIST"),
            ("[]", "i:1 i:0 /", "f:x.missing", "DIVZERO"),
            ("[]", "f:x.missing", "i:1 i:0 /", "INT"),
            ("[]", "i:9223372036854775807", "i:1 i:0 /", "DIVZERO"),
            ("[]", "i:0", "i:9223372036854775807 i:1 +", "OVERFLOW"),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "x.source"), ("value", source)]),
                    instruction("SET_FIELD", &[("key", "out.list"), ("value", "sentinel")]),
                    instruction("OUTPUT_FIELD", &[("key", "out.list")]),
                    list_transform_instruction(
                        "LIST_SLICE",
                        "Int",
                        "out.list",
                        "x.source",
                        &[("start", start), ("end", end)],
                    ),
                ],
                &[],
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:")),
                "{output:?}"
            );
        }
        let maximal = format!("[{}0]", "0,".repeat(65_535));
        for (second, family) in [("[0,bad]", "LIST"), ("[0]", "LIST-LIMIT")] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "x.first"), ("value", &maximal)]),
                    instruction("SET_FIELD", &[("key", "x.second"), ("value", second)]),
                    list_transform_instruction(
                        "LIST_CONCAT",
                        "Int",
                        "out.list",
                        "x.first",
                        &[("other", "x.second")],
                    ),
                ],
                &[],
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:")),
                "{output:?}"
            );
        }
        let output = run(
            vec![list_transform_instruction(
                "LIST_SORT",
                "Text",
                "out.list",
                "x.first",
                &[("order", "ascending")],
            )],
            &[r#"x.first=["\secret"]"#],
        );
        assert_eq!(output.status.code(), Some(1));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("AIL-RUNTIME-LIST:"));
        assert!(!error.contains("secret"));
    }

    #[test]
    fn native_list_transformations_reject_invalid_utf8_and_oversized_inputs() {
        use std::os::unix::ffi::OsStringExt;
        for (opcode, extra) in [
            ("LIST_CONCAT", vec![("other", "x.other")]),
            ("LIST_SLICE", vec![("start", "i:0"), ("end", "i:0")]),
            ("LIST_REVERSE", vec![]),
            ("LIST_SORT", vec![("order", "ascending")]),
        ] {
            let bad = std::ffi::OsString::from_vec(b"x.source=[\"\xff\"]".to_vec());
            let output = run_os(
                vec![
                    instruction("SET_FIELD", &[("key", "x.other"), ("value", "[]")]),
                    list_transform_instruction(opcode, "Text", "out.list", "x.source", &extra),
                ],
                &[bad],
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST:"));
        }
        let bad = std::ffi::OsString::from_vec(b"x.other=[\"\xff\"]".to_vec());
        let output = run_os(
            vec![
                instruction("SET_FIELD", &[("key", "x.source"), ("value", "[]")]),
                list_transform_instruction(
                    "LIST_CONCAT",
                    "Text",
                    "out.list",
                    "x.source",
                    &[("other", "x.other")],
                ),
            ],
            &[bad],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST:"));
        for source in [
            format!("[{}0]", "0,".repeat(65_536)),
            format!("[0]{}", " ".repeat(1_048_574)),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "x.source"), ("value", &source)]),
                    list_transform_instruction(
                        "LIST_SLICE",
                        "Int",
                        "out.list",
                        "x.source",
                        &[("start", "i:0"), ("end", "i:0")],
                    ),
                ],
                &[],
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT:"));
        }
    }

    #[test]
    fn native_list_transformations_allow_borrowed_canonical_expansion_until_result() {
        let source = format!("[\"{}\",\"small\"]", "\u{7f}".repeat(200_000));
        let prefix = vec![
            instruction("SET_FIELD", &[("key", "x.source"), ("value", &source)]),
            instruction("SET_FIELD", &[("key", "x.empty"), ("value", "[]")]),
        ];
        let mut small = prefix.clone();
        small.extend([
            list_transform_instruction(
                "LIST_SLICE",
                "Text",
                "out.list",
                "x.source",
                &[("start", "i:1"), ("end", "i:2")],
            ),
            instruction("OUTPUT_FIELD", &[("key", "out.list")]),
        ]);
        let output = run(small, &[]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.list=[\"small\"]\n");
        for (opcode, extra) in [
            ("LIST_REVERSE", vec![]),
            ("LIST_SORT", vec![("order", "ascending")]),
            ("LIST_CONCAT", vec![("other", "x.empty")]),
            ("LIST_SLICE", vec![("start", "i:0"), ("end", "i:2")]),
        ] {
            let mut code = prefix.clone();
            code.push(list_transform_instruction(
                opcode, "Text", "out.list", "x.source", &extra,
            ));
            let output = run(code, &[]);
            assert_eq!(output.status.code(), Some(1));
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT:"),
                "{opcode}: {output:?}"
            );
        }
    }

    #[test]
    fn native_list_transformations_sort_all_merge_parities_and_maximal_adversarial_inputs() {
        let _guard = EXECUTION.lock().unwrap();
        let integer_cases = [0, 1, 2, 3, 5, 17, 32, 65_536]
            .into_iter()
            .flat_map(|count| {
                let ordered: Vec<_> = (0..count).map(|i| i as i64 - 32_768).collect();
                let mut reverse = ordered.clone();
                reverse.reverse();
                let shuffled: Vec<_> = (0..count).map(|i| ordered[(i * 40_503) % count]).collect();
                let duplicate: Vec<_> = (0..count).map(|i| (i % 3) as i64 - 1).collect();
                [ordered, reverse, shuffled, duplicate]
            });
        let mut cases = Vec::new();
        for mut values in integer_cases {
            let source = format!(
                "[{}]",
                values
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            );
            values.sort();
            let expected = format!(
                "[{}]",
                values
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            );
            cases.push(("Int", source, expected));
        }
        let mut words: Vec<_> = (0..65_536)
            .map(|i| format!("prefix{:05}", 65_535 - i))
            .collect();
        let source = format!("[\"{}\"]", words.join("\",\""));
        words.sort();
        cases.push(("Text", source, format!("[\"{}\"]", words.join("\",\""))));
        for (kind, source, expected) in cases {
            let action = AilBytecodeAction {
                name: "Sort".into(),
                instructions: vec![
                    instruction("SET_FIELD", &[("key", "x.source"), ("value", &source)]),
                    list_transform_instruction(
                        "LIST_SORT",
                        kind,
                        "out.list",
                        "x.source",
                        &[("order", "ascending")],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.list")]),
                ],
            };
            let binary = emit(&action, &BTreeMap::new()).unwrap();
            // Int/Text transforms must not acquire the embedded Float runtime.
            assert_eq!(u16::from_le_bytes([binary[56], binary[57]]), 1);
            let path = std::env::temp_dir().join(format!(
                "ail-list-sort-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let output_path = path.with_extension("output");
            std::fs::write(&path, binary).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            let mut child = Command::new(&path)
                .stdout(std::fs::File::create(&output_path).unwrap())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while child.try_wait().unwrap().is_none() && std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            if child.try_wait().unwrap().is_none() {
                child.kill().unwrap();
                child.wait().unwrap();
                std::fs::remove_file(&path).unwrap();
                std::fs::remove_file(&output_path).unwrap();
                panic!(
                    "bounded descriptor mergesort exceeded five seconds: {kind}, {} bytes",
                    source.len()
                );
            }
            let output = child.wait_with_output().unwrap();
            let stdout = std::fs::read(&output_path).unwrap();
            std::fs::remove_file(&path).unwrap();
            std::fs::remove_file(&output_path).unwrap();
            assert!(output.status.success(), "{kind}: {output:?}");
            assert_eq!(stdout, format!("out.list={expected}\n").as_bytes());
        }
    }

    #[test]
    fn native_list_transformations_charge_only_each_result_and_keep_arenas_independent() {
        let large_list = format!("[\"{}\"]", "x".repeat(1_048_572));
        let large_text = "x".repeat(1_048_576);
        let mut code = vec![
            instruction("SET_FIELD", &[("key", "x.source"), ("value", &large_list)]),
            instruction("SET_FIELD", &[("key", "x.empty"), ("value", "[]")]),
            instruction("SET_FIELD", &[("key", "x.text"), ("value", &large_text)]),
            instruction("SET_FIELD", &[("key", "x.empty_text"), ("value", "")]),
            eval("x.count", "0"),
            instruction("LABEL", &[("name", "text_again")]),
            instruction(
                "TEXT_TRIM",
                &[("key", "x.text_result"), ("source", "x.text")],
            ),
            eval("x.count", "x.count + 1"),
            branch_int("x.count", "<", "64", "text_again"),
            eval("x.count", "0"),
            instruction("LABEL", &[("name", "list_again")]),
            list_transform_instruction(
                "LIST_CONCAT",
                "Text",
                "x.result",
                "x.source",
                &[("other", "x.empty")],
            ),
            list_transform_instruction(
                "LIST_SLICE",
                "Text",
                "x.result",
                "x.result",
                &[("start", "i:0"), ("end", "i:1")],
            ),
            list_transform_instruction("LIST_REVERSE", "Text", "x.result", "x.result", &[]),
            list_transform_instruction(
                "LIST_SORT",
                "Text",
                "x.result",
                "x.result",
                &[("order", "ascending")],
            ),
            eval("x.count", "x.count + 1"),
            branch_int("x.count", "<", "16", "list_again"),
        ];
        let output = run(code.clone(), &[]);
        assert!(
            output.status.success(),
            "{:?}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr[output.stderr.len().saturating_sub(300)..])
        );
        for (opcode, extra) in [
            ("LIST_CONCAT", vec![("other", "x.empty")]),
            ("LIST_SLICE", vec![("start", "i:0"), ("end", "i:0")]),
            ("LIST_REVERSE", vec![]),
            ("LIST_SORT", vec![("order", "descending")]),
        ] {
            let mut exhausted = code.clone();
            exhausted.push(list_transform_instruction(
                opcode, "Text", "x.result", "x.empty", &extra,
            ));
            let output = run(exhausted, &[]);
            assert_eq!(output.status.code(), Some(1), "{opcode}");
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT:"));
        }
        code.push(instruction(
            "TEXT_TRIM",
            &[("key", "x.result"), ("source", "x.empty_text")],
        ));
        let output = run(code, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT:"));
    }

    #[test]
    fn native_list_transformations_empty_results_cost_exactly_two_bytes() {
        let large = format!("[\"{}\"]", "x".repeat(1_048_572));
        let almost_large = format!("[\"{}\"]", "x".repeat(1_048_570));
        let mut code = vec![
            instruction("SET_FIELD", &[("key", "x.large"), ("value", &large)]),
            instruction(
                "SET_FIELD",
                &[("key", "x.almost"), ("value", &almost_large)],
            ),
            eval("x.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            list_transform_instruction("LIST_REVERSE", "Text", "x.result", "x.large", &[]),
            eval("x.count", "x.count + 1"),
            branch_int("x.count", "<", "63", "again"),
            list_transform_instruction(
                "LIST_SORT",
                "Text",
                "x.result",
                "x.almost",
                &[("order", "ascending")],
            ),
            list_transform_instruction(
                "LIST_SLICE",
                "Text",
                "x.empty",
                "x.large",
                &[("start", "i:0"), ("end", "i:0")],
            ),
        ];
        let output = run(code.clone(), &[]);
        assert!(output.status.success());
        code.push(list_transform_instruction(
            "LIST_REVERSE",
            "Text",
            "x.result",
            "x.empty",
            &[],
        ));
        let output = run(code, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT:"));
    }

    #[test]
    fn native_list_transformations_body_only_helpers_preserve_recursive_frames() {
        for (kind, source, expected) in [
            ("Int", "[3,0,0,-1]", "[3,3,3,3,3,0]"),
            ("Float", "[3,-0,0,nan]", "[3e0,3e0,3e0,3e0,3e0,0e0]"),
            ("Text", r#"["c","a","b"]"#, r#"["c","c","c","c","c","b"]"#),
        ] {
            let list_kind = format!("List<{kind}>");
            let function = value_function(
                "work",
                &format!("items:{list_kind};n:Int"),
                &format!("next:{list_kind};result:{list_kind}"),
                &list_kind,
                vec![
                    list_transform_instruction(
                        "LIST_CONCAT",
                        kind,
                        "local.next",
                        "local.items",
                        &[("other", "local.items")],
                    ),
                    list_transform_instruction(
                        "LIST_REVERSE",
                        kind,
                        "local.next",
                        "local.next",
                        &[],
                    ),
                    list_transform_instruction(
                        "LIST_SORT",
                        kind,
                        "local.next",
                        "local.next",
                        &[("order", "descending")],
                    ),
                    list_transform_instruction(
                        "LIST_SLICE",
                        kind,
                        "local.next",
                        "local.next",
                        &[("start", "i:0"), ("end", "i:3")],
                    ),
                    branch_int("local.n", "<=", "0", "done"),
                    call_value(
                        "work",
                        "items=f:local.next;n=f:local.n i:1 -",
                        "local.result",
                    ),
                    list_transform_instruction(
                        "LIST_CONCAT",
                        kind,
                        "local.result",
                        "local.result",
                        &[("other", "local.next")],
                    ),
                    return_value(&list_kind, "local.result"),
                    instruction("LABEL", &[("name", "done")]),
                    return_value(&list_kind, "local.next"),
                ],
            );
            let functions = BTreeMap::from([function]);
            let root = vec![
                instruction("SET_FIELD", &[("key", "x.list"), ("value", source)]),
                call_value("work", "items=f:x.list;n=i:1", "out.list"),
                instruction("OUTPUT_FIELD", &[("key", "out.list")]),
                instruction("OUTPUT_FIELD", &[("key", "x.list")]),
            ];
            let binary = emit_with_functions(
                &AilBytecodeAction {
                    name: "Work".into(),
                    instructions: root.clone(),
                },
                &BTreeMap::new(),
                &functions,
                &BTreeMap::new(),
            )
            .unwrap();
            assert_eq!(
                u16::from_le_bytes([binary[56], binary[57]]) > 1,
                kind == "Float"
            );
            let output = run_functions(root, functions, &[]);
            assert!(output.status.success(), "{kind}: {output:?}");
            assert_eq!(
                output.stdout,
                format!("out.list={expected}\nx.list={source}\n").as_bytes()
            );
        }
    }

    #[test]
    fn native_list_transformations_function_borrows_and_return_copies_have_exact_charges() {
        let functions = BTreeMap::from([value_function(
            "sorted",
            "items:List<Text>",
            "result:List<Text>",
            "List<Text>",
            vec![
                list_transform_instruction(
                    "LIST_SORT",
                    "Text",
                    "local.result",
                    "local.items",
                    &[("order", "ascending")],
                ),
                return_value("List<Text>", "local.result"),
            ],
        )]);
        let large = format!("[\"{}\"]", "x".repeat(1_048_572));
        let mut root = vec![
            instruction("SET_FIELD", &[("key", "x.list"), ("value", &large)]),
            instruction("SET_FIELD", &[("key", "x.empty"), ("value", "[]")]),
            eval("x.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            call_value("sorted", "items=f:x.list", "out.list"),
            eval("x.count", "x.count + 1"),
            branch_int("x.count", "<", "32", "again"),
        ];
        let output = run_functions(root.clone(), functions.clone(), &[]);
        assert!(output.status.success());
        root.push(list_transform_instruction(
            "LIST_REVERSE",
            "Text",
            "out.list",
            "x.empty",
            &[],
        ));
        let output = run_functions(root, functions, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT:"));

        let functions = BTreeMap::from([value_function(
            "shrink",
            "items:List<Text>",
            "result:List<Text>",
            "List<Text>",
            vec![
                list_transform_instruction(
                    "LIST_SLICE",
                    "Text",
                    "local.result",
                    "local.items",
                    &[("start", "i:1"), ("end", "i:2")],
                ),
                return_value("List<Text>", "local.result"),
            ],
        )]);
        let source = format!("[\"{}\",\"small\"]", "\u{7f}".repeat(200_000));
        let output = run_functions(
            vec![
                instruction("SET_FIELD", &[("key", "x.list"), ("value", &source)]),
                call_value("shrink", "items=f:x.list", "out.list"),
                instruction("OUTPUT_FIELD", &[("key", "out.list")]),
            ],
            functions,
            &[],
        );
        assert!(output.status.success());
        assert_eq!(output.stdout, b"out.list=[\"small\"]\n");
    }

    #[test]
    fn native_lists_decode_unicode_json_and_canonicalize_text_results() {
        let output = run(
            vec![
                instruction(
                    "SET_FIELD",
                    &[("key", "text.value"), ("value", "\t\\\"é\u{85}")],
                ),
                instruction(
                    "LIST_GET",
                    &[
                        ("key", "out.item"),
                        ("source", "data.source"),
                        ("type", "Text"),
                        ("index", "i:1"),
                    ],
                ),
                instruction(
                    "LIST_APPEND",
                    &[
                        ("key", "out.list"),
                        ("source", "data.source"),
                        ("type", "Text"),
                        ("value", "text.value"),
                    ],
                ),
                instruction(
                    "LIST_SET",
                    &[
                        ("key", "out.set"),
                        ("source", "data.source"),
                        ("type", "Text"),
                        ("index", "i:-1"),
                        ("value", "text.value"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.item")]),
                instruction("OUTPUT_FIELD", &[("key", "out.list")]),
                instruction("OUTPUT_FIELD", &[("key", "out.set")]),
            ],
            &[r#"data.source=["a\u0000\n","\ud83e\udd80","e\u0301","\u007f\u0085"]"#],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            concat!(
            "out.item=🦀\n",
            "out.list=[\"a\\u0000\\n\",\"🦀\",\"é\",\"\\u007f\\u0085\",\"\\t\\\\\\\"é\\u0085\"]\n",
            "out.set=[\"a\\u0000\\n\",\"🦀\",\"é\",\"\\t\\\\\\\"é\\u0085\"]\n"
        )
            .as_bytes()
        );
    }

    #[test]
    fn native_lists_validate_every_item_and_strict_flat_json_grammar() {
        for (element_type, inputs) in [
            (
                "Int",
                vec![
                    "",
                    "null",
                    "{}",
                    "[1,]",
                    "[,1]",
                    "[1]x",
                    "[+1]",
                    "[01]",
                    "[-01]",
                    "[1.0]",
                    "[1e0]",
                    "[9223372036854775808]",
                    "[-9223372036854775809]",
                    "[1,true]",
                    "[1,\"2\"]",
                    "[1,[]]",
                    "[1,{}]",
                    "[1,\u{a0}2]",
                    "[1]\0",
                ],
            ),
            (
                "Text",
                vec![
                    "[\"ok\",1]",
                    "[\"ok\",null]",
                    "[\"ok\",[]]",
                    "[\"ok\",\"\\x\"]",
                    "[\"ok\",\"\\uD800\"]",
                    "[\"ok\",\"\\uDC00\"]",
                    "[\"ok\",\"\\uD800\\u0041\"]",
                    "[\"ok\",\"\\u12x4\"]",
                    "[\"ok\",\"\n\"]",
                    "[\"ok\",\"\0\"]",
                    "[\"unterminated]",
                    "[\"ok\",]",
                ],
            ),
        ] {
            for input in inputs {
                let output = run(
                    vec![
                        instruction("SET_FIELD", &[("key", "data.source"), ("value", input)]),
                        instruction(
                            "LIST_GET",
                            &[
                                ("key", "out.item"),
                                ("source", "data.source"),
                                ("type", element_type),
                                ("index", "i:0"),
                            ],
                        ),
                    ],
                    &[],
                );
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{element_type} {input:?}: {output:?}"
                );
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST:"),
                    "{input:?}: {output:?}"
                );
            }
        }
        for element_type in ["Int", "Text"] {
            let output = run(
                vec![instruction(
                    "LIST_LENGTH",
                    &[
                        ("key", "out.count"),
                        ("source", "data.missing"),
                        ("type", element_type),
                    ],
                )],
                &[],
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST:"));
        }
    }

    #[test]
    fn native_lists_reject_invalid_utf8_before_decoding_strings() {
        use std::os::unix::ffi::OsStringExt;
        for bytes in [
            vec![0x80],
            vec![0xc0, 0x80],
            vec![0xe0, 0x80, 0x80],
            vec![0xed, 0xa0, 0x80],
            vec![0xf4, 0x90, 0x80, 0x80],
            vec![0xf0, 0x9f],
        ] {
            let mut argument = b"data.source=[\"valid\",\"".to_vec();
            argument.extend(bytes);
            argument.extend(b"\"]");
            let output = run_os(
                vec![instruction(
                    "LIST_GET",
                    &[
                        ("key", "out.item"),
                        ("source", "data.source"),
                        ("type", "Text"),
                        ("index", "i:0"),
                    ],
                )],
                &[std::ffi::OsString::from_vec(argument)],
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST:"));
        }
    }

    #[test]
    fn native_lists_normalize_indices_and_preserve_error_order() {
        for (index, expected) in [("0", 11), ("1", 22), ("-1", 22), ("-2", 11)] {
            let output = run(
                vec![
                    instruction(
                        "LIST_GET",
                        &[
                            ("key", "out.item"),
                            ("source", "data.source"),
                            ("type", "Int"),
                            ("index", &IntExpr::parse(index).unwrap().to_postfix()),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.item")]),
                ],
                &["data.source=[11,22]"],
            );
            assert!(output.status.success(), "{output:?}");
            assert_eq!(output.stdout, format!("out.item={expected}\n").as_bytes());
        }
        for (source, index, value, error) in [
            ("[1,bad]", "1 / 0", "1 / 0", "AIL-RUNTIME-LIST:"),
            ("[]", "1 / 0", "1 / 0", "AIL-RUNTIME-DIVZERO"),
            ("[]", "0", "1 / 0", "AIL-RUNTIME-INDEX"),
            ("[1]", "2", "1 / 0", "AIL-RUNTIME-INDEX"),
            ("[1]", "-2", "1 / 0", "AIL-RUNTIME-INDEX"),
            ("[1]", "-9223372036854775808", "1 / 0", "AIL-RUNTIME-INDEX"),
            ("[1]", "9223372036854775807", "1 / 0", "AIL-RUNTIME-INDEX"),
            ("[1]", "0", "1 / 0", "AIL-RUNTIME-DIVZERO"),
            ("[1]", "input.missing", "0", "AIL-RUNTIME-INT"),
            (
                "[1]",
                "0",
                "9223372036854775807 + 1",
                "AIL-RUNTIME-OVERFLOW",
            ),
        ] {
            let output = run(
                vec![instruction(
                    "LIST_SET",
                    &[
                        ("key", "data.source"),
                        ("source", "data.source"),
                        ("type", "Int"),
                        ("index", &IntExpr::parse(index).unwrap().to_postfix()),
                        ("value", &IntExpr::parse(value).unwrap().to_postfix()),
                    ],
                )],
                &[&format!("data.source={source}")],
            );
            assert_eq!(
                output.status.code(),
                Some(1),
                "{source}/{index}/{value}: {output:?}"
            );
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(error),
                "{source}/{index}/{value}: {output:?}"
            );
        }
    }

    #[test]
    fn native_lists_canonical_expansion_only_limits_the_final_produced_value() {
        let source = format!("[\"{}\"]", "\u{7f}".repeat(200_000));
        let prefix = vec![
            instruction("SET_FIELD", &[("key", "data.source"), ("value", &source)]),
            instruction("SET_FIELD", &[("key", "text.small"), ("value", "x")]),
        ];
        let mut readable = prefix.clone();
        readable.extend([
            instruction(
                "LIST_LENGTH",
                &[
                    ("key", "out.count"),
                    ("source", "data.source"),
                    ("type", "Text"),
                ],
            ),
            instruction(
                "LIST_GET",
                &[
                    ("key", "text.item"),
                    ("source", "data.source"),
                    ("type", "Text"),
                    ("index", "i:0"),
                ],
            ),
            instruction(
                "TEXT_LENGTH",
                &[("key", "out.characters"), ("source", "text.item")],
            ),
            instruction(
                "LIST_SET",
                &[
                    ("key", "out.small"),
                    ("source", "data.source"),
                    ("type", "Text"),
                    ("index", "i:0"),
                    ("value", "text.small"),
                ],
            ),
            instruction("OUTPUT_FIELD", &[("key", "out.count")]),
            instruction("OUTPUT_FIELD", &[("key", "out.characters")]),
            instruction("OUTPUT_FIELD", &[("key", "out.small")]),
        ]);
        let output = run(readable, &[]);
        assert!(output.status.success(), "{:?}", output.status);
        assert_eq!(
            output.stdout,
            b"out.characters=200000\nout.count=1\nout.small=[\"x\"]\n"
        );
        let mut oversized = prefix;
        oversized.push(instruction(
            "LIST_APPEND",
            &[
                ("key", "out.list"),
                ("source", "data.source"),
                ("type", "Text"),
                ("value", "text.small"),
            ],
        ));
        let output = run(oversized, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT"));
    }

    #[test]
    fn native_lists_enforce_item_and_byte_limits_after_value_evaluation() {
        for (count, expected) in [(65_536, true), (65_537, false)] {
            let source = format!("[{}0]", "0,".repeat(count - 1));
            let prefix = vec![instruction(
                "SET_FIELD",
                &[("key", "data.source"), ("value", &source)],
            )];
            let mut length = prefix.clone();
            length.push(instruction(
                "LIST_LENGTH",
                &[
                    ("key", "out.count"),
                    ("source", "data.source"),
                    ("type", "Int"),
                ],
            ));
            length.push(instruction("OUTPUT_FIELD", &[("key", "out.count")]));
            let output = run(length, &[]);
            assert_eq!(output.status.success(), expected);
            if expected {
                assert_eq!(output.stdout, b"out.count=65536\n");
                for (value, error) in [
                    ("i:1", "AIL-RUNTIME-LIST-LIMIT"),
                    ("f:input.missing", "AIL-RUNTIME-INT"),
                ] {
                    let mut append = prefix.clone();
                    append.push(instruction(
                        "LIST_APPEND",
                        &[
                            ("key", "out.list"),
                            ("source", "data.source"),
                            ("type", "Int"),
                            ("value", value),
                        ],
                    ));
                    let output = run(append, &[]);
                    assert_eq!(output.status.code(), Some(1));
                    assert!(output.stdout.is_empty());
                    assert!(String::from_utf8_lossy(&output.stderr).contains(error));
                }
            } else {
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT"));
            }
        }
        for (size, expected) in [(1_048_576, true), (1_048_577, false)] {
            let source = format!("[\"{}\"]", "a".repeat(size - 4));
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "data.source"), ("value", &source)]),
                    instruction(
                        "LIST_LENGTH",
                        &[
                            ("key", "out.count"),
                            ("source", "data.source"),
                            ("type", "Text"),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.count")]),
                ],
                &[],
            );
            assert_eq!(output.status.success(), expected);
            if expected {
                assert_eq!(output.stdout, b"out.count=1\n");
            } else {
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT"));
            }
        }
    }

    #[test]
    fn native_lists_share_result_budget_across_calls_and_charge_two_bytes_for_empty() {
        let value = "a".repeat(1_048_572);
        let almost = "b".repeat(1_048_570);
        let mut instructions = vec![
            instruction("SET_FIELD", &[("key", "data.seed"), ("value", "[\"\"]")]),
            instruction("SET_FIELD", &[("key", "text.value"), ("value", &value)]),
            instruction("SET_FIELD", &[("key", "text.almost"), ("value", &almost)]),
            eval("loop.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "2048")]),
            instruction(
                "LIST_SET",
                &[
                    ("key", "data.result"),
                    ("source", "data.seed"),
                    ("type", "Text"),
                    ("index", "i:0"),
                    ("value", "text.value"),
                ],
            ),
            instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "1")]),
            eval("loop.count", "loop.count + 1"),
            branch_int("loop.count", "<", "63", "again"),
            instruction(
                "LIST_SET",
                &[
                    ("key", "data.result"),
                    ("source", "data.seed"),
                    ("type", "Text"),
                    ("index", "i:0"),
                    ("value", "text.almost"),
                ],
            ),
            instruction("LIST_EMPTY", &[("key", "out.empty"), ("type", "Text")]),
            instruction(
                "LIST_GET",
                &[
                    ("key", "text.item"),
                    ("source", "data.result"),
                    ("type", "Text"),
                    ("index", "i:0"),
                ],
            ),
            instruction(
                "TEXT_LENGTH",
                &[("key", "out.characters"), ("source", "text.item")],
            ),
            instruction(
                "LIST_LENGTH",
                &[
                    ("key", "out.count"),
                    ("source", "data.result"),
                    ("type", "Text"),
                ],
            ),
            instruction("OUTPUT_FIELD", &[("key", "out.empty")]),
            instruction("OUTPUT_FIELD", &[("key", "out.count")]),
            instruction("OUTPUT_FIELD", &[("key", "out.characters")]),
        ];
        let output = run(instructions.clone(), &[]);
        assert!(output.status.success(), "{:?}", output.status);
        assert_eq!(
            output.stdout,
            b"out.characters=1048570\nout.count=1\nout.empty=[]\n"
        );
        instructions.push(instruction(
            "LIST_EMPTY",
            &[("key", "out.empty"), ("type", "Int")],
        ));
        let output = run(instructions, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT"));
    }

    #[test]
    fn native_lists_text_get_charges_the_separate_text_budget_even_for_empty_items() {
        let value = "a".repeat(1_048_576);
        let mut instructions = vec![
            instruction("SET_FIELD", &[("key", "text.seed"), ("value", &value)]),
            instruction("SET_FIELD", &[("key", "text.empty"), ("value", "")]),
            instruction("SET_FIELD", &[("key", "data.text"), ("value", "[\"\"]")]),
            instruction("SET_FIELD", &[("key", "data.int"), ("value", "[42]")]),
            eval("loop.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            instruction(
                "TEXT_JOIN",
                &[
                    ("key", "text.result"),
                    ("left", "text.seed"),
                    ("right", "text.empty"),
                ],
            ),
            eval("loop.count", "loop.count + 1"),
            branch_int("loop.count", "<", "64", "again"),
            instruction("LIST_EMPTY", &[("key", "out.empty"), ("type", "Int")]),
            instruction(
                "LIST_GET",
                &[
                    ("key", "out.integer"),
                    ("source", "data.int"),
                    ("type", "Int"),
                    ("index", "i:0"),
                ],
            ),
            instruction("OUTPUT_FIELD", &[("key", "out.empty")]),
            instruction("OUTPUT_FIELD", &[("key", "out.integer")]),
        ];
        let output = run(instructions.clone(), &[]);
        assert!(output.status.success(), "{:?}", output.status);
        assert_eq!(output.stdout, b"out.empty=[]\nout.integer=42\n");
        instructions.push(instruction(
            "LIST_GET",
            &[
                ("key", "text.result"),
                ("source", "data.text"),
                ("type", "Text"),
                ("index", "i:0"),
            ],
        ));
        let output = run(instructions, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT"));
    }

    #[test]
    fn native_lists_decoded_text_survives_scratch_reuse_and_list_updates() {
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "text.new"), ("value", "new\0\n")]),
                instruction(
                    "LIST_GET",
                    &[
                        ("key", "out.saved"),
                        ("source", "data.source"),
                        ("type", "Text"),
                        ("index", "i:0"),
                    ],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.original"), ("source", "data.source")],
                ),
                instruction(
                    "LIST_SET",
                    &[
                        ("key", "data.source"),
                        ("source", "data.source"),
                        ("type", "Text"),
                        ("index", "i:0"),
                        ("value", "text.new"),
                    ],
                ),
                instruction(
                    "LIST_GET",
                    &[
                        ("key", "out.new"),
                        ("source", "data.source"),
                        ("type", "Text"),
                        ("index", "i:0"),
                    ],
                ),
                instruction(
                    "LIST_APPEND",
                    &[
                        ("key", "data.source"),
                        ("source", "data.source"),
                        ("type", "Text"),
                        ("value", "out.saved"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "data.source")]),
                instruction("OUTPUT_FIELD", &[("key", "out.saved")]),
                instruction("OUTPUT_FIELD", &[("key", "out.original")]),
                instruction("OUTPUT_FIELD", &[("key", "out.new")]),
            ],
            &[r#"data.source=["old\u0000\n🦀"]"#],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            concat!(
                "data.source=[\"new\\u0000\\n\",\"old\\u0000\\n🦀\"]\n",
                "out.new=new\0\n\n",
                "out.original=[\"old\\u0000\\n🦀\"]\n",
                "out.saved=old\0\n🦀\n"
            )
            .as_bytes()
        );
    }

    #[test]
    fn native_lists_compose_with_loops_recursive_functions_and_text_arena() {
        let factorial = integer_function(
            "factorial",
            "n",
            vec![
                branch_int("local.n", "<=", "1", "base"),
                call_int("factorial", "n=f:local.n i:1 -", "local.partial"),
                return_int("local.n * local.partial"),
                instruction("LABEL", &[("name", "base")]),
                return_int("1"),
            ],
        );
        let output = run_functions(
            vec![
                instruction(
                    "TEXT_JOIN",
                    &[
                        ("key", "text.saved"),
                        ("left", "text.input"),
                        ("right", "text.input"),
                    ],
                ),
                instruction("LIST_EMPTY", &[("key", "out.list"), ("type", "Int")]),
                instruction(
                    "LIST_LENGTH",
                    &[
                        ("key", "loop.length"),
                        ("source", "data.source"),
                        ("type", "Int"),
                    ],
                ),
                eval("loop.index", "0"),
                eval("out.sum", "0"),
                instruction("LABEL", &[("name", "again")]),
                instruction(
                    "LIST_GET",
                    &[
                        ("key", "loop.item"),
                        ("source", "data.source"),
                        ("type", "Int"),
                        ("index", "f:loop.index"),
                    ],
                ),
                call_int("factorial", "n=f:loop.item", "loop.answer"),
                instruction(
                    "LIST_APPEND",
                    &[
                        ("key", "out.list"),
                        ("source", "out.list"),
                        ("type", "Int"),
                        ("value", "f:loop.answer"),
                    ],
                ),
                eval("out.sum", "out.sum + loop.answer"),
                eval("loop.index", "loop.index + 1"),
                branch_int("loop.index", "<", "loop.length", "again"),
                text_slice("out.text", "text.saved", "1", "-1"),
                instruction("OUTPUT_FIELD", &[("key", "out.list")]),
                instruction("OUTPUT_FIELD", &[("key", "out.sum")]),
                instruction("OUTPUT_FIELD", &[("key", "out.text")]),
            ],
            BTreeMap::from([factorial]),
            &["data.source=[3,4,5]", "text.input=é🦀"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            "out.list=[6,24,120]\nout.sum=150\nout.text=🦀é\n".as_bytes()
        );
    }

    #[test]
    fn native_text_counts_and_finds_unicode_scalars_without_normalizing() {
        for (source, needle, length, position) in [
            ("Aé🦀e\u{301}", "🦀", 5, 2),
            ("e\u{301}", "é", 2, -1),
            ("éé", "é", 2, 0),
            ("abc", "", 3, 0),
            ("", "", 0, 0),
            ("", "a", 0, -1),
            ("a\0b", "\0", 3, 1),
            ("\u{10ffff}z", "z", 2, 1),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "text.source"), ("value", source)]),
                    instruction("SET_FIELD", &[("key", "text.needle"), ("value", needle)]),
                    instruction(
                        "TEXT_LENGTH",
                        &[("key", "out.length"), ("source", "text.source")],
                    ),
                    instruction(
                        "TEXT_FIND",
                        &[
                            ("key", "out.position"),
                            ("source", "text.source"),
                            ("needle", "text.needle"),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.length")]),
                    instruction("OUTPUT_FIELD", &[("key", "out.position")]),
                ],
                &[],
            );
            assert!(output.status.success(), "{source:?}/{needle:?}: {output:?}");
            assert_eq!(
                output.stdout,
                format!("out.length={length}\nout.position={position}\n").as_bytes()
            );
        }
    }

    #[test]
    fn native_text_transformations_raw_opcodes_preserve_unicode_empty_parts_and_aliases() {
        let output = run(
            vec![
                instruction(
                    "SET_FIELD",
                    &[
                        ("key", "text.source"),
                        ("value", "\u{85} é||a\0|| \u{3000}"),
                    ],
                ),
                instruction("SET_FIELD", &[("key", "text.separator"), ("value", "||")]),
                instruction("SET_FIELD", &[("key", "text.replacement"), ("value", "_")]),
                instruction(
                    "TEXT_TRIM",
                    &[("key", "text.source"), ("source", "text.source")],
                ),
                instruction(
                    "TEXT_SPLIT",
                    &[
                        ("key", "out.parts"),
                        ("source", "text.source"),
                        ("separator", "text.separator"),
                    ],
                ),
                instruction(
                    "TEXT_JOIN_LIST",
                    &[
                        ("key", "out.joined"),
                        ("source", "out.parts"),
                        ("separator", "text.replacement"),
                    ],
                ),
                instruction(
                    "TEXT_REPLACE",
                    &[
                        ("key", "text.replacement"),
                        ("source", "out.joined"),
                        ("needle", "text.replacement"),
                        ("replacement", "text.separator"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.parts")]),
                instruction("OUTPUT_FIELD", &[("key", "out.joined")]),
                instruction("OUTPUT_FIELD", &[("key", "text.replacement")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.joined=\xc3\xa9_a\0_\nout.parts=[\"\xc3\xa9\",\"a\\u0000\",\"\"]\ntext.replacement=\xc3\xa9||a\0||\n");
    }

    #[test]
    fn native_text_transformations_cover_nonoverlap_and_empty_scalar_boundaries() {
        for (source, separator, expected) in [
            ("", ",", "[\"\"]"),
            (",a,,", ",", "[\"\",\"a\",\"\",\"\"]"),
            ("aaaaa", "aa", "[\"\",\"\",\"a\"]"),
            ("é🦀é", "é", "[\"\",\"🦀\",\"\"]"),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "x.source"), ("value", source)]),
                    instruction("SET_FIELD", &[("key", "x.sep"), ("value", separator)]),
                    instruction(
                        "TEXT_SPLIT",
                        &[
                            ("key", "x.source"),
                            ("source", "x.source"),
                            ("separator", "x.sep"),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "x.source")]),
                ],
                &[],
            );
            assert!(output.status.success(), "{output:?}");
            assert_eq!(output.stdout, format!("x.source={expected}\n").as_bytes());
        }
        for (source, needle, replacement, expected) in [
            ("", "", "_", "_"),
            ("é🦀e\u{301}\0", "", "_", "_é_🦀_e_\u{301}_\0_"),
            ("aaaaa", "aa", "b", "bba"),
            ("aa", "a", "aa", "aaaa"),
            ("abc", "longer", "x", "abc"),
            ("éé", "é", "", ""),
            ("abc", "", "", "abc"),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "x.source"), ("value", source)]),
                    instruction("SET_FIELD", &[("key", "x.needle"), ("value", needle)]),
                    instruction(
                        "SET_FIELD",
                        &[("key", "x.replacement"), ("value", replacement)],
                    ),
                    instruction(
                        "TEXT_REPLACE",
                        &[
                            ("key", "x.needle"),
                            ("source", "x.source"),
                            ("needle", "x.needle"),
                            ("replacement", "x.replacement"),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "x.needle")]),
                ],
                &[],
            );
            assert!(output.status.success(), "{output:?}");
            assert_eq!(output.stdout, format!("x.needle={expected}\n").as_bytes());
        }
        for (source, separator, expected) in [
            ("[]", "|", ""),
            ("[\"\"]", "|", ""),
            ("[\"\",\"é\",\"\\u0000\",\"\"]", "🦀", "🦀é🦀\0🦀"),
            ("[\"a\",\"b\"]", "", "ab"),
        ] {
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "x.source"), ("value", source)]),
                    instruction("SET_FIELD", &[("key", "x.sep"), ("value", separator)]),
                    instruction(
                        "TEXT_JOIN_LIST",
                        &[
                            ("key", "x.sep"),
                            ("source", "x.source"),
                            ("separator", "x.sep"),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "x.sep")]),
                ],
                &[],
            );
            assert!(output.status.success(), "{output:?}");
            assert_eq!(output.stdout, format!("x.sep={expected}\n").as_bytes());
        }
    }

    #[test]
    fn native_text_transformations_trim_exact_fixed_whitespace_set() {
        let whitespace: String = (9..=13)
            .chain(0x1c..=0x20)
            .chain([0x85, 0xa0, 0x1680])
            .chain(0x2000..=0x200a)
            .chain([0x2028, 0x2029, 0x202f, 0x205f, 0x3000])
            .map(|c| char::from_u32(c).unwrap())
            .collect();
        assert_eq!(whitespace.chars().count(), 29);
        for inner in ["", "\0", "\u{200b}", "\u{feff}", "é \t🦀"] {
            let input = format!("{whitespace}{inner}{whitespace}");
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "x.text"), ("value", &input)]),
                    instruction("TEXT_TRIM", &[("key", "x.text"), ("source", "x.text")]),
                    instruction("OUTPUT_FIELD", &[("key", "x.text")]),
                ],
                &[],
            );
            assert!(output.status.success(), "{output:?}");
            assert_eq!(output.stdout, format!("x.text={inner}\n").as_bytes());
        }
    }

    #[test]
    fn native_text_transformations_validate_all_inputs_in_contract_order() {
        for (opcode, source, second, third, family) in [
            ("TEXT_SPLIT", "".into(), Some("".into()), None, "TEXT"),
            (
                "TEXT_SPLIT",
                "x".repeat(1_048_577),
                Some("".into()),
                None,
                "TEXT-LIMIT",
            ),
            (
                "TEXT_JOIN_LIST",
                "[\"valid\",false]".into(),
                None,
                None,
                "LIST",
            ),
            ("TEXT_JOIN_LIST", "[] trailing".into(), None, None, "LIST"),
            ("TEXT_JOIN_LIST", "[]".into(), None, None, "TEXT"),
            (
                "TEXT_REPLACE",
                "x".repeat(1_048_577),
                None,
                None,
                "TEXT-LIMIT",
            ),
            (
                "TEXT_REPLACE",
                "x".into(),
                Some("x".repeat(1_048_577)),
                None,
                "TEXT-LIMIT",
            ),
            ("TEXT_REPLACE", "x".into(), Some("".into()), None, "TEXT"),
            ("TEXT_TRIM", "x".repeat(1_048_577), None, None, "TEXT-LIMIT"),
        ] {
            let third: Option<String> = third;
            let mut code = vec![instruction(
                "SET_FIELD",
                &[("key", "x.source"), ("value", &source)],
            )];
            if let Some(value) = second {
                code.push(instruction(
                    "SET_FIELD",
                    &[("key", "x.other"), ("value", &value)],
                ));
            }
            if let Some(value) = third {
                code.push(instruction(
                    "SET_FIELD",
                    &[("key", "x.replacement"), ("value", &value)],
                ));
            }
            let mut operands = vec![("key", "out.value"), ("source", "x.source")];
            if matches!(opcode, "TEXT_SPLIT" | "TEXT_JOIN_LIST") {
                operands.push(("separator", "x.other"));
            }
            if opcode == "TEXT_REPLACE" {
                operands.extend([("needle", "x.other"), ("replacement", "x.replacement")]);
            }
            code.push(instruction(opcode, &operands));
            code.push(instruction("OUTPUT_FIELD", &[("key", "out.value")]));
            let output = run(code, &[]);
            assert_eq!(output.status.code(), Some(1), "{opcode}: {output:?}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:")),
                "{opcode}: {output:?}"
            );
        }
    }

    #[test]
    fn native_text_transformations_reject_invalid_utf8_in_every_operand() {
        use std::os::unix::ffi::OsStringExt;
        for (opcode, operands, bad, source, family) in [
            (
                "TEXT_SPLIT",
                vec![
                    ("key", "out.value"),
                    ("source", "x.source"),
                    ("separator", "x.other"),
                ],
                "x.source",
                "abc",
                "TEXT",
            ),
            (
                "TEXT_SPLIT",
                vec![
                    ("key", "out.value"),
                    ("source", "x.source"),
                    ("separator", "x.other"),
                ],
                "x.other",
                "abc",
                "TEXT",
            ),
            (
                "TEXT_JOIN_LIST",
                vec![
                    ("key", "out.value"),
                    ("source", "x.source"),
                    ("separator", "x.other"),
                ],
                "x.source",
                "[]",
                "LIST",
            ),
            (
                "TEXT_JOIN_LIST",
                vec![
                    ("key", "out.value"),
                    ("source", "x.source"),
                    ("separator", "x.other"),
                ],
                "x.other",
                "[]",
                "TEXT",
            ),
            (
                "TEXT_REPLACE",
                vec![
                    ("key", "out.value"),
                    ("source", "x.source"),
                    ("needle", "x.other"),
                    ("replacement", "x.replacement"),
                ],
                "x.source",
                "abc",
                "TEXT",
            ),
            (
                "TEXT_REPLACE",
                vec![
                    ("key", "out.value"),
                    ("source", "x.source"),
                    ("needle", "x.other"),
                    ("replacement", "x.replacement"),
                ],
                "x.other",
                "abc",
                "TEXT",
            ),
            (
                "TEXT_REPLACE",
                vec![
                    ("key", "out.value"),
                    ("source", "x.source"),
                    ("needle", "x.other"),
                    ("replacement", "x.replacement"),
                ],
                "x.replacement",
                "abc",
                "TEXT",
            ),
            (
                "TEXT_TRIM",
                vec![("key", "out.value"), ("source", "x.source")],
                "x.source",
                "abc",
                "TEXT",
            ),
        ] {
            let args = [
                ("x.source", source),
                ("x.other", "a"),
                ("x.replacement", "b"),
            ]
            .into_iter()
            .map(|(key, value)| {
                let mut bytes = format!("{key}=").into_bytes();
                bytes.extend_from_slice(if key == bad {
                    &[0xc0, 0x80]
                } else {
                    value.as_bytes()
                });
                std::ffi::OsString::from_vec(bytes)
            })
            .collect::<Vec<_>>();
            let output = run_os(vec![instruction(opcode, &operands)], &args);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:")),
                "{opcode}/{bad}: {output:?}"
            );
        }
    }

    #[test]
    fn native_text_transformations_enforce_counts_final_sizes_and_borrowed_list_semantics() {
        for count in [65_535, 65_536] {
            let source = ",".repeat(count);
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "x.source"), ("value", &source)]),
                    instruction("SET_FIELD", &[("key", "x.sep"), ("value", ",")]),
                    instruction(
                        "TEXT_SPLIT",
                        &[
                            ("key", "x.parts"),
                            ("source", "x.source"),
                            ("separator", "x.sep"),
                        ],
                    ),
                    instruction(
                        "LIST_LENGTH",
                        &[
                            ("key", "out.count"),
                            ("source", "x.parts"),
                            ("type", "Text"),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.count")]),
                ],
                &[],
            );
            if count == 65_535 {
                assert!(output.status.success());
                assert_eq!(output.stdout, b"out.count=65536\n");
            } else {
                assert_eq!(output.status.code(), Some(1));
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT:")
                );
            }
        }
        let source = format!("[\"{}\"]", "\u{85}".repeat(180_000));
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "x.source"), ("value", &source)]),
                instruction("SET_FIELD", &[("key", "x.sep"), ("value", "")]),
                instruction(
                    "TEXT_JOIN_LIST",
                    &[
                        ("key", "x.source"),
                        ("source", "x.source"),
                        ("separator", "x.sep"),
                    ],
                ),
                instruction(
                    "TEXT_LENGTH",
                    &[("key", "out.count"), ("source", "x.source")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.count")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.count=180000\n");
        for size in [1_048_572, 1_048_573] {
            let source = "x".repeat(size);
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "x.source"), ("value", &source)]),
                    instruction("SET_FIELD", &[("key", "x.sep"), ("value", "y")]),
                    instruction(
                        "TEXT_SPLIT",
                        &[
                            ("key", "x.parts"),
                            ("source", "x.source"),
                            ("separator", "x.sep"),
                        ],
                    ),
                ],
                &[],
            );
            assert_eq!(output.status.success(), size == 1_048_572, "{output:?}");
            if size == 1_048_573 {
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT:")
                );
            }
        }
        for size in [524_288, 524_289] {
            let source = "x".repeat(size);
            let output = run(
                vec![
                    instruction("SET_FIELD", &[("key", "x.source"), ("value", &source)]),
                    instruction("SET_FIELD", &[("key", "x.needle"), ("value", "x")]),
                    instruction("SET_FIELD", &[("key", "x.replacement"), ("value", "xx")]),
                    instruction(
                        "TEXT_REPLACE",
                        &[
                            ("key", "x.source"),
                            ("source", "x.source"),
                            ("needle", "x.needle"),
                            ("replacement", "x.replacement"),
                        ],
                    ),
                ],
                &[],
            );
            assert_eq!(output.status.success(), size == 524_288, "{output:?}");
            if size == 524_289 {
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT:")
                );
            }
        }
    }

    #[test]
    fn native_text_transformations_charge_independent_arenas_once_and_empty_text_costs_one() {
        let large = "x".repeat(1_048_576);
        let split = "x".repeat(1_048_572);
        let code = vec![
            instruction("SET_FIELD", &[("key", "x.large"), ("value", &large)]),
            instruction("SET_FIELD", &[("key", "x.split"), ("value", &split)]),
            instruction("SET_FIELD", &[("key", "x.sep"), ("value", "z")]),
            instruction("SET_FIELD", &[("key", "x.empty"), ("value", "")]),
            eval("x.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            instruction("TEXT_TRIM", &[("key", "x.result"), ("source", "x.large")]),
            instruction(
                "TEXT_SPLIT",
                &[
                    ("key", "x.parts"),
                    ("source", "x.split"),
                    ("separator", "x.sep"),
                ],
            ),
            eval("x.count", "x.count + 1"),
            branch_int("x.count", "<", "64", "again"),
        ];
        let output = run(code.clone(), &[]);
        assert!(output.status.success(), "{output:?}");
        for (opcode, operands, family) in [
            (
                "TEXT_TRIM",
                vec![("key", "x.result"), ("source", "x.empty")],
                "TEXT-LIMIT",
            ),
            (
                "TEXT_SPLIT",
                vec![
                    ("key", "x.parts"),
                    ("source", "x.empty"),
                    ("separator", "x.sep"),
                ],
                "LIST-LIMIT",
            ),
        ] {
            let mut exhausted = code.clone();
            exhausted.push(instruction(opcode, &operands));
            let output = run(exhausted, &[]);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:")),
                "{output:?}"
            );
        }
    }

    #[test]
    fn native_text_transformations_preserve_recursive_frames_with_body_only_helpers() {
        let functions = BTreeMap::from([value_function(
            "clean",
            "n:Int;sep:Text;text:Text",
            "parts:List<Text>;result:Text",
            "Text",
            vec![
                instruction(
                    "TEXT_TRIM",
                    &[("key", "local.result"), ("source", "local.text")],
                ),
                instruction(
                    "TEXT_SPLIT",
                    &[
                        ("key", "local.parts"),
                        ("source", "local.result"),
                        ("separator", "local.sep"),
                    ],
                ),
                instruction(
                    "TEXT_JOIN_LIST",
                    &[
                        ("key", "local.result"),
                        ("source", "local.parts"),
                        ("separator", "local.sep"),
                    ],
                ),
                instruction(
                    "TEXT_REPLACE",
                    &[
                        ("key", "local.result"),
                        ("source", "local.result"),
                        ("needle", "local.sep"),
                        ("replacement", "local.sep"),
                    ],
                ),
                branch_int("local.n", "<=", "0", "base"),
                call_value(
                    "clean",
                    "n=f:local.n i:1 -;sep=f:local.sep;text=f:local.result",
                    "local.result",
                ),
                instruction("LABEL", &[("name", "base")]),
                return_value("Text", "local.result"),
            ],
        )]);
        let root = value_root("clean", "n=i:10;sep=f:x.sep;text=f:x.text");
        let binary = emit_with_functions(
            &AilBytecodeAction {
                name: "Root".into(),
                instructions: root.clone(),
            },
            &BTreeMap::new(),
            &functions,
            &BTreeMap::new(),
        )
        .unwrap();
        // Text-only procedures keep the ordinary scalar ELF's segment count;
        // the optional floating runtime introduces further load segments.
        let plain = emit(
            &AilBytecodeAction {
                name: "Root".into(),
                instructions: vec![eval("out.n", "1")],
            },
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(&binary[56..58], &plain[56..58]);
        let output = run_functions(root, functions, &["x.sep=|", "x.text= \té||🦀 \u{3000}"]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, "out.result=é||🦀\n".as_bytes());
        assert!(
            output.stderr.is_empty(),
            "local transformations add no trace"
        );
    }

    #[test]
    fn native_text_transformations_function_return_adds_one_owned_copy() {
        let functions = BTreeMap::from([value_function(
            "trim",
            "text:Text",
            "result:Text",
            "Text",
            vec![
                instruction(
                    "TEXT_TRIM",
                    &[("key", "local.result"), ("source", "local.text")],
                ),
                return_value("Text", "local.result"),
            ],
        )]);
        let large = "x".repeat(1_048_576);
        let mut root = vec![
            instruction("SET_FIELD", &[("key", "x.text"), ("value", &large)]),
            instruction("SET_FIELD", &[("key", "x.empty"), ("value", "")]),
            eval("x.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            call_value("trim", "text=f:x.text", "out.text"),
            eval("x.count", "x.count + 1"),
            branch_int("x.count", "<", "32", "again"),
        ];
        let output = run_functions(root.clone(), functions.clone(), &[]);
        assert!(output.status.success(), "{output:?}");
        root.push(instruction(
            "TEXT_TRIM",
            &[("key", "out.text"), ("source", "x.empty")],
        ));
        let output = run_functions(root, functions, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT:"));
    }

    #[test]
    fn native_text_transformations_join_replace_share_exact_text_budget() {
        let large = "x".repeat(1_048_576);
        let mut code = vec![
            instruction("SET_FIELD", &[("key", "x.large"), ("value", &large)]),
            instruction("SET_FIELD", &[("key", "x.list"), ("value", "[\"\",\"\"]")]),
            instruction("SET_FIELD", &[("key", "x.empty"), ("value", "")]),
            eval("x.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            instruction(
                "TEXT_JOIN_LIST",
                &[
                    ("key", "x.result"),
                    ("source", "x.list"),
                    ("separator", "x.large"),
                ],
            ),
            // A maximal empty-needle mapping includes 1,048,577 offsets. Empty
            // replacement preserves the source and must still charge one copy.
            instruction(
                "TEXT_REPLACE",
                &[
                    ("key", "x.result"),
                    ("source", "x.result"),
                    ("needle", "x.empty"),
                    ("replacement", "x.empty"),
                ],
            ),
            eval("x.count", "x.count + 1"),
            branch_int("x.count", "<", "32", "again"),
        ];
        let output = run(code.clone(), &[]);
        assert!(output.status.success(), "{output:?}");
        code.push(instruction(
            "TEXT_REPLACE",
            &[
                ("key", "x.result"),
                ("source", "x.empty"),
                ("needle", "x.empty"),
                ("replacement", "x.empty"),
            ],
        ));
        let output = run(code, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT:"));
    }

    #[test]
    fn native_text_transformations_split_return_adds_one_owned_list_copy() {
        let functions = BTreeMap::from([value_function(
            "split",
            "sep:Text;text:Text",
            "parts:List<Text>",
            "List<Text>",
            vec![
                instruction(
                    "TEXT_SPLIT",
                    &[
                        ("key", "local.parts"),
                        ("source", "local.text"),
                        ("separator", "local.sep"),
                    ],
                ),
                return_value("List<Text>", "local.parts"),
            ],
        )]);
        let large = "x".repeat(1_048_572);
        let mut root = vec![
            instruction("SET_FIELD", &[("key", "x.text"), ("value", &large)]),
            instruction("SET_FIELD", &[("key", "x.empty"), ("value", "")]),
            instruction("SET_FIELD", &[("key", "x.sep"), ("value", "y")]),
            eval("x.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            call_value("split", "sep=f:x.sep;text=f:x.text", "out.parts"),
            eval("x.count", "x.count + 1"),
            branch_int("x.count", "<", "32", "again"),
        ];
        let output = run_functions(root.clone(), functions.clone(), &[]);
        assert!(output.status.success(), "{output:?}");
        root.push(instruction(
            "TEXT_SPLIT",
            &[
                ("key", "out.parts"),
                ("source", "x.empty"),
                ("separator", "x.sep"),
            ],
        ));
        let output = run_functions(root, functions, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT:"));
    }

    #[test]
    fn native_text_transformations_match_repetitive_megabyte_inputs_in_linear_time() {
        let _guard = EXECUTION.lock().unwrap();
        for final_byte in ["a", "b"] {
            // The no-match path and a match at the end both force a long prefix
            // fallback. Rebuilding/searching a prefix per position is quadratic.
            let source = "a".repeat(1_048_569) + final_byte;
            let needle = "a".repeat(524_284) + "b";
            let action = AilBytecodeAction {
                name: "Match".into(),
                instructions: vec![
                    instruction("SET_FIELD", &[("key", "x.source"), ("value", &source)]),
                    instruction("SET_FIELD", &[("key", "x.needle"), ("value", &needle)]),
                    instruction("SET_FIELD", &[("key", "x.empty"), ("value", "")]),
                    instruction(
                        "TEXT_SPLIT",
                        &[
                            ("key", "x.parts"),
                            ("source", "x.source"),
                            ("separator", "x.needle"),
                        ],
                    ),
                    instruction(
                        "TEXT_REPLACE",
                        &[
                            ("key", "x.source"),
                            ("source", "x.source"),
                            ("needle", "x.needle"),
                            ("replacement", "x.empty"),
                        ],
                    ),
                    instruction(
                        "TEXT_LENGTH",
                        &[("key", "out.length"), ("source", "x.source")],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.length")]),
                ],
            };
            let binary = emit(&action, &BTreeMap::new()).unwrap();
            let path = std::env::temp_dir().join(format!(
                "ail-transform-linear-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::write(&path, binary).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            let mut child = Command::new(&path)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while child.try_wait().unwrap().is_none() && std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            if child.try_wait().unwrap().is_none() {
                child.kill().unwrap();
                child.wait().unwrap();
                std::fs::remove_file(&path).unwrap();
                panic!("bounded transform matcher exceeded three seconds");
            }
            let output = child.wait_with_output().unwrap();
            std::fs::remove_file(&path).unwrap();
            assert!(output.status.success(), "{output:?}");
            let expected = if final_byte == "b" {
                524_285
            } else {
                1_048_570
            };
            assert_eq!(output.stdout, format!("out.length={expected}\n").as_bytes());
        }
    }

    #[test]
    fn native_text_uses_explicit_lengths_for_nul_and_newline_values() {
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "text.source"), ("value", "é\0\n🦀")]),
                instruction("SET_FIELD", &[("key", "text.needle"), ("value", "\0\n")]),
                instruction(
                    "TEXT_FIND",
                    &[
                        ("key", "out.position"),
                        ("source", "text.source"),
                        ("needle", "text.needle"),
                    ],
                ),
                text_slice(
                    "text.part",
                    "text.source",
                    "out.position",
                    "out.position + 2",
                ),
                instruction(
                    "TEXT_JOIN",
                    &[
                        ("key", "out.joined"),
                        ("left", "text.part"),
                        ("right", "text.source"),
                    ],
                ),
                instruction(
                    "TEXT_LENGTH",
                    &[("key", "out.length"), ("source", "out.joined")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.position")]),
                instruction("OUTPUT_FIELD", &[("key", "out.length")]),
                instruction("OUTPUT_FIELD", &[("key", "out.joined")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            "out.joined=\0\né\0\n🦀\nout.length=6\nout.position=1\n".as_bytes()
        );
    }

    #[test]
    fn native_text_slices_scalar_boundaries_with_negative_and_extreme_indices() {
        for (start, end, expected) in [
            ("1", "3", "é🦀"),
            ("-3", "-1", "é🦀"),
            ("3", "1", ""),
            ("-9223372036854775808", "9223372036854775807", "aé🦀z"),
            ("9223372036854775807", "9223372036854775807", ""),
            ("0", "-9223372036854775808", ""),
            ("4", "5", ""),
            ("input.start + 1", "input.end - 1", "é🦀"),
        ] {
            let output = run(
                vec![
                    text_slice("out.part", "text.source", start, end),
                    instruction("OUTPUT_FIELD", &[("key", "out.part")]),
                ],
                &["text.source=aé🦀z", "input.start=0", "input.end=4"],
            );
            assert!(output.status.success(), "{start}/{end}: {output:?}");
            assert_eq!(output.stdout, format!("out.part={expected}\n").as_bytes());
        }
    }

    #[test]
    fn native_text_results_are_immutable_across_aliases_and_source_overwrites() {
        let output = run(
            vec![
                instruction(
                    "TEXT_JOIN",
                    &[
                        ("key", "text.left"),
                        ("left", "text.left"),
                        ("right", "text.right"),
                    ],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.saved"), ("source", "text.left")],
                ),
                instruction(
                    "TEXT_JOIN",
                    &[
                        ("key", "text.left"),
                        ("left", "text.left"),
                        ("right", "text.left"),
                    ],
                ),
                text_slice("out.slice", "text.left", "1", "-1"),
                text_slice("text.left", "text.left", "0", "1"),
                instruction("OUTPUT_FIELD", &[("key", "text.left")]),
                instruction("OUTPUT_FIELD", &[("key", "out.saved")]),
                instruction("OUTPUT_FIELD", &[("key", "out.slice")]),
            ],
            &["text.left=é", "text.right=🦀"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            "out.saved=é🦀\nout.slice=🦀é\ntext.left=é\n".as_bytes()
        );
    }

    #[test]
    fn native_text_rejects_missing_and_strictly_invalid_utf8_operands() {
        use std::os::unix::ffi::OsStringExt;
        let operations = vec![
            instruction(
                "TEXT_LENGTH",
                &[("key", "out.result"), ("source", "text.bad")],
            ),
            instruction(
                "TEXT_JOIN",
                &[
                    ("key", "out.result"),
                    ("left", "text.good"),
                    ("right", "text.bad"),
                ],
            ),
            instruction(
                "TEXT_FIND",
                &[
                    ("key", "out.result"),
                    ("source", "text.good"),
                    ("needle", "text.bad"),
                ],
            ),
            text_slice("out.result", "text.bad", "0", "1"),
        ];
        let invalid = [
            vec![0x80],
            vec![0xc0, 0x80],
            vec![0xe0, 0x80, 0x80],
            vec![0xed, 0xa0, 0x80],
            vec![0xf4, 0x90, 0x80, 0x80],
            vec![0xf5, 0x80, 0x80, 0x80],
            vec![0xf0, 0x9f],
            vec![0xe2, 0x28, 0xa1],
        ];
        for operation in operations {
            for bytes in std::iter::once(None).chain(invalid.iter().map(Some)) {
                let mut args = vec![std::ffi::OsString::from("text.good=valid")];
                if let Some(bytes) = bytes {
                    let mut arg = b"text.bad=".to_vec();
                    arg.extend(bytes);
                    args.push(std::ffi::OsString::from_vec(arg));
                }
                let output = run_os(vec![operation.clone()], &args);
                assert_eq!(output.status.code(), Some(1), "{bytes:?}: {output:?}");
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT"));
            }
        }
    }

    #[test]
    fn native_text_validates_slice_source_then_evaluates_start_before_end() {
        for (args, start, end, error) in [
            (vec![], "1 / 0", "input.missing", "AIL-RUNTIME-TEXT"),
            (
                vec!["text.source=abc"],
                "1 / 0",
                "input.missing",
                "AIL-RUNTIME-DIVZERO",
            ),
            (
                vec!["text.source=abc"],
                "input.missing",
                "1 / 0",
                "AIL-RUNTIME-INT",
            ),
            (
                vec!["text.source=abc"],
                "0",
                "9223372036854775807 + 1",
                "AIL-RUNTIME-OVERFLOW",
            ),
        ] {
            let output = run(
                vec![text_slice("out.result", "text.source", start, end)],
                &args,
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(error),
                "{output:?}"
            );
        }
    }

    #[test]
    fn native_text_enforces_operand_and_join_result_byte_limits() {
        for (size, join, success) in [
            (1_048_576, false, true),
            (1_048_577, false, false),
            (524_288, true, true),
            (524_289, true, false),
        ] {
            let value = "a".repeat(size);
            let mut instructions = vec![instruction(
                "SET_FIELD",
                &[("key", "text.value"), ("value", &value)],
            )];
            if join {
                instructions.push(instruction(
                    "TEXT_JOIN",
                    &[
                        ("key", "text.value"),
                        ("left", "text.value"),
                        ("right", "text.value"),
                    ],
                ));
            }
            instructions.extend([
                instruction(
                    "TEXT_LENGTH",
                    &[("key", "out.length"), ("source", "text.value")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.length")]),
            ]);
            let output = run(instructions, &[]);
            assert_eq!(
                output.status.success(),
                success,
                "size {size}: {:?}",
                output.status
            );
            if success {
                assert_eq!(output.stdout, b"out.length=1048576\n");
            } else {
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT"));
            }
        }
    }

    #[test]
    fn native_text_arena_budget_is_shared_across_calls_and_charges_empty_results() {
        let value = "a".repeat(1_048_576);
        let mut instructions = vec![
            instruction("SET_FIELD", &[("key", "text.seed"), ("value", &value)]),
            instruction("SET_FIELD", &[("key", "text.empty"), ("value", "")]),
            eval("loop.count", "0"),
            instruction("LABEL", &[("name", "again")]),
            instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "2048")]),
            instruction(
                "TEXT_JOIN",
                &[
                    ("key", "text.result"),
                    ("left", "text.seed"),
                    ("right", "text.empty"),
                ],
            ),
            instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "1")]),
            eval("loop.count", "loop.count + 1"),
            branch_int("loop.count", "<", "64", "again"),
            instruction(
                "TEXT_LENGTH",
                &[("key", "out.length"), ("source", "text.result")],
            ),
            instruction(
                "TEXT_FIND",
                &[
                    ("key", "out.position"),
                    ("source", "text.result"),
                    ("needle", "text.seed"),
                ],
            ),
            instruction("OUTPUT_FIELD", &[("key", "out.length")]),
            instruction("OUTPUT_FIELD", &[("key", "out.position")]),
        ];
        let output = run(instructions.clone(), &[]);
        assert!(output.status.success(), "{:?}", output.status);
        assert_eq!(output.stdout, b"out.length=1048576\nout.position=0\n");
        instructions.push(text_slice("text.result", "text.result", "0", "0"));
        let output = run(instructions, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT"));
    }

    #[test]
    fn native_text_find_handles_adversarial_repetitive_inputs_in_linear_time() {
        let _guard = EXECUTION.lock().unwrap();
        for (last, expected) in [('a', -1), ('b', 524_288)] {
            let source = "a".repeat(1_048_575) + &last.to_string();
            let needle = "a".repeat(524_287) + "b";
            let action = AilBytecodeAction {
                name: "Find".into(),
                instructions: vec![
                    instruction("SET_FIELD", &[("key", "text.source"), ("value", &source)]),
                    instruction("SET_FIELD", &[("key", "text.needle"), ("value", &needle)]),
                    instruction(
                        "TEXT_FIND",
                        &[
                            ("key", "out.position"),
                            ("source", "text.source"),
                            ("needle", "text.needle"),
                        ],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.position")]),
                ],
            };
            let binary = emit(&action, &BTreeMap::new()).unwrap();
            let path = std::env::temp_dir().join(format!(
                "ail-text-find-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::write(&path, binary).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            let mut child = Command::new(&path)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while child.try_wait().unwrap().is_none() && std::time::Instant::now() < deadline {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            if child.try_wait().unwrap().is_none() {
                child.kill().unwrap();
                child.wait().unwrap();
                std::fs::remove_file(path).unwrap();
                panic!("legal 1 MiB repetitive text search exceeded three seconds");
            }
            let output = child.wait_with_output().unwrap();
            std::fs::remove_file(path).unwrap();
            assert!(output.status.success());
            assert_eq!(
                output.stdout,
                format!("out.position={expected}\n").as_bytes()
            );
        }
    }

    fn integer_function(
        name: &str,
        parameters: &str,
        body: Vec<AilBytecodeInstruction>,
    ) -> (String, AilBytecodeAction) {
        let mut instructions = vec![instruction(
            "INTEGER_FUNCTION",
            &[("parameters", parameters)],
        )];
        instructions.extend(body);
        (
            name.into(),
            AilBytecodeAction {
                name: name.into(),
                instructions,
            },
        )
    }

    fn call_int(target: &str, arguments: &str, key: &str) -> AilBytecodeInstruction {
        instruction(
            "CALL_INT",
            &[("target", target), ("arguments", arguments), ("key", key)],
        )
    }

    fn return_int(expression: &str) -> AilBytecodeInstruction {
        let expression = IntExpr::parse(expression).unwrap().to_postfix();
        instruction("RETURN_INT", &[("expression", &expression)])
    }

    fn run_functions(
        instructions: Vec<AilBytecodeInstruction>,
        functions: BTreeMap<String, AilBytecodeAction>,
        args: &[&str],
    ) -> Output {
        run_functions_os(
            instructions,
            functions,
            &args
                .iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>(),
        )
    }

    fn run_functions_os(
        instructions: Vec<AilBytecodeInstruction>,
        functions: BTreeMap<String, AilBytecodeAction>,
        args: &[std::ffi::OsString],
    ) -> Output {
        let binary = emit_with_functions(
            &AilBytecodeAction {
                name: "Compute".into(),
                instructions,
            },
            &BTreeMap::new(),
            &functions,
            &BTreeMap::new(),
        )
        .expect("native function compilation");
        let _guard = EXECUTION.lock().unwrap();
        let path = std::env::temp_dir().join(format!(
            "ail-native-function-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, binary).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let output = Command::new(&path).args(args).output().unwrap();
        std::fs::remove_file(path).unwrap();
        output
    }

    fn function_root(target: &str, arguments: &str) -> Vec<AilBytecodeInstruction> {
        vec![
            call_int(target, arguments, "out.result"),
            instruction("OUTPUT_FIELD", &[("key", "out.result")]),
            instruction("RETURN_SUCCESS", &[]),
        ]
    }

    fn value_function(
        name: &str,
        parameters: &str,
        locals: &str,
        result: &str,
        body: Vec<AilBytecodeInstruction>,
    ) -> (String, AilBytecodeAction) {
        let mut instructions = vec![instruction(
            "VALUE_FUNCTION",
            &[
                ("parameters", parameters),
                ("locals", locals),
                ("result", result),
            ],
        )];
        instructions.extend(body);
        (
            name.into(),
            AilBytecodeAction {
                name: name.into(),
                instructions,
            },
        )
    }

    fn call_value(target: &str, arguments: &str, key: &str) -> AilBytecodeInstruction {
        instruction(
            "CALL_VALUE",
            &[("target", target), ("arguments", arguments), ("key", key)],
        )
    }

    fn return_value(kind: &str, expression: &str) -> AilBytecodeInstruction {
        instruction(
            "RETURN_VALUE",
            &[
                ("type", kind),
                (
                    "expression",
                    &IntExpr::parse(expression).unwrap().to_postfix(),
                ),
            ],
        )
    }

    #[test]
    fn native_value_function_text_result_survives_later_calls_and_local_overwrites() {
        let functions = BTreeMap::from([value_function(
            "decorate",
            "text:Text",
            "bang:Text;result:Text",
            "Text",
            vec![
                instruction(
                    "ASSIGN_LITERAL",
                    &[("key", "local.bang"), ("type", "Text"), ("value", "\"!\"")],
                ),
                instruction(
                    "TEXT_JOIN",
                    &[
                        ("key", "local.result"),
                        ("left", "local.text"),
                        ("right", "local.bang"),
                    ],
                ),
                return_value("Text", "local.result"),
            ],
        )]);
        let output = run_functions(
            vec![
                call_value("decorate", "text=f:input.text", "out.first"),
                call_value("decorate", "text=f:out.first", "out.second"),
                instruction(
                    "SET_FIELD",
                    &[("key", "input.text"), ("value", "overwritten")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.first")]),
                instruction("OUTPUT_FIELD", &[("key", "out.second")]),
                instruction("RETURN_SUCCESS", &[]),
            ],
            functions,
            &["input.text=é"],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, "out.first=é!\nout.second=é!!\n".as_bytes());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("local."));
    }

    fn value_root(target: &str, arguments: &str) -> Vec<AilBytecodeInstruction> {
        vec![
            call_value(target, arguments, "out.result"),
            instruction("OUTPUT_FIELD", &[("key", "out.result")]),
            instruction("RETURN_SUCCESS", &[]),
        ]
    }

    #[test]
    fn native_bool_short_circuits_missing_fields_and_checked_arithmetic() {
        let output = run(
            vec![
                instruction(
                    "EVAL_BOOL",
                    &[
                        ("key", "out.first"),
                        (
                            "expression",
                            "b:false if:missing.number i:0 icmp:> and b:true or",
                        ),
                    ],
                ),
                instruction(
                    "EVAL_BOOL",
                    &[
                        ("key", "out.second"),
                        ("expression", "b:true tf:missing.text s:\"x\" tcmp:== or"),
                    ],
                ),
                instruction(
                    "EVAL_BOOL",
                    &[
                        ("key", "out.third"),
                        ("expression", "b:false i:1 i:0 i/ i:0 icmp:== and"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.first")]),
                instruction("OUTPUT_FIELD", &[("key", "out.second")]),
                instruction("OUTPUT_FIELD", &[("key", "out.third")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"out.first=true\nout.second=true\nout.third=false\n"
        );
        assert_eq!(output.stderr, b"computed boolean out.first\ncomputed boolean out.second\ncomputed boolean out.third\n");
    }

    fn bool_eval(expression: &str) -> Vec<AilBytecodeInstruction> {
        vec![
            instruction(
                "EVAL_BOOL",
                &[("key", "out.result"), ("expression", expression)],
            ),
            instruction("OUTPUT_FIELD", &[("key", "out.result")]),
            instruction("RETURN_SUCCESS", &[]),
        ]
    }

    #[test]
    fn native_bool_all_comparisons_preserve_signed_numeric_and_unicode_ordering() {
        for (operator, expected) in [
            ("==", false),
            ("!=", true),
            ("<", true),
            ("<=", true),
            (">", false),
            (">=", false),
        ] {
            for (left, right, prefix) in [
                ("i:-9223372036854775808", "i:9223372036854775807", "i"),
                ("d:-1e300", "d:1e300", "f"),
                ("s:\"z\"", "s:\"é\"", "t"),
                ("b:false", "b:true", "b"),
            ] {
                let expression = format!("{left} {right} {prefix}cmp:{operator}");
                let output = run(bool_eval(&expression), &[]);
                assert!(output.status.success(), "{expression}: {output:?}");
                assert_eq!(output.stdout, format!("out.result={expected}\n").as_bytes());
            }
            let expression = format!("d:nan d:nan fcmp:{operator}");
            let output = run(bool_eval(&expression), &[]);
            assert!(output.status.success(), "{output:?}");
            assert_eq!(
                output.stdout,
                format!("out.result={}\n", operator == "!=").as_bytes()
            );
        }
        for expression in [
            "d:-0 d:0 fcmp:==",
            "d:5e-324 d:0 fcmp:>",
            "i:1 i:2 i+ i:3 i* i:9 icmp:==",
            "d:1 d:2 f+ fneg d:2 f/ d:-1.5 fcmp:==",
            "s:\"\" s:\"a\" tcmp:<",
            "s:\"a\" s:\"ab\" tcmp:<",
            "s:\"é\" s:\"😀\" tcmp:<",
            "s:\"é\" s:\"e\\u0301\" tcmp:!=",
            "s:\"a\\u0000a\" s:\"a\\u0000b\" tcmp:<",
            "b:true b:false or b:true b:true and bcmp:==",
            "b:false not b:true bcmp:==",
            "s:\"and or not (bf:missing.flag)\" s:\"and or not (bf:missing.flag)\" tcmp:==",
        ] {
            let output = run(bool_eval(expression), &[]);
            assert!(output.status.success(), "{expression}: {output:?}");
            assert_eq!(output.stdout, b"out.result=true\n", "{expression}");
        }
    }

    #[test]
    fn native_bool_reached_operands_keep_left_to_right_error_priority() {
        for (expression, family) in [
            ("bf:missing.flag", "BOOL"),
            ("bf:missing.flag i:1 i:0 i/ i:0 icmp:== bcmp:==", "BOOL"),
            ("i:1 i:0 i/ i:0 icmp:== bf:missing.flag bcmp:==", "DIVZERO"),
            ("if:missing.number i:1 i:0 i/ icmp:==", "INT"),
            ("i:1 i:0 i/ if:missing.number icmp:==", "DIVZERO"),
            ("ff:missing.number d:1 d:0 f/ fcmp:==", "FLOAT"),
            ("d:1 d:0 f/ ff:missing.number fcmp:==", "DIVZERO"),
            ("tf:missing.text s:\"a\" tcmp:==", "TEXT"),
            (
                "b:true i:9223372036854775807 i:1 i+ i:0 icmp:> and",
                "OVERFLOW",
            ),
            ("b:false bf:missing.flag or", "BOOL"),
        ] {
            let output = run(bool_eval(expression), &[]);
            assert!(!output.status.success(), "{expression}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:")),
                "{expression}: {output:?}"
            );
        }
        for bad in [
            "", "0", "1", "True", "TRUE", " true", "false ", "false!", "null",
        ] {
            let output = run(bool_eval("bf:input.flag"), &[&format!("input.flag={bad}")]);
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-BOOL:"));
        }
        let large = "x".repeat(1_048_577);
        for (expression, family) in [
            ("tf:input.large tf:missing.text tcmp:==", "TEXT-LIMIT"),
            ("tf:missing.text tf:input.large tcmp:==", "TEXT"),
        ] {
            let mut code = vec![instruction(
                "SET_FIELD",
                &[("key", "input.large"), ("value", &large)],
            )];
            code.extend(bool_eval(expression));
            let output = run(code, &[]);
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:"))
            );
        }
    }

    #[test]
    fn native_bool_skips_native_invalid_utf8_and_oversized_fields_until_reached() {
        use std::os::unix::ffi::OsStringExt;
        for (prefix, success) in [("b:true", true), ("b:false", false)] {
            for (expression, input, family) in [
                (
                    "tf:input.text s:\"x\" tcmp:==",
                    b"input.text=x\xff".as_slice(),
                    "TEXT",
                ),
                ("bf:input.flag", b"input.flag=tr\xffe".as_slice(), "BOOL"),
                (
                    "ff:input.float d:0 fcmp:==",
                    b"input.float=\xff".as_slice(),
                    "FLOAT",
                ),
            ] {
                let output = run_os(
                    bool_eval(&format!("{prefix} {expression} or")),
                    &[std::ffi::OsString::from_vec(input.to_vec())],
                );
                assert_eq!(output.status.success(), success, "{expression}: {output:?}");
                if success {
                    assert_eq!(output.stdout, b"out.result=true\n");
                } else {
                    assert!(output.stdout.is_empty());
                    assert!(
                        String::from_utf8_lossy(&output.stderr)
                            .contains(&format!("AIL-RUNTIME-{family}:"))
                    );
                }
            }
        }
        let huge = "x".repeat(1_048_577);
        let mut code = vec![instruction(
            "SET_FIELD",
            &[("key", "input.large"), ("value", &huge)],
        )];
        code.extend(bool_eval(
            "b:false tf:input.large tf:missing.text tcmp:== and not",
        ));
        let output = run(code, &[]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.result=true\n");
    }

    #[test]
    fn native_bool_branches_loop_without_implicit_traces_and_preserve_fuel() {
        let output = run(
            vec![
                eval("data.count", "0"),
                instruction("LABEL", &[("name", "again")]),
                eval("data.count", "data.count + 1"),
                instruction(
                    "BRANCH_BOOL",
                    &[
                        ("expression", "if:data.count i:3 icmp:<"),
                        ("label", "again"),
                    ],
                ),
                instruction(
                    "EVAL_BOOL",
                    &[
                        ("key", "out.result"),
                        ("expression", "if:data.count i:3 icmp:=="),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.result")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.result=true\n");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("branch"));
        let output = run(
            vec![
                instruction("LABEL", &[("name", "again")]),
                instruction(
                    "BRANCH_BOOL",
                    &[("expression", "b:true"), ("label", "again")],
                ),
            ],
            &[],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("step limit"));
    }

    #[test]
    fn native_bool_aliases_read_old_transport_and_keep_prior_copies() {
        let output = run(
            vec![
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.original"), ("source", "data.flag")],
                ),
                instruction(
                    "EVAL_BOOL",
                    &[("key", "data.flag"), ("expression", "bf:data.flag not")],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.saved"), ("source", "data.flag")],
                ),
                instruction(
                    "EVAL_BOOL",
                    &[("key", "data.flag"), ("expression", "bf:data.flag not")],
                ),
                instruction("SET_FIELD", &[("key", "data.text"), ("value", "a\0b")]),
                instruction(
                    "EVAL_BOOL",
                    &[
                        ("key", "data.text"),
                        ("expression", "tf:data.text s:\"a\\u0000b\" tcmp:=="),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.original")]),
                instruction("OUTPUT_FIELD", &[("key", "out.saved")]),
                instruction("OUTPUT_FIELD", &[("key", "data.flag")]),
                instruction("OUTPUT_FIELD", &[("key", "data.text")]),
            ],
            &["data.flag=true"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"data.flag=true\ndata.text=true\nout.original=true\nout.saved=false\n"
        );
    }

    #[test]
    fn native_bool_functions_link_body_only_float_dependencies_and_isolate_locals() {
        let functions = BTreeMap::from([
            value_function(
                "numeric",
                "n:Int",
                "result:Bool",
                "Bool",
                vec![
                    instruction(
                        "EVAL_BOOL",
                        &[
                            ("key", "local.result"),
                            (
                                "expression",
                                "if:local.n i:0 icmp:> d:nan d:nan fcmp:!= and",
                            ),
                        ],
                    ),
                    return_value("Bool", "local.result"),
                ],
            ),
            value_function(
                "lazy",
                "",
                "missing:Bool;result:Bool",
                "Bool",
                vec![
                    instruction(
                        "EVAL_BOOL",
                        &[
                            ("key", "local.result"),
                            ("expression", "b:true bf:local.missing or"),
                        ],
                    ),
                    return_value("Bool", "local.result"),
                ],
            ),
            value_function(
                "recursive",
                "n:Int",
                "child:Bool;result:Bool",
                "Bool",
                vec![
                    instruction(
                        "BRANCH_BOOL",
                        &[("expression", "if:local.n i:0 icmp:=="), ("label", "base")],
                    ),
                    call_value("recursive", "n=f:local.n i:1 -", "local.child"),
                    instruction(
                        "EVAL_BOOL",
                        &[
                            ("key", "local.result"),
                            ("expression", "bf:local.child s:\"é\" s:\"😀\" tcmp:< and"),
                        ],
                    ),
                    return_value("Bool", "local.result"),
                    instruction("LABEL", &[("name", "base")]),
                    instruction(
                        "EVAL_BOOL",
                        &[("key", "local.result"), ("expression", "b:true")],
                    ),
                    return_value("Bool", "local.result"),
                ],
            ),
        ]);
        let output = run_functions(
            vec![
                call_value("numeric", "n=i:7", "out.numeric"),
                call_value("lazy", "", "out.lazy"),
                call_value("recursive", "n=i:63", "out.recursive"),
                instruction("OUTPUT_FIELD", &[("key", "out.numeric")]),
                instruction("OUTPUT_FIELD", &[("key", "out.lazy")]),
                instruction("OUTPUT_FIELD", &[("key", "out.recursive")]),
                instruction("RETURN_SUCCESS", &[]),
            ],
            functions.clone(),
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"out.lazy=true\nout.numeric=true\nout.recursive=true\n"
        );
        assert!(output.stderr.is_empty());
        let output = run_functions(value_root("recursive", "n=i:64"), functions, &[]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-DEPTH:"));
        let plain = BTreeMap::from([value_function(
            "plain",
            "",
            "result:Bool",
            "Bool",
            vec![
                instruction(
                    "EVAL_BOOL",
                    &[
                        ("key", "local.result"),
                        (
                            "expression",
                            "s:\"floating d:nan\" s:\"floating d:nan\" tcmp:==",
                        ),
                    ],
                ),
                return_value("Bool", "local.result"),
            ],
        )]);
        let binary = emit_with_functions(
            &AilBytecodeAction {
                name: "Root".into(),
                instructions: value_root("plain", ""),
            },
            &BTreeMap::new(),
            &plain,
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(u16::from_le_bytes([binary[56], binary[57]]), 1);
    }

    #[test]
    fn native_bool_function_short_circuit_does_not_skip_parameter_validation() {
        for (kind, family, argument) in [
            ("Bool", "JSON", "input.value=invalid"),
            ("Text", "TEXT", ""),
        ] {
            let functions = BTreeMap::from([value_function(
                "unused",
                &format!("a:{kind};z:Int"),
                "result:Bool",
                "Bool",
                vec![
                    instruction(
                        "EVAL_BOOL",
                        &[("key", "local.result"), ("expression", "b:true")],
                    ),
                    return_value("Bool", "local.result"),
                ],
            )]);
            let output = run_functions(
                value_root("unused", "a=f:input.value;z=i:1 i:0 /"),
                functions,
                &[argument],
            );
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:")),
                "{output:?}"
            );
        }
    }

    #[test]
    fn native_bool_results_and_comparisons_consume_neither_arena() {
        let text = "x".repeat(1_048_576);
        let list = format!("[\"{}\"]", "y".repeat(1_048_572));
        let functions = BTreeMap::from([value_function(
            "compare",
            "text:Text",
            "result:Bool",
            "Bool",
            vec![
                instruction(
                    "EVAL_BOOL",
                    &[
                        ("key", "local.result"),
                        ("expression", "tf:local.text tf:local.text tcmp:=="),
                    ],
                ),
                return_value("Bool", "local.result"),
            ],
        )]);
        let mut code = vec![
            instruction("SET_FIELD", &[("key", "input.text"), ("value", &text)]),
            instruction("SET_FIELD", &[("key", "input.list"), ("value", &list)]),
            instruction("SET_FIELD", &[("key", "input.empty"), ("value", "")]),
        ];
        for _ in 0..64 {
            code.push(instruction(
                "TEXT_JOIN",
                &[
                    ("key", "data.text"),
                    ("left", "input.text"),
                    ("right", "input.empty"),
                ],
            ));
            code.push(instruction(
                "JSON_PARSE",
                &[
                    ("key", "data.list"),
                    ("source", "input.list"),
                    ("type", "List<Text>"),
                ],
            ));
        }
        code.push(call_value("compare", "text=f:input.text", "out.called"));
        code.push(instruction(
            "EVAL_BOOL",
            &[
                ("key", "out.result"),
                (
                    "expression",
                    "tf:data.text tf:input.text tcmp:== bf:out.called and",
                ),
            ],
        ));
        code.push(instruction("OUTPUT_FIELD", &[("key", "out.result")]));
        code.push(instruction("RETURN_SUCCESS", &[]));
        let output = run_functions(code, functions, &[]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"out.result=true\n");
    }

    #[test]
    fn native_bool_rejects_malformed_skipped_trees_and_missing_branch_targets() {
        for expression in [
            "b:false i:1 and".to_string(),
            "b:true ff:bad d:0 fcmp:== or".to_string(),
            "b:true s:\"\\ud800\" s:\"x\" tcmp:== or".to_string(),
            "b:true b:false bcmp:=".to_string(),
            "b:false bf:data.same if:data.same i:0 icmp:== and and".to_string(),
            format!("{}b:true", " ".repeat(65_536)),
            format!("b:true{}", " not".repeat(128)),
        ] {
            let result = emit(
                &AilBytecodeAction {
                    name: "Invalid".into(),
                    instructions: bool_eval(&expression),
                },
                &BTreeMap::new(),
            );
            assert!(result.is_err(), "{expression}");
        }
        let result = emit(
            &AilBytecodeAction {
                name: "Invalid".into(),
                instructions: vec![instruction(
                    "BRANCH_BOOL",
                    &[("expression", "b:false"), ("label", "missing")],
                )],
            },
            &BTreeMap::new(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn native_float_arithmetic_uses_canonical_scientific_transport() {
        let output = run(
            vec![
                instruction(
                    "EVAL_FLOAT",
                    &[("key", "out.value"), ("expression", "d:1.5 d:2 + d:4 /")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                instruction("RETURN_SUCCESS", &[]),
            ],
            &[],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"out.value=8.75e-1\n");
    }

    fn float_eval(key: &str, expression: &str) -> AilBytecodeInstruction {
        instruction("EVAL_FLOAT", &[("key", key), ("expression", expression)])
    }
    fn float_root(expression: &str) -> Vec<AilBytecodeInstruction> {
        vec![
            float_eval("out.value", expression),
            instruction("OUTPUT_FIELD", &[("key", "out.value")]),
            instruction("RETURN_SUCCESS", &[]),
        ]
    }

    #[test]
    fn native_float_transport_rounds_ties_subnormals_and_preserves_signed_zero() {
        for (input, expected) in [
            (
                "1.00000000000000011102230246251565404236316680908203125",
                "1e0",
            ),
            (
                "1.00000000000000011102230246251565404236316680908203126",
                "1.0000000000000002e0",
            ),
            ("5e-324", "5e-324"),
            ("1e-400", "0e0"),
            ("-1e-400", "-0e0"),
            ("1.7976931348623157e308", "1.7976931348623157e308"),
            ("1e400", "inf"),
            ("-0", "-0e0"),
            ("+001.50", "1.5e0"),
            (".125", "1.25e-1"),
            ("-nan", "nan"),
            ("-inf", "-inf"),
        ] {
            let output = run(
                float_root("f:input.value"),
                &[&format!("input.value={input}")],
            );
            assert!(
                output.status.success(),
                "{input}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                output.stdout,
                format!("out.value={expected}\n").as_bytes(),
                "{input}"
            );
        }
    }

    #[test]
    fn native_float_arithmetic_modulo_negation_and_nonfinite_values_match_binary64() {
        for (expression, expected) in [
            ("d:5 d:-2 %", "-1e0"),
            ("d:-5 d:2 %", "1e0"),
            ("d:4 d:-2 %", "-0e0"),
            ("d:1 d:-inf %", "-inf"),
            ("d:-1 d:inf %", "inf"),
            ("d:inf d:2 %", "nan"),
            ("d:0 neg", "-0e0"),
            ("d:-0 neg", "0e0"),
            ("d:nan neg", "nan"),
            ("d:inf d:inf -", "nan"),
            ("d:1e308 d:2 *", "inf"),
            ("d:7 d:2 / d:2 d:3 + *", "1.75e1"),
        ] {
            let output = run(float_root(expression), &[]);
            assert!(
                output.status.success(),
                "{expression}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                output.stdout,
                format!("out.value={expected}\n").as_bytes(),
                "{expression}"
            );
        }
        for expression in ["d:1 d:0 /", "d:nan d:-0 %", "d:inf d:0 %"] {
            let output = run(float_root(expression), &[]);
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-DIVZERO"));
        }
    }

    #[test]
    fn native_float_branches_follow_unordered_rules_and_preserve_loop_fuel() {
        for (comparison, expected) in [
            ("==", 0),
            ("!=", 1),
            ("<", 0),
            ("<=", 0),
            (">", 0),
            (">=", 0),
        ] {
            let output = run(
                vec![
                    instruction("EVAL_INT", &[("key", "out.result"), ("expression", "i:0")]),
                    instruction(
                        "BRANCH_FLOAT",
                        &[
                            ("left", "d:nan"),
                            ("comparison", comparison),
                            ("right", "d:1"),
                            ("label", "yes"),
                        ],
                    ),
                    instruction("JUMP", &[("label", "end")]),
                    instruction("LABEL", &[("name", "yes")]),
                    instruction("EVAL_INT", &[("key", "out.result"), ("expression", "i:1")]),
                    instruction("LABEL", &[("name", "end")]),
                    instruction("OUTPUT_FIELD", &[("key", "out.result")]),
                    instruction("RETURN_SUCCESS", &[]),
                ],
                &[],
            );
            assert!(output.status.success());
            assert_eq!(output.stdout, format!("out.result={expected}\n").as_bytes());
        }
        let output = run(
            vec![
                float_eval("out.value", "d:0"),
                instruction("LABEL", &[("name", "again")]),
                float_eval("out.value", "f:out.value d:0.5 +"),
                instruction(
                    "BRANCH_FLOAT",
                    &[
                        ("left", "f:out.value"),
                        ("comparison", "<"),
                        ("right", "d:2"),
                        ("label", "again"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                instruction("RETURN_SUCCESS", &[]),
            ],
            &[],
        );
        assert!(output.status.success());
        assert_eq!(output.stdout, b"out.value=2e0\n");
    }

    #[test]
    fn native_float_conversions_check_signed_i64_boundaries_before_truncating() {
        for (input, expected) in [
            ("-9223372036854775808", Some("-9223372036854775808")),
            ("9223372036854774784", Some("9223372036854774784")),
            ("-3.9", Some("-3")),
            ("-0", Some("0")),
            ("9223372036854775808", None),
            ("nan", None),
            ("inf", None),
        ] {
            let output = run(
                vec![
                    instruction(
                        "FLOAT_TO_INT",
                        &[("source", "input.value"), ("key", "out.value")],
                    ),
                    instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                    instruction("RETURN_SUCCESS", &[]),
                ],
                &[&format!("input.value={input}")],
            );
            if let Some(expected) = expected {
                assert!(output.status.success());
                assert_eq!(output.stdout, format!("out.value={expected}\n").as_bytes());
            } else {
                assert!(!output.status.success());
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-FLOAT:"));
            }
        }
        let output = run(
            vec![
                instruction(
                    "INT_TO_FLOAT",
                    &[("source", "input.value"), ("key", "out.value")],
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.value")]),
                instruction("RETURN_SUCCESS", &[]),
            ],
            &["input.value=9223372036854775807"],
        );
        assert!(output.status.success());
        assert_eq!(output.stdout, b"out.value=9.223372036854776e18\n");
        let output = run(
            vec![instruction(
                "INT_TO_FLOAT",
                &[("source", "input.value"), ("key", "out.value")],
            )],
            &["input.value=1.5"],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-INT:"));
    }

    #[test]
    fn native_float_copies_own_zero_and_long_numeric_buffers() {
        for expression in ["d:0", "d:-0", "d:1.7976931348623157e308"] {
            let mut root = vec![
                float_eval("data.value", expression),
                instruction(
                    "COPY_FIELD",
                    &[("key", "out.copy"), ("source", "data.value")],
                ),
                float_eval("data.value", "d:9"),
                instruction("OUTPUT_FIELD", &[("key", "out.copy")]),
                instruction("RETURN_SUCCESS", &[]),
            ];
            let output = run(std::mem::take(&mut root), &[]);
            let expected = match expression {
                "d:0" => "0e0",
                "d:-0" => "-0e0",
                _ => "1.7976931348623157e308",
            };
            assert!(output.status.success());
            assert_eq!(output.stdout, format!("out.copy={expected}\n").as_bytes());
        }
    }

    #[test]
    fn native_float_rejects_missing_malformed_invalid_utf8_and_oversize_inputs() {
        use std::os::unix::ffi::OsStringExt;
        for input in [
            "", " 1", "1 ", "1_0", "0x1", "NaN", "infinity", "1e", "1.2junk",
        ] {
            let output = run(
                float_root("f:input.value"),
                &[&format!("input.value={input}")],
            );
            assert!(!output.status.success(), "{input:?}");
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-FLOAT:"));
        }
        let output = run_os(
            float_root("f:input.value"),
            &[std::ffi::OsString::from_vec(b"input.value=\xff".to_vec())],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-FLOAT:"));
        let mut root = vec![instruction(
            "SET_FIELD",
            &[("key", "input.value"), ("value", &"0".repeat(1_048_577))],
        )];
        root.extend(float_root("f:input.value"));
        let output = run(root, &[]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-FLOAT-LIMIT:"));
    }

    #[test]
    fn native_float_functions_keep_scalar_packets_isolated_and_emit_body_only_helpers() {
        let functions = BTreeMap::from([value_function(
            "sum",
            "n:Int;value:Float",
            "a:Float;b:Float;result:Float",
            "Float",
            vec![
                branch_int("local.n", "==", "0", "base"),
                call_value("sum", "n=f:local.n i:1 -;value=f:local.value", "local.a"),
                float_eval("local.value", "f:local.value d:0.5 +"),
                call_value("sum", "n=i:0;value=f:local.value", "local.b"),
                float_eval("local.result", "f:local.a f:local.b +"),
                return_value("Float", "local.result"),
                instruction("LABEL", &[("name", "base")]),
                return_value("Float", "local.value"),
            ],
        )]);
        let output = run_functions(
            value_root("sum", "n=i:3;value=f:input.value"),
            functions,
            &["input.value=1.5"],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"out.result=7.5e0\n");
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn native_float_json_bridges_are_finite_strict_and_preserve_lossless_json() {
        let output = run(
            vec![
                instruction(
                    "JSON_PARSE",
                    &[
                        ("type", "Float"),
                        ("source", "input.number"),
                        ("key", "out.value"),
                    ],
                ),
                instruction(
                    "JSON_FORMAT",
                    &[
                        ("type", "Float"),
                        ("source", "out.value"),
                        ("key", "out.text"),
                    ],
                ),
                json_set("out.json", "input.json", "input.path", "out.value", "Float"),
                json_get("out.read", "out.json", "input.path", "Float"),
                instruction("OUTPUT_FIELD", &[("key", "out.json")]),
                instruction("OUTPUT_FIELD", &[("key", "out.read")]),
                instruction("OUTPUT_FIELD", &[("key", "out.text")]),
                instruction("RETURN_SUCCESS", &[]),
            ],
            &[
                "input.number= -0 ",
                "input.json={\"other\":1e400,\"value\":null}",
                "input.path=/value",
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            b"out.json={\"other\":1e400,\"value\":-0e0}\nout.read=-0e0\nout.text=-0e0\n"
        );
        for input in ["1e400", "nan", "+1", "01", "1.", "\"1\"", "null"] {
            let output = run(
                vec![
                    instruction(
                        "JSON_PARSE",
                        &[
                            ("type", "Float"),
                            ("source", "input.value"),
                            ("key", "out.value"),
                        ],
                    ),
                    instruction("RETURN_SUCCESS", &[]),
                ],
                &[&format!("input.value={input}")],
            );
            assert!(!output.status.success(), "{input}");
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"));
        }
        for op in ["JSON_FORMAT", "JSON_SET"] {
            let mut root = if op == "JSON_FORMAT" {
                vec![instruction(
                    op,
                    &[
                        ("type", "Float"),
                        ("source", "input.value"),
                        ("key", "out.value"),
                    ],
                )]
            } else {
                vec![json_set(
                    "out.value",
                    "input.json",
                    "input.path",
                    "input.value",
                    "Float",
                )]
            };
            root.push(instruction("RETURN_SUCCESS", &[]));
            let output = run(
                root,
                &[
                    "input.value=inf",
                    "input.json={\"value\":0}",
                    "input.path=/value",
                ],
            );
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"));
        }
    }

    #[test]
    fn native_float_seeded_finite_arithmetic_roundtrips_expected_result_bits() {
        let mut seed = 0x1248_369a_bcde_f057_u64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            f64::from_bits(seed & 0xffef_ffff_ffff_ffff)
        };
        let mut root = Vec::new();
        let mut expected = Vec::new();
        for n in 0..400 {
            let left = next();
            let right = next();
            let (operator, value) = match n % 4 {
                0 => ("+", left + right),
                1 => ("-", left - right),
                2 => ("*", left * right),
                _ => ("/", left / right),
            };
            let key = format!("out.v{n:03}");
            root.push(float_eval(
                &key,
                &format!("d:{left:e} d:{right:e} {operator}"),
            ));
            root.push(instruction("OUTPUT_FIELD", &[("key", &key)]));
            expected.push(value.to_bits());
        }
        root.push(instruction("RETURN_SUCCESS", &[]));
        let output = run(root, &[]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        for (line, bits) in text.lines().zip(&expected) {
            let value = line.split_once('=').unwrap().1.parse::<f64>().unwrap();
            assert_eq!(value.to_bits(), *bits, "{line}");
        }
        assert_eq!(text.lines().count(), expected.len());
    }

    #[test]
    fn native_float_function_parameters_validate_unused_values_before_later_arguments() {
        let functions = BTreeMap::from([value_function(
            "unused",
            "a:Float;z:Int",
            "",
            "Int",
            vec![return_value("Int", "7")],
        )]);
        for args in [vec![], vec!["input.value= 1"]] {
            let output = run_functions(
                value_root("unused", "a=f:input.value;z=i:1 i:0 /"),
                functions.clone(),
                &args,
            );
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-FLOAT:"));
            assert!(!String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-DIVZERO"));
        }
    }

    #[test]
    fn native_float_uses_no_arena_and_json_format_charges_the_shared_text_arena() {
        let functions = BTreeMap::from([value_function(
            "identity",
            "value:Float",
            "",
            "Float",
            vec![return_value("Float", "local.value")],
        )]);
        for format in [false, true] {
            let mut root = vec![
                instruction(
                    "SET_FIELD",
                    &[("key", "data.large"), ("value", &"x".repeat(1_048_576))],
                ),
                instruction("SET_FIELD", &[("key", "data.empty"), ("value", "")]),
            ];
            root.extend((0..64).map(|_| {
                instruction(
                    "TEXT_JOIN",
                    &[
                        ("left", "data.large"),
                        ("right", "data.empty"),
                        ("key", "data.copy"),
                    ],
                )
            }));
            root.push(float_eval("data.value", "d:0.5"));
            root.push(call_value("identity", "value=f:data.value", "data.value"));
            if format {
                root.push(instruction(
                    "JSON_FORMAT",
                    &[
                        ("source", "data.value"),
                        ("key", "out.text"),
                        ("type", "Float"),
                    ],
                ));
            }
            root.push(instruction("OUTPUT_FIELD", &[("key", "data.value")]));
            root.push(instruction("RETURN_SUCCESS", &[]));
            let output = run_functions(root, functions.clone(), &[]);
            assert_eq!(output.status.success(), !format);
            if format {
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT:")
                );
            } else {
                assert_eq!(output.stdout, b"data.value=5e-1\n");
            }
        }
    }

    #[test]
    fn native_float_json_preserves_source_path_value_error_order_and_limits() {
        let root = vec![
            json_set(
                "out.value",
                "input.json",
                "input.path",
                "input.value",
                "Float",
            ),
            instruction("RETURN_SUCCESS", &[]),
        ];
        for (source, path, value, family) in [
            ("{bad", "~bad", "nan", "JSON:"),
            ("{}", "~bad", "nan", "JSON-PATH:"),
            ("{}", "/missing/leaf", "nan", "JSON-PATH:"),
            ("{}", "/value", "nan", "JSON:"),
        ] {
            let output = run(
                root.clone(),
                &[
                    &format!("input.json={source}"),
                    &format!("input.path={path}"),
                    &format!("input.value={value}"),
                ],
            );
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}"))
            );
        }
        let huge = format!("{{\"x\":\"{}\"}}", "\u{7f}".repeat(200_000));
        let mut code = vec![instruction(
            "SET_FIELD",
            &[("key", "input.json"), ("value", &huge)],
        )];
        code.extend(root);
        let output = run(code, &["input.path=bad", "input.value=nan"]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT:"));
        // The entire source must fit canonical bounds before a wrong root type
        // is reported. Raw DEL expands from one byte to six during encoding.
        let expanded_string = format!("\"{}\"", "\u{7f}".repeat(200_000));
        let output = run(
            vec![
                instruction(
                    "SET_FIELD",
                    &[("key", "input.raw"), ("value", &expanded_string)],
                ),
                instruction(
                    "JSON_PARSE",
                    &[
                        ("source", "input.raw"),
                        ("key", "out.value"),
                        ("type", "Float"),
                    ],
                ),
            ],
            &[],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT:"));
        let mut code = vec![instruction(
            "SET_FIELD",
            &[
                ("key", "input.raw"),
                ("value", &format!("{}1", " ".repeat(1_048_576))),
            ],
        )];
        code.push(instruction(
            "JSON_PARSE",
            &[
                ("source", "input.raw"),
                ("key", "out.value"),
                ("type", "Float"),
            ],
        ));
        let output = run(code, &[]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON-LIMIT:"));
    }

    #[test]
    fn native_value_functions_forward_all_six_types_with_canonical_returns() {
        for (kind, input, expected) in [
            ("Int", "+001", "1"),
            ("Bool", "false", "false"),
            ("Text", "é\0\n", "é\0\n"),
            (
                "List<Int>",
                " [ -0, 9223372036854775807 ] ",
                "[0,9223372036854775807]",
            ),
            (
                "List<Text>",
                "[\"\\u00e9\",\"\\n\",\"\\u007f\"]",
                "[\"é\",\"\\n\",\"\\u007f\"]",
            ),
            (
                "Json",
                " {\"x\":1e+09,\"x\": [true,null,\"\\u00e9\"]} ",
                "{\"x\":1e+09,\"x\":[true,null,\"é\"]}",
            ),
        ] {
            let functions = BTreeMap::from([value_function(
                "identity",
                &format!("value:{kind}"),
                "",
                kind,
                vec![return_value(kind, "local.value")],
            )]);
            let mut root = vec![instruction(
                "SET_FIELD",
                &[("key", "in.value"), ("value", input)],
            )];
            root.extend(value_root("identity", "value=f:in.value"));
            let output = run_functions(root, functions, &[]);
            assert!(
                output.status.success(),
                "{kind}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                output.stdout,
                format!("out.result={expected}\n").as_bytes(),
                "{kind}"
            );
        }
    }

    #[test]
    fn native_value_functions_validate_unused_parameters_in_sorted_order() {
        for (kind, value, family) in [
            ("Bool", " true", "JSON"),
            ("List<Int>", "[1,true]", "LIST"),
            ("List<Text>", "[\"ok\",1]", "LIST"),
            ("Json", "{\"a\":1,}", "JSON"),
            ("Int", "oops", "INT"),
        ] {
            let functions = BTreeMap::from([value_function(
                "unused",
                &format!("a:{kind};z:Int"),
                "",
                "Int",
                vec![return_value("Int", "7")],
            )]);
            let output = run_functions(
                value_root("unused", "a=f:input.a;z=i:1 i:0 /"),
                functions,
                &[&format!("input.a={value}")],
            );
            assert!(!output.status.success(), "{kind}");
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:")),
                "{kind}: {:?}",
                output.stderr
            );
        }
        for (kind, family) in [
            ("Text", "TEXT"),
            ("Bool", "JSON"),
            ("List<Int>", "LIST"),
            ("List<Text>", "LIST"),
            ("Json", "JSON"),
        ] {
            let functions = BTreeMap::from([value_function(
                "unused",
                &format!("value:{kind}"),
                "",
                "Int",
                vec![return_value("Int", "7")],
            )]);
            let output = run_functions(
                value_root("unused", "value=f:input.missing"),
                functions,
                &[],
            );
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:")),
                "{kind}"
            );
        }
    }

    #[test]
    fn native_value_functions_call_integer_helpers_and_share_depth_counter() {
        let functions = BTreeMap::from([
            integer_function("one", "n", vec![return_int("local.n + 1")]),
            value_function(
                "count",
                "n:Int",
                "result:Int",
                "Int",
                vec![
                    branch_int("local.n", "==", "0", "base"),
                    call_value("count", "n=f:local.n i:1 -", "local.result"),
                    return_value("Int", "local.result + 1"),
                    instruction("LABEL", &[("name", "base")]),
                    call_int("one", "n=f:local.n", "local.result"),
                    return_value("Int", "local.result"),
                ],
            ),
        ]);
        let output = run_functions(value_root("count", "n=i:62"), functions.clone(), &[]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"out.result=63\n");
        let output = run_functions(value_root("count", "n=i:63"), functions, &[]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-DEPTH"));
    }

    #[test]
    fn native_value_functions_mutual_recursion_preserves_multiple_local_results() {
        let functions = BTreeMap::from([
            value_function(
                "fib",
                "n:Int",
                "a:Int;b:Int",
                "Int",
                vec![
                    branch_int("local.n", "<=", "1", "base"),
                    call_value("other", "n=f:local.n i:1 -", "local.a"),
                    call_value("other", "n=f:local.n i:2 -", "local.b"),
                    return_value("Int", "local.a + local.b"),
                    instruction("LABEL", &[("name", "base")]),
                    return_value("Int", "local.n"),
                ],
            ),
            value_function(
                "other",
                "n:Int",
                "zzz:Int",
                "Int",
                vec![
                    call_value("fib", "n=f:local.n", "local.zzz"),
                    return_value("Int", "local.zzz"),
                ],
            ),
        ]);
        let output = run_functions(value_root("fib", "n=i:10"), functions, &["local.n=999"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"out.result=55\n");
    }

    #[test]
    fn native_value_functions_emit_json_helpers_and_only_explicit_function_traces() {
        let functions = BTreeMap::from([value_function(
            "transform",
            "doc:Json;flag:Bool;n:Int;names:List<Text>;text:Text",
            "path:Text;result:Json;unused:Int",
            "Json",
            vec![
                instruction(
                    "ASSIGN_LITERAL",
                    &[
                        ("key", "local.path"),
                        ("type", "Text"),
                        ("value", "\"/names\""),
                    ],
                ),
                instruction(
                    "JSON_SET",
                    &[
                        ("key", "local.result"),
                        ("source", "local.doc"),
                        ("path", "local.path"),
                        ("type", "List<Text>"),
                        ("value", "local.names"),
                    ],
                ),
                instruction(
                    "ASSIGN_LITERAL",
                    &[
                        ("key", "local.path"),
                        ("type", "Text"),
                        ("value", "\"/flag\""),
                    ],
                ),
                instruction(
                    "JSON_SET",
                    &[
                        ("key", "local.result"),
                        ("source", "local.result"),
                        ("path", "local.path"),
                        ("type", "Bool"),
                        ("value", "local.flag"),
                    ],
                ),
                instruction("EMIT_TRACE", &[("event", "inside")]),
                return_value("Json", "local.result"),
                instruction(
                    "EVAL_INT",
                    &[("key", "local.unused"), ("expression", "i:1 i:0 /")],
                ),
            ],
        )]);
        let output = run_functions(
            value_root(
                "transform",
                "doc=f:input.doc;flag=f:input.flag;n=i:9;names=f:input.names;text=f:input.text",
            ),
            functions,
            &[
                "input.doc={\"names\":null,\"flag\":null}",
                "input.flag=true",
                "input.names=[\"é\"]",
                "input.text=unused",
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            "out.result={\"names\":[\"é\"],\"flag\":true}\n".as_bytes()
        );
        assert_eq!(output.stderr, b"trace inside\n");
    }

    #[test]
    fn native_value_functions_missing_returns_locals_and_fuel_use_runtime_errors() {
        for (locals, result, body, family) in [
            ("", "Int", vec![], "AIL-RUNTIME-RETURN"),
            (
                "missing:Text",
                "Text",
                vec![return_value("Text", "local.missing")],
                "AIL-RUNTIME-TEXT",
            ),
            (
                "missing:Bool",
                "Bool",
                vec![return_value("Bool", "local.missing")],
                "AIL-RUNTIME-JSON",
            ),
            (
                "missing:List<Text>",
                "List<Text>",
                vec![return_value("List<Text>", "local.missing")],
                "AIL-RUNTIME-LIST",
            ),
            (
                "missing:Json",
                "Json",
                vec![return_value("Json", "local.missing")],
                "AIL-RUNTIME-JSON",
            ),
            (
                "",
                "Int",
                vec![
                    instruction("LABEL", &[("name", "again")]),
                    instruction("JUMP", &[("label", "again")]),
                ],
                "step limit",
            ),
        ] {
            let output = run_functions(
                value_root("bad", ""),
                BTreeMap::from([value_function("bad", "", locals, result, body)]),
                &[],
            );
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(family),
                "{family}: {:?}",
                output.stderr
            );
        }
    }

    #[test]
    fn native_value_functions_list_parameters_allow_canonical_expansion_until_return() {
        let input = format!("[\"{}\"]", "\u{7f}".repeat(200_000));
        let functions = BTreeMap::from([
            value_function(
                "size",
                "items:List<Text>",
                "n:Int",
                "Int",
                vec![
                    instruction(
                        "LIST_LENGTH",
                        &[
                            ("key", "local.n"),
                            ("source", "local.items"),
                            ("type", "Text"),
                        ],
                    ),
                    return_value("Int", "local.n"),
                ],
            ),
            value_function(
                "identity",
                "items:List<Text>",
                "",
                "List<Text>",
                vec![return_value("List<Text>", "local.items")],
            ),
        ]);
        for (target, success) in [("size", true), ("identity", false)] {
            let mut root = vec![instruction(
                "SET_FIELD",
                &[("key", "input.items"), ("value", &input)],
            )];
            root.extend(value_root(target, "items=f:input.items"));
            let output = run_functions(root, functions.clone(), &[]);
            assert_eq!(output.status.success(), success);
            if success {
                assert_eq!(output.stdout, b"out.result=1\n");
            } else {
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-LIST-LIMIT"));
            }
        }
    }

    #[test]
    fn native_value_functions_borrow_parameters_and_charge_one_text_return_copy() {
        let text = "x".repeat(1_048_576);
        let functions = BTreeMap::from([
            value_function(
                "identity",
                "text:Text",
                "",
                "Text",
                vec![return_value("Text", "local.text")],
            ),
            value_function(
                "unused",
                "text:Text",
                "",
                "Int",
                vec![return_value("Int", "7")],
            ),
        ]);
        for (target, calls, success) in [
            ("unused", 70, true),
            ("identity", 64, true),
            ("identity", 65, false),
        ] {
            let mut root = vec![instruction(
                "ASSIGN_LITERAL",
                &[
                    ("key", "input.text"),
                    ("type", "Text"),
                    ("value", &format!("\"{text}\"")),
                ],
            )];
            root.extend((0..calls).map(|_| call_value(target, "text=f:input.text", "out.value")));
            root.push(instruction(
                "EVAL_INT",
                &[("key", "out.result"), ("expression", "i:1")],
            ));
            root.push(instruction("OUTPUT_FIELD", &[("key", "out.result")]));
            root.push(instruction("RETURN_SUCCESS", &[]));
            let output = run_functions(root, functions.clone(), &[]);
            assert_eq!(
                output.status.success(),
                success,
                "{target}/{calls}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            if success {
                assert_eq!(output.stdout, b"out.result=1\n");
            } else {
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT"));
            }
        }
    }

    #[test]
    fn native_value_functions_reject_invalid_utf8_and_parameter_limits_before_entry() {
        use std::os::unix::ffi::OsStringExt;
        for (kind, family) in [
            ("Text", "TEXT"),
            ("Bool", "JSON"),
            ("List<Text>", "LIST"),
            ("Json", "JSON"),
        ] {
            let function = value_function(
                "unused",
                &format!("data:{kind}"),
                "",
                "Int",
                vec![return_value("Int", "0")],
            );
            let output = run_functions_os(
                value_root("unused", "data=f:input.data"),
                BTreeMap::from([function]),
                &[std::ffi::OsString::from_vec(
                    b"input.data=\xed\xa0\x80".to_vec(),
                )],
            );
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:"))
            );
        }
        for (kind, source, family) in [
            ("Text", "x".repeat(1_048_577), "TEXT-LIMIT"),
            ("Bool", "x".repeat(1_048_577), "JSON-LIMIT"),
            (
                "Json",
                format!("{}0{}", "[".repeat(65), "]".repeat(65)),
                "JSON-LIMIT",
            ),
            (
                "Json",
                format!("\"{}\"", "\u{7f}".repeat(200_000)),
                "JSON-LIMIT",
            ),
        ] {
            let function = value_function(
                "unused",
                &format!("data:{kind}"),
                "",
                "Int",
                vec![return_value("Int", "0")],
            );
            let mut root = vec![instruction(
                "SET_FIELD",
                &[("key", "input.data"), ("value", &source)],
            )];
            root.extend(value_root("unused", "data=f:input.data"));
            let output = run_functions(root, BTreeMap::from([function]), &[]);
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("AIL-RUNTIME-{family}:")),
                "{kind}"
            );
        }
    }

    #[test]
    fn native_value_functions_validate_arguments_before_depth_entry() {
        let functions = BTreeMap::from([value_function(
            "count",
            "n:Int;text:Text",
            "missing:Text;result:Int",
            "Int",
            vec![
                branch_int("local.n", "==", "0", "base"),
                call_value(
                    "count",
                    "n=f:local.n i:1 -;text=f:local.text",
                    "local.result",
                ),
                return_value("Int", "local.result"),
                instruction("LABEL", &[("name", "base")]),
                call_value("count", "n=i:0;text=f:local.missing", "local.result"),
                return_value("Int", "0"),
            ],
        )]);
        let output = run_functions(
            value_root("count", "n=i:63;text=f:input.text"),
            functions,
            &["input.text=valid"],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT:"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-DEPTH"));
    }

    #[test]
    fn native_value_functions_list_and_json_return_budgets_charge_only_one_final_copy() {
        for (kind, input, family) in [
            (
                "List<Text>",
                format!("[\"{}\"]", "x".repeat(1_048_572)),
                "LIST-LIMIT",
            ),
            (
                "Json",
                format!("\"{}\"", "x".repeat(1_048_574)),
                "JSON-LIMIT",
            ),
        ] {
            let functions = BTreeMap::from([
                value_function(
                    "identity",
                    &format!("value:{kind}"),
                    "",
                    kind,
                    vec![return_value(kind, "local.value")],
                ),
                value_function(
                    "unused",
                    &format!("value:{kind}"),
                    "",
                    "Int",
                    vec![return_value("Int", "7")],
                ),
            ]);
            for (target, calls, success) in [
                ("unused", 70, true),
                ("identity", 64, true),
                ("identity", 65, false),
            ] {
                let mut root = vec![instruction(
                    "ASSIGN_LITERAL",
                    &[("key", "input.value"), ("type", kind), ("value", &input)],
                )];
                root.extend(
                    (0..calls).map(|_| call_value(target, "value=f:input.value", "out.value")),
                );
                root.push(instruction(
                    "EVAL_INT",
                    &[("key", "out.result"), ("expression", "i:1")],
                ));
                root.push(instruction("OUTPUT_FIELD", &[("key", "out.result")]));
                root.push(instruction("RETURN_SUCCESS", &[]));
                let output = run_functions(root, functions.clone(), &[]);
                assert_eq!(
                    output.status.success(),
                    success,
                    "{kind}/{target}/{calls}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                if success {
                    assert_eq!(output.stdout, b"out.result=1\n");
                } else {
                    assert!(output.stdout.is_empty());
                    assert!(
                        String::from_utf8_lossy(&output.stderr)
                            .contains(&format!("AIL-RUNTIME-{family}"))
                    );
                }
            }
        }
    }

    #[test]
    fn native_value_functions_preserve_arena_charges_between_body_return_and_root() {
        let functions = BTreeMap::from([value_function(
            "twice",
            "text:Text",
            "empty:Text;joined:Text",
            "Text",
            vec![
                instruction(
                    "ASSIGN_LITERAL",
                    &[("key", "local.empty"), ("type", "Text"), ("value", "\"\"")],
                ),
                instruction(
                    "TEXT_JOIN",
                    &[
                        ("key", "local.joined"),
                        ("left", "local.text"),
                        ("right", "local.empty"),
                    ],
                ),
                return_value("Text", "local.joined"),
            ],
        )]);
        for (calls, success) in [(32, true), (33, false)] {
            let mut root = vec![instruction(
                "ASSIGN_LITERAL",
                &[
                    ("key", "input.text"),
                    ("type", "Text"),
                    ("value", &format!("\"{}\"", "x".repeat(1_048_576))),
                ],
            )];
            root.extend((0..calls).map(|_| call_value("twice", "text=f:input.text", "out.value")));
            root.push(instruction(
                "TEXT_LENGTH",
                &[("key", "out.length"), ("source", "out.value")],
            ));
            root.push(instruction("OUTPUT_FIELD", &[("key", "out.length")]));
            root.push(instruction("RETURN_SUCCESS", &[]));
            let output = run_functions(root, functions.clone(), &[]);
            assert_eq!(output.status.success(), success);
            if success {
                assert_eq!(output.stdout, b"out.length=1048576\n");
            } else {
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT"));
            }
        }
    }

    #[test]
    fn native_value_functions_reject_forged_scopes_types_and_call_kinds_before_emission() {
        let root = AilBytecodeAction {
            name: "root".into(),
            instructions: value_root("bad", ""),
        };
        for body in [
            vec![return_value("Text", "global.secret")],
            vec![return_value("Int", "0")],
            vec![
                call_int("bad", "", "local.text"),
                return_value("Text", "local.text"),
            ],
            vec![instruction(
                "FILE_READ_TEXT",
                &[("key", "local.text"), ("resource", "input")],
            )],
            vec![instruction(
                "TEXT_LENGTH",
                &[("key", "local.text"), ("source", "local.text")],
            )],
        ] {
            let functions = BTreeMap::from([value_function("bad", "", "text:Text", "Text", body)]);
            assert!(
                emit_with_functions(&root, &BTreeMap::new(), &functions, &BTreeMap::new()).is_err()
            );
        }
        let (_, function) = value_function("bad", "", "", "Int", vec![return_value("Int", "0")]);
        assert!(
            emit_with_functions(
                &function,
                &BTreeMap::new(),
                &BTreeMap::new(),
                &BTreeMap::new()
            )
            .is_err()
        );
    }

    #[test]
    fn native_integer_functions_execute_recursive_factorial_and_early_return() {
        let function = integer_function(
            "factorial",
            "n",
            vec![
                branch_int("local.n", "<=", "1", "base"),
                call_int("factorial", "n=f:local.n i:1 -", "local.partial"),
                return_int("local.n * local.partial"),
                instruction("LABEL", &[("name", "base")]),
                return_int("1"),
                return_int("1 / 0"),
            ],
        );
        let output = run_functions(
            function_root("factorial", "n=i:20"),
            BTreeMap::from([function]),
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.result=2432902008176640000\n");
        assert!(
            output.stderr.is_empty(),
            "function execution should have no implicit trace"
        );
    }

    #[test]
    fn native_text_preserves_root_arena_and_recursive_function_registers() {
        let function = integer_function(
            "factorial",
            "n",
            vec![
                branch_int("local.n", "<=", "1", "base"),
                call_int("factorial", "n=f:local.n i:1 -", "local.partial"),
                return_int("local.n * local.partial"),
                instruction("LABEL", &[("name", "base")]),
                return_int("1"),
            ],
        );
        let output = run_functions(
            vec![
                instruction(
                    "TEXT_JOIN",
                    &[
                        ("key", "text.result"),
                        ("left", "text.source"),
                        ("right", "text.source"),
                    ],
                ),
                call_int("factorial", "n=i:5", "out.first"),
                instruction(
                    "TEXT_FIND",
                    &[
                        ("key", "out.position"),
                        ("source", "text.result"),
                        ("needle", "text.needle"),
                    ],
                ),
                call_int("factorial", "n=i:6", "out.second"),
                text_slice(
                    "out.slice",
                    "text.result",
                    "out.position",
                    "out.position + 2",
                ),
                instruction("OUTPUT_FIELD", &[("key", "out.first")]),
                instruction("OUTPUT_FIELD", &[("key", "out.second")]),
                instruction("OUTPUT_FIELD", &[("key", "out.position")]),
                instruction("OUTPUT_FIELD", &[("key", "out.slice")]),
            ],
            BTreeMap::from([function]),
            &["text.source=é🦀", "text.needle=🦀é"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            "out.first=120\nout.position=1\nout.second=720\nout.slice=🦀é\n".as_bytes()
        );
    }

    #[test]
    fn native_integer_functions_preserve_both_recursive_results_in_one_frame() {
        let function = integer_function(
            "fib",
            "n",
            vec![
                branch_int("local.n", "<=", "1", "base"),
                call_int("fib", "n=f:local.n i:1 -", "local.first"),
                call_int("fib", "n=f:local.n i:2 -", "local.second"),
                return_int("local.first + local.second"),
                instruction("LABEL", &[("name", "base")]),
                return_int("local.n"),
            ],
        );
        for (input, expected) in [
            ("n=i:10", "out.result=55\n"),
            ("n=i:20", "out.result=6765\n"),
        ] {
            let output = run_functions(
                function_root("fib", input),
                BTreeMap::from([function.clone()]),
                &[],
            );
            assert!(output.status.success(), "{output:?}");
            assert_eq!(output.stdout, expected.as_bytes());
            assert!(output.stderr.is_empty());
        }
    }

    #[test]
    fn native_integer_functions_support_mutual_recursion_and_forward_targets() {
        let functions = BTreeMap::from([
            integer_function(
                "even",
                "n",
                vec![
                    branch_int("local.n", "==", "0", "base"),
                    call_int("odd", "n=f:local.n i:1 -", "local.result"),
                    return_int("local.result"),
                    instruction("LABEL", &[("name", "base")]),
                    return_int("1"),
                ],
            ),
            integer_function(
                "odd",
                "n",
                vec![
                    branch_int("local.n", "==", "0", "base"),
                    call_int("even", "n=f:local.n i:1 -", "local.result"),
                    return_int("local.result"),
                    instruction("LABEL", &[("name", "base")]),
                    return_int("0"),
                ],
            ),
        ]);
        for (input, expected) in [("n=i:17", "out.result=0\n"), ("n=i:18", "out.result=1\n")] {
            let output = run_functions(function_root("even", input), functions.clone(), &[]);
            assert!(output.status.success(), "{output:?}");
            assert_eq!(output.stdout, expected.as_bytes());
        }
    }

    #[test]
    fn native_integer_functions_bind_named_arguments_and_isolate_global_local_keys() {
        let functions = BTreeMap::from([
            integer_function(
                "difference",
                "z;a",
                vec![eval("local.x", "local.z - local.a"), return_int("local.x")],
            ),
            integer_function("constant", "", vec![return_int("-9223372036854775808")]),
        ]);
        let output = run_functions(
            vec![
                call_int("difference", "z=f:local.x;a=i:3", "out.result"),
                instruction("OUTPUT_FIELD", &[("key", "out.result")]),
                instruction("OUTPUT_FIELD", &[("key", "local.x")]),
            ],
            functions.clone(),
            &["local.x=8"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"local.x=8\nout.result=5\n");
        let output = run_functions(function_root("constant", ""), functions, &[]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.result=-9223372036854775808\n");
    }

    #[test]
    fn native_integer_function_arguments_evaluate_in_sorted_name_order() {
        let functions = BTreeMap::from([integer_function("f", "z;a", vec![return_int("0")])]);
        let output = run_functions(
            function_root("f", "z=i:1 i:0 /;a=f:input.missing"),
            functions,
            &[],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-INT"));
    }

    #[test]
    fn native_integer_functions_enforce_64_active_frames() {
        let functions = BTreeMap::from([integer_function(
            "countdown",
            "n",
            vec![
                branch_int("local.n", "==", "0", "base"),
                call_int("countdown", "n=f:local.n i:1 -", "local.result"),
                return_int("local.result + 1"),
                instruction("LABEL", &[("name", "base")]),
                return_int("0"),
            ],
        )]);
        let output = run_functions(function_root("countdown", "n=i:63"), functions.clone(), &[]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.result=63\n");
        let output = run_functions(function_root("countdown", "n=i:64"), functions, &[]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-DEPTH"));
    }

    #[test]
    fn native_integer_functions_report_missing_return_local_and_arithmetic_errors() {
        for (body, code) in [
            (vec![eval("local.x", "1")], "AIL-RUNTIME-RETURN"),
            (vec![return_int("local.x")], "AIL-RUNTIME-INT"),
            (vec![return_int("1 / 0")], "AIL-RUNTIME-DIVZERO"),
            (
                vec![return_int("9223372036854775807 + 1")],
                "AIL-RUNTIME-OVERFLOW",
            ),
            (
                vec![return_int("-9223372036854775808 / -1")],
                "AIL-RUNTIME-OVERFLOW",
            ),
            (
                vec![
                    instruction("LABEL", &[("name", "again")]),
                    instruction("JUMP", &[("label", "again")]),
                ],
                "step limit",
            ),
        ] {
            let output = run_functions(
                function_root("f", ""),
                BTreeMap::from([integer_function("f", "", body)]),
                &["local.x=7", "out.result=99"],
            );
            assert_eq!(output.status.code(), Some(1), "{code}: {output:?}");
            assert!(output.stdout.is_empty(), "{code}: {output:?}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(code),
                "{code}: {output:?}"
            );
        }
    }

    #[test]
    fn native_integer_functions_preserve_caller_fuel_and_explicit_traces() {
        let mut root = vec![instruction("NATIVE_FUEL_ROOT", &[("limit", "3")])];
        root.extend(function_root("f", ""));
        let functions = BTreeMap::from([integer_function(
            "f",
            "",
            vec![
                instruction("EMIT_TRACE", &[("event", "Called")]),
                return_int("42"),
            ],
        )]);
        let output = run_functions(root, functions, &[]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.result=42\n");
        assert_eq!(output.stderr, b"trace Called\n");
    }

    #[test]
    fn native_integer_functions_reject_invalid_targets_arity_and_global_references() {
        let valid = BTreeMap::from([integer_function("f", "n", vec![return_int("local.n")])]);
        for (target, arguments) in [("unknown", "n=i:1"), ("f", ""), ("f", "n=i:1;x=i:2")] {
            let action = AilBytecodeAction {
                name: "Compute".into(),
                instructions: function_root(target, arguments),
            };
            assert!(
                emit_with_functions(&action, &BTreeMap::new(), &valid, &BTreeMap::new()).is_err()
            );
        }
        let action = AilBytecodeAction {
            name: "Compute".into(),
            instructions: function_root("f", ""),
        };
        for body in [
            vec![return_int("global.x")],
            vec![instruction("INTEGER_FUNCTION", &[("parameters", "")])],
            vec![instruction("CALL_ACTION", &[("target", "Compute")])],
        ] {
            assert!(
                emit_with_functions(
                    &action,
                    &BTreeMap::new(),
                    &BTreeMap::from([integer_function("f", "", body)]),
                    &BTreeMap::new()
                )
                .is_err()
            );
        }
        let function = &valid["f"];
        assert!(emit_with_functions(function, &BTreeMap::new(), &valid, &BTreeMap::new()).is_err());
    }

    #[test]
    fn native_returned_text_and_copies_use_latest_mutable_state() {
        let output = run(
            vec![
                instruction(
                    "SET_FIELD",
                    &[("key", "value.text"), ("value", "007 = text")],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "result.text"), ("source", "value.text")],
                ),
                instruction("SET_FIELD", &[("key", "value.text"), ("value", "changed")]),
                instruction("OUTPUT_FIELD", &[("key", "result.text")]),
                instruction("RETURN_SUCCESS", &[]),
            ],
            &["ignored.input=secret", "value.text=old"],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"result.text=007 = text\n");
    }
    fn eval(key: &str, expression: &str) -> AilBytecodeInstruction {
        let expression = crate::expression::IntExpr::parse(expression)
            .unwrap()
            .to_postfix();
        instruction("EVAL_INT", &[("key", key), ("expression", &expression)])
    }

    fn branch_int(
        left: &str,
        comparison: &str,
        right: &str,
        label: &str,
    ) -> AilBytecodeInstruction {
        let left = IntExpr::parse(left).unwrap().to_postfix();
        let right = IntExpr::parse(right).unwrap().to_postfix();
        instruction(
            "BRANCH_INT",
            &[
                ("left", &left),
                ("comparison", comparison),
                ("right", &right),
                ("label", label),
            ],
        )
    }

    #[test]
    fn native_branch_int_compares_runtime_expressions_at_signed_boundaries() {
        // Hand-checked truth tables in ==, !=, <, <=, >, >= order. The MIN/MAX
        // cases expose comparisons implemented as an unchecked subtraction.
        for (left, right, expected) in [
            (i64::MIN, i64::MAX, [false, true, true, true, false, false]),
            (i64::MAX, i64::MIN, [false, true, false, false, true, true]),
            (i64::MIN, i64::MIN, [true, false, false, true, false, true]),
            (i64::MAX, i64::MAX, [true, false, false, true, false, true]),
            (-7, -3, [false, true, true, true, false, false]),
            (-3, -7, [false, true, false, false, true, true]),
            (-7, -7, [true, false, false, true, false, true]),
            (-1, 0, [false, true, true, true, false, false]),
            (0, -1, [false, true, false, false, true, true]),
        ] {
            for (comparison, taken) in ["==", "!=", "<", "<=", ">", ">="].into_iter().zip(expected)
            {
                // Both input fields appear only inside the branch expressions,
                // so forgetting either operand during slot allocation fails.
                let output = run(
                    vec![
                        instruction("SET_FIELD", &[("key", "result.taken"), ("value", "false")]),
                        branch_int("input.left + 0", comparison, "input.right * 1", "taken"),
                        instruction("JUMP", &[("label", "done")]),
                        instruction("LABEL", &[("name", "taken")]),
                        instruction("SET_FIELD", &[("key", "result.taken"), ("value", "true")]),
                        instruction("LABEL", &[("name", "done")]),
                        instruction("OUTPUT_FIELD", &[("key", "result.taken")]),
                    ],
                    &[
                        &format!("input.left={left}"),
                        &format!("input.right={right}"),
                    ],
                );
                assert!(
                    output.status.success(),
                    "{left} {comparison} {right}: {output:?}"
                );
                assert_eq!(
                    output.stdout,
                    format!("result.taken={taken}\n").as_bytes(),
                    "{left} {comparison} {right}"
                );
                let branch_trace = if taken { "taken" } else { "skipped" };
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .contains(&format!("branch taken {branch_trace}\n"))
                );
            }
        }
    }

    #[test]
    fn native_branch_int_checks_runtime_errors_in_both_operands() {
        for (left, right, args, category) in [
            (
                "9223372036854775807 + 1",
                "0",
                vec![],
                "AIL-RUNTIME-OVERFLOW",
            ),
            (
                "0",
                "-9223372036854775808 / -1",
                vec![],
                "AIL-RUNTIME-OVERFLOW",
            ),
            ("1 / 0", "0", vec![], "AIL-RUNTIME-DIVZERO"),
            ("0", "1 % 0", vec![], "AIL-RUNTIME-DIVZERO"),
            ("missing.left", "0", vec![], "AIL-RUNTIME-INT"),
            ("0", "missing.right", vec![], "AIL-RUNTIME-INT"),
            ("input.left", "0", vec!["input.left=1x"], "AIL-RUNTIME-INT"),
            (
                "0",
                "input.right",
                vec!["input.right=9223372036854775808"],
                "AIL-RUNTIME-INT",
            ),
            // Evaluation stays left-to-right even when both sides would fail.
            ("missing.left", "1 / 0", vec![], "AIL-RUNTIME-INT"),
        ] {
            let output = run(
                vec![
                    branch_int(left, "==", right, "done"),
                    instruction("LABEL", &[("name", "done")]),
                ],
                &args,
            );
            assert_eq!(
                output.status.code(),
                Some(1),
                "{left} == {right}: {output:?}"
            );
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(category),
                "{left} == {right}: {output:?}"
            );
        }
    }

    #[test]
    fn native_branch_int_backward_loop_reads_mutations_and_enforces_fuel() {
        let output = run(
            vec![
                eval("counter.n", "0"),
                instruction("LABEL", &[("name", "again")]),
                eval("counter.n", "counter.n + 1"),
                branch_int("counter.n", "<", "input.limit + 0", "again"),
                instruction("OUTPUT_FIELD", &[("key", "counter.n")]),
            ],
            &["input.limit=3"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"counter.n=3\n");
        let looping = run(
            vec![
                instruction("LABEL", &[("name", "again")]),
                branch_int("-1", "<", "1", "again"),
            ],
            &[],
        );
        assert_eq!(looping.status.code(), Some(1));
        assert!(looping.stdout.is_empty());
        assert!(String::from_utf8_lossy(&looping.stderr).contains("step limit"));
    }

    #[test]
    fn native_branch_int_skips_unselected_arithmetic_and_conditions() {
        let output = run(
            vec![
                branch_int("input.choose + 1", "==", "2", "chosen"),
                eval("result.value", "1 / 0"),
                eval("result.value", "9223372036854775807 + 1"),
                branch_int("missing.value", "<", "1 / 0", "chosen"),
                instruction("LABEL", &[("name", "chosen")]),
                eval("result.value", "42"),
                instruction("OUTPUT_FIELD", &[("key", "result.value")]),
            ],
            &["input.choose=1"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"result.value=42\n");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-"));
    }

    #[test]
    fn native_branch_int_rejects_invalid_comparison_operands_and_labels() {
        for (operand, value, error) in [
            ("comparison", "===", "comparison"),
            ("left", "invalid-postfix", "AIL-EXPR"),
            ("right", "invalid-postfix", "AIL-EXPR"),
            ("label", "absent", "label"),
        ] {
            let mut branch = branch_int("1", "==", "1", "done");
            branch
                .operands
                .insert(operand.to_string(), value.to_string());
            let action = AilBytecodeAction {
                name: "InvalidBranch".to_string(),
                instructions: vec![branch, instruction("LABEL", &[("name", "done")])],
            };
            let actual = emit(&action, &BTreeMap::new()).unwrap_err();
            assert!(actual.contains(error), "{operand}: {actual}");
        }
        for operand in ["left", "comparison", "right", "label"] {
            let mut branch = branch_int("1", "==", "1", "done");
            branch.operands.remove(operand);
            let action = AilBytecodeAction {
                name: "MissingBranchOperand".to_string(),
                instructions: vec![branch, instruction("LABEL", &[("name", "done")])],
            };
            let actual = emit(&action, &BTreeMap::new()).unwrap_err();
            assert!(actual.contains(operand), "{operand}: {actual}");
        }
    }

    #[test]
    fn native_integer_operations_read_prior_writes_and_copy_values() {
        let output = run(
            vec![
                eval("value.a", "value.a + 2"),
                instruction(
                    "COPY_FIELD",
                    &[("key", "value.saved"), ("source", "value.a")],
                ),
                eval("value.a", "value.a * 3"),
                eval("value.b", "value.a - value.saved"),
                eval("value.q", "-7 / 3"),
                eval("value.r", "-7 % 3"),
                eval("value.max", "9223372036854775807"),
                eval("value.min", "-9223372036854775808"),
                instruction("OUTPUT_FIELD", &[("key", "value.b")]),
                instruction("OUTPUT_FIELD", &[("key", "value.saved")]),
                instruction("OUTPUT_FIELD", &[("key", "value.q")]),
                instruction("OUTPUT_FIELD", &[("key", "value.r")]),
                instruction("OUTPUT_FIELD", &[("key", "value.max")]),
                instruction("OUTPUT_FIELD", &[("key", "value.min")]),
                instruction("RETURN_SUCCESS", &[]),
            ],
            &["value.a=999", "value.a=+0005"],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"value.b=14\nvalue.max=9223372036854775807\nvalue.min=-9223372036854775808\nvalue.q=-2\nvalue.r=-1\nvalue.saved=7\n");
    }

    #[test]
    fn native_integer_errors_are_checked_before_returned_output() {
        for (expression, category) in [
            ("9223372036854775807 + 1", "AIL-RUNTIME-OVERFLOW"),
            ("-9223372036854775808 - 1", "AIL-RUNTIME-OVERFLOW"),
            ("9223372036854775807 * 2", "AIL-RUNTIME-OVERFLOW"),
            ("-9223372036854775808 / -1", "AIL-RUNTIME-OVERFLOW"),
            ("-9223372036854775808 % -1", "AIL-RUNTIME-OVERFLOW"),
            ("1 / 0", "AIL-RUNTIME-DIVZERO"),
            ("1 % 0", "AIL-RUNTIME-DIVZERO"),
            ("missing.value + 1", "AIL-RUNTIME-INT"),
        ] {
            let output = run(vec![eval("value.result", expression)], &[]);
            assert!(!output.status.success(), "{expression}");
            assert!(output.stdout.is_empty(), "{expression}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(category),
                "{expression}: {:?}",
                output
            );
        }
        for value in [
            "",
            "+",
            "-",
            "1x",
            " 2",
            "2 ",
            "9223372036854775808",
            "-9223372036854775809",
            "18446744073709551616",
        ] {
            let arg = format!("value.a={value}");
            let output = run(vec![eval("value.result", "value.a")], &[&arg]);
            assert!(!output.status.success(), "accepted {value:?}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-INT"),
                "{value:?}: {:?}",
                output
            );
        }
    }

    #[test]
    fn native_missing_selected_output_does_not_print_partial_results() {
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "result.a"), ("value", "present")]),
                instruction("OUTPUT_FIELD", &[("key", "result.a")]),
                instruction("OUTPUT_FIELD", &[("key", "result.z")]),
            ],
            &[],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-OUTPUT"));
    }

    #[test]
    fn native_full_state_preserves_unknown_arguments_and_latest_duplicate() {
        let output = run(
            vec![eval("value.result", "value.a + 1")],
            &["value.a=2", "unknown.key=old", "unknown.key=009"],
        );
        assert!(output.status.success(), "{:?}", output);
        let mut lines = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        lines.sort();
        assert_eq!(lines, ["unknown.key=009", "value.a=2", "value.result=3"]);
    }

    #[test]
    fn native_backward_branches_observe_updates_and_stop_at_step_limit() {
        let output = run(
            vec![
                instruction("SET_FIELD", &[("key", "counter.n"), ("value", "0")]),
                instruction("LABEL", &[("name", "again")]),
                instruction("ADD_INT_FIELD", &[("key", "counter.n"), ("delta", "1")]),
                instruction(
                    "BRANCH_FIELD_EQUALS",
                    &[("key", "counter.n"), ("value", "3"), ("label", "done")],
                ),
                instruction("JUMP", &[("label", "again")]),
                instruction("LABEL", &[("name", "done")]),
                instruction("OUTPUT_FIELD", &[("key", "counter.n")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{:?}", output);
        assert_eq!(output.stdout, b"counter.n=3\n");
        let looping = run(
            vec![
                instruction("LABEL", &[("name", "again")]),
                instruction("JUMP", &[("label", "again")]),
            ],
            &[],
        );
        assert!(!looping.status.success());
        assert!(String::from_utf8_lossy(&looping.stderr).contains("step limit"));
    }
    #[test]
    fn native_integer_input_boundaries_are_normalized_without_rounding() {
        for (input, expected) in [
            ("9223372036854775807", "9223372036854775807"),
            ("-9223372036854775808", "-9223372036854775808"),
            ("+000000000000000000000009", "9"),
            ("-000000000000000000000009", "-9"),
            ("-0", "0"),
            ("+0", "0"),
        ] {
            let arg = format!("value.input={input}");
            let output = run(
                vec![
                    eval("value.result", "value.input + 0"),
                    instruction("OUTPUT_FIELD", &[("key", "value.result")]),
                ],
                &[&arg],
            );
            assert!(output.status.success(), "{input}: {:?}", output);
            assert_eq!(
                output.stdout,
                format!("value.result={expected}\n").as_bytes()
            );
        }
    }

    #[test]
    fn native_text_and_numeric_self_copies_preserve_full_values() {
        let output = run(
            vec![
                instruction(
                    "SET_FIELD",
                    &[("key", "value.text"), ("value", "007\0=tail")],
                ),
                instruction(
                    "COPY_FIELD",
                    &[("key", "value.text"), ("source", "value.text")],
                ),
                eval("value.n", "-9223372036854775808"),
                instruction("COPY_FIELD", &[("key", "value.n"), ("source", "value.n")]),
                eval("value.result", "value.n / 2"),
                instruction("OUTPUT_FIELD", &[("key", "value.text")]),
                instruction("OUTPUT_FIELD", &[("key", "value.result")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{:?}", output);
        assert_eq!(
            output.stdout,
            b"value.result=-4611686018427387904\nvalue.text=007\0=tail\n"
        );
    }

    #[test]
    fn native_requirements_read_written_state_and_compare_lexically() {
        let passing = run(
            vec![
                instruction("SET_FIELD", &[("key", "value.a"), ("value", "3")]),
                instruction("SET_FIELD", &[("key", "value.b"), ("value", "20")]),
                instruction(
                    "REQUIRE_EXISTS",
                    &[
                        ("key", "value.a"),
                        ("rule", "present"),
                        ("failure", "Missing"),
                    ],
                ),
                instruction(
                    "REQUIRE_FIELD_NOT_EQUALS",
                    &[
                        ("key", "value.a"),
                        ("value", "999"),
                        ("rule", "changed"),
                        ("failure", "Unchanged"),
                    ],
                ),
                instruction(
                    "REQUIRE_FIELD_AFTER",
                    &[
                        ("source", "value.a"),
                        ("key", "value.b"),
                        ("rule", "after"),
                        ("failure", "Before"),
                    ],
                ),
                instruction(
                    "REQUIRE_FIELD_IN",
                    &[
                        ("key", "value.a"),
                        ("values", "1\u{1f}3"),
                        ("rule", "allowed"),
                        ("failure", "Excluded"),
                    ],
                ),
                instruction("OUTPUT_FIELD", &[("key", "value.a")]),
            ],
            &["value.a=999"],
        );
        assert!(passing.status.success(), "{:?}", passing);
        assert_eq!(passing.stdout, b"value.a=3\n");
        let failing = run(
            vec![
                instruction("SET_FIELD", &[("key", "value.a"), ("value", "3")]),
                instruction(
                    "REQUIRE_FIELD_NOT_EQUALS",
                    &[
                        ("key", "value.a"),
                        ("value", "3"),
                        ("rule", "forbidden"),
                        ("failure", "Forbidden"),
                    ],
                ),
            ],
            &[],
        );
        assert!(!failing.status.success());
        assert!(failing.stdout.is_empty());
        assert!(String::from_utf8_lossy(&failing.stderr).contains("failure Forbidden"));
    }

    #[test]
    fn native_output_selection_respects_executed_control_flow() {
        let output = run(
            vec![
                instruction("JUMP", &[("label", "chosen")]),
                instruction("OUTPUT_FIELD", &[("key", "missing.value")]),
                instruction("LABEL", &[("name", "chosen")]),
                instruction("OUTPUT_FIELD", &[("key", "value.good")]),
            ],
            &["value.good=chosen", "other.key=hidden"],
        );
        assert!(output.status.success(), "{:?}", output);
        assert_eq!(output.stdout, b"value.good=chosen\n");
    }

    #[test]
    fn native_rejects_unsupported_mixed_instructions() {
        let action = AilBytecodeAction {
            name: "Unsupported".to_string(),
            instructions: vec![instruction("OPTION_MAP", &[])],
        };
        assert!(
            emit(&action, &BTreeMap::new())
                .unwrap_err()
                .contains("unsupported native scalar opcode 'OPTION_MAP'")
        );
    }
    fn vm(
        instructions: Vec<AilBytecodeInstruction>,
        args: &[&str],
    ) -> Result<AilRunResult, String> {
        let action = AilBytecodeAction {
            name: "Compute".to_string(),
            instructions,
        };
        let program = AilBytecodeProgram {
            package_name: "native-integer-test".to_string(),
            package_version: "0.1.0".to_string(),
            profile: "Application".to_string(),
            capability_grants: Vec::new(),
            target_support: BTreeMap::new(),
            external_bindings_metadata_present: false,
            external_bindings: BTreeMap::new(),
            type_schemas: BTreeMap::new(),
            actions: BTreeMap::from([("Compute".to_string(), action)]),
            failures: BTreeMap::new(),
        };
        let state = args
            .iter()
            .map(|arg| {
                let (key, value) = arg.split_once('=').unwrap();
                (key.to_string(), value.to_string())
            })
            .collect();
        run_ail_bytecode_action(&program, "Compute", state)
    }

    #[test]
    fn native_division_and_remainder_match_vm_and_independent_signed_boundary_answers() {
        // Expected quotients and remainders are hand-derived, including MIN's
        // unsigned magnitude and divisors larger than the dividend magnitude.
        for (left, right, quotient, remainder) in [
            (7, 3, 2, 1),
            (-7, 3, -2, -1),
            (7, -3, -2, 1),
            (-7, -3, 2, -1),
            (0, 3, 0, 0),
            (i64::MIN, 1, i64::MIN, 0),
            (i64::MIN, 2, -4611686018427387904, 0),
            (i64::MIN, 3, -3074457345618258602, -2),
            (i64::MAX, -2, -4611686018427387903, 1),
            (1, i64::MIN, 0, 1),
            (-1, i64::MIN, 0, -1),
            (i64::MIN, i64::MIN, 1, 0),
            (i64::MAX, i64::MIN, 0, i64::MAX),
            (i64::MIN, i64::MAX, -1, -1),
            (i64::MAX, i64::MAX, 1, 0),
        ] {
            let instructions = vec![
                eval("value.q", "value.a / value.b"),
                eval("value.r", "value.a % value.b"),
                instruction("OUTPUT_FIELD", &[("key", "value.q")]),
                instruction("OUTPUT_FIELD", &[("key", "value.r")]),
            ];
            let a = format!("value.a={left}");
            let b = format!("value.b={right}");
            let virtual_run = vm(instructions.clone(), &[&a, &b]).unwrap();
            let native = run(instructions, &[&a, &b]);
            assert!(native.status.success(), "{left}/{right}: {:?}", native);
            assert_eq!(virtual_run.final_state["value.q"], quotient.to_string());
            assert_eq!(virtual_run.final_state["value.r"], remainder.to_string());
            assert_eq!(
                native.stdout,
                format!("value.q={quotient}\nvalue.r={remainder}\n").as_bytes()
            );
            assert_eq!(
                String::from_utf8_lossy(&native.stderr),
                virtual_run.trace.join("\n") + "\n"
            );
        }
    }

    #[test]
    fn native_arithmetic_failure_categories_match_vm_at_input_boundaries() {
        for (expression, left, right, category) in [
            (
                "value.a + value.b",
                "9223372036854775807",
                "1",
                "AIL-RUNTIME-OVERFLOW",
            ),
            (
                "value.a - value.b",
                "-9223372036854775808",
                "1",
                "AIL-RUNTIME-OVERFLOW",
            ),
            (
                "value.a * value.b",
                "-9223372036854775808",
                "-1",
                "AIL-RUNTIME-OVERFLOW",
            ),
            (
                "value.a / value.b",
                "-9223372036854775808",
                "-1",
                "AIL-RUNTIME-OVERFLOW",
            ),
            (
                "value.a % value.b",
                "-9223372036854775808",
                "-1",
                "AIL-RUNTIME-OVERFLOW",
            ),
            ("value.a / value.b", "0", "-0", "AIL-RUNTIME-DIVZERO"),
            ("value.a % value.b", "0", "+000", "AIL-RUNTIME-DIVZERO"),
            (
                "value.a + value.b",
                "9223372036854775808",
                "0",
                "AIL-RUNTIME-INT",
            ),
            (
                "value.a + value.b",
                "-9223372036854775809",
                "0",
                "AIL-RUNTIME-INT",
            ),
        ] {
            let instructions = vec![eval("value.result", expression)];
            let a = format!("value.a={left}");
            let b = format!("value.b={right}");
            let virtual_error = vm(instructions.clone(), &[&a, &b]).unwrap_err();
            let native = run(instructions, &[&a, &b]);
            assert!(
                virtual_error.contains(category),
                "{expression}: {virtual_error}"
            );
            assert!(!native.status.success(), "{expression}: {:?}", native);
            assert!(native.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&native.stderr).contains(category),
                "{expression}: {:?}",
                native
            );
        }
    }
    #[test]
    fn native_nested_output_scopes_restore_caller_selection() {
        let output = run(
            vec![
                instruction("OUTPUT_FIELD", &[("key", "root.answer")]),
                instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                eval("child.value", "7"),
                instruction("OUTPUT_FIELD", &[("key", "child.value")]),
                instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                eval("grandchild.value", "child.value * 6"),
                instruction("OUTPUT_FIELD", &[("key", "grandchild.value")]),
                instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
                instruction(
                    "COPY_FIELD",
                    &[("key", "root.answer"), ("source", "grandchild.value")],
                ),
                instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
            ],
            &[],
        );
        assert!(output.status.success(), "{:?}", output);
        assert_eq!(output.stdout, b"root.answer=42\n");
    }

    #[test]
    fn native_missing_callee_output_fails_before_caller_can_fill_it() {
        let output = run(
            vec![
                instruction("OUTPUT_FIELD", &[("key", "root.answer")]),
                instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                instruction("OUTPUT_FIELD", &[("key", "child.missing")]),
                instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
                instruction(
                    "SET_FIELD",
                    &[("key", "child.missing"), ("value", "too late")],
                ),
                instruction(
                    "SET_FIELD",
                    &[("key", "root.answer"), ("value", "incorrect success")],
                ),
            ],
            &[],
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-OUTPUT"));
    }

    #[test]
    fn native_repeated_output_scopes_balance_the_stack_and_preserve_default_outputs() {
        let output = run(
            vec![
                eval("counter.n", "0"),
                instruction("LABEL", &[("name", "again")]),
                instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                eval("counter.n", "counter.n + 1"),
                instruction("OUTPUT_FIELD", &[("key", "counter.n")]),
                instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
                instruction(
                    "BRANCH_FIELD_EQUALS",
                    &[("key", "counter.n"), ("value", "3"), ("label", "done")],
                ),
                instruction("JUMP", &[("label", "again")]),
                instruction("LABEL", &[("name", "done")]),
            ],
            &["untouched.key=still present"],
        );
        assert!(output.status.success(), "{:?}", output);
        assert_eq!(output.stdout, b"counter.n=3\nuntouched.key=still present\n");
    }

    #[test]
    fn native_fuel_scopes_allow_long_finite_loops_with_large_callees() {
        let mut instructions = vec![
            instruction("NATIVE_FUEL_ROOT", &[("limit", "12288")]),
            eval("counter.n", "0"),
            instruction("LABEL", &[("name", "again")]),
            branch_int("counter.n", ">=", "2000", "done"),
            instruction("NATIVE_TRACE_LINE", &[("text", "call Helper")]),
            instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "104448")]),
            instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
            instruction("ACTION_BEGIN", &[("action", "Helper")]),
        ];
        for _ in 0..100 {
            instructions.push(instruction(
                "SET_FIELD",
                &[("key", "helper.value"), ("value", "1")],
            ));
        }
        instructions.extend([
            instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
            instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "1")]),
            eval("counter.n", "counter.n + 1"),
            instruction("JUMP", &[("label", "again")]),
            instruction("LABEL", &[("name", "done")]),
            instruction("OUTPUT_FIELD", &[("key", "counter.n")]),
        ]);
        let output = run(instructions, &[]);
        assert!(output.status.success(), "{:?}", output.status);
        assert_eq!(output.stdout, b"counter.n=2000\n");
    }

    #[test]
    fn native_fuel_scopes_restore_nested_budgets_and_output_selection() {
        // Exact budgets: root selects/calls/returns, child computes/calls/selects/
        // returns, and grandchild computes/selects/returns. Scope metadata is free.
        for (root_limit, child_limit, grandchild_limit, success) in [
            ("3", "4", "3", true),
            ("2", "4", "3", false),
            ("3", "3", "3", false),
            ("3", "4", "2", false),
        ] {
            let output = run(
                vec![
                    instruction("NATIVE_FUEL_ROOT", &[("limit", root_limit)]),
                    instruction("OUTPUT_FIELD", &[("key", "root.answer")]),
                    instruction("NATIVE_TRACE_LINE", &[("text", "call Child")]),
                    instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", child_limit)]),
                    instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                    eval("child.value", "7"),
                    instruction("NATIVE_TRACE_LINE", &[("text", "call Grandchild")]),
                    instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", grandchild_limit)]),
                    instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                    eval("root.answer", "child.value * 6"),
                    instruction("OUTPUT_FIELD", &[("key", "root.answer")]),
                    instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
                    instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "1")]),
                    instruction("OUTPUT_FIELD", &[("key", "child.value")]),
                    instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
                    instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "1")]),
                    instruction("RETURN_SUCCESS", &[]),
                ],
                &[],
            );
            assert_eq!(output.status.success(), success, "{output:?}");
            if success {
                assert_eq!(output.stdout, b"root.answer=42\n");
            } else {
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                assert!(String::from_utf8_lossy(&output.stderr).contains("step limit"));
            }
        }
    }

    #[test]
    fn native_fuel_scopes_charge_repeat_once_and_reset_each_callee() {
        let mut instructions = vec![
            instruction("NATIVE_FUEL_ROOT", &[("limit", "3")]),
            eval("counter.n", "0"),
            instruction("NATIVE_TRACE_LINE", &[("text", "repeat Helper 100 times")]),
        ];
        for _ in 0..100 {
            instructions.extend([
                instruction(
                    "NATIVE_TRACE_LINE",
                    &[("text", "repeat iteration"), ("fuel", "0")],
                ),
                instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "2")]),
                instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                eval("counter.n", "counter.n + 1"),
                instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
                instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "1")]),
            ]);
        }
        instructions.push(instruction("OUTPUT_FIELD", &[("key", "counter.n")]));
        let output = run(instructions, &[]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"counter.n=100\n");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr)
                .matches("repeat iteration\n")
                .count(),
            100
        );
    }

    #[test]
    fn native_fuel_scopes_do_not_charge_an_absent_callee_return() {
        let output = run(
            vec![
                instruction("NATIVE_FUEL_ROOT", &[("limit", "2")]),
                instruction("NATIVE_TRACE_LINE", &[("text", "call Helper")]),
                instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "1")]),
                instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                eval("result.value", "42"),
                instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
                instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "0")]),
                instruction("OUTPUT_FIELD", &[("key", "result.value")]),
            ],
            &[],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"result.value=42\n");
    }

    #[test]
    fn native_fuel_scopes_bound_a_looping_callee_before_any_output() {
        let output = run(
            vec![
                instruction("NATIVE_FUEL_ROOT", &[("limit", "100")]),
                instruction("OUTPUT_FIELD", &[("key", "root.answer")]),
                instruction("NATIVE_TRACE_LINE", &[("text", "call Helper")]),
                instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "3")]),
                instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                instruction("LABEL", &[("name", "again")]),
                instruction("JUMP", &[("label", "again")]),
                instruction("NATIVE_OUTPUT_SCOPE_END", &[]),
                instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "0")]),
            ],
            &["root.answer=42"],
        );
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let trace = String::from_utf8_lossy(&output.stderr);
        assert!(trace.contains("step limit"));
        assert_eq!(trace.matches("jump again\n").count(), 1);
    }

    #[test]
    fn native_fuel_scopes_check_return_fuel_before_missing_callee_output() {
        for (limit, return_cost, expected_error) in [
            ("1", "1", "step limit"),
            ("2", "1", "AIL-RUNTIME-OUTPUT"),
            ("1", "0", "AIL-RUNTIME-OUTPUT"),
        ] {
            let output = run(
                vec![
                    instruction("NATIVE_FUEL_ROOT", &[("limit", "3")]),
                    instruction("OUTPUT_FIELD", &[("key", "root.answer")]),
                    instruction("NATIVE_TRACE_LINE", &[("text", "call Helper")]),
                    instruction("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", limit)]),
                    instruction("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                    instruction("OUTPUT_FIELD", &[("key", "child.missing")]),
                    instruction("NATIVE_OUTPUT_SCOPE_END", &[("return_cost", return_cost)]),
                    instruction("NATIVE_FUEL_SCOPE_END", &[("return_cost", "0")]),
                    instruction("RETURN_SUCCESS", &[]),
                ],
                &["root.answer=42"],
            );
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(expected_error),
                "limit {limit}, return cost {return_cost}: {output:?}"
            );
        }
    }
}
