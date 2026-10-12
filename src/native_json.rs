//! Typed JSON conversion shares proven scalar/list/text machine-code generators.
//! Relocatable variants change diagnostics while preserving legacy helper behavior.
//! Only the final value reserves its applicable immutable arena; scratch is freed.
use super::*;

pub(super) fn value_type(instruction: &AilBytecodeInstruction) -> Result<&str, String> {
    let value = required(instruction, "type")?;
    if !matches!(
        value,
        "Int" | "Float" | "Bool" | "Text" | "List<Int>" | "List<Text>" | "List<Float>" | "Json"
    ) {
        return Err("AIL-JSON-001: unsupported native JSON value type".into());
    }
    Ok(value)
}

pub(super) fn fields(instruction: &AilBytecodeInstruction) -> Result<BTreeSet<String>, String> {
    if !matches!(
        instruction.opcode.as_str(),
        "JSON_PARSE" | "JSON_FORMAT" | "JSON_GET" | "JSON_COUNT" | "JSON_KIND" | "JSON_KEYS"
    ) {
        return Ok(BTreeSet::new());
    }
    if matches!(
        instruction.opcode.as_str(),
        "JSON_PARSE" | "JSON_FORMAT" | "JSON_GET"
    ) {
        value_type(instruction)?;
    }
    let mut fields = BTreeSet::new();
    for operand in ["key", "source"]
        .into_iter()
        .chain((instruction.opcode == "JSON_GET").then_some("path"))
    {
        let key = required(instruction, operand)?;
        if !matches!(IntExpr::parse(key)?, IntExpr::Field(parsed) if parsed == key) {
            return Err("AIL-JSON-001: native JSON operands must be field keys".into());
        }
        fields.insert(key.to_string());
    }
    Ok(fields)
}

pub(super) fn emit_operation(
    emitter: &mut Emitter,
    instruction: &AilBytecodeInstruction,
) -> Result<(), String> {
    if matches!(
        instruction.opcode.as_str(),
        "JSON_GET" | "JSON_COUNT" | "JSON_KIND" | "JSON_KEYS"
    ) || value_type(instruction)? == "Json"
    {
        return native_structured_json::emit_operation(emitter, instruction);
    }
    let kind = value_type(instruction)?;
    let parsing = instruction.opcode == "JSON_PARSE";
    let key = required(instruction, "key")?;
    let source = emitter.slot(required(instruction, "source")?);
    emitter.load_rdi(source + POINTER);
    emitter.load_rsi(source + LENGTH);
    emit_conversion(emitter, key, kind, parsing)?;
    emitter.operation_trace(&format!(
        "{} JSON {key}\n",
        if parsing { "parsed" } else { "formatted" }
    ));
    Ok(())
}

