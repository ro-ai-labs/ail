use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Read, Write};
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
            "ail-standard-streams-{}-{}",
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

fn state(values: &[(&str, &str)]) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(key, value)| ((*key).into(), (*value).into()))
        .collect()
}

fn command_output(command: &mut Command, input: &[u8]) -> Output {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
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
    let mut stdin = child.stdin.take().unwrap();
    let input = input.to_vec();
    let feeder = std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
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
                "Stdio execution blocked: {}",
                String::from_utf8_lossy(&stderr[..stderr.len().min(2_000)])
            );
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    feeder.join().unwrap();
    Output {
        status: child.wait().unwrap(),
        stdout: stdout_reader.join().unwrap(),
        stderr: stderr_reader.join().unwrap(),
    }
}

fn native(binary: &Path, input: &[u8]) -> Output {
    command_output(Command::new(binary).arg("--stdio"), input)
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

fn seed(key: &str, value: &str) -> AilBytecodeInstruction {
    command(
        "SET_FIELD",
        &[("key", key), ("value", value), ("text", "fixture")],
    )
}
fn read(key: &str) -> AilBytecodeInstruction {
    command("STDIN_READ_TEXT", &[("key", key)])
}
fn write(key: &str) -> AilBytecodeInstruction {
    command("STDOUT_WRITE_TEXT", &[("source", key)])
}
fn warning(key: &str) -> AilBytecodeInstruction {
    command("STDERR_WRITE_TEXT", &[("source", key)])
}
fn begin(key: &str) -> AilBytecodeInstruction {
    command("TRY_BEGIN", &[("errors", "TEXT"), ("key", key)])
}

fn vm(
    program: &AilBytecodeProgram,
    input: &[u8],
) -> (Result<AilRunResult, String>, Vec<u8>, Vec<u8>) {
    let mut input = Cursor::new(input);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = run_ail_bytecode_action_with_io(
        program,
        "Run",
        BTreeMap::new(),
        &AilFileResources::default(),
        &mut AilStandardStreams {
            stdin: Some(&mut input),
            stdout: Some(&mut out),
            stderr: Some(&mut err),
        },
    );
    (result, out, err)
}

#[test]
fn standard_streams_preserve_exact_bytes_through_core_source_ste_vm_and_native() {
    let original = core(&source(
        "- the system reads text from standard input into data.raw\n- the system writes text data.raw to standard output\n- the system assigns \"stderr λ\\u0000\\r\\n\" to data.other\n- the system writes text data.other to standard error\n- the system returns data.raw",
    ));
    assert_eq!(
        original
            .graph
            .nodes
            .iter()
            .filter(|node| node.kind == "StandardStreamOperation")
            .count(),
        3
    );
    let scratch = Scratch::new();
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = compile_ail_core_bytecode(&candidate).unwrap();
        let ste = ail::ste::render_ail_ste(&program).unwrap();
        assert!(ste.contains("Read text from standard input into field"));
        for program in [program, ail::ste::parse_ail_ste(&ste).unwrap()] {
            for bytes in [b"".as_slice(), "a\0λ🙂\r\nno final LF".as_bytes()] {
                let (run, out, err) = vm(&program, bytes);
                let run = run.unwrap();
                assert_eq!(run.final_state["data.raw"].as_bytes(), bytes);
                assert_eq!(out, bytes);
                assert_eq!(err, "stderr λ\0\r\n".as_bytes());
                assert!(!run.trace.is_empty());
                let output = native(&scratch.binary(&program, "Run"), bytes);
                assert!(output.status.success(), "{}", brief(&output));
                assert_eq!(output.stdout, bytes);
                assert_eq!(output.stderr, err);
            }
        }
    }
}

