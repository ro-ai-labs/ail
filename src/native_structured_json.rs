//! Bounded recursive JSON decoding and RFC 6901 traversal, emitted directly as
//! x86-64. The 8 MiB scratch mapping holds 65,536 fixed nodes, decoded strings,
//! canonical bytes, and a reusable pointer-token buffer. Semantic allocations
//! occur only after full document validation, canonicalization, and traversal.
use super::*;

pub(super) fn emit_operation(
    emitter: &mut Emitter,
    instruction: &AilBytecodeInstruction,
) -> Result<(), String> {
    let source = emitter.slot(required(instruction, "source")?);
    let key = required(instruction, "key")?;
    emitter.load_rdi(source + POINTER);
    emitter.load_rsi(source + LENGTH);
    emitter.code.emit_call_label("structured_parse");
    emitter.code.emit(&[0x50]); // retain scratch through conversion
    match instruction.opcode.as_str() {
        "JSON_KIND" => {
            emit_kind(emitter)?;
            store_pointer(emitter, key);
        }
        "JSON_KEYS" => {
            emitter.code.emit_mov_rdi_rax();
            emitter.code.emit_call_label("structured_keys");
            store_pointer(emitter, key);
        }
        "JSON_PARSE" | "JSON_FORMAT" => {
            emitter
                .code
                .emit(&[0x48, 0x8b, 0xb0, 0x40, 0x10, 0x30, 0x00]); // root canonical pointer
            emitter
                .code
                .emit(&[0x48, 0x8b, 0x90, 0x48, 0x10, 0x30, 0x00]); // root canonical length
            emitter
                .code
                .emit_call_label(if instruction.opcode == "JSON_PARSE" {
                    "json_copy_list"
                } else {
                    "json_copy_text"
                });
            store_pointer(emitter, key);
        }
        "JSON_COUNT" => {
            emitter
                .code
                .emit(&[0x48, 0x83, 0xb8, 0x00, 0x10, 0x30, 0x00, 0x04]); // root kind >= array
            emitter.code.emit_jcc_label(&[0x0f, 0x82], "error_json");
            emitter
                .code
                .emit(&[0x48, 0x8b, 0x80, 0x28, 0x10, 0x30, 0x00]); // immediate member count
            emitter.store_integer(key);
        }
        "JSON_GET" => {
            emitter.code.emit(&[0x48, 0x89, 0xc7]); // scratch
            let path = emitter.slot(required(instruction, "path")?);
            emitter.load_rsi(path + POINTER);
            emitter.load_rdx(path + LENGTH);
            emitter.code.emit_call_label("structured_pointer");
            emitter
                .code
                .emit(&[0x48, 0x8b, 0x78, 0x40, 0x48, 0x8b, 0x70, 0x48]); // selected canonical span
            let kind = native_json::value_type(instruction)?;
            if kind == "Json" {
                emitter.code.emit(&[0x48, 0x89, 0xf2, 0x48, 0x89, 0xfe]);
                emitter.code.emit_call_label("json_copy_list");
                store_pointer(emitter, key);
            } else {
                native_json::emit_conversion(emitter, key, kind, true)?;
            }
        }
        _ => return Err("unsupported native recursive JSON opcode".into()),
    }
    emitter.code.emit(&[0x5f]);
    emitter.code.emit_call_label("structured_release");
    let prefix = match instruction.opcode.as_str() {
        "JSON_PARSE" => "parsed JSON",
        "JSON_FORMAT" => "formatted JSON",
        "JSON_GET" => "got JSON",
        "JSON_KIND" => "got JSON kind",
        "JSON_KEYS" => "got JSON keys",
        _ => "counted JSON",
    };
    emitter.operation_trace(&format!("{prefix} {key}\n"));
    Ok(())
}

// Root kind is already validated. Copy the selected word rather than borrowing
// static bytes, so the Text result follows the same arena contract as the VM.
fn emit_kind(emitter: &mut Emitter) -> Result<(), String> {
    emitter
        .code
        .emit(&[0x48, 0x8b, 0x80, 0x00, 0x10, 0x30, 0x00]);
    let serial = emitter.next;
    emitter.next += 1;
    let done = format!("json_kind_copy_{serial}");
    for (kind, word) in ["null", "boolean", "number", "string", "array", "object"]
        .into_iter()
        .enumerate()
    {
        let next = format!("json_kind_next_{serial}_{kind}");
        if kind < 5 {
            emitter.code.emit(&[0x83, 0xf8, kind as u8]);
            emitter.code.emit_jcc_label(&[0x0f, 0x85], &next);
        }
        let data = emitter.data(word.as_bytes());
        emitter.code.emit_lea_rsi_label(&data);
        emitter.code.emit_mov_edx_imm32(word.len() as u32);
        if kind < 5 {
            emitter.code.emit_jmp_label(&done);
            emitter.code.label(next)?;
        }
    }
    emitter.code.label(done)?;
    emitter.code.emit_call_label("json_copy_text");
    Ok(())
}