// Source pointer/length are already loaded in RDI/RSI. This emits only the
// conversion and final assignment, allowing JSON_GET to reuse typed semantics.
pub(super) fn emit_conversion(
    emitter: &mut Emitter,
    key: &str,
    kind: &str,
    parsing: bool,
) -> Result<(), String> {
    let mut pointer_result = true;
    match (parsing, kind) {
        (true, "Float") => {
            emitter.code.emit_call_label("json_float_parse");
            native_float::store(emitter, key);
            pointer_result = false;
        }
        (false, "Float") => emitter.code.emit_call_label("json_float_format"),
        (true, "Int") => {
            emitter.code.emit_call_label("json_parse_int");
            emitter.store_integer(key);
            pointer_result = false;
        }
        (true, "Bool") => {
            emitter.code.emit_call_label("json_parse_boolean");
            let false_label = format!("json_false_{}", emitter.next);
            let done = format!("json_bool_done_{}", emitter.next);
            emitter.next += 1;
            emitter.code.emit_test_rax_rax();
            emitter.code.emit_jcc_label(&[0x0f, 0x84], &false_label);
            let truth = emitter.data(b"true");
            let falsehood = emitter.data(b"false");
            emitter.code.emit_lea_rsi_label(&truth);
            emitter.store_rsi(emitter.slot(key) + POINTER);
            emitter.set(emitter.slot(key) + LENGTH, 4);
            emitter.code.emit_jmp_label(&done);
            emitter.code.label(false_label)?;
            emitter.code.emit_lea_rsi_label(&falsehood);
            emitter.store_rsi(emitter.slot(key) + POINTER);
            emitter.set(emitter.slot(key) + LENGTH, 5);
            emitter.code.label(done)?;
            emitter.set(emitter.slot(key) + NUMERIC, 0);
            pointer_result = false;
        }
        (true, "Text") => emitter.code.emit_call_label("json_parse_text"),
        (false, "Int") => {
            emitter.code.emit_call_label("json_text_validate");
            emitter.code.emit_call_label("json_scalar_parse");
            emitter.code.emit_call_label("json_format_integer");
        }
        (false, "Bool") => emitter.code.emit_call_label("json_format_boolean"),
        (false, "Text") => emitter.code.emit_call_label("json_format_text"),
        (_, "List<Int>" | "List<Text>" | "List<Float>") => {
            let mode = match kind {
                "List<Int>" => 0,
                "List<Text>" => 1,
                _ => {
                    if parsing {
                        3
                    } else {
                        4
                    }
                }
            };
            emitter.code.emit_mov_edx_imm32(mode);
            emitter.code.emit_call_label("json_list_parse");
            emitter.code.emit(&[0x50, 0x48, 0x89, 0xc7]); // save scratch; rdi=scratch
            emitter.code.emit_call_label("json_list_encode");
            emitter.code.emit(&[0x48, 0x89, 0xc6]); // rsi=encoded bytes, rdx=len
            emitter.code.emit_call_label(if parsing {
                "json_copy_list"
            } else {
                "json_copy_text"
            });
            emitter.code.emit(&[0x5f]); // scratch
            emitter.code.emit_call_label("json_list_release");
        }
        _ => return Err("AIL-JSON-001: unsupported native JSON conversion".into()),
    }
    if pointer_result {
        let destination = emitter.slot(key);
        emitter.store_rax(destination + POINTER);
        emitter.store_rdx(destination + LENGTH);
        emitter.set(destination + NUMERIC, 0);
    }
    Ok(())
}

// Copy generated helpers with their relocations intact. Internal labels are
// namespaced, and only external diagnostic/allocator targets change.
fn append_variant(
    code: &mut X64Code,
    source: &X64Code,
    first: &str,
    until: Option<&str>,
) -> Result<(), String> {
    let start = source.labels[first];
    let end = until.map_or(source.bytes.len(), |label| source.labels[label]);
    let base = code.bytes.len();
    let target_name = |name: &str| {
        if source
            .labels
            .get(name)
            .is_some_and(|offset| *offset >= start && *offset < end)
        {
            format!("json_{name}")
        } else {
            match name {
                "error_int" | "error_text" | "error_list" | "error_index" => "error_json".into(),
                "error_text_limit"
                | "error_text_allocation"
                | "error_list_limit"
                | "error_list_allocation" => "error_json_limit".into(),
                "text_allocate" => "json_text_allocate".into(),
                other => other.into(),
            }
        }
    };
    for (name, offset) in &source.labels {
        if *offset >= start && *offset < end {
            let name = target_name(name);
            if code
                .labels
                .insert(name.clone(), base + offset - start)
                .is_some()
            {
                return Err(format!("duplicate native JSON helper label {name}"));
            }
        }
    }
    for (offset, target) in &source.rel32_patches {
        if *offset >= start && *offset < end {
            code.rel32_patches
                .push((base + offset - start, target_name(target)));
        }
    }
    code.emit(&source.bytes[start..end]);
    Ok(())
}

