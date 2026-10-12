//! Independent public-surface checks; filesystem oracle lives in scripts/ail_directory_differential.py.
use ail::ail::*;

const SOURCE: &str = "The application Workspace manages files.\n\nA Data has:\n- path: Text\n- body: Text\n- names: List<Text>\n- failure: Text\n\nAction: Run.\nWhen Run happens:\n- the system lists names in directory resource workspace at data.path into data.names\n- the system reads text from directory resource workspace at data.path into data.body\n- the system writes text data.body to directory resource reports at data.path\n- the system records a trace event named Done\n";

fn core(source: &str) -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}

#[test]
fn directory_effects_survive_every_public_representation_and_native_lowering() {
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
            let native =
                compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").unwrap();
            assert_eq!(&native[..4], b"\x7fELF");
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
fn directory_graph_roles_require_exact_metadata_and_genuine_global_fields() {
    let original = core(SOURCE);
    for role in [
        "directory_step",
        "directory_path",
        "directory_destination",
        "directory_source",
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
                    let mut edge = changed.graph.edges[index].clone();
                    edge.id = format!("extra-{role}");
                    edge.ordinal = changed.graph.edges.len();
                    changed.graph.edges.push(edge);
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
            rejected(&changed, &format!("{role}/{mutation}"));
        }
    }
    for role in [
        "directory_path",
        "directory_destination",
        "directory_source",
    ] {
        for mutation in ["type", "local", "orphan", "competing"] {
            let mut changed = original.clone();
            let id = changed
                .graph
                .edges
                .iter()
                .find(|e| e.kind == role)
                .unwrap()
                .target
                .clone();
            let index = changed.graph.nodes.iter().position(|n| n.id == id).unwrap();
            match mutation {
                "type" => changed.graph.nodes[index].type_name = Some("Bool".into()),
                "local" => {
                    changed.graph.nodes[index]
                        .attributes
                        .insert("function_owner".into(), "foreign".into());
                }
                "orphan" => changed
                    .graph
                    .edges
                    .retain(|e| e.kind != "has_field" || e.target != id),
                _ => {
                    let mut alias = changed.graph.nodes[index].clone();
                    alias.id = format!("alias-{id}");
                    let mut declaration = changed
                        .graph
                        .edges
                        .iter()
                        .find(|e| e.kind == "has_field" && e.target == id)
                        .unwrap()
                        .clone();
                    declaration.id = format!("alias-{role}");
                    declaration.target = alias.id.clone();
                    declaration.ordinal = changed.graph.edges.len();
                    changed.graph.nodes.push(alias);
                    changed.graph.edges.push(declaration);
                }
            }
            rejected(&changed, &format!("{role}/{mutation}"));
        }
    }
    for attribute in [
        "path",
        "source",
        "destination",
        "root",
        "type",
        "executable",
    ] {
        let mut changed = original.clone();
        changed
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "DirectoryOperation")
            .unwrap()
            .attributes
            .insert(attribute.into(), "/hidden/authority".into());
        rejected(&changed, attribute);
    }
}

#[test]
fn directory_relevant_node_identities_and_edge_multiplicity_are_unambiguous() {
    let original = core(SOURCE);
    let mut relevant = std::collections::BTreeSet::new();
    for edge in &original.graph.edges {
        if edge.kind.starts_with("directory_") {
            relevant.insert(edge.source.clone());
            relevant.insert(edge.target.clone());
        }
    }
    for edge in &original.graph.edges {
        if edge.kind == "has_field" && relevant.contains(&edge.target) {
            relevant.insert(edge.source.clone());
        }
    }
    for edge in &original.graph.edges {
        if edge.kind == "has_provenance" && relevant.contains(&edge.source) {
            relevant.insert(edge.target.clone());
        }
    }
    for kind in [
        "DirectoryOperation",
        "Action",
        "Field",
        "Thing",
        "Provenance",
    ] {
        let ids: Vec<_> = original
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind == kind && relevant.contains(&n.id))
            .map(|n| n.id.clone())
            .collect();
        assert!(!ids.is_empty(), "missing fixture nodes {kind}");
        for id in ids {
            let mut changed = original.clone();
            let duplicate = changed
                .graph
                .nodes
                .iter()
                .find(|n| n.id == id)
                .unwrap()
                .clone();
            changed.graph.nodes.push(duplicate);
            rejected(&changed, &format!("duplicate {kind} {id}"));
        }
    }
    for kind in [
        "directory_step",
        "directory_path",
        "directory_destination",
        "directory_source",
    ] {
        let mut changed = original.clone();
        let edge = changed
            .graph
            .edges
            .iter()
            .find(|e| e.kind == kind)
            .unwrap()
            .clone();
        changed.graph.edges.push(edge);
        rejected(&changed, &format!("same-ID duplicate {kind}"));
    }
}

