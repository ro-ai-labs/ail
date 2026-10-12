use ail::ail::*;
use std::collections::BTreeMap;

fn core(source: &str) -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}

fn source(literal: &str) -> String {
    format!(
        "The application Binary manages bytes.\n\nA Data has:\n- body: Bytes\n- count: Int\n- failure: Text\n\nAction: Run.\nWhen Run happens:\n- the system assigns {literal} to data.body\n- the system counts bytes in data.body into data.count\n- the system records a trace event named Completed\n"
    )
}

#[test]
fn bytes_type_literal_core_and_ste_round_trip() {
    let original = core(&source("\"hex:00ff80\""));
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        assert!(
            check_ail_core(&candidate).is_empty(),
            "{:?}",
            check_ail_core(&candidate)
        );
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let ste = ail::ste::render_ail_ste(&program).unwrap();
        assert!(ste.contains("Count bytes in field "));
        assert_eq!(ail::ste::parse_ail_ste(&ste).unwrap(), program);
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(result.final_state["data.body"], "hex:00ff80");
        assert_eq!(result.final_state["data.count"], "3");
    }
}

#[test]
fn bytes_literals_above_text_limit_survive_all_serializers() {
    let value = format!("hex:{}", "ab".repeat(1_048_576));
    let original = core(&source(&format!("\"{value}\"")));
    let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
    let rendered = core(&render_ail_spec_from_core(&saved));
    let program = compile_ail_core_bytecode(&rendered).unwrap();
    let program = ail::ste::parse_ail_ste(&ail::ste::render_ail_ste(&program).unwrap()).unwrap();
    let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(result.final_state["data.body"], value);
    assert_eq!(result.final_state["data.count"], "1048576");
}

#[test]
fn large_bytes_literals_in_functions_keep_their_type_through_core() {
    let value = format!("hex:{}", "ff".repeat(600_000));
    let source = format!(
        "The application Binary manages bytes.\nA Data has:\n- body: Bytes\n- count: Int\nFunction: create.\nThe function needs:\nThe function produces:\n- result: Bytes\nThe function stores:\n- result: Bytes\nThe function executes:\n- the function assigns \"{value}\" to local.result\n- the function returns local.result\nAction: Run.\nWhen Run happens:\n- the system calls create with none into data.body\n- the system counts bytes in data.body into data.count\n- the system records a trace event named Completed\n"
    );
    let original = core(&source);
    for candidate in [
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let program =
            ail::ste::parse_ail_ste(&ail::ste::render_ail_ste(&program).unwrap()).unwrap();
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(result.final_state["data.body"], value);
        assert_eq!(result.final_state["data.count"], "600000");
    }
}

#[test]
fn invalid_bytes_literals_reject_without_reinterpreting_as_text() {
    for literal in [
        "\"hex:FF\"",
        "\"hex:0\"",
        "\"00ff\"",
        "\"hex:gg\"",
        "\"hex:00 \"",
        "12",
        "null",
        "[]",
        "true",
    ] {
        let candidate = core(&source(literal));
        assert!(
            compile_ail_core_bytecode(&candidate).is_err(),
            "accepted {literal}"
        );
    }
}

#[test]
fn bytes_recovery_is_available_and_bytes_limit_is_fatal() {
    let source = "The application Binary manages bytes.\nA Data has:\n- body: Bytes\n- count: Int\n- failure: Text\nAction: Run.\nWhen Run happens:\n- try:\n- the system counts bytes in data.body into data.count\n- recover BYTES into data.failure:\n- the system assigns 7 to data.count\n- end try\n- the system records a trace event named Completed\n";
    let program = compile_ail_core_bytecode(&core(source)).unwrap();
    let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(result.final_state["data.count"], "7");
    assert_eq!(result.final_state["data.failure"], "AIL-RUNTIME-BYTES");
    let error = run_ail_bytecode_action(
        &program,
        "Run",
        BTreeMap::from([(
            "data.body".into(),
            format!("hex:{}", "00".repeat(1_048_577)),
        )]),
    )
    .unwrap_err();
    assert!(error.starts_with("AIL-RUNTIME-BYTES-LIMIT:"), "{error}");
}

