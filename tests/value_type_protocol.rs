use ail::ail::*;
use std::collections::BTreeMap;
fn core(text: &str) -> AilCore {
    let p = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    elaborate_ail_core(&p, &parse_ail_package_spec_text(&p, text).unwrap())
}
fn fixture() -> AilCore {
    core(
        "The application Variants manages trees.\nType: Tree<T>.\nTree has variants:\n- Empty\n- Node(left: Tree<T>, value: T)\nA Data has:\n- empty: Tree<Int>\n- tree: Tree<Int>\n- flag: Bool\n- value: Int\nAction: Run.\nWhen Run happens:\n- the system constructs Tree<Int> variant Empty with none into data.empty\n- the system constructs Tree<Int> variant Node with left = data.empty; value = 7 into data.tree\n- the system records a trace event named Done\n",
    )
}
#[test]
fn closed_schema_roundtrips_and_rejects_unknown_even_unused_fields() {
    let p = compile_ail_core_bytecode(&fixture()).unwrap();
    assert_eq!(p.type_schemas["Tree<Int>"]["Node"]["left"], "Tree<Int>");
    let encoded = render_ail_bytecode(&p);
    assert_eq!(parse_ail_bytecode(&encoded).unwrap(), p);
    assert_eq!(
        ail::ste::parse_ail_ste(&ail::ste::render_ail_ste(&p).unwrap()).unwrap(),
        p
    );
    let mut bad = p.clone();
    bad.type_schemas
        .get_mut("Tree<Int>")
        .unwrap()
        .get_mut("Node")
        .unwrap()
        .insert("value".into(), "Unknown".into());
    assert!(
        verify_ail_bytecode(&bad)
            .iter()
            .any(|e| e.contains("AILBC024"))
    );
    assert!(ail::ste::render_ail_ste(&bad).is_err());
}
#[test]
fn same_shaped_nominal_types_cannot_cross_source_boundaries() {
    let text = "The application Variants manages values.\nType: A.\nA has variants:\n- One(value: Int)\nType: B.\nB has variants:\n- One(value: Int)\nA Data has:\n- a: A\n- b: B\n- flag: Bool\nAction: Run.\nWhen Run happens:\n- the system tests A variant One of data.b into data.flag\n- the system records a trace event named Done\n";
    let c = core(text);
    assert!(
        compile_ail_core_bytecode(&c)
            .unwrap_err()
            .contains("AIL-VALUE-001")
    );
}
#[test]
fn nested_generic_arguments_and_mutual_recursion_resolve() {
    let c = core(
        "The application Variants manages values.\nType: Pair<A, B>.\nPair has variants:\n- Both(first: A, second: B)\nType: Chain<T>.\nChain has variants:\n- End\n- Link(pair: Pair<T, Chain<T>>)\nA Data has:\n- end: Chain<Int>\n- pair: Pair<Int, Chain<Int>>\n- chain: Chain<Int>\nAction: Run.\nWhen Run happens:\n- the system constructs Chain<Int> variant End with none into data.end\n- the system constructs Pair<Int, Chain<Int>> variant Both with first = 3; second = data.end into data.pair\n- the system constructs Chain<Int> variant Link with pair = data.pair into data.chain\n- the system records a trace event named Done\n",
    );
    let p = compile_ail_core_bytecode(&c).unwrap();
    assert_eq!(p.type_schemas.len(), 2);
    let r = run_ail_bytecode_action(&p, "Run", BTreeMap::new()).unwrap();
    assert_eq!(
        r.final_state["data.chain"],
        "[\"Link\",[\"Both\",\"3\",[\"End\"]]]"
    );
}
#[test]
fn corrupt_core_arguments_and_declaration_owners_reject() {
    let c = fixture();
    for kind in ["variant_argument", "contains", "has_field"] {
        let mut bad = c.clone();
        let edge = bad
            .graph
            .edges
            .iter()
            .find(|e| {
                e.kind == kind
                    && if kind == "contains" {
                        bad.graph
                            .nodes
                            .iter()
                            .any(|n| n.id == e.source && n.kind == "Type")
                    } else if kind == "has_field" {
                        bad.graph
                            .nodes
                            .iter()
                            .any(|n| n.id == e.source && n.kind == "Variant")
                    } else {
                        true
                    }
            })
            .unwrap()
            .clone();
        bad.graph.edges.push(edge);
        assert!(compile_ail_core_bytecode(&bad).is_err(), "{kind}");
        let saved = parse_ail_core_text(&render_ail_core(&bad)).unwrap();
        assert!(compile_ail_core_bytecode(&saved).is_err(), "saved {kind}");
    }
}
#[test]
fn unreachable_operations_and_orphan_operands_are_checked() {
    let mut p = compile_ail_core_bytecode(&fixture()).unwrap();
    p.actions
        .get_mut("Run")
        .unwrap()
        .instructions
        .push(AilBytecodeInstruction {
            opcode: "VALUE_GET".into(),
            operands: BTreeMap::from([
                ("type".into(), "Tree<Int>".into()),
                ("variant".into(), "Absent".into()),
                ("field".into(), "value".into()),
                ("source".into(), "data.tree".into()),
                ("key".into(), "data.value".into()),
            ]),
        });
    assert!(
        verify_ail_bytecode(&p)
            .iter()
            .any(|e| e.contains("AILBC024"))
    );
    let mut c = fixture();
    let mut edge = c
        .graph
        .edges
        .iter()
        .find(|e| e.kind == "variant_argument")
        .unwrap()
        .clone();
    edge.source = c
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Action")
        .unwrap()
        .id
        .clone();
    c.graph.edges.push(edge);
    assert!(compile_ail_core_bytecode(&c).is_err());
}
#[test]
fn metadata_requires_closed_type_arguments_and_respects_builtin_precedence() {
    let mut p = compile_ail_core_bytecode(&fixture()).unwrap();
    p.type_schemas.insert(
        "Int".into(),
        BTreeMap::from([("Fake".into(), BTreeMap::new())]),
    );
    assert!(
        verify_ail_bytecode(&p)
            .iter()
            .any(|e| e.contains("AILBC024"))
    );
}
#[test]
fn imported_types_function_locals_and_variant_operands_are_qualified() {
    let root = std::env::temp_dir().join(format!("ail-adt-import-{}", std::process::id()));
    std::fs::create_dir_all(root.join("library")).unwrap();
    std::fs::create_dir_all(root.join("caller")).unwrap();
    std::fs::write(root.join("library/ail-package.md"),"# AIL Package\nname: library\nversion: 0.1.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: types, functions\nconformance: first-slice\n").unwrap();
    std::fs::write(root.join("library/spec.ail-spec.md"),"The application Library manages values.\nType: T.\nT has variants:\n- Zero\nType: Box<T>.\nBox has variants:\n- Wrap(value: T)\nFunction: box.\nThe function needs:\n- value: Int\nThe function produces:\n- result: Box<Int>\nThe function stores:\n- result: Box<Int>\nThe function executes:\n- the function constructs Box<Int> variant Wrap with value = local.value into local.result\n- the function returns local.result\n").unwrap();
    std::fs::write(root.join("caller/ail-package.md"),"# AIL Package\nname: caller\nversion: 0.1.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: types, functions, actions\nimports: ../library compatible ^0.1 as Lib\nconformance: first-slice\n").unwrap();
    std::fs::write(root.join("caller/spec.ail-spec.md"),"The application Caller manages values.\nA Data has:\n- box: Lib.Box<Int>\n- value: Int\nAction: Run.\nWhen Run happens:\n- the system calls Lib.box with value = 9 into data.box\n- the system gets field value of Lib.Box<Int> variant Wrap from data.box into data.value\n- the system records a trace event named Done\n").unwrap();
    let package = load_ail_package_dir(root.join("caller")).unwrap();
    let doc = parse_ail_package_document(&package).unwrap();
    let c = elaborate_ail_core(&package, &doc);
    let p = compile_ail_core_bytecode(&c).unwrap();
    assert!(p.type_schemas.contains_key("Lib.Box<Int>"));
    assert_eq!(
        run_ail_bytecode_action(&p, "Run", BTreeMap::new())
            .unwrap()
            .final_state["data.value"],
        "9"
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn malformed_bytecode_schema_reports_protocol_diagnostic() {
    let p = compile_ail_core_bytecode(&fixture()).unwrap();
    let text = render_ail_bytecode(&p).replace("\"value\":\"Int\"", "\"value\":\"Unknown\"");
    assert!(parse_ail_bytecode(&text).unwrap_err().contains("AILBC024"));
}
#[test]
fn constructor_expression_provenance_is_checked_by_variant_protocol() {
    let mut c = fixture();
    let expression = c
        .graph
        .edges
        .iter()
        .find(|e| {
            e.kind == "variant_argument" && e.attributes.get("name").is_some_and(|n| n == "value")
        })
        .unwrap()
        .target
        .clone();
    c.graph
        .edges
        .retain(|e| !(e.source == expression && e.kind == "has_provenance"));
    assert!(
        compile_ail_core_bytecode(&c)
            .unwrap_err()
            .contains("AIL-VALUE-001")
    );
}

#[test]
fn descriptive_head_ambiguity_is_checked_only_when_reachable() {
    let declarations = "The application Variants manages values.\nType: Box<T>.\nBox has variants:\n- One\nType: Box<X,Y>.\nBox has variants:\n- Two\n";
    let unused = format!(
        "{declarations}Action: Run.\nWhen Run happens:\n- the system records a trace event named Done\n"
    );
    assert!(compile_ail_core_bytecode(&core(&unused)).is_ok());
    let reached = format!(
        "{declarations}A Data has:\n- box: Box<Int>\nAction: Run.\nWhen Run happens:\n- the system constructs Box<Int> variant One with none into data.box\n- the system records a trace event named Done\n"
    );
    assert!(
        compile_ail_core_bytecode(&core(&reached))
            .unwrap_err()
            .contains("AIL-VALUE-001")
    );
}

#[test]
fn bytecode_action_fields_preserve_custom_nominal_types_across_uses_and_calls() {
    fn code(opcode: &str, operands: &[(&str, &str)]) -> AilBytecodeInstruction {
        AilBytecodeInstruction {
            opcode: opcode.into(),
            operands: operands
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
        }
    }
    let mut p = compile_ail_core_bytecode(&core("The application Variants manages values.\nType: A.\nA has variants:\n- Empty\nA Data has:\n- value: A\nAction: Run.\nWhen Run happens:\n- the system constructs A variant Empty with none into data.value\n- the system records a trace event named Done\n")).unwrap();
    p.type_schemas.insert(
        "B".into(),
        BTreeMap::from([
            ("Empty".into(), BTreeMap::new()),
            (
                "Wrap".into(),
                BTreeMap::from([("value".into(), "B".into())]),
            ),
        ]),
    );
    p.actions.insert(
        "forward".into(),
        AilBytecodeAction {
            name: "forward".into(),
            instructions: vec![
                code(
                    "VALUE_FUNCTION",
                    &[("parameters", "value:B"), ("locals", ""), ("result", "B")],
                ),
                code(
                    "RETURN_VALUE",
                    &[("type", "B"), ("expression", "f:local.value")],
                ),
            ],
        },
    );
    for op in [
        code(
            "VALUE_TEST",
            &[
                ("type", "B"),
                ("variant", "Empty"),
                ("source", "data.value"),
                ("key", "data.flag"),
            ],
        ),
        code(
            "VALUE_GET",
            &[
                ("type", "B"),
                ("variant", "Wrap"),
                ("field", "value"),
                ("source", "data.value"),
                ("key", "data.other"),
            ],
        ),
        code(
            "VALUE_CONSTRUCT",
            &[
                ("type", "B"),
                ("variant", "Wrap"),
                ("arguments", "value=f:data.value"),
                ("key", "data.other"),
            ],
        ),
        code(
            "CALL_VALUE",
            &[
                ("target", "forward"),
                ("arguments", "value=f:data.value"),
                ("key", "data.other"),
            ],
        ),
        code(
            "CALL_VALUE",
            &[
                ("target", "forward"),
                ("arguments", "value=f:data.other"),
                ("key", "data.value"),
            ],
        ),
        code(
            "ASSIGN_LITERAL",
            &[("type", "Text"), ("value", "\"x\""), ("key", "data.value")],
        ),
        code(
            "TEXT_LENGTH",
            &[("source", "data.value"), ("key", "data.value")],
        ),
    ] {
        for separate_action in [false, true] {
            let mut bad = p.clone();
            if separate_action {
                bad.actions.insert(
                    "Other".into(),
                    AilBytecodeAction {
                        name: "Other".into(),
                        instructions: vec![op.clone()],
                    },
                );
            } else {
                bad.actions
                    .get_mut("Run")
                    .unwrap()
                    .instructions
                    .push(op.clone());
            }
            assert!(
                verify_ail_bytecode(&bad)
                    .iter()
                    .any(|e| e.contains("AILBC024")),
                "{} separate={separate_action}: {:?}",
                op.opcode,
                verify_ail_bytecode(&bad)
            );
            assert!(run_ail_bytecode_action(&bad, "Run", BTreeMap::new()).is_err());
            assert!(compile_ail_bytecode_native_elf(&bad, "Run", "linux-x86_64-elf").is_err());
        }
    }
    // Raw external bytes have no nominal identity; one expected schema remains valid.
    p.actions
        .get_mut("Run")
        .unwrap()
        .instructions
        .retain(|i| i.opcode != "VALUE_CONSTRUCT");
    p.actions.get_mut("Run").unwrap().instructions.insert(
        0,
        code(
            "VALUE_TEST",
            &[
                ("type", "B"),
                ("variant", "Empty"),
                ("source", "data.value"),
                ("key", "data.flag"),
            ],
        ),
    );
    assert!(verify_ail_bytecode(&p).is_empty());
    assert_eq!(
        run_ail_bytecode_action(
            &p,
            "Run",
            BTreeMap::from([("data.value".into(), "[\"Empty\"]".into())])
        )
        .unwrap()
        .final_state["data.flag"],
        "true"
    );
    let instructions = &mut p.actions.get_mut("Run").unwrap().instructions;
    instructions.push(code(
        "ASSIGN_LITERAL",
        &[("type", "Int"), ("value", "1"), ("key", "data.dynamic")],
    ));
    instructions.push(code(
        "ASSIGN_LITERAL",
        &[
            ("type", "Text"),
            ("value", "\"x\""),
            ("key", "data.dynamic"),
        ],
    ));
    instructions.push(code(
        "TEXT_LENGTH",
        &[("source", "data.dynamic"), ("key", "data.dynamic")],
    ));
    assert!(verify_ail_bytecode(&p).is_empty());
    for op in [
        code(
            "VALUE_TEST",
            &[
                ("type", "B"),
                ("variant", "Empty"),
                ("source", "data.value"),
                ("key", "data.value"),
            ],
        ),
        code(
            "VALUE_GET",
            &[
                ("type", "A"),
                ("variant", "Number"),
                ("field", "value"),
                ("source", "data.value"),
                ("key", "data.value"),
            ],
        ),
    ] {
        let mut bad = p.clone();
        bad.type_schemas.get_mut("A").unwrap().insert(
            "Number".into(),
            BTreeMap::from([("value".into(), "Int".into())]),
        );
        bad.actions.get_mut("Run").unwrap().instructions = vec![op];
        assert!(
            verify_ail_bytecode(&bad)
                .iter()
                .any(|e| e.contains("AILBC024"))
        );
    }
    p.actions.get_mut("Run").unwrap().instructions = vec![code(
        "VALUE_GET",
        &[
            ("type", "B"),
            ("variant", "Wrap"),
            ("field", "value"),
            ("source", "data.value"),
            ("key", "data.value"),
        ],
    )];
    assert!(verify_ail_bytecode(&p).is_empty());
    assert_eq!(
        run_ail_bytecode_action(
            &p,
            "Run",
            BTreeMap::from([("data.value".into(), "[\"Wrap\",[\"Empty\"]]".into())])
        )
        .unwrap()
        .final_state["data.value"],
        "[\"Empty\"]"
    );
}

#[test]
fn builtin_function_results_still_validate_unused_custom_local_schemas() {
    let c = core(
        "The application Variants manages values.\nType: Box.\nBox has variants:\n- Empty\n- Wrap(value: Unknown)\nFunction: number.\nThe function produces:\n- result: Int\nThe function stores:\n- unused: Box\nThe function executes:\n- the function returns 0\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Done\n",
    );
    for candidate in [
        c.clone(),
        parse_ail_core_text(&render_ail_core(&c)).unwrap(),
    ] {
        assert!(
            check_ail_core(&candidate)
                .iter()
                .any(|d| d.contains("AIL-VALUE-001"))
        );
        assert!(compile_ail_core_bytecode(&candidate).is_err());
    }
}
