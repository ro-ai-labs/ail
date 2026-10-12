use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn source(name: &str, locals: &str, body: &str) -> String {
    format!(
        "The application Functions manages numbers.\n\nA N has:\n- value: Int\n\nAn Out has:\n- result: Int\n\nFunction: {name}.\n\nThe function needs:\n- n: Int\n\nThe function produces:\n- result: Int\n\nThe function stores:\n{locals}\nThe function executes:\n{body}\n\nAction: Run.\n\nWhen Run happens:\n- the system calls {name} with n = n.value into out.result\n- the system returns out.result\n- the system records a trace event named Calculated\n"
    )
}
fn core(source: &str) -> AilCore {
    let package = load_ail_package_dir("bench/R003-max/program.ail").unwrap();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}
fn native(program: &AilBytecodeProgram, input: &str) -> std::process::Output {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "ail-function-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(
        &path,
        compile_ail_bytecode_native_elf(program, "Run", "linux-x86_64-elf").unwrap(),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let mut retries = 0;
    let output = loop {
        match Command::new(&path).arg(format!("n.value={input}")).output() {
            Err(e) if e.raw_os_error() == Some(26) && retries < 20 => {
                retries += 1;
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            result => break result.unwrap(),
        }
    };
    fs::remove_file(path).unwrap();
    output
}
fn assert_result(source: &str, input: &str, expected: &str) {
    let original = core(source);
    for c in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&c).unwrap()).unwrap();
        let vm = run_ail_bytecode_action(
            &program,
            "Run",
            BTreeMap::from([("n.value".into(), input.into())]),
        )
        .unwrap();
        assert_eq!(
            vm.output_state(),
            BTreeMap::from([("out.result".into(), expected.into())])
        );
        assert_eq!(vm.final_state["n.value"], input);
        assert!(!vm.final_state.keys().any(|k| k.starts_with("local.")));
        let output = native(&program, input);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("out.result={expected}\n")
        );
    }
}
#[test]
fn recursive_factorial_executes_with_values_and_early_return() {
    let source = source(
        "factorial",
        "- partial: Int",
        "- if local.n <= 1:\n- the function returns 1\n- end if\n- the function calls factorial with n = local.n - 1 into local.partial\n- the function returns local.n * local.partial",
    );
    assert_result(&source, "0", "1");
    assert_result(&source, "20", "2432902008176640000");
}
#[test]
fn fibonacci_preserves_both_recursive_results_in_each_frame() {
    let source = source(
        "fibonacci",
        "- left: Int\n- right: Int",
        "- if local.n <= 1:\n- the function returns local.n\n- end if\n- the function calls fibonacci with n = local.n - 1 into local.left\n- the function calls fibonacci with n = local.n - 2 into local.right\n- the function returns local.left + local.right",
    );
    assert_result(&source, "10", "55");
}

#[test]
fn named_arguments_and_zero_argument_calls_execute() {
    let pair = source("pair", "", "- the function returns local.a * 10 + local.b")
        .replace("- n: Int", "- a: Int\n- b: Int")
        .replace(
            "with n = n.value into",
            "with b = n.value + 1; a = n.value into",
        );
    assert_result(&pair, "4", "45");
    let constant = source("constant", "", "- the function returns 7")
        .replace("- n: Int", "")
        .replace("with n = n.value into", "with none into");
    assert_result(&constant, "99", "7");
}

#[test]
fn forward_and_mutually_recursive_functions_execute() {
    let even = source(
        "even",
        "- result: Int",
        "- if local.n == 0:\n- the function returns 1\n- end if\n- the function calls odd with n = local.n - 1 into local.result\n- the function returns local.result",
    );
    let odd = "\nFunction: odd.\nThe function needs:\n- n: Int\nThe function produces:\n- result: Int\nThe function stores:\n- result: Int\nThe function executes:\n- if local.n == 0:\n- the function returns 0\n- end if\n- the function calls even with n = local.n - 1 into local.result\n- the function returns local.result\n";
    assert_result(&(even + odd), "8", "1");
}