#[test]
fn bytes_core_structural_duplicates_survive_save_and_are_rejected() {
    let original = core(&source("\"hex:00ff80\""));
    for role in ["bytes_step", "bytes_source", "bytes_destination"] {
        let mut duplicate = original.clone();
        let edge = duplicate
            .graph
            .edges
            .iter()
            .find(|e| e.kind == role)
            .unwrap()
            .clone();
        duplicate.graph.edges.push(edge);
        let saved = parse_ail_core_text(&render_ail_core(&duplicate)).unwrap();
        assert_eq!(
            saved.graph.edges.iter().filter(|e| e.kind == role).count(),
            2,
            "{role}"
        );
        assert!(
            compile_ail_core_bytecode(&saved).is_err(),
            "accepted duplicate {role}"
        );
    }
}

#[test]
fn bytes_duplicate_provenance_edges_remain_rejectable_after_core_save() {
    let original = core(&source("\"hex:00ff80\""));
    for kind in ["BytesOperation", "Action", "Field"] {
        let mut duplicate = original.clone();
        let participant = duplicate
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == kind)
            .unwrap();
        let edge = duplicate
            .graph
            .edges
            .iter()
            .find(|e| e.kind == "has_provenance" && e.source == participant.id)
            .unwrap()
            .clone();
        duplicate.graph.edges.push(edge);
        assert!(
            check_ail_core(&duplicate)
                .iter()
                .any(|d| d.contains("AIL-BYTES-001")),
            "direct {kind}"
        );
        let saved = parse_ail_core_text(&render_ail_core(&duplicate)).unwrap();
        assert!(
            check_ail_core(&saved)
                .iter()
                .any(|d| d.contains("AIL-BYTES-001")),
            "saved {kind}"
        );
    }
}

#[test]
fn bytes_function_signatures_and_local_operations_round_trip_without_coercion() {
    let original = core(
        "The application Binary manages bytes.\nA Data has:\n- body: Bytes\nFunction: change.\nThe function needs:\n- body: Bytes\nThe function produces:\n- result: Bytes\nThe function stores:\n- result: Bytes\nThe function executes:\n- the function sets byte -1 in bytes local.body to 255 into local.result\n- the function returns local.result\nAction: Run.\nWhen Run happens:\n- the system assigns \"hex:000180\" to data.body\n- the system calls change with body = data.body into data.body\n- the system records a trace event named Completed\n",
    );
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let program =
            ail::ste::parse_ail_ste(&ail::ste::render_ail_ste(&program).unwrap()).unwrap();
        assert_eq!(
            run_ail_bytecode_action(&program, "Run", BTreeMap::new())
                .unwrap()
                .final_state["data.body"],
            "hex:0001ff"
        );
    }
    for wrong_type in ["Text", "Json", "Bool", "List<Int>"] {
        let mut forged = original.clone();
        forged
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name.eq_ignore_ascii_case("data.body"))
            .unwrap()
            .type_name = Some(wrong_type.into());
        assert!(
            compile_ail_core_bytecode(&forged).is_err(),
            "accepted {wrong_type} as Bytes"
        );
    }
}

