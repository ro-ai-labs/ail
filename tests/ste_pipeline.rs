use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn ail(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ail"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap()
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("ail-ste-{name}-{}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn ste_export_can_be_checked_executed_and_compiled() {
    let root = scratch("pipeline");
    let ste = success(ail(&["ail-ste", "bench/R000-increment/program.ail"]));
    assert!(ste.starts_with("AIL-STE 1\n"));
    assert!(ste.contains("Add "));
    assert!(!ste.contains("ADD_INT_FIELD"));
    let source = root.join("counter.ail-ste");
    fs::write(&source, &ste).unwrap();
    let source = source.to_str().unwrap();
    success(ail(&["ail-check", source]));
    let run = success(ail(&[
        "ail-run",
        source,
        "--action",
        "IncrementCounter",
        "counter.value=41",
    ]));
    assert!(run.starts_with("ail-run succeeded\n"), "{run}");
    assert!(run.contains("counter.value=42"), "{run}");
    let lowered = success(ail(&["ail-lower", source]));
    let original = success(ail(&["ail-lower", "bench/R000-increment/program.ail"]));
    assert_eq!(lowered, original);
    let binary = root.join("counter");
    success(ail(&[
        "ail-compile",
        source,
        "--action",
        "IncrementCounter",
        "--target",
        "linux-x86_64-elf",
        "--out",
        binary.to_str().unwrap(),
    ]));
    let native = success(
        Command::new(&binary)
            .arg("counter.value=41")
            .output()
            .unwrap(),
    );
    assert!(native.contains("counter.value=42"), "{native}");
}

#[test]
fn normal_compilation_saves_the_executable_ste_intermediate() {
    let root = scratch("artifacts");
    success(ail(&[
        "ail-compile",
        "bench/R000-increment/program.ail",
        "--action",
        "IncrementCounter",
        "--target",
        "linux-x86_64-elf",
        "--out",
        root.join("counter").to_str().unwrap(),
        "--artifact-dir",
        root.to_str().unwrap(),
    ]));
    let ste = fs::read_to_string(root.join("artifact.ail-ste")).expect("compiler must save STE");
    let direct = success(ail(&["ail-ste", "bench/R000-increment/program.ail"]));
    assert_eq!(ste, direct);
    let reparsed = success(ail(&[
        "ail-lower",
        root.join("artifact.ail-ste").to_str().unwrap(),
    ]));
    assert_eq!(
        reparsed,
        fs::read_to_string(root.join("artifact.ailbc.json")).unwrap()
    );
}

#[test]
fn ste_rejects_unknown_instructions_instead_of_dropping_them() {
    let root = scratch("invalid");
    let ste = success(ail(&["ail-ste", "bench/R000-increment/program.ail"]));
    let source = root.join("invalid.ail-ste");
    fs::write(
        &source,
        ste.replace("Return success.", "Launch the unknown operation."),
    )
    .unwrap();
    for command in ["ail-check", "ail-lower", "ail-run"] {
        let output = if command == "ail-run" {
            ail(&[
                command,
                source.to_str().unwrap(),
                "--action",
                "IncrementCounter",
            ])
        } else {
            ail(&[command, source.to_str().unwrap()])
        };
        assert!(
            !output.status.success(),
            "{command} accepted unknown instruction"
        );
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(diagnostics.contains("AIL-STE-001"), "{diagnostics}");
        assert!(diagnostics.contains("line"), "{diagnostics}");
    }
}

#[test]
fn authored_ste_changes_vm_and_native_results_when_an_operand_changes() {
    let root = scratch("authored");
    let original = include_str!("../examples/ste/counter.ail-ste");
    for (delta, expected) in [(7, 48), (-3, 38)] {
        let source = root.join(format!("counter-{delta}.ail-ste"));
        fs::write(
            &source,
            original.replace("Data delta = \"7\"", &format!("Data delta = \"{delta}\"")),
        )
        .unwrap();
        let source = source.to_str().unwrap();
        let vm = success(ail(&[
            "ail-vm",
            source,
            "--action",
            "IncrementCounter",
            "counter.value=41",
        ]));
        assert!(vm.contains(&format!("counter.value={expected}")), "{vm}");
        let binary = root.join(format!("counter-{delta}"));
        success(ail(&[
            "ail-compile",
            source,
            "--action",
            "IncrementCounter",
            "--target",
            "linux-x86_64-elf",
            "--out",
            binary.to_str().unwrap(),
        ]));
        let native = success(
            Command::new(binary)
                .arg("counter.value=41")
                .output()
                .unwrap(),
        );
        assert!(
            native.contains(&format!("counter.value={expected}")),
            "{native}"
        );
    }
}

#[test]
fn ste_rejects_ambiguous_incomplete_and_hidden_executable_input() {
    use ail::ste::parse_ail_ste;
    let original = include_str!("../examples/ste/counter.ail-ste");
    let cases = [
        original.replace("AIL-STE 1", "AIL-STE 2"),
        original.replace(
            "Data delta = \"7\"",
            "Data delta = \"7\"\nData delta = \"8\"",
        ),
        original.replace("Data delta = \"7\"", "Data delta = \"7\" trailing"),
        original.replace("Add delta", "Add absent"),
        original.replace("Add delta", "add delta"),
        original.replace(
            "Attribute text = description",
            "Attribute typo = description",
        ),
        original.replace(
            "Attribute text = description",
            "Attribute text = description\nAttribute text = description",
        ),
        original.replace("Attribute text = description", ""),
        original.replace("End procedure.", ""),
        original.replace("\"failures\":[]", "\"failures\":[],\"actions\":[]"),
        original.replace(
            "\"failures\":[]",
            "\"failures\":[],\"unexpected\":\"ignored\"",
        ),
        original.replace(
            "\"profile\":\"Application\"",
            "\"profile\":\"Application\",\"profile\":\"System\"",
        ),
        original.replace("Return success.", "Go to label event."),
        original.replace("Return success.", "Do action event."),
        format!("{original}\nProcedure action.\nReturn success.\nEnd procedure.\n"),
    ];
    for source in cases {
        let error = parse_ail_ste(&source).expect_err(&source);
        assert!(error.starts_with("AIL-STE-001 line "), "{error}");
    }
}

#[test]
fn ste_preserves_data_that_looks_like_code() {
    use ail::ste::{parse_ail_ste, render_ail_ste};
    let original = include_str!("../examples/ste/counter.ail-ste");
    assert!(
        parse_ail_ste(&original.replace("Add seven to the counter.", "text\u{0085}text")).is_ok()
    );
    // Construct through the bytecode API to exercise quotes, slashes, controls,
    // Unicode and sentence-shaped content without depending on source escaping.
    let mut program = parse_ail_ste(original).unwrap();
    let payload = "café 🦀 \"quoted\" \\ path\nReturn success.\r\t\u{0001}\u{0085}";
    program
        .actions
        .get_mut("IncrementCounter")
        .unwrap()
        .instructions[1]
        .operands
        .insert("text".to_string(), payload.to_string());
    let rendered = render_ail_ste(&program).unwrap();
    assert_eq!(parse_ail_ste(&rendered).unwrap(), program);
    assert_eq!(rendered.matches("Return success.").count(), 2);
}

#[test]
fn checked_example_packages_keep_their_bytecode_through_ste() {
    use ail::ail::{
        check_ail_core, compile_ail_core_bytecode, compile_ail_core_ste, elaborate_ail_core,
        load_ail_package_dir, parse_ail_package_document,
    };
    use ail::ste::{parse_ail_ste, render_ail_ste};
    let mut checked = 0;
    for entry in fs::read_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")).unwrap() {
        let path = entry.unwrap().path();
        if !path.join("ail-package.md").is_file() {
            continue;
        }
        let Ok(package) = load_ail_package_dir(&path) else {
            continue;
        };
        let document = parse_ail_package_document(&package).unwrap();
        let core = elaborate_ail_core(&package, &document);
        if !check_ail_core(&core).is_empty() {
            continue;
        }
        let ste = compile_ail_core_ste(&core).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let bytecode = parse_ail_ste(&ste).unwrap();
        assert_eq!(bytecode, compile_ail_core_bytecode(&core).unwrap());
        assert_eq!(render_ail_ste(&bytecode).unwrap(), ste);
        checked += 1;
    }
    assert!(
        checked >= 25,
        "only {checked} example packages were checked"
    );
}

