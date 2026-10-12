use ail::ail::*;
use std::collections::BTreeMap;

const CATEGORIES: [&str; 14] = [
    "INT",
    "OVERFLOW",
    "DIVZERO",
    "FLOAT",
    "BOOL",
    "TEXT",
    "LIST",
    "INDEX",
    "JSON",
    "JSON-PATH",
    "PROCESS",
    "TIMEOUT",
    "PATH",
    "BYTES",
];
const HEADER: &str = "The application Raises manages data.\nA Data has:\n- value: Int\n- error: Text\n- text: Text\n";
fn package() -> AilPackage {
    load_ail_package_dir("bench/R001-add/program.ail").unwrap()
}
fn core(source: &str) -> AilCore {
    let package = package();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}
fn source(body: &str) -> String {
    format!(
        "{HEADER}Action: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Done\n"
    )
}
fn instruction(opcode: &str, operands: &[(&str, &str)]) -> AilBytecodeInstruction {
    AilBytecodeInstruction {
        opcode: opcode.into(),
        operands: operands
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect(),
    }
}
fn raw(code: Vec<AilBytecodeInstruction>) -> AilBytecodeProgram {
    let mut program =
        compile_ail_core_bytecode(&core(&source("- the system computes data.value as 1"))).unwrap();
    program.actions.get_mut("Run").unwrap().instructions = code;
    program
}

#[test]
fn every_category_roundtrips_source_core_and_ste_and_is_catchable() {
    for category in CATEGORIES {
        let original = core(&source(&format!(
            "- try:\n- the system assigns 7 to data.value\n- the system raises error {category}\n- the system assigns 99 to data.value\n- recover {category} into data.error:\n- end try"
        )));
        for graph in [
            original.clone(),
            parse_ail_core_text(&render_ail_core(&original)).unwrap(),
            core(&render_ail_spec_from_core(&original)),
        ] {
            let program = compile_ail_core_bytecode(&graph).unwrap();
            let ste = ail::ste::render_ail_ste(&program).unwrap();
            assert!(
                ste.lines()
                    .any(|line| line.starts_with("Raise error t") && line.ends_with('.')),
                "{ste}"
            );
            let program = ail::ste::parse_ail_ste(&ste).unwrap();
            let output = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
            assert_eq!(output.final_state["data.value"], "7");
            assert_eq!(
                output.final_state["data.error"],
                format!("AIL-RUNTIME-{category}")
            );
        }
    }
}

