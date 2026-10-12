use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application Values manages records.\n\nA Data has:\n- raw: Text\n- path: Text\n- document: Json\n- copy: Json\n- count: Int\n- index: Int\n- item: Int\n- total: Int\n- text: Text\n- flag: Bool\n- values: List<Int>\n- names: List<Text>\n\nAn Output has:\n- text: Text\n- value: Int\n- count: Int\n- document: Json\n\n";

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
            "ail-value-functions-{}-{}",
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
                "structured JSON operation blocked: {}",
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
fn text_function_local_operation_returns_through_core_ste_vm_and_native() {
    let scratch = Scratch::new();
    let original = core(&with_functions(
        "Function: double.\nThe function needs:\n- text: Text\nThe function produces:\n- result: Text\nThe function stores:\n- result: Text\nThe function executes:\n- the function joins text local.text and local.text into local.result\n- the function returns local.result",
        "- the system calls double with text = data.text into output.text\n- the system returns output.text",
    ));
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        assert_eq!(
            program.actions["double"].instructions[0].opcode,
            "VALUE_FUNCTION"
        );
        let input = state(&[("data.text", "é🙂")]);
        let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(result.final_state["output.text"], "é🙂é🙂");
        assert_eq!(result.final_state["data.text"], "é🙂");
        assert!(
            !result
                .final_state
                .keys()
                .any(|key| key.starts_with("local."))
        );
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(output.stdout, "output.text=é🙂é🙂\n".as_bytes());
    }
}

fn function(name: &str, parameters: &str, result: &str, locals: &str, body: &str) -> String {
    format!(
        "Function: {name}.\nThe function needs:\n{parameters}\nThe function produces:\n- result: {result}\nThe function stores:\n{locals}\nThe function executes:\n{body}\n"
    )
}

fn signature(parameters: &str, locals: &str, result: &str) -> AilBytecodeInstruction {
    command(
        "VALUE_FUNCTION",
        &[
            ("parameters", parameters),
            ("locals", locals),
            ("result", result),
        ],
    )
}

fn call(target: &str, arguments: &str, key: &str) -> AilBytecodeInstruction {
    command(
        "CALL_VALUE",
        &[("target", target), ("arguments", arguments), ("key", key)],
    )
}

fn returning(type_name: &str, expression: &str) -> AilBytecodeInstruction {
    command(
        "RETURN_VALUE",
        &[("type", type_name), ("expression", expression)],
    )
}

fn procedure(
    program: &mut AilBytecodeProgram,
    name: &str,
    instructions: Vec<AilBytecodeInstruction>,
) {
    program.actions.insert(
        name.into(),
        AilBytecodeAction {
            name: name.into(),
            instructions,
        },
    );
}

fn seed(key: &str, value: &str) -> AilBytecodeInstruction {
    command(
        "SET_FIELD",
        &[
            ("key", key),
            ("value", value),
            ("text", "value function fixture"),
        ],
    )
}

#[test]
fn six_types_return_normalized_values_and_copies_outlive_reused_frames() {
    let scratch = Scratch::new();
    for (type_name, field, raw, expected) in [
        ("Int", "data.item", "+00042", "42"),
        ("Bool", "data.flag", "true", "true"),
        ("Text", "data.text", "é🙂\0\u{85}", "é🙂\0\u{85}"),
        (
            "List<Int>",
            "data.values",
            " [1,-0,-9223372036854775808] ",
            "[1,0,-9223372036854775808]",
        ),
        (
            "List<Text>",
            "data.names",
            r#"["\ud83d\ude42","\u0000"]"#,
            "[\"🙂\",\"\\u0000\"]",
        ),
        (
            "Json",
            "data.document",
            r#" {"n":-0.00E+09,"n":1e999} "#,
            r#"{"n":-0.00E+09,"n":1e999}"#,
        ),
    ] {
        let functions = function(
            "identity",
            &format!("- value: {type_name}"),
            type_name,
            "- marker: Bool",
            "- the function returns local.value",
        );
        let original = core(&with_functions(
            &functions,
            &format!(
                "- the system calls identity with value = {field} into {field}\n- the system calls identity with value = {field} into {field}\n- the system returns {field}"
            ),
        ));
        for candidate in [
            original.clone(),
            parse_ail_core_text(&render_ail_core(&original)).unwrap(),
            core(&render_ail_spec_from_core(&original)),
        ] {
            let mut program =
                ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
            program
                .actions
                .get_mut("Run")
                .unwrap()
                .instructions
                .insert(0, seed(field, raw));
            let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
            assert_eq!(result.final_state[field], expected);
            let output = native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &BTreeMap::new(),
            );
            assert!(output.status.success(), "{}", brief(&output));
            assert_eq!(output.stdout, format!("{field}={expected}\n").as_bytes());
        }
    }
}