#[test]
fn directory_semantic_accesses_enforce_secrets_without_descriptive_edges() {
    for (role, diagnostic) in [
        ("directory_path", "AIL005"),
        ("directory_source", "AIL005"),
        ("directory_destination", "AIL002"),
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
fn directory_effects_reject_unsupported_profiles_functions_and_unreachable_bad_ir() {
    let original = core(SOURCE);
    for profile in ["Compiler", "CompilerPass", "AgentTool", "UI", "System"] {
        let mut changed = original.clone();
        changed.package.profile = profile.into();
        rejected(&changed, profile);
    }
    let mut interop = original.clone();
    interop.package.profile = "C interop".into();
    assert!(compile_ail_core_bytecode(&interop).is_ok());
    for attempt in [
        "the function lists names in directory resource workspace at local.value into local.value",
        "the function reads text from directory resource workspace at local.value into local.value",
        "the function writes text local.value to directory resource workspace at local.value",
        "the function lists names in directory",
        "the function reads text from directory",
        "the function writes text to directory",
    ] {
        let source = format!(
            "The application Workspace manages files.\nFunction: identity.\nThe function needs:\n- value: Text\nThe function produces:\n- result: Text\nThe function executes:\n- {attempt}\n- the function returns local.value\nAction: Run.\nWhen Run happens:\n- the system records a trace event named Done\n"
        );
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        if let Ok(document) = parse_ail_package_spec_text(&package, &source) {
            rejected(&elaborate_ail_core(&package, &document), attempt);
        }
    }
    let good = compile_ail_core_bytecode(&original).unwrap();
    for opcode in [
        "DIRECTORY_LIST_NAMES",
        "DIRECTORY_READ_TEXT",
        "DIRECTORY_WRITE_TEXT",
    ] {
        for mutation in [
            "extra",
            "missing",
            "resource",
            "expression",
            "literal",
            "unknown_opcode",
        ] {
            let mut changed = good.clone();
            let mut action = changed.actions["Run"].clone();
            action.name = "Unreachable".into();
            let instruction = action
                .instructions
                .iter_mut()
                .find(|i| i.opcode == opcode)
                .unwrap();
            match mutation {
                "extra" => {
                    instruction.operands.insert("root".into(), "/tmp".into());
                }
                "missing" => {
                    instruction.operands.remove("path");
                }
                "resource" => {
                    instruction
                        .operands
                        .insert("resource".into(), "bad..name".into());
                }
                "expression" => {
                    instruction
                        .operands
                        .insert("path".into(), "data.path + 1".into());
                }
                "literal" => {
                    instruction
                        .operands
                        .insert("path".into(), "\"a.txt\"".into());
                }
                _ => instruction.opcode = "DIRECTORY_UNKNOWN".into(),
            }
            changed.actions.insert("Unreachable".into(), action);
            assert!(
                !verify_ail_bytecode(&changed).is_empty(),
                "{opcode}/{mutation}"
            );
            assert!(compile_ail_bytecode_native_elf(&changed, "Run", "linux-x86_64-elf").is_err());
            assert!(
                run_ail_bytecode_action(&changed, "Run", std::collections::BTreeMap::new())
                    .unwrap_err()
                    .contains("invalid AIL bytecode")
            );
        }
    }
}

#[test]
fn imported_directory_effects_qualify_resources_and_every_field() {
    let mut imported = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    imported.metadata.name = "workspace-child".into();
    imported.spec_text = SOURCE.into();
    let mut parent = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    parent.spec_text = "The application Parent manages files.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system records a trace event named Done\n".into();
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
    let operations: Vec<_> = program
        .actions
        .values()
        .flat_map(|a| &a.instructions)
        .filter(|i| i.opcode.starts_with("DIRECTORY_"))
        .collect();
    assert_eq!(operations.len(), 3);
    for operation in operations {
        assert!(operation.operands["resource"].starts_with("child."));
        for (name, value) in &operation.operands {
            if name != "resource" {
                assert!(value.starts_with("child.data."), "{name}: {value}");
            }
        }
    }
    let error =
        run_ail_bytecode_action(&program, "Run", std::collections::BTreeMap::new()).unwrap_err();
    assert!(error.starts_with("AIL-RUNTIME-CAPABILITY:"), "{error}");
}

#[test]
fn previous_apis_default_deny_directory_authority() {
    use std::collections::BTreeMap;
    let program = compile_ail_core_bytecode(&core(SOURCE)).unwrap();
    let files = AilFileResources {
        reads: BTreeMap::from([("workspace".into(), "/tmp".into())]),
        writes: BTreeMap::new(),
    };
    let processes = AilProcessResources::default();
    let outcomes = [
        run_ail_bytecode_action(&program, "Run", BTreeMap::new()),
        run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &files),
        run_ail_bytecode_action_with_io(
            &program,
            "Run",
            BTreeMap::new(),
            &files,
            &mut AilStandardStreams::default(),
        ),
        run_ail_bytecode_action_with_resources(
            &program,
            "Run",
            BTreeMap::new(),
            &files,
            &mut AilStandardStreams::default(),
            &processes,
        ),
        run_ail_bytecode_action_with_host_resources(
            &program,
            "Run",
            BTreeMap::new(),
            &AilHostResources::default(),
            &mut AilStandardStreams::default(),
        ),
    ];
    for result in outcomes {
        let error = result.unwrap_err();
        assert!(error.starts_with("AIL-RUNTIME-CAPABILITY:"), "{error}");
    }
}

#[test]
fn every_unused_api_directory_binding_validates_before_stream_effects() {
    use std::collections::BTreeMap;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;
    let source = "The application Workspace manages files.\nA Data has:\n- body: Text\nAction: Run.\nWhen Run happens:\n- the system writes text data.body to standard output\n- the system records a trace event named Done\n";
    let program = compile_ail_core_bytecode(&core(source)).unwrap();
    for write_map in [false, true] {
        for (name, path) in [
            ("unused".to_string(), PathBuf::new()),
            ("unused".into(), PathBuf::from("relative")),
            ("bad..name".into(), PathBuf::from("/tmp")),
            ("x".repeat(257), PathBuf::from("/tmp")),
            (
                "unused".into(),
                PathBuf::from(std::ffi::OsString::from_vec(
                    b"/tmp/hidden\0suffix".to_vec(),
                )),
            ),
            (
                "unused".into(),
                PathBuf::from(format!("/{}", "x".repeat(4095))),
            ),
        ] {
            let mut grants = AilHostResources::default();
            let selected = if write_map {
                &mut grants.directories.writes
            } else {
                &mut grants.directories.reads
            };
            selected.insert(name, path);
            let mut output = Vec::new();
            let mut streams = AilStandardStreams {
                stdin: None,
                stdout: Some(&mut output),
                stderr: None,
            };
            let result = run_ail_bytecode_action_with_host_resources(
                &program,
                "Run",
                BTreeMap::from([("data.body".into(), "must not write".into())]),
                &grants,
                &mut streams,
            );
            let error = result.unwrap_err();
            assert!(error.starts_with("AIL-RUNTIME-CAPABILITY:"), "{error}");
            assert!(output.is_empty());
        }
    }
    let mut valid_unused = AilHostResources::default();
    valid_unused.directories.reads.insert(
        "x".repeat(256),
        PathBuf::from(format!("/{}", "x".repeat(4094))),
    );
    let mut output = Vec::new();
    let mut streams = AilStandardStreams {
        stdin: None,
        stdout: Some(&mut output),
        stderr: None,
    };
    run_ail_bytecode_action_with_host_resources(
        &program,
        "Run",
        BTreeMap::from([("data.body".into(), "valid unused".into())]),
        &valid_unused,
        &mut streams,
    )
    .unwrap();
    assert_eq!(output, b"valid unused");
    let mut invalid_program = program.clone();
    invalid_program.actions.get_mut("Run").unwrap().instructions[0].opcode =
        "DIRECTORY_UNKNOWN".into();
    valid_unused
        .directories
        .reads
        .insert("bad-name".into(), PathBuf::from("relative"));
    let error = run_ail_bytecode_action_with_host_resources(
        &invalid_program,
        "Run",
        BTreeMap::new(),
        &valid_unused,
        &mut AilStandardStreams::default(),
    )
    .unwrap_err();
    assert!(error.starts_with("invalid AIL bytecode:"), "{error}");
}