#[test]
fn function_errors_match_on_vm_and_native_without_result_stdout() {
    for (locals, body, expected) in [
        (
            "",
            "- the function returns 1 / local.n",
            "AIL-RUNTIME-DIVZERO",
        ),
        (
            "",
            "- the function returns 9223372036854775807 + 1",
            "AIL-RUNTIME-OVERFLOW",
        ),
        (
            "- unset: Int",
            "- the function returns local.unset",
            "AIL-RUNTIME-INT",
        ),
        (
            "",
            "- if local.n > 0:\n- the function returns 1\n- end if",
            "AIL-RUNTIME-RETURN",
        ),
        (
            "- result: Int",
            "- the function calls bad with n = local.n into local.result\n- the function returns local.result",
            "AIL-RUNTIME-DEPTH",
        ),
        ("", "- while 1 == 1:\n- continue\n- end while", "step limit"),
    ] {
        let program = compile_ail_core_bytecode(&core(&source("bad", locals, body))).unwrap();
        assert!(
            run_ail_bytecode_action(
                &program,
                "Run",
                BTreeMap::from([("n.value".into(), "0".into())])
            )
            .unwrap_err()
            .contains(expected)
        );
        let output = native(&program, "0");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn integer_function_depth_accepts_64_frames_and_rejects_65() {
    let source = source(
        "count",
        "- result: Int",
        "- if local.n == 0:\n- the function returns 0\n- end if\n- the function calls count with n = local.n - 1 into local.result\n- the function returns local.result + 1",
    );
    assert_result(&source, "63", "63");
    let program = compile_ail_core_bytecode(&core(&source)).unwrap();
    assert!(
        run_ail_bytecode_action(
            &program,
            "Run",
            BTreeMap::from([("n.value".into(), "64".into())])
        )
        .unwrap_err()
        .contains("DEPTH")
    );
    let output = native(&program, "64");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("DEPTH"));
}

#[test]
fn source_function_types_scope_and_arity_are_checked() {
    let original = source("identity", "", "- the function returns local.n");
    for text in [
        original.replace("- n: Int", "- n: Text"),
        original.replace("- result: Int", "- result: Text"),
        original.replace("with n = n.value", "with missing = n.value"),
        original.replace("with n = n.value", "with n = n.value; extra = 1"),
        original.replace("with n = n.value", "with n = n.value; n = 1"),
        original.replace("returns local.n", "returns n.value"),
        original.replace("returns local.n", "returns local.missing"),
        original.replace("calls identity with", "calls unknown with"),
        original.replace("- n: Int", "- n: Int\n- n: Int"),
    ] {
        let package = load_ail_package_dir("bench/R003-max/program.ail").unwrap();
        if let Ok(document) = parse_ail_package_spec_text(&package, &text) {
            let core = elaborate_ail_core(&package, &document);
            assert!(!check_ail_core(&core).is_empty(), "accepted: {text}");
            assert!(compile_ail_core_bytecode(&core).is_err());
        }
    }
}

#[test]
fn bytecode_verifier_rejects_invalid_calls_and_function_bodies() {
    let original = compile_ail_core_bytecode(&core(&source(
        "identity",
        "",
        "- the function returns local.n",
    )))
    .unwrap();
    let mut variants = Vec::new();
    for (key, value) in [
        ("target", "missing"),
        ("arguments", "n=i:1;n=i:2"),
        ("arguments", "extra=i:2"),
        ("key", "bad"),
    ] {
        let mut p = original.clone();
        p.actions
            .get_mut("Run")
            .unwrap()
            .instructions
            .iter_mut()
            .find(|i| i.opcode == "CALL_INT")
            .unwrap()
            .operands
            .insert(key.into(), value.into());
        variants.push(p);
    }
    for (key, value) in [("parameters", "n;n"), ("parameters", "z;a")] {
        let mut p = original.clone();
        p.actions.get_mut("identity").unwrap().instructions[0]
            .operands
            .insert(key.into(), value.into());
        variants.push(p);
    }
    let mut globals = original.clone();
    globals.actions.get_mut("identity").unwrap().instructions[1]
        .operands
        .insert("expression".into(), "f:n.value".into());
    variants.push(globals);
    let mut name = original.clone();
    name.actions.get_mut("identity").unwrap().name = "other".into();
    variants.push(name);
    let mut actioncall = original.clone();
    actioncall
        .actions
        .get_mut("Run")
        .unwrap()
        .instructions
        .iter_mut()
        .find(|i| i.opcode == "CALL_INT")
        .unwrap()
        .opcode = "CALL_ACTION".into();
    variants.push(actioncall);
    let mut badreturn = original.clone();
    let instruction = badreturn.actions["identity"].instructions[1].clone();
    badreturn
        .actions
        .get_mut("Run")
        .unwrap()
        .instructions
        .push(instruction);
    variants.push(badreturn);
    for p in variants {
        assert!(!verify_ail_bytecode(&p).is_empty());
        assert!(run_ail_bytecode_action(&p, "Run", BTreeMap::new()).is_err());
        assert!(compile_ail_bytecode_native_elf(&p, "Run", "linux-x86_64-elf").is_err());
    }
    assert!(run_ail_bytecode_action(&original, "identity", BTreeMap::new()).is_err());
    assert!(compile_ail_bytecode_native_elf(&original, "identity", "linux-x86_64-elf").is_err());
}

#[test]
fn imported_recursive_functions_keep_locals_and_qualify_calls() {
    let child = load_ail_package_dir("bench/R005-factorial/program.ail").unwrap();
    let mut parent = load_ail_package_dir("bench/R003-max/program.ail").unwrap();
    parent.spec_text="The application Root manages functions.\nA N has:\n- value: Int\nAn Out has:\n- result: Int\nAction: Run.\nWhen Run happens:\n- the system calls Math.factorial with n = n.value into out.result\n- the system returns out.result\n- the system records a trace event named Calculated\n".into();
    parent.imports.push(AilLoadedImport {
        spec: AilImportSpec {
            path: "unused".into(),
            version: None,
            alias: "Math".into(),
            resolved_package: None,
            registry_identity: None,
        },
        package: Box::new(child),
    });
    let core = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    for candidate in [
        core.clone(),
        parse_ail_core_text(&render_ail_core(&core)).unwrap(),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let vm = run_ail_bytecode_action(
            &program,
            "Run",
            BTreeMap::from([("n.value".into(), "7".into())]),
        )
        .unwrap();
        assert_eq!(
            vm.output_state(),
            BTreeMap::from([("out.result".into(), "5040".into())])
        );
        let output = native(&program, "7");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "out.result=5040\n"
        );
    }
}

