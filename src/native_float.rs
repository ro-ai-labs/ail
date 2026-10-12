//! Binary64 execution through the embedded allocation-free numeric runtime.
//! Values cross arithmetic/function boundaries as bits. Field writes render into
//! the existing owned 32-byte numeric buffer; reads validate scalar transport.
use super::*;
use crate::float_expression::{FloatCondition, FloatExpr};

pub(super) fn needs_runtime(i: &AilBytecodeInstruction) -> bool {
    if matches!(i.opcode.as_str(), "EVAL_BOOL" | "BRANCH_BOOL") {
        return i
            .operands
            .get("expression")
            .and_then(|value| crate::bool_expression::BoolExpr::from_postfix(value).ok())
            .is_some_and(|expression| expression.uses_float());
    }
    matches!(
        i.opcode.as_str(),
        "EVAL_FLOAT" | "BRANCH_FLOAT" | "INT_TO_FLOAT" | "FLOAT_TO_INT"
    ) || i
        .operands
        .get("type")
        .is_some_and(|s| matches!(s.as_str(), "Float" | "List<Float>"))
        || i.opcode == "VALUE_FUNCTION"
            && (i
                .operands
                .get("result")
                .is_some_and(|s| matches!(s.as_str(), "Float" | "List<Float>"))
                || ["parameters", "locals"].iter().any(|key| {
                    i.operands.get(*key).is_some_and(|s| {
                        s.split(';')
                            .any(|item| item.ends_with(":Float") || item.ends_with(":List<Float>"))
                    })
                }))
}

pub(super) fn fields(i: &AilBytecodeInstruction) -> Result<BTreeSet<String>, String> {
    let mut fields = BTreeSet::new();
    match i.opcode.as_str() {
        "EVAL_FLOAT" => {
            fields.extend(FloatExpr::from_postfix(required(i, "expression")?)?.fields());
        }
        "BRANCH_FLOAT" => {
            fields.extend(
                FloatCondition::from_postfix(
                    required(i, "left")?,
                    required(i, "comparison")?,
                    required(i, "right")?,
                )?
                .fields(),
            );
        }
        "INT_TO_FLOAT" | "FLOAT_TO_INT" => {
            fields.insert(required(i, "source")?.into());
        }
        _ => return Ok(fields),
    }
    if i.opcode != "BRANCH_FLOAT" {
        fields.insert(required(i, "key")?.into());
    }
    Ok(fields)
}

