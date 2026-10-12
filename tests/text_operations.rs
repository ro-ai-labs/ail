use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

const SOURCE: &str = "The application TextTools manages text.\n\nAn Input has:\n- first: Text\n- second: Text\n- start: Int\n- end: Int\n\nAn Output has:\n- joined: Text\n- part: Text\n- count: Int\n- position: Int\n\nAction: Process.\n\nWhen Process happens:\n- the system joins text input.first and input.second into output.joined\n- the system counts characters in output.joined into output.count\n- the system slices text output.joined from input.start to input.end into output.part\n- the system finds text input.second in output.joined into output.position\n- the system returns output.joined\n- the system returns output.part\n- the system returns output.count\n- the system returns output.position\n- the system records a trace event named Processed\n";

fn package() -> AilPackage {
    load_ail_package_dir(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/bench/R001-add/program.ail"
    ))
    .unwrap()
}
fn core(source: &str) -> AilCore {
    let package = package();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}
fn state(first: &str, second: &str, start: &str, end: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("input.first".into(), first.into()),
        ("input.second".into(), second.into()),
        ("input.start".into(), start.into()),
        ("input.end".into(), end.into()),
    ])
}
fn native(core: &AilCore, action: &str, state: &BTreeMap<String, String>) -> Output {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "ail-text-{}-{}.elf",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(
        &path,
        compile_ail_core_native_elf(core, action, "linux-x86_64-elf").unwrap(),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let arguments = state
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>();
    let mut retries = 0;
    let output = loop {
        match Command::new(&path).args(&arguments).output() {
            Ok(output) => break output,
            Err(error) if error.raw_os_error() == Some(26) && retries < 10 => {
                retries += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => panic!("{error}"),
        }
    };
    fs::remove_file(path).unwrap();
    output
}

#[test]
fn unicode_text_operations_execute_through_typed_core_ste_and_native() {
    let original = core(SOURCE);
    assert_eq!(
        original
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind == "TextOperation")
            .count(),
        4
    );
    assert!(
        check_ail_core(&original).is_empty(),
        "{:?}",
        check_ail_core(&original)
    );
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let ste = compile_ail_core_ste(&candidate).unwrap();
        let program = ail::ste::parse_ail_ste(&ste).unwrap();
        let input = state("Aé🙂", "e\u{301}", "-4", "99");
        let result = run_ail_bytecode_action(&program, "Process", input.clone()).unwrap();
        assert_eq!(result.final_state["output.joined"], "Aé🙂e\u{301}");
        assert_eq!(result.final_state["output.part"], "é🙂e\u{301}");
        assert_eq!(result.final_state["output.count"], "5");
        assert_eq!(result.final_state["output.position"], "3");
        let native = native(&candidate, "Process", &input);
        assert!(
            native.status.success(),
            "{}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(
            String::from_utf8(native.stdout).unwrap(),
            "output.count=5\noutput.joined=Aé🙂e\u{301}\noutput.part=é🙂e\u{301}\noutput.position=3\n"
        );
    }
}

#[test]
fn slices_clamp_extreme_bounds_and_keep_empty_text_present() {
    let core = core(SOURCE);
    let program = compile_ail_core_bytecode(&core).unwrap();
    for (first, second, start, end, part, count, position) in [
        ("", "", "0", "1", "", "0", "0"),
        (
            "ab🙂",
            "",
            "-9223372036854775808",
            "9223372036854775807",
            "ab🙂",
            "3",
            "0",
        ),
        ("ab🙂", "", "2", "1", "", "3", "0"),
        ("ab🙂", "", "99", "100", "", "3", "0"),
        ("éé", "é", "-2", "-1", "é", "3", "0"),
    ] {
        let input = state(first, second, start, end);
        let result = run_ail_bytecode_action(&program, "Process", input.clone()).unwrap();
        assert_eq!(result.final_state["output.part"], part);
        assert_eq!(result.final_state["output.count"], count);
        assert_eq!(result.final_state["output.position"], position);
        let native = native(&core, "Process", &input);
        assert!(
            native.status.success(),
            "{}",
            String::from_utf8_lossy(&native.stderr)
        );
        let expected = format!(
            "output.count={count}\noutput.joined={first}{second}\noutput.part={part}\noutput.position={position}\n"
        );
        assert_eq!(String::from_utf8(native.stdout).unwrap(), expected);
    }
}

