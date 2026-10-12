//! Immutable directory authority and adapters for the shared no_std engine.
use super::*;

pub(super) fn is_operation(opcode: &str) -> bool {
    matches!(
        opcode,
        "DIRECTORY_LIST_NAMES" | "DIRECTORY_READ_TEXT" | "DIRECTORY_WRITE_TEXT"
    )
}
fn prefix(instruction: &AilBytecodeInstruction) -> Result<String, String> {
    let resource = required(instruction, "resource")?;
    if !file_operations::valid_resource(resource) {
        return Err(
            "native directory resource must be a bounded qualified ASCII identifier".into(),
        );
    }
    let mode = if instruction.opcode == "DIRECTORY_WRITE_TEXT" {
        "write"
    } else {
        "read"
    };
    Ok(format!("--{mode}-directory={resource}="))
}
pub(super) fn fields(instruction: &AilBytecodeInstruction) -> Result<BTreeSet<String>, String> {
    if !is_operation(&instruction.opcode) {
        return Ok(BTreeSet::new());
    }
    if instruction.operands.len() != 3 {
        return Err("native directory requires exactly three operands".into());
    }
    prefix(instruction)?;
    let mut fields = BTreeSet::new();
    for role in [
        "path",
        if instruction.opcode == "DIRECTORY_WRITE_TEXT" {
            "source"
        } else {
            "key"
        },
    ] {
        let key = required(instruction, role)?;
        if !matches!(IntExpr::parse(key)?, IntExpr::Field(parsed) if parsed == key) {
            return Err("native directory operands must be global field keys".into());
        }
        fields.insert(key.into());
    }
    Ok(fields)
}
pub(super) fn grants(
    action: &AilBytecodeAction,
    base: usize,
) -> Result<BTreeMap<String, usize>, String> {
    let mut names = BTreeSet::new();
    for instruction in &action.instructions {
        if is_operation(&instruction.opcode) {
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

// Inline and helper-independent: also called by the legacy emitter. Every
// reserved flag is checked, including overwritten and unused grants. The fatal
// diagnostic blocks SIGPIPE only on its terminal path and grants no authority.
pub(super) fn validate_startup(code: &mut X64Code) -> Result<(), String> {
    code.emit(&[0x4d, 0x89, 0xe0, 0x4d, 0x89, 0xe9]); // argv/count -> r8/r9
    code.label("directory_grant_scan")?;
    code.emit(&[0x4d, 0x85, 0xc9]);
    code.emit_jcc_label(&[0x0f, 0x84], "directory_grants_valid");
    code.emit(&[0x49, 0x8b, 0x38]);
    for (prefix, next) in [
        (b"--read-directory".as_slice(), "directory_grant_write"),
        (b"--write-directory".as_slice(), "directory_grant_next"),
    ] {
        for (offset, byte) in prefix.iter().enumerate() {
            code.emit(&[0x80, 0x7f, offset as u8, *byte]);
            code.emit_jcc_label(&[0x0f, 0x85], next);
        }
        code.emit(&[0x80, 0x7f, prefix.len() as u8, b'=']);
        code.emit_jcc_label(&[0x0f, 0x85], "directory_grant_bad");
        code.emit(&[
            0x48,
            0x83,
            0xc7,
            (prefix.len() + 1) as u8,
            0x31,
            0xc9,
            0x41,
            0xba,
            1,
            0,
            0,
            0,
        ]);
        code.emit_jmp_label("directory_grant_name");
        if next == "directory_grant_write" {
            code.label(next)?;
        }
    }
    code.label("directory_grant_name")?;
    code.emit(&[0x0f, 0xb6, 0x04, 0x0f, 0x3c, b'=']);
    code.emit_jcc_label(&[0x0f, 0x84], "directory_grant_path");
    code.emit(&[0x81, 0xf9, 0, 1, 0, 0]);
    code.emit_jcc_label(&[0x0f, 0x83], "directory_grant_bad");
    code.emit(&[0x3c, b'.']);
    code.emit_jcc_label(&[0x0f, 0x84], "directory_grant_dot");
    code.emit(&[0x3c, b'_']);
    code.emit_jcc_label(&[0x0f, 0x84], "directory_grant_character");
    code.emit(&[0x3c, b'A']);
    code.emit_jcc_label(&[0x0f, 0x82], "directory_grant_digit");
    code.emit(&[0x3c, b'Z']);
    code.emit_jcc_label(&[0x0f, 0x86], "directory_grant_character");
    code.emit(&[0x3c, b'a']);
    code.emit_jcc_label(&[0x0f, 0x82], "directory_grant_bad");
    code.emit(&[0x3c, b'z']);
    code.emit_jcc_label(&[0x0f, 0x87], "directory_grant_bad");
    code.label("directory_grant_character")?;
    code.emit(&[0x45, 0x31, 0xd2, 0x48, 0xff, 0xc1]);
    code.emit_jmp_label("directory_grant_name");
    code.label("directory_grant_digit")?;
    code.emit(&[0x45, 0x85, 0xd2]);
    code.emit_jcc_label(&[0x0f, 0x85], "directory_grant_bad");
    code.emit(&[0x3c, b'0']);
    code.emit_jcc_label(&[0x0f, 0x82], "directory_grant_bad");
    code.emit(&[0x3c, b'9']);
    code.emit_jcc_label(&[0x0f, 0x87], "directory_grant_bad");
    code.emit(&[0x48, 0xff, 0xc1]);
    code.emit_jmp_label("directory_grant_name");
    code.label("directory_grant_dot")?;
    code.emit(&[0x45, 0x85, 0xd2]);
    code.emit_jcc_label(&[0x0f, 0x85], "directory_grant_bad");
    code.emit(&[0x41, 0xba, 1, 0, 0, 0, 0x48, 0xff, 0xc1]);
    code.emit_jmp_label("directory_grant_name");
    code.label("directory_grant_path")?;
    code.emit(&[0x45, 0x85, 0xd2]);
    code.emit_jcc_label(&[0x0f, 0x85], "directory_grant_bad");
    code.emit(&[0x48, 0x8d, 0x7c, 0x0f, 1, 0x80, 0x3f, b'/']);
    code.emit_jcc_label(&[0x0f, 0x85], "directory_grant_bad");
    code.emit(&[0x31, 0xc9]);
    code.label("directory_grant_path_byte")?;
    code.emit(&[0x80, 0x3c, 0x0f, 0]);
    code.emit_jcc_label(&[0x0f, 0x84], "directory_grant_next");
    code.emit(&[0x81, 0xf9, 0xff, 0x0f, 0, 0]);
    code.emit_jcc_label(&[0x0f, 0x83], "directory_grant_bad");
    code.emit(&[0x48, 0xff, 0xc1]);
    code.emit_jmp_label("directory_grant_path_byte");
    code.label("directory_grant_next")?;
    code.emit(&[0x49, 0x83, 0xc0, 8, 0x49, 0xff, 0xc9]);
    code.emit_jmp_label("directory_grant_scan");
    code.label("directory_grant_bad")?;
    code.emit(&[
        0x68, 0, 0x10, 0, 0, 0x48, 0x89, 0xe6, 0x31, 0xff, 0x31, 0xd2, 0x41, 0xba, 8, 0, 0, 0,
        0xb8, 14, 0, 0, 0, 0x0f, 0x05,
    ]);
    code.emit(&[0x48, 0x85, 0xc0]);
    code.emit_jcc_label(&[0x0f, 0x88], "directory_grant_exit");
    let message = b"AIL-RUNTIME-CAPABILITY: malformed directory resource grant\n";
    code.emit_lea_rsi_label("directory_grant_message");
    code.emit_mov_edx_imm32(message.len() as u32);
    code.emit(&[0xbf, 2, 0, 0, 0]);
    code.label("directory_grant_diagnostic")?;
    code.emit(&[0xb8, 1, 0, 0, 0, 0x0f, 0x05, 0x48, 0x83, 0xf8, 0xfc]);
    code.emit_jcc_label(&[0x0f, 0x84], "directory_grant_diagnostic");
    code.emit(&[0x48, 0x85, 0xc0]);
    code.emit_jcc_label(&[0x0f, 0x8e], "directory_grant_exit");
    code.emit(&[0x48, 0x01, 0xc6, 0x48, 0x29, 0xc2]);
    code.emit_jcc_label(&[0x0f, 0x85], "directory_grant_diagnostic");
    code.label("directory_grant_exit")?;
    code.emit_exit(1);
    code.label("directory_grant_message")?;
    code.emit(message);
    code.label("directory_grants_valid")?;
    Ok(())
}

fn status(code: &mut X64Code) {
    for (value, label) in [
        (1, "error_path"),
        (2, "error_text"),
        (3, "error_text_limit"),
        (4, "error_list_limit"),
    ] {
        code.emit(&[0x48, 0x83, 0xf8, value]);
        code.emit_jcc_label(&[0x0f, 0x84], label);
    }
    code.emit_test_rax_rax();
    code.emit_jcc_label(&[0x0f, 0x85], "error_directory_io");
}
pub(super) fn emit_operation(
    emitter: &mut Emitter,
    instruction: &AilBytecodeInstruction,
    grants: &BTreeMap<String, usize>,
) -> Result<(), String> {
    let grant = *grants
        .get(&prefix(instruction)?)
        .ok_or("missing native directory grant slot")?;
    emitter.load_rax(grant);
    emitter.code.emit_test_rax_rax();
    emitter
        .code
        .emit_jcc_label(&[0x0f, 0x84], "error_directory_capability");
    // Save R15, then reserve and zero request80/result16 on the runtime stack.
    emitter.code.emit(&[
        0x41, 0x57, 0x48, 0x83, 0xec, 96, 0x49, 0x89, 0xe7, 0x48, 0x89, 0xe7, 0x31, 0xc0, 0xb9, 12,
        0, 0, 0, 0xf3, 0x48, 0xab,
    ]);
    let kind = match instruction.opcode.as_str() {
        "DIRECTORY_LIST_NAMES" => 0,
        "DIRECTORY_READ_TEXT" => 1,
        _ => 2,
    };
    emitter.code.emit(&[0x49, 0xc7, 0x07, kind, 0, 0, 0]);
    emitter.load_rax(grant);
    emitter.code.emit(&[0x49, 0x89, 0x47, 8]);
    for (role, offset) in [("path", 16), ("source", 32)] {
        if role == "source" && kind != 2 {
            continue;
        }
        let field = emitter.slot(required(instruction, role)?);
        emitter.load_rax(field + POINTER);
        emitter.code.emit(&[0x49, 0x89, 0x47, offset]);
        emitter.load_rax(field + LENGTH);
        emitter.code.emit(&[0x49, 0x89, 0x47, offset + 8]);
    }
    emitter.code.emit(&[0x4c, 0x89, 0xff]);
    native_float::runtime_call(&mut emitter.code, "ail_directory_validate")?;
    status(&mut emitter.code);
    // One tracked mapping: scratch0x311000, followed by output0x100001.
    emitter.code.emit(&[
        0x31, 0xff, 0xbe, 0, 0x20, 0x41, 0, 0xba, 3, 0, 0, 0, 0x41, 0xba, 0x22, 0, 0, 0, 0x49,
        0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff, 0x45, 0x31, 0xc9,
    ]);
    emitter.code.emit_call_label("recovery_temp_map");
    emitter.code.emit_test_rax_rax();
    emitter
        .code
        .emit_jcc_label(&[0x0f, 0x88], "error_directory_io");
    emitter.code.emit(&[
        0x49, 0x89, 0x47, 64, 0x49, 0xc7, 0x47, 72, 0, 0x10, 0x31, 0, 0x48, 0x05, 0, 0x10, 0x31, 0,
        0x49, 0x89, 0x47, 48, 0x49, 0xc7, 0x47, 56, 1, 0, 0x10, 0, 0x4c, 0x89, 0xff, 0x49, 0x8d,
        0x77, 80,
    ]);
    native_float::runtime_call(&mut emitter.code, "ail_directory_run")?;
    status(&mut emitter.code);
    if kind != 2 {
        // Engine returned a complete validated result. Charge exactly one arena.
        emitter.code.emit(&[0x49, 0x8b, 0x57, 80]);
        emitter.code.emit_call_label(if kind == 0 {
            "list_allocate"
        } else {
            "text_allocate"
        });
        emitter.code.emit(&[
            0x49, 0x8b, 0x77, 48, 0x48, 0x89, 0xc7, 0x48, 0x89, 0xd1, 0xf3, 0xa4,
        ]);
        let destination = emitter.slot(required(instruction, "key")?);
        emitter.store_rax(destination + POINTER);
        emitter.store_rdx(destination + LENGTH);
        emitter.set(destination + NUMERIC, 0);
    }
    emitter.code.emit(&[0x49, 0x8b, 0x7f, 64]);
    emitter.code.emit_call_label("recovery_temp_unmap");
    emitter.code.emit(&[0x48, 0x83, 0xc4, 96, 0x41, 0x5f]);
    let verb = ["listed names in", "read text from", "wrote text to"][kind as usize];
    emitter.trace(&format!(
        "{verb} directory resource {}\n",
        required(instruction, "resource")?
    ));
    Ok(())
}
