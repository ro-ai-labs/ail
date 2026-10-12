//! Native flat typed lists. Scratch stores scalar bits and decoded text;
//! producing operations copy canonical transport into an immutable arena.
//! RBP+16/+24 hold the root list arena pointer and cumulative allocation charge.
//! Helpers preserve RBX, RBP and R12–R15, including action and function fuel.
use super::*;

fn element_type(instruction: &AilBytecodeInstruction) -> Result<u32, String> {
    match required(instruction, "type")? {
        "Int" => Ok(0),
        "Text" => Ok(1),
        "Float" => Ok(2),
        _ => Err("native list element type must be Int, Text or Float".into()),
    }
}

pub(super) fn fields(instruction: &AilBytecodeInstruction) -> Result<BTreeSet<String>, String> {
    let opcode = instruction.opcode.as_str();
    if !matches!(
        opcode,
        "LIST_EMPTY"
            | "LIST_LENGTH"
            | "LIST_GET"
            | "LIST_APPEND"
            | "LIST_SET"
            | "LIST_CONCAT"
            | "LIST_SLICE"
            | "LIST_REVERSE"
            | "LIST_SORT"
    ) {
        return Ok(BTreeSet::new());
    }
    let kind = element_type(instruction)?;
    let text = kind == 1;
    let mut fields = BTreeSet::new();
    let mut operands = vec!["key"];
    if opcode != "LIST_EMPTY" {
        operands.push("source");
    }
    if opcode == "LIST_CONCAT" {
        operands.push("other");
    }
    if text && matches!(opcode, "LIST_APPEND" | "LIST_SET") {
        operands.push("value");
    }
    for operand in operands {
        let key = required(instruction, operand)?;
        if !matches!(IntExpr::parse(key)?, IntExpr::Field(parsed) if parsed == key) {
            return Err("native list operand must be a field key".into());
        }
        fields.insert(key.to_string());
    }
    if matches!(opcode, "LIST_GET" | "LIST_SET") {
        fields.extend(IntExpr::from_postfix(required(instruction, "index")?)?.fields());
    }
    if opcode == "LIST_SLICE" {
        for bound in ["start", "end"] {
            fields.extend(IntExpr::from_postfix(required(instruction, bound)?)?.fields());
        }
    }
    if kind == 0 && matches!(opcode, "LIST_APPEND" | "LIST_SET") {
        fields.extend(IntExpr::from_postfix(required(instruction, "value")?)?.fields());
    }
    if kind == 2 && matches!(opcode, "LIST_APPEND" | "LIST_SET") {
        fields.extend(
            crate::float_expression::FloatExpr::from_postfix(required(instruction, "value")?)?
                .fields(),
        );
    }
    Ok(fields)
}

pub(super) fn emit_operation(
    emitter: &mut Emitter,
    instruction: &AilBytecodeInstruction,
) -> Result<(), String> {
    let kind = element_type(instruction)?;
    let text = kind == 1;
    let opcode = instruction.opcode.as_str();
    let key = required(instruction, "key")?;
    if opcode == "LIST_EMPTY" {
        emitter.code.emit_call_label("list_empty");
    } else {
        let source = emitter.slot(required(instruction, "source")?);
        emitter.load_rdi(source + POINTER);
        emitter.load_rsi(source + LENGTH);
        emitter.code.emit(&[0xba]); // mov edx, type
        emitter.code.emit(&kind.to_le_bytes());
        emitter.code.emit_call_label("list_parse");
        emitter.code.emit(&[0x50]); // save scratch mapping
        if opcode == "LIST_CONCAT" {
            let other = emitter.slot(required(instruction, "other")?);
            emitter.load_rdi(other + POINTER);
            emitter.load_rsi(other + LENGTH);
            emitter.code.emit(&[0xba]); // mov edx, type
            emitter.code.emit(&kind.to_le_bytes());
            emitter.code.emit_call_label("list_parse");
            // Both inputs are validated before checking the combined result.
            emitter
                .code
                .emit(&[0x48, 0x89, 0xc6, 0x48, 0x8b, 0x3c, 0x24, 0x50]);
            emitter.code.emit_call_label("list_transform_concat");
            emitter.code.emit(&[0x5f]); // release second scratch after owned copy
            emitter.code.emit_call_label("list_release");
        } else if opcode == "LIST_SLICE" {
            emitter.expression(&IntExpr::from_postfix(required(instruction, "start")?)?)?;
            emitter.code.emit(&[0x50]); // start; evaluate end even for an empty slice
            emitter.expression(&IntExpr::from_postfix(required(instruction, "end")?)?)?;
            emitter
                .code
                .emit(&[0x48, 0x89, 0xc2, 0x5e, 0x48, 0x8b, 0x3c, 0x24]);
            emitter.code.emit_call_label("list_transform_slice");
        } else if opcode == "LIST_REVERSE" || opcode == "LIST_SORT" {
            emitter.code.emit(&[0x48, 0x8b, 0x3c, 0x24]); // rdi=scratch
            if opcode == "LIST_SORT" {
                let descending: u32 = match required(instruction, "order")? {
                    "ascending" => 0,
                    "descending" => 1,
                    _ => {
                        return Err("native list sort order must be ascending or descending".into());
                    }
                };
                emitter.code.emit(&[0xbe]); // esi=descending
                emitter.code.emit(&descending.to_le_bytes());
                emitter.code.emit_call_label("list_transform_sort");
            } else {
                emitter.code.emit_call_label("list_transform_reverse");
            }
        } else if opcode == "LIST_LENGTH" {
            emitter.code.emit(&[0x48, 0x8b, 0x00]); // mov rax, [rax] (count)
        } else {
            if opcode == "LIST_APPEND" {
                emitter.code.emit(&[0x48, 0x8b, 0x00]); // append index = count
            } else {
                emitter.expression(&IntExpr::from_postfix(required(instruction, "index")?)?)?;
                emitter.code.emit(&[0x48, 0x8b, 0x3c, 0x24]); // mov rdi, [rsp]
                emitter.code.emit_call_label("list_index");
            }
            if opcode == "LIST_GET" {
                emitter.code.emit(&[0x48, 0x8b, 0x3c, 0x24]); // mov rdi, [rsp]
                emitter.code.emit_call_label(if text {
                    "list_get_text"
                } else {
                    "list_get_int"
                });
            } else {
                emitter.code.emit(&[0x50]); // save normalized replacement/append index
                if text {
                    let value = emitter.slot(required(instruction, "value")?);
                    emitter.load_rdi(value + POINTER);
                    emitter.load_rsi(value + LENGTH);
                    emitter.code.emit_call_label("text_validate");
                    emitter.code.emit(&[0x48, 0x89, 0xfa, 0x48, 0x89, 0xf1]); // rdx=pointer, rcx=length
                } else {
                    if kind == 2 {
                        native_float::expression(
                            emitter,
                            &crate::float_expression::FloatExpr::from_postfix(required(
                                instruction,
                                "value",
                            )?)?,
                        )?;
                    } else {
                        emitter
                            .expression(&IntExpr::from_postfix(required(instruction, "value")?)?)?;
                    }
                    emitter.code.emit(&[0x48, 0x89, 0xc2, 0x31, 0xc9]); // rdx=value, ecx=0
                }
                emitter.code.emit(&[0x5e, 0x48, 0x8b, 0x3c, 0x24]); // pop rsi=index; rdi=scratch
                emitter.code.emit_call_label("list_build");
            }
        }
        emitter.code.emit(&[0x5f]); // pop scratch mapping
        emitter.code.emit_call_label("list_release"); // preserves result rax/rdx
        if opcode == "LIST_GET" && kind == 2 {
            native_float::store(emitter, key);
            return Ok(());
        }
        if opcode == "LIST_LENGTH" || (opcode == "LIST_GET" && !text) {
            emitter.store_integer(key);
            return Ok(());
        }
    }
    let destination = emitter.slot(key);
    emitter.store_rax(destination + POINTER);
    emitter.store_rdx(destination + LENGTH);
    emitter.set(destination + NUMERIC, 0);
    Ok(())
}

