use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const DECLARATIONS: &str = "The application Transformations manages records.\n\nA Data has:\n- raw: Text\n- separator: Text\n- needle: Text\n- replacement: Text\n- parts: List<Text>\n- count: Int\n\nAn Output has:\n- raw: Text\n- parts: List<Text>\n- count: Int\n\n";

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
            "ail-text-transformations-{}-{}",
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
                "Text transformation blocked: {}",
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
fn transformations_survive_source_saved_core_rendered_ste_vm_and_native() {
    let text = source(
        r#"- the system assigns " \u0085é\u0000🙂::x:::: " to data.raw
- the system assigns "::" to data.separator
- the system assigns "x" to data.needle
- the system assigns "and [value=Text],name=shadow" to data.replacement
- the system trims text data.raw into data.raw
- the system splits text data.raw by data.separator into data.parts
- the system joins list data.parts with data.separator into output.raw
- the system replaces text data.needle in output.raw with data.replacement into output.raw
- the system returns output.raw"#,
    );
    let original = core(&text);
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let ste = compile_ail_core_ste(&candidate).unwrap();
        assert!(ste.contains("Split text in field"));
        let program = ail::ste::parse_ail_ste(&ste).unwrap();
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(
            vm.final_state["data.parts"],
            "[\"é\\u0000🙂\",\"x\",\"\",\"\"]"
        );
        let expected = "é\0🙂::and [value=Text],name=shadow::::";
        assert_eq!(vm.final_state["output.raw"], expected);
        let output = native(
            &scratch.binary(&program, "Run"),
            &AilFileResources::default(),
            &BTreeMap::new(),
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(output.stdout, format!("output.raw={expected}\n").as_bytes());
    }
}

fn seed(key: &str, value: &str) -> AilBytecodeInstruction {
    command(
        "SET_FIELD",
        &[("key", key), ("value", value), ("text", "seed")],
    )
}

fn assert_backends(
    program: &AilBytecodeProgram,
    values: &BTreeMap<String, String>,
    expected: Result<&str, &str>,
) {
    let scratch = Scratch::new();
    let vm = run_ail_bytecode_action(program, "Run", values.clone());
    let output = native(
        &scratch.binary(program, "Run"),
        &resources(&[], &[]),
        values,
    );
    match expected {
        Ok(value) => {
            assert_eq!(vm.unwrap().final_state["out.value"], value);
            assert!(output.status.success(), "{}", brief(&output));
            assert_eq!(output.stdout, format!("out.value={value}\n").as_bytes());
        }
        Err(category) => {
            let marker = format!("AIL-RUNTIME-{category}:");
            assert!(vm.unwrap_err().contains(&marker));
            failed(&output, &marker);
        }
    }
}

