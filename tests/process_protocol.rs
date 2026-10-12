use ail::ail::*;
use std::collections::BTreeMap;

fn core() -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let source = "The application Tools manages processes.\n\nA Data has:\n- arguments: List<Text>\n- input: Text\n- timeout: Int\n- output: Text\n- errors: Text\n- status: Int\n- failure: Text\n\nAction: Run.\nWhen Run happens:\n- try:\n- the system runs process resource tool with arguments data.arguments and input data.input within data.timeout milliseconds into data.output with errors data.errors and status data.status\n- recover PROCESS or TIMEOUT into data.failure:\n- the system assigns 9 to data.status\n- end try\n- the system records a trace event named Completed\n";
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}

#[test]
fn process_source_core_and_ste_preserve_effect_and_recovery() {
    let original = core();
    assert_eq!(
        original
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind == "ProcessOperation")
            .count(),
        1
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
        assert!(
            render_ail_spec_from_core(&candidate)
                .contains("runs process resource tool with arguments data.arguments")
        );
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        assert!(
            program.actions["Run"]
                .instructions
                .iter()
                .any(|i| i.opcode == "PROCESS_RUN")
        );
        let ste = ail::ste::render_ail_ste(&program).unwrap();
        assert!(ste.contains("Run process "));
        assert_eq!(ail::ste::parse_ail_ste(&ste).unwrap(), program);
        // Authority wins over missing inputs and cannot be caught by the tool handler.
        assert!(
            run_ail_bytecode_action(&program, "Run", BTreeMap::new())
                .unwrap_err()
                .contains("AIL-RUNTIME-CAPABILITY")
        );
    }
}

#[test]
fn process_protocol_rejects_hidden_authority_and_output_aliases() {
    let program = compile_ail_core_bytecode(&core()).unwrap();
    for (key, value) in [("grant", "/bin/true"), ("stderr", "data.output")] {
        let mut forged = program.clone();
        forged
            .actions
            .get_mut("Run")
            .unwrap()
            .instructions
            .iter_mut()
            .find(|i| i.opcode == "PROCESS_RUN")
            .unwrap()
            .operands
            .insert(key.into(), value.into());
        assert!(
            verify_ail_bytecode(&forged)
                .iter()
                .any(|d| d.contains("AIL-PROCESS-001"))
        );
    }
    let mut forged = program;
    forged.profile = "CompilerPass".into();
    assert!(
        verify_ail_bytecode(&forged)
            .iter()
            .any(|d| d.contains("AIL-PROCESS-001"))
    );
}