fn bits(emitter: &mut Emitter, value: u64) {
    emitter.code.emit(&[0x48, 0xb8]);
    emitter.code.emit(&value.to_le_bytes());
}
pub(super) fn literal(emitter: &mut Emitter, value: &str) -> Result<(), String> {
    bits(
        emitter,
        crate::float_number::parse(value)
            .map_err(|e| format!("invalid native Float literal: {e:?}"))?,
    );
    Ok(())
}
pub(super) fn read(emitter: &mut Emitter, key: &str) {
    emitter.load_rdi(emitter.slot(key) + POINTER);
    emitter.load_rsi(emitter.slot(key) + LENGTH);
    emitter.code.emit_call_label("float_parse");
}
pub(super) fn store(emitter: &mut Emitter, key: &str) {
    emitter.code.emit_mov_rdi_rax();
    // Rendering can report FLOAT. Keep the previous destination buffer intact
    // until the runtime has returned successfully, including self assignments.
    emitter.code.emit(&[0x48, 0x83, 0xec, 32, 0x48, 0x89, 0xe6]);
    emitter.code.emit_call_label("float_render");
    emitter.lea_rsi(emitter.slot(key) + BUFFER);
    emitter.code.emit(&[
        0x48, 0x89, 0xf7, 0x48, 0x89, 0xc6, 0x48, 0x89, 0xd1, 0xf3, 0xa4, 0x48, 0x83, 0xc4, 32,
    ]);
    emitter.lea_rsi(emitter.slot(key) + BUFFER);
    emitter.store_rsi(emitter.slot(key) + POINTER);
    emitter.store_rdx(emitter.slot(key) + LENGTH);
    emitter.set(emitter.slot(key) + NUMERIC, 1);
}
pub(super) fn expression(emitter: &mut Emitter, expr: &FloatExpr) -> Result<(), String> {
    match expr {
        FloatExpr::Literal(value) => bits(emitter, *value),
        FloatExpr::Field(key) => read(emitter, key),
        FloatExpr::Negate(value) => {
            expression(emitter, value)?;
            emitter.code.emit_call_label("float_negate");
        }
        FloatExpr::Binary { op, left, right } => {
            expression(emitter, left)?;
            emitter.code.emit(&[0x50]);
            expression(emitter, right)?;
            emitter.code.emit(&[0x48, 0x89, 0xc2, 0x5e, 0xbf]); // right rdx, left rsi, op edi
            emitter.code.emit(&(op.code() as u32).to_le_bytes());
            emitter.code.emit_call_label("float_binary");
        }
    }
    Ok(())
}
pub(super) fn emit_operation(
    emitter: &mut Emitter,
    i: &AilBytecodeInstruction,
) -> Result<(), String> {
    let key = required(i, "key")?;
    match i.opcode.as_str() {
        "EVAL_FLOAT" => {
            expression(
                emitter,
                &FloatExpr::from_postfix(required(i, "expression")?)?,
            )?;
            store(emitter, key);
        }
        "INT_TO_FLOAT" => {
            emitter.integer(required(i, "source")?);
            emitter.code.emit_mov_rdi_rax();
            emitter.code.emit_call_label("float_from_integer");
            store(emitter, key);
        }
        "FLOAT_TO_INT" => {
            read(emitter, required(i, "source")?);
            emitter.code.emit_mov_rdi_rax();
            emitter.code.emit_call_label("float_to_integer");
            emitter.store_integer(key);
        }
        _ => return Err("unsupported native Float operation".into()),
    }
    emitter.operation_trace(&format!("computed floating {key}\n"));
    Ok(())
}
pub(super) fn emit_branch(
    emitter: &mut Emitter,
    i: &AilBytecodeInstruction,
    target: &str,
) -> Result<(), String> {
    let condition = FloatCondition::from_postfix(
        required(i, "left")?,
        required(i, "comparison")?,
        required(i, "right")?,
    )?;
    expression(emitter, &condition.left)?;
    emitter.code.emit(&[0x50]);
    expression(emitter, &condition.right)?;
    emitter.code.emit(&[0x48, 0x89, 0xc2, 0x5e, 0xbf]);
    use crate::condition::IntComparison::*;
    let opcode: u32 = match condition.comparison {
        Equal => 0,
        NotEqual => 1,
        Less => 2,
        LessEqual => 3,
        Greater => 4,
        GreaterEqual => 5,
    };
    emitter.code.emit(&opcode.to_le_bytes());
    emitter.code.emit_call_label("float_compare");
    emitter.code.emit_test_rax_rax();
    emitter.code.emit_jcc_label(&[0x0f, 0x85], target);
    Ok(())
}

