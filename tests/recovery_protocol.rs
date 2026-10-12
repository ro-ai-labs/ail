use ail::ail::*;

fn instruction(opcode: &str, operands: &[(&str, &str)]) -> AilBytecodeInstruction {
    AilBytecodeInstruction {
        opcode: opcode.into(),
        operands: operands
            .iter()
            .map(|(key, value)| ((*key).into(), (*value).into()))
            .collect(),
    }
}

fn program(instructions: Vec<AilBytecodeInstruction>) -> AilBytecodeProgram {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let mut program = compile_ail_core_bytecode(&elaborate_ail_core(&package, &document)).unwrap();
    program.actions.clear();
    program.actions.insert(
        "Run".into(),
        AilBytecodeAction {
            name: "Run".into(),
            instructions,
        },
    );
    program
}

fn scope(errors: &str) -> Vec<AilBytecodeInstruction> {
    vec![
        instruction("TRY_BEGIN", &[("errors", errors), ("key", "data.error")]),
        instruction("LABEL", &[("name", "protected")]),
        instruction("RECOVER", &[]),
        instruction("LABEL", &[("name", "handler")]),
        instruction("TRY_END", &[]),
        instruction("LABEL", &[("name", "outside")]),
        instruction("RETURN_SUCCESS", &[]),
    ]
}

#[test]
fn balanced_recovery_protocol_accepts_exact_masks_and_scope_exits() {
    for errors in [
        "any",
        "INT",
        "DIVZERO;INT",
        "BOOL;DIVZERO;FLOAT;INDEX;INT;JSON;JSON-PATH;LIST;OVERFLOW;TEXT",
    ] {
        let mut instructions = scope(errors);
        instructions.insert(2, instruction("JUMP", &[("label", "outside")]));
        let diagnostics = verify_ail_bytecode(&program(instructions));
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }
}

#[test]
fn recovery_protocol_rejects_bad_masks_metadata_and_delimiters() {
    for errors in [
        "",
        "int",
        "INT;DIVZERO",
        "INT;INT",
        "INT;JSON-LIMIT",
        "any;INT",
        "INT;",
        "INT or JSON",
    ] {
        let diagnostics = verify_ail_bytecode(&program(scope(errors)));
        assert!(
            diagnostics.iter().any(|d| d.contains("AILBC019")),
            "{errors}: {diagnostics:?}"
        );
    }
    for opcode in ["TRY_BEGIN", "RECOVER", "TRY_END"] {
        let mut instructions = scope("any");
        instructions
            .iter_mut()
            .find(|i| i.opcode == opcode)
            .unwrap()
            .operands
            .insert("extra".into(), "ignored".into());
        let diagnostics = verify_ail_bytecode(&program(instructions));
        assert!(
            diagnostics.iter().any(|d| d.contains("AILBC019")),
            "{opcode}: {diagnostics:?}"
        );
    }
    for removed in [0, 2, 4] {
        let mut instructions = scope("any");
        instructions.remove(removed);
        let diagnostics = verify_ail_bytecode(&program(instructions));
        assert!(
            diagnostics.iter().any(|d| d.contains("AILBC019")),
            "{removed}: {diagnostics:?}"
        );
    }
}

#[test]
fn recovery_protocol_rejects_jumps_into_bodies_and_sibling_regions() {
    for target in ["protected", "handler"] {
        let mut instructions = scope("any");
        instructions.insert(0, instruction("JUMP", &[("label", target)]));
        let diagnostics = verify_ail_bytecode(&program(instructions));
        assert!(
            diagnostics.iter().any(|d| d.contains("AILBC019")),
            "{target}: {diagnostics:?}"
        );
    }
    let mut instructions = scope("any");
    instructions.insert(2, instruction("JUMP", &[("label", "handler")]));
    let diagnostics = verify_ail_bytecode(&program(instructions));
    assert!(
        diagnostics.iter().any(|d| d.contains("AILBC019")),
        "{diagnostics:?}"
    );
}

#[test]
fn action_recovery_preserves_completed_callee_writes_and_failed_destination() {
    let mut program = program(vec![
        instruction("TRY_BEGIN", &[("errors", "DIVZERO"), ("key", "data.error")]),
        instruction("EVAL_INT", &[("key", "data.before"), ("expression", "i:7")]),
        instruction("CALL_ACTION", &[("target", "Child")]),
        instruction(
            "EVAL_INT",
            &[("key", "data.unreached"), ("expression", "i:1")],
        ),
        instruction("RECOVER", &[]),
        instruction(
            "EVAL_INT",
            &[("key", "data.handled"), ("expression", "i:1")],
        ),
        instruction("TRY_END", &[]),
        instruction("RETURN_SUCCESS", &[]),
    ]);
    program.actions.insert(
        "Child".into(),
        AilBytecodeAction {
            name: "Child".into(),
            instructions: vec![
                instruction(
                    "EVAL_INT",
                    &[("key", "data.shared"), ("expression", "i:11")],
                ),
                instruction(
                    "EVAL_INT",
                    &[("key", "data.destination"), ("expression", "i:1 i:0 /")],
                ),
                instruction("RETURN_SUCCESS", &[]),
            ],
        },
    );
    let output = run_ail_bytecode_action(
        &program,
        "Run",
        [("data.destination".into(), "42".into())].into(),
    )
    .unwrap();
    for (key, value) in [
        ("data.before", "7"),
        ("data.shared", "11"),
        ("data.destination", "42"),
        ("data.handled", "1"),
        ("data.error", "AIL-RUNTIME-DIVZERO"),
    ] {
        assert_eq!(output.final_state[key], value);
    }
    assert!(!output.final_state.contains_key("data.unreached"));
    assert!(
        output
            .trace
            .iter()
            .any(|line| line == "recovered DIVZERO into data.error")
    );
}

