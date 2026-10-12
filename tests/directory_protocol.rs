use ail::ail::*;
use std::collections::BTreeMap;

fn core() -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    let source = "The application Workspace manages reports.\n\nA Data has:\n- path: Text\n- body: Text\n- names: List<Text>\n- failure: Text\n\nAction: Run.\nWhen Run happens:\n- try:\n- the system lists names in directory resource workspace at data.path into data.names\n- the system reads text from directory resource workspace at data.path into data.body\n- the system writes text data.body to directory resource reports at data.path\n- recover PATH into data.failure:\n- the system assigns \"unavailable\" to data.body\n- end try\n- the system records a trace event named Completed\n";
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}

#[test]
fn directory_source_core_and_ste_preserve_all_effects_and_path_recovery() {
    let original = core();
    assert_eq!(
        original
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind == "DirectoryOperation")
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
        let rendered = render_ail_spec_from_core(&candidate);
        assert!(rendered.contains("lists names in directory resource workspace at data.path"));
        assert!(
            rendered.contains("writes text data.body to directory resource reports at data.path")
        );
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let operations: Vec<_> = program.actions["Run"]
            .instructions
            .iter()
            .filter(|i| i.opcode.starts_with("DIRECTORY_"))
            .map(|i| i.opcode.as_str())
            .collect();
        assert_eq!(
            operations,
            [
                "DIRECTORY_LIST_NAMES",
                "DIRECTORY_READ_TEXT",
                "DIRECTORY_WRITE_TEXT"
            ]
        );
        let ste = ail::ste::render_ail_ste(&program).unwrap();
        assert!(ste.contains("List names in directory resource "));
        assert_eq!(ail::ste::parse_ail_ste(&ste).unwrap(), program);
        // Every previous API defaults to no directory authority.
        let mut streams = AilStandardStreams::default();
        for error in [
            run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap_err(),
            run_ail_bytecode_action_with_resources(
                &program,
                "Run",
                BTreeMap::new(),
                &AilFileResources::default(),
                &mut streams,
                &AilProcessResources::default(),
            )
            .unwrap_err(),
        ] {
            assert!(error.starts_with("AIL-RUNTIME-CAPABILITY:"), "{error}");
        }
    }
}

#[test]
fn directory_bytecode_rejects_embedded_authority_and_nonfield_paths() {
    let program = compile_ail_core_bytecode(&core()).unwrap();
    for (key, value) in [("root", "/tmp"), ("path", "data.path + 1")] {
        let mut forged = program.clone();
        forged
            .actions
            .get_mut("Run")
            .unwrap()
            .instructions
            .iter_mut()
            .find(|i| i.opcode == "DIRECTORY_READ_TEXT")
            .unwrap()
            .operands
            .insert(key.into(), value.into());
        assert!(
            verify_ail_bytecode(&forged)
                .iter()
                .any(|d| d.contains("AIL-DIRECTORY-001"))
        );
    }
    let mut forged = program;
    forged.profile = "CompilerPass".into();
    assert!(
        verify_ail_bytecode(&forged)
            .iter()
            .any(|d| d.contains("AIL-DIRECTORY-001"))
    );
}

#[test]
fn malformed_directory_attempts_cannot_become_descriptive_function_or_pass_steps() {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    for operation in [
        "lists names in directory",
        "reads text from directory",
        "writes text to directory",
        "writes text to directory resource tree at data.path",
    ] {
        for subject in ["the system", "the function"] {
            for source in [
                format!(
                    "The application Workspace manages files.\nFunction: describe.\nWhen describe runs:\n- {subject} {operation}\n"
                ),
                format!(
                    "Compiler pass: Describe.\nWhen the compiler runs Describe:\n- {subject} {operation}\n"
                ),
            ] {
                let result = parse_ail_package_spec_text(&package, &source);
                assert!(result.is_err(), "accepted reserved attempt: {source}");
                assert!(
                    result.unwrap_err().contains("AIL-DIRECTORY-001"),
                    "wrong diagnostic: {source}"
                );
            }
        }
    }
}