// Runtime functions obey SysV. Preserve the pre-alignment stack pointer in a
// reserved aligned slot; it must survive arbitrary caller-saved registers.
pub(super) fn runtime_call(code: &mut X64Code, name: &str) -> Result<(), String> {
    code.emit(&[
        0x49, 0x89, 0xe3, 0x48, 0x83, 0xe4, 0xf0, 0x48, 0x83, 0xec, 0x10, 0x4c, 0x89, 0x1c, 0x24,
        0x48, 0xb8,
    ]);
    code.emit(&native_float_runtime::symbol(name)?.to_le_bytes());
    code.emit(&[0xff, 0xd0, 0x48, 0x8b, 0x24, 0x24]);
    Ok(())
}
fn status(code: &mut X64Code, failure: &str) {
    code.emit_test_rax_rax();
    code.emit_jcc_label(&[0x0f, 0x85], failure);
}
fn finite(code: &mut X64Code) {
    code.emit(&[
        0x48, 0x89, 0xc1, 0x48, 0xc1, 0xe1, 0x01, 0x48, 0xc1, 0xe9, 0x35, 0x48, 0x81, 0xf9, 0xff,
        0x07, 0, 0,
    ]); // exponent == 2047
    code.emit_jcc_label(&[0x0f, 0x84], "error_json");
}
pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    for (label, error, limit) in [
        ("float_parse", "error_float", "error_float_limit"),
        ("json_float_transport", "error_json", "error_json_limit"),
    ] {
        code.label(label)?;
        code.emit(&[0x48, 0x85, 0xff]);
        code.emit_jcc_label(&[0x0f, 0x84], error);
        code.emit(&[0x48, 0x81, 0xfe, 0, 0, 0x10, 0]);
        code.emit_jcc_label(&[0x0f, 0x87], limit);
        code.emit(&[0x48, 0x83, 0xec, 0x08, 0x48, 0x89, 0xe2]);
        runtime_call(code, "ail_float_parse")?;
        code.emit(&[0x83, 0xf8, 0x02]);
        code.emit_jcc_label(&[0x0f, 0x84], limit);
        status(code, error);
        code.emit(&[0x58, 0xc3]);
    }
    code.label("float_render")?; // rdi=bits,rsi=buffer -> rax=buffer,rdx=len
    code.emit(&[0x56, 0xba, 32, 0, 0, 0]);
    runtime_call(code, "ail_float_format")?;
    code.emit(&[0x48, 0x83, 0xf8, 0xff]);
    code.emit_jcc_label(&[0x0f, 0x84], "error_float");
    code.emit(&[0x48, 0x89, 0xc2, 0x58, 0xc3]);
    code.label("float_binary")?;
    code.emit(&[0x48, 0x83, 0xec, 0x08, 0x48, 0x89, 0xe1]);
    runtime_call(code, "ail_float_binary")?;
    code.emit(&[0x83, 0xf8, 0x03]);
    code.emit_jcc_label(&[0x0f, 0x84], "error_divzero");
    status(code, "error_float");
    code.emit(&[0x58, 0xc3]);
    code.label("float_compare")?;
    runtime_call(code, "ail_float_compare")?;
    code.emit(&[0xc3]);
    code.label("float_from_integer")?;
    runtime_call(code, "ail_float_from_integer")?;
    code.emit(&[0xc3]);
    code.label("float_to_integer")?;
    code.emit(&[0x48, 0x83, 0xec, 0x08, 0x48, 0x89, 0xe6]);
    runtime_call(code, "ail_float_to_integer")?;
    status(code, "error_float");
    code.emit(&[0x58, 0xc3]);
    code.label("float_negate")?;
    code.emit(&[0x48, 0xb9]);
    code.emit(&0x8000000000000000_u64.to_le_bytes());
    code.emit(&[0x48, 0x31, 0xc8]);
    code.emit(&[0x48, 0x89, 0xc2, 0x48, 0x0f, 0xba, 0xf2, 0x3f, 0x48, 0xb9]); // btr rdx,63
    code.emit(&0x7ff0000000000000_u64.to_le_bytes());
    code.emit(&[0x48, 0x39, 0xca]);
    code.emit_jcc_label(&[0x0f, 0x86], "float_negate_done");
    code.emit(&[0x48, 0xb8]);
    code.emit(&0x7ff8000000000000_u64.to_le_bytes());
    code.label("float_negate_done")?;
    code.emit(&[0xc3]);

    code.label("json_float_parse")?;
    code.emit_call_label("structured_parse");
    code.emit(&[0x50]);
    code.emit(&[0x48, 0x83, 0xb8, 0, 0x10, 0x30, 0, 0x02]);
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[
        0x48, 0x8b, 0xb0, 0x48, 0x10, 0x30, 0, 0x48, 0x8b, 0xb8, 0x40, 0x10, 0x30, 0,
    ]);
    code.emit_call_label("json_float_transport");
    finite(code);
    code.emit(&[0x5f]);
    code.emit_call_label("structured_release");
    code.emit(&[0xc3]);
    code.label("json_float_format")?;
    code.emit_call_label("json_float_transport");
    finite(code);
    code.emit(&[0x48, 0x83, 0xec, 32, 0x48, 0x89, 0xc7, 0x48, 0x89, 0xe6]);
    code.emit_call_label("float_render");
    code.emit(&[0x48, 0x89, 0xc6]);
    code.emit_call_label("json_copy_text");
    code.emit(&[0x48, 0x83, 0xc4, 32, 0xc3]);
    code.label("float_mutation_format")?;
    code.emit_call_label("json_float_transport");
    finite(code);
    code.emit(&[0x50]);
    code.emit_call_label("json_scratch_allocate");
    code.emit(&[
        0x50, 0x48, 0x8b, 0x7c, 0x24, 0x08, 0x48, 0x8d, 0xb0, 0, 0x10, 0x20, 0,
    ]);
    code.emit_call_label("float_render");
    code.emit(&[0x58, 0x48, 0x89, 0x10, 0x48, 0x83, 0xc4, 0x08, 0xc3]);
    Ok(())
}
