use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn package() -> AilPackage {
    load_ail_package_dir("bench/R001-add/program.ail").unwrap()
}
fn source(body: &str) -> String {
    format!(
        "The application Updates manages values.\n\nA Data has:\n- value: Int\n- delta: Int\n\nAn Output has:\n- value: Int\n\nAction: Run.\nWhen Run happens:\n{body}\n- the system returns data.value\n- the system returns output.value\n- the system records a trace event named Done\n"
    )
}
fn core(text: &str) -> AilCore {
    let package = package();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, text).unwrap(),
    )
}
fn routes(original: &AilCore) -> Vec<AilCore> {
    vec![
        original.clone(),
        parse_ail_core_text(&render_ail_core(original)).unwrap(),
        core(&render_ail_spec_from_core(original)),
    ]
}
fn initial(prefix: &str) -> BTreeMap<String, String> {
    [
        ("data.value", "11"),
        ("output.value", "100"),
        ("data.delta", "3"),
    ]
    .into_iter()
    .map(|(key, value)| (format!("{prefix}{key}"), value.into()))
    .collect()
}
fn assert_native(
    program: &AilBytecodeProgram,
    action: &str,
    state: &BTreeMap<String, String>,
    expected: &str,
) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "ail-qualified-update-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(
        &path,
        compile_ail_bytecode_native_elf(program, action, "linux-x86_64-elf").unwrap(),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(&path)
        .args(state.iter().map(|(key, value)| format!("{key}={value}")))
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}
fn assert_routes(text: &str, expected_data: &str, expected_output: &str) {
    let original = core(text);
    assert!(
        check_ail_core(&original).is_empty(),
        "{:?}",
        check_ail_core(&original)
    );
    for candidate in routes(&original) {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let state = initial("");
        let vm = run_ail_bytecode_action(&program, "Run", state.clone()).unwrap();
        assert_eq!(vm.final_state["data.value"], expected_data);
        assert_eq!(vm.final_state["output.value"], expected_output);
        assert_native(
            &program,
            "Run",
            &state,
            &format!("data.value={expected_data}\noutput.value={expected_output}\n"),
        );
    }
}
#[test]
fn qualified_literal_increment_and_decrement_select_the_named_owner() {
    assert_routes(
        &source(
            "- the system increments data.value by 1\n- the system decrements output.value by 2",
        ),
        "12",
        "98",
    );
}
#[test]
fn qualified_expression_updates_select_the_named_owner() {
    assert_routes(
        &source(
            "- the system increments data.value by data.delta\n- the system decrements output.value by data.delta + 1",
        ),
        "14",
        "96",
    );
}
#[test]
fn natural_language_updates_keep_their_owner_in_core_and_execution() {
    let original = core(&source(
        "- the system increments the data value by 1\n- the system decrements the output value by 2",
    ));
    for edge in original
        .graph
        .edges
        .iter()
        .filter(|edge| edge.kind == "writes")
    {
        let provenance = &edge.attributes["provenance"];
        let target = original
            .graph
            .nodes
            .iter()
            .find(|node| node.id == edge.target)
            .unwrap();
        assert_eq!(
            target.name,
            if provenance.contains("the data value") {
                "Data.value"
            } else {
                "Output.value"
            }
        );
    }
    assert_routes(&render_ail_spec_from_core(&original), "12", "98");
}
#[test]
fn ambiguous_bare_names_and_unknown_qualifiers_remain_rejected() {
    for destination in ["value", "missing.value", "other.data.value"] {
        let invalid = core(&source(&format!(
            "- the system increments {destination} by 1"
        )));
        assert!(
            compile_ail_core_bytecode(&invalid).is_err(),
            "{destination}"
        );
    }
    let single = "The application Updates manages values.\n\nA Data has:\n- value: Int\n\nAction: Run.\nWhen Run happens:\n- the system increments missing.value by 1\n- the system records a trace event named Done\n";
    assert!(
        compile_ail_core_bytecode(&core(single)).is_err(),
        "unknown qualification must not resolve a unique bare name"
    );
}
#[test]
fn imported_qualified_literal_updates_resolve_only_imported_storage() {
    let mut child = package();
    child.spec_text = source(
        "- the system increments data.value by 1\n- the system decrements output.value by 2",
    );
    let mut root = package();
    root.spec_text = "The application Root manages imports.\n\nA Data has:\n- value: Int\n".into();
    root.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Child".into(),
            resolved_package: None,
            registry_identity: None,
        },
        package: Box::new(child),
    });
    let original = elaborate_ail_core(&root, &parse_ail_package_document(&root).unwrap());
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let mut state = initial("child.");
        state.insert("data.value".into(), "777".into());
        let vm = run_ail_bytecode_action(&program, "Child.Run", state.clone()).unwrap();
        assert_eq!(vm.final_state["data.value"], "777");
        assert_eq!(vm.final_state["child.data.value"], "12");
        assert_eq!(vm.final_state["child.output.value"], "98");
        assert_native(
            &program,
            "Child.Run",
            &state,
            "child.data.value=12\nchild.output.value=98\n",
        );
    }
}
