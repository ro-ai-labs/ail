use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application Recovery manages records.\n\nA Data has:\n- raw: Text\n- error: Text\n- other: Text\n- path: Text\n- value: Int\n- count: Int\n- document: Json\n- values: List<Int>\n- floating: Float\n- flag: Bool\n\nAn Output has:\n- value: Int\n- count: Int\n- raw: Text\n- error: Text\n\n";

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
            "ail-runtime-recovery-{}-{}",
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
    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            let _ = stdout_reader.join().unwrap();
            let stderr = stderr_reader.join().unwrap();
            panic!(
                "Recovery execution blocked: {}",
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
fn recovery_runs_through_source_saved_core_rendered_ste_vm_and_native() {
    let original = core(&source(
        "- the system assigns \"bad\" to data.raw\n- the system assigns 99 to output.value\n- the system assigns \"unchanged\" to data.error\n- try:\n- the system parses JSON text data.raw into output.value\n- the system assigns 100 to output.value\n- recover JSON or INT into data.error:\n- the system increments output.value by 1\n- end try\n- the system returns output.value\n- the system returns data.error",
    ));
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let ste = compile_ail_core_ste(&candidate).unwrap();
        assert!(ste.contains("Begin recovery for errors"));
        let program = ail::ste::parse_ail_ste(&ste).unwrap();
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(vm.final_state["output.value"], "100");
        assert_eq!(vm.final_state["data.error"], "AIL-RUNTIME-JSON");
        assert!(
            vm.trace
                .iter()
                .any(|line| line == "recovered JSON into data.error")
        );
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(
            output.stdout,
            b"data.error=AIL-RUNTIME-JSON\noutput.value=100\n"
        );
        assert!(!String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-JSON:"));
    }
}

fn seed(key: &str, value: &str) -> AilBytecodeInstruction {
    command(
        "SET_FIELD",
        &[("key", key), ("value", value), ("text", "fixture")],
    )
}
fn begin(errors: &str, key: &str) -> AilBytecodeInstruction {
    command("TRY_BEGIN", &[("errors", errors), ("key", key)])
}
fn recover() -> AilBytecodeInstruction {
    command("RECOVER", &[])
}
fn end() -> AilBytecodeInstruction {
    command("TRY_END", &[])
}
fn routine(program: &mut AilBytecodeProgram, name: &str, body: Vec<AilBytecodeInstruction>) {
    program.actions.insert(
        name.into(),
        AilBytecodeAction {
            name: name.into(),
            instructions: body,
        },
    );
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
fn output(key: &str) -> AilBytecodeInstruction {
    command("OUTPUT_FIELD", &[("key", key)])
}
fn assert_success(program: &AilBytecodeProgram, expected: &[(&str, &str)]) {
    let vm = run_ail_bytecode_action(program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(vm.status, "succeeded");
    let expected = state(expected);
    assert_eq!(vm.output_state(), expected);
    let scratch = Scratch::new();
    let result = native(
        &scratch.binary(program, "Run"),
        &resources(&[], &[]),
        &BTreeMap::new(),
    );
    assert!(result.status.success(), "{}", brief(&result));
    assert_eq!(
        result.stdout,
        expected
            .iter()
            .map(|(key, value)| format!("{key}={value}\n"))
            .collect::<String>()
            .as_bytes()
    );
}

#[test]
fn every_recoverable_family_is_static_silent_and_preserves_failed_destination() {
    let cases = [
        (
            "INT",
            command(
                "EVAL_INT",
                &[("key", "out.value"), ("expression", "f:missing.integer")],
            ),
        ),
        (
            "OVERFLOW",
            command(
                "EVAL_INT",
                &[
                    ("key", "out.value"),
                    ("expression", "i:9223372036854775807 i:1 +"),
                ],
            ),
        ),
        (
            "DIVZERO",
            command(
                "EVAL_INT",
                &[("key", "out.value"), ("expression", "i:1 i:0 /")],
            ),
        ),
        (
            "FLOAT",
            command(
                "EVAL_FLOAT",
                &[("key", "out.value"), ("expression", "f:missing.floating")],
            ),
        ),
        (
            "BOOL",
            command(
                "EVAL_BOOL",
                &[("key", "out.value"), ("expression", "bf:missing.flag")],
            ),
        ),
        (
            "TEXT",
            command(
                "TEXT_TRIM",
                &[("key", "out.value"), ("source", "missing.text")],
            ),
        ),
        (
            "LIST",
            command(
                "LIST_LENGTH",
                &[
                    ("key", "out.value"),
                    ("source", "data.bad"),
                    ("type", "Int"),
                ],
            ),
        ),
        (
            "INDEX",
            command(
                "LIST_GET",
                &[
                    ("key", "out.value"),
                    ("source", "data.empty"),
                    ("index", "i:0"),
                    ("type", "Int"),
                ],
            ),
        ),
        (
            "JSON",
            command(
                "JSON_PARSE",
                &[
                    ("key", "out.value"),
                    ("source", "data.bad"),
                    ("type", "Int"),
                ],
            ),
        ),
        (
            "JSON-PATH",
            command(
                "JSON_GET",
                &[
                    ("key", "out.value"),
                    ("source", "data.document"),
                    ("path", "data.path"),
                    ("type", "Int"),
                ],
            ),
        ),
    ];
    for (category, operation) in cases {
        for filter in [category, "any"] {
            let program = raw_program(vec![
                seed("out.value", "sentinel"),
                seed("out.error", "old"),
                seed("data.bad", "PRIVATE_INVALID"),
                seed("data.empty", "[]"),
                seed("data.document", "{}"),
                seed("data.path", "/missing"),
                begin(filter, "out.error"),
                operation.clone(),
                seed("out.value", "wrong-normal-tail"),
                recover(),
                end(),
                output("out.value"),
                output("out.error"),
            ]);
            assert_success(
                &program,
                &[
                    ("out.error", &format!("AIL-RUNTIME-{category}")),
                    ("out.value", "sentinel"),
                ],
            );
        }
    }
    let program = raw_program(vec![
        seed("out.error", "old"),
        seed("out.value", "8"),
        begin("any", "out.error"),
        command(
            "ADD_INT_FIELD",
            &[
                ("key", "missing.integer"),
                ("delta", "1"),
                ("text", "increment"),
            ],
        ),
        recover(),
        end(),
        output("out.error"),
        output("out.value"),
    ]);
    assert_success(
        &program,
        &[("out.error", "AIL-RUNTIME-INT"), ("out.value", "8")],
    );
}

#[test]
fn normal_fallthrough_nested_nonmatching_and_handler_errors_choose_live_outer_scope() {
    let normal = raw_program(vec![
        seed("out.error", "unchanged"),
        seed("out.value", "1"),
        begin("any", "out.error"),
        seed("out.value", "2"),
        recover(),
        seed("out.value", "wrong"),
        end(),
        output("out.error"),
        output("out.value"),
    ]);
    assert_success(&normal, &[("out.error", "unchanged"), ("out.value", "2")]);
    let nested = raw_program(vec![
        seed("out.inner", "old-inner"),
        seed("out.value", "prior"),
        begin("INT", "out.error"),
        begin("JSON", "out.inner"),
        command(
            "EVAL_INT",
            &[("key", "out.value"), ("expression", "f:missing.number")],
        ),
        recover(),
        seed("out.value", "wrong-inner"),
        end(),
        recover(),
        seed("out.value", "outer"),
        end(),
        output("out.error"),
        output("out.inner"),
        output("out.value"),
    ]);
    assert_success(
        &nested,
        &[
            ("out.error", "AIL-RUNTIME-INT"),
            ("out.inner", "old-inner"),
            ("out.value", "outer"),
        ],
    );
    let handler_error = raw_program(vec![
        seed("out.value", "prior"),
        begin("DIVZERO", "out.error"),
        begin("JSON", "out.inner"),
        seed("data.raw", "bad"),
        command(
            "JSON_PARSE",
            &[
                ("key", "out.value"),
                ("source", "data.raw"),
                ("type", "Int"),
            ],
        ),
        recover(),
        command(
            "EVAL_INT",
            &[("key", "out.value"), ("expression", "i:7 i:0 /")],
        ),
        end(),
        recover(),
        end(),
        output("out.error"),
        output("out.inner"),
        output("out.value"),
    ]);
    assert_success(
        &handler_error,
        &[
            ("out.error", "AIL-RUNTIME-DIVZERO"),
            ("out.inner", "AIL-RUNTIME-JSON"),
            ("out.value", "prior"),
        ],
    );
}

#[test]
fn caught_action_call_keeps_completed_writes_traces_and_restores_output_selection() {
    let mut program = raw_program(vec![
        seed("out.value", "1"),
        output("out.value"),
        begin("INT", "out.error"),
        command("CALL_ACTION", &[("target", "Child")]),
        recover(),
        end(),
        output("out.error"),
    ]);
    routine(
        &mut program,
        "Child",
        vec![
            seed("out.value", "44"),
            seed("out.child", "hidden-selection"),
            output("out.child"),
            command("EMIT_TRACE", &[("event", "BeforeFailure")]),
            command(
                "EVAL_INT",
                &[("key", "out.value"), ("expression", "f:missing.integer")],
            ),
            command("RETURN_SUCCESS", &[]),
        ],
    );
    let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(vm.final_state["out.child"], "hidden-selection");
    assert!(vm.trace.iter().any(|event| event.contains("BeforeFailure")));
    assert_success(
        &program,
        &[("out.error", "AIL-RUNTIME-INT"), ("out.value", "44")],
    );
}

#[test]
fn function_return_errors_recover_in_frame_and_failed_calls_do_not_commit_destination() {
    let functions = "Function: safe.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function stores:\n- error: Text\nThe function executes:\n- try:\n- the function returns 10 / local.n\n- recover DIVZERO into local.error:\n- the function returns 7\n- end try\nFunction: fail.\nThe function produces:\n- result: Int\nThe function executes:\n- the function returns 1 / 0";
    let original = core(&with_functions(
        functions,
        "- the system calls safe with n = 0 into output.value\n- try:\n- the system calls fail with none into output.value\n- recover DIVZERO into data.error:\n- the system increments output.value by 1\n- end try\n- the system returns output.value\n- the system returns data.error",
    ));
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(vm.final_state["output.value"], "8");
        assert_eq!(vm.final_state["data.error"], "AIL-RUNTIME-DIVZERO");
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(
            output.stdout,
            b"data.error=AIL-RUNTIME-DIVZERO\noutput.value=8\n"
        );
    }
}

#[test]
fn text_return_validation_recovers_before_leaving_the_local_frame() {
    let functions = "Function: textFallback.\nThe function produces:\n- result: Text\nThe function stores:\n- missing: Text\n- value: Text\n- error: Text\nThe function executes:\n- try:\n- the function returns local.missing\n- recover TEXT into local.error:\n- the function assigns \"recovered λ\\u0000\" to local.value\n- the function returns local.value\n- end try";
    let original = core(&with_functions(
        functions,
        "- the system calls textFallback with none into output.raw\n- the system returns output.raw",
    ));
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        assert_success(&program, &[("output.raw", "recovered λ\0")]);
    }
}

#[test]
fn declared_rule_failure_keeps_failed_status_and_never_enters_runtime_recovery() {
    let mut program = raw_program(vec![
        seed("out.error", "unchanged"),
        seed("out.value", "41"),
        begin("any", "out.error"),
        command(
            "REQUIRE_EXISTS",
            &[
                ("key", "missing.required"),
                ("rule", "the required field to exist"),
                ("failure", "MissingRequired"),
            ],
        ),
        recover(),
        seed("out.value", "incorrect handler"),
        end(),
        output("out.value"),
    ]);
    program.failures.insert(
        "MissingRequired".into(),
        AilBytecodeFailure {
            name: "MissingRequired".into(),
            traces: vec!["DeclaredFailureTrace".into()],
        },
    );
    let program = ail::ste::parse_ail_ste(&ail::ste::render_ail_ste(&program).unwrap()).unwrap();
    let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(vm.status, "failed");
    assert_eq!(vm.failure.as_deref(), Some("MissingRequired"));
    assert_eq!(vm.final_state["out.error"], "unchanged");
    assert_eq!(vm.final_state["out.value"], "41");
    assert!(vm.returned_fields.is_empty());
    assert!(
        vm.trace
            .iter()
            .any(|line| line == "trace DeclaredFailureTrace")
    );
    assert!(!vm.trace.iter().any(|line| line.starts_with("recovered ")));
    let scratch = Scratch::new();
    let native = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &BTreeMap::new(),
    );
    failed(&native, "failure MissingRequired");
    let trace = String::from_utf8_lossy(&native.stderr);
    assert!(
        trace.contains("trace DeclaredFailureTrace"),
        "{}",
        brief(&native)
    );
    assert!(!trace.contains("incorrect handler"), "{}", brief(&native));
}