pub(super) fn emit_helpers(code: &mut X64Code, has_float: bool) -> Result<(), String> {
    let mut text = X64Code::default();
    native_text::emit_helpers(&mut text)?;
    append_variant(code, &text, "text_validate", Some("text_join"))?;
    let mut list = X64Code::default();
    native_list::emit_helpers(&mut list, has_float)?;
    append_variant(code, &list, "list_parse", None)?;
    let mut scalar = X64Code::default();
    super::emit_helpers(&mut scalar)?;
    append_variant(code, &scalar, "scalar_parse", Some("scalar_render"))?;

    // Scalar parsing uses the list decoder's cursor ABI, then explicitly checks
    // the complete input. Int and Bool return values without arena allocations.
    code.label("json_parse_int")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit_call_label("json_text_validate");
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x4c, 0x8d, 0x2c, 0x37]); // lea    r13,[rdi+rsi*1]
    code.emit_call_label("json_list_skip_space");
    code.emit_call_label("json_list_parse_int");
    code.emit(&[0x48, 0x89, 0xc3]); // mov    rbx,rax
    code.emit_call_label("json_list_skip_space");
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x48, 0x89, 0xd8]); // mov    rax,rbx
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret
    code.label("json_parse_boolean")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit_call_label("json_text_validate");
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x4c, 0x8d, 0x2c, 0x37]); // lea    r13,[rdi+rsi*1]
    code.emit_call_label("json_list_skip_space");
    code.emit_call_label("json_boolean_token");
    code.emit(&[0x48, 0x89, 0xc3]); // mov    rbx,rax
    code.emit_call_label("json_list_skip_space");
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x48, 0x89, 0xd8]); // mov    rax,rbx
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret
    code.label("json_boolean_token")?;
    code.emit(&[0x4c, 0x89, 0xe9]); // mov    rcx,r13
    code.emit(&[0x4c, 0x29, 0xe1]); // sub    rcx,r12
    code.emit(&[0x48, 0x83, 0xf9, 0x04]); // cmp    rcx,0x4
    code.emit_jcc_label(&[0x0f, 0x82], "error_json");
    code.emit(&[0x41, 0x81, 0x3c, 0x24, 0x74, 0x72, 0x75, 0x65]); // cmp    DWORD PTR [r12],0x65757274
    code.emit_jcc_label(&[0x0f, 0x85], "json_boolean_false");
    code.emit(&[0x49, 0x83, 0xc4, 0x04]); // add    r12,0x4
    code.emit(&[0xb8, 0x01, 0x00, 0x00, 0x00]); // mov    eax,0x1
    code.emit(&[0xc3]); // ret
    code.label("json_boolean_false")?;
    code.emit(&[0x48, 0x83, 0xf9, 0x05]); // cmp    rcx,0x5
    code.emit_jcc_label(&[0x0f, 0x82], "error_json");
    code.emit(&[0x41, 0x81, 0x3c, 0x24, 0x66, 0x61, 0x6c, 0x73]); // cmp    DWORD PTR [r12],0x736c6166
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x41, 0x80, 0x7c, 0x24, 0x04, 0x65]); // cmp    BYTE PTR [r12+0x4],0x65
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x49, 0x83, 0xc4, 0x05]); // add    r12,0x5
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.emit(&[0xc3]); // ret
    code.label("json_format_boolean")?;
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit_call_label("json_text_validate");
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x4c, 0x8d, 0x2c, 0x37]); // lea    r13,[rdi+rsi*1]
    code.emit(&[0x57]); // push   rdi
    code.emit(&[0x56]); // push   rsi
    code.emit_call_label("json_boolean_token");
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x5a]); // pop    rdx
    code.emit(&[0x5e]); // pop    rsi
    code.emit_call_label("json_copy_text");
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0xc3]); // ret
    code.label("json_format_integer")?;
    code.emit(&[0x48, 0x83, 0xec, 0x20]); // sub    rsp,0x20
    code.emit(&[0x48, 0x89, 0xc7]); // mov    rdi,rax
    code.emit(&[0x48, 0x8d, 0x74, 0x24, 0x20]); // lea    rsi,[rsp+0x20]
    code.emit_call_label("scalar_render");
    code.emit(&[0x48, 0x89, 0xc6]); // mov    rsi,rax
    code.emit_call_label("json_copy_text");
    code.emit(&[0x48, 0x83, 0xc4, 0x20]); // add    rsp,0x20
    code.emit(&[0xc3]); // ret

    // Match the shared list scratch layout. This temporary mapping is outside
    // both deterministic budgets and is released after successful conversion.
    code.label("json_scratch_allocate")?;
    code.emit(&[0xb8, 0x09, 0x00, 0x00, 0x00]); // mov    eax,0x9
    code.emit(&[0x31, 0xff]); // xor    edi,edi
    code.emit(&[0xbe, 0x00, 0x10, 0x30, 0x00]); // mov    esi,0x301000
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov    edx,0x3
    code.emit(&[0x41, 0xba, 0x22, 0x00, 0x00, 0x00]); // mov    r10d,0x22
    code.emit(&[0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]); // mov    r8,0xffffffffffffffff
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.emit_call_label("recovery_temp_map");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_json_limit");
    code.emit(&[0xc3]); // ret

    // Decode into scratch and validate trailing input before reserving the final
    // decoded text bytes. RBP and the enclosing frame/fuel registers are retained.
    code.label("json_parse_text")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit_call_label("json_text_validate");
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x4c, 0x8d, 0x2c, 0x37]); // lea    r13,[rdi+rsi*1]
    code.emit_call_label("json_scratch_allocate");
    code.emit(&[0x49, 0x89, 0xc6]); // mov    r14,rax
    code.emit(&[0x4d, 0x8d, 0xbe, 0x00, 0x10, 0x10, 0x00]); // lea    r15,[r14+0x101000]
    code.emit_call_label("json_list_skip_space");
    code.emit_call_label("json_list_parse_string");
    code.emit(&[0x48, 0x89, 0xc3]); // mov    rbx,rax
    code.emit_call_label("json_list_skip_space");
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x48, 0x89, 0xde]); // mov    rsi,rbx
    code.emit(&[0x4c, 0x89, 0xfa]); // mov    rdx,r15
    code.emit(&[0x48, 0x29, 0xda]); // sub    rdx,rbx
    code.emit_call_label("json_copy_text");
    code.emit(&[0x4c, 0x89, 0xf7]); // mov    rdi,r14
    code.emit_call_label("json_list_release");
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret

    // Quote directly into the full 1 MiB output span, without list delimiters.
    code.label("json_format_text")?;
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit_call_label("json_text_validate");
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x49, 0x89, 0xf5]); // mov    r13,rsi
    code.emit_call_label("json_scratch_allocate");
    code.emit(&[0x49, 0x89, 0xc6]); // mov    r14,rax
    code.emit(&[0x4d, 0x8d, 0xbe, 0x00, 0x10, 0x20, 0x00]); // lea    r15,[r14+0x201000]
    code.emit(&[0x4c, 0x89, 0xe6]); // mov    rsi,r12
    code.emit(&[0x4c, 0x89, 0xea]); // mov    rdx,r13
    code.emit_call_label("json_list_quote_string");
    code.emit(&[0x49, 0x8d, 0xb6, 0x00, 0x10, 0x20, 0x00]); // lea    rsi,[r14+0x201000]
    code.emit(&[0x4c, 0x89, 0xfa]); // mov    rdx,r15
    code.emit(&[0x48, 0x29, 0xf2]); // sub    rdx,rsi
    code.emit_call_label("json_copy_text");
    code.emit(&[0x4c, 0x89, 0xf7]); // mov    rdi,r14
    code.emit_call_label("json_list_release");
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0xc3]); // ret

    // The sole final allocation, chosen by conversion mode and resulting type.
    code.label("json_copy_text")?;
    code.emit(&[0x56]); // push   rsi
    code.emit_call_label("json_text_allocate");
    code.emit_jmp_label("json_copy_finish");
    code.label("json_copy_list")?;
    code.emit(&[0x56]); // push   rsi
    code.emit_call_label("json_list_allocate");
    code.label("json_copy_finish")?;
    code.emit(&[0x5e]); // pop    rsi
    code.emit(&[0x48, 0x89, 0xc7]); // mov    rdi,rax
    code.emit(&[0x48, 0x89, 0xd1]); // mov    rcx,rdx
    code.emit(&[0x50]); // push   rax
    code.emit(&[0xf3, 0xa4]); // rep movs BYTE PTR es:[rdi],BYTE PTR ds:[rsi]
    code.emit(&[0x58]); // pop    rax
    code.emit(&[0xc3]); // ret
    Ok(())
}