#[test]
fn value_function_can_recover_a_failing_return_before_leaving_its_frame() {
    let mut program = program(vec![
        instruction(
            "CALL_VALUE",
            &[
                ("target", "Divide"),
                ("arguments", "n=f:data.n"),
                ("key", "data.result"),
            ],
        ),
        instruction("RETURN_SUCCESS", &[]),
    ]);
    program.actions.insert(
        "Divide".into(),
        AilBytecodeAction {
            name: "Divide".into(),
            instructions: vec![
                instruction(
                    "VALUE_FUNCTION",
                    &[
                        ("parameters", "n:Int"),
                        ("locals", "error:Text"),
                        ("result", "Int"),
                    ],
                ),
                instruction(
                    "TRY_BEGIN",
                    &[("errors", "DIVZERO"), ("key", "local.error")],
                ),
                instruction(
                    "RETURN_VALUE",
                    &[("type", "Int"), ("expression", "i:8 f:local.n /")],
                ),
                instruction("RECOVER", &[]),
                instruction("RETURN_VALUE", &[("type", "Int"), ("expression", "i:9")]),
                instruction("TRY_END", &[]),
            ],
        },
    );
    for (n, expected) in [("0", "9"), ("2", "4")] {
        let output =
            run_ail_bytecode_action(&program, "Run", [("data.n".into(), n.into())].into()).unwrap();
        assert_eq!(output.final_state["data.result"], expected);
    }
}

fn nested_recovery(
    depth: usize,
    errors: &str,
    key: &str,
    body: AilBytecodeInstruction,
) -> Vec<AilBytecodeInstruction> {
    let mut code = vec![instruction("TRY_BEGIN", &[("errors", errors), ("key", key)]); depth];
    code.push(body);
    for _ in 0..depth {
        code.push(instruction("RECOVER", &[]));
        code.push(instruction("TRY_END", &[]));
    }
    code
}

#[test]
fn recovery_depth_limit_applies_to_each_original_action_before_native_inlining() {
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt;
    for root_depth in [1, 64] {
        for child_filter in ["DIVZERO", "INT"] {
            let mut code = nested_recovery(
                root_depth,
                "DIVZERO",
                "data.outer",
                instruction("CALL_ACTION", &[("target", "Child")]),
            );
            code.push(instruction("OUTPUT_FIELD", &[("key", "data.inner")]));
            code.push(instruction("OUTPUT_FIELD", &[("key", "data.outer")]));
            code.push(instruction("RETURN_SUCCESS", &[]));
            let mut program = program(code);
            let mut child = nested_recovery(
                64,
                child_filter,
                "data.inner",
                instruction(
                    "EVAL_INT",
                    &[("key", "data.result"), ("expression", "i:1 i:0 /")],
                ),
            );
            child.push(instruction("RETURN_SUCCESS", &[]));
            program.actions.insert(
                "Child".into(),
                AilBytecodeAction {
                    name: "Child".into(),
                    instructions: child,
                },
            );
            assert!(verify_ail_bytecode(&program).is_empty());
            let state = BTreeMap::from([
                ("data.inner".into(), "untouched".into()),
                ("data.outer".into(), "untouched".into()),
            ]);
            let vm = run_ail_bytecode_action(&program, "Run", state.clone()).unwrap();
            let (inner, outer) = if child_filter == "DIVZERO" {
                ("AIL-RUNTIME-DIVZERO", "untouched")
            } else {
                ("untouched", "AIL-RUNTIME-DIVZERO")
            };
            assert_eq!(vm.final_state["data.inner"], inner);
            assert_eq!(vm.final_state["data.outer"], outer);
            let binary =
                compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").unwrap();
            let path = std::env::temp_dir().join(format!(
                "ail-recovery-inlining-{}-{root_depth}-{child_filter}",
                std::process::id()
            ));
            std::fs::write(&path, binary).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            let output = std::process::Command::new(&path)
                .args(state.iter().map(|(key, value)| format!("{key}={value}")))
                .output()
                .unwrap();
            std::fs::remove_file(path).unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                format!("data.inner={inner}\ndata.outer={outer}\n")
            );
        }
    }
}

#[test]
fn sixty_five_recovery_scopes_in_one_action_remain_invalid() {
    let program = program(nested_recovery(
        65,
        "any",
        "data.error",
        instruction("EVAL_INT", &[("key", "data.value"), ("expression", "i:1")]),
    ));
    assert!(
        verify_ail_bytecode(&program)
            .iter()
            .any(|message| message.contains("AILBC019"))
    );
    assert!(run_ail_bytecode_action(&program, "Run", Default::default()).is_err());
    assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
}