#[test]
fn general_classification_propagates_through_integer_signatures_and_preserves_pure_graphs() {
    let scratch = Scratch::new();
    let functions = [
        function("wrapper", "- n: Int", "Int", "- result: Int", "- the function calls general with n = local.n into local.result\n- the function returns local.result"),
        function("general", "- n: Int", "Int", "- text: Text\n- count: Int", "- the function assigns \"é🙂\" to local.text\n- the function counts characters in local.text into local.count\n- the function calls pure with n = local.n + local.count into local.count\n- the function returns local.count"),
        function("pure", "- n: Int", "Int", "", "- the function returns local.n * local.n"),
    ].join("\n");
    let original = core(&with_functions(
        &functions,
        "- the system calls pure with n = 3 into data.item\n- the system calls wrapper with n = data.item into output.value\n- the system returns output.value",
    ));
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        assert_eq!(
            program.actions["wrapper"].instructions[0].opcode,
            "VALUE_FUNCTION"
        );
        assert_eq!(
            program.actions["general"].instructions[0].opcode,
            "VALUE_FUNCTION"
        );
        assert_eq!(
            program.actions["pure"].instructions[0].opcode,
            "INTEGER_FUNCTION"
        );
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(result.final_state["output.value"], "121");
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(output.stdout, b"output.value=121\n");
    }
}

#[test]
fn mutual_text_recursion_preserves_parent_locals_and_emits_only_explicit_traces() {
    let scratch = Scratch::new();
    let functions = [
        function("even", "- n: Int\n- text: Text", "Text", "- child: Text\n- result: Text", "- if local.n == 0:\n- the function records a trace event named Base\n- the function returns local.text\n- end if\n- the function calls odd with text = local.text; n = local.n - 1 into local.child\n- the function joins text local.text and local.child into local.result\n- the function returns local.result\n- the function computes local.n as 1 / 0"),
        function("odd", "- n: Int\n- text: Text", "Text", "- child: Text", "- if local.n == 0:\n- the function returns local.text\n- end if\n- the function calls even with n = local.n - 1; text = local.text into local.child\n- the function returns local.child"),
    ].join("\n");
    let program = compile_ail_core_bytecode(&core(&with_functions(&functions, "- the system calls even with text = data.text; n = 4 into output.text\n- the system returns output.text"))).unwrap();
    let input = state(&[
        ("data.text", "🙂"),
        ("local.text", "forged"),
        ("local.child", "forged"),
    ]);
    let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
    assert_eq!(result.final_state["output.text"], "🙂🙂🙂");
    assert_eq!(result.final_state["local.text"], "forged");
    assert_eq!(result.final_state["local.child"], "forged");
    assert!(result.trace.contains(&"trace Base".into()));
    assert!(
        !result
            .trace
            .iter()
            .any(|event| event.contains("local.") || event.contains("value function")),
        "{:?}",
        result.trace
    );
    let output = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(output.stdout, "output.text=🙂🙂🙂\n".as_bytes());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("trace Base"));
    assert!(!stderr.contains("local."), "{stderr}");
}

