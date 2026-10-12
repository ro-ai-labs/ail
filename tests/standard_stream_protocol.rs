use ail::ail::*;
use std::collections::BTreeMap;

fn core() -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let source = "The application Streams manages text.\n\nA Data has:\n- text: Text\n\nAction: Run.\nWhen Run happens:\n- the system reads text from standard input into data.text\n- the system writes text data.text to standard output\n- the system writes text data.text to standard error\n- the system records a trace event named Completed\n";
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}

#[test]
fn standard_streams_preserve_order_through_core_source_and_ste() {
    let original = core();
    assert_eq!(
        original
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind == "StandardStreamOperation")
            .count(),
        3
    );
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
    ] {
        assert!(
            check_ail_core(&candidate).is_empty(),
            "{:?}",
            check_ail_core(&candidate)
        );
        let source = render_ail_spec_from_core(&candidate);
        assert!(source.contains("writes text data.text to standard error"));
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let ste = ail::ste::render_ail_ste(&program).unwrap();
        for sentence in [
            "Read text from standard input into field",
            "Write text from field",
            "to standard output.",
            "to standard error.",
        ] {
            assert!(ste.contains(sentence), "missing {sentence}: {ste}");
        }
        assert_eq!(ail::ste::parse_ail_ste(&ste).unwrap(), program);
        let operations = program.actions["Run"]
            .instructions
            .iter()
            .filter(|i| i.opcode.starts_with("STD"))
            .map(|i| i.opcode.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            operations,
            ["STDIN_READ_TEXT", "STDOUT_WRITE_TEXT", "STDERR_WRITE_TEXT"]
        );
        assert!(
            run_ail_bytecode_action(&program, "Run", BTreeMap::new())
                .unwrap_err()
                .contains("AIL-RUNTIME-CAPABILITY")
        );
    }
}

#[test]
fn standard_stream_protocol_rejects_extra_operands_and_profiles() {
    let program = compile_ail_core_bytecode(&core()).unwrap();
    for opcode in ["STDIN_READ_TEXT", "STDOUT_WRITE_TEXT", "STDERR_WRITE_TEXT"] {
        let mut forged = program.clone();
        forged
            .actions
            .get_mut("Run")
            .unwrap()
            .instructions
            .iter_mut()
            .find(|i| i.opcode == opcode)
            .unwrap()
            .operands
            .insert("grant".into(), "true".into());
        assert!(
            verify_ail_bytecode(&forged)
                .iter()
                .any(|e| e.contains("AIL-STDIO-001"))
        );
    }
    let mut forged = program;
    forged.profile = "CompilerPass".into();
    assert!(
        verify_ail_bytecode(&forged)
            .iter()
            .any(|e| e.contains("AIL-STDIO-001"))
    );
}
