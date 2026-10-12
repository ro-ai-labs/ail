//! Native adapters for the shared variant engine. R15 addresses a temporary
//! request while the caller's depth register is saved on the stack.
use super::*;
const RECORD: usize = 144;
fn set(code: &mut X64Code, offset: usize, value: u64) {
    code.emit(&[0x48, 0xb8]);
    code.emit(&value.to_le_bytes());
    store(code, offset, 0);
}
// Register numbers: RAX=0, RDX=2, RSI=6, RDI=7. R15 base uses disp32.
fn store(code: &mut X64Code, offset: usize, reg: u8) {
    code.emit(&[0x49, 0x89, 0x87 | (reg << 3)]);
    code.emit(&(offset as u32).to_le_bytes());
}
fn load(code: &mut X64Code, offset: usize, reg: u8) {
    code.emit(&[0x49, 0x8b, 0x87 | (reg << 3)]);
    code.emit(&(offset as u32).to_le_bytes());
}
fn address(code: &mut X64Code, offset: usize, reg: u8) {
    code.emit(&[0x49, 0x8d, 0x87 | (reg << 3)]);
    code.emit(&(offset as u32).to_le_bytes());
}
fn status(code: &mut X64Code) {
    for (n, label) in [(1, "error_value"), (2, "error_value_limit")] {
        code.emit(&[0x48, 0x83, 0xf8, n]);
        code.emit_jcc_label(&[0x0f, 0x84], label);
    }
    code.emit_test_rax_rax();
    code.emit_jcc_label(&[0x0f, 0x85], "error_value_io");
}
fn mapping(code: &mut X64Code, offset: usize) {
    code.emit(&[
        0x31, 0xff, 0xbe, 0, 0, 0x80, 0, 0xba, 3, 0, 0, 0, 0x41, 0xba, 0x22, 0, 0, 0, 0x49, 0xc7,
        0xc0, 0xff, 0xff, 0xff, 0xff, 0x45, 0x31, 0xc9,
    ]);
    code.emit_call_label("recovery_temp_map");
    code.emit_test_rax_rax();
    code.emit_jcc_label(&[0x0f, 0x88], "error_value_limit");
    store(code, offset, 0);
    set(code, offset + 8, 8_388_608);
}
fn begin(emitter: &mut Emitter, mode: u64, ty: usize, variant: usize, field: usize, count: usize) {
    let size = RECORD + count * 48;
    let code = &mut emitter.code;
    code.emit(&[0x41, 0x57, 0x48, 0x81, 0xec]);
    code.emit(&(size as u32).to_le_bytes());
    code.emit(&[0x49, 0x89, 0xe7, 0x48, 0x89, 0xe7, 0x31, 0xc0, 0xb9]);
    code.emit(&((size / 8) as u32).to_le_bytes());
    code.emit(&[0xf3, 0x48, 0xab]);
    set(code, 0, mode);
    code.emit_lea_rsi_label(&emitter.value_schema);
    store(code, 8, 6);
    set(code, 16, emitter.value_schema_len as u64);
    set(code, 24, ty as u64);
    set(code, 32, variant as u64);
    set(code, 40, field as u64);
    address(code, RECORD, 0);
    store(code, 64, 0);
    set(code, 72, count as u64);
    mapping(code, 96);
}
fn invoke(code: &mut X64Code) -> Result<(), String> {
    code.emit(&[0x4c, 0x89, 0xff]);
    address(code, 112, 6);
    native_float::runtime_call(code, "ail_value_run")?;
    status(code);
    Ok(())
}
fn finish(code: &mut X64Code, count: usize, owned: bool) {
    code.emit(&[0x50, 0x52]);
    if owned {
        load(code, 80, 7);
        code.emit_call_label("recovery_temp_unmap");
    }
    load(code, 96, 7);
    code.emit_call_label("recovery_temp_unmap");
    code.emit(&[0x5a, 0x58, 0x48, 0x81, 0xc4]);
    code.emit(&((RECORD + count * 48) as u32).to_le_bytes());
    code.emit(&[0x41, 0x5f]);
}
fn copy_result(code: &mut X64Code, kind: u32) {
    load(code, 112, 2);
    load(code, 80, 6);
    code.emit(&[0x56]);
    code.emit_call_label(match kind {
        3 => "text_allocate",
        4 => "bytes_allocate",
        5..=7 => "list_allocate",
        8 => "json_list_allocate",
        _ => "value_allocate",
    });
    code.emit(&[0x5e, 0x48, 0x89, 0xc7, 0x48, 0x89, 0xd1, 0xf3, 0xa4]);
}
/// RDI/RSI input -> RAX/RDX borrowed or owned custom transport.
pub(super) fn boundary(emitter: &mut Emitter, kind: &str, owned: bool) -> Result<(), String> {
    let ty = value_types::type_code(&emitter.value_schemas, kind)? as usize - 9;
    emitter.code.emit(&[0x57, 0x56]);
    begin(emitter, u64::from(owned), ty, 0, 0, 0);
    load(&mut emitter.code, RECORD + 16, 0);
    store(&mut emitter.code, 48, 0);
    load(&mut emitter.code, RECORD + 8, 0);
    store(&mut emitter.code, 56, 0);
    if owned {
        mapping(&mut emitter.code, 80);
    }
    invoke(&mut emitter.code)?;
    if owned {
        copy_result(&mut emitter.code, 9);
    } else {
        load(&mut emitter.code, 48, 0);
        load(&mut emitter.code, 56, 2);
    }
    finish(&mut emitter.code, 0, owned);
    emitter.code.emit(&[0x48, 0x83, 0xc4, 16]);
    Ok(())
}
pub(super) fn emit_operation(
    emitter: &mut Emitter,
    i: &AilBytecodeInstruction,
) -> Result<(), String> {
    let kind = required(i, "type")?;
    let variant = required(i, "variant")?;
    let (ty, var, field) = value_types::indices(
        &emitter.value_schemas,
        kind,
        variant,
        i.operands.get("field").map(String::as_str),
    )?;
    let constructing = i.opcode == "VALUE_CONSTRUCT";
    let testing = i.opcode == "VALUE_TEST";
    let arguments = if constructing {
        integer_function::decode_arguments(required(i, "arguments")?)?
    } else {
        BTreeMap::new()
    };
    let payloads = emitter.value_schemas[kind][variant].clone();
    let result_kind = if constructing {
        9 + ty as u32
    } else if testing {
        2
    } else {
        value_types::type_code(&emitter.value_schemas, &payloads[required(i, "field")?])?
    };
    begin(
        emitter,
        if constructing {
            2
        } else if testing {
            3
        } else {
            4
        },
        ty,
        var,
        field.unwrap_or(0),
        arguments.len(),
    );
    if constructing {
        for (index, (name, expression)) in arguments.iter().enumerate() {
            let code = value_types::type_code(&emitter.value_schemas, &payloads[name])?;
            if code == 0 {
                emitter.expression(expression)?;
                emitter.code.emit_mov_rdi_rax();
                address(
                    &mut emitter.code,
                    RECORD + arguments.len() * 16 + (index + 1) * 32,
                    6,
                );
                emitter.code.emit_call_label("scalar_render");
            } else {
                let IntExpr::Field(key) = expression else {
                    return Err("native variant argument requires a field".into());
                };
                emitter.load_rax(emitter.slot(key) + POINTER);
                emitter.load_rdx(emitter.slot(key) + LENGTH);
            }
            store(&mut emitter.code, RECORD + index * 16, 0);
            store(&mut emitter.code, RECORD + index * 16 + 8, 2);
            store(&mut emitter.code, 48, 0);
            store(&mut emitter.code, 56, 2);
            set(&mut emitter.code, 24, code as u64);
            emitter.code.emit(&[0x4c, 0x89, 0xff]);
            native_float::runtime_call(&mut emitter.code, "ail_value_validate_argument")?;
            status(&mut emitter.code);
        }
        set(&mut emitter.code, 24, ty as u64);
    } else {
        let source = emitter.slot(required(i, "source")?);
        emitter.load_rax(source + POINTER);
        store(&mut emitter.code, 48, 0);
        emitter.load_rax(source + LENGTH);
        store(&mut emitter.code, 56, 0);
    }
    if !testing {
        mapping(&mut emitter.code, 80);
    }
    invoke(&mut emitter.code)?;
    let key = required(i, "key")?;
    let destination = emitter.slot(key);
    if testing {
        let truth = emitter.data(b"true");
        let falsehood = emitter.data(b"false");
        let no = format!("value_test_false_{}", emitter.next);
        let done = format!("value_test_done_{}", emitter.next);
        emitter.next += 1;
        load(&mut emitter.code, 120, 0);
        emitter.code.emit_test_rax_rax();
        emitter.code.emit_jcc_label(&[0x0f, 0x84], &no);
        emitter.code.emit_lea_rsi_label(&truth);
        emitter.store_rsi(destination + POINTER);
        emitter.set(destination + LENGTH, 4);
        emitter.code.emit_jmp_label(&done);
        emitter.code.label(no)?;
        emitter.code.emit_lea_rsi_label(&falsehood);
        emitter.store_rsi(destination + POINTER);
        emitter.set(destination + LENGTH, 5);
        emitter.code.label(done)?;
        emitter.set(destination + NUMERIC, 0);
    } else if result_kind <= 2 {
        emitter.lea_rsi(destination + BUFFER);
        emitter.code.emit(&[0x48, 0x89, 0xf7]);
        load(&mut emitter.code, 80, 6);
        load(&mut emitter.code, 112, 2);
        emitter.code.emit(&[0x48, 0x89, 0xd1, 0xf3, 0xa4]);
        emitter.lea_rsi(destination + BUFFER);
        emitter.store_rsi(destination + POINTER);
        emitter.store_rdx(destination + LENGTH);
        emitter.set(destination + NUMERIC, u64::from(result_kind != 2));
    } else {
        copy_result(&mut emitter.code, result_kind);
        emitter.store_rax(destination + POINTER);
        emitter.store_rdx(destination + LENGTH);
        emitter.set(destination + NUMERIC, 0);
    }
    finish(&mut emitter.code, arguments.len(), !testing);
    emitter.operation_trace(&format!("computed value {key}\n"));
    Ok(())
}
pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    code.label("value_allocate")?;
    code.emit(&[0x48, 0x81, 0xfa, 0, 0, 0x80, 0]);
    code.emit_jcc_label(&[0x0f, 0x87], "error_value_limit");
    code.emit(&[
        0x52, 0x48, 0x8b, 0x45, 24, 0x48, 0x01, 0xd0, 0x48, 0x3d, 0, 0, 0, 4,
    ]);
    code.emit_jcc_label(&[0x0f, 0x87], "error_value_limit");
    code.emit(&[0x50, 0x48, 0x83, 0x7d, 16, 0]);
    code.emit_jcc_label(&[0x0f, 0x85], "value_allocate_ready");
    code.emit(&[
        0xb8, 9, 0, 0, 0, 0x31, 0xff, 0xbe, 0, 0, 0, 4, 0xba, 3, 0, 0, 0, 0x41, 0xba, 0x22, 0, 0,
        0, 0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff, 0x45, 0x31, 0xc9, 0x0f, 0x05,
    ]);
    code.emit_test_rax_rax();
    code.emit_jcc_label(&[0x0f, 0x88], "error_value_limit");
    code.emit(&[0x48, 0x89, 0x45, 16]);
    code.label("value_allocate_ready")?;
    code.emit(&[
        0x48, 0x8b, 0x45, 16, 0x48, 0x03, 0x45, 24, 0x59, 0x48, 0x89, 0x4d, 24, 0x5a, 0xc3,
    ]);
    Ok(())
}