#[test]
fn unused_parameters_validate_in_sorted_order_with_their_type_error_family() {
    let scratch = Scratch::new();
    for (type_name, raw, category) in [
        ("Int", "not_private_number", "AIL-RUNTIME-INT"),
        ("Bool", "True", "AIL-RUNTIME-JSON:"),
        ("List<Int>", "[1,false]", "AIL-RUNTIME-LIST:"),
        ("List<Text>", "[\"\\ud800\"]", "AIL-RUNTIME-LIST:"),
        ("Json", "{\"private\":}", "AIL-RUNTIME-JSON:"),
    ] {
        let mut program = raw_program(vec![
            call("ignore", "a=f:input.value", "out.value"),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        procedure(
            &mut program,
            "ignore",
            vec![
                signature(&format!("a:{type_name}"), "", "Int"),
                returning("Int", "i:7"),
            ],
        );
        let binary = scratch.binary(&program, "Run");
        for input in [state(&[("input.value", raw)]), BTreeMap::new()] {
            let error = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap_err();
            assert!(error.contains(category), "{type_name}: {error}");
            assert!(!error.contains("not_private_number"));
            let output = native(&binary, &resources(&[], &[]), &input);
            failed(&output, category);
            assert!(!String::from_utf8_lossy(&output.stderr).contains("not_private_number"));
        }
    }
    let mut program = raw_program(vec![call(
        "ignore",
        "a=f:input.text;z=f:input.number",
        "out.value",
    )]);
    procedure(
        &mut program,
        "ignore",
        vec![
            signature("a:Text;z:Int", "", "Int"),
            returning("Int", "i:7"),
        ],
    );
    let input = state(&[("input.number", "bad")]);
    assert!(
        run_ail_bytecode_action(&program, "Run", input.clone())
            .unwrap_err()
            .contains("AIL-RUNTIME-TEXT:")
    );
    failed(
        &native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        ),
        "AIL-RUNTIME-TEXT:",
    );
}

#[test]
fn mixed_integer_and_general_frames_share_depth_limits_and_missing_return_errors() {
    let scratch = Scratch::new();
    let functions = [
        function("general", "- n: Int", "Int", "- marker: Text\n- result: Int", "- the function calls countdown with n = local.n into local.result\n- the function returns local.result"),
        function("countdown", "- n: Int", "Int", "- result: Int", "- if local.n == 0:\n- the function returns 0\n- end if\n- the function calls countdown with n = local.n - 1 into local.result\n- the function returns local.result + 1"),
    ].join("\n");
    let program = compile_ail_core_bytecode(&core(&with_functions(&functions, "- the system calls general with n = data.item into output.value\n- the system returns output.value"))).unwrap();
    let binary = scratch.binary(&program, "Run");
    for n in [62, 63] {
        let input = state(&[("data.item", &n.to_string())]);
        let result = run_ail_bytecode_action(&program, "Run", input.clone());
        let output = native(&binary, &resources(&[], &[]), &input);
        if n == 62 {
            assert_eq!(result.unwrap().final_state["output.value"], "62");
            assert!(output.status.success(), "{}", brief(&output));
        } else {
            assert!(result.unwrap_err().contains("AIL-RUNTIME-DEPTH"));
            failed(&output, "AIL-RUNTIME-DEPTH");
        }
    }
    for (body, category) in [
        (
            vec![
                signature("", "value:Text", "Text"),
                returning("Text", "f:local.value"),
            ],
            "AIL-RUNTIME-TEXT:",
        ),
        (vec![signature("", "", "Text")], "AIL-RUNTIME-RETURN"),
        (
            vec![
                signature("", "", "Text"),
                command("LABEL", &[("name", "again")]),
                command("JUMP", &[("label", "again")]),
            ],
            "step limit",
        ),
    ] {
        let mut program = raw_program(vec![call("bad", "", "out.value")]);
        procedure(&mut program, "bad", body);
        assert!(
            run_ail_bytecode_action(&program, "Run", BTreeMap::new())
                .unwrap_err()
                .contains(category)
        );
        failed(
            &native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &BTreeMap::new(),
            ),
            category,
        );
    }
}

