use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application Predicates manages records.\n\nA Data has:\n- enabled: Bool\n- disabled: Bool\n- done: Bool\n- label: Text\n- other: Text\n- count: Int\n- index: Int\n- score: Float\n- limit: Float\n- document: Json\n- raw: Text\n\nAn Output has:\n- accept: Bool\n- selected: Bool\n- count: Int\n- raw: Text\n\n";

fn package() -> AilPackage {
    load_ail_package_dir("bench/R001-add/program.ail").unwrap()
}

fn source(body: &str) -> String {
    format!(
        "{DECLARATIONS}Action: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Values\n"
    )
}

fn core(text: &str) -> AilCore {
    let package = package();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, text).unwrap(),
    )
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "ail-boolean-predicates-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn file(&self, name: &str, value: &[u8]) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, value).unwrap();
        path
    }

    fn binary(&self, program: &AilBytecodeProgram, action: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = self.path("program.elf");
        fs::write(
            &path,
            compile_ail_bytecode_native_elf(program, action, "linux-x86_64-elf").unwrap(),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn resources(reads: &[(&str, &Path)], writes: &[(&str, &Path)]) -> AilFileResources {
    AilFileResources {
        reads: reads
            .iter()
            .map(|(name, path)| ((*name).into(), path.to_path_buf()))
            .collect(),
        writes: writes
            .iter()
            .map(|(name, path)| ((*name).into(), path.to_path_buf()))
            .collect(),
    }
}

fn state(values: &[(&str, &str)]) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect()
}

fn command_output(command: &mut Command) -> Output {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut attempts = 0;
    let mut child = loop {
        match command.spawn() {
            Ok(child) => break child,
            Err(error) if error.raw_os_error() == Some(26) && attempts < 10 => {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("cannot start test process: {error}"),
        }
    };
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let stdout_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            let _ = stdout_reader.join().unwrap();
            let stderr = stderr_reader.join().unwrap();
            panic!(
                "Boolean predicate blocked: {}",
                String::from_utf8_lossy(&stderr[..stderr.len().min(2_000)])
            );
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Output {
        status: child.wait().unwrap(),
        stdout: stdout_reader.join().unwrap(),
        stderr: stderr_reader.join().unwrap(),
    }
}

fn native(binary: &Path, grants: &AilFileResources, values: &BTreeMap<String, String>) -> Output {
    let mut command = Command::new(binary);
    for (name, path) in &grants.reads {
        command.arg(format!("--read-file={name}={}", path.display()));
    }
    for (name, path) in &grants.writes {
        command.arg(format!("--write-file={name}={}", path.display()));
    }
    for (key, value) in values {
        command.arg(format!("{key}={value}"));
    }
    command_output(&mut command)
}

fn brief(output: &Output) -> String {
    format!(
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout[..output.stdout.len().min(500)]),
        String::from_utf8_lossy(&output.stderr[..output.stderr.len().min(1_500)])
    )
}

fn failed(output: &Output, category: &str) {
    assert!(!output.status.success(), "{}", brief(output));
    assert!(output.stdout.is_empty(), "{}", brief(output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(category),
        "expected {category}: {}",
        brief(output)
    );
}

fn command(opcode: &str, operands: &[(&str, &str)]) -> AilBytecodeInstruction {
    AilBytecodeInstruction {
        opcode: opcode.into(),
        operands: state(operands),
    }
}

fn raw_program(instructions: Vec<AilBytecodeInstruction>) -> AilBytecodeProgram {
    let mut program =
        compile_ail_core_bytecode(&core(&source("- the system assigns 0 to output.count")))
            .unwrap();
    let code = &mut program.actions.get_mut("Run").unwrap().instructions;
    *code = instructions;
    code.push(command("RETURN_SUCCESS", &[]));
    program
}

fn with_functions(functions: &str, body: &str) -> String {
    format!(
        "{DECLARATIONS}{functions}\nAction: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Finished\n"
    )
}

#[test]
fn quoted_boolean_predicates_survive_source_core_rendered_ste_and_native() {
    let text = source(
        r#"- the system assigns "and or not (data.secret) [value=1],type=Text \u0000é🙂" to data.label
- the system assigns false to data.disabled
- the system computes boolean output.accept as not data.disabled and (text data.label == "and or not (data.secret) [value=1],type=Text \u0000é🙂" or false)
- the system assigns 0 to data.index
- while boolean integer data.index < 3 and not data.disabled:
- the system computes data.index as data.index + 1
- if boolean integer data.index == 2:
- continue
- end if
- the system computes output.count as data.index
- end while
- if boolean output.accept:
- the system computes boolean output.selected as floating -0 == 0 and boolean false < true
- otherwise:
- the system assigns false to output.selected
- end if
- the system returns output.accept
- the system returns output.selected
- the system returns output.count"#,
    );
    let original = core(&text);
    let scratch = Scratch::new();
    for (index, candidate) in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ]
    .into_iter()
    .enumerate()
    {
        let ste = compile_ail_core_ste(&candidate)
            .unwrap_or_else(|error| panic!("route {index}: {error}"));
        assert!(ste.contains("Compute Boolean expression"));
        assert!(ste.contains("If Boolean expression"));
        let program = ail::ste::parse_ail_ste(&ste).unwrap();
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(vm.final_state["output.accept"], "true");
        assert_eq!(vm.final_state["output.selected"], "true");
        assert_eq!(vm.final_state["output.count"], "3");
        assert!(vm.trace.contains(&"computed boolean output.accept".into()));
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(
            elf.stdout,
            b"output.accept=true\noutput.count=3\noutput.selected=true\n"
        );
    }
}

fn seed(key: &str, value: &str) -> AilBytecodeInstruction {
    command(
        "SET_FIELD",
        &[("key", key), ("value", value), ("text", "seed")],
    )
}

#[test]
fn short_circuit_skips_missing_globals_and_left_to_right_errors_stay_typed() {
    let scratch = Scratch::new();
    for (expression, expected) in [
        ("b:false bf:missing.flag and", Ok("false")),
        ("b:true tf:missing.text s:\"x\" tcmp:== or", Ok("true")),
        ("b:false if:missing.n i:0 i/ i:1 icmp:== and", Ok("false")),
        ("b:true ff:missing.x d:0 f/ d:1 fcmp:== or", Ok("true")),
        ("b:false bf:missing.flag or", Err("BOOL")),
        ("i:1 i:0 i/ if:missing.n icmp:==", Err("DIVZERO")),
        ("if:missing.n i:1 i:0 i/ icmp:==", Err("INT")),
        ("d:1 d:0 f/ ff:missing.x fcmp:==", Err("DIVZERO")),
        ("ff:missing.x d:1 d:0 f/ fcmp:==", Err("FLOAT")),
        (
            "bf:missing.flag i:1 i:0 i/ i:0 icmp:== bcmp:==",
            Err("BOOL"),
        ),
        ("tf:missing.text s:\"x\" tcmp:<", Err("TEXT")),
    ] {
        let program = raw_program(vec![
            command(
                "EVAL_BOOL",
                &[("key", "out.value"), ("expression", expression)],
            ),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new());
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        match expected {
            Ok(value) => {
                assert_eq!(vm.unwrap().final_state["out.value"], value);
                assert!(elf.status.success(), "{}", brief(&elf));
                assert_eq!(elf.stdout, format!("out.value={value}\n").as_bytes());
            }
            Err(category) => {
                assert!(
                    vm.unwrap_err()
                        .contains(&format!("AIL-RUNTIME-{category}:")),
                    "{expression}"
                );
                failed(&elf, &format!("AIL-RUNTIME-{category}:"));
            }
        }
    }
}

#[test]
fn all_scalar_comparisons_handle_unicode_nul_nan_and_signed_zero() {
    let scratch = Scratch::new();
    for (left, right, kind, expected) in [
        (
            "i:-1",
            "i:0",
            "icmp",
            [false, true, true, true, false, false],
        ),
        (
            "d:nan",
            "d:nan",
            "fcmp",
            [false, true, false, false, false, false],
        ),
        (
            "d:-0",
            "d:0",
            "fcmp",
            [true, false, false, true, false, true],
        ),
        (
            r#"s:"e\u0301""#,
            r#"s:"é""#,
            "tcmp",
            [false, true, true, true, false, false],
        ),
        (
            r#"s:"\u0000🙂""#,
            r#"s:"\u0000é""#,
            "tcmp",
            [false, true, false, false, true, true],
        ),
        (
            "b:false",
            "b:true",
            "bcmp",
            [false, true, true, true, false, false],
        ),
    ] {
        let mut steps = Vec::new();
        for (index, operator) in ["==", "!=", "<", "<=", ">", ">="].iter().enumerate() {
            steps.push(command(
                "EVAL_BOOL",
                &[
                    ("key", &format!("out.v{index}")),
                    ("expression", &format!("{left} {right} {kind}:{operator}")),
                ],
            ));
            steps.push(command(
                "OUTPUT_FIELD",
                &[("key", &format!("out.v{index}"))],
            ));
        }
        let program = raw_program(steps);
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        let mut wanted = String::new();
        for (index, value) in expected.iter().enumerate() {
            assert_eq!(vm.final_state[&format!("out.v{index}")], value.to_string());
            wanted.push_str(&format!("out.v{index}={value}\n"));
        }
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(elf.stdout, wanted.as_bytes());
    }
}

#[test]
fn bool_transport_and_reached_text_limits_fail_without_leaking_values() {
    let scratch = Scratch::new();
    let program = raw_program(vec![command(
        "EVAL_BOOL",
        &[("key", "out.value"), ("expression", "bf:input.flag")],
    )]);
    let binary = scratch.binary(&program, "Run");
    for raw in [
        "True",
        "FALSE",
        "1",
        "0",
        "null",
        " true",
        "true ",
        "PRIVATE_INVALID_BOOL",
    ] {
        let input = state(&[("input.flag", raw)]);
        let error = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap_err();
        assert!(error.contains("AIL-RUNTIME-BOOL:"));
        assert!(!error.contains("PRIVATE_INVALID_BOOL"));
        let elf = native(&binary, &resources(&[], &[]), &input);
        failed(&elf, "AIL-RUNTIME-BOOL:");
        assert!(!String::from_utf8_lossy(&elf.stderr).contains("PRIVATE_INVALID_BOOL"));
    }
    for (length, skip) in [(1_048_576, false), (1_048_577, false), (1_048_577, true)] {
        let expression = if skip {
            "b:true tf:input.text s:\"\" tcmp:> or"
        } else {
            "tf:input.text s:\"\" tcmp:>"
        };
        let program = raw_program(vec![
            seed("input.text", &"x".repeat(length)),
            command(
                "EVAL_BOOL",
                &[("key", "out.value"), ("expression", expression)],
            ),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new());
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        if length > 1_048_576 && !skip {
            assert!(vm.unwrap_err().contains("AIL-RUNTIME-TEXT-LIMIT:"));
            failed(&elf, "AIL-RUNTIME-TEXT-LIMIT:");
        } else {
            assert_eq!(vm.unwrap().final_state["out.value"], "true");
            assert!(elf.status.success(), "{}", brief(&elf));
            assert_eq!(elf.stdout, b"out.value=true\n");
        }
    }
}

#[test]
fn functions_promote_integer_signatures_and_skip_uninitialized_locals() {
    let functions = "Function: decide.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function executes:\n- if boolean integer local.n > 0 and integer local.n < 10:\n- the function returns 1\n- end if\n- the function returns 0\nFunction: dormant.\nThe function produces:\n- result: Bool\nThe function stores:\n- result: Bool\n- absent: Bool\n- text: Text\n- number: Int\n- floating: Float\nThe function executes:\n- the function computes boolean local.result as true or (local.absent and text local.text == \"unused\" and integer local.number / 0 == 1 and floating local.floating == nan)\n- the function returns local.result";
    let original = core(&with_functions(
        functions,
        "- the system calls decide with n = data.count into output.count\n- the system calls dormant with none into output.accept\n- the system returns output.accept\n- the system returns output.count",
    ));
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        assert_eq!(
            program.actions["decide"].instructions[0].opcode,
            "VALUE_FUNCTION"
        );
        let input = state(&[("data.count", "7")]);
        let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(vm.final_state["output.count"], "1");
        assert_eq!(vm.final_state["output.accept"], "true");
        assert!(!vm.final_state.keys().any(|k| k.starts_with("local.")));
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(elf.stdout, b"output.accept=true\noutput.count=1\n");
    }
}

#[test]
fn function_entry_validates_unused_parameters_before_lazy_body() {
    let text = with_functions(
        "Function: ignore.\nThe function needs:\n- flag: Bool\n- text: Text\nThe function produces:\n- result: Bool\nThe function stores:\n- result: Bool\nThe function executes:\n- the function computes boolean local.result as true or local.flag or text local.text == \"unused\"\n- the function returns local.result",
        "- the system calls ignore with flag = data.enabled; text = data.label into output.accept",
    );
    let program = compile_ail_core_bytecode(&core(&text)).unwrap();
    let scratch = Scratch::new();
    let binary = scratch.binary(&program, "Run");
    for (flag, category) in [("bad", "JSON"), ("true", "TEXT")] {
        let input = state(&[("data.enabled", flag)]);
        assert!(
            run_ail_bytecode_action(&program, "Run", input.clone())
                .unwrap_err()
                .contains(&format!("AIL-RUNTIME-{category}:"))
        );
        failed(
            &native(&binary, &resources(&[], &[]), &input),
            &format!("AIL-RUNTIME-{category}:"),
        );
    }
}

#[test]
fn skipped_operands_still_require_types_genuine_ownership_and_secret_permission() {
    let original = core(&source(
        "- the system computes boolean output.accept as true or data.enabled or text data.label == \"unused\" or integer data.count == 0 or floating data.score == 0",
    ));
    for (name, category) in [
        ("Data.enabled", "AIL005"),
        ("Data.label", "AIL005"),
        ("Data.count", "AIL005"),
        ("Data.score", "AIL005"),
        ("Output.accept", "AIL002"),
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name == name)
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        candidate
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        let candidate = parse_ail_core_text(&render_ail_core(&candidate)).unwrap();
        assert!(
            check_ail_core(&candidate)
                .iter()
                .any(|d| d.contains(category)),
            "{name}: {:?}",
            check_ail_core(&candidate)
        );
        assert!(compile_ail_core_ste(&candidate).is_err());
    }
    for predicate in [
        "true or data.label",
        "false and integer data.score > 0",
        "true or floating data.count == 0",
        "false and text data.enabled == \"true\"",
        "true or missing.flag",
    ] {
        let candidate = core(&source(&format!(
            "- the system computes boolean output.accept as {predicate}"
        )));
        assert!(
            compile_ail_core_bytecode(&candidate).is_err(),
            "{predicate}"
        );
    }
    let mut forged = original.clone();
    let mut fake = forged
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Field" && n.name == "Data.enabled")
        .unwrap()
        .clone();
    let original_id = fake.id.clone();
    fake.id = "forged-bool-field".into();
    fake.attributes.clear();
    forged.graph.nodes.push(fake.clone());
    forged
        .graph
        .edges
        .iter_mut()
        .find(|e| e.kind == "boolean_field" && e.target == original_id)
        .unwrap()
        .target = fake.id;
    assert!(compile_ail_core_bytecode(&forged).is_err());
}

#[test]
fn malformed_boolean_protocol_and_direct_ste_reject_eager_or_untyped_forms() {
    let original = raw_program(vec![command(
        "EVAL_BOOL",
        &[
            ("key", "out.value"),
            ("expression", "b:true bf:input.flag or"),
        ],
    )]);
    let ste = ail::ste::render_ail_ste(&original).unwrap();
    for expression in [
        "",
        "i:1",
        "b:True",
        "bf:bare",
        "b:true b:false",
        "b:true and",
        "b:true i:1 and",
        "if:input.n d:1 icmp:==",
        "tf:input.text s:\"x\" fcmp:==",
        "s:\"unterminated",
        "b:true bf:input.x if:input.x i:0 icmp:== and or",
        "b:true b:false bcmp:===",
    ] {
        let mut program = original.clone();
        program.actions.get_mut("Run").unwrap().instructions[0]
            .operands
            .insert("expression".into(), expression.into());
        assert!(!verify_ail_bytecode(&program).is_empty(), "{expression}");
        assert!(ail::ste::render_ail_ste(&program).is_err());
        assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
    }
    for (key, value) in [("extra", "ignored"), ("key", "bare")] {
        let mut program = original.clone();
        program.actions.get_mut("Run").unwrap().instructions[0]
            .operands
            .insert(key.into(), value.into());
        assert!(!verify_ail_bytecode(&program).is_empty());
    }
    for profile in ["Compiler", "UI", "AgentTool", "System"] {
        let mut program = original.clone();
        program.profile = profile.into();
        assert!(
            verify_ail_bytecode(&program)
                .iter()
                .any(|d| d.contains("AILBC018"))
        );
        assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
        assert!(
            ail::ste::parse_ail_ste(&ste.replace("\"Application\"", &format!("\"{profile}\"")))
                .is_err()
        );
    }
    for label in ["", "missing"] {
        let program = raw_program(vec![command(
            "BRANCH_BOOL",
            &[("expression", "b:true"), ("label", label)],
        )]);
        assert!(!verify_ail_bytecode(&program).is_empty());
        assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
    }
}

#[test]
fn boolean_core_rejects_shared_orphan_metadata_and_combined_raw_numeric_overflow() {
    let original = core(&source(
        "- the system computes boolean output.accept as not data.enabled and (text data.label == \"x\" or floating 1 < 2)",
    ));
    for kind in [
        "boolean_step",
        "boolean_destination",
        "boolean_expression",
        "boolean_operand",
        "boolean_left",
        "boolean_right",
        "boolean_field",
        "text_field",
    ] {
        let mut candidate = original.clone();
        let mut edge = candidate
            .graph
            .edges
            .iter()
            .find(|e| e.kind == kind)
            .unwrap()
            .clone();
        edge.id = format!("duplicate-{kind}");
        edge.ordinal = candidate.graph.edges.len();
        candidate.graph.edges.push(edge);
        assert!(compile_ail_core_ste(&candidate).is_err(), "{kind}");
    }
    for kind in ["BooleanExpression", "BooleanOperation", "TextOperand"] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == kind)
            .unwrap()
            .attributes
            .insert("extra".into(), "ignored".into());
        assert!(compile_ail_core_bytecode(&candidate).is_err(), "{kind}");
    }
    let mut orphan = original.clone();
    let mut node = orphan
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "BooleanExpression")
        .unwrap()
        .clone();
    node.id = "orphan-predicate".into();
    orphan.graph.nodes.push(node);
    assert!(compile_ail_core_bytecode(&orphan).is_err());
    let mut oversized = original.clone();
    let mut count = 0;
    for node in &mut oversized.graph.nodes {
        if node.kind == "FloatExpression"
            && node
                .attributes
                .get("expression_kind")
                .is_some_and(|k| k == "literal")
        {
            node.attributes
                .insert("value".into(), format!("{}1", "0".repeat(33_000)));
            count += 1;
        }
    }
    assert_eq!(count, 2);
    assert!(compile_ail_core_bytecode(&oversized).is_err());
    for expression in [
        format!("{}true{}", "(".repeat(129), ")".repeat(129)),
        format!("{}true", "not ".repeat(129)),
        format!("true{}", " or false".repeat(1024)),
        format!("text \"{}\" == \"\"", "x".repeat(65_536)),
    ] {
        let package = package();
        let text = source(&format!(
            "- the system computes boolean output.accept as {expression}"
        ));
        match parse_ail_package_spec_text(&package, &text) {
            Err(_) => {}
            Ok(document) => assert!(
                compile_ail_core_bytecode(&elaborate_ail_core(&package, &document)).is_err()
            ),
        }
    }
}