#[test]
fn break_continue_and_valid_jumps_remove_scopes_and_reentry_installs_new_handlers() {
    let text = source(
        "- the system assigns 0 to data.count\n- the system assigns 0 to output.value\n- while data.count < 5:\n- the system increments data.count by 1\n- try:\n- if data.count == 1:\n- continue\n- end if\n- if data.count == 4:\n- break\n- end if\n- the system computes data.value as 1 / 0\n- recover DIVZERO into data.error:\n- the system increments output.value by 1\n- end try\n- end while\n- the system returns output.value\n- the system returns data.count",
    );
    let program = compile_ail_core_bytecode(&core(&text)).unwrap();
    let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(vm.final_state["output.value"], "2");
    assert_eq!(vm.final_state["data.count"], "4");
    let scratch = Scratch::new();
    let output = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &BTreeMap::new(),
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(output.stdout, b"data.count=4\noutput.value=2\n");
    let jump = raw_program(vec![
        seed("out.error", "old"),
        begin("INT", "out.error"),
        command("JUMP", &[("label", "outside")]),
        recover(),
        seed("out.error", "wrong"),
        end(),
        command("LABEL", &[("name", "outside")]),
        command(
            "EVAL_INT",
            &[("key", "out.value"), ("expression", "f:missing.integer")],
        ),
    ]);
    assert!(
        run_ail_bytecode_action(&jump, "Run", BTreeMap::new())
            .unwrap_err()
            .contains("AIL-RUNTIME-INT:")
    );
    failed(
        &native(
            &scratch.binary(&jump, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        ),
        "AIL-RUNTIME-INT:",
    );
}

#[test]
fn any_does_not_catch_limits_capability_io_return_depth_or_fuel_failures() {
    let scratch = Scratch::new();
    for (operation, setup, category) in [
        (
            command("TEXT_TRIM", &[("key", "out.value"), ("source", "data.raw")]),
            vec![seed("data.raw", &"x".repeat(1_048_577))],
            "TEXT-LIMIT",
        ),
        (
            command(
                "LIST_LENGTH",
                &[
                    ("key", "out.value"),
                    ("source", "data.raw"),
                    ("type", "Int"),
                ],
            ),
            vec![seed(
                "data.raw",
                &format!("[{}]", vec!["0"; 65_537].join(",")),
            )],
            "LIST-LIMIT",
        ),
        (
            command(
                "JSON_PARSE",
                &[
                    ("key", "out.value"),
                    ("source", "data.raw"),
                    ("type", "Json"),
                ],
            ),
            vec![seed("data.raw", &" ".repeat(1_048_577))],
            "JSON-LIMIT",
        ),
        (
            command(
                "EVAL_FLOAT",
                &[("key", "out.value"), ("expression", "f:data.raw")],
            ),
            vec![seed("data.raw", &"1".repeat(1_048_577))],
            "FLOAT-LIMIT",
        ),
        (
            command(
                "FILE_READ_TEXT",
                &[("key", "out.value"), ("resource", "input")],
            ),
            vec![],
            "CAPABILITY",
        ),
    ] {
        let mut steps = setup;
        steps.extend([
            begin("any", "out.error"),
            operation,
            recover(),
            seed("out.value", "wrong-catch"),
            end(),
            output("out.value"),
        ]);
        let program = raw_program(steps);
        let marker = format!("AIL-RUNTIME-{category}:");
        assert!(
            run_ail_bytecode_action(&program, "Run", BTreeMap::new())
                .unwrap_err()
                .contains(&marker)
        );
        failed(
            &native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &BTreeMap::new(),
            ),
            &marker,
        );
    }
    for recursive in [false, true] {
        let mut program = raw_program(vec![
            begin("any", "out.error"),
            command(
                "CALL_VALUE",
                &[("key", "out.value"), ("target", "bad"), ("arguments", "")],
            ),
            recover(),
            seed("out.value", "caught"),
            end(),
            output("out.value"),
        ]);
        let mut body = vec![signature("", "value:Text", "Text")];
        if recursive {
            body.push(command(
                "CALL_VALUE",
                &[("key", "local.value"), ("target", "bad"), ("arguments", "")],
            ));
            body.push(command(
                "RETURN_VALUE",
                &[("type", "Text"), ("expression", "f:local.value")],
            ));
        }
        routine(&mut program, "bad", body);
        let marker = if recursive {
            "AIL-RUNTIME-DEPTH:"
        } else {
            "AIL-RUNTIME-RETURN:"
        };
        assert!(
            run_ail_bytecode_action(&program, "Run", BTreeMap::new())
                .unwrap_err()
                .contains(marker)
        );
        failed(
            &native(
                &scratch.binary(&program, "Run"),
                &resources(&[], &[]),
                &BTreeMap::new(),
            ),
            marker,
        );
    }
    let program = raw_program(vec![
        command("LABEL", &[("name", "again")]),
        begin("any", "out.error"),
        command(
            "EVAL_INT",
            &[("key", "out.value"), ("expression", "i:1 i:0 /")],
        ),
        recover(),
        end(),
        command("JUMP", &[("label", "again")]),
    ]);
    assert!(
        run_ail_bytecode_action(&program, "Run", BTreeMap::new())
            .unwrap_err()
            .contains("step limit")
    );
    failed(
        &native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &BTreeMap::new(),
        ),
        "step limit",
    );
    let program = raw_program(vec![
        begin("any", "out.error"),
        command(
            "FILE_READ_TEXT",
            &[("key", "out.value"), ("resource", "input")],
        ),
        recover(),
        end(),
    ]);
    let missing = scratch.path("absent-file");
    let grants = resources(&[("input", &missing)], &[]);
    assert!(
        run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants)
            .unwrap_err()
            .contains("AIL-RUNTIME-IO:")
    );
    failed(
        &native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new()),
        "AIL-RUNTIME-IO:",
    );
}