#[test]
fn direct_value_protocol_rejects_bad_signatures_types_scope_profiles_and_entrypoints() {
    let mut original = raw_program(vec![call("identity", "value=f:input.text", "out.text")]);
    procedure(
        &mut original,
        "identity",
        vec![
            signature("value:Text", "result:Text", "Text"),
            returning("Text", "f:local.value"),
        ],
    );
    let mut variants = Vec::new();
    for (key, value) in [
        ("parameters", "z:Text;a:Int"),
        ("parameters", "value:Text;value:Text"),
        ("parameters", "value:Decimal"),
        ("locals", "value:Text"),
        ("locals", "a:Text;bad-name:Int"),
        ("result", "List<Json>"),
        ("ignored", "metadata"),
    ] {
        let mut program = original.clone();
        program.actions.get_mut("identity").unwrap().instructions[0]
            .operands
            .insert(key.into(), value.into());
        variants.push(program);
    }
    for (key, value) in [
        ("expression", "f:local.value i:1 +"),
        ("expression", "f:input.text"),
        ("expression", "f:local.absent"),
        ("type", "Int"),
        ("extra", "ignored"),
    ] {
        let mut program = original.clone();
        program.actions.get_mut("identity").unwrap().instructions[1]
            .operands
            .insert(key.into(), value.into());
        variants.push(program);
    }
    for (key, value) in [
        ("arguments", "value=f:input.text i:1 +"),
        ("arguments", "extra=f:input.text"),
        ("arguments", "value=f:input.text;value=f:input.text"),
        ("target", "Run"),
        ("key", "not a field"),
        ("extra", "ignored"),
    ] {
        let mut program = original.clone();
        program.actions.get_mut("Run").unwrap().instructions[0]
            .operands
            .insert(key.into(), value.into());
        variants.push(program);
    }
    for instruction in [
        command(
            "FILE_READ_TEXT",
            &[("key", "local.value"), ("resource", "input")],
        ),
        command("CALL_ACTION", &[("target", "Run")]),
        command("OUTPUT_FIELD", &[("key", "local.value")]),
        command(
            "TEXT_LENGTH",
            &[("key", "local.result"), ("source", "local.value")],
        ),
        command(
            "ASSIGN_LITERAL",
            &[("key", "local.result"), ("type", "Int"), ("value", "1")],
        ),
    ] {
        let mut program = original.clone();
        program
            .actions
            .get_mut("identity")
            .unwrap()
            .instructions
            .insert(1, instruction);
        variants.push(program);
    }
    for profile in ["Compiler", "UI", "AgentTool", "System"] {
        let mut program = original.clone();
        program.profile = profile.into();
        variants.push(program);
    }
    let mut parameters = original.clone();
    parameters.actions.get_mut("identity").unwrap().instructions[0]
        .operands
        .insert(
            "locals".into(),
            (0..256)
                .map(|n| format!("v{n:03}:Int"))
                .collect::<Vec<_>>()
                .join(";"),
        );
    variants.push(parameters);
    for program in variants {
        assert!(
            !verify_ail_bytecode(&program).is_empty(),
            "accepted {:?}",
            program.actions
        );
        assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
        assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
        assert!(ail::ste::render_ail_ste(&program).is_err());
    }
    assert!(run_ail_bytecode_action(&original, "identity", BTreeMap::new()).is_err());
    assert!(compile_ail_bytecode_native_elf(&original, "identity", "linux-x86_64-elf").is_err());
    // The old protocol cannot bypass source call-graph promotion.
    let mut mixed = original.clone();
    procedure(
        &mut mixed,
        "integer",
        vec![
            command("INTEGER_FUNCTION", &[("parameters", "")]),
            call("identity", "value=f:local.text", "local.result"),
            command("RETURN_INT", &[("expression", "i:0")]),
        ],
    );
    assert!(!verify_ail_bytecode(&mixed).is_empty());
}

#[test]
fn source_value_functions_reject_global_storage_effects_and_wrong_typed_expressions() {
    for body in [
        "- the function returns data.text",
        "- the function returns local.text + 1",
        "- the function returns local.missing",
        "- the function joins text local.text and data.text into local.result\n- the function returns local.result",
        "- the function joins text local.text and local.text into data.text\n- the function returns local.text",
        "- the system joins text local.text and local.text into local.result\n- the function returns local.result",
        "- the function reads text from file resource input into local.result\n- the function returns local.result",
        "- the function calls Run\n- the function returns local.text",
        "- the function assigns 1 to local.result\n- the function returns local.result",
    ] {
        let text = with_functions(
            &function("bad", "- text: Text", "Text", "- result: Text", body),
            "- the system calls bad with text = data.text into output.text",
        );
        match parse_ail_package_spec_text(&package(), &text) {
            Err(_) => {}
            Ok(document) => assert!(
                compile_ail_core_bytecode(&elaborate_ail_core(&package(), &document)).is_err(),
                "accepted {body}"
            ),
        }
    }
}