#[test]
fn imports_qualify_predicate_fields_but_keep_quoted_names_opaque() {
    let mut child = package();
    child.metadata.name = "bool-child".into();
    child.spec_text = source(
        "- the system computes boolean output.accept as data.enabled and text data.label == \"data.enabled Child.data.label\"\n- if boolean output.accept:\n- the system assigns true to output.selected\n- end if\n- the system returns output.accept\n- the system returns output.selected",
    );
    let mut parent = package();
    parent.spec_text="The application Parent manages records.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system returns child.output.accept\n- the system returns child.output.selected\n- the system records a trace event named Done\n".into();
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
    let program = ail::ste::parse_ail_ste(
        &compile_ail_core_ste(&parse_ail_core_text(&render_ail_core(&original)).unwrap()).unwrap(),
    )
    .unwrap();
    let input = state(&[
        ("child.data.enabled", "true"),
        ("child.data.label", "data.enabled Child.data.label"),
    ]);
    assert_eq!(
        run_ail_bytecode_action(&program, "Run", input.clone())
            .unwrap()
            .final_state["child.output.accept"],
        "true"
    );
    let scratch = Scratch::new();
    let elf = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(elf.status.success(), "{}", brief(&elf));
    assert_eq!(
        elf.stdout,
        b"child.output.accept=true\nchild.output.selected=true\n"
    );
}