#[test]
fn endpoint_authority_is_independent_and_checked_before_values_or_input() {
    for operation in [
        read("data.raw"),
        write("missing.text"),
        warning("missing.text"),
    ] {
        let program = raw_program(vec![operation.clone()]);
        assert!(
            run_ail_bytecode_action(&program, "Run", BTreeMap::new())
                .unwrap_err()
                .contains("AIL-RUNTIME-CAPABILITY:")
        );
        assert!(
            run_ail_bytecode_action_with_files(
                &program,
                "Run",
                state(&[("stdio", "true"), ("--stdio", "true")]),
                &AilFileResources::default()
            )
            .unwrap_err()
            .contains("AIL-RUNTIME-CAPABILITY:")
        );
        let mut input = Cursor::new(b"unconsumed".to_vec());
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut streams = AilStandardStreams {
            stdin: Some(&mut input),
            stdout: Some(&mut out),
            stderr: Some(&mut err),
        };
        match operation.opcode.as_str() {
            "STDIN_READ_TEXT" => streams.stdin = None,
            "STDOUT_WRITE_TEXT" => streams.stdout = None,
            _ => streams.stderr = None,
        };
        let denied = run_ail_bytecode_action_with_io(
            &program,
            "Run",
            BTreeMap::new(),
            &AilFileResources::default(),
            &mut streams,
        )
        .unwrap_err();
        assert!(denied.contains("AIL-RUNTIME-CAPABILITY:"), "{denied}");
        assert_eq!(input.position(), 0);
        assert!(out.is_empty() && err.is_empty());
    }
    let mut input = Cursor::new(b"read only".to_vec());
    let result = run_ail_bytecode_action_with_io(
        &raw_program(vec![read("data.raw")]),
        "Run",
        BTreeMap::new(),
        &AilFileResources::default(),
        &mut AilStandardStreams {
            stdin: Some(&mut input),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.final_state["data.raw"], "read only");
    for stderr in [false, true] {
        let mut sink = Vec::new();
        let mut streams = AilStandardStreams::default();
        if stderr {
            streams.stderr = Some(&mut sink)
        } else {
            streams.stdout = Some(&mut sink)
        }
        let program = raw_program(vec![
            seed("data.raw", "only endpoint"),
            if stderr {
                warning("data.raw")
            } else {
                write("data.raw")
            },
        ]);
        run_ail_bytecode_action_with_io(
            &program,
            "Run",
            BTreeMap::new(),
            &AilFileResources::default(),
            &mut streams,
        )
        .unwrap();
        assert_eq!(sink, b"only endpoint");
    }
}

struct Reader {
    bytes: Vec<u8>,
    position: usize,
    interrupted: bool,
    error_at: Option<usize>,
}
impl Read for Reader {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.interrupted {
            self.interrupted = false;
            return Err(std::io::ErrorKind::Interrupted.into());
        }
        if self.error_at.is_some_and(|at| self.position >= at) {
            return Err(std::io::ErrorKind::WouldBlock.into());
        }
        let count = out.len().min(1).min(self.bytes.len() - self.position);
        out[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}
#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
    calls: usize,
    flushes: usize,
    interrupted: bool,
    zero: bool,
    fail_after: Option<usize>,
    flush_error: bool,
}
impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.calls += 1;
        if self.interrupted {
            self.interrupted = false;
            return Err(std::io::ErrorKind::Interrupted.into());
        }
        if self.zero {
            return Ok(0);
        }
        if self.fail_after.is_some_and(|at| self.bytes.len() >= at) {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
        let count = bytes.len().min(2);
        self.bytes.extend_from_slice(&bytes[..count]);
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.flushes += 1;
        if self.flush_error {
            Err(std::io::ErrorKind::Other.into())
        } else {
            Ok(())
        }
    }
}