#[test]
fn saved_core_uses_the_same_ste_path_as_package_compilation() {
    let root = scratch("core");
    let core = success(ail(&["ail-core", "bench/R000-increment/program.ail"]));
    let path = root.join("checked.ail-core.txt");
    fs::write(&path, core).unwrap();
    assert_eq!(
        success(ail(&["ail-ste", "--core-file", path.to_str().unwrap()])),
        success(ail(&["ail-ste", "bench/R000-increment/program.ail"]))
    );
}

#[test]
fn ste_comparison_order_matches_the_sentence_in_vm_and_native() {
    use ail::ail::{compile_ail_bytecode_native_elf, run_ail_bytecode_action};
    use ail::ste::parse_ail_ste;
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt;
    let root = scratch("comparison");
    let source = include_str!("../examples/ste/counter.ail-ste")
        .replace("Procedure action.", "Data earlier = \"event.start\"\nData later = \"event.end\"\nData failure = \"BadOrder\"\nProcedure action.")
        .replace("Add delta to field counter.\nAttribute text = description", "Check that field later is after field earlier for rule description with failure failure.");
    let program = parse_ail_ste(&source).unwrap();
    let binary = root.join("compare");
    fs::write(
        &binary,
        compile_ail_bytecode_native_elf(&program, "IncrementCounter", "linux-x86_64-elf").unwrap(),
    )
    .unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    for (start, end, expected) in [
        ("2026-01-01", "2026-01-02", true),
        ("2026-01-02", "2026-01-01", false),
        ("same", "same", false),
    ] {
        let state = BTreeMap::from([
            ("event.start".to_string(), start.to_string()),
            ("event.end".to_string(), end.to_string()),
        ]);
        let vm = run_ail_bytecode_action(&program, "IncrementCounter", state).unwrap();
        assert_eq!(
            vm.status == "succeeded",
            expected,
            "start={start}, end={end}"
        );
        let native = Command::new(&binary)
            .args([format!("event.start={start}"), format!("event.end={end}")])
            .output()
            .unwrap();
        assert_eq!(
            native.status.success(),
            expected,
            "start={start}, end={end}"
        );
    }
}