#[test]
fn uncaught_raise_and_handler_raise_preserve_category() {
    for category in CATEGORIES {
        let program = raw(vec![
            instruction("RAISE", &[("category", category)]),
            instruction("RETURN_SUCCESS", &[]),
        ]);
        let error = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap_err();
        assert!(
            error.starts_with(&format!("AIL-RUNTIME-{category}:")),
            "{error}"
        );
    }
    let graph = core(&source(
        "- try:\n- try:\n- the system raises error INT\n- recover INT into data.text:\n- the system raises error BYTES\n- end try\n- recover BYTES into data.error:\n- the system assigns 3 to data.value\n- end try",
    ));
    let output = run_ail_bytecode_action(
        &compile_ail_core_bytecode(&graph).unwrap(),
        "Run",
        BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(output.final_state["data.error"], "AIL-RUNTIME-BYTES");
    assert_eq!(output.final_state["data.text"], "AIL-RUNTIME-INT");
}

#[test]
fn protocol_checks_dead_raises_and_exact_operands() {
    for category in [
        "",
        "any",
        "int",
        "INT;TEXT",
        "INT or TEXT",
        "IO",
        "TEXT-LIMIT",
        "DEPTH",
        "FUEL",
        "CAPABILITY",
        "data.error",
        " INT",
        "INT ",
    ] {
        let program = raw(vec![
            instruction("RETURN_SUCCESS", &[]),
            instruction("RAISE", &[("category", category)]),
        ]);
        let diagnostics = verify_ail_bytecode(&program);
        assert!(
            diagnostics.iter().any(|d| d.contains("AILBC019")),
            "{category}: {diagnostics:?}"
        );
    }
    for operands in [vec![], vec![("category", "INT"), ("message", "secret")]] {
        assert!(
            verify_ail_bytecode(&raw(vec![instruction("RAISE", &operands)]))
                .iter()
                .any(|d| d.contains("AILBC019"))
        );
    }
    for profile in ["Compiler", "AgentTool", "System", "UI"] {
        let mut program = raw(vec![instruction("RAISE", &[("category", "INT")])]);
        program.profile = profile.into();
        assert!(
            verify_ail_bytecode(&program)
                .iter()
                .any(|d| d.contains("AILBC019"))
        );
    }
}

#[test]
fn raising_integer_and_value_functions_propagate_without_publishing_results() {
    for (kind, key) in [("Int", "data.value"), ("Text", "data.text")] {
        let program_source = format!(
            "{HEADER}Function: fail.\nThe function needs:\nThe function produces:\n- result: {kind}\nThe function executes:\n- the function raises error BYTES\nAction: Run.\nWhen Run happens:\n- try:\n- the system calls fail with none into {key}\n- recover BYTES into data.error:\n- end try\n- the system records a trace event named Done\n"
        );
        let original = core(&program_source);
        for graph in [
            original.clone(),
            parse_ail_core_text(&render_ail_core(&original)).unwrap(),
            core(&render_ail_spec_from_core(&original)),
        ] {
            let program = compile_ail_core_bytecode(&graph).unwrap();
            let expected = if kind == "Int" { "41" } else { "untouched" };
            let output =
                run_ail_bytecode_action(&program, "Run", [(key.into(), expected.into())].into())
                    .unwrap();
            assert_eq!(output.final_state[key], expected);
            assert_eq!(output.final_state["data.error"], "AIL-RUNTIME-BYTES");
        }
    }
}

#[test]
fn a_raise_does_not_hide_a_reachable_function_fallthrough() {
    for kind in ["Int", "Text"] {
        for body in [
            "- if local.n == 0:\n- the function raises error INT\n- end if",
            "- while local.n == 0:\n- the function raises error INT\n- end while",
            "- the function raises error INT",
        ] {
            let text = format!(
                "{HEADER}Function: fail.\nThe function needs:\n- n: Int\nThe function produces:\n- result: {kind}\nThe function executes:\n{body}\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Done\n"
            );
            let result = compile_ail_core_bytecode(&core(&text));
            assert_eq!(
                result.is_ok(),
                body == "- the function raises error INT",
                "{kind} {body}: {result:?}"
            );
        }
    }
}

#[test]
fn malformed_raise_forms_are_reserved_in_all_source_bodies() {
    for statement in [
        "raises",
        "raises error",
        "raises error any",
        "raises error INT with message hidden",
        "raises error TEXT-LIMIT",
    ] {
        for subject in ["the system", "the function"] {
            for (heading, section) in [
                ("Function: fail.", "When fail runs:"),
                ("Compiler pass: fail.", "When the compiler runs fail:"),
            ] {
                let text = format!("{HEADER}{heading}\n{section}\n- {subject} {statement}\n");
                assert!(
                    parse_ail_package_spec_text(&package(), &text).is_err(),
                    "{text}"
                );
            }
        }
    }
}

#[test]
fn raise_core_rejects_hidden_metadata_duplicate_identity_and_provenance() {
    let original = core(&source("- the system raises error INT"));
    let index = original
        .graph
        .nodes
        .iter()
        .position(|n| {
            n.attributes
                .get("control_kind")
                .is_some_and(|k| k == "raise")
        })
        .expect("raise Core node");
    for mutation in 0..6 {
        let mut graph = original.clone();
        match mutation {
            0 => {
                graph.graph.nodes[index]
                    .attributes
                    .insert("message".into(), "hidden".into());
            }
            1 => {
                graph.graph.nodes[index]
                    .attributes
                    .insert("error_category".into(), "any".into());
            }
            2 => {
                graph.graph.nodes.push(graph.graph.nodes[index].clone());
            }
            3 => {
                let edge = graph
                    .graph
                    .edges
                    .iter()
                    .find(|e| e.target == graph.graph.nodes[index].id)
                    .unwrap()
                    .clone();
                graph.graph.edges.push(edge);
            }
            4 => {
                let id = graph.graph.nodes[index].id.clone();
                let edge = graph
                    .graph
                    .edges
                    .iter_mut()
                    .find(|e| e.source == id && e.kind == "has_provenance")
                    .unwrap();
                edge.attributes.insert("hidden".into(), "true".into());
            }
            5 => {
                let id = graph.graph.nodes[index].id.clone();
                let provenance = graph
                    .graph
                    .edges
                    .iter()
                    .find(|e| e.source == id && e.kind == "has_provenance")
                    .unwrap()
                    .target
                    .clone();
                graph.graph.nodes.push(
                    graph
                        .graph
                        .nodes
                        .iter()
                        .find(|n| n.id == provenance)
                        .unwrap()
                        .clone(),
                );
            }
            _ => unreachable!(),
        }
        assert!(
            compile_ail_core_bytecode(&graph).is_err(),
            "mutation {mutation}"
        );
        if let Ok(saved) = parse_ail_core_text(&render_ail_core(&graph)) {
            assert!(
                compile_ail_core_bytecode(&saved).is_err(),
                "saved mutation {mutation}"
            );
        }
    }
}

#[test]
fn function_raise_core_validates_local_owner_and_all_participant_metadata() {
    for kind in ["Int", "Text"] {
        let text = format!(
            "{HEADER}Function: fail.\nThe function needs:\nThe function produces:\n- result: {kind}\nThe function executes:\n- the function raises error BYTES\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Done\n"
        );
        let original = core(&text);
        let index = original
            .graph
            .nodes
            .iter()
            .position(|n| {
                n.attributes
                    .get("control_kind")
                    .is_some_and(|v| v == "raise")
            })
            .unwrap();
        for mutation in 0..9 {
            let mut graph = original.clone();
            let id = graph.graph.nodes[index].id.clone();
            let owner_id = graph
                .graph
                .edges
                .iter()
                .find(|e| e.target == id)
                .unwrap()
                .source
                .clone();
            match mutation {
                0 => {
                    graph.graph.nodes[index]
                        .attributes
                        .insert("message".into(), "hidden".into());
                }
                1 => {
                    graph.graph.nodes[index].type_name = Some("Text".into());
                }
                2 => {
                    graph.graph.nodes[index]
                        .attributes
                        .insert("step_kind".into(), "trace".into());
                    graph.graph.nodes[index]
                        .attributes
                        .insert("event".into(), "IgnoredRaise".into());
                }
                3 => {
                    graph.graph.nodes.push(graph.graph.nodes[index].clone());
                }
                4 => {
                    let owner = graph
                        .graph
                        .nodes
                        .iter()
                        .find(|n| n.id == owner_id)
                        .unwrap()
                        .clone();
                    graph.graph.nodes.push(owner);
                }
                5 => {
                    let edge = graph
                        .graph
                        .edges
                        .iter()
                        .find(|e| e.target == id)
                        .unwrap()
                        .clone();
                    graph.graph.edges.push(edge);
                }
                6 => {
                    graph
                        .graph
                        .edges
                        .iter_mut()
                        .find(|e| e.target == id)
                        .unwrap()
                        .attributes
                        .insert("operand".into(), "hidden".into());
                }
                7 => {
                    let edge = graph
                        .graph
                        .edges
                        .iter_mut()
                        .find(|e| e.source == owner_id && e.kind == "has_provenance")
                        .unwrap();
                    edge.attributes.insert("hidden".into(), "true".into());
                }
                8 => {
                    let provenance = graph
                        .graph
                        .edges
                        .iter()
                        .find(|e| e.source == owner_id && e.kind == "has_provenance")
                        .unwrap()
                        .target
                        .clone();
                    let node = graph
                        .graph
                        .nodes
                        .iter()
                        .find(|n| n.id == provenance)
                        .unwrap()
                        .clone();
                    graph.graph.nodes.push(node);
                }
                _ => unreachable!(),
            }
            assert!(
                compile_ail_core_bytecode(&graph).is_err(),
                "{kind} mutation {mutation}"
            );
            if let Ok(saved) = parse_ail_core_text(&render_ail_core(&graph)) {
                assert!(
                    compile_ail_core_bytecode(&saved).is_err(),
                    "{kind} saved mutation {mutation}"
                );
            }
        }
    }
}

#[test]
fn raise_subject_must_match_the_executable_body() {
    assert!(
        parse_ail_package_spec_text(&package(), &source("- the function raises error INT"))
            .is_err()
    );
    let text = format!(
        "{HEADER}Function: fail.\nThe function needs:\nThe function produces:\n- result: Int\nThe function executes:\n- the system raises error INT\n"
    );
    assert!(parse_ail_package_spec_text(&package(), &text).is_err());
}

#[test]
fn termination_analysis_follows_matching_handlers_and_loop_exits() {
    for (body, valid) in [
        (
            "- try:\n- the function raises error BYTES\n- recover BYTES into local.error:\n- end try",
            false,
        ),
        (
            "- try:\n- the function raises error BYTES\n- recover INT into local.error:\n- end try",
            true,
        ),
        (
            "- try:\n- the function raises error BYTES\n- recover BYTES into local.error:\n- the function raises error INT\n- end try",
            true,
        ),
        (
            "- while local.n > 0:\n- break\n- end while\n- the function raises error INT",
            true,
        ),
        (
            "- while local.n > 0:\n- if local.n == 1:\n- break\n- end if\n- the function raises error INT\n- end while",
            false,
        ),
        (
            "- if local.n == 0:\n- the function raises error INT\n- otherwise:\n- the function returns 1\n- end if",
            true,
        ),
    ] {
        let text = format!(
            "{HEADER}Function: fail.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function stores:\n- error: Text\nThe function executes:\n{body}\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Done\n"
        );
        assert_eq!(
            compile_ail_core_bytecode(&core(&text)).is_ok(),
            valid,
            "{body}"
        );
    }
}

#[test]
fn imported_function_and_action_raises_keep_categories_and_order() {
    let mut child = package();
    child.metadata.name = "raise-child".into();
    child.spec_text = format!(
        "{HEADER}Function: fail.\nThe function needs:\nThe function produces:\n- result: Int\nThe function executes:\n- the function raises error JSON-PATH\nAction: Run.\nWhen Run happens:\n- the system assigns 11 to data.value\n- try:\n- the system calls fail with none into data.value\n- recover JSON-PATH into data.error:\n- end try\n- the system raises error BYTES\n- the system records a trace event named Done\n"
    );
    let mut parent = package();
    parent.spec_text =
        source("- try:\n- the system calls Child.Run\n- recover BYTES into data.error:\n- end try");
    parent.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Child".into(),
            resolved_package: None,
            registry_identity: None,
        },
        package: Box::new(child),
    });
    let original = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    for graph in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&graph).unwrap()).unwrap();
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(result.final_state["child.data.value"], "11");
        assert_eq!(
            result.final_state["child.data.error"],
            "AIL-RUNTIME-JSON-PATH"
        );
        assert_eq!(result.final_state["data.error"], "AIL-RUNTIME-BYTES");
    }
}