#[test]
fn missing_text_and_invalid_bounds_fail_without_native_result_stdout() {
    let core = core(SOURCE);
    let program = compile_ail_core_bytecode(&core).unwrap();
    let mut missing = state("hello", "!", "0", "99");
    missing.remove("input.first");
    for (input, category) in [
        (missing, "AIL-RUNTIME-TEXT"),
        (state("hello", "!", "bad", "99"), "AIL-RUNTIME-INT"),
    ] {
        assert!(
            run_ail_bytecode_action(&program, "Process", input.clone())
                .unwrap_err()
                .contains(category)
        );
        let output = native(&core, "Process", &input);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(category),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn text_source_types_profiles_and_protocol_are_checked() {
    for source in [
        SOURCE.replace("text input.first and", "text input.start and"),
        SOURCE.replace("into output.joined", "into output.count"),
        SOURCE.replace(
            "characters in output.joined into output.count",
            "characters in output.joined into output.part",
        ),
        SOURCE.replace("from input.start", "from input.first"),
        SOURCE.replace("text input.first and", "text input.missing and"),
    ] {
        let core = core(&source);
        assert!(!check_ail_core(&core).is_empty());
        assert!(compile_ail_core_bytecode(&core).is_err());
    }
    for profile in ["Compiler", "AgentTool", "UI", "System"] {
        let mut core = core(SOURCE);
        core.package.profile = profile.into();
        assert!(
            check_ail_core(&core)
                .iter()
                .any(|s| s.contains("AIL-TEXT-")),
            "{profile}"
        );
    }
    let original = core(SOURCE);
    let mut compiler_pass = original.clone();
    compiler_pass
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Action")
        .unwrap()
        .attributes
        .insert("kind".into(), "CompilerPass".into());
    assert!(!check_ail_core(&compiler_pass).is_empty());
    for edge_kind in [
        "text_source",
        "text_other",
        "text_destination",
        "text_start",
        "text_end",
    ] {
        let mut duplicated = original.clone();
        duplicated.graph.edges.push(
            duplicated
                .graph
                .edges
                .iter()
                .find(|e| e.kind == edge_kind)
                .unwrap()
                .clone(),
        );
        let saved = parse_ail_core_text(&render_ail_core(&duplicated)).unwrap();
        assert!(!check_ail_core(&saved).is_empty(), "{edge_kind}");
        assert!(compile_ail_core_bytecode(&saved).is_err());
    }
    let program = compile_ail_core_bytecode(&original).unwrap();
    for (opcode, operand, value) in [
        ("TEXT_JOIN", "hidden", "discarded"),
        ("TEXT_LENGTH", "source", "not a field"),
        ("TEXT_SLICE", "start", "i:1 nonsense"),
        ("TEXT_FIND", "needle", ""),
    ] {
        let mut malformed = program.clone();
        malformed
            .actions
            .get_mut("Process")
            .unwrap()
            .instructions
            .iter_mut()
            .find(|i| i.opcode == opcode)
            .unwrap()
            .operands
            .insert(operand.into(), value.into());
        assert!(
            !verify_ail_bytecode(&malformed).is_empty(),
            "{opcode} {operand}"
        );
        assert!(ail::ste::render_ail_ste(&malformed).is_err());
    }
}

#[test]
fn find_uses_character_offsets_and_reports_absent_substrings() {
    let source = SOURCE.replace(
        "text input.second in output.joined",
        "text input.second in input.first",
    );
    let core = core(&source);
    let program = compile_ail_core_bytecode(&core).unwrap();
    for (haystack, needle, expected) in [
        ("é🙂e\u{301}", "e\u{301}", "2"),
        ("é🙂", "x", "-1"),
        ("e\u{301}", "é", "-1"),
        ("🙂🙂", "🙂", "0"),
    ] {
        let input = state(haystack, needle, "0", "99");
        let result = run_ail_bytecode_action(&program, "Process", input.clone()).unwrap();
        assert_eq!(result.final_state["output.position"], expected);
        let output = native(&core, "Process", &input);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .ends_with(&format!("output.position={expected}\n"))
        );
    }
}

#[test]
fn text_order_copies_and_in_place_writes_preserve_value_lifetimes() {
    let source = SOURCE.replace(
        "- the system counts characters in output.joined into output.count",
        "- the system changes the output joined as the output part\n- the system joins text output.joined and output.joined into output.joined\n- the system counts characters in output.part into output.count",
    ).replace("- the system slices text output.joined from input.start to input.end into output.part\n", "");
    let original = core(&source);
    for candidate in [
        original.clone(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let input = state("é", "🙂", "0", "99");
        let result = run_ail_bytecode_action(&program, "Process", input.clone()).unwrap();
        assert_eq!(result.final_state["output.part"], "é🙂");
        assert_eq!(result.final_state["output.joined"], "é🙂é🙂");
        assert_eq!(result.final_state["output.count"], "2");
        let output = native(&candidate, "Process", &input);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "output.count=2\noutput.joined=é🙂é🙂\noutput.part=é🙂\noutput.position=1\n"
        );
    }
}

#[test]
fn text_limits_and_shared_allocation_budget_match_across_action_calls() {
    let declarations = SOURCE.split("Action: Process.").next().unwrap();
    let grow = "- the system computes input.start as 0\n- while input.start < input.end:\n- the system joins text input.first and input.first into input.first\n- the system computes input.start as input.start + 1\n- end while\n";
    let source = format!(
        "{declarations}Action: Process.\nWhen Process happens:\n{grow}- the system calls CopyMany\n- the system calls CopyMany\n- the system returns input.first\n- the system records a trace event named Processed\n\nAction: CopyMany.\nWhen CopyMany happens:\n- the system computes input.start as 0\n- while input.start < 35:\n- the system slices text input.first from 0 to 1048576 into output.part\n- the system computes input.start as input.start + 1\n- end while\n- the system records a trace event named Copied\n"
    );
    let core = core(&source);
    let program = compile_ail_core_bytecode(&core).unwrap();
    for end in ["10", "11"] {
        let input = state(&"x".repeat(1024), "", "0", end);
        let error = run_ail_bytecode_action(&program, "Process", input.clone()).unwrap_err();
        assert!(error.contains("AIL-RUNTIME-TEXT-LIMIT"), "{error}");
        let output = native(&core, "Process", &input);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn public_document_execution_and_imports_include_text_operations() {
    let package = package();
    let document = parse_ail_package_spec_text(&package, SOURCE).unwrap();
    let result = run_ail_action(&document, "Process", state("a", "b", "0", "2")).unwrap();
    assert_eq!(result.final_state["output.joined"], "ab");
    let mut child = package.clone();
    child.metadata.name = "text-child".into();
    child.spec_text = SOURCE.into();
    let mut parent = package;
    parent.spec_text = "The application Parent manages text.\nAction: Root.\nWhen Root happens:\n- the system calls Words.Process\n- the system returns words.output.joined\n- the system records a trace event named Completed\n".into();
    parent.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Words".into(),
            resolved_package: None,
            registry_identity: None,
        },
        package: Box::new(child),
    });
    let original = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
    ] {
        let input = state("é", "🙂", "0", "2")
            .into_iter()
            .map(|(k, v)| (format!("words.{k}"), v))
            .collect::<BTreeMap<_, _>>();
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let result = run_ail_bytecode_action(&program, "Root", input.clone()).unwrap();
        assert_eq!(
            result.output_state(),
            BTreeMap::from([("words.output.joined".into(), "é🙂".into())])
        );
        let output = native(&candidate, "Root", &input);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "words.output.joined=é🙂\n"
        );
    }
}