#[test]
fn value_core_checks_owned_fields_steps_metadata_and_secret_call_operands() {
    let functions = function(
        "copy",
        "- n: Int\n- text: Text",
        "Text",
        "- result: Text",
        "- the function joins text local.text and local.text into local.result\n- the function returns local.result",
    );
    let original = core(&with_functions(
        &functions,
        "- the system calls copy with n = data.item + 1; text = data.text into output.text",
    ));
    for profile in ["Compiler", "UI", "AgentTool", "System"] {
        let mut candidate = original.clone();
        candidate.package.profile = profile.into();
        assert!(compile_ail_core_bytecode(&candidate).is_err(), "{profile}");
    }
    let mut conflicting = original.clone();
    conflicting
        .graph
        .nodes
        .iter_mut()
        .find(|node| node.kind == "Function")
        .unwrap()
        .attributes
        .insert("executable_integer".into(), "true".into());
    assert!(compile_ail_core_bytecode(&conflicting).is_err());
    for kind in [
        "value_parameter",
        "value_local",
        "value_function_step",
        "value_field_operand",
    ] {
        let mut candidate = original.clone();
        let edge = candidate
            .graph
            .edges
            .iter()
            .find(|edge| edge.kind == kind)
            .unwrap()
            .clone();
        candidate.graph.edges.push(edge);
        assert!(
            compile_ail_core_bytecode(&parse_ail_core_text(&render_ail_core(&candidate)).unwrap())
                .is_err(),
            "duplicate {kind}"
        );
    }
    for field in ["text", "result"] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| {
                node.kind == "Field"
                    && node
                        .attributes
                        .get("local_name")
                        .is_some_and(|name| name == field)
            })
            .unwrap()
            .attributes
            .insert("function_owner".into(), "Run".into());
        assert!(compile_ail_core_bytecode(&candidate).is_err());
    }
    let mut candidate = original.clone();
    candidate
        .graph
        .nodes
        .iter_mut()
        .find(|node| node.kind == "ValueFunctionStep")
        .unwrap()
        .attributes
        .insert("ignored".into(), "body prose".into());
    assert!(compile_ail_core_bytecode(&candidate).is_err());
    for (field_name, code) in [
        ("Data.item", "AIL005"),
        ("Data.text", "AIL005"),
        ("Output.text", "AIL002"),
    ] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|node| node.kind == "Field" && node.name == field_name)
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        candidate
            .graph
            .edges
            .retain(|edge| !matches!(edge.kind.as_str(), "reads" | "writes"));
        let candidate = parse_ail_core_text(&render_ail_core(&candidate)).unwrap();
        assert!(
            check_ail_core(&candidate)
                .iter()
                .any(|diagnostic| diagnostic.contains(code)),
            "{field_name}: {:?}",
            check_ail_core(&candidate)
        );
        assert!(compile_ail_core_bytecode(&candidate).is_err());
    }
}

#[test]
fn imports_namespace_function_targets_and_globals_without_changing_local_keys() {
    let scratch = Scratch::new();
    let mut child = package();
    child.metadata.name = "value-child".into();
    child.spec_text = with_functions(
        &function(
            "copy",
            "- text: Text",
            "Text",
            "",
            "- the function returns local.text",
        ),
        "- the system calls copy with text = data.text into output.text\n- the system returns output.text",
    );
    let mut parent = package();
    parent.spec_text = "The application Parent manages values.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system returns child.output.text\n- the system records a trace event named Done\n".into();
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
    assert_eq!(
        program.actions["Child.copy"].instructions[0].operands["parameters"],
        "text:Text"
    );
    let input = state(&[("child.data.text", "local.text;data.text🙂")]);
    let result = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
    assert_eq!(
        result.final_state["child.output.text"],
        "local.text;data.text🙂"
    );
    let output = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(
        output.stdout,
        "child.output.text=local.text;data.text🙂\n".as_bytes()
    );
}

