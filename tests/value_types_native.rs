use ail::ail::*;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn code(op: &str, args: &[(&str, &str)]) -> AilBytecodeInstruction {
    AilBytecodeInstruction {
        opcode: op.into(),
        operands: args
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}
fn program(steps: Vec<AilBytecodeInstruction>) -> AilBytecodeProgram {
    let mut program = parse_ail_bytecode(r#"{"kind":"AIL-Bytecode","package":"value-test","version":"1","profile":"Application","actions":[],"failures":[],"type_schemas":{"Tree":{"Empty":{},"Node":{"left":"Tree","right":"Tree","value":"Int"}}}}"#).unwrap();
    program.actions.insert(
        "Run".into(),
        AilBytecodeAction {
            name: "Run".into(),
            instructions: steps,
        },
    );
    program
}
fn run(program: &AilBytecodeProgram, args: &[&str]) -> std::process::Output {
    use std::os::unix::fs::PermissionsExt;
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let bytes = compile_ail_bytecode_native_elf(program, "Run", "linux-x86_64-elf")
        .expect("native executable variants");
    let path = std::env::temp_dir().join(format!(
        "ail-value-native-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, bytes).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut attempts = 0;
    let output = loop {
        match Command::new(&path).args(args).output() {
            Ok(output) => break output,
            Err(error) if error.raw_os_error() == Some(26) && attempts < 10 => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => panic!("native execution: {error}"),
        }
    };
    std::fs::remove_file(path).unwrap();
    output
}
#[test]
fn standalone_native_constructs_recursive_values_and_extracts_a_scalar() {
    let p = program(vec![
        code(
            "VALUE_CONSTRUCT",
            &[
                ("type", "Tree"),
                ("variant", "Empty"),
                ("arguments", ""),
                ("key", "data.empty"),
            ],
        ),
        code(
            "VALUE_CONSTRUCT",
            &[
                ("type", "Tree"),
                ("variant", "Node"),
                (
                    "arguments",
                    "left=f:data.empty;right=f:data.empty;value=i:7",
                ),
                ("key", "data.tree"),
            ],
        ),
        code(
            "VALUE_GET",
            &[
                ("type", "Tree"),
                ("variant", "Node"),
                ("field", "value"),
                ("source", "data.tree"),
                ("key", "out.value"),
            ],
        ),
        code("OUTPUT_FIELD", &[("key", "out.value")]),
        code("RETURN_SUCCESS", &[]),
    ]);
    let output = run(&p, &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("7"));
}
#[test]
fn native_tag_test_rejects_an_invalid_unselected_payload() {
    let p = program(vec![
        code(
            "VALUE_TEST",
            &[
                ("type", "Tree"),
                ("variant", "Empty"),
                ("source", "data.tree"),
                ("key", "out.value"),
            ],
        ),
        code("OUTPUT_FIELD", &[("key", "out.value")]),
        code("RETURN_SUCCESS", &[]),
    ]);
    let output = run(
        &p,
        &["data.tree=[\"Node\",[\"Empty\"],[\"Empty\"],\"invalid\"]"],
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-VALUE"),
        "{output:?}"
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn native_constructor_and_get_preserve_every_builtin_leaf() {
    for (kind, input, expected) in [
        ("Int", "+0007", "7"),
        ("Float", "-0", "-0e0"),
        ("Float", "+nan", "nan"),
        ("Bool", "false", "false"),
        ("Text", "hello\n\u{85}", "hello\n\u{85}"),
        ("Bytes", "hex:00ff", "hex:00ff"),
        ("List<Int>", "[1, -0]", "[1,0]"),
        ("List<Float>", "[+1,nan,-0]", "[1e0,nan,-0e0]"),
        ("List<Text>", "[\"a\", \"b\"]", "[\"a\",\"b\"]"),
        ("Json", "{\"x\":1e02,\"x\":2}", "{\"x\":1e02,\"x\":2}"),
    ] {
        let mut p = program(vec![
            code(
                "VALUE_CONSTRUCT",
                &[
                    ("type", "Box"),
                    ("variant", "Box"),
                    ("arguments", "value=f:input.leaf"),
                    ("key", "data.box"),
                ],
            ),
            code(
                "VALUE_GET",
                &[
                    ("type", "Box"),
                    ("variant", "Box"),
                    ("field", "value"),
                    ("source", "data.box"),
                    ("key", "out.value"),
                ],
            ),
            code("OUTPUT_FIELD", &[("key", "out.value")]),
            code("RETURN_SUCCESS", &[]),
        ]);
        p.type_schemas = std::collections::BTreeMap::from([(
            "Box".into(),
            std::collections::BTreeMap::from([(
                "Box".into(),
                std::collections::BTreeMap::from([("value".into(), kind.into())]),
            )]),
        )]);
        let arg = format!("input.leaf={input}");
        let output = run(&p, &[&arg]);
        assert!(output.status.success(), "{kind}: {output:?}");
        assert_eq!(
            output.stdout,
            format!("out.value={expected}\n").as_bytes(),
            "{kind}"
        );
    }
}
fn forwarding(p: &mut AilBytecodeProgram, unused: bool) {
    p.actions.insert(
        "Forward".into(),
        AilBytecodeAction {
            name: "Forward".into(),
            instructions: vec![
                code(
                    "VALUE_FUNCTION",
                    &[
                        ("parameters", "tree:Tree"),
                        ("locals", if unused { "result:Tree" } else { "" }),
                        ("result", "Tree"),
                    ],
                ),
                if unused {
                    code(
                        "VALUE_CONSTRUCT",
                        &[
                            ("type", "Tree"),
                            ("variant", "Empty"),
                            ("arguments", ""),
                            ("key", "local.result"),
                        ],
                    )
                } else {
                    code("LABEL", &[("name", "entry")])
                },
                code(
                    "RETURN_VALUE",
                    &[
                        ("type", "Tree"),
                        (
                            "expression",
                            if unused {
                                "f:local.result"
                            } else {
                                "f:local.tree"
                            },
                        ),
                    ],
                ),
            ],
        },
    );
}
#[test]
fn native_custom_function_returns_survive_aliases_and_validate_unused_parameters() {
    let mut p = program(vec![
        code(
            "CALL_VALUE",
            &[
                ("target", "Forward"),
                ("arguments", "tree=f:input.tree"),
                ("key", "input.tree"),
            ],
        ),
        code(
            "VALUE_GET",
            &[
                ("type", "Tree"),
                ("variant", "Node"),
                ("field", "value"),
                ("source", "input.tree"),
                ("key", "out.value"),
            ],
        ),
        code("OUTPUT_FIELD", &[("key", "out.value")]),
        code("RETURN_SUCCESS", &[]),
    ]);
    forwarding(&mut p, false);
    let output = run(
        &p,
        &["input.tree= [ \"Node\",[\"Empty\"],[\"Empty\"],\"+007\" ]"],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"out.value=7\n");
    forwarding(&mut p, true);
    let output = run(
        &p,
        &["input.tree=[\"Node\",[\"Empty\"],[\"Empty\"],\"bad\"]"],
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-VALUE"),
        "{output:?}"
    );
}
#[test]
fn repeated_wrong_variant_recovery_preserves_alias_and_releases_temporary_storage() {
    let mut steps = vec![code(
        "VALUE_CONSTRUCT",
        &[
            ("type", "Tree"),
            ("variant", "Empty"),
            ("arguments", ""),
            ("key", "data.tree"),
        ],
    )];
    for _ in 0..100 {
        steps.extend([
            code("TRY_BEGIN", &[("errors", "VALUE"), ("key", "data.error")]),
            code(
                "VALUE_GET",
                &[
                    ("type", "Tree"),
                    ("variant", "Node"),
                    ("field", "left"),
                    ("source", "data.tree"),
                    ("key", "data.tree"),
                ],
            ),
            code("RECOVER", &[]),
            code("TRY_END", &[]),
        ]);
    }
    steps.extend([
        code("OUTPUT_FIELD", &[("key", "data.tree")]),
        code("OUTPUT_FIELD", &[("key", "data.error")]),
        code("RETURN_SUCCESS", &[]),
    ]);
    let output = run(&program(steps), &[]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        output.stdout,
        b"data.error=AIL-RUNTIME-VALUE\ndata.tree=[\"Empty\"]\n"
    );
}

#[test]
fn custom_arena_charges_each_owned_copy_and_text_get_uses_separate_arena() {
    let literal = format!("\"{}\"", "a".repeat(1_048_576));
    let mut steps = vec![code(
        "ASSIGN_LITERAL",
        &[("type", "Text"), ("value", &literal), ("key", "data.text")],
    )];
    for _ in 0..63 {
        steps.push(code(
            "VALUE_CONSTRUCT",
            &[
                ("type", "Box"),
                ("variant", "Box"),
                ("arguments", "value=f:data.text"),
                ("key", "data.box"),
            ],
        ));
    }
    steps.extend([
        code(
            "VALUE_TEST",
            &[
                ("type", "Box"),
                ("variant", "Box"),
                ("source", "data.box"),
                ("key", "out.match"),
            ],
        ),
        code(
            "VALUE_GET",
            &[
                ("type", "Box"),
                ("variant", "Box"),
                ("field", "value"),
                ("source", "data.box"),
                ("key", "data.extracted"),
            ],
        ),
        code(
            "TEXT_LENGTH",
            &[("source", "data.extracted"), ("key", "out.length")],
        ),
        code("OUTPUT_FIELD", &[("key", "out.length")]),
        code("OUTPUT_FIELD", &[("key", "out.match")]),
        code("RETURN_SUCCESS", &[]),
    ]);
    let mut p = program(steps);
    p.type_schemas = std::collections::BTreeMap::from([(
        "Box".into(),
        std::collections::BTreeMap::from([(
            "Box".into(),
            std::collections::BTreeMap::from([("value".into(), "Text".into())]),
        )]),
    )]);
    let output = run(&p, &[]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"out.length=1048576\nout.match=true\n");
    let steps = &mut p.actions.get_mut("Run").unwrap().instructions;
    steps.truncate(64);
    steps.extend([
        code("TRY_BEGIN", &[("errors", "any"), ("key", "data.error")]),
        code(
            "VALUE_CONSTRUCT",
            &[
                ("type", "Box"),
                ("variant", "Box"),
                ("arguments", "value=f:data.text"),
                ("key", "data.box"),
            ],
        ),
        code("RECOVER", &[]),
        code("TRY_END", &[]),
        code("OUTPUT_FIELD", &[("key", "data.error")]),
        code("RETURN_SUCCESS", &[]),
    ]);
    let output = run(&p, &[]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-VALUE-LIMIT"),
        "{output:?}"
    );
    assert!(output.stdout.is_empty());
}
#[test]
fn sorted_constructor_validation_precedes_later_integer_division() {
    let p = program(vec![
        code("TRY_BEGIN", &[("errors", "any"), ("key", "out.error")]),
        code(
            "VALUE_CONSTRUCT",
            &[
                ("type", "Tree"),
                ("variant", "Node"),
                (
                    "arguments",
                    "left=f:data.invalid;right=f:data.invalid;value=i:1 i:0 /",
                ),
                ("key", "data.result"),
            ],
        ),
        code("RECOVER", &[]),
        code("TRY_END", &[]),
        code("OUTPUT_FIELD", &[("key", "out.error")]),
        code("RETURN_SUCCESS", &[]),
    ]);
    let output = run(&p, &["data.invalid=broken"]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"out.error=AIL-RUNTIME-VALUE\n");
}

#[test]
fn mutually_recursive_custom_returns_survive_callee_frames() {
    let mut p = program(vec![
        code(
            "CALL_VALUE",
            &[
                ("target", "Left"),
                ("arguments", "n=i:20;tree=f:input.tree"),
                ("key", "out.tree"),
            ],
        ),
        code("OUTPUT_FIELD", &[("key", "out.tree")]),
        code("RETURN_SUCCESS", &[]),
    ]);
    for (name, other) in [("Left", "Right"), ("Right", "Left")] {
        p.actions.insert(
            name.into(),
            AilBytecodeAction {
                name: name.into(),
                instructions: vec![
                    code(
                        "VALUE_FUNCTION",
                        &[
                            ("parameters", "n:Int;tree:Tree"),
                            ("locals", "next:Tree"),
                            ("result", "Tree"),
                        ],
                    ),
                    code(
                        "BRANCH_INT",
                        &[
                            ("left", "f:local.n"),
                            ("right", "i:0"),
                            ("comparison", "=="),
                            ("label", "done"),
                        ],
                    ),
                    code(
                        "CALL_VALUE",
                        &[
                            ("target", other),
                            ("arguments", "n=f:local.n i:1 -;tree=f:local.tree"),
                            ("key", "local.next"),
                        ],
                    ),
                    code(
                        "RETURN_VALUE",
                        &[("type", "Tree"), ("expression", "f:local.next")],
                    ),
                    code("LABEL", &[("name", "done")]),
                    code(
                        "RETURN_VALUE",
                        &[("type", "Tree"), ("expression", "f:local.tree")],
                    ),
                ],
            },
        );
    }
    let output = run(
        &p,
        &["input.tree=[\"Node\",[\"Empty\"],[\"Empty\"],\"+007\"]"],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        output.stdout,
        b"out.tree=[\"Node\",[\"Empty\"],[\"Empty\"],\"7\"]\n"
    );
}