#[test]
fn agent_text_pipeline_combines_search_integer_calls_and_slices() {
    let declarations = SOURCE.split("Action: Process.").next().unwrap();
    let source = format!(
        "{declarations}Function: next.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function executes:\n- the function returns local.n + 1\n\nAction: Process.\nWhen Process happens:\n- the system finds text input.second in input.first into input.start\n- the system calls next with n = input.start into input.start\n- the system slices text input.first from input.start to input.end into output.part\n- the system counts characters in output.part into output.count\n- the system joins text output.part and input.second into output.joined\n- the system finds text input.second in output.joined into output.position\n- the system returns output.part\n- the system returns output.joined\n- the system returns output.count\n- the system returns output.position\n- the system records a trace event named Extracted\n"
    );
    let core = core(&source);
    let program = compile_ail_core_bytecode(&core).unwrap();
    let input = state("name:é🙂", ":", "0", "99");
    let result = run_ail_bytecode_action(&program, "Process", input.clone()).unwrap();
    assert_eq!(result.final_state["output.part"], "é🙂");
    assert_eq!(result.final_state["output.count"], "2");
    assert_eq!(result.final_state["output.joined"], "é🙂:");
    assert_eq!(result.final_state["output.position"], "2");
    let output = native(&core, "Process", &input);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "output.count=2\noutput.joined=é🙂:\noutput.part=é🙂\noutput.position=2\n"
    );
}

#[test]
fn json_text_transport_decodes_unicode_surrogate_pairs() {
    let mut program = compile_ail_core_bytecode(&core(SOURCE)).unwrap();
    program.package_name = "Aé🙂".into();
    let encoded = render_ail_bytecode(&program);
    assert_eq!(
        parse_ail_bytecode(&encoded.replace("Aé🙂", r"A\u00e9\ud83d\ude42"))
            .unwrap()
            .package_name,
        "Aé🙂"
    );
    for invalid in [r"\ud83d", r"\ude42", r"\ud83d\u0041", r"\ud83d\ud83d"] {
        assert!(
            parse_ail_bytecode(&encoded.replace("Aé🙂", invalid)).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn text_operations_enforce_secret_access_rules_without_inferred_edges() {
    for (field, code) in [
        ("Input.first", "AIL005"),
        ("Input.start", "AIL005"),
        ("Output.joined", "AIL002"),
    ] {
        let mut candidate = core(SOURCE);
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name == field)
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        candidate
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        for candidate in [
            candidate.clone(),
            parse_ail_core_text(&render_ail_core(&candidate)).unwrap(),
        ] {
            let diagnostics = check_ail_core(&candidate).join("\n");
            assert!(diagnostics.contains(code), "{field}: {diagnostics}");
            assert!(compile_ail_core_bytecode(&candidate).is_err());
        }
        let owner = candidate
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "Action" && n.name == "Process")
            .unwrap()
            .clone();
        let target = candidate
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == "Field" && n.name == field)
            .unwrap()
            .clone();
        candidate
            .graph
            .add_edge("protects_secret", &owner, &target, BTreeMap::new());
        assert!(
            check_ail_core(&candidate).is_empty(),
            "{:?}",
            check_ail_core(&candidate)
        );
    }
}
