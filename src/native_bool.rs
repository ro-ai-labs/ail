//! Lazy Boolean AST execution. Comparisons evaluate checked operands left to
//! right; Boolean results use static scalar storage and never reserve an arena.
use super::*;
use crate::bool_expression::{BoolExpr, TextOperand};
use crate::condition::IntComparison;

pub(super) fn fields(i: &AilBytecodeInstruction) -> Result<BTreeSet<String>, String> {
    if !matches!(i.opcode.as_str(), "EVAL_BOOL" | "BRANCH_BOOL") {
        return Ok(BTreeSet::new());
    }
    let expression = BoolExpr::from_postfix(required(i, "expression")?)?;
    let mut fields: BTreeSet<_> = expression.typed_fields()?.into_keys().collect();
    if i.opcode == "EVAL_BOOL" {
        fields.insert(required(i, "key")?.into());
    }
    Ok(fields)
}

pub(super) fn emit_operation(
    emitter: &mut Emitter,
    i: &AilBytecodeInstruction,
) -> Result<(), String> {
    let key = required(i, "key")?;
    expression(
        emitter,
        &BoolExpr::from_postfix(required(i, "expression")?)?,
    )?;
    store(emitter, key)?;
    emitter.operation_trace(&format!("computed boolean {key}\n"));
    Ok(())
}

pub(super) fn emit_branch(
    emitter: &mut Emitter,
    i: &AilBytecodeInstruction,
    target: &str,
) -> Result<(), String> {
    expression(
        emitter,
        &BoolExpr::from_postfix(required(i, "expression")?)?,
    )?;
    emitter.code.emit_test_rax_rax();
    emitter.code.emit_jcc_label(&[0x0f, 0x85], target);
    Ok(())
}

fn store(emitter: &mut Emitter, key: &str) -> Result<(), String> {
    let no = format!("bool_store_false_{}", emitter.next);
    let done = format!("bool_store_done_{}", emitter.next);
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
    Ok(())
}

fn set_comparison(code: &mut X64Code, comparison: IntComparison) {
    use IntComparison::*;
    let setcc = match comparison {
        Equal => 0x94,
        NotEqual => 0x95,
        Less => 0x9c,
        LessEqual => 0x9e,
        Greater => 0x9f,
        GreaterEqual => 0x9d,
    };
    code.emit(&[0x0f, setcc, 0xc0, 0x0f, 0xb6, 0xc0]);
}

fn text_operand(emitter: &mut Emitter, operand: &TextOperand) {
    match operand {
        TextOperand::Literal(value) => {
            let data = emitter.data(value.as_bytes());
            emitter.code.emit_lea_rsi_label(&data);
            emitter.code.emit(&[0x48, 0x89, 0xf7]);
            emitter.code.emit_mov_esi_imm32(value.len() as u32);
        }
        TextOperand::Field(key) => {
            emitter.load_rdi(emitter.slot(key) + POINTER);
            emitter.load_rsi(emitter.slot(key) + LENGTH);
            emitter.code.emit_call_label("text_validate");
        }
    }
}