#[test]
fn document_execution_calls_integer_functions() {
    let package = load_ail_package_dir("bench/R003-max/program.ail").unwrap();
    let doc = parse_ail_package_spec_text(
        &package,
        &source("square", "", "- the function returns local.n * local.n"),
    )
    .unwrap();
    let result = run_ail_action(
        &doc,
        "Run",
        BTreeMap::from([("n.value".into(), "9".into())]),
    )
    .unwrap();
    assert_eq!(
        result.output_state(),
        BTreeMap::from([("out.result".into(), "81".into())])
    );
}

#[test]
fn cli_all_actions_builds_entrypoints_with_function_dependencies() {
    let dir = std::env::temp_dir().join(format!("ail-function-bundle-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_ail"))
        .args([
            "ail-compile",
            "bench/R005-factorial/program.ail",
            "--all-actions",
            "--target",
            "linux-x86_64-elf",
            "--artifact-dir",
        ])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(dir.join("target-Factorial.elf").exists());
    assert!(!dir.join("target-factorial.elf").exists());
    let result = Command::new(dir.join("target-Factorial.elf"))
        .arg("n.value=6")
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "out.result=720\n"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn function_callee_traces_are_included_in_wasm_contract_reports() {
    let original = compile_ail_core_bytecode(&core(&source(
        "traced",
        "",
        "- the function records a trace event named Called\n- the function returns local.n",
    )))
    .unwrap();
    let mut program = original;
    program
        .target_support
        .insert("wasm32-unknown-sandbox-wasm".into(), "supported".into());
    // Isolate the callee's trace requirement from the action wrapper's trace.
    program
        .actions
        .get_mut("Run")
        .unwrap()
        .instructions
        .retain(|i| i.opcode != "EMIT_TRACE");
    let dir = std::env::temp_dir().join(format!("ail-function-contract-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("input.ailbc.json");
    fs::write(&path, render_ail_bytecode(&program)).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_ail"))
        .arg("ail-compile")
        .arg(&path)
        .args([
            "--action",
            "Run",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
        ])
        .arg(dir.join("bundle"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report = fs::read_to_string(dir.join("bundle/wasm-contract-report.txt")).unwrap();
    assert!(report.contains("trace-preservation required"), "{report}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mixed_legacy_and_executable_function_sections_are_rejected() {
    let original = source("identity", "", "- the function returns local.n");
    let package = load_ail_package_dir("bench/R003-max/program.ail").unwrap();
    for text in [
        original.replace(
            "The function stores:",
            "When identity runs:\n- arbitrary ignored prose\nThe function stores:",
        ),
        original.replace(
            "Action: Run.",
            "When identity runs:\n- arbitrary ignored prose\nAction: Run.",
        ),
    ] {
        assert!(parse_ail_package_spec_text(&package, &text).is_err());
    }
}

#[test]
fn function_protocol_rejects_unexpected_operands() {
    let mut program = compile_ail_core_bytecode(&core(&source(
        "identity",
        "",
        "- the function returns local.n",
    )))
    .unwrap();
    program.actions.get_mut("identity").unwrap().instructions[1]
        .operands
        .insert("left".into(), "ignored".into());
    assert!(!verify_ail_bytecode(&program).is_empty());
    assert!(run_ail_bytecode_action(&program, "Run", BTreeMap::new()).is_err());
    assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
}