// Cursor R12..R13, scratch R14: scan a bounded flat numeric token, then use
// the shared allocation-free binary64 parser. All enclosing parser state lives
// in callee-saved registers across the aligned SysV call.
fn emit_float_parser(code: &mut X64Code) -> Result<(), String> {
    code.label("list_parse_float")?;
    code.emit(&[0x4c, 0x89, 0xe7]); // rdi=token start
    code.label("list_float_scan")?;
    code.emit(&[0x4d, 0x39, 0xec]); // cursor=end
    code.emit_jcc_label(&[0x0f, 0x83], "list_float_token");
    code.emit(&[0x41, 0x8a, 0x04, 0x24]);
    for delimiter in [b' ', b'\t', b'\n', b'\r', b',', b']'] {
        code.emit(&[0x3c, delimiter]);
        code.emit_jcc_label(&[0x0f, 0x84], "list_float_token");
    }
    code.emit(&[0x49, 0xff, 0xc4]);
    code.emit_jmp_label("list_float_scan");
    code.label("list_float_token")?;
    code.emit(&[0x4c, 0x89, 0xe6, 0x48, 0x29, 0xfe]); // rsi=token length
    code.emit(&[0x48, 0x83, 0xec, 0x08, 0x48, 0x89, 0xe2]); // output bits
    code.emit(&[0x49, 0x83, 0x7e, 0x08, 0x03]);
    code.emit_jcc_label(&[0x0f, 0x84], "list_float_strict");
    native_float::runtime_call(code, "ail_float_parse")?;
    code.emit_jmp_label("list_float_status");
    code.label("list_float_strict")?;
    native_float::runtime_call(code, "ail_float_parse_json")?;
    code.label("list_float_status")?;
    code.emit(&[0x83, 0xf8, 0x02]);
    code.emit_jcc_label(&[0x0f, 0x84], "error_list_limit");
    code.emit_test_rax_rax();
    code.emit_jcc_label(&[0x0f, 0x85], "error_list");
    code.emit(&[0x58]);
    code.emit(&[0x31, 0xd2, 0xc3]);
    Ok(())
}