fn store_pointer(emitter: &mut Emitter, key: &str) {
    let destination = emitter.slot(key);
    emitter.store_rax(destination + POINTER);
    emitter.store_rdx(destination + LENGTH);
    emitter.set(destination + NUMERIC, 0);
}

pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    // Reuse canonical output scratch after full-source validation. Decoded keys
    // occupy the separate preceding region, so quoted output cannot clobber
    // unread keys. Nodes remain intact while their sibling links are traversed.
    code.label("structured_keys")?;
    code.emit(&[0x48, 0x83, 0xbf, 0x00, 0x10, 0x30, 0x00, 0x05]);
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x53, 0x41, 0x56, 0x41, 0x57]);
    code.emit(&[0x49, 0x89, 0xfe]); // r14=scratch
    code.emit(&[0x4d, 0x8d, 0xbe, 0x00, 0x10, 0x20, 0x00]);
    code.emit(&[0x49, 0x8b, 0x9e, 0x18, 0x10, 0x30, 0x00]); // first child
    code.emit(&[0xb0, b'[']);
    code.emit_call_label("json_list_put_byte");
    code.emit(&[0x48, 0x85, 0xdb]);
    code.emit_jcc_label(&[0x0f, 0x84], "structured_keys_done");
    code.label("structured_keys_next")?;
    code.emit(&[0x48, 0x8b, 0x73, 0x30, 0x48, 0x8b, 0x53, 0x38]);
    code.emit_call_label("json_list_quote_string");
    code.emit(&[0x48, 0x8b, 0x5b, 0x20]); // next sibling
    code.emit(&[0x48, 0x85, 0xdb]);
    code.emit_jcc_label(&[0x0f, 0x84], "structured_keys_done");
    code.emit(&[0xb0, b',']);
    code.emit_call_label("json_list_put_byte");
    code.emit_jmp_label("structured_keys_next");
    code.label("structured_keys_done")?;
    code.emit(&[0xb0, b']']);
    code.emit_call_label("json_list_put_byte");
    code.emit(&[0x49, 0x8d, 0xb6, 0x00, 0x10, 0x20, 0x00]);
    code.emit(&[0x4c, 0x89, 0xfa, 0x48, 0x29, 0xf2]); // final length
    code.emit_call_label("json_copy_list");
    code.emit(&[0x41, 0x5f, 0x41, 0x5e, 0x5b, 0xc3]);

    // Node layout (80 bytes): kind, scalar pointer/length, first child, sibling,
    // child count, decoded object key pointer/length, canonical pointer/length.
    // Parser cursor R12/end R13, mapping R14, decoded/output cursor R15.
    // Every recursive call is bounded by the checked 64-container depth.
    code.label("structured_parse")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit_call_label("json_text_validate");
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x4c, 0x8d, 0x2c, 0x37]); // lea    r13,[rdi+rsi*1]
    code.emit(&[0xb8, 0x09, 0x00, 0x00, 0x00]); // mov    eax,0x9
    code.emit(&[0x31, 0xff]); // xor    edi,edi
    code.emit(&[0xbe, 0x00, 0x10, 0x80, 0x00]); // mov    esi,0x801000
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov    edx,0x3
    code.emit(&[0x41, 0xba, 0x22, 0x00, 0x00, 0x00]); // mov    r10d,0x22
    code.emit(&[0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]); // mov    r8,0xffffffffffffffff
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.emit_call_label("recovery_temp_map");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_json_limit");
    code.emit(&[0x49, 0x89, 0xc6]); // mov    r14,rax
    code.emit(&[0x4d, 0x8d, 0xbe, 0x00, 0x10, 0x10, 0x00]); // lea    r15,[r14+0x101000]
    code.emit_call_label("structured_value");
    code.emit(&[0x48, 0x89, 0xc3]); // mov    rbx,rax
    code.emit_call_label("json_list_skip_space");
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x4d, 0x8d, 0xbe, 0x00, 0x10, 0x20, 0x00]); // lea    r15,[r14+0x201000]
    code.emit(&[0x48, 0x89, 0xdf]); // mov    rdi,rbx
    code.emit_call_label("structured_encode");
    code.emit(&[0x4c, 0x89, 0xf0]); // mov    rax,r14
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret
    code.label("structured_release")?;
    code.emit(&[0x50]); // push   rax
    code.emit(&[0x52]); // push   rdx
    code.emit(&[0xb8, 0x0b, 0x00, 0x00, 0x00]); // mov    eax,0xb
    code.emit(&[0xbe, 0x00, 0x10, 0x80, 0x00]); // mov    esi,0x801000
    code.emit_call_label("recovery_temp_unmap");
    code.emit(&[0x5a]); // pop    rdx
    code.emit(&[0x58]); // pop    rax
    code.emit(&[0xc3]); // ret
    code.label("structured_value")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x48, 0x83, 0xec, 0x18]); // sub    rsp,0x18
    code.emit(&[0x49, 0x8b, 0x06]); // mov    rax,QWORD PTR [r14]
    code.emit(&[0x48, 0x3d, 0x00, 0x00, 0x01, 0x00]); // cmp    rax,0x10000
    code.emit_jcc_label(&[0x0f, 0x83], "error_json_limit");
    code.emit(&[0x48, 0x8d, 0x48, 0x01]); // lea    rcx,[rax+0x1]
    code.emit(&[0x49, 0x89, 0x0e]); // mov    QWORD PTR [r14],rcx
    code.emit(&[0x48, 0x6b, 0xc0, 0x50]); // imul   rax,rax,0x50
    code.emit(&[0x49, 0x8d, 0x9c, 0x06, 0x00, 0x10, 0x30, 0x00]); // lea    rbx,[r14+rax*1+0x301000]
    code.emit_call_label("json_list_skip_space");
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "error_json");
    code.emit(&[0x3c, 0x5b]); // cmp    al,0x5b
    code.emit_jcc_label(&[0x0f, 0x84], "structured_array");
    code.emit(&[0x3c, 0x7b]); // cmp    al,0x7b
    code.emit_jcc_label(&[0x0f, 0x84], "structured_object");
    code.emit(&[0x3c, 0x22]); // cmp    al,0x22
    code.emit_jcc_label(&[0x0f, 0x84], "structured_string");
    code.emit(&[0x3c, 0x74]); // cmp    al,0x74
    code.emit_jcc_label(&[0x0f, 0x84], "structured_boolean");
    code.emit(&[0x3c, 0x66]); // cmp    al,0x66
    code.emit_jcc_label(&[0x0f, 0x84], "structured_boolean");
    code.emit(&[0x3c, 0x6e]); // cmp    al,0x6e
    code.emit_jcc_label(&[0x0f, 0x84], "structured_null");
    code.emit(&[0x48, 0xc7, 0x03, 0x02, 0x00, 0x00, 0x00]); // mov    QWORD PTR [rbx],0x2
    code.emit(&[0x4c, 0x89, 0x63, 0x08]); // mov    QWORD PTR [rbx+0x8],r12
    code.emit_call_label("structured_number");
    code.emit(&[0x4c, 0x89, 0xe0]); // mov    rax,r12
    code.emit(&[0x48, 0x2b, 0x43, 0x08]); // sub    rax,QWORD PTR [rbx+0x8]
    code.emit(&[0x48, 0x89, 0x43, 0x10]); // mov    QWORD PTR [rbx+0x10],rax
    code.emit_jmp_label("structured_value_done");
    code.label("structured_string")?;
    code.emit(&[0x48, 0xc7, 0x03, 0x03, 0x00, 0x00, 0x00]); // mov    QWORD PTR [rbx],0x3
    code.emit_call_label("json_list_parse_string");
    code.emit(&[0x48, 0x89, 0x43, 0x08]); // mov    QWORD PTR [rbx+0x8],rax
    code.emit(&[0x48, 0x89, 0x53, 0x10]); // mov    QWORD PTR [rbx+0x10],rdx
    code.emit_jmp_label("structured_value_done");
    code.label("structured_boolean")?;
    code.emit(&[0x48, 0xc7, 0x03, 0x01, 0x00, 0x00, 0x00]); // mov    QWORD PTR [rbx],0x1
    code.emit(&[0x4c, 0x89, 0x63, 0x08]); // mov    QWORD PTR [rbx+0x8],r12
    code.emit_call_label("json_boolean_token");
    code.emit(&[0x4c, 0x89, 0xe0]); // mov    rax,r12
    code.emit(&[0x48, 0x2b, 0x43, 0x08]); // sub    rax,QWORD PTR [rbx+0x8]
    code.emit(&[0x48, 0x89, 0x43, 0x10]); // mov    QWORD PTR [rbx+0x10],rax
    code.emit_jmp_label("structured_value_done");
    code.label("structured_null")?;
    code.emit(&[0x4c, 0x89, 0xe8]); // mov    rax,r13
    code.emit(&[0x4c, 0x29, 0xe0]); // sub    rax,r12
    code.emit(&[0x48, 0x83, 0xf8, 0x04]); // cmp    rax,0x4
    code.emit_jcc_label(&[0x0f, 0x82], "error_json");
    code.emit(&[0x41, 0x81, 0x3c, 0x24, 0x6e, 0x75, 0x6c, 0x6c]); // cmp    DWORD PTR [r12],0x6c6c756e
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x4c, 0x89, 0x63, 0x08]); // mov    QWORD PTR [rbx+0x8],r12
    code.emit(&[0x48, 0xc7, 0x43, 0x10, 0x04, 0x00, 0x00, 0x00]); // mov    QWORD PTR [rbx+0x10],0x4
    code.emit(&[0x49, 0x83, 0xc4, 0x04]); // add    r12,0x4
    code.emit_jmp_label("structured_value_done");
    code.label("structured_array")?;
    code.emit(&[0x48, 0xc7, 0x03, 0x04, 0x00, 0x00, 0x00]); // mov    QWORD PTR [rbx],0x4
    code.emit_jmp_label("structured_container");
    code.label("structured_object")?;
    code.emit(&[0x48, 0xc7, 0x03, 0x05, 0x00, 0x00, 0x00]); // mov    QWORD PTR [rbx],0x5
    code.label("structured_container")?;
    code.emit(&[0x49, 0xff, 0x46, 0x08]); // inc    QWORD PTR [r14+0x8]
    code.emit(&[0x49, 0x83, 0x7e, 0x08, 0x40]); // cmp    QWORD PTR [r14+0x8],0x40
    code.emit_jcc_label(&[0x0f, 0x87], "error_json_limit");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x48, 0xc7, 0x04, 0x24, 0x00, 0x00, 0x00, 0x00]); // mov    QWORD PTR [rsp],0x0
    code.emit_call_label("json_list_skip_space");
    code.emit(&[0xb2, 0x5d]); // mov    dl,0x5d
    code.emit(&[0x48, 0x83, 0x3b, 0x04]); // cmp    QWORD PTR [rbx],0x4
    code.emit_jcc_label(&[0x0f, 0x84], "structured_empty_check");
    code.emit(&[0xb2, 0x7d]); // mov    dl,0x7d
    code.label("structured_empty_check")?;
    code.emit(&[0x38, 0xd0]); // cmp    al,dl
    code.emit_jcc_label(&[0x0f, 0x84], "structured_container_done");
    code.label("structured_child")?;
    code.emit(&[0x48, 0x83, 0x3b, 0x05]); // cmp    QWORD PTR [rbx],0x5
    code.emit_jcc_label(&[0x0f, 0x85], "structured_array_child");
    code.emit_call_label("json_list_skip_space");
    code.emit_call_label("json_list_parse_string");
    code.emit(&[0x48, 0x89, 0x44, 0x24, 0x08]); // mov    QWORD PTR [rsp+0x8],rax
    code.emit(&[0x48, 0x89, 0x54, 0x24, 0x10]); // mov    QWORD PTR [rsp+0x10],rdx
    code.emit_call_label("json_list_skip_space");
    code.emit(&[0x3c, 0x3a]); // cmp    al,0x3a
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.label("structured_array_child")?;
    code.emit_call_label("structured_value");
    code.emit(&[0x48, 0x83, 0x3b, 0x05]); // cmp    QWORD PTR [rbx],0x5
    code.emit_jcc_label(&[0x0f, 0x85], "structured_link");
    code.emit(&[0x48, 0x8b, 0x4c, 0x24, 0x08]); // mov    rcx,QWORD PTR [rsp+0x8]
    code.emit(&[0x48, 0x89, 0x48, 0x30]); // mov    QWORD PTR [rax+0x30],rcx
    code.emit(&[0x48, 0x8b, 0x4c, 0x24, 0x10]); // mov    rcx,QWORD PTR [rsp+0x10]
    code.emit(&[0x48, 0x89, 0x48, 0x38]); // mov    QWORD PTR [rax+0x38],rcx
    code.label("structured_link")?;
    code.emit(&[0x48, 0x8b, 0x0c, 0x24]); // mov    rcx,QWORD PTR [rsp]
    code.emit(&[0x48, 0x85, 0xc9]); // test   rcx,rcx
    code.emit_jcc_label(&[0x0f, 0x84], "structured_first_child");
    code.emit(&[0x48, 0x89, 0x41, 0x20]); // mov    QWORD PTR [rcx+0x20],rax
    code.emit_jmp_label("structured_child_linked");
    code.label("structured_first_child")?;
    code.emit(&[0x48, 0x89, 0x43, 0x18]); // mov    QWORD PTR [rbx+0x18],rax
    code.label("structured_child_linked")?;
    code.emit(&[0x48, 0x89, 0x04, 0x24]); // mov    QWORD PTR [rsp],rax
    code.emit(&[0x48, 0xff, 0x43, 0x28]); // inc    QWORD PTR [rbx+0x28]
    code.emit_call_label("json_list_skip_space");
    code.emit(&[0x3c, 0x2c]); // cmp    al,0x2c
    code.emit_jcc_label(&[0x0f, 0x84], "structured_next_child");
    code.emit(&[0xb2, 0x5d]); // mov    dl,0x5d
    code.emit(&[0x48, 0x83, 0x3b, 0x04]); // cmp    QWORD PTR [rbx],0x4
    code.emit_jcc_label(&[0x0f, 0x84], "structured_end_check");
    code.emit(&[0xb2, 0x7d]); // mov    dl,0x7d
    code.label("structured_end_check")?;
    code.emit(&[0x38, 0xd0]); // cmp    al,dl
    code.emit_jcc_label(&[0x0f, 0x85], "error_json");
    code.label("structured_container_done")?;
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x49, 0xff, 0x4e, 0x08]); // dec    QWORD PTR [r14+0x8]
    code.emit_jmp_label("structured_value_done");
    code.label("structured_next_child")?;
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit_jmp_label("structured_child");
    code.label("structured_value_done")?;
    code.emit(&[0x48, 0x89, 0xd8]); // mov    rax,rbx
    code.emit(&[0x48, 0x83, 0xc4, 0x18]); // add    rsp,0x18
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret
    code.label("structured_number")?;
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "error_json");
    code.emit(&[0x41, 0x80, 0x3c, 0x24, 0x2d]); // cmp    BYTE PTR [r12],0x2d
    code.emit_jcc_label(&[0x0f, 0x85], "structured_number_integer");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "error_json");
    code.label("structured_number_integer")?;
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x3c, 0x30]); // cmp    al,0x30
    code.emit_jcc_label(&[0x0f, 0x84], "structured_number_zero");
    code.emit(&[0x3c, 0x31]); // cmp    al,0x31
    code.emit_jcc_label(&[0x0f, 0x82], "error_json");
    code.emit(&[0x3c, 0x39]); // cmp    al,0x39
    code.emit_jcc_label(&[0x0f, 0x87], "error_json");
    code.label("structured_number_digits")?;
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "structured_number_done");
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x2c, 0x30]); // sub    al,0x30
    code.emit(&[0x3c, 0x09]); // cmp    al,0x9
    code.emit_jcc_label(&[0x0f, 0x86], "structured_number_digits");
    code.emit_jmp_label("structured_number_fraction");
    code.label("structured_number_zero")?;
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "structured_number_done");
    code.label("structured_number_fraction")?;
    code.emit(&[0x41, 0x80, 0x3c, 0x24, 0x2e]); // cmp    BYTE PTR [r12],0x2e
    code.emit_jcc_label(&[0x0f, 0x85], "structured_number_exponent");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "error_json");
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x2c, 0x30]); // sub    al,0x30
    code.emit(&[0x3c, 0x09]); // cmp    al,0x9
    code.emit_jcc_label(&[0x0f, 0x87], "error_json");
    code.label("structured_fraction_digits")?;
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "structured_number_done");
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x2c, 0x30]); // sub    al,0x30
    code.emit(&[0x3c, 0x09]); // cmp    al,0x9
    code.emit_jcc_label(&[0x0f, 0x86], "structured_fraction_digits");
    code.label("structured_number_exponent")?;
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x0c, 0x20]); // or     al,0x20
    code.emit(&[0x3c, 0x65]); // cmp    al,0x65
    code.emit_jcc_label(&[0x0f, 0x85], "structured_number_done");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "error_json");
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x3c, 0x2b]); // cmp    al,0x2b
    code.emit_jcc_label(&[0x0f, 0x84], "structured_exponent_sign");
    code.emit(&[0x3c, 0x2d]); // cmp    al,0x2d
    code.emit_jcc_label(&[0x0f, 0x85], "structured_exponent_first");
    code.label("structured_exponent_sign")?;
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "error_json");
    code.label("structured_exponent_first")?;
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x2c, 0x30]); // sub    al,0x30
    code.emit(&[0x3c, 0x09]); // cmp    al,0x9
    code.emit_jcc_label(&[0x0f, 0x87], "error_json");
    code.label("structured_exponent_digits")?;
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "structured_number_done");
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x2c, 0x30]); // sub    al,0x30
    code.emit(&[0x3c, 0x09]); // cmp    al,0x9
    code.emit_jcc_label(&[0x0f, 0x86], "structured_exponent_digits");
    code.label("structured_number_done")?;
    code.emit(&[0xc3]); // ret
    code.label("structured_encode")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x48, 0x89, 0xfb]); // mov    rbx,rdi
    code.emit(&[0x4c, 0x89, 0x7b, 0x40]); // mov    QWORD PTR [rbx+0x40],r15
    code.emit(&[0x48, 0x8b, 0x03]); // mov    rax,QWORD PTR [rbx]
    code.emit(&[0x83, 0xf8, 0x03]); // cmp    eax,0x3
    code.emit_jcc_label(&[0x0f, 0x84], "structured_encode_string");
    code.emit(&[0x83, 0xf8, 0x04]); // cmp    eax,0x4
    code.emit_jcc_label(&[0x0f, 0x83], "structured_encode_container");
    code.emit(&[0x48, 0x8b, 0x73, 0x08]); // mov    rsi,QWORD PTR [rbx+0x8]
    code.emit(&[0x48, 0x8b, 0x53, 0x10]); // mov    rdx,QWORD PTR [rbx+0x10]
    code.label("structured_encode_raw")?;
    code.emit(&[0x48, 0x85, 0xd2]); // test   rdx,rdx
    code.emit_jcc_label(&[0x0f, 0x84], "structured_encode_done");
    code.emit(&[0x8a, 0x06]); // mov    al,BYTE PTR [rsi]
    code.emit_call_label("json_list_put_byte");
    code.emit(&[0x48, 0xff, 0xc6]); // inc    rsi
    code.emit(&[0x48, 0xff, 0xca]); // dec    rdx
    code.emit_jmp_label("structured_encode_raw");
    code.label("structured_encode_string")?;
    code.emit(&[0x48, 0x8b, 0x73, 0x08]); // mov    rsi,QWORD PTR [rbx+0x8]
    code.emit(&[0x48, 0x8b, 0x53, 0x10]); // mov    rdx,QWORD PTR [rbx+0x10]
    code.emit_call_label("json_list_quote_string");
    code.emit_jmp_label("structured_encode_done");
    code.label("structured_encode_container")?;
    code.emit(&[0xb0, 0x5b]); // mov    al,0x5b
    code.emit(&[0x48, 0x83, 0x3b, 0x04]); // cmp    QWORD PTR [rbx],0x4
    code.emit_jcc_label(&[0x0f, 0x84], "structured_encode_open");
    code.emit(&[0xb0, 0x7b]); // mov    al,0x7b
    code.label("structured_encode_open")?;
    code.emit_call_label("json_list_put_byte");
    code.emit(&[0x4c, 0x8b, 0x63, 0x18]); // mov    r12,QWORD PTR [rbx+0x18]
    code.emit(&[0x4d, 0x85, 0xe4]); // test   r12,r12
    code.emit_jcc_label(&[0x0f, 0x84], "structured_encode_close");
    code.label("structured_encode_child")?;
    code.emit(&[0x48, 0x83, 0x3b, 0x05]); // cmp    QWORD PTR [rbx],0x5
    code.emit_jcc_label(&[0x0f, 0x85], "structured_encode_value");
    code.emit(&[0x49, 0x8b, 0x74, 0x24, 0x30]); // mov    rsi,QWORD PTR [r12+0x30]
    code.emit(&[0x49, 0x8b, 0x54, 0x24, 0x38]); // mov    rdx,QWORD PTR [r12+0x38]
    code.emit_call_label("json_list_quote_string");
    code.emit(&[0xb0, 0x3a]); // mov    al,0x3a
    code.emit_call_label("json_list_put_byte");
    code.label("structured_encode_value")?;
    code.emit(&[0x4c, 0x89, 0xe7]); // mov    rdi,r12
    code.emit_call_label("structured_encode");
    code.emit(&[0x4d, 0x8b, 0x64, 0x24, 0x20]); // mov    r12,QWORD PTR [r12+0x20]
    code.emit(&[0x4d, 0x85, 0xe4]); // test   r12,r12
    code.emit_jcc_label(&[0x0f, 0x84], "structured_encode_close");
    code.emit(&[0xb0, 0x2c]); // mov    al,0x2c
    code.emit_call_label("json_list_put_byte");
    code.emit_jmp_label("structured_encode_child");
    code.label("structured_encode_close")?;
    code.emit(&[0xb0, 0x5d]); // mov    al,0x5d
    code.emit(&[0x48, 0x83, 0x3b, 0x04]); // cmp    QWORD PTR [rbx],0x4
    code.emit_jcc_label(&[0x0f, 0x84], "structured_encode_end");
    code.emit(&[0xb0, 0x7d]); // mov    al,0x7d
    code.label("structured_encode_end")?;
    code.emit_call_label("json_list_put_byte");
    code.label("structured_encode_done")?;
    code.emit(&[0x4c, 0x89, 0xf8]); // mov    rax,r15
    code.emit(&[0x48, 0x2b, 0x43, 0x40]); // sub    rax,QWORD PTR [rbx+0x40]
    code.emit(&[0x48, 0x89, 0x43, 0x48]); // mov    QWORD PTR [rbx+0x48],rax
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret
    code.label("structured_pointer")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit(&[0x49, 0x89, 0xfe]); // mov    r14,rdi
    code.emit(&[0x48, 0x89, 0xf7]); // mov    rdi,rsi
    code.emit(&[0x48, 0x89, 0xd6]); // mov    rsi,rdx
    code.emit_call_label("json_text_validate");
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x4c, 0x8d, 0x2c, 0x37]); // lea    r13,[rdi+rsi*1]
    code.emit(&[0x49, 0x8d, 0x9e, 0x00, 0x10, 0x30, 0x00]); // lea    rbx,[r14+0x301000]
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "structured_pointer_done");
    code.emit(&[0x41, 0x80, 0x3c, 0x24, 0x2f]); // cmp    BYTE PTR [r12],0x2f
    code.emit_jcc_label(&[0x0f, 0x85], "error_json_path");
    code.emit(&[0x4c, 0x89, 0xe1]); // mov    rcx,r12
    code.label("structured_pointer_validate")?;
    code.emit(&[0x4c, 0x39, 0xe9]); // cmp    rcx,r13
    code.emit_jcc_label(&[0x0f, 0x84], "structured_pointer_segment");
    code.emit(&[0x8a, 0x01]); // mov    al,BYTE PTR [rcx]
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit(&[0x3c, 0x7e]); // cmp    al,0x7e
    code.emit_jcc_label(&[0x0f, 0x85], "structured_pointer_validate");
    code.emit(&[0x4c, 0x39, 0xe9]); // cmp    rcx,r13
    code.emit_jcc_label(&[0x0f, 0x84], "error_json_path");
    code.emit(&[0x8a, 0x01]); // mov    al,BYTE PTR [rcx]
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit(&[0x2c, 0x30]); // sub    al,0x30
    code.emit(&[0x3c, 0x01]); // cmp    al,0x1
    code.emit_jcc_label(&[0x0f, 0x87], "error_json_path");
    code.emit_jmp_label("structured_pointer_validate");
    code.label("structured_pointer_segment")?;
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x4d, 0x8d, 0xbe, 0x00, 0x10, 0x00, 0x00]); // lea    r15,[r14+0x1000]
    code.label("structured_pointer_decode")?;
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x84], "structured_pointer_lookup");
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x3c, 0x2f]); // cmp    al,0x2f
    code.emit_jcc_label(&[0x0f, 0x84], "structured_pointer_lookup");
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x3c, 0x7e]); // cmp    al,0x7e
    code.emit_jcc_label(&[0x0f, 0x85], "structured_pointer_byte");
    code.emit(&[0x41, 0x8a, 0x04, 0x24]); // mov    al,BYTE PTR [r12]
    code.emit(&[0x49, 0xff, 0xc4]); // inc    r12
    code.emit(&[0x3c, 0x30]); // cmp    al,0x30
    code.emit(&[0xb0, 0x7e]); // mov    al,0x7e
    code.emit_jcc_label(&[0x0f, 0x84], "structured_pointer_byte");
    code.emit(&[0xb0, 0x2f]); // mov    al,0x2f
    code.label("structured_pointer_byte")?;
    code.emit(&[0x41, 0x88, 0x07]); // mov    BYTE PTR [r15],al
    code.emit(&[0x49, 0xff, 0xc7]); // inc    r15
    code.emit_jmp_label("structured_pointer_decode");
    code.label("structured_pointer_lookup")?;
    code.emit(&[0x49, 0x8d, 0xb6, 0x00, 0x10, 0x00, 0x00]); // lea    rsi,[r14+0x1000]
    code.emit(&[0x4c, 0x89, 0xfa]); // mov    rdx,r15
    code.emit(&[0x48, 0x29, 0xf2]); // sub    rdx,rsi
    code.emit(&[0x48, 0x83, 0x3b, 0x04]); // cmp    QWORD PTR [rbx],0x4
    code.emit_jcc_label(&[0x0f, 0x84], "structured_pointer_array");
    code.emit(&[0x48, 0x83, 0x3b, 0x05]); // cmp    QWORD PTR [rbx],0x5
    code.emit_jcc_label(&[0x0f, 0x85], "error_json_path");
    code.emit(&[0x4c, 0x8b, 0x43, 0x18]); // mov    r8,QWORD PTR [rbx+0x18]
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.label("structured_pointer_member")?;
    code.emit(&[0x4d, 0x85, 0xc0]); // test   r8,r8
    code.emit_jcc_label(&[0x0f, 0x84], "structured_pointer_member_done");
    code.emit(&[0x49, 0x39, 0x50, 0x38]); // cmp    QWORD PTR [r8+0x38],rdx
    code.emit_jcc_label(&[0x0f, 0x85], "structured_pointer_member_next");
    code.emit(&[0x49, 0x8b, 0x78, 0x30]); // mov    rdi,QWORD PTR [r8+0x30]
    code.emit(&[0x31, 0xc9]); // xor    ecx,ecx
    code.label("structured_pointer_member_compare")?;
    code.emit(&[0x48, 0x39, 0xd1]); // cmp    rcx,rdx
    code.emit_jcc_label(&[0x0f, 0x84], "structured_pointer_member_match");
    code.emit(&[0x8a, 0x04, 0x0f]); // mov    al,BYTE PTR [rdi+rcx*1]
    code.emit(&[0x3a, 0x04, 0x0e]); // cmp    al,BYTE PTR [rsi+rcx*1]
    code.emit_jcc_label(&[0x0f, 0x85], "structured_pointer_member_next");
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit_jmp_label("structured_pointer_member_compare");
    code.label("structured_pointer_member_match")?;
    code.emit(&[0x4d, 0x85, 0xc9]); // test   r9,r9
    code.emit_jcc_label(&[0x0f, 0x85], "error_json_path");
    code.emit(&[0x4d, 0x89, 0xc1]); // mov    r9,r8
    code.label("structured_pointer_member_next")?;
    code.emit(&[0x4d, 0x8b, 0x40, 0x20]); // mov    r8,QWORD PTR [r8+0x20]
    code.emit_jmp_label("structured_pointer_member");
    code.label("structured_pointer_member_done")?;
    code.emit(&[0x4d, 0x85, 0xc9]); // test   r9,r9
    code.emit_jcc_label(&[0x0f, 0x84], "error_json_path");
    code.emit(&[0x4c, 0x89, 0xcb]); // mov    rbx,r9
    code.emit_jmp_label("structured_pointer_selected");
    code.label("structured_pointer_array")?;
    code.emit(&[0x48, 0x85, 0xd2]); // test   rdx,rdx
    code.emit_jcc_label(&[0x0f, 0x84], "error_json_path");
    code.emit(&[0x80, 0x3e, 0x30]); // cmp    BYTE PTR [rsi],0x30
    code.emit_jcc_label(&[0x0f, 0x85], "structured_pointer_index_start");
    code.emit(&[0x48, 0x83, 0xfa, 0x01]); // cmp    rdx,0x1
    code.emit_jcc_label(&[0x0f, 0x85], "error_json_path");
    code.label("structured_pointer_index_start")?;
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.emit(&[0x31, 0xc9]); // xor    ecx,ecx
    code.label("structured_pointer_index")?;
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
    code.emit_jcc_label(&[0x0f, 0x82], "structured_pointer_index");
    code.emit(&[0x48, 0x3b, 0x43, 0x28]); // cmp    rax,QWORD PTR [rbx+0x28]
    code.emit_jcc_label(&[0x0f, 0x83], "error_json_path");
    code.emit(&[0x48, 0x8b, 0x5b, 0x18]); // mov    rbx,QWORD PTR [rbx+0x18]
    code.label("structured_pointer_index_walk")?;
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x84], "structured_pointer_selected");
    code.emit(&[0x48, 0x8b, 0x5b, 0x20]); // mov    rbx,QWORD PTR [rbx+0x20]
    code.emit(&[0x48, 0xff, 0xc8]); // dec    rax
    code.emit_jmp_label("structured_pointer_index_walk");
    code.label("structured_pointer_selected")?;
    code.emit(&[0x4d, 0x39, 0xec]); // cmp    r12,r13
    code.emit_jcc_label(&[0x0f, 0x85], "structured_pointer_segment");
    code.label("structured_pointer_done")?;
    code.emit(&[0x48, 0x89, 0xd8]); // mov    rax,rbx
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret
    Ok(())
}
