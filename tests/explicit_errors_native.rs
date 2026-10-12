//! Native explicit raises use the same category/unwind paths as computed errors.
#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
use ail::ail::*;
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

fn i(opcode: &str, operands: &[(&str, &str)]) -> AilBytecodeInstruction {
    AilBytecodeInstruction {
        opcode: opcode.into(),
        operands: operands
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect(),
    }
}
fn seed(key: &str, value: &str) -> AilBytecodeInstruction {
    i(
        "SET_FIELD",
        &[("key", key), ("value", value), ("text", "fixture")],
    )
}
fn raise(category: &str) -> AilBytecodeInstruction {
    i("RAISE", &[("category", category)])
}
fn begin(category: &str, key: &str) -> AilBytecodeInstruction {
    i("TRY_BEGIN", &[("errors", category), ("key", key)])
}
fn select(key: &str) -> AilBytecodeInstruction {
    i("OUTPUT_FIELD", &[("key", key)])
}
fn core(source: &str) -> AilCore {
    let p = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    elaborate_ail_core(&p, &parse_ail_package_spec_text(&p, source).unwrap())
}
fn program(body: Vec<AilBytecodeInstruction>) -> AilBytecodeProgram {
    let mut p=compile_ail_core_bytecode(&core("The application Raises manages errors.\nA Data has:\n- value: Int\nAction: Run.\nWhen Run happens:\n- the system assigns 1 to data.value\n- the system records a trace event named Done\n")).unwrap();
    p.actions.get_mut("Run").unwrap().instructions = body;
    p
}
fn routine(p: &mut AilBytecodeProgram, name: &str, body: Vec<AilBytecodeInstruction>) {
    p.actions.insert(
        name.into(),
        AilBytecodeAction {
            name: name.into(),
            instructions: body,
        },
    );
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "ail-explicit-errors-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn run(&self, p: &AilBytecodeProgram, args: &[String]) -> Output {
        let path = self.0.join("program.elf");
        fs::write(
            &path,
            compile_ail_bytecode_native_elf(p, "Run", "linux-x86_64-elf").unwrap(),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let mut child = Command::new(path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let out = std::thread::spawn(move || {
            let mut b = Vec::new();
            stdout.read_to_end(&mut b).unwrap();
            b
        });
        let err = std::thread::spawn(move || {
            let mut b = Vec::new();
            stderr.read_to_end(&mut b).unwrap();
            b
        });
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("native raise exceeded ten seconds");
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        Output {
            status: child.wait().unwrap(),
            stdout: out.join().unwrap(),
            stderr: err.join().unwrap(),
        }
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn success(p: &AilBytecodeProgram, expected: &[u8]) {
    let output = Scratch::new().run(p, &[]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, expected);
}

#[test]
fn all_fourteen_unmatched_raises_fail_without_automatic_output_or_later_trace() {
    for category in [
        "INT",
        "OVERFLOW",
        "DIVZERO",
        "FLOAT",
        "BOOL",
        "TEXT",
        "LIST",
        "INDEX",
        "JSON",
        "JSON-PATH",
        "PROCESS",
        "TIMEOUT",
        "PATH",
        "BYTES",
    ] {
        let p = program(vec![
            seed("out.value", "7"),
            select("out.value"),
            raise(category),
            i("EMIT_TRACE", &[("event", "MustNotRun")]),
            i("RETURN_SUCCESS", &[]),
        ]);
        let output = Scratch::new().run(&p, &["--stdio".into()]);
        assert_eq!(output.status.code(), Some(1), "{category}: {output:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.starts_with(&format!("AIL-RUNTIME-{category}:")),
            "{stderr}"
        );
        assert!(!stderr.contains("MustNotRun"));
    }
}
#[test]
fn nearest_matching_handler_and_reraise_skip_inactive_recovery_body() {
    let p = program(vec![
        seed("out.value", "7"),
        begin("INDEX", "out.outer"),
        begin("BYTES", "out.inner"),
        begin("JSON", "out.unmatched"),
        raise("BYTES"),
        i("RECOVER", &[]),
        seed("out.value", "100"),
        i("TRY_END", &[]),
        i("RECOVER", &[]),
        raise("INDEX"),
        seed("out.value", "101"),
        i("TRY_END", &[]),
        i("RECOVER", &[]),
        i("TRY_END", &[]),
        select("out.value"),
        select("out.inner"),
        select("out.outer"),
        i("RETURN_SUCCESS", &[]),
    ]);
    success(
        &p,
        b"out.inner=AIL-RUNTIME-BYTES\nout.outer=AIL-RUNTIME-INDEX\nout.value=7\n",
    );
}
#[test]
fn repeated_deep_integer_and_bytes_calls_restore_depth_and_preserve_destinations() {
    for value in [false, true] {
        let opcode = if value { "CALL_VALUE" } else { "CALL_INT" };
        let mut body = vec![seed("out.value", if value { "hex:aa" } else { "7" })];
        for _ in 0..3 {
            body.extend([
                begin("BYTES", "out.error"),
                i(
                    opcode,
                    &[
                        ("target", "deep"),
                        ("arguments", "n=i:63"),
                        ("key", "out.value"),
                    ],
                ),
                i("RECOVER", &[]),
                i("TRY_END", &[]),
            ]);
        }
        body.extend([
            select("out.value"),
            select("out.error"),
            i("RETURN_SUCCESS", &[]),
        ]);
        let mut p = program(body);
        let marker = if value {
            i(
                "VALUE_FUNCTION",
                &[
                    ("parameters", "n:Int"),
                    ("locals", "unused:Bytes"),
                    ("result", "Bytes"),
                ],
            )
        } else {
            i("INTEGER_FUNCTION", &[("parameters", "n")])
        };
        routine(
            &mut p,
            "deep",
            vec![
                marker,
                i(
                    "BRANCH_INT",
                    &[
                        ("left", "f:local.n"),
                        ("comparison", "=="),
                        ("right", "i:0"),
                        ("label", "base"),
                    ],
                ),
                i(
                    opcode,
                    &[
                        ("target", "deep"),
                        ("arguments", "n=f:local.n i:1 -"),
                        ("key", "local.unused"),
                    ],
                ),
                raise("TEXT"),
                i("LABEL", &[("name", "base")]),
                raise("BYTES"),
            ],
        );
        success(
            &p,
            if value {
                b"out.error=AIL-RUNTIME-BYTES\nout.value=hex:aa\n"
            } else {
                b"out.error=AIL-RUNTIME-BYTES\nout.value=7\n"
            },
        );
    }
}
#[test]
fn value_handler_reraise_unwinds_function_without_publishing_a_return() {
    let mut p = program(vec![
        seed("out.value", "hex:00ff"),
        begin("INDEX", "out.error"),
        i(
            "CALL_VALUE",
            &[("target", "fail"), ("arguments", ""), ("key", "out.value")],
        ),
        i("RECOVER", &[]),
        i("TRY_END", &[]),
        select("out.value"),
        select("out.error"),
        i("RETURN_SUCCESS", &[]),
    ]);
    routine(
        &mut p,
        "fail",
        vec![
            i(
                "VALUE_FUNCTION",
                &[
                    ("parameters", ""),
                    ("locals", "error:Text"),
                    ("result", "Bytes"),
                ],
            ),
            begin("BYTES", "local.error"),
            raise("BYTES"),
            i("RECOVER", &[]),
            raise("INDEX"),
            i("TRY_END", &[]),
            raise("TEXT"),
        ],
    );
    success(&p, b"out.error=AIL-RUNTIME-INDEX\nout.value=hex:00ff\n");
}
#[test]
fn action_raise_restores_output_selection_and_preserves_only_prior_file_effects() {
    let scratch = Scratch::new();
    let path = scratch.0.join("effect.bin");
    fs::write(&path, b"original").unwrap();
    let mut p = program(vec![
        seed("out.keep", "7"),
        seed("data.before", "before"),
        seed("data.after", "after"),
        select("out.keep"),
        begin("BYTES", "out.error"),
        i("CALL_ACTION", &[("target", "Child")]),
        i("RECOVER", &[]),
        i("TRY_END", &[]),
        select("out.error"),
        select("out.prior"),
        i("RETURN_SUCCESS", &[]),
    ]);
    routine(
        &mut p,
        "Child",
        vec![
            seed("out.prior", "11"),
            select("out.absent"),
            i(
                "FILE_WRITE_TEXT",
                &[("resource", "effect"), ("source", "data.before")],
            ),
            raise("BYTES"),
            i(
                "FILE_WRITE_TEXT",
                &[("resource", "effect"), ("source", "data.after")],
            ),
            i("RETURN_SUCCESS", &[]),
        ],
    );
    let output = scratch.run(&p, &[format!("--write-file=effect={}", path.display())]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        output.stdout,
        b"out.error=AIL-RUNTIME-BYTES\nout.keep=7\nout.prior=11\n"
    );
    assert_eq!(fs::read(path).unwrap(), b"before");
}
#[test]
fn malformed_raises_in_dead_code_fail_native_verification() {
    for category in [
        "any",
        "BYTES-LIMIT",
        "IO",
        "DEPTH",
        "bytes",
        "BYTES;TEXT",
        "BYTES or TEXT",
        "data.category",
        "",
    ] {
        let p = program(vec![i("RETURN_SUCCESS", &[]), raise(category)]);
        let error = compile_ail_bytecode_native_elf(&p, "Run", "linux-x86_64-elf").unwrap_err();
        assert!(error.contains("AILBC019"), "{category}: {error}");
    }
    for bad in [
        i("RAISE", &[]),
        i("RAISE", &[("category", "BYTES"), ("message", "hidden")]),
    ] {
        let p = program(vec![i("RETURN_SUCCESS", &[]), bad]);
        assert!(
            compile_ail_bytecode_native_elf(&p, "Run", "linux-x86_64-elf")
                .unwrap_err()
                .contains("AILBC019")
        );
    }
}

#[test]
fn raise_only_integer_and_bytes_functions_survive_source_core_and_ste() {
    let source = "The application Raises manages errors.\n\nA Data has:\n- value: Int\n- raw: Bytes\n- error: Text\n\nFunction: failInt.\nThe function needs:\nThe function produces:\n- result: Int\nThe function executes:\n- the function raises error INT\n\nFunction: failBytes.\nThe function needs:\nThe function produces:\n- result: Bytes\nThe function executes:\n- the function raises error BYTES\n\nAction: Run.\nWhen Run happens:\n- the system assigns 7 to data.value\n- the system assigns \"hex:00ff\" to data.raw\n- try:\n- the system calls failInt with none into data.value\n- recover INT into data.error:\n- end try\n- try:\n- the system calls failBytes with none into data.raw\n- recover BYTES into data.error:\n- end try\n- the system returns data.value\n- the system returns data.raw\n- the system returns data.error\n- the system records a trace event named Done\n";
    let original = core(source);
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let direct = compile_ail_core_bytecode(&candidate).unwrap();
        let through_ste =
            ail::ste::parse_ail_ste(&ail::ste::render_ail_ste(&direct).unwrap()).unwrap();
        for p in [direct, through_ste] {
            success(
                &p,
                b"data.error=AIL-RUNTIME-BYTES\ndata.raw=hex:00ff\ndata.value=7\n",
            );
        }
    }
}
