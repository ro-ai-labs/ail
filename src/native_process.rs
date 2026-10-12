//! Caller-bound process grants and atomic native adapters for the shared engine.
//! Scratch layout: request0/result64, immutable output pointers88/96, argv4096,
//! argument bytes8192, stdout0x12000, stderr0x113000; total mapping0x214000.
use super::*;

const ROLES: [&str; 6] = [
    "arguments",
    "input",
    "timeout_ms",
    "stdout",
    "stderr",
    "status",
];

fn prefix(instruction: &AilBytecodeInstruction) -> Result<String, String> {
    let resource = required(instruction, "resource")?;
    if resource.len() > 256
        || !resource.split('.').all(|part| {
            let mut bytes = part.bytes();
            bytes
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
    {
        return Err("native process resource must be a bounded qualified ASCII identifier".into());
    }
    Ok(format!("--run-process={resource}="))
}

pub(super) fn fields(instruction: &AilBytecodeInstruction) -> Result<BTreeSet<String>, String> {
    if instruction.opcode != "PROCESS_RUN" {
        return Ok(BTreeSet::new());
    }
    if instruction.operands.len() != 7 {
        return Err("native process requires exactly seven operands".into());
    }
    prefix(instruction)?;
    let mut fields = BTreeSet::new();
    for role in ROLES {
        let key = required(instruction, role)?;
        if !matches!(IntExpr::parse(key)?, IntExpr::Field(parsed) if parsed == key) {
            return Err("native process operands must be global field keys".into());
        }
        fields.insert(key.to_owned());
    }
    let outputs = [
        required(instruction, "stdout")?,
        required(instruction, "stderr")?,
        required(instruction, "status")?,
    ];
    if outputs.into_iter().collect::<BTreeSet<_>>().len() != 3 {
        return Err("native process destinations must be distinct".into());
    }
    Ok(fields)
}

pub(super) fn grants(
    action: &AilBytecodeAction,
    base: usize,
) -> Result<BTreeMap<String, usize>, String> {
    let mut names = BTreeSet::new();
    for instruction in &action.instructions {
        if instruction.opcode == "PROCESS_RUN" {
            names.insert(prefix(instruction)?);
        }
    }
    Ok(names
        .into_iter()
        .enumerate()
        .map(|(i, name)| (name, base + i * 8))
        .collect())
}

pub(super) fn initialize(
    emitter: &mut Emitter,
    grants: &BTreeMap<String, usize>,
) -> Result<(), String> {
    validate_startup(&mut emitter.code)?;
    for (prefix, offset) in grants {
        let label = emitter.data(prefix.as_bytes());
        emitter.code.emit_lea_rsi_label(&label);
        emitter.code.emit_mov_edx_imm32(prefix.len() as u32);
        emitter.code.emit_call_label("scalar_find_last");
        emitter.store_rax(*offset);
    }
    Ok(())
}

/// Inline, helper-independent validation is also used by the legacy emitter.
/// Every reserved argument is checked before any effect; last valid grant wins.
/// Invalid startup diagnostics block SIGPIPE only on their terminal error path.
pub(super) fn validate_startup(code: &mut X64Code) -> Result<(), String> {
    code.emit(&[0x4d, 0x89, 0xe0, 0x4d, 0x89, 0xe9]); // argv/count -> r8/r9
    code.label("process_grant_scan")?;
    code.emit(&[0x4d, 0x85, 0xc9]);
    code.emit_jcc_label(&[0x0f, 0x84], "process_grants_valid");
    code.emit(&[0x49, 0x8b, 0x38]);
    for (offset, byte) in b"--run-process".iter().enumerate() {
        code.emit(&[0x80, 0x7f, offset as u8, *byte]);
        code.emit_jcc_label(&[0x0f, 0x85], "process_grant_next");
    }
    code.emit(&[0x80, 0x7f, 13, b'=']);
    code.emit_jcc_label(&[0x0f, 0x85], "process_grant_bad");
    code.emit(&[0x48, 0x83, 0xc7, 14, 0x31, 0xc9, 0x41, 0xba, 1, 0, 0, 0]); // name, count, first
    code.label("process_grant_name")?;
    code.emit(&[0x0f, 0xb6, 0x04, 0x0f, 0x3c, b'=']);
    code.emit_jcc_label(&[0x0f, 0x84], "process_grant_path");
    code.emit(&[0x81, 0xf9, 0, 1, 0, 0]);
    code.emit_jcc_label(&[0x0f, 0x83], "process_grant_bad");
    code.emit(&[0x3c, b'.']);
    code.emit_jcc_label(&[0x0f, 0x84], "process_grant_dot");
    code.emit(&[0x3c, b'_']);
    code.emit_jcc_label(&[0x0f, 0x84], "process_grant_character");
    code.emit(&[0x3c, b'A']);
    code.emit_jcc_label(&[0x0f, 0x82], "process_grant_digit");
    code.emit(&[0x3c, b'Z']);
    code.emit_jcc_label(&[0x0f, 0x86], "process_grant_character");
    code.emit(&[0x3c, b'a']);
    code.emit_jcc_label(&[0x0f, 0x82], "process_grant_bad");
    code.emit(&[0x3c, b'z']);
    code.emit_jcc_label(&[0x0f, 0x87], "process_grant_bad");
    code.label("process_grant_character")?;
    code.emit(&[0x45, 0x31, 0xd2, 0x48, 0xff, 0xc1]);
    code.emit_jmp_label("process_grant_name");
    code.label("process_grant_digit")?;
    code.emit(&[0x45, 0x85, 0xd2]);
    code.emit_jcc_label(&[0x0f, 0x85], "process_grant_bad");
    code.emit(&[0x3c, b'0']);
    code.emit_jcc_label(&[0x0f, 0x82], "process_grant_bad");
    code.emit(&[0x3c, b'9']);
    code.emit_jcc_label(&[0x0f, 0x87], "process_grant_bad");
    code.emit(&[0x48, 0xff, 0xc1]);
    code.emit_jmp_label("process_grant_name");
    code.label("process_grant_dot")?;
    code.emit(&[0x45, 0x85, 0xd2]);
    code.emit_jcc_label(&[0x0f, 0x85], "process_grant_bad");
    code.emit(&[0x41, 0xba, 1, 0, 0, 0, 0x48, 0xff, 0xc1]);
    code.emit_jmp_label("process_grant_name");
    code.label("process_grant_path")?;
    code.emit(&[0x45, 0x85, 0xd2]);
    code.emit_jcc_label(&[0x0f, 0x85], "process_grant_bad");
    code.emit(&[0x48, 0x8d, 0x7c, 0x0f, 1, 0x80, 0x3f, b'/']);
    code.emit_jcc_label(&[0x0f, 0x85], "process_grant_bad");
    code.emit(&[0x31, 0xc9]);
    code.label("process_grant_path_byte")?;
    code.emit(&[0x80, 0x3c, 0x0f, 0]);
    code.emit_jcc_label(&[0x0f, 0x84], "process_grant_next");
    code.emit(&[0x81, 0xf9, 0xff, 0x0f, 0, 0]);
    code.emit_jcc_label(&[0x0f, 0x83], "process_grant_bad");
    code.emit(&[0x48, 0xff, 0xc1]);
    code.emit_jmp_label("process_grant_path_byte");
    code.label("process_grant_next")?;
    code.emit(&[0x49, 0x83, 0xc0, 8, 0x49, 0xff, 0xc9]);
    code.emit_jmp_label("process_grant_scan");
    code.label("process_grant_bad")?;
    code.emit(&[
        0x68, 0, 0x10, 0, 0, 0x48, 0x89, 0xe6, 0x31, 0xff, 0x31, 0xd2, 0x41, 0xba, 8, 0, 0, 0,
        0xb8, 14, 0, 0, 0, 0x0f, 0x05,
    ]);
    code.emit(&[0x48, 0x85, 0xc0]);
    code.emit_jcc_label(&[0x0f, 0x88], "process_grant_exit");
    let message = b"AIL-RUNTIME-CAPABILITY: malformed process resource grant\n";
    code.emit_lea_rsi_label("process_grant_message");
    code.emit_mov_edx_imm32(message.len() as u32);
    code.emit(&[0xbf, 2, 0, 0, 0]);
    code.label("process_grant_diagnostic")?;
    code.emit(&[0xb8, 1, 0, 0, 0, 0x0f, 0x05, 0x48, 0x83, 0xf8, 0xfc]);
    code.emit_jcc_label(&[0x0f, 0x84], "process_grant_diagnostic");
    code.emit(&[0x48, 0x85, 0xc0]);
    code.emit_jcc_label(&[0x0f, 0x8e], "process_grant_exit");
    code.emit(&[0x48, 0x01, 0xc6, 0x48, 0x29, 0xc2]);
    code.emit_jcc_label(&[0x0f, 0x85], "process_grant_diagnostic");
    code.label("process_grant_exit")?;
    code.emit_exit(1);
    code.label("process_grant_message")?;
    code.emit(message);
    code.label("process_grants_valid")?;
    Ok(())
}

pub(super) fn emit_operation(
    emitter: &mut Emitter,
    instruction: &AilBytecodeInstruction,
    grants: &BTreeMap<String, usize>,
) -> Result<(), String> {
    let grant = grants
        .get(&prefix(instruction)?)
        .ok_or("missing native process grant slot")?;
    emitter.load_rdi(*grant);
    emitter.code.emit(&[0x48, 0x85, 0xff]);
    emitter
        .code
        .emit_jcc_label(&[0x0f, 0x84], "error_process_capability");
    let arguments = emitter.slot(required(instruction, "arguments")?);
    emitter.load_rsi(arguments + POINTER);
    emitter.load_rdx(arguments + LENGTH);
    emitter.code.emit_call_label("process_prepare_arguments");
    emitter.code.emit(&[0x41, 0x57, 0x49, 0x89, 0xc7]); // preserve r15, request -> r15
    let input = emitter.slot(required(instruction, "input")?);
    emitter.load_rdi(input + POINTER);
    emitter.load_rsi(input + LENGTH);
    emitter.code.emit_call_label("text_validate");
    emitter
        .code
        .emit(&[0x49, 0x89, 0x7f, 16, 0x49, 0x89, 0x77, 24]);
    emitter.integer(required(instruction, "timeout_ms")?);
    emitter.code.emit(&[0x48, 0x83, 0xf8, 1]);
    emitter.code.emit_jcc_label(&[0x0f, 0x8c], "error_process");
    emitter.code.emit(&[0x48, 0x3d, 0x60, 0xea, 0, 0]);
    emitter.code.emit_jcc_label(&[0x0f, 0x8f], "error_process");
    emitter
        .code
        .emit(&[0x49, 0x89, 0x47, 32, 0x4c, 0x89, 0xff, 0x49, 0x8d, 0x77, 64]);
    native_float::runtime_call(&mut emitter.code, "ail_process_run")?;
    for (status, label) in [
        (1, "error_process"),
        (2, "error_timeout"),
        (3, "error_process_io"),
        (4, "error_text_limit"),
    ] {
        emitter.code.emit(&[0x48, 0x83, 0xf8, status]);
        emitter.code.emit_jcc_label(&[0x0f, 0x84], label);
    }
    emitter.code.emit(&[0x48, 0x85, 0xc0]);
    emitter
        .code
        .emit_jcc_label(&[0x0f, 0x85], "error_process_io");
    emitter.code.emit(&[0x4c, 0x89, 0xff]);
    emitter.code.emit_call_label("process_publish_prepare");
    for (role, pointer, length) in [("stdout", 88, 64), ("stderr", 96, 72)] {
        let destination = emitter.slot(required(instruction, role)?);
        emitter
            .code
            .emit(&[0x49, 0x8b, 0x47, pointer, 0x49, 0x8b, 0x57, length]);
        emitter.store_rax(destination + POINTER);
        emitter.store_rdx(destination + LENGTH);
        emitter.set(destination + NUMERIC, 0);
    }
    emitter.code.emit(&[0x49, 0x8b, 0x47, 80]);
    emitter.store_integer(required(instruction, "status")?);
    emitter.code.emit(&[0x4c, 0x89, 0xff]);
    emitter.code.emit_call_label("recovery_temp_unmap");
    emitter.code.emit(&[0x41, 0x5f]);
    emitter.trace(&format!(
        "ran process resource {}\n",
        required(instruction, "resource")?
    ));
    Ok(())
}

pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    code.label("process_prepare_arguments")?;
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x41, 0x57]); // push   r15
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x48, 0x89, 0xf7]); // mov    rdi,rsi
    code.emit(&[0x48, 0x89, 0xd6]); // mov    rsi,rdx
    code.emit(&[0xba, 0x01, 0x00, 0x00, 0x00]); // mov    edx,0x1
    code.emit_call_label("list_parse");
    code.emit(&[0x49, 0x89, 0xc5]); // mov    r13,rax
    code.emit(&[0x49, 0x81, 0x7d, 0x00, 0x00, 0x01, 0x00, 0x00]); // cmp    QWORD PTR [r13+0x0],0x100
    code.emit_jcc_label(&[0x0f, 0x87], "error_process_limit");
    code.emit(&[0x4c, 0x89, 0xe7]); // mov    rdi,r12
    code.emit_call_label("scalar_strlen");
    code.emit(&[0x4c, 0x8d, 0x70, 0x01]); // lea    r14,[rax+0x1]
    code.emit(&[0x31, 0xc9]); // xor    ecx,ecx
    code.label("process_argument_size")?;
    code.emit(&[0x49, 0x3b, 0x4d, 0x00]); // cmp    rcx,QWORD PTR [r13+0x0]
    code.emit_jcc_label(&[0x0f, 0x83], "process_arguments_sized");
    code.emit(&[0x48, 0x89, 0xca]); // mov    rdx,rcx
    code.emit(&[0x48, 0xc1, 0xe2, 0x04]); // shl    rdx,0x4
    code.emit(&[0x4d, 0x03, 0xb4, 0x15, 0x08, 0x10, 0x00, 0x00]); // add    r14,QWORD PTR [r13+rdx*1+0x1008]
    code.emit(&[0x49, 0xff, 0xc6]); // inc    r14
    code.emit(&[0x48, 0xff, 0xc1]); // inc    rcx
    code.emit_jmp_label("process_argument_size");
    code.label("process_arguments_sized")?;
    code.emit(&[0x49, 0x81, 0xfe, 0x00, 0x00, 0x01, 0x00]); // cmp    r14,0x10000
    code.emit_jcc_label(&[0x0f, 0x87], "error_process_limit");
    code.emit(&[0x31, 0xff]); // xor    edi,edi
    code.emit(&[0xbe, 0x00, 0x40, 0x21, 0x00]); // mov    esi,0x214000
    code.emit(&[0xba, 0x03, 0x00, 0x00, 0x00]); // mov    edx,0x3
    code.emit(&[0x41, 0xba, 0x22, 0x00, 0x00, 0x00]); // mov    r10d,0x22
    code.emit(&[0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff]); // mov    r8,0xffffffffffffffff
    code.emit(&[0x45, 0x31, 0xc9]); // xor    r9d,r9d
    code.emit_call_label("recovery_temp_map");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_process_io");
    code.emit(&[0x49, 0x89, 0xc7]); // mov    r15,rax
    code.emit(&[0x4d, 0x89, 0x27]); // mov    QWORD PTR [r15],r12
    code.emit(&[0x49, 0x8d, 0x87, 0x00, 0x10, 0x00, 0x00]); // lea    rax,[r15+0x1000]
    code.emit(&[0x49, 0x89, 0x47, 0x08]); // mov    QWORD PTR [r15+0x8],rax
    code.emit(&[0x4c, 0x89, 0x20]); // mov    QWORD PTR [rax],r12
    code.emit(&[0x49, 0x8d, 0x87, 0x00, 0x20, 0x01, 0x00]); // lea    rax,[r15+0x12000]
    code.emit(&[0x49, 0x89, 0x47, 0x28]); // mov    QWORD PTR [r15+0x28],rax
    code.emit(&[0x49, 0x8d, 0x87, 0x00, 0x30, 0x11, 0x00]); // lea    rax,[r15+0x113000]
    code.emit(&[0x49, 0x89, 0x47, 0x30]); // mov    QWORD PTR [r15+0x30],rax
    code.emit(&[0x49, 0xc7, 0x47, 0x38, 0x01, 0x00, 0x10, 0x00]); // mov    QWORD PTR [r15+0x38],0x100001
    code.emit(&[0x45, 0x31, 0xf6]); // xor    r14d,r14d
    code.emit(&[0x49, 0x8d, 0xbf, 0x00, 0x20, 0x00, 0x00]); // lea    rdi,[r15+0x2000]
    code.label("process_argument_copy")?;
    code.emit(&[0x4d, 0x3b, 0x75, 0x00]); // cmp    r14,QWORD PTR [r13+0x0]
    code.emit_jcc_label(&[0x0f, 0x83], "process_arguments_copied");
    code.emit(&[0x4b, 0x89, 0xbc, 0xf7, 0x08, 0x10, 0x00, 0x00]); // mov    QWORD PTR [r15+r14*8+0x1008],rdi
    code.emit(&[0x4c, 0x89, 0xf0]); // mov    rax,r14
    code.emit(&[0x48, 0xc1, 0xe0, 0x04]); // shl    rax,0x4
    code.emit(&[0x49, 0x8b, 0xb4, 0x05, 0x00, 0x10, 0x00, 0x00]); // mov    rsi,QWORD PTR [r13+rax*1+0x1000]
    code.emit(&[0x49, 0x8b, 0x8c, 0x05, 0x08, 0x10, 0x00, 0x00]); // mov    rcx,QWORD PTR [r13+rax*1+0x1008]
    code.label("process_argument_bytes")?;
    code.emit(&[0x48, 0x85, 0xc9]); // test   rcx,rcx
    code.emit_jcc_label(&[0x0f, 0x84], "process_argument_end");
    code.emit(&[0xac]); // lods   al,BYTE PTR ds:[rsi]
    code.emit(&[0x84, 0xc0]); // test   al,al
    code.emit_jcc_label(&[0x0f, 0x84], "error_process");
    code.emit(&[0xaa]); // stos   BYTE PTR es:[rdi],al
    code.emit(&[0x48, 0xff, 0xc9]); // dec    rcx
    code.emit_jmp_label("process_argument_bytes");
    code.label("process_argument_end")?;
    code.emit(&[0xc6, 0x07, 0x00]); // mov    BYTE PTR [rdi],0x0
    code.emit(&[0x48, 0xff, 0xc7]); // inc    rdi
    code.emit(&[0x49, 0xff, 0xc6]); // inc    r14
    code.emit_jmp_label("process_argument_copy");
    code.label("process_arguments_copied")?;
    code.emit(&[
        0x4b, 0xc7, 0x84, 0xf7, 0x08, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]); // mov    QWORD PTR [r15+r14*8+0x1008],0x0
    code.emit(&[0x4c, 0x89, 0xef]); // mov    rdi,r13
    code.emit_call_label("recovery_temp_unmap");
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit_jcc_label(&[0x0f, 0x88], "error_process_io");
    code.emit(&[0x4c, 0x89, 0xf8]); // mov    rax,r15
    code.emit(&[0x41, 0x5f]); // pop    r15
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0xc3]); // ret
    code.label("process_publish_prepare")?;
    code.emit(&[0x41, 0x54]); // push   r12
    code.emit(&[0x41, 0x55]); // push   r13
    code.emit(&[0x41, 0x56]); // push   r14
    code.emit(&[0x49, 0x89, 0xfc]); // mov    r12,rdi
    code.emit(&[0x49, 0x8b, 0x7c, 0x24, 0x28]); // mov    rdi,QWORD PTR [r12+0x28]
    code.emit(&[0x49, 0x8b, 0x74, 0x24, 0x40]); // mov    rsi,QWORD PTR [r12+0x40]
    code.emit_call_label("text_validate");
    code.emit(&[0x49, 0x8b, 0x7c, 0x24, 0x30]); // mov    rdi,QWORD PTR [r12+0x30]
    code.emit(&[0x49, 0x8b, 0x74, 0x24, 0x48]); // mov    rsi,QWORD PTR [r12+0x48]
    code.emit_call_label("text_validate");
    code.emit(&[0x49, 0x8b, 0x44, 0x24, 0x40]); // mov    rax,QWORD PTR [r12+0x40]
    code.emit(&[0x48, 0xc7, 0xc1, 0x01, 0x00, 0x00, 0x00]); // mov    rcx,0x1
    code.emit(&[0x48, 0x85, 0xc0]); // test   rax,rax
    code.emit(&[0x48, 0x0f, 0x44, 0xc1]); // cmove  rax,rcx
    code.emit(&[0x49, 0x8b, 0x54, 0x24, 0x48]); // mov    rdx,QWORD PTR [r12+0x48]
    code.emit(&[0x48, 0x85, 0xd2]); // test   rdx,rdx
    code.emit(&[0x48, 0x0f, 0x44, 0xd1]); // cmove  rdx,rcx
    code.emit(&[0x48, 0x01, 0xd0]); // add    rax,rdx
    code.emit(&[0x48, 0x03, 0x45, 0x08]); // add    rax,QWORD PTR [rbp+0x8]
    code.emit(&[0x48, 0x3d, 0x00, 0x00, 0x00, 0x04]); // cmp    rax,0x4000000
    code.emit_jcc_label(&[0x0f, 0x87], "error_text_limit");
    code.emit(&[0x49, 0x8b, 0x54, 0x24, 0x40]); // mov    rdx,QWORD PTR [r12+0x40]
    code.emit_call_label("text_allocate");
    code.emit(&[0x49, 0x89, 0x44, 0x24, 0x58]); // mov    QWORD PTR [r12+0x58],rax
    code.emit(&[0x48, 0x89, 0xc7]); // mov    rdi,rax
    code.emit(&[0x49, 0x8b, 0x74, 0x24, 0x28]); // mov    rsi,QWORD PTR [r12+0x28]
    code.emit(&[0x49, 0x8b, 0x4c, 0x24, 0x40]); // mov    rcx,QWORD PTR [r12+0x40]
    code.emit(&[0xf3, 0xa4]); // rep movs BYTE PTR es:[rdi],BYTE PTR ds:[rsi]
    code.emit(&[0x49, 0x8b, 0x54, 0x24, 0x48]); // mov    rdx,QWORD PTR [r12+0x48]
    code.emit_call_label("text_allocate");
    code.emit(&[0x49, 0x89, 0x44, 0x24, 0x60]); // mov    QWORD PTR [r12+0x60],rax
    code.emit(&[0x48, 0x89, 0xc7]); // mov    rdi,rax
    code.emit(&[0x49, 0x8b, 0x74, 0x24, 0x30]); // mov    rsi,QWORD PTR [r12+0x30]
    code.emit(&[0x49, 0x8b, 0x4c, 0x24, 0x48]); // mov    rcx,QWORD PTR [r12+0x48]
    code.emit(&[0xf3, 0xa4]); // rep movs BYTE PTR es:[rdi],BYTE PTR ds:[rsi]
    code.emit(&[0x41, 0x5e]); // pop    r14
    code.emit(&[0x41, 0x5d]); // pop    r13
    code.emit(&[0x41, 0x5c]); // pop    r12
    code.emit(&[0xc3]); // ret
    Ok(())
}