#[test]
fn borrowed_list_parameters_can_expand_beyond_canonical_bound_until_returned() {
    let scratch = Scratch::new();
    let raw = format!("[\"{}\"]", "\u{7f}".repeat(180_000));
    for (type_name, returned, category) in [
        ("List<Text>", false, None),
        ("List<Text>", true, Some("AIL-RUNTIME-LIST-LIMIT")),
        ("Json", false, Some("AIL-RUNTIME-JSON-LIMIT")),
    ] {
        let mut program = raw_program(vec![
            seed("input.value", &raw),
            call("inspect", "value=f:input.value", "out.value"),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        procedure(
            &mut program,
            "inspect",
            vec![
                signature(
                    &format!("value:{type_name}"),
                    "",
                    if returned { type_name } else { "Int" },
                ),
                returning(
                    if returned { type_name } else { "Int" },
                    if returned { "f:local.value" } else { "i:7" },
                ),
            ],
        );
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new());
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        if let Some(category) = category {
            assert!(result.unwrap_err().contains(category));
            failed(&output, category);
        } else {
            assert_eq!(result.unwrap().final_state["out.value"], "7");
            assert!(output.status.success(), "{}", brief(&output));
            assert_eq!(output.stdout, b"out.value=7\n");
        }
    }
}

#[test]
fn return_copies_charge_once_per_invocation_and_share_caller_arenas() {
    let scratch = Scratch::new();
    for (type_name, raw, category) in [
        ("Text", "x".repeat(1_048_576), "AIL-RUNTIME-TEXT-LIMIT"),
        (
            "List<Text>",
            format!("[\"{}\"]", "x".repeat(1_048_572)),
            "AIL-RUNTIME-LIST-LIMIT",
        ),
        (
            "Json",
            format!("\"{}\"", "x".repeat(1_048_574)),
            "AIL-RUNTIME-JSON-LIMIT",
        ),
    ] {
        for extra in [false, true] {
            let mut steps = vec![seed("input.value", &raw)];
            // Each forwarding invocation performs two return copies. Parameter
            // validation and the caller's destination write must remain free.
            steps.extend((0..32).map(|_| call("forward", "value=f:input.value", "out.value")));
            if extra {
                steps.push(call("identity", "value=f:input.value", "out.value"));
            }
            steps.extend([
                command(
                    "TEXT_LENGTH",
                    &[("key", "out.count"), ("source", "out.value")],
                ),
                command("OUTPUT_FIELD", &[("key", "out.count")]),
            ]);
            let mut program = raw_program(steps);
            procedure(
                &mut program,
                "identity",
                vec![
                    signature(&format!("value:{type_name}"), "", type_name),
                    returning(type_name, "f:local.value"),
                ],
            );
            procedure(
                &mut program,
                "forward",
                vec![
                    signature(
                        &format!("value:{type_name}"),
                        &format!("result:{type_name}"),
                        type_name,
                    ),
                    call("identity", "value=f:local.value", "local.result"),
                    returning(type_name, "f:local.result"),
                ],
            );
            let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new());
            let output = native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &BTreeMap::new(),
            );
            if extra {
                assert!(result.unwrap_err().contains(category), "{type_name}");
                failed(&output, category);
            } else {
                assert_eq!(result.unwrap().final_state["out.count"], "1048576");
                assert!(output.status.success(), "{type_name}: {}", brief(&output));
                assert_eq!(output.stdout, b"out.count=1048576\n");
            }
        }
    }
}

#[test]
fn typed_functions_transform_nested_file_records_through_all_six_routes() {
    let scratch = Scratch::new();
    let input = scratch.file("input.json", br#"[{"value":3,"private":"remove","names":["a","b"]},{"value":-2,"private":"remove","names":["\ud83d\ude42"]}]"#);
    let report = scratch.path("report.json");
    let grants = resources(&[("input", &input)], &[("report", &report)]);
    let functions = [
        function("square", "- n: Int", "Int", "", "- the function returns local.n * local.n"),
        function("labels", "- names: List<Text>", "List<Text>", "- first: Text\n- result: List<Text>", "- the function gets item 0 from list local.names into local.first\n- the function joins text local.first and local.first into local.first\n- the function sets item 0 in list local.names to text local.first into local.result\n- the function appends text local.first to list local.result into local.result\n- the function returns local.result"),
        function("transform", "- document: Json", "Json", "- result: Json\n- path: Text\n- value: Int\n- names: List<Text>\n- raw: Text\n- count: Int", "- the function assigns \"/value\" to local.path\n- the function gets JSON value at path local.path from local.document into local.value\n- the function calls square with n = local.value into local.value\n- the function sets JSON value at path local.path in local.document to local.value into local.result\n- the function assigns \"/private\" to local.path\n- the function removes JSON value at path local.path from local.result into local.result\n- the function assigns \"/names\" to local.path\n- the function gets JSON value at path local.path from local.result into local.names\n- the function calls labels with names = local.names into local.names\n- the function sets JSON value at path local.path in local.result to local.names into local.result\n- the function counts entries in JSON local.result into local.count\n- the function formats local.result as JSON text into local.raw\n- the function parses JSON text local.raw into local.result\n- the function returns local.result"),
    ].join("\n");
    let body = "- the system reads text from file resource input into data.raw\n- the system parses JSON text data.raw into data.document\n- the system counts entries in JSON data.document into data.count\n- the system assigns [] to output.document\n- the system assigns 0 to data.index\n- the system assigns \"/\" to data.text\n- while data.index < data.count:\n- the system formats data.index as JSON text into data.raw\n- the system joins text data.text and data.raw into data.path\n- the system gets JSON value at path data.path from data.document into data.copy\n- the system calls transform with document = data.copy into data.copy\n- the system assigns \"/-\" to data.path\n- the system sets JSON value at path data.path in output.document to data.copy into output.document\n- the system computes data.index as data.index + 1\n- end while\n- the system formats output.document as JSON text into output.text\n- the system writes text output.text to file resource report\n- the system returns data.count";
    let original = core(&with_functions(&functions, body));
    let expected = "[{\"value\":9,\"names\":[\"aa\",\"b\",\"aa\"]},{\"value\":4,\"names\":[\"🙂🙂\",\"🙂🙂\"]}]";
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let result =
            run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
        assert_eq!(result.final_state["data.count"], "2");
        assert_eq!(fs::read(&report).unwrap(), expected.as_bytes());
        fs::remove_file(&report).unwrap();
        let output = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(output.stdout, b"data.count=2\n");
        assert_eq!(fs::read(&report).unwrap(), expected.as_bytes());
    }
}

