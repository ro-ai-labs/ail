//! Named file grants have dedicated pointer slots, separate from mutable fields.
//! Linux helpers accept only regular files and preserve RBX/RBP/R12–R15.
//! Reads share bounded raw scratch, then reserve immutable Text or Bytes storage.
//! Host scratch allocation failures report IO; deterministic size/budget failures
//! use the result type’s TEXT-LIMIT/BYTES-LIMIT. Close errors are ignored without
//! retrying a released Linux fd.
use super::*;

fn prefix(instruction: &AilBytecodeInstruction) -> Result<String, String> {
    let resource = required(instruction, "resource")?;
    if resource.len() > 256
        || !resource.split('.').all(|segment| {
            let mut bytes = segment.bytes();
            bytes
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
    {
        return Err("native file resource must be an ASCII identifier of at most 256 bytes".into());
    }
    let mode = match instruction.opcode.as_str() {
        "FILE_READ_TEXT" | "FILE_READ_BYTES" => "read",
        "FILE_WRITE_TEXT" | "FILE_WRITE_BYTES" => "write",
        _ => return Err("unsupported native file operation".into()),
    };
    Ok(format!("--{mode}-file={resource}="))
}

pub(super) fn grants(
    action: &AilBytecodeAction,
    base: usize,
) -> Result<BTreeMap<String, usize>, String> {
    let mut names = BTreeSet::new();
    for instruction in &action.instructions {
        if !matches!(
            instruction.opcode.as_str(),
            "FILE_READ_TEXT" | "FILE_WRITE_TEXT" | "FILE_READ_BYTES" | "FILE_WRITE_BYTES"
        ) {
            continue;
        }
        names.insert(prefix(instruction)?);
        let operand = if matches!(
            instruction.opcode.as_str(),
            "FILE_READ_TEXT" | "FILE_READ_BYTES"
        ) {
            "key"
        } else {
            "source"
        };
        let key = required(instruction, operand)?;
        if !matches!(IntExpr::parse(key)?, IntExpr::Field(parsed) if parsed == key) {
            return Err("native file operand must be a field key".into());
        }
    }
    Ok(names
        .into_iter()
        .enumerate()
        .map(|(index, name)| (name, base + index * 8))
        .collect())
}

pub(super) fn initialize(emitter: &mut Emitter, grants: &BTreeMap<String, usize>) {
    emitter.code.emit_call_label("file_validate_grants");
    for (prefix, offset) in grants {
        let label = emitter.data(prefix.as_bytes());
        emitter.code.emit_lea_rsi_label(&label);
        emitter.code.emit_mov_edx_imm32(prefix.len() as u32);
        emitter.code.emit_call_label("scalar_find_last");
        emitter.store_rax(*offset);
    }
}

pub(super) fn emit_operation(
    emitter: &mut Emitter,
    instruction: &AilBytecodeInstruction,
    grants: &BTreeMap<String, usize>,
) -> Result<(), String> {
    let grant = prefix(instruction)?;
    let offset = *grants
        .get(&grant)
        .ok_or("native file grant slot is missing")?;
    emitter.load_rdi(offset);
    emitter.code.emit(&[0x48, 0x85, 0xff]); // test grant pointer before any field read
    emitter
        .code
        .emit_jcc_label(&[0x0f, 0x84], "error_capability");
    if matches!(
        instruction.opcode.as_str(),
        "FILE_READ_TEXT" | "FILE_READ_BYTES"
    ) {
        emitter
            .code
            .emit_call_label(if instruction.opcode == "FILE_READ_BYTES" {
                "file_read_bytes"
            } else {
                "file_read_text"
            });
        let destination = emitter.slot(required(instruction, "key")?);
        emitter.store_rax(destination + POINTER);
        emitter.store_rdx(destination + LENGTH);
        emitter.set(destination + NUMERIC, 0);
        emitter.trace(&format!(
            "read file resource {}\n",
            required(instruction, "resource")?
        ));
    } else {
        emitter.code.emit(&[0x57]); // preserve granted path
        let source = emitter.slot(required(instruction, "source")?);
        emitter.load_rdi(source + POINTER);
        emitter.load_rsi(source + LENGTH);
        if instruction.opcode == "FILE_WRITE_BYTES" {
            // Decoding completes before open/truncate. The returned tracked
            // mapping is released after the existing raw write-all helper.
            emitter.code.emit_call_label("bytes_to_raw");
            emitter.code.emit(&[0x48, 0x89, 0xc6, 0x5f, 0x50]);
            emitter.code.emit_call_label("file_write_text");
            emitter.code.emit(&[0x5f]);
            emitter.code.emit_call_label("recovery_temp_unmap");
        } else {
            emitter.code.emit_call_label("text_validate");
            emitter
                .code
                .emit(&[0x48, 0x89, 0xf2, 0x48, 0x89, 0xfe, 0x5f]);
            emitter.code.emit_call_label("file_write_text");
        }
        emitter.trace(&format!(
            "wrote file resource {}\n",
            required(instruction, "resource")?
        ));
    }
    Ok(())
}

pub(super) fn emit_helpers(code: &mut X64Code, has_bytes: bool) -> Result<(), String> {
    // Classify reserved argv prefixes without reading beyond their terminating
    // NUL. Invalid reserved forms fail even if the program never uses the grant.
    code.label("file_grant_kind")?;
    code.emit(&[0x80, 0x3f, 0x2d]); // cmp    BYTE PTR [rdi],0x2d
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_write");
    code.emit(&[0x80, 0x7f, 0x01, 0x2d]); // cmp    BYTE PTR [rdi+0x1],0x2d
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_write");
    code.emit(&[0x80, 0x7f, 0x02, 0x72]); // cmp    BYTE PTR [rdi+0x2],0x72
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_write");
    code.emit(&[0x80, 0x7f, 0x03, 0x65]); // cmp    BYTE PTR [rdi+0x3],0x65
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_write");
    code.emit(&[0x80, 0x7f, 0x04, 0x61]); // cmp    BYTE PTR [rdi+0x4],0x61
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_write");
    code.emit(&[0x80, 0x7f, 0x05, 0x64]); // cmp    BYTE PTR [rdi+0x5],0x64
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_write");
    code.emit(&[0x80, 0x7f, 0x06, 0x2d]); // cmp    BYTE PTR [rdi+0x6],0x2d
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_write");
    code.emit(&[0x80, 0x7f, 0x07, 0x66]); // cmp    BYTE PTR [rdi+0x7],0x66
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_write");
    code.emit(&[0x80, 0x7f, 0x08, 0x69]); // cmp    BYTE PTR [rdi+0x8],0x69
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_write");
    code.emit(&[0x80, 0x7f, 0x09, 0x6c]); // cmp    BYTE PTR [rdi+0x9],0x6c
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_write");
    code.emit(&[0x80, 0x7f, 0x0a, 0x65]); // cmp    BYTE PTR [rdi+0xa],0x65
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_write");
    code.emit(&[0x80, 0x7f, 0x0b, 0x3d]); // cmp    BYTE PTR [rdi+0xb],0x3d
    code.emit_jcc_label(&[0x0f, 0x85], "error_file_grant");
    code.emit(&[0xb8, 0x0c, 0x00, 0x00, 0x00]); // mov    eax,0xc
    code.emit(&[0xc3]); // ret
    code.label("file_grant_write")?;
    code.emit(&[0x80, 0x3f, 0x2d]); // cmp    BYTE PTR [rdi],0x2d
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x01, 0x2d]); // cmp    BYTE PTR [rdi+0x1],0x2d
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x02, 0x77]); // cmp    BYTE PTR [rdi+0x2],0x77
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x03, 0x72]); // cmp    BYTE PTR [rdi+0x3],0x72
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x04, 0x69]); // cmp    BYTE PTR [rdi+0x4],0x69
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x05, 0x74]); // cmp    BYTE PTR [rdi+0x5],0x74
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x06, 0x65]); // cmp    BYTE PTR [rdi+0x6],0x65
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x07, 0x2d]); // cmp    BYTE PTR [rdi+0x7],0x2d
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x08, 0x66]); // cmp    BYTE PTR [rdi+0x8],0x66
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x09, 0x69]); // cmp    BYTE PTR [rdi+0x9],0x69
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x0a, 0x6c]); // cmp    BYTE PTR [rdi+0xa],0x6c
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x0b, 0x65]); // cmp    BYTE PTR [rdi+0xb],0x65
    code.emit_jcc_label(&[0x0f, 0x85], "file_grant_none");
    code.emit(&[0x80, 0x7f, 0x0c, 0x3d]); // cmp    BYTE PTR [rdi+0xc],0x3d
    code.emit_jcc_label(&[0x0f, 0x85], "error_file_grant");
    code.emit(&[0xb8, 0x0d, 0x00, 0x00, 0x00]); // mov    eax,0xd
    code.emit(&[0xc3]); // ret
    code.label("file_grant_none")?;
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.emit(&[0xc3]); // ret

    // R12/R13 retain argv/argc. Validate identifiers and nonempty paths in every
    // binding; later reverse-prefix lookup selects the final duplicate binding.
    code.label("file_validate_grants")?;
    code.emit(&[0x45, 0x31, 0xc0]); // xor    r8d,r8d
    code.label("file_grants_loop")?;
    code.emit(&[0x4d, 0x39, 0xe8]); // cmp    r8,r13
    code.emit_jcc_label(&[0x0f, 0x83], "file_grants_done");
    code.emit(&[0x4b, 0x8b, 0x3c, 0xc4]); // mov    rdi,QWORD PTR [r12+r8*8]
    code.emit_call_label("file_grant_kind");
    code.emit(&[0x85, 0xc0]); // test   eax,eax
    code.emit_jcc_label(&[0x0f, 0x84], "file_grants_next");
    code.emit(&[0x48, 0x01, 0xc7]); // add    rdi,rax
    code.emit(&[0x31, 0xc9]); // xor    ecx,ecx
    code.emit(&[0x41, 0xb9, 0x01, 0x00, 0x00, 0x00]); // mov    r9d,0x1
    code.label("file_grant_name")?;
    code.emit(&[0x0f, 0xb6, 0x04, 0x0f]); // movzx  eax,BYTE PTR [rdi+rcx*1]
    code.emit(&[0x3c, 0x3d]); // cmp    al,0x3d
    code.emit_jcc_label(&[0x0f, 0x84], "file_grant_path");
    code.emit(&[0x81, 0xf9, 0x00, 0x01, 0x00, 0x00]); // cmp    ecx,0x100
    code.emit_jcc_label(&[0x0f, 0x83], "error_file_grant");
    code.emit(&[0x3c, 0x2e]); // cmp    al,0x2e
    code.emit_jcc_label(&[0x0f, 0x84], "file_grant_dot");
    code.emit(&[0x3c, 0x5f]); // cmp    al,0x5f
    code.emit_jcc_label(&[0x0f, 0x84], "file_grant_identifier");
    code.emit(&[0x3c, 0x41]); // cmp    al,0x41
    code.emit_jcc_label(&[0x0f, 0x82], "file_grant_digit");
    code.emit(&[0x3c, 0x5a]); // cmp    al,0x5a
    code.emit_jcc_label(&[0x0f, 0x86], "file_grant_identifier");
    code.emit(&[0x3c, 0x61]); // cmp    al,0x61
    code.emit_jcc_label(&[0x0f, 0x82], "error_file_grant");
    code.emit(&[0x3c, 0x7a]); // cmp    al,0x7a
    code.emit_jcc_label(&[0x0f, 0x87], "error_file_grant");
    code.label("file_grant_identifier")?;
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit_jmp_label("file_grant_name");
    code.label("file_grant_digit")?;
    code.emit(&[0x45, 0x85, 0xc9]); // test   r9d,r9d
    code.emit_jcc_label(&[0x0f, 0x85], "error_file_grant");
    code.emit(&[0x3c, 0x30]); // cmp    al,0x30
    code.emit_jcc_label(&[0x0f, 0x82], "error_file_grant");
    code.emit(&[0x3c, 0x39]); // cmp    al,0x39
    code.emit_jcc_label(&[0x0f, 0x87], "error_file_grant");
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit_jmp_label("file_grant_name");
    code.label("file_grant_dot")?;
    code.emit(&[0x45, 0x85, 0xc9]); // test   r9d,r9d
    code.emit_jcc_label(&[0x0f, 0x85], "error_file_grant");
    code.emit(&[0x41, 0xb9, 0x01, 0x00, 0x00, 0x00]); // mov    r9d,0x1
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit_jmp_label("file_grant_name");
    code.label("file_grant_path")?;
    code.emit(&[0x45, 0x85, 0xc9]); // test   r9d,r9d
    code.emit_jcc_label(&[0x0f, 0x85], "error_file_grant");
    code.emit(&[0x80, 0x7c, 0x0f, 0x01, 0x00]); // cmp    BYTE PTR [rdi+rcx*1+0x1],0x0
    code.emit_jcc_label(&[0x0f, 0x84], "error_file_grant");
    code.label("file_grants_next")?;
    code.emit(&[0x49, 0xff, 0xc0]); // inc    r8
    code.emit_jmp_label("file_grants_loop");
    code.label("file_grants_done")?;
    code.emit(&[0xc3]); // ret

    // Open with NONBLOCK|NOCTTY|CLOEXEC, then verify the opened descriptor's type.
    // Linux x86-64 fstat uses 144 bytes and st_mode at offset 24. Read at most
    // 1 MiB + 1 to establish size before type conversion or result budget checks.
    code.label("file_read_text")?;
    code.emit(&[0x31, 0xd2]); // text mode
    code.emit_jmp_label("file_read_common");
    if has_bytes {
        code.label("file_read_bytes")?;
        code.emit(&[0xba, 1, 0, 0, 0]);
    }
    code.label("file_read_common")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit(&[0x48, 0x81, 0xec, 0xa0, 0x00, 0x00, 0x00]); // sub rsp,160: stat144 plus mode
    code.emit(&[0x89, 0x94, 0x24, 0x90, 0, 0, 0]); // mode after stat buffer
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.label("file_read_open")?;
    code.emit(&[0xb8, 0x02, 0x00, 0x00, 0x00]); // mov    eax,0x2
    code.emit(&[0x4c, 0x89, 0xe7]); // mov    rdi,r12
    code.emit(&[0xbe, 0x00, 0x09, 0x08, 0x00]); // mov    esi,0x80900
    code.emit(&[0x31, 0xd2]); // xor    edx,edx
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x48, 0x83, 0xf8, 0xfc]); // cmp    rax,0xfffffffffffffffc
    code.emit_jcc_label(&[0x0f, 0x84], "file_read_open");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_io");
    code.emit(&[0x48, 0x89, 0xc3]); // mov    rbx,rax
    code.label("file_read_stat")?;
    code.emit(&[0xb8, 0x05, 0x00, 0x00, 0x00]); // mov    eax,0x5
    code.emit(&[0x48, 0x89, 0xdf]); // mov    rdi,rbx
    code.emit(&[0x48, 0x89, 0xe6]); // mov    rsi,rsp
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x48, 0x83, 0xf8, 0xfc]); // cmp    rax,0xfffffffffffffffc
    code.emit_jcc_label(&[0x0f, 0x84], "file_read_stat");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "file_read_io_close");
    code.emit(&[0x8b, 0x44, 0x24, 0x18]); // mov    eax,DWORD PTR [rsp+0x18]
    code.emit(&[0x25, 0x00, 0xf0, 0x00, 0x00]); // and    eax,0xf000
    code.emit(&[0x3d, 0x00, 0x80, 0x00, 0x00]); // cmp    eax,0x8000
    code.emit_jcc_label(&[0x0f, 0x85], "file_read_io_close");
    code.emit(&[0xb8, 0x09, 0x00, 0x00, 0x00]); // mov    eax,0x9
    code.emit(&[0x31, 0xff]); // xor    edi,edi
    code.emit(&[0xbe, 0x01, 0x00, 0x10, 0x00]); // mov    esi,0x100001
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov    edx,0x3
    code.emit(&[0x41, 0xba, 0x22, 0x00, 0x00, 0x00]); // mov    r10d,0x22
    code.emit(&[0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]); // mov    r8,0xffffffffffffffff
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.emit_call_label("recovery_temp_map");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "file_read_io_close");
    code.emit(&[0x49, 0x89, 0xc4]); // mov    r12,rax
    code.emit(&[0x45, 0x31, 0xed]); // xor    r13d,r13d
    code.label("file_read_loop")?;
    code.emit(&[0x31, 0xc0]); // xor    eax,eax
    code.emit(&[0x48, 0x89, 0xdf]); // mov    rdi,rbx
    code.emit(&[0x4b, 0x8d, 0x34, 0x2c]); // lea    rsi,[r12+r13*1]
    code.emit(&[0xba, 0x01, 0x00, 0x10, 0x00]); // mov    edx,0x100001
    code.emit(&[0x4c, 0x29, 0xea]); // sub    rdx,r13
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x48, 0x83, 0xf8, 0xfc]); // cmp    rax,0xfffffffffffffffc
    code.emit_jcc_label(&[0x0f, 0x84], "file_read_loop");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "file_read_io_close");
    code.emit_jcc_label(&[0x0f, 0x84], "file_read_done");
    code.emit(&[0x49, 0x01, 0xc5]); // add    r13,rax
    code.emit(&[0x49, 0x81, 0xfd, 0x00, 0x00, 0x10, 0x00]); // cmp    r13,0x100000
    code.emit_jcc_label(&[0x0f, 0x87], "file_read_limit_close");
    code.emit_jmp_label("file_read_loop");
    code.label("file_read_done")?;
    code.emit(&[0xb8, 0x03, 0x00, 0x00, 0x00]); // mov    eax,0x3
    code.emit(&[0x48, 0x89, 0xdf]); // mov    rdi,rbx
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x4c, 0x89, 0xe7, 0x4c, 0x89, 0xee]); // raw pointer/length
    if has_bytes {
        code.emit(&[0x83, 0xbc, 0x24, 0x90, 0, 0, 0, 0]);
        code.emit_jcc_label(&[0x0f, 0x84], "file_read_text_result");
        code.emit_call_label("bytes_from_raw");
        code.emit(&[0x49, 0x89, 0xc7, 0x49, 0x89, 0xd6]);
        code.emit_jmp_label("file_read_release");
        code.label("file_read_text_result")?;
    }
    code.emit_call_label("text_validate");
    code.emit(&[0x4c, 0x89, 0xea]);
    code.emit_call_label("text_allocate");
    code.emit(&[
        0x49, 0x89, 0xc7, 0x49, 0x89, 0xd6, 0x48, 0x89, 0xc7, 0x4c, 0x89, 0xe6, 0x4c, 0x89, 0xe9,
        0xf3, 0xa4,
    ]);
    if has_bytes {
        code.label("file_read_release")?;
    }
    code.emit(&[0xb8, 0x0b, 0x00, 0x00, 0x00]); // mov    eax,0xb
    code.emit(&[0x4c, 0x89, 0xe7]); // mov    rdi,r12
    code.emit(&[0xbe, 0x01, 0x00, 0x10, 0x00]); // mov    esi,0x100001
    code.emit_call_label("recovery_temp_unmap");
    code.emit(&[0x4c, 0x89, 0xf8]); // mov    rax,r15
    code.emit(&[0x4c, 0x89, 0xf2]); // result length from r14
    code.emit(&[0x48, 0x81, 0xc4, 0xa0, 0x00, 0x00, 0x00]); // add rsp,160
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret

    code.label("file_read_io_close")?;
    code.emit(&[0xb8, 3, 0, 0, 0, 0x48, 0x89, 0xdf, 0x0f, 0x05]);
    code.emit_jmp_label("error_io");
    code.label("file_read_limit_close")?;
    code.emit(&[0xb8, 3, 0, 0, 0, 0x48, 0x89, 0xdf, 0x0f, 0x05]);
    if has_bytes {
        code.emit(&[0x83, 0xbc, 0x24, 0x90, 0, 0, 0, 0]);
        code.emit_jcc_label(&[0x0f, 0x85], "error_bytes_limit");
    }
    code.emit_jmp_label("error_text_limit");

    // The caller checks authority and validates text before opening. O_TRUNC is
    // intentionally absent: fstat must approve a regular file before ftruncate.
    // EINTR retains progress; positive short writes advance by their actual size.
    code.label("file_write_text")?;
    code.emit(&[0x53]); // push   rbx
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit(&[0x48, 0x81, 0xec, 0x90, 0x00, 0x00, 0x00]); // sub    rsp,0x90
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x49, 0x89, 0xf5]); // mov    r13,rsi
    code.emit(&[0x49, 0x89, 0xd6]); // mov    r14,rdx
    code.label("file_write_open")?;
    code.emit(&[0xb8, 0x02, 0x00, 0x00, 0x00]); // mov    eax,0x2
    code.emit(&[0x4c, 0x89, 0xe7]); // mov    rdi,r12
    code.emit(&[0xbe, 0x41, 0x09, 0x08, 0x00]); // mov    esi,0x80941
    code.emit(&[0xba, 0xb6, 0x01, 0x00, 0x00]); // mov    edx,0x1b6
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x48, 0x83, 0xf8, 0xfc]); // cmp    rax,0xfffffffffffffffc
    code.emit_jcc_label(&[0x0f, 0x84], "file_write_open");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_io");
    code.emit(&[0x48, 0x89, 0xc3]); // mov    rbx,rax
    code.label("file_write_stat")?;
    code.emit(&[0xb8, 0x05, 0x00, 0x00, 0x00]); // mov    eax,0x5
    code.emit(&[0x48, 0x89, 0xdf]); // mov    rdi,rbx
    code.emit(&[0x48, 0x89, 0xe6]); // mov    rsi,rsp
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x48, 0x83, 0xf8, 0xfc]); // cmp    rax,0xfffffffffffffffc
    code.emit_jcc_label(&[0x0f, 0x84], "file_write_stat");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "file_write_io_close");
    code.emit(&[0x8b, 0x44, 0x24, 0x18]); // mov    eax,DWORD PTR [rsp+0x18]
    code.emit(&[0x25, 0x00, 0xf0, 0x00, 0x00]); // and    eax,0xf000
    code.emit(&[0x3d, 0x00, 0x80, 0x00, 0x00]); // cmp    eax,0x8000
    code.emit_jcc_label(&[0x0f, 0x85], "file_write_io_close");
    code.label("file_write_truncate")?;
    code.emit(&[0xb8, 0x4d, 0x00, 0x00, 0x00]); // mov    eax,0x4d
    code.emit(&[0x48, 0x89, 0xdf]); // mov    rdi,rbx
    code.emit(&[0x31, 0xf6]); // xor    esi,esi
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x48, 0x83, 0xf8, 0xfc]); // cmp    rax,0xfffffffffffffffc
    code.emit_jcc_label(&[0x0f, 0x84], "file_write_truncate");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "file_write_io_close");
    code.emit(&[0x45, 0x31, 0xff]); // xor    r15d,r15d
    code.label("file_write_loop")?;
    code.emit(&[0x4d, 0x39, 0xf7]); // cmp    r15,r14
    code.emit_jcc_label(&[0x0f, 0x83], "file_write_done");
    code.emit(&[0xb8, 0x01, 0x00, 0x00, 0x00]); // mov    eax,0x1
    code.emit(&[0x48, 0x89, 0xdf]); // mov    rdi,rbx
    code.emit(&[0x4b, 0x8d, 0x74, 0x3d, 0x00]); // lea    rsi,[r13+r15*1+0x0]
    code.emit(&[0x4c, 0x89, 0xf2]); // mov    rdx,r14
    code.emit(&[0x4c, 0x29, 0xfa]); // sub    rdx,r15
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x48, 0x83, 0xf8, 0xfc]); // cmp    rax,0xfffffffffffffffc
    code.emit_jcc_label(&[0x0f, 0x84], "file_write_loop");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x8e], "file_write_io_close");
    code.emit(&[0x49, 0x01, 0xc7]); // add    r15,rax
    code.emit_jmp_label("file_write_loop");
    code.label("file_write_done")?;
    code.emit(&[0xb8, 0x03, 0x00, 0x00, 0x00]); // mov    eax,0x3
    code.emit(&[0x48, 0x89, 0xdf]); // mov    rdi,rbx
    code.emit(&[0x0f, 0x05]); // syscall
    code.emit(&[0x48, 0x81, 0xc4, 0x90, 0x00, 0x00, 0x00]); // add    rsp,0x90
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0x5b]); // pop    rbx
    code.emit(&[0xc3]); // ret
    code.label("file_write_io_close")?;
    code.emit(&[0xb8, 3, 0, 0, 0, 0x48, 0x89, 0xdf, 0x0f, 0x05]);
    code.emit_jmp_label("error_io");
    Ok(())
}
