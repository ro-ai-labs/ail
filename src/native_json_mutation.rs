//! Immutable JSON mutations splice validated canonical spans, then reparse the
//! final document before charging its one list-arena allocation. Source, path,
//! target and replacement are validated in that order. Scratch mappings are
//! bounded and released; replacement formatting never charges a semantic arena.
use super::*;

pub(super) fn fields(instruction: &AilBytecodeInstruction) -> Result<BTreeSet<String>, String> {
    if !matches!(instruction.opcode.as_str(), "JSON_SET" | "JSON_REMOVE") {
        return Ok(BTreeSet::new());
    }
    if instruction.opcode == "JSON_SET" {
        native_json::value_type(instruction)?;
    }
    let mut fields = BTreeSet::new();
    for operand in ["key", "source", "path"]
        .into_iter()
        .chain((instruction.opcode == "JSON_SET").then_some("value"))
    {
        let key = required(instruction, operand)?;
        if !matches!(IntExpr::parse(key)?, IntExpr::Field(parsed) if parsed == key) {
            return Err("AIL-JSON-001: JSON mutation operands must be field keys".into());
        }
        fields.insert(key.to_string());
    }
    Ok(fields)
}

pub(super) fn emit_operation(
    emitter: &mut Emitter,
    instruction: &AilBytecodeInstruction,
) -> Result<(), String> {
    let setting = instruction.opcode == "JSON_SET";
    let source = emitter.slot(required(instruction, "source")?);
    let key = required(instruction, "key")?;
    emitter.load_rdi(source + POINTER);
    emitter.load_rsi(source + LENGTH);
    emitter.code.emit_call_label("structured_parse");
    emitter.code.emit(&[0x50, 0x48, 0x89, 0xc7]); // retain source scratch; RDI=source
    let path = emitter.slot(required(instruction, "path")?);
    emitter.load_rsi(path + POINTER);
    emitter.load_rdx(path + LENGTH);
    emitter.code.emit(&[0xb9, u8::from(setting), 0, 0, 0]);
    emitter.code.emit_call_label("mutation_target");
    if setting {
        let value = emitter.slot(required(instruction, "value")?);
        emitter.load_rdi(value + POINTER);
        emitter.load_rsi(value + LENGTH);
        let value_type = native_json::value_type(instruction)?;
        if value_type == "Float" {
            emitter.code.emit_call_label("float_mutation_format");
        } else if value_type == "List<Float>" {
            emitter.code.emit_mov_edx_imm32(4);
            emitter.code.emit_call_label("json_list_parse");
            emitter.code.emit(&[0x50, 0x48, 0x89, 0xc7]);
            emitter.code.emit_call_label("json_list_encode");
            // The mutation composer consumes encoded length at offset zero,
            // and bytes at +0x201000. The descriptors are no longer needed.
            emitter.code.emit(&[0x58, 0x48, 0x89, 0x10]);
        } else {
            let kind = match value_type {
                "Int" => 0,
                "Bool" => 1,
                "Text" => 2,
                "List<Int>" => 3,
                "List<Text>" => 4,
                "Json" => 5,
                _ => unreachable!(),
            };
            emitter.code.emit_mov_edx_imm32(kind);
            emitter.code.emit_call_label("mutation_format");
        }
    } else {
        emitter.code.emit(&[0x31, 0xc0]);
    }
    emitter
        .code
        .emit(&[0x50, 0x48, 0x89, 0xc6, 0x48, 0x8b, 0x7c, 0x24, 0x08]); // retain formatter; RSI=formatter,RDI=source
    emitter.code.emit_call_label("mutation_compose");
    emitter.code.emit(&[0x5f]);
    if setting {
        emitter.code.emit_call_label("json_list_release");
    }
    emitter.code.emit(&[0x5f]);
    emitter.code.emit_call_label("structured_release");
    emitter.code.emit(&[
        0x50, 0x48, 0x8d, 0xb8, 0x00, 0x10, 0x20, 0x00, 0x48, 0x8b, 0x30,
    ]); // retain composed text; RDI=bytes,RSI=size
    emitter.code.emit_call_label("structured_parse");
    emitter.code.emit(&[
        0x50, 0x48, 0x8b, 0xb0, 0x40, 0x10, 0x30, 0x00, 0x48, 0x8b, 0x90, 0x48, 0x10, 0x30, 0x00,
    ]);
    emitter.code.emit_call_label("json_copy_list");
    let destination = emitter.slot(key);
    emitter.store_rax(destination + POINTER);
    emitter.store_rdx(destination + LENGTH);
    emitter.set(destination + NUMERIC, 0);
    emitter.code.emit(&[0x5f]);
    emitter.code.emit_call_label("structured_release");
    emitter.code.emit(&[0x5f]);
    emitter.code.emit_call_label("json_list_release");
    emitter.operation_trace(&format!(
        "{} JSON {key}\n",
        if setting { "set" } else { "removed" }
    ));
    Ok(())
}

pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    // Source scratch metadata (past parser count/depth): parent+16, target+24,
    // previous sibling+32, decoded final-key length+40, set flag+48,
    // retained prefix length+56, suffix offset+64, append mode+72 (array1/object2).
    // Formatted replacement/composed scratch: encoded length+0, bytes+0x201000.
    code.label("mutation_target")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit(&[0x49, 0x89, 0xfe]); // mov    r14,rdi
    code.emit(&[0x49, 0x89, 0x4e, 0x30]); // mov    QWORD PTR [r14+0x30],rcx
    code.emit(&[0x48, 0x89, 0xf7]); // mov    rdi,rsi
    code.emit(&[0x48, 0x89, 0xd6]); // mov    rsi,rdx
    code.emit_call_label("json_text_validate");
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x4c, 0x8d, 0x2c, 0x37]); // lea    r13,[rdi+rsi*1]
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_path_nonempty");
    code.emit(&[0x49, 0x83, 0x7e, 0x30, 0x00]); // cmp    QWORD PTR [r14+0x30],0x0
    code.emit_jcc_label(&[0x0f, 0x84], "error_json_path");
    code.emit(&[0x49, 0x8b, 0x86, 0x48, 0x10, 0x30, 0x00]); // mov    rax,QWORD PTR [r14+0x301048]
    code.emit(&[0x49, 0x89, 0x46, 0x40]); // mov    QWORD PTR [r14+0x40],rax
    code.emit_jmp_label("mutation_target_done");
    code.label("mutation_path_nonempty")?;
    code.emit(&[0x41, 0x80, 0x3c, 0x24, 0x2f]); // cmp    BYTE PTR [r12],0x2f
    code.emit_jcc_label(&[0x0f, 0x85], "error_json_path");
    code.emit(&[0x4c, 0x89, 0xe1]); // mov    rcx,r12
    code.emit(&[0x4d, 0x89, 0xe7]); // mov    r15,r12
    code.label("mutation_validate_path")?;
    code.emit(&[0x4c, 0x39, 0xe9]); // cmp    rcx,r13
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_parent");
    code.emit(&[0x8a, 0x01]); // mov    al,BYTE PTR [rcx]
    code.emit(&[0x3c, 0x2f]); // cmp    al,0x2f
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_validate_escape");
    code.emit(&[0x49, 0x89, 0xcf]); // mov    r15,rcx
    code.label("mutation_validate_escape")?;
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit(&[0x3c, 0x7e]); // cmp    al,0x7e
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_validate_path");
    code.emit(&[0x4c, 0x39, 0xe9]); // cmp    rcx,r13
    code.emit_jcc_label(&[0x0f, 0x84], "error_json_path");
    code.emit(&[0x8a, 0x01]); // mov    al,BYTE PTR [rcx]
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit(&[0x2c, 0x30]); // sub    al,0x30
    code.emit(&[0x3c, 0x01]); // cmp    al,0x1
    code.emit_jcc_label(&[0x0f, 0x87], "error_json_path");
    code.emit_jmp_label("mutation_validate_path");
    code.label("mutation_parent")?;
    code.emit(&[0x4c, 0x89, 0xf7]); // mov    rdi,r14
    code.emit(&[0x4c, 0x89, 0xe6]); // mov    rsi,r12
    code.emit(&[0x4c, 0x89, 0xfa]); // mov    rdx,r15
    code.emit(&[0x4c, 0x29, 0xe2]); // sub    rdx,r12
    code.emit_call_label("structured_pointer");
    code.emit(&[0x48, 0x89, 0xc3]); // mov    rbx,rax
    code.emit(&[0x49, 0x89, 0x46, 0x10]); // mov    QWORD PTR [r14+0x10],rax
    code.emit(&[0x4d, 0x8d, 0x67, 0x01]); // lea    r12,[r15+0x1]
    code.emit(&[0x4d, 0x8d, 0xbe, 0x00, 0x10, 0x00, 0x00]); // lea    r15,[r14+0x1000]
    code.label("mutation_decode_key")?;
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_lookup");
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x3c, 0x7e]); // cmp    al,0x7e
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_decoded_byte");
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x3c, 0x30]); // cmp    al,0x30
    code.emit(&[0xb0, 0x7e]); // mov    al,0x7e
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_decoded_byte");
    code.emit(&[0xb0, 0x2f]); // mov    al,0x2f
    code.label("mutation_decoded_byte")?;
    code.emit(&[0x41, 0x88, 0x07]); // mov    BYTE PTR [r15],al
    code.emit(&[0x49, 0xff, 0xc7]); // inc    r15
    code.emit_jmp_label("mutation_decode_key");
    code.label("mutation_lookup")?;
    code.emit(&[0x49, 0x8d, 0xb6, 0x00, 0x10, 0x00, 0x00]); // lea    rsi,[r14+0x1000]
    code.emit(&[0x4c, 0x89, 0xfa]); // mov    rdx,r15
    code.emit(&[0x48, 0x29, 0xf2]); // sub    rdx,rsi
    code.emit(&[0x49, 0x89, 0x56, 0x28]); // mov    QWORD PTR [r14+0x28],rdx
    code.emit(&[0x48, 0x83, 0x3b, 0x04]); // cmp    QWORD PTR [rbx],0x4
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_array");
    code.emit(&[0x48, 0x83, 0x3b, 0x05]); // cmp    QWORD PTR [rbx],0x5
    code.emit_jcc_label(&[0x0f, 0x85], "error_json_path");
    code.emit(&[0x4c, 0x8b, 0x43, 0x18]); // mov    r8,QWORD PTR [rbx+0x18]
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.emit(&[0x45, 0x31, 0xd2]); // xor    r10d,r10d
    code.emit(&[0x45, 0x31, 0xdb]); // xor    r11d,r11d
    code.label("mutation_member")?;
    code.emit(&[0x4d, 0x85, 0xc0]); // test   r8,r8
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_member_done");
    code.emit(&[0x49, 0x39, 0x50, 0x38]); // cmp    QWORD PTR [r8+0x38],rdx
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_member_next");
    code.emit(&[0x49, 0x8b, 0x78, 0x30]); // mov    rdi,QWORD PTR [r8+0x30]
    code.emit(&[0x31, 0xc9]); // xor    ecx,ecx
    code.label("mutation_member_compare")?;
    code.emit(&[0x48, 0x39, 0xd1]); // cmp    rcx,rdx
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_member_match");
    code.emit(&[0x8a, 0x04, 0x0f]); // mov    al,BYTE PTR [rdi+rcx*1]
    code.emit(&[0x3a, 0x04, 0x0e]); // cmp    al,BYTE PTR [rsi+rcx*1]
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_member_next");
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit_jmp_label("mutation_member_compare");
    code.label("mutation_member_match")?;
    code.emit(&[0x4d, 0x85, 0xc9]); // test   r9,r9
    code.emit_jcc_label(&[0x0f, 0x85], "error_json_path");
    code.emit(&[0x4d, 0x89, 0xc1]); // mov    r9,r8
    code.emit(&[0x4d, 0x89, 0xd3]); // mov    r11,r10
    code.label("mutation_member_next")?;
    code.emit(&[0x4d, 0x89, 0xc2]); // mov    r10,r8
    code.emit(&[0x4d, 0x8b, 0x40, 0x20]); // mov    r8,QWORD PTR [r8+0x20]
    code.emit_jmp_label("mutation_member");
    code.label("mutation_member_done")?;
    code.emit(&[0x4d, 0x89, 0x4e, 0x18]); // mov    QWORD PTR [r14+0x18],r9
    code.emit(&[0x4d, 0x89, 0x5e, 0x20]); // mov    QWORD PTR [r14+0x20],r11
    code.emit_jmp_label("mutation_bounds");
    code.label("mutation_array")?;
    code.emit(&[0x48, 0x85, 0xd2]); // test   rdx,rdx
    code.emit_jcc_label(&[0x0f, 0x84], "error_json_path");
    code.emit(&[0x48, 0x83, 0xfa, 0x01]); // cmp    rdx,0x1
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_array_integer");
    code.emit(&[0x80, 0x3e, 0x2d]); // cmp    BYTE PTR [rsi],0x2d
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_array_integer");
    code.emit(&[0x49, 0x83, 0x7e, 0x30, 0x00]); // cmp    QWORD PTR [r14+0x30],0x0
    code.emit_jcc_label(&[0x0f, 0x84], "error_json_path");
    code.emit_jmp_label("mutation_bounds");
    code.label("mutation_array_integer")?;
    code.emit(&[0x80, 0x3e, 0x30]); // cmp    BYTE PTR [rsi],0x30
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_array_index_start");
    code.emit(&[0x48, 0x83, 0xfa, 0x01]); // cmp    rdx,0x1
    code.emit_jcc_label(&[0x0f, 0x85], "error_json_path");
    code.label("mutation_array_index_start")?;
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.emit(&[0x31, 0xc9]); // xor    ecx,ecx
    code.label("mutation_array_index")?;
    code.emit(&[0x0f, 0xb6, 0x3c, 0x0e]); // movzx  edi,BYTE PTR [rsi+rcx*1]
    code.emit(&[0x83, 0xef, 0x30]); // sub    edi,0x30
    code.emit(&[0x83, 0xff, 0x09]); // cmp    edi,0x9
    code.emit_jcc_label(&[0x0f, 0x87], "error_json_path");
    code.emit(&[0x48, 0x6b, 0xc0, 0x0a]); // imul   rax,rax,0xa
    code.emit_jcc_label(&[0x0f, 0x80], "error_json_path");
    code.emit(&[0x48, 0x01, 0xf8]); // add    rax,rdi
    code.emit_jcc_label(&[0x0f, 0x80], "error_json_path");
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit(&[0x48, 0x39, 0xd1]); // cmp    rcx,rdx
    code.emit_jcc_label(&[0x0f, 0x82], "mutation_array_index");
    code.emit(&[0x48, 0x3b, 0x43, 0x28]); // cmp    rax,QWORD PTR [rbx+0x28]
    code.emit_jcc_label(&[0x0f, 0x83], "error_json_path");
    code.emit(&[0x4c, 0x8b, 0x4b, 0x18]); // mov    r9,QWORD PTR [rbx+0x18]
    code.emit(&[0x45, 0x31, 0xd2]); // xor    r10d,r10d
    code.label("mutation_array_walk")?;
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_array_found");
    code.emit(&[0x4d, 0x89, 0xca]); // mov    r10,r9
    code.emit(&[0x4d, 0x8b, 0x49, 0x20]); // mov    r9,QWORD PTR [r9+0x20]
    code.emit(&[0x48, 0xff, 0xc8]); // dec    rax
    code.emit_jmp_label("mutation_array_walk");
    code.label("mutation_array_found")?;
    code.emit(&[0x4d, 0x89, 0x4e, 0x18]); // mov    QWORD PTR [r14+0x18],r9
    code.emit(&[0x4d, 0x89, 0x56, 0x20]); // mov    QWORD PTR [r14+0x20],r10
    code.label("mutation_bounds")?;
    code.emit(&[0x4d, 0x8b, 0x46, 0x18]); // mov    r8,QWORD PTR [r14+0x18]
    code.emit(&[0x4d, 0x85, 0xc0]); // test   r8,r8
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_existing");
    code.emit(&[0x49, 0x83, 0x7e, 0x30, 0x00]); // cmp    QWORD PTR [r14+0x30],0x0
    code.emit_jcc_label(&[0x0f, 0x84], "error_json_path");
    code.emit(&[0x48, 0x8b, 0x43, 0x40]); // mov    rax,QWORD PTR [rbx+0x40]
    code.emit(&[0x48, 0x03, 0x43, 0x48]); // add    rax,QWORD PTR [rbx+0x48]
    code.emit(&[0x48, 0xff, 0xc8]); // dec    rax
    code.emit(&[0x49, 0x2b, 0x86, 0x40, 0x10, 0x30, 0x00]); // sub    rax,QWORD PTR [r14+0x301040]
    code.emit(&[0x49, 0x89, 0x46, 0x38]); // mov    QWORD PTR [r14+0x38],rax
    code.emit(&[0x49, 0x89, 0x46, 0x40]); // mov    QWORD PTR [r14+0x40],rax
    code.emit(&[0x48, 0x8b, 0x03]); // mov    rax,QWORD PTR [rbx]
    code.emit(&[0x48, 0x83, 0xe8, 0x03]); // sub    rax,0x3
    code.emit(&[0x49, 0x89, 0x46, 0x48]); // mov    QWORD PTR [r14+0x48],rax
    code.emit_jmp_label("mutation_target_done");
    code.label("mutation_existing")?;
    code.emit(&[0x49, 0x8b, 0x40, 0x40]); // mov    rax,QWORD PTR [r8+0x40]
    code.emit(&[0x48, 0x89, 0xc2]); // mov    rdx,rax
    code.emit(&[0x49, 0x03, 0x50, 0x48]); // add    rdx,QWORD PTR [r8+0x48]
    code.emit(&[0x49, 0x83, 0x7e, 0x30, 0x00]); // cmp    QWORD PTR [r14+0x30],0x0
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_relative_bounds");
    code.emit(&[0x49, 0x8b, 0x4e, 0x20]); // mov    rcx,QWORD PTR [r14+0x20]
    code.emit(&[0x48, 0x85, 0xc9]); // test   rcx,rcx
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_remove_first");
    code.emit(&[0x48, 0x8b, 0x41, 0x40]); // mov    rax,QWORD PTR [rcx+0x40]
    code.emit(&[0x48, 0x03, 0x41, 0x48]); // add    rax,QWORD PTR [rcx+0x48]
    code.emit_jmp_label("mutation_relative_bounds");
    code.label("mutation_remove_first")?;
    code.emit(&[0x48, 0x8b, 0x43, 0x40]); // mov    rax,QWORD PTR [rbx+0x40]
    code.emit(&[0x48, 0xff, 0xc0]); // inc    rax
    code.emit(&[0x49, 0x83, 0x78, 0x20, 0x00]); // cmp    QWORD PTR [r8+0x20],0x0
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_relative_bounds");
    code.emit(&[0x48, 0xff, 0xc2]); // inc    rdx
    code.label("mutation_relative_bounds")?;
    code.emit(&[0x49, 0x2b, 0x86, 0x40, 0x10, 0x30, 0x00]); // sub    rax,QWORD PTR [r14+0x301040]
    code.emit(&[0x49, 0x2b, 0x96, 0x40, 0x10, 0x30, 0x00]); // sub    rdx,QWORD PTR [r14+0x301040]
    code.emit(&[0x49, 0x89, 0x46, 0x38]); // mov    QWORD PTR [r14+0x38],rax
    code.emit(&[0x49, 0x89, 0x56, 0x40]); // mov    QWORD PTR [r14+0x40],rdx
    code.label("mutation_target_done")?;
    code.emit(&[0x4c, 0x89, 0xf0]); // mov    rax,r14
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret
    code.label("mutation_format")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit(&[0x41, 0x89, 0xd4]); // mov    r12d,edx
    code.emit_call_label("json_text_validate");
    code.emit(&[0x49, 0x89, 0xfd]); // mov    r13,rdi
    code.emit(&[0x49, 0x89, 0xf7]); // mov    r15,rsi
    code.emit_call_label("json_scratch_allocate");
    code.emit(&[0x49, 0x89, 0xc6]); // mov    r14,rax
    code.emit(&[0x4c, 0x89, 0xef]); // mov    rdi,r13
    code.emit(&[0x4c, 0x89, 0xfe]); // mov    rsi,r15
    code.emit(&[0x41, 0x83, 0xfc, 0x05]); // cmp    r12d,0x5
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_format_json");
    code.emit(&[0x41, 0x83, 0xfc, 0x03]); // cmp    r12d,0x3
    code.emit_jcc_label(&[0x0f, 0x83], "mutation_format_list");
    code.emit(&[0x41, 0x83, 0xfc, 0x02]); // cmp    r12d,0x2
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_format_text");
    code.emit(&[0x41, 0x83, 0xfc, 0x01]); // cmp    r12d,0x1
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_format_bool");
    code.emit_call_label("json_scalar_parse");
    code.emit(&[0x48, 0x89, 0xc7]); // mov    rdi,rax
    code.emit(&[0x49, 0x8d, 0xb6, 0x20, 0x10, 0x20, 0x00]); // lea    rsi,[r14+0x201020]
    code.emit_call_label("scalar_render");
    code.emit(&[0x48, 0x89, 0xc6]); // mov    rsi,rax
    code.emit_call_label("mutation_format_copy");
    code.emit_jmp_label("mutation_format_done");
    code.label("mutation_format_bool")?;
    code.emit(&[0x48, 0x83, 0xfe, 0x04]); // cmp    rsi,0x4
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_format_true");
    code.emit(&[0x48, 0x83, 0xfe, 0x05]); // cmp    rsi,0x5
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x81, 0x3f, 0x66, 0x61, 0x6c, 0x73]); // cmp    DWORD PTR [rdi],0x736c6166
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x80, 0x7f, 0x04, 0x65]); // cmp    BYTE PTR [rdi+0x4],0x65
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit_jmp_label("mutation_format_bool_copy");
    code.label("mutation_format_true")?;
    code.emit(&[0x81, 0x3f, 0x74, 0x72, 0x75, 0x65]); // cmp    DWORD PTR [rdi],0x65757274
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.label("mutation_format_bool_copy")?;
    code.emit(&[0x48, 0x89, 0xf2]); // mov    rdx,rsi
    code.emit(&[0x48, 0x89, 0xfe]); // mov    rsi,rdi
    code.emit_call_label("mutation_format_copy");
    code.emit_jmp_label("mutation_format_done");
    code.label("mutation_format_text")?;
    code.emit(&[0x48, 0x89, 0xf2]); // mov    rdx,rsi
    code.emit(&[0x48, 0x89, 0xfe]); // mov    rsi,rdi
    code.emit(&[0x4d, 0x8d, 0xbe, 0x00, 0x10, 0x20, 0x00]); // lea    r15,[r14+0x201000]
    code.emit_call_label("json_list_quote_string");
    code.emit(&[0x4c, 0x89, 0xfa]); // mov    rdx,r15
    code.emit(&[0x49, 0x8d, 0x86, 0x00, 0x10, 0x20, 0x00]); // lea    rax,[r14+0x201000]
    code.emit(&[0x48, 0x29, 0xc2]); // sub    rdx,rax
    code.emit(&[0x49, 0x89, 0x16]); // mov    QWORD PTR [r14],rdx
    code.emit_jmp_label("mutation_format_done");
    code.label("mutation_format_list")?;
    code.emit(&[0x44, 0x89, 0xe2]); // mov    edx,r12d
    code.emit(&[0x83, 0xea, 0x03]); // sub    edx,0x3
    code.emit_call_label("json_list_parse");
    code.emit(&[0x50]); // push   rax
    code.emit(&[0x48, 0x89, 0xc7]); // mov    rdi,rax
    code.emit_call_label("json_list_encode");
    code.emit(&[0x48, 0x89, 0xc6]); // mov    rsi,rax
    code.emit_call_label("mutation_format_copy");
    code.emit(&[0x5f]); // pop    rdi
    code.emit_call_label("json_list_release");
    code.emit_jmp_label("mutation_format_done");
    code.label("mutation_format_json")?;
    code.emit_call_label("structured_parse");
    code.emit(&[0x50]); // push   rax
    code.emit(&[0x48, 0x8b, 0xb0, 0x40, 0x10, 0x30, 0x00]); // mov    rsi,QWORD PTR [rax+0x301040]
    code.emit(&[0x48, 0x8b, 0x90, 0x48, 0x10, 0x30, 0x00]); // mov    rdx,QWORD PTR [rax+0x301048]
    code.emit_call_label("mutation_format_copy");
    code.emit(&[0x5f]); // pop    rdi
    code.emit_call_label("structured_release");
    code.label("mutation_format_done")?;
    code.emit(&[0x4c, 0x89, 0xf0]); // mov    rax,r14
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret
    code.label("mutation_format_copy")?;
    code.emit(&[0x49, 0x89, 0x16]); // mov    QWORD PTR [r14],rdx
    code.emit(&[0x49, 0x8d, 0xbe, 0x00, 0x10, 0x20, 0x00]); // lea    rdi,[r14+0x201000]
    code.emit(&[0x48, 0x89, 0xd1]); // mov    rcx,rdx
    code.emit(&[0xf3, 0xa4]); // rep movs BYTE PTR es:[rdi],BYTE PTR ds:[rsi]
    code.emit(&[0xc3]); // ret
    code.label("mutation_compose")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit(&[0x48, 0x89, 0xfb]); // mov    rbx,rdi
    code.emit(&[0x49, 0x89, 0xf4]); // mov    r12,rsi
    code.emit_call_label("json_scratch_allocate");
    code.emit(&[0x49, 0x89, 0xc6]); // mov    r14,rax
    code.emit(&[0x4d, 0x8d, 0xbe, 0x00, 0x10, 0x20, 0x00]); // lea    r15,[r14+0x201000]
    code.emit(&[0x48, 0x8b, 0xb3, 0x40, 0x10, 0x30, 0x00]); // mov    rsi,QWORD PTR [rbx+0x301040]
    code.emit(&[0x48, 0x8b, 0x53, 0x38]); // mov    rdx,QWORD PTR [rbx+0x38]
    code.emit_call_label("mutation_put_span");
    code.emit(&[0x48, 0x83, 0x7b, 0x30, 0x00]); // cmp    QWORD PTR [rbx+0x30],0x0
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_suffix");
    code.emit(&[0x48, 0x83, 0x7b, 0x48, 0x00]); // cmp    QWORD PTR [rbx+0x48],0x0
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_replacement");
    code.emit(&[0x48, 0x8b, 0x43, 0x10]); // mov    rax,QWORD PTR [rbx+0x10]
    code.emit(&[0x48, 0x83, 0x78, 0x28, 0x00]); // cmp    QWORD PTR [rax+0x28],0x0
    code.emit_jcc_label(&[0x0f, 0x84], "mutation_new_key");
    code.emit(&[0xb0, 0x2c]); // mov    al,0x2c
    code.emit_call_label("json_list_put_byte");
    code.label("mutation_new_key")?;
    code.emit(&[0x48, 0x83, 0x7b, 0x48, 0x02]); // cmp    QWORD PTR [rbx+0x48],0x2
    code.emit_jcc_label(&[0x0f, 0x85], "mutation_replacement");
    code.emit(&[0x48, 0x8d, 0xb3, 0x00, 0x10, 0x00, 0x00]); // lea    rsi,[rbx+0x1000]
    code.emit(&[0x48, 0x8b, 0x53, 0x28]); // mov    rdx,QWORD PTR [rbx+0x28]
    code.emit_call_label("json_list_quote_string");
    code.emit(&[0xb0, 0x3a]); // mov    al,0x3a
    code.emit_call_label("json_list_put_byte");
    code.label("mutation_replacement")?;
    code.emit(&[0x49, 0x8d, 0xb4, 0x24, 0x00, 0x10, 0x20, 0x00]); // lea    rsi,[r12+0x201000]
    code.emit(&[0x49, 0x8b, 0x14, 0x24]); // mov    rdx,QWORD PTR [r12]
    code.emit_call_label("mutation_put_span");
    code.label("mutation_suffix")?;
    code.emit(&[0x48, 0x8b, 0xb3, 0x40, 0x10, 0x30, 0x00]); // mov    rsi,QWORD PTR [rbx+0x301040]
    code.emit(&[0x48, 0x03, 0x73, 0x40]); // add    rsi,QWORD PTR [rbx+0x40]
    code.emit(&[0x48, 0x8b, 0x93, 0x48, 0x10, 0x30, 0x00]); // mov    rdx,QWORD PTR [rbx+0x301048]
    code.emit(&[0x48, 0x2b, 0x53, 0x40]); // sub    rdx,QWORD PTR [rbx+0x40]
    code.emit_call_label("mutation_put_span");
    code.emit(&[0x4c, 0x89, 0xfa]); // mov    rdx,r15
    code.emit(&[0x49, 0x8d, 0x86, 0x00, 0x10, 0x20, 0x00]); // lea    rax,[r14+0x201000]
    code.emit(&[0x48, 0x29, 0xc2]); // sub    rdx,rax
    code.emit(&[0x49, 0x89, 0x16]); // mov    QWORD PTR [r14],rdx
    code.emit(&[0x4c, 0x89, 0xf0]); // mov    rax,r14
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret
    code.label("mutation_put_span")?;
    code.emit(&[0x4d, 0x8d, 0x14, 0x17]); // lea    r10,[r15+rdx*1]
    code.emit(&[0x4d, 0x8d, 0x9e, 0x00, 0x10, 0x30, 0x00]); // lea    r11,[r14+0x301000]
    code.emit(&[0x4d, 0x39, 0xda]); // cmp    r10,r11
    code.emit_jcc_label(&[0x0f, 0x87], "error_json_limit");
    code.emit(&[0x4c, 0x89, 0xff]); // mov    rdi,r15
    code.emit(&[0x48, 0x89, 0xd1]); // mov    rcx,rdx
    code.emit(&[0xf3, 0xa4]); // rep movs BYTE PTR es:[rdi],BYTE PTR ds:[rsi]
    code.emit(&[0x49, 0x89, 0xff]); // mov    r15,rdi
    code.emit(&[0xc3]); // ret
    Ok(())
}
