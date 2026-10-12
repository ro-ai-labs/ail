//! Native adapters for the shared allocation-free Bytes engine.
//! R15 temporarily addresses a request72/result16 stack record; every entry
//! restores it. Engine calls use the existing aligned SysV bridge. Temporary
//! outputs are tracked for recovery and released before successful publication.
use super::*;

fn kind(opcode: &str) -> Option<u8> {
    Some(match opcode {
        "BYTES_LENGTH" => 0,
        "BYTES_GET" => 1,
        "BYTES_SLICE" => 2,
        "BYTES_CONCAT" => 3,
        "BYTES_SET" => 4,
        "BYTES_FROM_UTF8" => 5,
        "BYTES_TO_UTF8" => 6,
        "BYTES_FROM_HEX" => 7,
        "BYTES_TO_HEX" => 8,
        "BYTES_COMPARE" => 9,
        _ => return None,
    })
}
pub(super) fn is_operation(opcode: &str) -> bool {
    kind(opcode).is_some()
}
pub(super) fn fields(instruction: &AilBytecodeInstruction) -> Result<BTreeSet<String>, String> {
    let Some(kind) = kind(&instruction.opcode) else {
        return Ok(BTreeSet::new());
    };
    let mut fields = BTreeSet::new();
    let operands: &[&str] = if matches!(kind, 3 | 9) {
        &["key", "left", "right"]
    } else {
        &["key", "source"]
    };
    for operand in operands {
        let key = required(instruction, operand)?;
        if !matches!(IntExpr::parse(key)?, IntExpr::Field(parsed) if parsed == key) {
            return Err("native Bytes operand must be a field".into());
        }
        fields.insert(key.into());
    }
    let numeric: &[&str] = match kind {
        1 => &["index"],
        2 => &["start", "end"],
        4 => &["index", "value"],
        _ => &[],
    };
    if instruction.operands.len() != operands.len() + numeric.len() {
        return Err("native Bytes requires exact operands".into());
    }
    for operand in numeric {
        fields.extend(IntExpr::from_postfix(required(instruction, operand)?)?.fields());
    }
    Ok(fields)
}
fn status(code: &mut X64Code) {
    for (value, label) in [
        (1, "error_bytes"),
        (2, "error_bytes_limit"),
        (3, "error_text"),
        (4, "error_text_limit"),
        (5, "error_index"),
    ] {
        code.emit(&[0x48, 0x83, 0xf8, value]);
        code.emit_jcc_label(&[0x0f, 0x84], label);
    }
    code.emit_test_rax_rax();
    code.emit_jcc_label(&[0x0f, 0x85], "error_bytes_io");
}
fn zero_request(code: &mut X64Code) {
    code.emit(&[
        0x48, 0x83, 0xec, 96, 0x49, 0x89, 0xe7, 0x48, 0x89, 0xe7, 0x31, 0xc0, 0xb9, 12, 0, 0, 0,
        0xf3, 0x48, 0xab,
    ]);
}
pub(super) fn emit_operation(
    emitter: &mut Emitter,
    instruction: &AilBytecodeInstruction,
) -> Result<(), String> {
    let kind = kind(&instruction.opcode).ok_or("unsupported native Bytes operation")?;
    fields(instruction)?;
    emitter.code.emit(&[0x41, 0x57]);
    zero_request(&mut emitter.code);
    emitter.code.emit(&[0x49, 0xc7, 0x07, kind, 0, 0, 0]);
    let operands: &[(&str, u8)] = if matches!(kind, 3 | 9) {
        &[("left", 8), ("right", 24)]
    } else {
        &[("source", 8)]
    };
    for (role, offset) in operands {
        let slot = emitter.slot(required(instruction, role)?);
        emitter.load_rax(slot + POINTER);
        emitter.code.emit(&[0x49, 0x89, 0x47, *offset]);
        emitter.load_rax(slot + LENGTH);
        emitter.code.emit(&[0x49, 0x89, 0x47, *offset + 8]);
    }
    // Validate both inputs before evaluating either numeric expression.
    emitter.code.emit(&[0x4c, 0x89, 0xff]);
    native_float::runtime_call(&mut emitter.code, "ail_bytes_validate")?;
    status(&mut emitter.code);
    let numeric: &[&str] = match kind {
        1 => &["index"],
        2 => &["start", "end"],
        4 => &["index", "value"],
        _ => &[],
    };
    for (position, role) in numeric.iter().enumerate() {
        emitter.expression(&IntExpr::from_postfix(required(instruction, role)?)?)?;
        emitter
            .code
            .emit(&[0x49, 0x89, 0x47, 40 + position as u8 * 8]);
    }
    emitter.code.emit_call_label("bytes_execute");
    let key = required(instruction, "key")?;
    if matches!(kind, 0 | 1 | 9) {
        emitter.store_integer(key);
    } else {
        let slot = emitter.slot(key);
        emitter.store_rax(slot + POINTER);
        emitter.store_rdx(slot + LENGTH);
        emitter.set(slot + NUMERIC, 0);
    }
    emitter.code.emit(&[0x48, 0x83, 0xc4, 96, 0x41, 0x5f]);
    emitter.operation_trace(&format!("computed bytes {key}\n"));
    Ok(())
}

pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    // Request R15 -> scalar or owned result RAX/RDX. Kind11 returns tracked raw
    // scratch, whose caller must release; it deliberately charges neither arena.
    code.label("bytes_execute")?;
    for kind in [0, 1, 9] {
        code.emit(&[0x49, 0x83, 0x3f, kind]);
        code.emit_jcc_label(&[0x0f, 0x84], "bytes_execute_run");
    }
    code.emit(&[
        0x31, 0xff, 0xbe, 4, 0, 0x20, 0, 0xba, 3, 0, 0, 0, 0x41, 0xba, 0x22, 0, 0, 0, 0x49, 0xc7,
        0xc0, 0xff, 0xff, 0xff, 0xff, 0x45, 0x31, 0xc9,
    ]);
    code.emit_call_label("recovery_temp_map");
    code.emit_test_rax_rax();
    code.emit_jcc_label(&[0x0f, 0x88], "error_bytes_io");
    code.emit(&[0x49, 0x89, 0x47, 56, 0x49, 0xc7, 0x47, 64, 4, 0, 0x20, 0]);
    code.label("bytes_execute_run")?;
    code.emit(&[0x4c, 0x89, 0xff, 0x49, 0x8d, 0x77, 72]);
    native_float::runtime_call(code, "ail_bytes_run")?;
    status(code);
    code.emit(&[0x49, 0x8b, 0x57, 72]);
    for kind in [0, 1, 9] {
        code.emit(&[0x49, 0x83, 0x3f, kind]);
        code.emit_jcc_label(&[0x0f, 0x84], "bytes_execute_scalar");
    }
    code.emit(&[0x49, 0x83, 0x3f, 11]);
    code.emit_jcc_label(&[0x0f, 0x84], "bytes_execute_raw");
    for kind in [6, 8] {
        code.emit(&[0x49, 0x83, 0x3f, kind]);
        code.emit_jcc_label(&[0x0f, 0x84], "bytes_execute_text");
    }
    code.emit_call_label("bytes_allocate");
    code.emit_jmp_label("bytes_execute_copy");
    code.label("bytes_execute_text")?;
    code.emit_call_label("text_allocate");
    code.label("bytes_execute_copy")?;
    code.emit(&[
        0x49, 0x8b, 0x77, 56, 0x48, 0x89, 0xc7, 0x48, 0x89, 0xd1, 0xf3, 0xa4, 0x50, 0x49, 0x8b,
        0x7f, 56,
    ]);
    code.emit_call_label("recovery_temp_unmap");
    code.emit(&[0x58, 0xc3]);
    code.label("bytes_execute_scalar")?;
    code.emit(&[0x49, 0x8b, 0x47, 80, 0xc3]);
    code.label("bytes_execute_raw")?;
    code.emit(&[0x49, 0x8b, 0x47, 56, 0xc3]);

    // Descriptor entrypoints preserve original RDI/RSI and all callee-saved
    // registers, including R15's actual function depth outside the helper.
    for (label, kind, validate_only) in [
        ("bytes_validate", 0, true),
        ("bytes_from_raw", 10, false),
        ("bytes_to_raw", 11, false),
    ] {
        code.label(label)?;
        code.emit(&[0x41, 0x57, 0x57, 0x56]);
        zero_request(code);
        code.emit(&[
            0x49, 0xc7, 0x07, kind, 0, 0, 0, 0x49, 0x8b, 0x47, 104, 0x49, 0x89, 0x47, 8, 0x49,
            0x8b, 0x47, 96, 0x49, 0x89, 0x47, 16, 0x4c, 0x89, 0xff,
        ]);
        native_float::runtime_call(code, "ail_bytes_validate")?;
        status(code);
        if !validate_only {
            code.emit_call_label("bytes_execute");
        }
        code.emit(&[0x48, 0x83, 0xc4, 96, 0x5e, 0x5f, 0x41, 0x5f, 0xc3]);
    }
    code.label("bytes_copy")?;
    code.emit(&[0x56]);
    code.emit_call_label("bytes_allocate");
    code.emit(&[0x5e, 0x48, 0x89, 0xc7, 0x48, 0x89, 0xd1, 0xf3, 0xa4, 0xc3]);

    // RDX canonical length -> RAX immutable storage. Share List/Json arena and
    // counter, with the distinct Bytes bound/category (empty Bytes costs four).
    code.label("bytes_allocate")?;
    code.emit(&[0x48, 0x81, 0xfa, 4, 0, 0x20, 0]);
    code.emit_jcc_label(&[0x0f, 0x87], "error_bytes_limit");
    code.emit(&[
        0x52, 0x48, 0x8b, 0x45, 24, 0x48, 0x01, 0xd0, 0x48, 0x3d, 0, 0, 0, 4,
    ]);
    code.emit_jcc_label(&[0x0f, 0x87], "error_bytes_limit");
    code.emit(&[0x50, 0x48, 0x83, 0x7d, 16, 0]);
    code.emit_jcc_label(&[0x0f, 0x85], "bytes_allocate_ready");
    code.emit(&[
        0xb8, 9, 0, 0, 0, 0x31, 0xff, 0xbe, 0, 0, 0, 4, 0xba, 3, 0, 0, 0, 0x41, 0xba, 0x22, 0, 0,
        0, 0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff, 0x45, 0x31, 0xc9, 0x0f, 0x05,
    ]);
    code.emit_test_rax_rax();
    code.emit_jcc_label(&[0x0f, 0x88], "error_bytes_limit");
    code.emit(&[0x48, 0x89, 0x45, 16]);
    code.label("bytes_allocate_ready")?;
    code.emit(&[
        0x48, 0x8b, 0x45, 16, 0x48, 0x03, 0x45, 24, 0x59, 0x48, 0x89, 0x4d, 24, 0x5a, 0xc3,
    ]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    fn i(opcode: &str, operands: &[(&str, &str)]) -> AilBytecodeInstruction {
        AilBytecodeInstruction {
            opcode: opcode.into(),
            operands: operands
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
    fn run(instructions: Vec<AilBytecodeInstruction>, args: &[&str]) -> std::process::Output {
        let action = AilBytecodeAction {
            name: "Bytes".into(),
            instructions,
        };
        let elf = emit(&action, &BTreeMap::new()).expect("native Bytes compilation");
        execute(elf, args)
    }
    fn execute(elf: Vec<u8>, args: &[&str]) -> std::process::Output {
        let path = std::env::temp_dir().join(format!(
            "ail-native-bytes-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, elf).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let result = std::process::Command::new(&path)
            .args(args)
            .output()
            .unwrap();
        std::fs::remove_file(path).unwrap();
        result
    }
    #[test]
    fn native_bytes_raw_operations_alias_and_unsigned_values() {
        let output = run(
            vec![
                i(
                    "BYTES_FROM_HEX",
                    &[("source", "data.hex"), ("key", "data.bytes")],
                ),
                i(
                    "BYTES_GET",
                    &[
                        ("source", "data.bytes"),
                        ("index", "i:-1"),
                        ("key", "out.last"),
                    ],
                ),
                i(
                    "BYTES_SET",
                    &[
                        ("source", "data.bytes"),
                        ("index", "i:0"),
                        ("value", "i:255"),
                        ("key", "data.bytes"),
                    ],
                ),
                i(
                    "BYTES_SLICE",
                    &[
                        ("source", "data.bytes"),
                        ("start", "i:-2"),
                        ("end", "i:99"),
                        ("key", "out.bytes"),
                    ],
                ),
                i(
                    "BYTES_LENGTH",
                    &[("source", "out.bytes"), ("key", "out.length")],
                ),
                i("OUTPUT_FIELD", &[("key", "out.last")]),
                i("OUTPUT_FIELD", &[("key", "out.bytes")]),
                i("OUTPUT_FIELD", &[("key", "out.length")]),
                i("RETURN_SUCCESS", &[]),
            ],
            &["data.hex=0080FF"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"out.bytes=hex:80ff\nout.last=255\nout.length=2\n"
        );
    }
    #[test]
    fn native_bytes_function_borrow_and_return_without_body_operations() {
        let functions = BTreeMap::from([(
            "identity".into(),
            AilBytecodeAction {
                name: "identity".into(),
                instructions: vec![
                    i(
                        "VALUE_FUNCTION",
                        &[
                            ("parameters", "body:Bytes"),
                            ("locals", ""),
                            ("result", "Bytes"),
                        ],
                    ),
                    i(
                        "RETURN_VALUE",
                        &[("expression", "f:local.body"), ("type", "Bytes")],
                    ),
                ],
            },
        )]);
        let action = AilBytecodeAction {
            name: "Call".into(),
            instructions: vec![
                i(
                    "CALL_VALUE",
                    &[
                        ("target", "identity"),
                        ("arguments", "body=f:input.body"),
                        ("key", "out.body"),
                    ],
                ),
                i("OUTPUT_FIELD", &[("key", "out.body")]),
                i("RETURN_SUCCESS", &[]),
            ],
        };
        let binary = emit_with_functions(&action, &BTreeMap::new(), &functions, &BTreeMap::new())
            .expect("Bytes function emission");
        let path =
            std::env::temp_dir().join(format!("ail-native-bytes-function-{}", std::process::id()));
        std::fs::write(&path, binary).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let output = std::process::Command::new(&path)
            .arg("input.body=hex:00ff80")
            .output()
            .unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.body=hex:00ff80\n");
    }
    #[test]
    fn native_bytes_binary_file_roundtrip() {
        let root =
            std::env::temp_dir().join(format!("ail-native-bytes-files-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let input = root.join("input");
        let output = root.join("output");
        let raw: Vec<u8> = (0..=255).collect();
        std::fs::write(&input, &raw).unwrap();
        let read = format!("--read-file=source={}", input.display());
        let write = format!("--write-file=target={}", output.display());
        let result = run(
            vec![
                i(
                    "FILE_READ_BYTES",
                    &[("resource", "source"), ("key", "data.body")],
                ),
                i(
                    "FILE_WRITE_BYTES",
                    &[("resource", "target"), ("source", "data.body")],
                ),
                i(
                    "BYTES_LENGTH",
                    &[("source", "data.body"), ("key", "out.count")],
                ),
                i("OUTPUT_FIELD", &[("key", "out.count")]),
                i("RETURN_SUCCESS", &[]),
            ],
            &[&read, &write],
        );
        assert!(result.status.success(), "{result:?}");
        assert_eq!(result.stdout, b"out.count=256\n");
        assert_eq!(std::fs::read(output).unwrap(), raw);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn native_bytes_all_conversion_concat_and_compare_paths() {
        let output = run(
            vec![
                i("SET_FIELD", &[("key", "data.text"), ("value", "A\0é🦀")]),
                i(
                    "BYTES_FROM_UTF8",
                    &[("source", "data.text"), ("key", "data.bytes")],
                ),
                i(
                    "BYTES_TO_UTF8",
                    &[("source", "data.bytes"), ("key", "out.text")],
                ),
                i(
                    "BYTES_TO_HEX",
                    &[("source", "data.bytes"), ("key", "out.hex")],
                ),
                i(
                    "BYTES_CONCAT",
                    &[
                        ("left", "data.bytes"),
                        ("right", "input.other"),
                        ("key", "input.other"),
                    ],
                ),
                i(
                    "BYTES_COMPARE",
                    &[
                        ("left", "data.bytes"),
                        ("right", "input.other"),
                        ("key", "out.order"),
                    ],
                ),
                i("OUTPUT_FIELD", &[("key", "out.text")]),
                i("OUTPUT_FIELD", &[("key", "out.hex")]),
                i("OUTPUT_FIELD", &[("key", "out.order")]),
                i("RETURN_SUCCESS", &[]),
            ],
            &["input.other=hex:ff"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            "out.hex=4100c3a9f09fa680\nout.order=-1\nout.text=A\0é🦀\n".as_bytes()
        );
    }
    #[test]
    fn native_bytes_validation_order_and_recovery_are_atomic() {
        let bad = IntExpr::parse("1 / 0").unwrap().to_postfix();
        for (source, index, value, category) in [
            ("bad", bad.as_str(), "i:256", "BYTES"),
            ("hex:00", bad.as_str(), "i:256", "DIVZERO"),
            ("hex:00", "i:99", bad.as_str(), "DIVZERO"),
            ("hex:00", "i:99", "i:256", "INDEX"),
            ("hex:00", "i:0", "i:256", "BYTES"),
        ] {
            let input = format!("input.bytes={source}");
            let output = run(
                vec![
                    i("SET_FIELD", &[("key", "out.bytes"), ("value", "hex:1122")]),
                    i("TRY_BEGIN", &[("errors", "any"), ("key", "out.error")]),
                    i(
                        "BYTES_SET",
                        &[
                            ("source", "input.bytes"),
                            ("index", index),
                            ("value", value),
                            ("key", "out.bytes"),
                        ],
                    ),
                    i("RECOVER", &[]),
                    i("TRY_END", &[]),
                    i("OUTPUT_FIELD", &[("key", "out.bytes")]),
                    i("OUTPUT_FIELD", &[("key", "out.error")]),
                    i("RETURN_SUCCESS", &[]),
                ],
                &[&input],
            );
            assert!(output.status.success(), "{output:?}");
            assert_eq!(
                output.stdout,
                format!("out.bytes=hex:1122\nout.error=AIL-RUNTIME-{category}\n").as_bytes()
            );
        }
        let output = run(
            vec![
                i("SET_FIELD", &[("key", "out.text"), ("value", "old")]),
                i("TRY_BEGIN", &[("errors", "TEXT"), ("key", "out.error")]),
                i(
                    "BYTES_TO_UTF8",
                    &[("source", "input.bytes"), ("key", "out.text")],
                ),
                i("RECOVER", &[]),
                i("TRY_END", &[]),
                i("OUTPUT_FIELD", &[("key", "out.text")]),
                i("RETURN_SUCCESS", &[]),
            ],
            &["input.bytes=hex:eda080"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.text=old\n");
    }
    #[test]
    fn native_bytes_binary_file_limits_and_pretruncate_validation() {
        let root =
            std::env::temp_dir().join(format!("ail-native-bytes-limits-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let input = root.join("input");
        let output = root.join("output");
        let read = format!("--read-file=source={}", input.display());
        let write = format!("--write-file=target={}", output.display());
        std::fs::write(&input, vec![0xff; 1_048_576]).unwrap();
        let instructions = vec![
            i(
                "FILE_READ_BYTES",
                &[("resource", "source"), ("key", "data.body")],
            ),
            i(
                "FILE_WRITE_BYTES",
                &[("resource", "target"), ("source", "data.body")],
            ),
            i(
                "BYTES_LENGTH",
                &[("source", "data.body"), ("key", "out.count")],
            ),
            i("OUTPUT_FIELD", &[("key", "out.count")]),
            i("RETURN_SUCCESS", &[]),
        ];
        let result = run(instructions.clone(), &[&read, &write]);
        assert!(result.status.success(), "{result:?}");
        assert_eq!(result.stdout, b"out.count=1048576\n");
        assert_eq!(std::fs::read(&output).unwrap(), vec![0xff; 1_048_576]);
        std::fs::write(&input, vec![0xff; 1_048_577]).unwrap();
        let result = run(instructions, &[&read, &write]);
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        assert!(String::from_utf8_lossy(&result.stderr).contains("AIL-RUNTIME-BYTES-LIMIT"));
        std::fs::write(&output, b"sentinel").unwrap();
        let write_bad = vec![
            i(
                "FILE_WRITE_BYTES",
                &[("resource", "target"), ("source", "data.body")],
            ),
            i("RETURN_SUCCESS", &[]),
        ];
        for (args, category) in [
            (vec!["data.body=hex:FF"], "CAPABILITY"),
            (vec![write.as_str(), "data.body=hex:FF"], "BYTES"),
        ] {
            let result = run(write_bad.clone(), &args);
            assert_eq!(result.status.code(), Some(1));
            assert!(
                String::from_utf8_lossy(&result.stderr)
                    .contains(&format!("AIL-RUNTIME-{category}"))
            );
            assert_eq!(std::fs::read(&output).unwrap(), b"sentinel");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn native_bytes_recursive_local_operations_preserve_inputs_and_depth() {
        let next = IntExpr::parse("local.n - 1").unwrap().to_postfix();
        let arguments = format!("body=f:local.part;n={next}");
        let functions = BTreeMap::from([(
            "tail".into(),
            AilBytecodeAction {
                name: "tail".into(),
                instructions: vec![
                    i(
                        "VALUE_FUNCTION",
                        &[
                            ("parameters", "body:Bytes;n:Int"),
                            ("locals", "part:Bytes"),
                            ("result", "Bytes"),
                        ],
                    ),
                    i(
                        "BRANCH_INT",
                        &[
                            ("left", "f:local.n"),
                            ("comparison", "=="),
                            ("right", "i:0"),
                            ("label", "done"),
                        ],
                    ),
                    i(
                        "BYTES_SLICE",
                        &[
                            ("source", "local.body"),
                            ("start", "i:1"),
                            ("end", "i:999"),
                            ("key", "local.part"),
                        ],
                    ),
                    i(
                        "CALL_VALUE",
                        &[
                            ("target", "tail"),
                            ("arguments", &arguments),
                            ("key", "local.part"),
                        ],
                    ),
                    i(
                        "RETURN_VALUE",
                        &[("expression", "f:local.part"), ("type", "Bytes")],
                    ),
                    i("LABEL", &[("name", "done")]),
                    i(
                        "RETURN_VALUE",
                        &[("expression", "f:local.body"), ("type", "Bytes")],
                    ),
                ],
            },
        )]);
        let action = AilBytecodeAction {
            name: "Run".into(),
            instructions: vec![
                i(
                    "CALL_VALUE",
                    &[
                        ("target", "tail"),
                        ("arguments", "body=f:input.body;n=i:8"),
                        ("key", "out.body"),
                    ],
                ),
                i(
                    "BYTES_LENGTH",
                    &[("source", "input.body"), ("key", "out.original")],
                ),
                i("OUTPUT_FIELD", &[("key", "out.body")]),
                i("OUTPUT_FIELD", &[("key", "out.original")]),
                i("RETURN_SUCCESS", &[]),
            ],
        };
        let output = execute(
            emit_with_functions(&action, &BTreeMap::new(), &functions, &BTreeMap::new()).unwrap(),
            &["input.body=hex:000102030405060708"],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"out.body=hex:08\nout.original=9\n");
    }
    #[test]
    fn native_bytes_returns_charge_exact_shared_list_budget() {
        let functions = BTreeMap::from([(
            "copy".into(),
            AilBytecodeAction {
                name: "copy".into(),
                instructions: vec![
                    i(
                        "VALUE_FUNCTION",
                        &[
                            ("parameters", "body:Bytes"),
                            ("locals", ""),
                            ("result", "Bytes"),
                        ],
                    ),
                    i(
                        "RETURN_VALUE",
                        &[("expression", "f:local.body"), ("type", "Bytes")],
                    ),
                ],
            },
        )]);
        // Each static input costs nothing; each successful return costs exactly
        // 2 MiB. Thirty-two returns fill the shared 64 MiB List/Json arena.
        let body = format!("\"hex:{}\"", "00".repeat(1_048_574));
        let mut instructions = vec![i(
            "ASSIGN_LITERAL",
            &[("key", "data.body"), ("type", "Bytes"), ("value", &body)],
        )];
        for _ in 0..32 {
            instructions.push(i(
                "CALL_VALUE",
                &[
                    ("target", "copy"),
                    ("arguments", "body=f:data.body"),
                    ("key", "data.copy"),
                ],
            ));
        }
        for (last, category) in [
            (
                i(
                    "BYTES_LENGTH",
                    &[("source", "data.copy"), ("key", "out.count")],
                ),
                None,
            ),
            (
                i(
                    "BYTES_SLICE",
                    &[
                        ("source", "data.copy"),
                        ("start", "i:0"),
                        ("end", "i:0"),
                        ("key", "out.copy"),
                    ],
                ),
                Some("BYTES-LIMIT"),
            ),
            (
                i("LIST_EMPTY", &[("key", "out.list"), ("type", "Int")]),
                Some("LIST-LIMIT"),
            ),
        ] {
            let mut steps = instructions.clone();
            steps.push(last);
            steps.push(i("OUTPUT_FIELD", &[("key", "out.count")]));
            steps.push(i("RETURN_SUCCESS", &[]));
            let action = AilBytecodeAction {
                name: "Budget".into(),
                instructions: steps,
            };
            let output = execute(
                emit_with_functions(&action, &BTreeMap::new(), &functions, &BTreeMap::new())
                    .unwrap(),
                &[],
            );
            if let Some(category) = category {
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .contains(&format!("AIL-RUNTIME-{category}")),
                    "{output:?}"
                );
            } else {
                assert!(output.status.success(), "{output:?}");
                assert_eq!(output.stdout, b"out.count=1048574\n");
            }
        }
    }
}