#[test]
fn earlier_file_effects_remain_visible_after_a_caught_failure() {
    let scratch = Scratch::new();
    let input = scratch.file("input.json", b"not-json");
    let report = scratch.path("report.txt");
    let grants = resources(&[("input", &input)], &[("report", &report)]);
    let text = source(
        "- the system assigns \"prior effect\" to output.raw\n- the system writes text output.raw to file resource report\n- try:\n- the system reads text from file resource input into data.raw\n- the system parses JSON text data.raw into data.value\n- recover JSON into data.error:\n- the system assigns 9 to output.value\n- end try\n- the system returns output.value",
    );
    let program = compile_ail_core_bytecode(&core(&text)).unwrap();
    let vm = run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
    assert_eq!(vm.final_state["output.value"], "9");
    assert_eq!(fs::read(&report).unwrap(), b"prior effect");
    let output = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(fs::read(report).unwrap(), b"prior effect");
}

#[test]
fn malformed_protocol_and_cross_region_jumps_are_rejected_before_execution() {
    let mut maximal = vec![begin("INT", "out.error"); 64];
    maximal.push(command(
        "EVAL_INT",
        &[("key", "out.value"), ("expression", "f:missing.integer")],
    ));
    for _ in 0..64 {
        maximal.extend([recover(), end()]);
    }
    maximal.push(output("out.error"));
    let mut valid = raw_program(maximal);
    for profile in ["Application", "C interop"] {
        valid.profile = profile.into();
        assert_success(&valid, &[("out.error", "AIL-RUNTIME-INT")]);
    }
    let mut invalid = vec![
        vec![recover()],
        vec![end()],
        vec![begin("any", "out.error")],
        vec![begin("any", "out.error"), end()],
        vec![begin("any", "out.error"), recover(), recover(), end()],
        vec![
            command("JUMP", &[("label", "inside")]),
            begin("any", "out.error"),
            command("LABEL", &[("name", "inside")]),
            recover(),
            end(),
        ],
        vec![
            begin("any", "out.error"),
            command("JUMP", &[("label", "handler")]),
            recover(),
            command("LABEL", &[("name", "handler")]),
            end(),
        ],
        vec![
            begin("any", "out.error"),
            command("JUMP", &[("label", "sibling")]),
            recover(),
            end(),
            begin("any", "out.error"),
            command("LABEL", &[("name", "sibling")]),
            recover(),
            end(),
        ],
    ];
    for errors in [
        "",
        "int",
        "INT;INT",
        "JSON;INT",
        "any;JSON",
        "INT or JSON",
        "TEXT-LIMIT",
        "IO",
        "FUEL",
    ] {
        invalid.push(vec![begin(errors, "out.error"), recover(), end()]);
    }
    invalid.push(vec![begin("any", "bare"), recover(), end()]);
    let mut depth = vec![begin("any", "out.error"); 65];
    for _ in 0..65 {
        depth.extend([recover(), end()]);
    }
    invalid.push(depth);
    for (index, steps) in invalid.into_iter().enumerate() {
        let program = raw_program(steps);
        assert!(
            verify_ail_bytecode(&program)
                .iter()
                .any(|line| line.contains("AILBC019")),
            "case {index}: {:?}",
            verify_ail_bytecode(&program)
        );
        assert!(ail::ste::render_ail_ste(&program).is_err());
        assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
    }
    let original = raw_program(vec![begin("INT;JSON", "out.error"), recover(), end()]);
    let ste = ail::ste::render_ail_ste(&original).unwrap();
    for profile in ["Compiler", "System", "AgentTool", "UI"] {
        let mut forged = original.clone();
        forged.profile = profile.into();
        assert!(!verify_ail_bytecode(&forged).is_empty());
        assert!(
            ail::ste::parse_ail_ste(&ste.replace("\"Application\"", &format!("\"{profile}\"")))
                .is_err()
        );
    }
    for index in 0..3 {
        let mut forged = original.clone();
        forged.actions.get_mut("Run").unwrap().instructions[index]
            .operands
            .insert("ignored".into(), "true".into());
        assert!(
            verify_ail_bytecode(&forged)
                .iter()
                .any(|line| line.contains("AILBC019"))
        );
    }
    let mut integer = raw_program(vec![command(
        "CALL_INT",
        &[("key", "out.value"), ("target", "bad"), ("arguments", "")],
    )]);
    routine(
        &mut integer,
        "bad",
        vec![
            command("INTEGER_FUNCTION", &[("parameters", "")]),
            begin("any", "local.error"),
            recover(),
            end(),
            command("RETURN_INT", &[("expression", "i:1")]),
        ],
    );
    assert!(!verify_ail_bytecode(&integer).is_empty());
}