#[test]
fn injected_short_io_interrupted_zero_progress_and_flush_errors_follow_contract() {
    let program = raw_program(vec![read("data.raw"), write("data.raw"), write("data.raw")]);
    let bytes = "aλ🙂\0\n".as_bytes();
    let mut input = Reader {
        bytes: bytes.to_vec(),
        position: 0,
        interrupted: true,
        error_at: None,
    };
    let mut out = Writer {
        interrupted: true,
        ..Default::default()
    };
    run_ail_bytecode_action_with_io(
        &program,
        "Run",
        BTreeMap::new(),
        &AilFileResources::default(),
        &mut AilStandardStreams {
            stdin: Some(&mut input),
            stdout: Some(&mut out),
            stderr: None,
        },
    )
    .unwrap();
    assert_eq!(out.bytes, bytes.repeat(2));
    assert_eq!(out.flushes, 2);
    for mut out in [
        Writer {
            zero: true,
            ..Default::default()
        },
        Writer {
            fail_after: Some(2),
            ..Default::default()
        },
        Writer {
            flush_error: true,
            ..Default::default()
        },
    ] {
        let program = raw_program(vec![
            seed("data.raw", "abcdef"),
            command("TRY_BEGIN", &[("errors", "any"), ("key", "data.error")]),
            write("data.raw"),
            command("RECOVER", &[]),
            command("TRY_END", &[]),
        ]);
        let error = run_ail_bytecode_action_with_io(
            &program,
            "Run",
            BTreeMap::new(),
            &AilFileResources::default(),
            &mut AilStandardStreams {
                stdout: Some(&mut out),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(error.contains("AIL-RUNTIME-IO:"), "{error}");
        if out.zero {
            assert!(out.bytes.is_empty())
        } else if out.flush_error {
            assert_eq!(out.bytes, b"abcdef")
        } else {
            assert_eq!(out.bytes, b"ab")
        }
    }
    let mut out = Writer {
        zero: true,
        flush_error: true,
        ..Default::default()
    };
    run_ail_bytecode_action_with_io(
        &raw_program(vec![seed("data.raw", ""), write("data.raw")]),
        "Run",
        BTreeMap::new(),
        &AilFileResources::default(),
        &mut AilStandardStreams {
            stdout: Some(&mut out),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!((out.calls, out.flushes), (0, 0));
    let mut input = Reader {
        bytes: b"abcdef".to_vec(),
        position: 0,
        interrupted: false,
        error_at: Some(2),
    };
    let error = run_ail_bytecode_action_with_io(
        &raw_program(vec![read("data.raw")]),
        "Run",
        BTreeMap::new(),
        &AilFileResources::default(),
        &mut AilStandardStreams {
            stdin: Some(&mut input),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(error.contains("AIL-RUNTIME-IO:"));
    assert_eq!(input.position, 2);
}

#[test]
fn catchable_bad_utf8_preserves_destination_consumes_input_and_keeps_streams_open() {
    let program = raw_program(vec![
        seed("data.raw", "prior"),
        seed("data.other", "before"),
        write("data.other"),
        begin("data.error"),
        read("data.raw"),
        command("RECOVER", &[]),
        write("data.error"),
        command("TRY_END", &[]),
        write("data.raw"),
        read("data.other"),
        write("data.other"),
    ]);
    let scratch = Scratch::new();
    let binary = scratch.binary(&program, "Run");
    for bytes in [
        vec![0x80],
        vec![0xc0, 0x80],
        vec![0xed, 0xa0, 0x80],
        vec![0xf4, 0x90, 0x80, 0x80],
        vec![0xf0, 0x9f, 0x99],
        vec![b'a', 0xff],
    ] {
        let (result, out, err) = vm(&program, &bytes);
        let result = result.unwrap();
        assert_eq!(result.final_state["data.raw"], "prior");
        assert_eq!(result.final_state["data.other"], "");
        assert_eq!(out, b"beforeAIL-RUNTIME-TEXTprior");
        assert!(err.is_empty());
        let output = native(&binary, &bytes);
        assert!(output.status.success(), "{}", brief(&output));
        assert_eq!(output.stdout, out);
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn read_size_utf8_and_shared_arena_checks_have_exact_priority() {
    const LIMIT: usize = 1_048_576;
    let scratch = Scratch::new();
    let program = raw_program(vec![read("data.raw"), write("data.raw")]);
    let binary = scratch.binary(&program, "Run");
    let bytes = vec![b'x'; LIMIT];
    assert_eq!(vm(&program, &bytes).1, bytes);
    let out = native(&binary, &bytes);
    assert!(out.status.success(), "{}", brief(&out));
    assert_eq!(out.stdout, bytes);
    for mut bytes in [vec![b'x'; LIMIT + 1], vec![b'x'; LIMIT + 2]] {
        bytes[0] = 0xff;
        let (result, out, err) = vm(&program, &bytes);
        assert!(result.unwrap_err().contains("AIL-RUNTIME-TEXT-LIMIT:"));
        assert!(out.is_empty() && err.is_empty());
        failed(&native(&binary, &bytes), "AIL-RUNTIME-TEXT-LIMIT:");
    }
    let mut steps = vec![
        seed("data.large", &"x".repeat(LIMIT)),
        seed("data.empty", ""),
    ];
    for _ in 0..63 {
        steps.push(command(
            "TEXT_JOIN",
            &[
                ("key", "data.fill"),
                ("left", "data.large"),
                ("right", "data.empty"),
            ],
        ))
    }
    steps.extend([read("data.raw"), write("data.raw"), read("data.empty")]);
    let program = raw_program(steps);
    let (result, out, _) = vm(&program, &bytes);
    assert!(result.unwrap_err().contains("AIL-RUNTIME-TEXT-LIMIT:"));
    assert_eq!(out, bytes);
    let native = native(&scratch.binary(&program, "Run"), &bytes);
    assert!(!native.status.success());
    assert_eq!(native.stdout, bytes);
    assert!(String::from_utf8_lossy(&native.stderr).contains("AIL-RUNTIME-TEXT-LIMIT:"));
}

#[test]
fn called_actions_share_authority_cursor_and_preserve_read_aliases() {
    let mut program = raw_program(vec![
        command("CALL_ACTION", &[("target", "Read")]),
        command(
            "COPY_FIELD",
            &[
                ("key", "data.alias"),
                ("source", "data.raw"),
                ("text", "copy"),
            ],
        ),
        command("CALL_ACTION", &[("target", "Read")]),
        write("data.alias"),
        write("data.raw"),
    ]);
    program.actions.insert(
        "Read".into(),
        AilBytecodeAction {
            name: "Read".into(),
            instructions: vec![read("data.raw"), command("RETURN_SUCCESS", &[])],
        },
    );
    let (result, out, err) = vm(&program, b"first");
    let result = result.unwrap();
    assert_eq!(result.final_state["data.raw"], "");
    assert_eq!(out, b"first");
    assert!(err.is_empty());
    let scratch = Scratch::new();
    let out = native(&scratch.binary(&program, "Run"), b"first");
    assert!(out.status.success(), "{}", brief(&out));
    assert_eq!(out.stdout, b"first");
    assert!(out.stderr.is_empty());
}

#[test]
fn cli_and_both_native_emitters_are_quiet_only_in_exact_stdio_mode() {
    let scratch = Scratch::new();
    for program in [
        raw_program(vec![
            seed("data.value", "23"),
            command("EMIT_TRACE", &[("event", "VisibleWithoutMode")]),
        ]),
        raw_program(vec![command(
            "EMIT_TRACE",
            &[("event", "LegacyWithoutMode")],
        )]),
    ] {
        let artifact = scratch.file("program.json", render_ail_bytecode(&program).as_bytes());
        let binary = scratch.binary(&program, "Run");
        for native_backend in [false, true] {
            let make = || {
                let mut c = if native_backend {
                    Command::new(&binary)
                } else {
                    let mut c = Command::new(env!("CARGO_BIN_EXE_ail"));
                    c.arg("ail-vm").arg(&artifact).args(["--action", "Run"]);
                    c
                };
                c.arg("data.input=7");
                c
            };
            let ordinary = command_output(&mut make(), b"");
            assert!(ordinary.status.success());
            assert!(!ordinary.stdout.is_empty() || !ordinary.stderr.is_empty());
            let quiet = command_output(make().args(["--stdio", "--stdio"]), b"");
            assert!(quiet.status.success(), "{}", brief(&quiet));
            assert!(
                quiet.stdout.is_empty() && quiet.stderr.is_empty(),
                "{}",
                brief(&quiet)
            );
            for flag in ["--stdio=true", "--stdio=", "--stdioX", "--stdio=false"] {
                let output = command_output(make().args(["--stdio", flag]), b"");
                failed(&output, "AIL-RUNTIME-CAPABILITY:");
            }
        }
    }
}

#[test]
fn malformed_artifact_diagnostics_in_stdio_mode_never_pollute_stdout() {
    let scratch = Scratch::new();
    let mut program = raw_program(vec![read("data.raw")]);
    program.actions.get_mut("Run").unwrap().instructions[0]
        .operands
        .insert("extra".into(), "ignored".into());
    let invalid = scratch.file("invalid.json", render_ail_bytecode(&program).as_bytes());
    let bad_json = scratch.file("broken.json", b"{");
    let bad_ste = scratch.file("broken.ail-ste", b"AIL-STE broken");
    for path in [&invalid, &bad_json, &bad_ste] {
        let output = command_output(
            Command::new(env!("CARGO_BIN_EXE_ail"))
                .arg("ail-vm")
                .arg(path)
                .args(["--action", "Run", "--stdio"]),
            b"unconsumed",
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty(), "{}", brief(&output));
        assert!(!output.stderr.is_empty());
    }
    let bad_core = scratch.file("broken.core", b"not a Core graph");
    let output = command_output(
        Command::new(env!("CARGO_BIN_EXE_ail"))
            .args(["ail-run", "--core-file"])
            .arg(bad_core)
            .args(["--action", "Run", "--stdio"]),
        b"",
    );
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn quiet_mode_preserves_rule_failure_and_return_validation_after_explicit_output() {
    let scratch = Scratch::new();
    let mut rule = raw_program(vec![
        seed("data.raw", "prior bytes"),
        write("data.raw"),
        command(
            "REQUIRE_EXISTS",
            &[
                ("key", "missing.value"),
                ("rule", "field exists"),
                ("failure", "MissingRecord"),
            ],
        ),
    ]);
    rule.failures.insert(
        "MissingRecord".into(),
        AilBytecodeFailure {
            name: "MissingRecord".into(),
            traces: vec!["HiddenFailureTrace".into()],
        },
    );
    let (result, out, err) = vm(&rule, b"");
    assert_eq!(result.unwrap().status, "failed");
    assert_eq!(out, b"prior bytes");
    assert!(err.is_empty());
    let missing_return = raw_program(vec![
        seed("data.raw", "prior bytes"),
        write("data.raw"),
        command("OUTPUT_FIELD", &[("key", "missing.returned")]),
    ]);
    for (program, marker) in [
        (rule, "MissingRecord"),
        (missing_return, "AIL-RUNTIME-OUTPUT:"),
    ] {
        let artifact = scratch.file("failed.json", render_ail_bytecode(&program).as_bytes());
        let binary = scratch.binary(&program, "Run");
        for native_backend in [false, true] {
            let output = if native_backend {
                native(&binary, b"")
            } else {
                command_output(
                    Command::new(env!("CARGO_BIN_EXE_ail"))
                        .arg("ail-vm")
                        .arg(&artifact)
                        .args(["--action", "Run", "--stdio"]),
                    b"",
                )
            };
            assert!(!output.status.success(), "{}", brief(&output));
            assert_eq!(output.stdout, b"prior bytes");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains(marker), "{}", brief(&output));
            assert!(!error.contains("HiddenFailureTrace"), "{}", brief(&output));
        }
    }
}

#[test]
fn large_bidirectional_pipes_drain_concurrently_without_automatic_traces() {
    let chunk = "λ".repeat(262_144);
    let input = "🙂".repeat(196_608);
    let program = raw_program(vec![
        seed("data.prefix", &chunk),
        write("data.prefix"),
        warning("data.prefix"),
        read("data.raw"),
        write("data.raw"),
    ]);
    let scratch = Scratch::new();
    let binary = scratch.binary(&program, "Run");
    let out = native(&binary, input.as_bytes());
    assert!(out.status.success(), "{}", brief(&out));
    assert_eq!(out.stdout, [chunk.as_bytes(), input.as_bytes()].concat());
    assert_eq!(out.stderr, chunk.as_bytes());
    let artifact = scratch.file("large.json", render_ail_bytecode(&program).as_bytes());
    let out = command_output(
        Command::new(env!("CARGO_BIN_EXE_ail"))
            .arg("ail-vm")
            .arg(artifact)
            .args(["--action", "Run", "--stdio"]),
        input.as_bytes(),
    );
    assert!(out.status.success(), "{}", brief(&out));
    assert_eq!(out.stdout, [chunk.as_bytes(), input.as_bytes()].concat());
    assert_eq!(out.stderr, chunk.as_bytes());
}

#[test]
fn stream_core_types_ownership_secret_access_and_profile_cannot_be_forged() {
    let original = core(&source(
        "- the system reads text from standard input into data.raw\n- the system writes text data.raw to standard output\n- the system writes text data.raw to standard error",
    ));
    for relation in ["stream_destination", "stream_source", "stream_step"] {
        let edge = original
            .graph
            .edges
            .iter()
            .find(|e| e.kind == relation)
            .unwrap();
        for duplicate in [false, true] {
            let mut candidate = original.clone();
            if duplicate {
                let mut forged = edge.clone();
                forged.id = format!("duplicate-{relation}");
                forged.ordinal = candidate.graph.edges.len();
                candidate.graph.edges.push(forged)
            } else {
                candidate
                    .graph
                    .edges
                    .iter_mut()
                    .find(|e| e.id == edge.id)
                    .unwrap()
                    .attributes
                    .insert("extra".into(), "ignored".into());
            }
            assert!(compile_ail_core_bytecode(&candidate).is_err());
            if let Ok(saved) = parse_ail_core_text(&render_ail_core(&candidate)) {
                assert!(compile_ail_core_bytecode(&saved).is_err());
            }
        }
    }
    for profile in ["Compiler", "AgentTool", "UI", "System"] {
        let mut candidate = original.clone();
        candidate.package.profile = profile.into();
        assert!(compile_ail_core_ste(&candidate).is_err());
        let mut program = compile_ail_core_bytecode(&original).unwrap();
        program.profile = profile.into();
        assert!(!verify_ail_bytecode(&program).is_empty());
    }
    let field = original
        .graph
        .edges
        .iter()
        .find(|e| e.kind == "stream_destination")
        .unwrap()
        .target
        .clone();
    for secret in [false, true] {
        let mut candidate = original.clone();
        let field = candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.id == field)
            .unwrap();
        if secret {
            field.attributes.insert("secret".into(), "true".into());
            candidate
                .graph
                .edges
                .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        } else {
            field.type_name = Some("Int".into());
        }
        let errors = check_ail_core(&candidate);
        assert!(!errors.is_empty());
        if secret {
            assert!(errors.iter().any(|e| e.contains("AIL002")));
            assert!(errors.iter().any(|e| e.contains("AIL005")));
        }
        assert!(compile_ail_core_bytecode(&candidate).is_err());
    }
    for opcode in ["STDIN_READ_TEXT", "STDOUT_WRITE_TEXT", "STDERR_WRITE_TEXT"] {
        for operands in [
            vec![],
            vec![("key", "data.raw"), ("source", "data.raw")],
            vec![(
                if opcode == "STDIN_READ_TEXT" {
                    "key"
                } else {
                    "source"
                },
                "bare",
            )],
        ] {
            let program = raw_program(vec![command(opcode, &operands)]);
            assert!(!verify_ail_bytecode(&program).is_empty());
            assert!(compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").is_err());
        }
    }
}

#[test]
fn pure_legacy_and_compiler_bodies_reject_recognized_stream_effects() {
    let compiler = load_ail_package_dir("examples/compiler_pass.ail").unwrap();
    for statement in [
        "reads text from standard input into local.raw",
        "writes text local.raw to standard output",
        "writes text local.raw to standard error",
        "reads text from standard input",
        "writes text local.raw to standard output extra",
    ] {
        for period in ["", "."] {
            for subject in ["the system", "the function"] {
                let bullet = format!("{subject} {statement}{period}");
                let pure = with_functions(
                    &format!(
                        "Function: bad.\nThe function produces:\n- result: Text\nThe function stores:\n- raw: Text\nThe function executes:\n- {bullet}\n- the function returns local.raw"
                    ),
                    "- the system assigns 1 to output.value",
                );
                let legacy = format!(
                    "{DECLARATIONS}Function: old.\nThe function produces:\n- result: Text\nWhen old runs:\n- {bullet}\n- the function returns raw unchanged\n"
                );
                let pass = compiler.spec_text.replace(
                    "- the system reads every edge",
                    &format!("- {bullet}\n- the system reads every edge"),
                );
                for (package, text) in [(&package(), pure), (&package(), legacy), (&compiler, pass)]
                {
                    let rejected = match parse_ail_package_spec_text(package, &text) {
                        Err(_) => true,
                        Ok(document) => {
                            compile_ail_core_bytecode(&elaborate_ail_core(package, &document))
                                .is_err()
                        }
                    };
                    assert!(rejected, "accepted {bullet}");
                }
            }
        }
    }
}

#[test]
fn imported_streams_qualify_fields_but_keep_standard_stream_identity() {
    let mut child = package();
    child.metadata.name = "stream-child".into();
    child.spec_text = source(
        "- the system reads text from standard input into data.raw\n- the system writes text data.raw to standard output",
    );
    let mut parent = package();
    parent.spec_text="The application Parent manages streams.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system records a trace event named Done\n".into();
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
    let core = elaborate_ail_core(&parent, &parse_ail_package_document(&parent).unwrap());
    let program = ail::ste::parse_ail_ste(
        &compile_ail_core_ste(&parse_ail_core_text(&render_ail_core(&core)).unwrap()).unwrap(),
    )
    .unwrap();
    let (result, out, err) = vm(&program, b"import\0");
    assert_eq!(result.unwrap().final_state["child.data.raw"], "import\0");
    assert_eq!(out, b"import\0");
    assert!(err.is_empty());
    let scratch = Scratch::new();
    let native = native(&scratch.binary(&program, "Run"), b"import\0");
    assert!(native.status.success(), "{}", brief(&native));
    assert_eq!(native.stdout, out);
    assert!(native.stderr.is_empty());
}