#[test]
fn cli_bundles_keep_value_functions_as_dependencies_and_reject_direct_entry() {
    let scratch = Scratch::new();
    let original = core(&with_functions(
        &function(
            "identity",
            "- text: Text",
            "Text",
            "",
            "- the function returns local.text",
        ),
        "- the system calls identity with text = data.text into output.text\n- the system returns output.text",
    ));
    let program = compile_ail_core_bytecode(&original).unwrap();
    let input = scratch.file(
        "program.ailbc.json",
        render_ail_bytecode(&program).as_bytes(),
    );
    let bundle = scratch.path("bundle");
    let result = Command::new(env!("CARGO_BIN_EXE_ail"))
        .arg("ail-compile")
        .arg(&input)
        .args([
            "--all-actions",
            "--target",
            "linux-x86_64-elf",
            "--artifact-dir",
        ])
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", brief(&result));
    assert!(bundle.join("target-Run.elf").exists());
    assert!(!bundle.join("target-identity.elf").exists());
    let output = native(
        &bundle.join("target-Run.elf"),
        &resources(&[], &[]),
        &state(&[("data.text", "Zoë 🙂")]),
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(output.stdout, "output.text=Zoë 🙂\n".as_bytes());
    let result = Command::new(env!("CARGO_BIN_EXE_ail"))
        .arg("ail-vm")
        .arg(&input)
        .args(["--action", "Run", "data.text=hello"])
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", brief(&result));
    assert!(String::from_utf8_lossy(&result.stdout).contains("output.text=hello"));
    let rejected = Command::new(env!("CARGO_BIN_EXE_ail"))
        .arg("ail-vm")
        .arg(&input)
        .args(["--action", "identity"])
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("require an action caller"));
}

#[test]
fn value_callee_traces_are_included_in_wasm_contract_reports() {
    let scratch = Scratch::new();
    let original = core(&with_functions(
        &function(
            "identity",
            "- text: Text",
            "Text",
            "",
            "- the function records a trace event named Called\n- the function returns local.text",
        ),
        "- the system calls identity with text = data.text into output.text",
    ));
    let mut program = compile_ail_core_bytecode(&original).unwrap();
    program
        .target_support
        .insert("wasm32-unknown-sandbox-wasm".into(), "supported".into());
    program
        .actions
        .get_mut("Run")
        .unwrap()
        .instructions
        .retain(|i| i.opcode != "EMIT_TRACE");
    let input = scratch.file(
        "program.ailbc.json",
        render_ail_bytecode(&program).as_bytes(),
    );
    let bundle = scratch.path("bundle");
    let result = Command::new(env!("CARGO_BIN_EXE_ail"))
        .arg("ail-compile")
        .arg(input)
        .args([
            "--action",
            "Run",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
        ])
        .arg(&bundle)
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", brief(&result));
    assert!(
        fs::read_to_string(bundle.join("wasm-contract-report.txt"))
            .unwrap()
            .contains("trace-preservation required")
    );
}