#[test]
fn ste_observation_sentences_do_not_promise_runtime_effects() {
    use ail::ail::run_ail_bytecode_action;
    use ail::ste::parse_ail_ste;
    use std::collections::BTreeMap;
    let source = include_str!("../examples/ste/counter.ail-ste")
        .replace("Add delta to field counter.\nAttribute text = description", "Record field write counter with description description.\nRecord tool call action.\nRecord field read counter.\nAttribute text = description");
    let program = parse_ail_ste(&source).unwrap();
    let state = BTreeMap::from([("counter.value".to_string(), "41".to_string())]);
    let result = run_ail_bytecode_action(&program, "IncrementCounter", state.clone()).unwrap();
    assert_eq!(result.final_state, state);
    assert!(
        result
            .trace
            .iter()
            .any(|line| line == "write counter.value")
    );
}

#[test]
fn ste_export_rejects_an_ignored_spec_override() {
    let output = ail(&[
        "ail-ste",
        "examples/ste/counter.ail-ste",
        "--spec-file",
        "/DOES_NOT_EXIST",
    ]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot be combined with --spec-file")
    );
}

#[test]
fn ste_bundle_preserves_absent_external_binding_metadata() {
    use ail::ail::{parse_ail_bytecode, render_ail_bytecode};
    use ail::ste::parse_ail_ste;
    let root = scratch("absent-bindings");
    let source = root.join("source.ail-ste");
    fs::write(
        &source,
        include_str!("../examples/ste/counter.ail-ste").replace(",\"external_bindings\":[]", ""),
    )
    .unwrap();
    let stdout = success(ail(&[
        "ail-ste",
        source.to_str().unwrap(),
        "--artifact-dir",
        root.to_str().unwrap(),
    ]));
    let saved = fs::read_to_string(root.join("artifact.ail-ste")).unwrap();
    assert_eq!(stdout, saved);
    let program = parse_ail_ste(&saved).unwrap();
    assert!(!program.external_bindings_metadata_present);
    assert_eq!(
        parse_ail_bytecode(&render_ail_bytecode(&program)).unwrap(),
        program
    );
}