#[test]
fn recovery_core_checks_exact_edges_types_owners_and_secret_destination_writes() {
    let original = core(&source(
        "- try:\n- the system computes data.value as 1 / 0\n- recover DIVZERO into data.error:\n- the system assigns 7 to output.value\n- end try",
    ));
    let operand = original
        .graph
        .edges
        .iter()
        .find(|edge| edge.kind == "recovery_destination")
        .unwrap()
        .clone();
    for variant in [
        "duplicate",
        "metadata",
        "wrong_type",
        "forged_field",
        "secret",
    ] {
        let mut candidate = original.clone();
        match variant {
            "duplicate" => {
                let mut edge = operand.clone();
                edge.id = "duplicate-recovery-destination".into();
                edge.ordinal = candidate.graph.edges.len();
                candidate.graph.edges.push(edge);
            }
            "metadata" => {
                candidate
                    .graph
                    .edges
                    .iter_mut()
                    .find(|e| e.id == operand.id)
                    .unwrap()
                    .attributes
                    .insert("ignored".into(), "true".into());
            }
            "wrong_type" => {
                candidate
                    .graph
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == operand.target)
                    .unwrap()
                    .type_name = Some("Int".into())
            }
            "forged_field" => {
                let mut field = candidate
                    .graph
                    .nodes
                    .iter()
                    .find(|n| n.id == operand.target)
                    .unwrap()
                    .clone();
                field.id = "forged-recovery-field".into();
                candidate.graph.nodes.push(field.clone());
                candidate
                    .graph
                    .edges
                    .iter_mut()
                    .find(|e| e.id == operand.id)
                    .unwrap()
                    .target = field.id;
            }
            "secret" => {
                candidate
                    .graph
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == operand.target)
                    .unwrap()
                    .attributes
                    .insert("secret".into(), "true".into());
                candidate
                    .graph
                    .edges
                    .retain(|e| !matches!(e.kind.as_str(), "writes" | "reads"));
            }
            _ => unreachable!(),
        }
        assert!(compile_ail_core_bytecode(&candidate).is_err(), "{variant}");
        if let Ok(restored) = parse_ail_core_text(&render_ail_core(&candidate)) {
            assert!(
                compile_ail_core_bytecode(&restored).is_err(),
                "saved {variant}"
            );
        }
        if variant == "secret" {
            assert!(
                check_ail_core(&candidate)
                    .iter()
                    .any(|d| d.contains("AIL002"))
            );
        }
    }
    for (key, value) in [("recovery_errors", "JSON;INT"), ("ignored", "true")] {
        let mut candidate = original.clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| {
                n.kind == "Control"
                    && n.attributes
                        .get("control_kind")
                        .is_some_and(|v| v == "recover")
            })
            .unwrap()
            .attributes
            .insert(key.into(), value.into());
        assert!(compile_ail_core_ste(&candidate).is_err());
    }
    for profile in ["Compiler", "UI", "System", "AgentTool"] {
        let mut candidate = original.clone();
        candidate.package.profile = profile.into();
        assert!(compile_ail_core_ste(&candidate).is_err());
    }
}