pub(super) fn emit_helpers(code: &mut X64Code, has_float: bool) -> Result<(), String> {
    // RDI/RSI transport, EDX mode: 0 Int, 1 Text, 2 Float, 3 strict finite
    // JSON Float, 4 finite runtime Float. Modes >=2 share bit descriptors.
    // The mapping is 3 MiB + 4 KiB: header, 65,536 16-byte descriptors, decoded
    // text, and canonical output. A descriptor holds i64/unused or pointer/length.
    // Decoded text cannot exceed the validated input size. Canonical expansion
    // is checked only after replacing/appending the requested descriptor.
    code.label("list_parse")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit(&[0x52]); // push   rdx
    code.emit(&[0x57]); // push   rdi
    code.emit(&[0x56]); // push   rsi
    code.emit_call_label("list_validate_utf8");
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x4d, 0x01, 0xe5]); // add    r13,r12
    code.emit(&[0xb8, 0x09, 0x00, 0x00, 0x00]); // mov    eax,0x9
    code.emit(&[0x31, 0xff]); // xor    edi,edi
    code.emit(&[0xbe, 0x00, 0x10, 0x30, 0x00]); // mov    esi,0x301000
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov    edx,0x3
    code.emit(&[0x41, 0xba, 0x22, 0x00, 0x00, 0x00]); // mov    r10d,0x22
    code.emit(&[0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]); // mov    r8,0xffffffffffffffff
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.emit_call_label("recovery_temp_map");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_list_allocation");
    code.emit(&[0x49, 0x89, 0xc6]); // mov    r14,rax
    code.emit(&[0x4d, 0x89, 0x7e, 0x08]); // mov    QWORD PTR [r14+0x8],r15
    code.emit(&[0x4d, 0x8d, 0xbe, 0x00, 0x10, 0x10, 0x00]); // lea    r15,[r14+0x101000]
    code.emit(&[0x31, 0xdb]); // xor    ebx,ebx
    code.emit_call_label("list_skip_space");
    code.emit(&[0x3c, 0x5b]); // cmp    al,0x5b
    code.emit_jcc_label(&[0x0f, 0x85], "error_list");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit_call_label("list_skip_space");
    code.emit(&[0x3c, 0x5d]); // cmp    al,0x5d
    code.emit_jcc_label(&[0x0f, 0x84], "list_parse_close");
    code.label("list_parse_item")?;
    code.emit(&[0x48, 0x81, 0xfb, 0x00, 0x00, 0x01, 0x00]); // cmp    rbx,0x10000
    code.emit_jcc_label(&[0x0f, 0x83], "error_list_limit");
    if has_float {
        code.emit(&[0x49, 0x83, 0x7e, 0x08, 0x02]);
        code.emit_jcc_label(&[0x0f, 0x83], "list_parse_float_item");
    }
    code.emit(&[0x49, 0x83, 0x7e, 0x08, 0x00]); // cmp    QWORD PTR [r14+0x8],0x0
    code.emit_jcc_label(&[0x0f, 0x85], "list_parse_text");
    code.emit_call_label("list_parse_int");
    code.emit_jmp_label("list_parse_store");
    code.label("list_parse_text")?;
    code.emit_call_label("list_parse_string");
    if has_float {
        code.emit_jmp_label("list_parse_store");
        code.label("list_parse_float_item")?;
        code.emit_call_label("list_parse_float");
    }
    code.label("list_parse_store")?;
    code.emit(&[0x48, 0x89, 0xd9]); // mov    rcx,rbx
    code.emit(&[0x48, 0xc1, 0xe1, 0x04]); // shl    rcx,0x4
    code.emit(&[0x49, 0x89, 0x84, 0x0e, 0x00, 0x10, 0x00, 0x00]); // mov    QWORD PTR [r14+rcx*1+0x1000],rax
    code.emit(&[0x49, 0x89, 0x94, 0x0e, 0x08, 0x10, 0x00, 0x00]); // mov    QWORD PTR [r14+rcx*1+0x1008],rdx
    code.emit(&[0x48, 0xff, 0xc3]); // inc    rbx
    code.emit_call_label("list_skip_space");
    code.emit(&[0x3c, 0x5d]); // cmp    al,0x5d
    code.emit_jcc_label(&[0x0f, 0x84], "list_parse_close");
    code.emit(&[0x3c, 0x2c]); // cmp    al,0x2c
    code.emit_jcc_label(&[0x0f, 0x85], "error_list");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit_call_label("list_skip_space");
    code.emit_jmp_label("list_parse_item");
    code.label("list_parse_close")?;
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit_call_label("list_skip_space");
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x85], "error_list");
    code.emit(&[0x49, 0x89, 0x1e]); // mov    QWORD PTR [r14],rbx
    if has_float {
        // Formatting first validates all runtime members and the flat count.
        // Only then reject nonfinite members, before any canonical-size check.
        code.emit(&[0x49, 0x83, 0x7e, 0x08, 0x04]);
        code.emit_jcc_label(&[0x0f, 0x85], "list_parse_return");
        code.emit(&[0x31, 0xc9]);
        code.label("list_finite_loop")?;
        code.emit(&[0x48, 0x39, 0xd9]); // rcx versus count
        code.emit_jcc_label(&[0x0f, 0x83], "list_parse_return");
        code.emit(&[0x48, 0x89, 0xca, 0x48, 0xc1, 0xe2, 0x04]);
        code.emit(&[0x49, 0x8b, 0x84, 0x16, 0x00, 0x10, 0x00, 0x00]);
        code.emit(&[
            0x48, 0xc1, 0xe0, 0x01, 0x48, 0xc1, 0xe8, 0x35, 0x48, 0x3d, 0xff, 0x07, 0, 0,
        ]);
        code.emit_jcc_label(&[0x0f, 0x84], "error_list");
        code.emit(&[0x48, 0xff, 0xc1]);
        code.emit_jmp_label("list_finite_loop");
        code.label("list_parse_return")?;
    }
    code.emit(&[0x4c, 0x89, 0xf0]); // mov    rax,r14
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret

    if has_float {
        emit_float_parser(code)?;
    }

    // JSON permits only ASCII space, tab, CR and LF outside strings.
    code.label("list_skip_space")?;
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x83], "list_skip_end");
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x3c, 0x20]); // cmp    al,0x20
    code.emit_jcc_label(&[0x0f, 0x84], "list_skip_advance");
    code.emit(&[0x3c, 0x09]); // cmp    al,0x9
    code.emit_jcc_label(&[0x0f, 0x84], "list_skip_advance");
    code.emit(&[0x3c, 0x0a]); // cmp    al,0xa
    code.emit_jcc_label(&[0x0f, 0x84], "list_skip_advance");
    code.emit(&[0x3c, 0x0d]); // cmp    al,0xd
    code.emit_jcc_label(&[0x0f, 0x85], "list_skip_return");
    code.label("list_skip_advance")?;
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit_jmp_label("list_skip_space");
    code.label("list_skip_end")?;
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.label("list_skip_return")?;
    code.emit(&[0xc3]); // ret

    // Accumulate negatively to accept i64::MIN. The first zero ends the token;
    // the enclosing delimiter check rejects leading zeroes and noninteger forms.
    code.label("list_parse_int")?;
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.emit(&[0x45, 0x31, 0xc0]); // xor    r8d,r8d
    code.emit(&[0x41, 0x80, 0x3c, 0x24, 0x2d]); // cmp    BYTE PTR [r12],0x2d
    code.emit_jcc_label(&[0x0f, 0x85], "list_int_first");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x41, 0xb8, 0x01, 0x00, 0x00, 0x00]); // mov    r8d,0x1
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.label("list_int_first")?;
    code.emit(&[0x41, 0x0f, 0xb6, 0x0c, 0x24]); // movzx  ecx,BYTE PTR [r12]
    code.emit(&[0x83, 0xe9, 0x30]); // sub    ecx,0x30
    code.emit(&[0x83, 0xf9, 0x09]); // cmp    ecx,0x9
    code.emit_jcc_label(&[0x0f, 0x87], "error_list");
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.emit(&[0x85, 0xc9]); // test   ecx,ecx
    code.emit_jcc_label(&[0x0f, 0x85], "list_int_digit");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit_jmp_label("list_int_finish");
    code.label("list_int_digit")?;
    code.emit(&[0x48, 0x6b, 0xc0, 0x0a]); // imul   rax,rax,0xa
    code.emit_jcc_label(&[0x0f, 0x80], "error_list");
    code.emit(&[0x48, 0x29, 0xc8]); // sub    rax,rcx
    code.emit_jcc_label(&[0x0f, 0x80], "error_list");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x83], "list_int_finish");
    code.emit(&[0x41, 0x0f, 0xb6, 0x0c, 0x24]); // movzx  ecx,BYTE PTR [r12]
    code.emit(&[0x83, 0xe9, 0x30]); // sub    ecx,0x30
    code.emit(&[0x83, 0xf9, 0x09]); // cmp    ecx,0x9
    code.emit_jcc_label(&[0x0f, 0x86], "list_int_digit");
    code.label("list_int_finish")?;
    code.emit(&[0x45, 0x85, 0xc0]); // test   r8d,r8d
    code.emit_jcc_label(&[0x0f, 0x85], "list_int_return");
    code.emit(&[0x48, 0xf7, 0xd8]); // neg    rax
    code.emit_jcc_label(&[0x0f, 0x80], "error_list");
    code.label("list_int_return")?;
    code.emit(&[0x31, 0xd2]); // xor    edx,edx
    code.emit(&[0xc3]); // ret

    // Transport UTF-8 is already valid. Decode escapes without recursion and
    // require a valid low surrogate immediately after every high surrogate.
    code.label("list_parse_string")?;
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.emit(&[0x41, 0x80, 0x3c, 0x24, 0x22]); // cmp    BYTE PTR [r12],0x22
    code.emit_jcc_label(&[0x0f, 0x85], "error_list");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x41, 0x57]); // push   r15
    code.label("list_string_loop")?;
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.emit(&[0x41, 0x0f, 0xb6, 0x04, 0x24]); // movzx  eax,BYTE PTR [r12]
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x3c, 0x22]); // cmp    al,0x22
    code.emit_jcc_label(&[0x0f, 0x84], "list_string_done");
    code.emit(&[0x3c, 0x20]); // cmp    al,0x20
    code.emit_jcc_label(&[0x0f, 0x82], "error_list");
    code.emit(&[0x3c, 0x5c]); // cmp    al,0x5c
    code.emit_jcc_label(&[0x0f, 0x84], "list_string_escape");
    code.label("list_string_byte")?;
    code.emit(&[0x41, 0x88, 0x07]); // mov    BYTE PTR [r15],al
    code.emit(&[0x49, 0xff, 0xc7]); // inc    r15
    code.emit_jmp_label("list_string_loop");
    code.label("list_string_escape")?;
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.emit(&[0x41, 0x0f, 0xb6, 0x04, 0x24]); // movzx  eax,BYTE PTR [r12]
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x3c, 0x22]); // cmp    al,0x22
    code.emit_jcc_label(&[0x0f, 0x84], "list_string_byte");
    code.emit(&[0x3c, 0x5c]); // cmp    al,0x5c
    code.emit_jcc_label(&[0x0f, 0x84], "list_string_byte");
    code.emit(&[0x3c, 0x2f]); // cmp    al,0x2f
    code.emit_jcc_label(&[0x0f, 0x84], "list_string_byte");
    code.emit(&[0x3c, 0x62]); // cmp    al,0x62
    code.emit_jcc_label(&[0x0f, 0x85], "list_string_f");
    code.emit(&[0xb0, 0x08]); // mov    al,0x8
    code.emit_jmp_label("list_string_byte");
    code.label("list_string_f")?;
    code.emit(&[0x3c, 0x66]); // cmp    al,0x66
    code.emit_jcc_label(&[0x0f, 0x85], "list_string_n");
    code.emit(&[0xb0, 0x0c]); // mov    al,0xc
    code.emit_jmp_label("list_string_byte");
    code.label("list_string_n")?;
    code.emit(&[0x3c, 0x6e]); // cmp    al,0x6e
    code.emit_jcc_label(&[0x0f, 0x85], "list_string_r");
    code.emit(&[0xb0, 0x0a]); // mov    al,0xa
    code.emit_jmp_label("list_string_byte");
    code.label("list_string_r")?;
    code.emit(&[0x3c, 0x72]); // cmp    al,0x72
    code.emit_jcc_label(&[0x0f, 0x85], "list_string_t");
    code.emit(&[0xb0, 0x0d]); // mov    al,0xd
    code.emit_jmp_label("list_string_byte");
    code.label("list_string_t")?;
    code.emit(&[0x3c, 0x74]); // cmp    al,0x74
    code.emit_jcc_label(&[0x0f, 0x85], "list_string_unicode");
    code.emit(&[0xb0, 0x09]); // mov    al,0x9
    code.emit_jmp_label("list_string_byte");
    code.label("list_string_unicode")?;
    code.emit(&[0x3c, 0x75]); // cmp    al,0x75
    code.emit_jcc_label(&[0x0f, 0x85], "error_list");
    code.emit_call_label("list_hex_quad");
    code.emit(&[0x3d, 0x00, 0xd8, 0x00, 0x00]); // cmp    eax,0xd800
    code.emit_jcc_label(&[0x0f, 0x82], "list_string_codepoint");
    code.emit(&[0x3d, 0x00, 0xe0, 0x00, 0x00]); // cmp    eax,0xe000
    code.emit_jcc_label(&[0x0f, 0x83], "list_string_codepoint");
    code.emit(&[0x3d, 0x00, 0xdc, 0x00, 0x00]); // cmp    eax,0xdc00
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.emit(&[0x50]); // push   rax
    code.emit(&[0x49, 0x8d, 0x4c, 0x24, 0x02]); // lea    rcx,[r12+0x2]
    code.emit(&[0x4c, 0x39, 0xe9]); // cmp    rcx,r13
    code.emit_jcc_label(&[0x0f, 0x87], "error_list");
    code.emit(&[0x66, 0x41, 0x81, 0x3c, 0x24, 0x5c, 0x75]); // cmp    WORD PTR [r12],0x755c
    code.emit_jcc_label(&[0x0f, 0x85], "error_list");
    code.emit(&[0x49, 0x83, 0xc4, 0x02]); // add    r12,0x2
    code.emit_call_label("list_hex_quad");
    code.emit(&[0x3d, 0x00, 0xdc, 0x00, 0x00]); // cmp    eax,0xdc00
    code.emit_jcc_label(&[0x0f, 0x82], "error_list");
    code.emit(&[0x3d, 0x00, 0xe0, 0x00, 0x00]); // cmp    eax,0xe000
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.emit(&[0x2d, 0x00, 0xdc, 0x00, 0x00]); // sub    eax,0xdc00
    code.emit(&[0x59]); // pop    rcx
    code.emit(&[0x81, 0xe9, 0x00, 0xd8, 0x00, 0x00]); // sub    ecx,0xd800
    code.emit(&[0xc1, 0xe1, 0x0a]); // shl    ecx,0xa
    code.emit(&[0x01, 0xc8]); // add    eax,ecx
    code.emit(&[0x05, 0x00, 0x00, 0x01, 0x00]); // add    eax,0x10000
    code.label("list_string_codepoint")?;
    code.emit(&[0x3d, 0x80, 0x00, 0x00, 0x00]); // cmp    eax,0x80
    code.emit_jcc_label(&[0x0f, 0x82], "list_string_byte");
    code.emit(&[0x89, 0xc2]); // mov    edx,eax
    code.emit(&[0x3d, 0x00, 0x08, 0x00, 0x00]); // cmp    eax,0x800
    code.emit_jcc_label(&[0x0f, 0x83], "list_string_three");
    code.emit(&[0xc1, 0xe8, 0x06]); // shr    eax,0x6
    code.emit(&[0x0c, 0xc0]); // or     al,0xc0
    code.emit(&[0x41, 0x88, 0x07]); // mov    BYTE PTR [r15],al
    code.emit(&[0x49, 0xff, 0xc7]); // inc    r15
    code.emit_jmp_label("list_string_last");
    code.label("list_string_three")?;
    code.emit(&[0x3d, 0x00, 0x00, 0x01, 0x00]); // cmp    eax,0x10000
    code.emit_jcc_label(&[0x0f, 0x83], "list_string_four");
    code.emit(&[0xc1, 0xe8, 0x0c]); // shr    eax,0xc
    code.emit(&[0x0c, 0xe0]); // or     al,0xe0
    code.emit(&[0x41, 0x88, 0x07]); // mov    BYTE PTR [r15],al
    code.emit(&[0x49, 0xff, 0xc7]); // inc    r15
    code.emit_jmp_label("list_string_middle");
    code.label("list_string_four")?;
    code.emit(&[0xc1, 0xe8, 0x12]); // shr    eax,0x12
    code.emit(&[0x0c, 0xf0]); // or     al,0xf0
    code.emit(&[0x41, 0x88, 0x07]); // mov    BYTE PTR [r15],al
    code.emit(&[0x49, 0xff, 0xc7]); // inc    r15
    code.emit(&[0x89, 0xd0]); // mov    eax,edx
    code.emit(&[0xc1, 0xe8, 0x0c]); // shr    eax,0xc
    code.emit(&[0x24, 0x3f]); // and    al,0x3f
    code.emit(&[0x0c, 0x80]); // or     al,0x80
    code.emit(&[0x41, 0x88, 0x07]); // mov    BYTE PTR [r15],al
    code.emit(&[0x49, 0xff, 0xc7]); // inc    r15
    code.label("list_string_middle")?;
    code.emit(&[0x89, 0xd0]); // mov    eax,edx
    code.emit(&[0xc1, 0xe8, 0x06]); // shr    eax,0x6
    code.emit(&[0x24, 0x3f]); // and    al,0x3f
    code.emit(&[0x0c, 0x80]); // or     al,0x80
    code.emit(&[0x41, 0x88, 0x07]); // mov    BYTE PTR [r15],al
    code.emit(&[0x49, 0xff, 0xc7]); // inc    r15
    code.label("list_string_last")?;
    code.emit(&[0x89, 0xd0]); // mov    eax,edx
    code.emit(&[0x24, 0x3f]); // and    al,0x3f
    code.emit(&[0x0c, 0x80]); // or     al,0x80
    code.emit_jmp_label("list_string_byte");
    code.label("list_string_done")?;
    code.emit(&[0x58]); // pop    rax
    code.emit(&[0x4c, 0x89, 0xfa]); // mov    rdx,r15
    code.emit(&[0x48, 0x29, 0xc2]); // sub    rdx,rax
    code.emit(&[0xc3]); // ret
    code.label("list_hex_quad")?;
    code.emit(&[0x49, 0x8d, 0x4c, 0x24, 0x04]); // lea    rcx,[r12+0x4]
    code.emit(&[0x4c, 0x39, 0xe9]); // cmp    rcx,r13
    code.emit_jcc_label(&[0x0f, 0x87], "error_list");
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.emit(&[0x41, 0xb9, 0x04, 0x00, 0x00, 0x00]); // mov    r9d,0x4
    code.label("list_hex_digit")?;
    code.emit(&[0x41, 0x0f, 0xb6, 0x0c, 0x24]); // movzx  ecx,BYTE PTR [r12]
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x83, 0xf9, 0x30]); // cmp    ecx,0x30
    code.emit_jcc_label(&[0x0f, 0x82], "error_list");
    code.emit(&[0x83, 0xf9, 0x39]); // cmp    ecx,0x39
    code.emit_jcc_label(&[0x0f, 0x86], "list_hex_decimal");
    code.emit(&[0x83, 0xf9, 0x41]); // cmp    ecx,0x41
    code.emit_jcc_label(&[0x0f, 0x82], "error_list");
    code.emit(&[0x83, 0xf9, 0x46]); // cmp    ecx,0x46
    code.emit_jcc_label(&[0x0f, 0x86], "list_hex_upper");
    code.emit(&[0x83, 0xf9, 0x61]); // cmp    ecx,0x61
    code.emit_jcc_label(&[0x0f, 0x82], "error_list");
    code.emit(&[0x83, 0xf9, 0x66]); // cmp    ecx,0x66
    code.emit_jcc_label(&[0x0f, 0x87], "error_list");
    code.emit(&[0x83, 0xe9, 0x57]); // sub    ecx,0x57
    code.emit_jmp_label("list_hex_accumulate");
    code.label("list_hex_upper")?;
    code.emit(&[0x83, 0xe9, 0x37]); // sub    ecx,0x37
    code.emit_jmp_label("list_hex_accumulate");
    code.label("list_hex_decimal")?;
    code.emit(&[0x83, 0xe9, 0x30]); // sub    ecx,0x30
    code.label("list_hex_accumulate")?;
    code.emit(&[0xc1, 0xe0, 0x04]); // shl    eax,0x4
    code.emit(&[0x09, 0xc8]); // or     eax,ecx
    code.emit(&[0x41, 0xff, 0xc9]); // dec    r9d
    code.emit_jcc_label(&[0x0f, 0x85], "list_hex_digit");
    code.emit(&[0xc3]); // ret

    // RDI scratch and signed RAX index -> normalized RAX, never a clamped index.
    code.label("list_index")?;
    code.emit(&[0x48, 0x8b, 0x0f]); // mov    rcx,QWORD PTR [rdi]
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x89], "list_index_nonnegative");
    code.emit(&[0x48, 0x01, 0xc8]); // add    rax,rcx
    code.label("list_index_nonnegative")?;
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_index");
    code.emit(&[0x48, 0x39, 0xc8]); // cmp    rax,rcx
    code.emit_jcc_label(&[0x0f, 0x83], "error_index");
    code.emit(&[0xc3]); // ret
    code.label("list_get_int")?;
    code.emit(&[0x48, 0xc1, 0xe0, 0x04]); // shl    rax,0x4
    code.emit(&[0x48, 0x8b, 0x84, 0x07, 0x00, 0x10, 0x00, 0x00]); // mov    rax,QWORD PTR [rdi+rax*1+0x1000]
    code.emit(&[0xc3]); // ret

    // A fetched text item outlives scratch. Charge the existing text budget and
    // copy even an empty item's storage before releasing the parsed mapping.
    code.label("list_get_text")?;
    code.emit(&[0x48, 0xc1, 0xe0, 0x04]); // shl    rax,0x4
    code.emit(&[0x48, 0x8b, 0xb4, 0x07, 0x00, 0x10, 0x00, 0x00]); // mov    rsi,QWORD PTR [rdi+rax*1+0x1000]
    code.emit(&[0x48, 0x8b, 0x94, 0x07, 0x08, 0x10, 0x00, 0x00]); // mov    rdx,QWORD PTR [rdi+rax*1+0x1008]
    code.emit(&[0x56]); // push   rsi
    code.emit_call_label("text_allocate");
    code.emit(&[0x5e]); // pop    rsi
    code.emit(&[0x48, 0x89, 0xc7]); // mov    rdi,rax
    code.emit(&[0x48, 0x89, 0xd1]); // mov    rcx,rdx
    code.emit(&[0x50]); // push   rax
    code.emit(&[0xf3, 0xa4]); // rep movs BYTE PTR es:[rdi],BYTE PTR ds:[rsi]
    code.emit(&[0x58]); // pop    rax
    code.emit(&[0xc3]); // ret

    // Preserve both integer and pointer/length result registers while unmapping.
    code.label("list_release")?;
    code.emit(&[0x50]); // push   rax
    code.emit(&[0x52]); // push   rdx
    code.emit(&[0xb8, 0x0b, 0x00, 0x00, 0x00]); // mov    eax,0xb
    code.emit(&[0xbe, 0x00, 0x10, 0x30, 0x00]); // mov    esi,0x301000
    code.emit_call_label("recovery_temp_unmap");
    code.emit(&[0x5a]); // pop    rdx
    code.emit(&[0x58]); // pop    rax
    code.emit(&[0xc3]); // ret
    code.label("list_empty")?;
    code.emit(&[0xba, 0x02, 0x00, 0x00, 0x00]); // mov    edx,0x2
    code.emit_call_label("list_allocate");
    code.emit(&[0x66, 0xc7, 0x00, 0x5b, 0x5d]); // mov    WORD PTR [rax],0x5d5b
    code.emit(&[0xc3]); // ret

    // RDI scratch, RSI index (count means append), RDX/RCX replacement value.
    // Operands are fully evaluated before entry. Change only scratch, encode the
    // final descriptors, then allocate and copy one immutable canonical result.
    code.label("list_build")?;
    code.emit(&[0x4c, 0x8b, 0x07]); // mov    r8,QWORD PTR [rdi]
    code.emit(&[0x4c, 0x39, 0xc6]); // cmp    rsi,r8
    code.emit_jcc_label(&[0x0f, 0x85], "list_build_replace");
    code.emit(&[0x49, 0xff, 0xc0]); // inc    r8
    code.emit(&[0x49, 0x81, 0xf8, 0x00, 0x00, 0x01, 0x00]); // cmp    r8,0x10000
    code.emit_jcc_label(&[0x0f, 0x87], "error_list_limit");
    code.emit(&[0x4c, 0x89, 0x07]); // mov    QWORD PTR [rdi],r8
    code.label("list_build_replace")?;
    code.emit(&[0x48, 0xc1, 0xe6, 0x04]); // shl    rsi,0x4
    code.emit(&[0x48, 0x89, 0x94, 0x37, 0x00, 0x10, 0x00, 0x00]); // mov    QWORD PTR [rdi+rsi*1+0x1000],rdx
    code.emit(&[0x48, 0x89, 0x8c, 0x37, 0x08, 0x10, 0x00, 0x00]); // mov    QWORD PTR [rdi+rsi*1+0x1008],rcx
    code.emit_call_label("list_encode");
    code.emit(&[0x48, 0x89, 0xc6]); // mov    rsi,rax
    code.emit(&[0x56]); // push   rsi
    code.emit_call_label("list_allocate");
    code.emit(&[0x5e]); // pop    rsi
    code.emit(&[0x48, 0x89, 0xc7]); // mov    rdi,rax
    code.emit(&[0x48, 0x89, 0xd1]); // mov    rcx,rdx
    code.emit(&[0x50]); // push   rax
    code.emit(&[0xf3, 0xa4]); // rep movs BYTE PTR es:[rdi],BYTE PTR ds:[rsi]
    code.emit(&[0x58]); // pop    rax
    code.emit(&[0xc3]); // ret

    // RDI parsed scratch -> RAX canonical bytes and RDX length inside scratch.
    // Encoding reserves no immutable storage; list and JSON callers select the
    // destination arena only after the complete result passes its size bound.
    code.label("list_encode")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit(&[0x49, 0x89, 0xfe]); // mov    r14,rdi
    code.emit(&[0x4d, 0x8b, 0x26]); // mov    r12,QWORD PTR [r14]
    code.emit(&[0x4d, 0x8d, 0xae, 0x00, 0x10, 0x00, 0x00]); // lea    r13,[r14+0x1000]
    code.emit(&[0x4d, 0x8d, 0xbe, 0x00, 0x10, 0x20, 0x00]); // lea    r15,[r14+0x201000]
    code.emit(&[0xb0, 0x5b]); // mov    al,0x5b
    code.emit_call_label("list_put_byte");
    code.emit(&[0x31, 0xdb]); // xor    ebx,ebx
    code.label("list_build_loop")?;
    code.emit(&[0x4c, 0x39, 0xe3]); // cmp    rbx,r12
    code.emit_jcc_label(&[0x0f, 0x83], "list_build_done");
    code.emit(&[0x48, 0x85, 0xdb]); // test   rbx,rbx
    code.emit_jcc_label(&[0x0f, 0x84], "list_build_element");
    code.emit(&[0xb0, 0x2c]); // mov    al,0x2c
    code.emit_call_label("list_put_byte");
    code.label("list_build_element")?;
    if has_float {
        code.emit(&[0x49, 0x83, 0x7e, 0x08, 0x02]);
        code.emit_jcc_label(&[0x0f, 0x83], "list_build_float");
    }
    code.emit(&[0x49, 0x83, 0x7e, 0x08, 0x00]); // cmp    QWORD PTR [r14+0x8],0x0
    code.emit_jcc_label(&[0x0f, 0x85], "list_build_text");
    code.emit(&[0x49, 0x8b, 0x7d, 0x00]); // mov    rdi,QWORD PTR [r13+0x0]
    code.emit(&[0x49, 0x8d, 0xb6, 0x80, 0x00, 0x00, 0x00]); // lea    rsi,[r14+0x80]
    code.emit_call_label("scalar_render");
    if has_float {
        code.emit_jmp_label("list_build_number");
        code.label("list_build_float")?;
        code.emit(&[0x49, 0x8b, 0x7d, 0x00]); // bits
        code.emit(&[0x49, 0x8d, 0xb6, 0x80, 0x00, 0x00, 0x00]);
        code.emit_call_label("float_render");
        code.label("list_build_number")?;
    }
    code.emit(&[0x48, 0x89, 0xc6]); // mov    rsi,rax
    code.label("list_build_int_copy")?;
    code.emit(&[0x48, 0x85, 0xd2]); // test   rdx,rdx
    code.emit_jcc_label(&[0x0f, 0x84], "list_build_next");
    code.emit(&[0x8a, 0x06]); // mov    al,BYTE PTR [rsi]
    code.emit_call_label("list_put_byte");
    code.emit(&[0x48, 0xff, 0xc6]); // inc    rsi
    code.emit(&[0x48, 0xff, 0xca]); // dec    rdx
    code.emit_jmp_label("list_build_int_copy");
    code.label("list_build_text")?;
    code.emit(&[0x49, 0x8b, 0x75, 0x00]); // mov    rsi,QWORD PTR [r13+0x0]
    code.emit(&[0x49, 0x8b, 0x55, 0x08]); // mov    rdx,QWORD PTR [r13+0x8]
    code.emit_call_label("list_quote_string");
    code.label("list_build_next")?;
    code.emit(&[0x48, 0xff, 0xc3]); // inc    rbx
    code.emit(&[0x49, 0x83, 0xc5, 0x10]); // add    r13,0x10
    code.emit_jmp_label("list_build_loop");
    code.label("list_build_done")?;
    code.emit(&[0xb0, 0x5d]); // mov    al,0x5d
    code.emit_call_label("list_put_byte");
    code.emit(&[0x49, 0x8d, 0x86, 0x00, 0x10, 0x20, 0x00]); // lea    rax,[r14+0x201000]
    code.emit(&[0x4c, 0x89, 0xfa]); // mov    rdx,r15
    code.emit(&[0x48, 0x29, 0xc2]); // sub    rdx,rax
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret

    // Quote validated RSI/RDX text at R15 within R14's output buffer. Returning
    // with R15 advanced lets scalar JSON use every byte without array wrappers.
    code.label("list_quote_string")?;
    code.emit(&[0xb0, 0x22]); // mov    al,0x22
    code.emit_call_label("list_put_byte");
    code.label("list_quote_loop")?;
    code.emit(&[0x48, 0x85, 0xd2]); // test   rdx,rdx
    code.emit_jcc_label(&[0x0f, 0x84], "list_quote_done");
    code.emit(&[0x0f, 0xb6, 0x06]); // movzx  eax,BYTE PTR [rsi]
    code.emit(&[0x48, 0xff, 0xc6]); // inc    rsi
    code.emit(&[0x48, 0xff, 0xca]); // dec    rdx
    code.emit(&[0x41, 0x89, 0xc0]); // mov    r8d,eax
    code.emit(&[0x3c, 0x22]); // cmp    al,0x22
    code.emit_jcc_label(&[0x0f, 0x84], "list_quote_short");
    code.emit(&[0x3c, 0x5c]); // cmp    al,0x5c
    code.emit_jcc_label(&[0x0f, 0x84], "list_quote_short");
    code.emit(&[0x3c, 0x0a]); // cmp    al,0xa
    code.emit_jcc_label(&[0x0f, 0x85], "list_quote_r");
    code.emit(&[0x41, 0xb0, 0x6e]); // mov    r8b,0x6e
    code.emit_jmp_label("list_quote_short");
    code.label("list_quote_r")?;
    code.emit(&[0x3c, 0x0d]); // cmp    al,0xd
    code.emit_jcc_label(&[0x0f, 0x85], "list_quote_t");
    code.emit(&[0x41, 0xb0, 0x72]); // mov    r8b,0x72
    code.emit_jmp_label("list_quote_short");
    code.label("list_quote_t")?;
    code.emit(&[0x3c, 0x09]); // cmp    al,0x9
    code.emit_jcc_label(&[0x0f, 0x85], "list_quote_control");
    code.emit(&[0x41, 0xb0, 0x74]); // mov    r8b,0x74
    code.emit_jmp_label("list_quote_short");
    code.label("list_quote_control")?;
    code.emit(&[0x3c, 0x20]); // cmp    al,0x20
    code.emit_jcc_label(&[0x0f, 0x82], "list_quote_unicode");
    code.emit(&[0x3c, 0x7f]); // cmp    al,0x7f
    code.emit_jcc_label(&[0x0f, 0x84], "list_quote_unicode");
    code.emit(&[0x3c, 0xc2]); // cmp    al,0xc2
    code.emit_jcc_label(&[0x0f, 0x85], "list_quote_raw");
    code.emit(&[0x80, 0x3e, 0xa0]); // cmp    BYTE PTR [rsi],0xa0
    code.emit_jcc_label(&[0x0f, 0x83], "list_quote_raw");
    code.emit(&[0x44, 0x0f, 0xb6, 0x06]); // movzx  r8d,BYTE PTR [rsi]
    code.emit(&[0x48, 0xff, 0xc6]); // inc    rsi
    code.emit(&[0x48, 0xff, 0xca]); // dec    rdx
    code.emit_jmp_label("list_quote_unicode");
    code.label("list_quote_short")?;
    code.emit(&[0xb0, 0x5c]); // mov    al,0x5c
    code.emit_call_label("list_put_byte");
    code.emit(&[0x44, 0x89, 0xc0]); // mov    eax,r8d
    code.emit_jmp_label("list_quote_raw");
    code.label("list_quote_unicode")?;
    code.emit(&[0xb0, 0x5c]); // mov    al,0x5c
    code.emit_call_label("list_put_byte");
    code.emit(&[0xb0, 0x75]); // mov    al,0x75
    code.emit_call_label("list_put_byte");
    code.emit(&[0xb0, 0x30]); // mov    al,0x30
    code.emit_call_label("list_put_byte");
    code.emit_call_label("list_put_byte");
    code.emit(&[0x44, 0x89, 0xc0]); // mov    eax,r8d
    code.emit(&[0xc1, 0xe8, 0x04]); // shr    eax,0x4
    code.emit_call_label("list_quote_hex");
    code.emit(&[0x44, 0x89, 0xc0]); // mov    eax,r8d
    code.emit(&[0x83, 0xe0, 0x0f]); // and    eax,0xf
    code.emit_call_label("list_quote_hex");
    code.emit_jmp_label("list_quote_loop");
    code.label("list_quote_hex")?;
    code.emit(&[0x3c, 0x09]); // cmp    al,0x9
    code.emit_jcc_label(&[0x0f, 0x86], "list_quote_hex_decimal");
    code.emit(&[0x04, 0x57]); // add    al,0x57
    code.emit_jmp_label("list_put_byte");
    code.label("list_quote_hex_decimal")?;
    code.emit(&[0x04, 0x30]); // add    al,0x30
    code.emit_jmp_label("list_put_byte");
    code.label("list_quote_raw")?;
    code.emit_call_label("list_put_byte");
    code.emit_jmp_label("list_quote_loop");
    code.label("list_quote_done")?;
    code.emit(&[0xb0, 0x22]); // mov    al,0x22
    code.emit_call_label("list_put_byte");
    code.emit(&[0xc3]); // ret
    code.label("list_put_byte")?;
    code.emit(&[0x4d, 0x8d, 0x96, 0x00, 0x10, 0x30, 0x00]); // lea    r10,[r14+0x301000]
    code.emit(&[0x4d, 0x39, 0xd7]); // cmp    r15,r10
    code.emit_jcc_label(&[0x0f, 0x83], "error_list_limit");
    code.emit(&[0x41, 0x88, 0x07]); // mov    BYTE PTR [r15],al
    code.emit(&[0x49, 0xff, 0xc7]); // inc    r15
    code.emit(&[0xc3]); // ret

    // The root's list arena and byte charge are separate from its text arena.
    // RDX encoded size -> RAX immutable pointer; successful results cost >= 2.
    code.label("list_allocate")?;
    code.emit(&[0x48, 0x81, 0xfa, 0x00, 0x00, 0x10, 0x00]); // cmp    rdx,0x100000
    code.emit_jcc_label(&[0x0f, 0x87], "error_list_limit");
    code.emit(&[0x52]); // push   rdx
    code.emit(&[0x48, 0x8b, 0x45, 0x18]); // mov    rax,QWORD PTR [rbp+0x18]
    code.emit(&[0x48, 0x01, 0xd0]); // add    rax,rdx
    code.emit(&[0x48, 0x3d, 0x00, 0x00, 0x00, 0x04]); // cmp    rax,0x4000000
    code.emit_jcc_label(&[0x0f, 0x87], "error_list_limit");
    code.emit(&[0x50]); // push   rax
    code.emit(&[0x48, 0x83, 0x7d, 0x10, 0x00]); // cmp    QWORD PTR [rbp+0x10],0x0
    code.emit_jcc_label(&[0x0f, 0x85], "list_allocate_ready");
    code.emit(&[0xb8, 0x09, 0x00, 0x00, 0x00]); // mov    eax,0x9
    code.emit(&[0x31, 0xff]); // xor    edi,edi
    code.emit(&[0xbe, 0x00, 0x00, 0x00, 0x04]); // mov    esi,0x4000000
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov    edx,0x3
    code.emit(&[0x41, 0xba, 0x22, 0x00, 0x00, 0x00]); // mov    r10d,0x22
    code.emit(&[0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]); // mov    r8,0xffffffffffffffff
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_list_allocation");
    code.emit(&[0x48, 0x89, 0x45, 0x10]); // mov    QWORD PTR [rbp+0x10],rax
    code.label("list_allocate_ready")?;
    code.emit(&[0x48, 0x8b, 0x45, 0x10]); // mov    rax,QWORD PTR [rbp+0x10]
    code.emit(&[0x48, 0x03, 0x45, 0x18]); // add    rax,QWORD PTR [rbp+0x18]
    code.emit(&[0x59]); // pop    rcx
    code.emit(&[0x48, 0x89, 0x4d, 0x18]); // mov    QWORD PTR [rbp+0x18],rcx
    code.emit(&[0x5a]); // pop    rdx
    code.emit(&[0xc3]); // ret

    // Strict UTF-8 transport validation uses list diagnostics, including raw
    // invalid bytes in elements that a particular GET would otherwise skip.
    code.label("list_validate_utf8")?;
    code.emit(&[0x48, 0x85, 0xff]); // test   rdi,rdi
    code.emit_jcc_label(&[0x0f, 0x84], "error_list");
    code.emit(&[0x48, 0x81, 0xfe, 0x00, 0x00, 0x10, 0x00]); // cmp    rsi,0x100000
    code.emit_jcc_label(&[0x0f, 0x87], "error_list_limit");
    code.emit(&[0x49, 0x89, 0xf8]); // mov    r8,rdi
    code.emit(&[0x4c, 0x8d, 0x0c, 0x37]); // lea    r9,[rdi+rsi*1]
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.label("list_validate_utf8_loop")?;
    code.emit(&[0x4d, 0x39, 0xc8]); // cmp    r8,r9
    code.emit_jcc_label(&[0x0f, 0x84], "list_validate_utf8_done");
    code.emit(&[0x41, 0x0f, 0xb6, 0x08]); // movzx  ecx,BYTE PTR [r8]
    code.emit(&[0x49, 0xff, 0xc0]); // inc    r8
    code.emit(&[0x48, 0xff, 0xc0]); // inc    rax
    code.emit(&[0x81, 0xf9, 0x80, 0x00, 0x00, 0x00]); // cmp    ecx,0x80
    code.emit_jcc_label(&[0x0f, 0x82], "list_validate_utf8_loop");
    code.emit(&[0x81, 0xf9, 0xc2, 0x00, 0x00, 0x00]); // cmp    ecx,0xc2
    code.emit_jcc_label(&[0x0f, 0x82], "error_list");
    code.emit(&[0x81, 0xf9, 0xe0, 0x00, 0x00, 0x00]); // cmp    ecx,0xe0
    code.emit_jcc_label(&[0x0f, 0x82], "list_validate_utf8_two");
    code.emit(&[0x81, 0xf9, 0xf0, 0x00, 0x00, 0x00]); // cmp    ecx,0xf0
    code.emit_jcc_label(&[0x0f, 0x82], "list_validate_utf8_three");
    code.emit(&[0x81, 0xf9, 0xf5, 0x00, 0x00, 0x00]); // cmp    ecx,0xf5
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.emit(&[0x4d, 0x39, 0xc8]); // cmp    r8,r9
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.emit(&[0x41, 0x0f, 0xb6, 0x10]); // movzx  edx,BYTE PTR [r8]
    code.emit(&[0x81, 0xf9, 0xf0, 0x00, 0x00, 0x00]); // cmp    ecx,0xf0
    code.emit_jcc_label(&[0x0f, 0x85], "list_validate_utf8_four_max");
    code.emit(&[0x81, 0xfa, 0x90, 0x00, 0x00, 0x00]); // cmp    edx,0x90
    code.emit_jcc_label(&[0x0f, 0x82], "error_list");
    code.label("list_validate_utf8_four_max")?;
    code.emit(&[0x81, 0xf9, 0xf4, 0x00, 0x00, 0x00]); // cmp    ecx,0xf4
    code.emit_jcc_label(&[0x0f, 0x85], "list_validate_utf8_four_ready");
    code.emit(&[0x81, 0xfa, 0x90, 0x00, 0x00, 0x00]); // cmp    edx,0x90
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.label("list_validate_utf8_four_ready")?;
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov    edx,0x3
    code.emit_jmp_label("list_validate_utf8_continuation");
    code.label("list_validate_utf8_three")?;
    code.emit(&[0x4d, 0x39, 0xc8]); // cmp    r8,r9
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.emit(&[0x41, 0x0f, 0xb6, 0x10]); // movzx  edx,BYTE PTR [r8]
    code.emit(&[0x81, 0xf9, 0xe0, 0x00, 0x00, 0x00]); // cmp    ecx,0xe0
    code.emit_jcc_label(&[0x0f, 0x85], "list_validate_utf8_three_max");
    code.emit(&[0x81, 0xfa, 0xa0, 0x00, 0x00, 0x00]); // cmp    edx,0xa0
    code.emit_jcc_label(&[0x0f, 0x82], "error_list");
    code.label("list_validate_utf8_three_max")?;
    code.emit(&[0x81, 0xf9, 0xed, 0x00, 0x00, 0x00]); // cmp    ecx,0xed
    code.emit_jcc_label(&[0x0f, 0x85], "list_validate_utf8_three_ready");
    code.emit(&[0x81, 0xfa, 0xa0, 0x00, 0x00, 0x00]); // cmp    edx,0xa0
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.label("list_validate_utf8_three_ready")?;
    code.emit(&[0xba, 0x02, 0x00, 0x00, 0x00]); // mov    edx,0x2
    code.emit_jmp_label("list_validate_utf8_continuation");
    code.label("list_validate_utf8_two")?;
    code.emit(&[0xba, 0x01, 0x00, 0x00, 0x00]); // mov    edx,0x1
    code.label("list_validate_utf8_continuation")?;
    code.emit(&[0x4d, 0x39, 0xc8]); // cmp    r8,r9
    code.emit_jcc_label(&[0x0f, 0x83], "error_list");
    code.emit(&[0x41, 0x0f, 0xb6, 0x08]); // movzx  ecx,BYTE PTR [r8]
    code.emit(&[0x81, 0xe1, 0xc0, 0x00, 0x00, 0x00]); // and    ecx,0xc0
    code.emit(&[0x81, 0xf9, 0x80, 0x00, 0x00, 0x00]); // cmp    ecx,0x80
    code.emit_jcc_label(&[0x0f, 0x85], "error_list");
    code.emit(&[0x49, 0xff, 0xc0]); // inc    r8
    code.emit(&[0xff, 0xca]); // dec    edx
    code.emit_jcc_label(&[0x0f, 0x85], "list_validate_utf8_continuation");
    code.emit_jmp_label("list_validate_utf8_loop");
    code.label("list_validate_utf8_done")?;
    code.emit(&[0xc3]); // ret
    Ok(())
}