fn expression(emitter: &mut Emitter, expr: &BoolExpr) -> Result<(), String> {
    match expr {
        BoolExpr::Literal(value) => emitter.code.emit(&[0xb8, u8::from(*value), 0, 0, 0]),
        BoolExpr::Field(key) => {
            emitter.load_rdi(emitter.slot(key) + POINTER);
            emitter.load_rsi(emitter.slot(key) + LENGTH);
            emitter.code.emit_call_label("bool_parse");
        }
        BoolExpr::Not(value) => {
            expression(emitter, value)?;
            emitter.code.emit(&[0x48, 0x83, 0xf0, 0x01]);
        }
        BoolExpr::And { left, right } | BoolExpr::Or { left, right } => {
            let done = format!("bool_short_circuit_{}", emitter.next);
            emitter.next += 1;
            expression(emitter, left)?;
            emitter.code.emit_test_rax_rax();
            let skip = if matches!(expr, BoolExpr::And { .. }) {
                0x84
            } else {
                0x85
            };
            emitter.code.emit_jcc_label(&[0x0f, skip], &done);
            expression(emitter, right)?;
            emitter.code.label(done)?;
        }
        BoolExpr::CompareInt(condition) => {
            emitter.expression(&condition.left)?;
            emitter.code.emit(&[0x50]);
            emitter.expression(&condition.right)?;
            emitter.code.emit(&[0x59, 0x48, 0x39, 0xc1]);
            set_comparison(&mut emitter.code, condition.comparison);
        }
        BoolExpr::CompareFloat(condition) => {
            native_float::expression(emitter, &condition.left)?;
            emitter.code.emit(&[0x50]);
            native_float::expression(emitter, &condition.right)?;
            emitter.code.emit(&[0x48, 0x89, 0xc2, 0x5e, 0xbf]);
            use IntComparison::*;
            let comparison: u32 = match condition.comparison {
                Equal => 0,
                NotEqual => 1,
                Less => 2,
                LessEqual => 3,
                Greater => 4,
                GreaterEqual => 5,
            };
            emitter.code.emit(&comparison.to_le_bytes());
            emitter.code.emit_call_label("float_compare");
        }
        BoolExpr::CompareText {
            left,
            comparison,
            right,
        } => {
            text_operand(emitter, left);
            emitter.code.emit(&[0x57, 0x56]);
            text_operand(emitter, right);
            emitter
                .code
                .emit(&[0x49, 0x89, 0xf8, 0x49, 0x89, 0xf1, 0x5e, 0x5f]);
            emitter.code.emit_call_label("bool_text_compare");
            emitter.code.emit_test_rax_rax();
            set_comparison(&mut emitter.code, *comparison);
        }
        BoolExpr::CompareBool {
            left,
            comparison,
            right,
        } => {
            expression(emitter, left)?;
            emitter.code.emit(&[0x50]);
            expression(emitter, right)?;
            emitter.code.emit(&[0x59, 0x48, 0x39, 0xc1]);
            set_comparison(&mut emitter.code, *comparison);
        }
    }
    Ok(())
}

pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    code.label("bool_parse")?;
    code.emit(&[0x48, 0x85, 0xff]);
    code.emit_jcc_label(&[0x0f, 0x84], "error_bool");
    code.emit(&[0x48, 0x83, 0xfe, 0x04]);
    code.emit_jcc_label(&[0x0f, 0x85], "bool_parse_false");
    code.emit(&[0x81, 0x3f, b't', b'r', b'u', b'e']);
    code.emit_jcc_label(&[0x0f, 0x85], "error_bool");
    code.emit(&[0xb8, 1, 0, 0, 0, 0xc3]);
    code.label("bool_parse_false")?;
    code.emit(&[0x48, 0x83, 0xfe, 0x05]);
    code.emit_jcc_label(&[0x0f, 0x85], "error_bool");
    code.emit(&[0x81, 0x3f, b'f', b'a', b'l', b's']);
    code.emit_jcc_label(&[0x0f, 0x85], "error_bool");
    code.emit(&[0x80, 0x7f, 0x04, b'e']);
    code.emit_jcc_label(&[0x0f, 0x85], "error_bool");
    code.emit(&[0x31, 0xc0, 0xc3]);

    // Validated left RDI/RSI and right R8/R9. Unsigned UTF-8 byte ordering
    // agrees with Unicode scalar ordering. Compare explicit lengths for NUL.
    code.label("bool_text_compare")?;
    code.emit(&[0x48, 0x85, 0xf6]);
    code.emit_jcc_label(&[0x0f, 0x84], "bool_text_left_end");
    code.emit(&[0x4d, 0x85, 0xc9]);
    code.emit_jcc_label(&[0x0f, 0x84], "bool_text_greater");
    code.emit(&[0x0f, 0xb6, 0x07, 0x41, 0x0f, 0xb6, 0x08, 0x39, 0xc8]);
    code.emit_jcc_label(&[0x0f, 0x82], "bool_text_less");
    code.emit_jcc_label(&[0x0f, 0x87], "bool_text_greater");
    code.emit(&[
        0x48, 0xff, 0xc7, 0x49, 0xff, 0xc0, 0x48, 0xff, 0xce, 0x49, 0xff, 0xc9,
    ]);
    code.emit_jmp_label("bool_text_compare");
    code.label("bool_text_left_end")?;
    code.emit(&[0x4d, 0x85, 0xc9]);
    code.emit_jcc_label(&[0x0f, 0x85], "bool_text_less");
    code.emit(&[0x31, 0xc0, 0xc3]);
    code.label("bool_text_less")?;
    code.emit(&[0x48, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff, 0xc3]);
    code.label("bool_text_greater")?;
    code.emit(&[0xb8, 1, 0, 0, 0, 0xc3]);
    Ok(())
}