#[test]
fn malformed_and_descriptive_recovery_source_is_never_ignored() {
    let package = package();
    for body in [
        "- try:",
        "- recover JSON into data.error:",
        "- end try",
        "- try:\n- recover any or JSON into data.error:\n- end try",
        "- try:\n- recover JSON or JSON into data.error:\n- end try",
        "- try:\n- recover IO into data.error:\n- end try",
        "- try:\n- recover JSON INT into data.error:\n- end try",
        "- try:\n- recover JSON:\n- end try",
        "- try:\n- recover JSON into data.value:\n- end try",
        "- try:\n- recover JSON into data.error:\n- recover INT into data.error:\n- end try",
    ] {
        let text = source(body);
        let rejected = match parse_ail_package_spec_text(&package, &text) {
            Err(_) => true,
            Ok(document) => {
                compile_ail_core_bytecode(&elaborate_ail_core(&package, &document)).is_err()
            }
        };
        assert!(rejected, "accepted {body}");
    }
    let compiler = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for statement in [
        "try:",
        "try:.",
        "recover any into data.error:",
        "recover any into data.error:.",
        "end try",
        "end try.",
    ] {
        let legacy = format!(
            "{DECLARATIONS}Function: old.\nThe function needs:\n- value: Int\nThe function produces:\n- result: Int\nWhen old runs:\n- {statement}\n- the function returns value unchanged\n"
        );
        let pass = compiler.spec_text.replace(
            "- the system reads every edge",
            &format!("- {statement}\n- the system reads every edge"),
        );
        for (package, text) in [(&package, legacy), (&compiler, pass)] {
            assert!(
                parse_ail_package_spec_text(package, &text).is_err(),
                "accepted {statement}"
            );
        }
    }
}

#[test]
fn imported_recovery_qualifies_destination_and_preserves_canonical_category_tokens() {
    let mut child = package();
    child.metadata.name = "recovery-child".into();
    child.spec_text = source(
        "- try:\n- the system computes output.value as 1 / 0\n- recover OVERFLOW or DIVZERO into data.error:\n- the system assigns 3 to output.value\n- end try\n- the system returns output.value\n- the system returns data.error",
    );
    let mut parent = package();
    parent.spec_text="The application Parent manages recovery.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system returns child.output.value\n- the system returns child.data.error\n- the system records a trace event named Done\n".into();
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
    assert_success(
        &program,
        &[
            ("child.data.error", "AIL-RUNTIME-DIVZERO"),
            ("child.output.value", "3"),
        ],
    );
}