#[test]
fn explicit_separators_empty_needles_and_fixed_whitespace_have_scalar_semantics() {
    let whitespace = "\u{9}\u{a}\u{b}\u{c}\u{d}\u{1c}\u{1d}\u{1e}\u{1f} \u{85}\u{a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}";
    assert_eq!(whitespace.chars().count(), 29);
    for (opcode, source, other, replacement, expected) in [
        ("TEXT_SPLIT", "", ":", "", "[\"\"]"),
        ("TEXT_SPLIT", "aaaaa", "aa", "", "[\"\",\"\",\"a\"]"),
        (
            "TEXT_SPLIT",
            "\0é\0🙂\0",
            "\0",
            "",
            "[\"\",\"é\",\"🙂\",\"\"]",
        ),
        ("TEXT_JOIN_LIST", "[]", "🙂", "", ""),
        (
            "TEXT_JOIN_LIST",
            "[\"\",\"\\u0000\",\"é\",\"\"]",
            "::",
            "",
            "::\0::é::",
        ),
        ("TEXT_REPLACE", "é🙂e\u{301}", "", "|", "|é|🙂|e|\u{301}|"),
        ("TEXT_REPLACE", "aaaaa", "aa", "aaa", "aaaaaaa"),
        ("TEXT_REPLACE", "", "", "🙂", "🙂"),
        (
            "TEXT_TRIM",
            "\u{200b} x \u{feff}",
            "",
            "",
            "\u{200b} x \u{feff}",
        ),
        ("TEXT_TRIM", " \0 ", "", "", "\0"),
        ("TEXT_TRIM", whitespace, "", "", ""),
    ] {
        let mut operands = vec![("key", "out.value"), ("source", "data.source")];
        if matches!(opcode, "TEXT_SPLIT" | "TEXT_JOIN_LIST") {
            operands.push(("separator", "data.other"));
        }
        if opcode == "TEXT_REPLACE" {
            operands.extend([
                ("needle", "data.other"),
                ("replacement", "data.replacement"),
            ]);
        }
        let program = raw_program(vec![
            seed("data.source", source),
            seed("data.other", other),
            seed("data.replacement", replacement),
            command(opcode, &operands),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        assert_backends(&program, &BTreeMap::new(), Ok(expected));
    }
    let text = format!("{whitespace}\0é{whitespace}🙂\u{200b}{whitespace}");
    let program = raw_program(vec![
        seed("data.source", &text),
        command(
            "TEXT_TRIM",
            &[("key", "out.value"), ("source", "data.source")],
        ),
        command("OUTPUT_FIELD", &[("key", "out.value")]),
    ]);
    assert_backends(
        &program,
        &BTreeMap::new(),
        Ok(&format!("\0é{whitespace}🙂\u{200b}")),
    );
}

#[test]
fn direct_aliases_read_every_operand_before_mutating_and_preserve_old_values() {
    for (opcode, operand, original, wanted, extra) in [
        (
            "TEXT_SPLIT",
            "source",
            "a::b",
            "[\"a\",\"b\"]",
            vec![("separator", "data.separator")],
        ),
        (
            "TEXT_SPLIT",
            "separator",
            "::",
            "[\"a\",\"b\"]",
            vec![("separator", "out.value")],
        ),
        (
            "TEXT_JOIN_LIST",
            "source",
            "[\"a\",\"b\"]",
            "a::b",
            vec![("separator", "data.separator")],
        ),
        (
            "TEXT_JOIN_LIST",
            "separator",
            "::",
            "a::b",
            vec![("separator", "out.value")],
        ),
        (
            "TEXT_REPLACE",
            "source",
            "a::b",
            "a🙂b",
            vec![
                ("needle", "data.separator"),
                ("replacement", "data.replacement"),
            ],
        ),
        (
            "TEXT_REPLACE",
            "needle",
            "::",
            "a🙂b",
            vec![("needle", "out.value"), ("replacement", "data.replacement")],
        ),
        (
            "TEXT_REPLACE",
            "replacement",
            "🙂",
            "a🙂b",
            vec![("needle", "data.separator"), ("replacement", "out.value")],
        ),
        ("TEXT_TRIM", "source", " a::b ", "a::b", vec![]),
    ] {
        let source = if opcode == "TEXT_JOIN_LIST" {
            "[\"a\",\"b\"]"
        } else {
            "a::b"
        };
        let mut args = vec![
            ("key", "out.value"),
            (
                "source",
                if operand == "source" {
                    "out.value"
                } else {
                    "data.source"
                },
            ),
        ];
        args.extend(extra);
        let program = raw_program(vec![
            seed("data.source", source),
            seed("data.separator", "::"),
            seed("data.replacement", "🙂"),
            seed("out.value", original),
            command(
                "COPY_FIELD",
                &[
                    ("key", "data.snapshot"),
                    ("source", "out.value"),
                    ("text", "snapshot"),
                ],
            ),
            command(opcode, &args),
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        let vm = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(vm.final_state["data.snapshot"], original);
        assert_backends(&program, &BTreeMap::new(), Ok(wanted));
    }
}

#[test]
fn validation_order_is_typed_even_when_output_would_be_empty() {
    let oversized = "x".repeat(1_048_577);
    for (opcode, input, category) in [
        (
            "TEXT_SPLIT",
            state(&[("data.source", &oversized)]),
            "TEXT-LIMIT",
        ),
        (
            "TEXT_SPLIT",
            state(&[("data.source", ""), ("data.separator", "")]),
            "TEXT",
        ),
        (
            "TEXT_JOIN_LIST",
            state(&[("data.source", "[\"a\",false]")]),
            "LIST",
        ),
        ("TEXT_JOIN_LIST", state(&[("data.source", "[]")]), "TEXT"),
        (
            "TEXT_JOIN_LIST",
            state(&[("data.source", "[]"), ("data.separator", &oversized)]),
            "TEXT-LIMIT",
        ),
        (
            "TEXT_REPLACE",
            state(&[("data.source", &oversized)]),
            "TEXT-LIMIT",
        ),
        (
            "TEXT_REPLACE",
            state(&[("data.source", ""), ("data.needle", &oversized)]),
            "TEXT-LIMIT",
        ),
        (
            "TEXT_REPLACE",
            state(&[("data.source", ""), ("data.needle", "absent")]),
            "TEXT",
        ),
        (
            "TEXT_REPLACE",
            state(&[
                ("data.source", ""),
                ("data.needle", "absent"),
                ("data.replacement", &oversized),
            ]),
            "TEXT-LIMIT",
        ),
        ("TEXT_TRIM", BTreeMap::new(), "TEXT"),
    ] {
        let mut args = vec![("key", "out.value"), ("source", "data.source")];
        match opcode {
            "TEXT_SPLIT" | "TEXT_JOIN_LIST" => args.push(("separator", "data.separator")),
            "TEXT_REPLACE" => args.extend([
                ("needle", "data.needle"),
                ("replacement", "data.replacement"),
            ]),
            _ => {}
        }
        let mut instructions: Vec<_> = input.iter().map(|(key, value)| seed(key, value)).collect();
        instructions.push(command(opcode, &args));
        assert_backends(&raw_program(instructions), &BTreeMap::new(), Err(category));
    }
}

#[test]
fn split_canonical_and_item_limits_join_borrowed_expansion_and_text_result_limits() {
    let limit = 1_048_576;
    for (opcode, raw, separator, expected) in [
        ("TEXT_SPLIT", "x".repeat(limit - 4), ":".into(), Ok("1")),
        (
            "TEXT_SPLIT",
            "x".repeat(limit - 3),
            ":".into(),
            Err("LIST-LIMIT"),
        ),
        ("TEXT_SPLIT", ",".repeat(65_535), ",".into(), Ok("65536")),
        (
            "TEXT_SPLIT",
            ",".repeat(65_536),
            ",".into(),
            Err("LIST-LIMIT"),
        ),
        (
            "TEXT_SPLIT",
            "\u{7f}".repeat(180_000),
            ":".into(),
            Err("LIST-LIMIT"),
        ),
        (
            "TEXT_JOIN_LIST",
            format!("[\"{}\"]", "\u{7f}".repeat(180_000)),
            "".into(),
            Ok("180000"),
        ),
        (
            "TEXT_JOIN_LIST",
            "[\"\",\"\"]".into(),
            "x".repeat(limit),
            Ok("1048576"),
        ),
        (
            "TEXT_JOIN_LIST",
            "[\"\",\"\",\"\"]".into(),
            "x".repeat(limit),
            Err("TEXT-LIMIT"),
        ),
        ("TEXT_TRIM", "x".repeat(limit), "".into(), Ok("1048576")),
        (
            "TEXT_TRIM",
            " ".repeat(limit + 1),
            "".into(),
            Err("TEXT-LIMIT"),
        ),
    ] {
        let mut args = vec![("key", "data.result"), ("source", "data.raw")];
        if opcode != "TEXT_TRIM" {
            args.push(("separator", "data.separator"));
        }
        let length = if opcode == "TEXT_SPLIT" {
            command(
                "LIST_LENGTH",
                &[
                    ("key", "out.value"),
                    ("source", "data.result"),
                    ("type", "Text"),
                ],
            )
        } else {
            command(
                "TEXT_LENGTH",
                &[("key", "out.value"), ("source", "data.result")],
            )
        };
        let program = raw_program(vec![
            seed("data.raw", &raw),
            seed("data.separator", &separator),
            command(opcode, &args),
            length,
            command("OUTPUT_FIELD", &[("key", "out.value")]),
        ]);
        assert_backends(&program, &BTreeMap::new(), expected);
    }
}

#[test]
fn local_transformations_survive_function_frames_and_public_round_trips() {
    let functions = "Function: pieces.\nThe function needs:\n- raw: Text\n- separator: Text\nThe function produces:\n- result: List<Text>\nThe function stores:\n- result: List<Text>\nThe function executes:\n- the function splits text local.raw by local.separator into local.result\n- the function returns local.result\nFunction: clean.\nThe function needs:\n- raw: Text\n- separator: Text\nThe function produces:\n- result: Text\nThe function stores:\n- parts: List<Text>\n- result: Text\n- needle: Text\n- replacement: Text\nThe function executes:\n- the function trims text local.raw into local.raw\n- the function calls pieces with raw = local.raw; separator = local.separator into local.parts\n- the function joins list local.parts with local.separator into local.result\n- the function assigns \"\" to local.needle\n- the function assigns \"|\" to local.replacement\n- the function replaces text local.needle in local.result with local.replacement into local.result\n- the function returns local.result";
    let original = core(&with_functions(
        functions,
        "- the system calls clean with raw = data.raw; separator = data.separator into output.raw\n- the system returns output.raw",
    ));
    let input = state(&[("data.raw", "  é::🙂  "), ("data.separator", "::")]);
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let vm = run_ail_bytecode_action(&program, "Run", input.clone()).unwrap();
        assert_eq!(vm.final_state["output.raw"], "|é|:|:|🙂|");
        assert!(!vm.final_state.keys().any(|key| key.starts_with("local.")));
        assert!(
            !vm.trace
                .iter()
                .any(|line| line.contains("computed text local."))
        );
        let output = native(
            &scratch.binary(&program, "Run"),
            &resources(&[], &[]),
            &input,
        );
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(
            output.stdout,
            b"output.raw=|\xc3\xa9|:|:|\xf0\x9f\x99\x82|\n"
        );
    }
}

#[test]
fn protocol_exact_operands_and_nonexecution_profiles_are_rejected_before_execution() {
    for (opcode, operands) in [
        (
            "TEXT_SPLIT",
            vec![
                ("key", "out.value"),
                ("source", "data.raw"),
                ("separator", "data.separator"),
            ],
        ),
        (
            "TEXT_JOIN_LIST",
            vec![
                ("key", "out.value"),
                ("source", "data.parts"),
                ("separator", "data.separator"),
            ],
        ),
        (
            "TEXT_REPLACE",
            vec![
                ("key", "out.value"),
                ("source", "data.raw"),
                ("needle", "data.needle"),
                ("replacement", "data.replacement"),
            ],
        ),
        (
            "TEXT_TRIM",
            vec![("key", "out.value"), ("source", "data.raw")],
        ),
    ] {
        let original = raw_program(vec![command(opcode, &operands)]);
        let ste = ail::ste::render_ail_ste(&original).unwrap();
        for operand in operands.iter().map(|(key, _)| *key).chain(["extra"]) {
            let mut forged = original.clone();
            let args = &mut forged.actions.get_mut("Run").unwrap().instructions[0].operands;
            if operand == "extra" {
                args.insert("extra".into(), "ignored".into());
            } else {
                args.remove(operand);
            }
            assert!(
                verify_ail_bytecode(&forged)
                    .iter()
                    .any(|d| d.contains("AILBC012")),
                "{opcode}: {operand}"
            );
            assert!(ail::ste::render_ail_ste(&forged).is_err());
            assert!(compile_ail_bytecode_native_elf(&forged, "Run", "linux-x86_64-elf").is_err());
        }
        for profile in ["Compiler", "UI", "AgentTool", "System"] {
            let mut forged = original.clone();
            forged.profile = profile.into();
            assert!(
                !verify_ail_bytecode(&forged).is_empty(),
                "{opcode}: {profile}"
            );
            assert!(
                ail::ste::parse_ail_ste(&ste.replace("\"Application\"", &format!("\"{profile}\"")))
                    .is_err()
            );
        }
    }
}

#[test]
fn core_metadata_edges_types_and_semantic_secret_access_are_checked() {
    let original = core(&source(
        "- the system replaces text data.needle in data.raw with data.replacement into output.raw",
    ));
    for kind in [
        "text_source",
        "text_other",
        "text_replacement",
        "text_destination",
        "text_step",
    ] {
        let edge = original
            .graph
            .edges
            .iter()
            .find(|e| e.kind == kind)
            .unwrap()
            .clone();
        let mut duplicate = original.clone();
        let mut extra = edge.clone();
        extra.id = format!("duplicate-{kind}");
        extra.ordinal = duplicate.graph.edges.len();
        duplicate.graph.edges.push(extra);
        assert!(
            compile_ail_core_bytecode(&duplicate).is_err(),
            "duplicate {kind}"
        );
        let mut metadata = original.clone();
        metadata
            .graph
            .edges
            .iter_mut()
            .find(|e| e.id == edge.id)
            .unwrap()
            .attributes
            .insert("ignored".into(), "true".into());
        assert!(
            compile_ail_core_bytecode(&metadata).is_err(),
            "metadata {kind}"
        );
    }
    for name in ["Data.raw", "Data.needle", "Data.replacement", "Output.raw"] {
        let mut secret = original.clone();
        secret
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name == name)
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        secret
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        let code = if name == "Output.raw" {
            "AIL002"
        } else {
            "AIL005"
        };
        let secret = parse_ail_core_text(&render_ail_core(&secret)).unwrap();
        assert!(
            check_ail_core(&secret).iter().any(|d| d.contains(code)),
            "{name}: {:?}",
            check_ail_core(&secret)
        );
        let mut wrong = original.clone();
        wrong
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "Field" && n.name == name)
            .unwrap()
            .type_name = Some("Int".into());
        assert!(compile_ail_core_ste(&wrong).is_err(), "wrong type {name}");
    }
    for body in [
        "- the system splits text data.raw by data.separator into output.raw",
        "- the system joins list data.raw with data.separator into output.raw",
        "- the system trims text data.count into output.raw",
    ] {
        assert!(compile_ail_core_bytecode(&core(&source(body))).is_err());
    }
}

#[test]
fn descriptive_legacy_and_compiler_bodies_reject_new_full_and_malformed_forms() {
    let application = package();
    let compiler = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for body in [
        "splits text data.raw by data.separator into data.parts",
        "joins list data.parts with data.separator into data.raw",
        "replaces text data.needle in data.raw with data.replacement into data.raw",
        "trims text data.raw into data.raw",
        "splits text",
        "joins list",
        "replaces text",
        "trims text",
    ] {
        for subject in ["the system", "the function"] {
            let statement = format!("{subject} {body}");
            let legacy = format!(
                "{DECLARATIONS}Function: legacy.\nThe function needs:\n- raw: Text\nThe function produces:\n- result: Text\nWhen legacy runs:\n- {statement}\n- the function returns raw unchanged\n"
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
}

#[test]
fn file_transformations_preserve_exact_unicode_nul_and_explicit_grants() {
    let text = source(
        "- the system reads text from file resource input into data.raw\n- the system assigns \"\\t\" to data.separator\n- the system trims text data.raw into data.raw\n- the system splits text data.raw by data.separator into data.parts\n- the system assigns \" | \" to data.separator\n- the system joins list data.parts with data.separator into output.raw\n- the system assigns \"old\" to data.needle\n- the system assigns \"new\" to data.replacement\n- the system replaces text data.needle in output.raw with data.replacement into output.raw\n- the system writes text output.raw to file resource report",
    );
    let program = compile_ail_core_bytecode(&core(&text)).unwrap();
    let scratch = Scratch::new();
    let input = scratch.file("input file=tsv", " \u{85}é\0\told\t🙂\u{3000}".as_bytes());
    let report = scratch.path("report file=txt");
    let grants = resources(&[("input", &input)], &[("report", &report)]);
    run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants).unwrap();
    assert_eq!(fs::read(&report).unwrap(), "é\0 | new | 🙂".as_bytes());
    let output = native(&scratch.binary(&program, "Run"), &grants, &BTreeMap::new());
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(fs::read(report).unwrap(), "é\0 | new | 🙂".as_bytes());
}

#[test]
fn imports_qualify_all_transform_operands_and_preserve_local_frame_names() {
    let mut child = package();
    child.metadata.name = "transform-child".into();
    child.spec_text = with_functions(
        "Function: rewrite.\nThe function needs:\n- raw: Text\n- needle: Text\n- replacement: Text\nThe function produces:\n- result: Text\nThe function stores:\n- result: Text\nThe function executes:\n- the function replaces text local.needle in local.raw with local.replacement into local.result\n- the function returns local.result",
        "- the system trims text data.raw into data.raw\n- the system splits text data.raw by data.separator into data.parts\n- the system joins list data.parts with data.separator into output.raw\n- the system calls rewrite with raw = output.raw; needle = data.needle; replacement = data.replacement into output.raw\n- the system returns output.raw",
    );
    let mut parent = package();
    parent.spec_text="The application Parent manages records.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system returns child.output.raw\n- the system records a trace event named Done\n".into();
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
        ("child.data.raw", " data.raw::and "),
        ("child.data.separator", "::"),
        ("child.data.needle", "and"),
        ("child.data.replacement", "local.replacement"),
    ]);
    assert_eq!(
        run_ail_bytecode_action(&program, "Run", input.clone())
            .unwrap()
            .final_state["child.output.raw"],
        "data.raw::local.replacement"
    );
    let scratch = Scratch::new();
    let output = native(
        &scratch.binary(&program, "Run"),
        &resources(&[], &[]),
        &input,
    );
    assert!(output.status.success(), "{}", brief(&output));
    assert_eq!(
        output.stdout,
        b"child.output.raw=data.raw::local.replacement\n"
    );
}

#[test]
fn forged_global_and_local_operand_ownership_never_becomes_executable() {
    let original = core(&source(
        "- the system replaces text data.needle in data.raw with data.replacement into output.raw",
    ));
    for profile in ["Compiler", "UI", "AgentTool", "System"] {
        let mut candidate = original.clone();
        candidate.package.profile = profile.into();
        assert!(compile_ail_core_ste(&candidate).is_err());
    }
    for mutation in [
        "unowned_field",
        "typed_operation",
        "orphan_operation",
        "compiler_pass_owner",
    ] {
        let mut candidate = original.clone();
        match mutation {
            "unowned_field" => {
                let mut field = candidate
                    .graph
                    .nodes
                    .iter()
                    .find(|node| node.kind == "Field" && node.name == "Data.replacement")
                    .unwrap()
                    .clone();
                let original_id = field.id.clone();
                field.id = "forged-replacement-field".into();
                candidate.graph.nodes.push(field.clone());
                candidate
                    .graph
                    .edges
                    .iter_mut()
                    .find(|edge| edge.kind == "text_replacement" && edge.target == original_id)
                    .unwrap()
                    .target = field.id;
            }
            "typed_operation" => {
                candidate
                    .graph
                    .nodes
                    .iter_mut()
                    .find(|node| node.kind == "TextOperation")
                    .unwrap()
                    .type_name = Some("Text".into())
            }
            "orphan_operation" => {
                let mut node = candidate
                    .graph
                    .nodes
                    .iter()
                    .find(|node| node.kind == "TextOperation")
                    .unwrap()
                    .clone();
                node.id = "orphan-text-operation".into();
                candidate.graph.nodes.push(node);
            }
            "compiler_pass_owner" => {
                candidate
                    .graph
                    .nodes
                    .iter_mut()
                    .find(|node| node.kind == "Action")
                    .unwrap()
                    .attributes
                    .insert("kind".into(), "CompilerPass".into());
            }
            _ => unreachable!(),
        }
        assert!(compile_ail_core_bytecode(&candidate).is_err(), "{mutation}");
    }
    let function = "Function: clean.\nThe function needs:\n- raw: Text\nThe function produces:\n- result: Text\nThe function stores:\n- result: Text\nThe function executes:\n- the function trims text local.raw into local.result\n- the function returns local.result";
    let original = core(&with_functions(
        function,
        "- the system calls clean with raw = data.raw into output.raw",
    ));
    let local_step = original
        .graph
        .nodes
        .iter()
        .find(|node| {
            node.kind == "ValueFunctionStep"
                && node
                    .attributes
                    .get("opcode")
                    .is_some_and(|opcode| opcode == "TEXT_TRIM")
        })
        .unwrap();
    let operand = original
        .graph
        .edges
        .iter()
        .find(|edge| edge.kind == "value_field_operand" && edge.source == local_step.id)
        .unwrap();
    for mutation in [
        "global_operand",
        "foreign_local",
        "wrong_operand_type",
        "duplicate_role",
    ] {
        let mut candidate = original.clone();
        match mutation {
            "global_operand" => {
                let id = candidate
                    .graph
                    .nodes
                    .iter()
                    .find(|node| node.kind == "Field" && node.name == "Data.raw")
                    .unwrap()
                    .id
                    .clone();
                candidate
                    .graph
                    .edges
                    .iter_mut()
                    .find(|edge| edge.id == operand.id)
                    .unwrap()
                    .target = id;
            }
            "foreign_local" => {
                candidate
                    .graph
                    .nodes
                    .iter_mut()
                    .find(|node| node.id == operand.target)
                    .unwrap()
                    .attributes
                    .insert("function_owner".into(), "different".into());
            }
            "wrong_operand_type" => {
                candidate
                    .graph
                    .edges
                    .iter_mut()
                    .find(|edge| edge.id == operand.id)
                    .unwrap()
                    .attributes
                    .insert("type".into(), "Int".into());
            }
            "duplicate_role" => {
                let mut edge = operand.clone();
                edge.id = "duplicate-local-operand".into();
                edge.ordinal = candidate.graph.edges.len();
                candidate.graph.edges.push(edge);
            }
            _ => unreachable!(),
        }
        assert!(compile_ail_core_bytecode(&candidate).is_err(), "{mutation}");
    }
}
