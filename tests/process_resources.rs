//! Independent public-surface checks. The larger real-child oracle lives in
//! scripts/ail_process_differential.py and supplies its own external fixtures.
use ail::ail::*;
use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::PathBuf;

const SOURCE: &str = "The application Tools manages processes.\n\nA Data has:\n- arguments: List<Text>\n- input: Text\n- timeout: Int\n- output: Text\n- errors: Text\n- status: Int\n- failure: Text\n\nAction: Run.\nWhen Run happens:\n- the system runs process resource tool with arguments data.arguments and input data.input within data.timeout milliseconds into data.output with errors data.errors and status data.status\n- the system writes text data.output to standard output\n- the system writes text data.errors to standard error\n- the system records a trace event named Completed\n";

fn core(source: &str) -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}

#[test]
fn process_effect_survives_all_public_representations_and_native_lowering() {
    let original = core(SOURCE);
    let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
    let rendered = core(&render_ail_spec_from_core(&saved));
    for candidate in [original, saved, rendered] {
        assert!(
            check_ail_core(&candidate).is_empty(),
            "{:?}",
            check_ail_core(&candidate)
        );
        let direct = compile_ail_core_bytecode(&candidate).unwrap();
        let ste = ail::ste::parse_ail_ste(&ail::ste::render_ail_ste(&direct).unwrap()).unwrap();
        for program in [direct, ste] {
            assert_eq!(
                program.actions["Run"]
                    .instructions
                    .iter()
                    .filter(|i| i.opcode == "PROCESS_RUN")
                    .count(),
                1
            );
            let bytes =
                compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").unwrap();
            assert_eq!(&bytes[..4], b"\x7fELF");
        }
    }
}

fn rejected(candidate: &AilCore, context: &str) {
    assert!(
        compile_ail_core_bytecode(candidate).is_err(),
        "accepted {context}"
    );
    if let Ok(saved) = parse_ail_core_text(&render_ail_core(candidate)) {
        assert!(
            compile_ail_core_bytecode(&saved).is_err(),
            "saved Core accepted {context}"
        );
    }
}

#[test]
fn every_process_role_rejects_duplicate_metadata_missing_and_foreign_ownership() {
    let original = core(SOURCE);
    for role in [
        "process_step",
        "process_arguments",
        "process_input",
        "process_timeout",
        "process_stdout",
        "process_stderr",
        "process_status",
    ] {
        for mutation in ["duplicate", "metadata", "missing", "reverse"] {
            let mut changed = original.clone();
            let index = changed
                .graph
                .edges
                .iter()
                .position(|e| e.kind == role)
                .unwrap();
            match mutation {
                "duplicate" => {
                    let mut copy = changed.graph.edges[index].clone();
                    copy.id = format!("duplicate-{role}");
                    copy.ordinal = changed.graph.edges.len();
                    changed.graph.edges.push(copy);
                }
                "metadata" => {
                    changed.graph.edges[index]
                        .attributes
                        .insert("hidden".into(), "ignored".into());
                }
                "missing" => {
                    changed.graph.edges.remove(index);
                }
                _ => {
                    let edge = &mut changed.graph.edges[index];
                    std::mem::swap(&mut edge.source, &mut edge.target);
                }
            }
            rejected(&changed, &format!("{role}: {mutation}"));
        }
    }
    for role in [
        "process_arguments",
        "process_input",
        "process_timeout",
        "process_stdout",
        "process_stderr",
        "process_status",
    ] {
        for mutation in ["type", "local", "orphan"] {
            let mut changed = original.clone();
            let id = changed
                .graph
                .edges
                .iter()
                .find(|e| e.kind == role)
                .unwrap()
                .target
                .clone();
            let field = changed.graph.nodes.iter_mut().find(|n| n.id == id).unwrap();
            match mutation {
                "type" => {
                    field.type_name = Some("Bool".into());
                }
                "local" => {
                    field
                        .attributes
                        .insert("function_owner".into(), "foreign".into());
                }
                _ => {
                    changed
                        .graph
                        .edges
                        .retain(|e| e.kind != "has_field" || e.target != id);
                }
            }
            rejected(&changed, &format!("{role}: {mutation}"));
        }
    }
}

#[test]
fn process_semantic_reads_and_writes_enforce_secrets_without_descriptive_edges() {
    for (role, diagnostic) in [
        ("process_arguments", "AIL005"),
        ("process_input", "AIL005"),
        ("process_timeout", "AIL005"),
        ("process_stdout", "AIL002"),
        ("process_stderr", "AIL002"),
        ("process_status", "AIL002"),
    ] {
        let mut changed = core(SOURCE);
        let id = changed
            .graph
            .edges
            .iter()
            .find(|e| e.kind == role)
            .unwrap()
            .target
            .clone();
        changed
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.id == id)
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        changed
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        let errors = check_ail_core(&changed);
        assert!(
            errors.iter().any(|e| e.contains(diagnostic)),
            "{role}: {errors:?}"
        );
        rejected(&changed, role);
    }
}