#[test]
fn bytes_reserved_forms_reject_in_descriptive_bodies_and_compiler_passes() {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    for operation in [
        "counts bytes",
        "gets byte",
        "slices bytes",
        "joins bytes",
        "sets byte",
        "encodes text data.text as UTF-8 bytes into data.body",
        "decodes bytes",
        "decodes hexadecimal text",
        "encodes bytes",
        "compares bytes",
    ] {
        for subject in ["the system", "the function"] {
            for source in [
                format!("Function: describe.\nWhen describe runs:\n- {subject} {operation}\n"),
                format!(
                    "Compiler pass: Describe.\nWhen the compiler runs Describe:\n- {subject} {operation}\n"
                ),
            ] {
                let result = parse_ail_package_spec_text(&package, &source);
                assert!(result.is_err(), "accepted reserved form {source}");
                assert!(
                    result.unwrap_err().contains("AIL-BYTES-001"),
                    "wrong diagnostic for {source}"
                );
            }
        }
    }
}

#[test]
fn bytes_bytecode_checks_dead_operations_and_profile_and_extra_operands() {
    let original = compile_ail_core_bytecode(&core(&source("\"hex:00ff80\""))).unwrap();
    let mut unreachable = original.clone();
    let mut dead = original.actions["Run"].clone();
    dead.name = "Unused".into();
    dead.instructions
        .iter_mut()
        .find(|i| i.opcode == "BYTES_LENGTH")
        .unwrap()
        .operands
        .insert("source".into(), "data.body + 1".into());
    unreachable.actions.insert("Unused".into(), dead);
    assert!(
        verify_ail_bytecode(&unreachable)
            .iter()
            .any(|d| d.contains("AILBC023"))
    );
    assert!(run_ail_bytecode_action(&unreachable, "Run", BTreeMap::new()).is_err());
    for profile in ["Application", "CompilerPass"] {
        let mut forged = original.clone();
        forged.profile = profile.into();
        let instruction = forged
            .actions
            .get_mut("Run")
            .unwrap()
            .instructions
            .iter_mut()
            .find(|i| i.opcode == "BYTES_LENGTH")
            .unwrap();
        if profile == "Application" {
            instruction
                .operands
                .insert("hidden".into(), "data.body".into());
        }
        assert!(
            verify_ail_bytecode(&forged)
                .iter()
                .any(|d| d.contains("AILBC023"))
        );
        assert!(ail::ste::render_ail_ste(&forged).is_err());
    }
}

#[test]
fn legacy_writes_cannot_cast_bytes_to_or_from_other_declared_types() {
    for (left, right, write) in [
        ("Bytes", "Text", "the data left as the data right"),
        ("Text", "Bytes", "the data left as the data right"),
        ("Bytes", "Json", "the data left as the data right"),
        ("Int", "Bytes", "the data left as the data right"),
        ("Bytes", "Bytes", "the data right to hex:00"),
    ] {
        let source = format!(
            "The application Binary manages bytes.\nA Data has:\n- left: {left}\n- right: {right}\nAction: Run.\nWhen Run happens:\n- the system changes {write}\n- the system records a trace event named Completed\n"
        );
        let original = core(&source);
        for candidate in [
            original.clone(),
            parse_ail_core_text(&render_ail_core(&original)).unwrap(),
            core(&render_ail_spec_from_core(&original)),
        ] {
            assert!(
                check_ail_core(&candidate)
                    .iter()
                    .any(|d| d.contains("AIL-BYTES-001")),
                "accepted {source}"
            );
            assert!(compile_ail_core_bytecode(&candidate).is_err());
            assert!(
                run_ail_action(&ail_document_from_core(&candidate), "Run", BTreeMap::new())
                    .is_err()
            );
        }
    }
    let original = core(
        "The application Binary manages bytes.\nA Data has:\n- left: Bytes\n- right: Bytes\n- count: Int\nAction: Run.\nWhen Run happens:\n- the system assigns \"hex:00ff\" to data.left\n- the system changes the data left as the data right\n- the system counts bytes in data.right into data.count\n- the system records a trace event named Completed\n",
    );
    let program = compile_ail_core_bytecode(&original).unwrap();
    assert_eq!(
        run_ail_bytecode_action(&program, "Run", BTreeMap::new())
            .unwrap()
            .final_state["data.count"],
        "2"
    );
}