#[test]
fn descriptive_bodies_reject_boolean_execution_and_recognized_malformed_prefixes() {
    let application = package();
    let compiler = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for statement in [
        "the system computes boolean output.accept as true",
        "the function computes boolean local.result as true",
        "the system computes boolean",
        "the function computes boolean",
        "if boolean true:",
        "while boolean false:",
        "if boolean",
        "while boolean",
    ] {
        let legacy = format!(
            "{DECLARATIONS}Function: legacy.\nThe function needs:\n- enabled: Bool\nThe function produces:\n- result: Bool\nWhen legacy runs:\n- {statement}\n- the function returns enabled unchanged\n"
        );
        let pass = compiler.spec_text.replace(
            "- the system reads every edge",
            &format!("- {statement}\n- the system reads every edge"),
        );
        for (package, text) in [(&application, legacy), (&compiler, pass)] {
            assert!(
                parse_ail_package_spec_text(package, &text).is_err(),
                "accepted {statement}"
            );
        }
    }
}

#[test]
fn boolean_file_decision_uses_decoded_unicode_and_exact_json_bool_output() {
    let text = source(
        "- the system reads text from file resource input into data.raw\n- the system parses JSON text data.raw into data.document\n- the system gets JSON kind from data.document into data.label\n- the system counts entries in JSON data.document into data.count\n- the system computes boolean output.accept as text data.label == \"object\" and integer data.count >= 2\n- the system formats output.accept as JSON text into output.raw\n- the system writes text output.raw to file resource report\n- the system returns output.accept",
    );
    let program = compile_ail_core_bytecode(&core(&text)).unwrap();
    let scratch = Scratch::new();
    let input = scratch.file(
        "input file=json",
        r#"{"\u0000":false,"é🙂":1e9999999999}"#.as_bytes(),
    );
    let report = scratch.path("report file=json");
    let grants = resources(&[("input", &input)], &[("report", &report)]);
    run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
    assert_eq!(fs::read(&report).unwrap(), b"true");
    let elf = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
    assert!(elf.status.success(), "{}", brief(&elf));
    assert_eq!(elf.stdout, b"output.accept=true\n");
    assert_eq!(fs::read(report).unwrap(), b"true");
}

#[test]
fn keywords_inside_dotted_field_names_are_operands_across_public_routes() {
    let text = "The application Keywords manages predicates.\nA Data has:\n- and: Bool\n- or: Bool\n- not: Bool\n- integer: Int\n- floating: Float\n- text: Text\n- boolean: Bool\nAn Output has:\n- accept: Bool\nAction: Run.\nWhen Run happens:\n- the system computes boolean output.accept as data.and or not data.or and data.not and boolean data.boolean == true and integer data.integer == 1 and floating data.floating == -0 and text data.text == \"and\"\n- the system returns output.accept\n- the system records a trace event named Done\n";
    let original = core(text);
    let scratch = Scratch::new();
    let input = state(&[
        ("data.and", "false"),
        ("data.or", "false"),
        ("data.not", "true"),
        ("data.boolean", "true"),
        ("data.integer", "1"),
        ("data.floating", "-0"),
        ("data.text", "and"),
    ]);
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        assert_eq!(
            run_ail_bytecode_action(&program, "Run", input.clone())
                .unwrap()
                .final_state["output.accept"],
            "true"
        );
        let elf = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(elf.status.success(), "{}", brief(&elf));
        assert_eq!(elf.stdout, b"output.accept=true\n");
    }
}