#[test]
fn unsupported_profiles_function_bodies_and_unreachable_bad_protocol_are_rejected() {
    let original = core(SOURCE);
    for profile in ["Compiler", "CompilerPass", "AgentTool", "UI", "System"] {
        let mut changed = original.clone();
        changed.package.profile = profile.into();
        rejected(&changed, profile);
    }
    for result_type in ["Int", "Text"] {
        let valid = format!(
            "The application Tools manages tools.\nFunction: identity.\nThe function needs:\n- value: {result_type}\nThe function produces:\n- result: {result_type}\nThe function executes:\n- the function returns local.value\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Done\n"
        );
        assert!(compile_ail_core_bytecode(&core(&valid)).is_ok());
        let source = valid.replace(
            "- the function returns local.value",
            "- the function runs process:\n- the function returns local.value",
        );
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        if let Ok(document) = parse_ail_package_spec_text(&package, &source) {
            rejected(&elaborate_ail_core(&package, &document), result_type);
        }
    }
    let good = compile_ail_core_bytecode(&original).unwrap();
    for mutation in [
        "extra",
        "missing",
        "status_alias",
        "output_alias",
        "resource",
        "expression",
    ] {
        let mut changed = good.clone();
        let mut hidden = changed.actions["Run"].clone();
        hidden.name = "Unreachable".into();
        let instruction = hidden
            .instructions
            .iter_mut()
            .find(|i| i.opcode == "PROCESS_RUN")
            .unwrap();
        match mutation {
            "extra" => {
                instruction
                    .operands
                    .insert("executable".into(), "/bin/true".into());
            }
            "missing" => {
                instruction.operands.remove("timeout_ms");
            }
            "status_alias" => {
                instruction
                    .operands
                    .insert("status".into(), "data.errors".into());
            }
            "output_alias" => {
                instruction
                    .operands
                    .insert("stderr".into(), "data.output".into());
            }
            "resource" => {
                instruction
                    .operands
                    .insert("resource".into(), "bad..name".into());
            }
            _ => {
                instruction
                    .operands
                    .insert("timeout_ms".into(), "data.timeout + 1".into());
            }
        }
        changed.actions.insert("Unreachable".into(), hidden);
        assert!(!verify_ail_bytecode(&changed).is_empty(), "{mutation}");
        assert!(compile_ail_bytecode_native_elf(&changed, "Run", "linux-x86_64-elf").is_err());
        assert!(
            run_ail_bytecode_action(&changed, "Run", BTreeMap::new())
                .unwrap_err()
                .contains("invalid AIL bytecode")
        );
    }
}

#[test]
fn imported_actions_namespace_process_authority_and_all_six_fields() {
    let mut imported = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    imported.metadata.name = "tool-child".into();
    imported.spec_text = SOURCE.into();
    let mut parent = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    parent.spec_text = "The application Parent manages tools.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system records a trace event named Done\n".into();
    parent.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Child".into(),
            resolved_package: None,
            registry_identity: None,
        },
        package: Box::new(imported),
    });
    let candidate = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    let program =
        compile_ail_core_bytecode(&parse_ail_core_text(&render_ail_core(&candidate)).unwrap())
            .unwrap();
    let operation = program
        .actions
        .values()
        .flat_map(|a| &a.instructions)
        .find(|i| i.opcode == "PROCESS_RUN")
        .unwrap();
    assert_eq!(operation.operands["resource"], "child.tool");
    for name in [
        "arguments",
        "input",
        "timeout_ms",
        "stdout",
        "stderr",
        "status",
    ] {
        assert!(
            operation.operands[name].starts_with("child.data."),
            "{name}"
        );
    }
    let grants = AilProcessResources {
        executables: BTreeMap::from([("tool".into(), PathBuf::from("/bin/true"))]),
    };
    let error = run_ail_bytecode_action_with_resources(
        &program,
        "Run",
        BTreeMap::new(),
        &AilFileResources::default(),
        &mut AilStandardStreams::default(),
        &grants,
    )
    .unwrap_err();
    assert!(error.starts_with("AIL-RUNTIME-CAPABILITY:"), "{error}");
}

#[test]
fn invalid_unused_api_grants_are_rejected_before_prior_stream_effects() {
    use std::os::unix::ffi::OsStringExt;
    let candidate = core(&SOURCE.replace(
        "- the system runs process",
        "- the system writes text data.input to standard output\n- the system runs process",
    ));
    let program = compile_ail_core_bytecode(&candidate).unwrap();
    for (name, path) in [
        ("unused", PathBuf::from("relative")),
        ("bad-name", PathBuf::from("/bin/true")),
        (
            "unused",
            PathBuf::from(std::ffi::OsString::from_vec(b"/bin/true\0suffix".to_vec())),
        ),
        ("unused", PathBuf::from(format!("/{}", "x".repeat(4095)))),
    ] {
        let grants = AilProcessResources {
            executables: BTreeMap::from([
                ("tool".into(), PathBuf::from("/bin/true")),
                (name.into(), path),
            ]),
        };
        let mut input = Cursor::new(Vec::<u8>::new());
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let mut streams = AilStandardStreams {
            stdin: Some(&mut input),
            stdout: Some(&mut output),
            stderr: Some(&mut errors),
        };
        let result = run_ail_bytecode_action_with_resources(
            &program,
            "Run",
            BTreeMap::from([("data.input".into(), "must not write".into())]),
            &AilFileResources::default(),
            &mut streams,
            &grants,
        );
        assert!(result.unwrap_err().starts_with("AIL-RUNTIME-CAPABILITY:"));
        assert!(output.is_empty() && errors.is_empty());
    }
}
