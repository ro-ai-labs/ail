//! Bounded, explicitly authorized standard streams for standalone native code.
use super::*;

const MODE: u8 = 72;

pub(super) fn skip(code: &mut X64Code, target: &str) {
    code.emit(&[0x48, 0x83, 0x7d, MODE, 0]); // cmp qword [rbp+72], 0
    code.emit_jcc_label(&[0x0f, 0x85], target);
}

pub(super) fn legacy_initialize(code: &mut X64Code) -> Result<(), String> {
    // The legacy caller has already captured argc/argv in RBX/R12/R13. RBP
    // was previously unused. Preserve argv and RBX/R12–R15.
    code.emit(&[
        0x48, 0x83, 0xec, 80, // sub rsp, 80
        0x48, 0x89, 0xe5, // mov rbp, rsp
        0x48, 0x89, 0xe7, // mov rdi, rsp
        0x31, 0xc0, // xor eax, eax
        0xb9, 10, 0, 0, 0, // mov ecx, 10
        0xf3, 0x48, 0xab, // rep stosq
    ]);
    initialize(code)
}

/// Inline startup works with either emitter and owns its error/data labels.
/// Validate arguments before ordinary mode setup or program effects. Comparing
/// bytes individually avoids reading beyond a short argument's terminating NUL.
/// Terminal diagnostics independently protect against a broken stderr pipe.
pub(super) fn initialize(code: &mut X64Code) -> Result<(), String> {
    code.emit(&[0x4d, 0x89, 0xe0, 0x4d, 0x89, 0xe9]); // r8=argv; r9=count
    code.label("stdio_startup_arg")?;
    code.emit(&[0x4d, 0x85, 0xc9]); // test r9, r9
    code.emit_jcc_label(&[0x0f, 0x84], "stdio_startup_scanned");
    code.emit(&[0x49, 0x8b, 0x38]); // mov rdi, [r8]
    for (offset, byte) in b"--stdio".iter().enumerate() {
        code.emit(&[0x80, 0x7f, offset as u8, *byte]);
        code.emit_jcc_label(&[0x0f, 0x85], "stdio_startup_next");
    }
    code.emit(&[0x80, 0x7f, 7, 0]); // full token must end here
    code.emit_jcc_label(&[0x0f, 0x85], "stdio_startup_bad_flag");
    code.emit(&[0x48, 0xc7, 0x45, MODE, 1, 0, 0, 0]);
    code.label("stdio_startup_next")?;
    code.emit(&[0x49, 0x83, 0xc0, 8, 0x49, 0xff, 0xc9]); // next argv; --count
    code.emit_jmp_label("stdio_startup_arg");
    code.label("stdio_startup_scanned")?;
    code.emit(&[0x48, 0x83, 0x7d, MODE, 0]);
    code.emit_jcc_label(&[0x0f, 0x84], "stdio_startup_done");
    ignore_sigpipe(code);
    code.emit(&[0x48, 0x85, 0xc0]);
    code.emit_jcc_label(&[0x0f, 0x88], "stdio_startup_bad_signal");
    code.emit_jmp_label("stdio_startup_done");
    for (label, message) in [
        (
            "stdio_startup_bad_flag",
            "AIL-RUNTIME-CAPABILITY: malformed --stdio option\n",
        ),
        (
            "stdio_startup_bad_signal",
            "AIL-RUNTIME-IO: unable to configure standard streams\n",
        ),
    ] {
        code.label(label)?;
        // A malformed flag can arrive before the mode is enabled, or without
        // any valid flag. Protect only this terminal path; never grant access.
        // If protection fails, exit directly instead of risking signal death
        // while trying to report the setup error through an unprotected pipe.
        ignore_sigpipe(code);
        code.emit(&[0x48, 0x85, 0xc0]);
        code.emit_jcc_label(&[0x0f, 0x88], "stdio_startup_exit");
        code.emit(&[0xbf, 2, 0, 0, 0]);
        code.emit_lea_rsi_label(&format!("{label}_message"));
        code.emit_mov_edx_imm32(message.len() as u32);
        code.emit_call_label("stdio_startup_diagnostic");
        code.emit_exit(1);
        code.label(format!("{label}_message"))?;
        code.emit(message.as_bytes());
    }
    code.label("stdio_startup_exit")?;
    code.emit_exit(1);
    emit_write_loop(code, "stdio_startup_diagnostic", false)?;
    code.label("stdio_startup_sigaction")?;
    for value in [1_u64, 0, 0, 0] {
        code.emit(&value.to_le_bytes());
    }
    code.label("stdio_startup_done")?;
    Ok(())
}

fn ignore_sigpipe(code: &mut X64Code) {
    // Raw Linux x86-64 rt_sigaction: kernel sigset size is 8, not libc's 128.
    // Ignoring SIGPIPE makes broken inherited pipes return EPIPE to write-all.
    code.emit(&[0xb8, 13, 0, 0, 0, 0xbf, 13, 0, 0, 0]);
    code.emit_lea_rsi_label("stdio_startup_sigaction");
    code.emit(&[0x31, 0xd2, 0x41, 0xba, 8, 0, 0, 0, 0x0f, 0x05]);
}

pub(super) fn fields(instruction: &AilBytecodeInstruction) -> Result<BTreeSet<String>, String> {
    let operand = match instruction.opcode.as_str() {
        "STDIN_READ_TEXT" => "key",
        "STDOUT_WRITE_TEXT" | "STDERR_WRITE_TEXT" => "source",
        _ => return Ok(BTreeSet::new()),
    };
    if instruction.operands.len() != 1 {
        return Err("native standard-stream operation requires exactly one operand".into());
    }
    let key = required(instruction, operand)?;
    if !matches!(IntExpr::parse(key)?, IntExpr::Field(parsed) if parsed == key) {
        return Err("native standard-stream operand must be a field key".into());
    }
    Ok(BTreeSet::from([key.into()]))
}

pub(super) fn emit_operation(
    emitter: &mut Emitter,
    instruction: &AilBytecodeInstruction,
) -> Result<(), String> {
    emitter.code.emit(&[0x48, 0x83, 0x7d, MODE, 0]);
    emitter
        .code
        .emit_jcc_label(&[0x0f, 0x84], "error_stdio_capability");
    if instruction.opcode == "STDIN_READ_TEXT" {
        emitter.code.emit_call_label("stdio_read_text");
        let slot = emitter.slot(required(instruction, "key")?);
        emitter.store_rax(slot + POINTER);
        emitter.store_rdx(slot + LENGTH);
        emitter.set(slot + NUMERIC, 0);
    } else {
        let slot = emitter.slot(required(instruction, "source")?);
        emitter.load_rdi(slot + POINTER);
        emitter.load_rsi(slot + LENGTH);
        emitter.code.emit_call_label("text_validate");
        emitter.code.emit(&[0x48, 0x89, 0xf2, 0x48, 0x89, 0xfe]); // len->rdx; ptr->rsi
        emitter.code.emit(&[0xbf]);
        let fd = if instruction.opcode == "STDOUT_WRITE_TEXT" {
            1_u32
        } else {
            2_u32
        };
        emitter.code.emit(&fd.to_le_bytes());
        emitter.code.emit_call_label("stdio_write_all");
    }
    Ok(())
}

/// RDI=borrowed fd, RSI=bytes, RDX=remaining. No userspace buffer exists, so
/// completing the syscall loop also completes native flushing. Empty writes
/// perform no syscall. Diagnostic failure returns instead of recursively raising.
fn emit_write_loop(code: &mut X64Code, label: &str, checked: bool) -> Result<(), String> {
    let done = format!("{label}_done");
    code.label(label)?;
    code.emit(&[0x48, 0x85, 0xd2]);
    code.emit_jcc_label(&[0x0f, 0x84], &done);
    code.emit(&[0xb8, 1, 0, 0, 0, 0x0f, 0x05]);
    code.emit(&[0x48, 0x83, 0xf8, 0xfc]); // EINTR leaves progress unchanged
    code.emit_jcc_label(&[0x0f, 0x84], label);
    code.emit(&[0x48, 0x85, 0xc0]);
    code.emit_jcc_label(
        &[0x0f, 0x8e],
        if checked { "error_stdio_io" } else { &done },
    );
    code.emit(&[0x48, 0x01, 0xc6, 0x48, 0x29, 0xc2]); // advance pointer, remaining
    code.emit_jmp_label(label);
    code.label(done)?;
    code.emit(&[0xc3]);
    Ok(())
}

pub(super) fn emit_helpers(code: &mut X64Code) -> Result<(), String> {
    emit_write_loop(code, "stdio_write_all", true)?;
    emit_write_loop(code, "stdio_diagnostic_write", false)?;
    // R12=scratch, R13=bytes read, R15=owned result. No standard descriptor is
    // opened or closed. Recovery owns scratch if validation transfers control.
    code.label("stdio_read_text")?;
    code.emit(&[0x41, 0x54, 0x41, 0x55, 0x41, 0x57]);
    code.emit(&[0x31, 0xff, 0xbe, 1, 0, 0x10, 0, 0xba, 3, 0, 0, 0]);
    code.emit(&[0x41, 0xba, 0x22, 0, 0, 0]);
    code.emit(&[0x49, 0xc7, 0xc0, 0xff, 0xff, 0xff, 0xff, 0x45, 0x31, 0xc9]);
    code.emit_call_label("recovery_temp_map");
    code.emit(&[0x48, 0x85, 0xc0]);
    code.emit_jcc_label(&[0x0f, 0x88], "error_stdio_io");
    code.emit(&[0x49, 0x89, 0xc4, 0x45, 0x31, 0xed]);
    code.label("stdio_read_loop")?;
    code.emit(&[0x31, 0xc0, 0x31, 0xff]); // read(fd=0)
    code.emit(&[0x4b, 0x8d, 0x34, 0x2c]); // lea rsi, [r12+r13]
    code.emit(&[0xba, 1, 0, 0x10, 0, 0x4c, 0x29, 0xea, 0x0f, 0x05]);
    code.emit(&[0x48, 0x83, 0xf8, 0xfc]);
    code.emit_jcc_label(&[0x0f, 0x84], "stdio_read_loop");
    code.emit(&[0x48, 0x85, 0xc0]);
    code.emit_jcc_label(&[0x0f, 0x88], "error_stdio_io");
    code.emit_jcc_label(&[0x0f, 0x84], "stdio_read_done");
    code.emit(&[0x49, 0x01, 0xc5, 0x49, 0x81, 0xfd, 0, 0, 0x10, 0]);
    code.emit_jcc_label(&[0x0f, 0x87], "error_text_limit");
    code.emit_jmp_label("stdio_read_loop");
    code.label("stdio_read_done")?;
    code.emit(&[0x4c, 0x89, 0xe7, 0x4c, 0x89, 0xee]);
    code.emit_call_label("text_validate");
    code.emit(&[0x4c, 0x89, 0xea]);
    code.emit_call_label("text_allocate");
    code.emit(&[0x49, 0x89, 0xc7, 0x48, 0x89, 0xc7]);
    code.emit(&[0x4c, 0x89, 0xe6, 0x4c, 0x89, 0xe9, 0xf3, 0xa4]);
    code.emit(&[0x4c, 0x89, 0xe7]);
    code.emit_call_label("recovery_temp_unmap");
    code.emit(&[0x48, 0x85, 0xc0]);
    code.emit_jcc_label(&[0x0f, 0x88], "error_recovery_cleanup");
    code.emit(&[0x4c, 0x89, 0xf8, 0x4c, 0x89, 0xea]);
    code.emit(&[0x41, 0x5f, 0x41, 0x5d, 0x41, 0x5c, 0xc3]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Output, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn op(opcode: &str, operands: &[(&str, &str)]) -> AilBytecodeInstruction {
        AilBytecodeInstruction {
            opcode: opcode.into(),
            operands: operands
                .iter()
                .map(|(key, value)| ((*key).into(), (*value).into()))
                .collect(),
        }
    }

    fn binary(instructions: Vec<AilBytecodeInstruction>) -> Vec<u8> {
        emit(
            &AilBytecodeAction {
                name: "Streams".into(),
                instructions,
            },
            &BTreeMap::new(),
        )
        .unwrap()
    }

    fn run_binary(binary: &[u8], args: &[&str], input: &[u8], shell: Option<&str>) -> Output {
        run_configured(binary, args, input, shell, |_| {})
    }

    fn run_configured(
        binary: &[u8],
        args: &[&str],
        input: &[u8],
        shell: Option<&str>,
        configure: impl FnOnce(&mut Command),
    ) -> Output {
        let path = std::env::temp_dir().join(format!(
            "ail-native-stdio-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, binary).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut command = if let Some(script) = shell {
            let mut command = Command::new("sh");
            command.args(["-c", script, "stdio-test"]).arg(&path);
            command
        } else {
            Command::new(&path)
        };
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure(&mut command);
        let mut child = command.spawn().unwrap();
        let stdin = child.stdin.take();
        let input = input.to_vec();
        let writer = std::thread::spawn(move || {
            if let Some(mut stdin) = stdin {
                stdin.write_all(&input)
            } else {
                assert!(input.is_empty());
                Ok(())
            }
        });
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let out = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut stdout) = stdout {
                stdout.read_to_end(&mut bytes).unwrap();
            }
            bytes
        });
        let err = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut stderr) = stderr {
                stderr.read_to_end(&mut bytes).unwrap();
            }
            bytes
        });
        let deadline = Instant::now() + Duration::from_secs(15);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() > deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("native standard-stream probe exceeded fifteen seconds");
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        if let Err(error) = writer.join().unwrap() {
            assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        }
        let output = Output {
            status,
            stdout: out.join().unwrap(),
            stderr: err.join().unwrap(),
        };
        std::fs::remove_file(path).unwrap();
        output
    }

    fn run(instructions: Vec<AilBytecodeInstruction>, args: &[&str], input: &[u8]) -> Output {
        run_binary(&binary(instructions), args, input, None)
    }

    #[test]
    fn native_stdio_raw_operations_preserve_bytes_aliases_and_shared_eof_cursor() {
        let input = "first\0é水🙂\r\n".as_bytes();
        let output = run(
            vec![
                op("STDIN_READ_TEXT", &[("key", "data.body")]),
                op(
                    "COPY_FIELD",
                    &[("key", "data.alias"), ("source", "data.body")],
                ),
                op("STDIN_READ_TEXT", &[("key", "data.body")]),
                op("STDOUT_WRITE_TEXT", &[("source", "data.body")]),
                op("STDOUT_WRITE_TEXT", &[("source", "data.alias")]),
                op("STDERR_WRITE_TEXT", &[("source", "data.alias")]),
            ],
            &["--stdio"],
            input,
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, input);
        assert_eq!(output.stderr, input);
    }

    #[test]
    fn native_stdio_quiets_scalar_programs_without_stream_opcodes() {
        let instructions = vec![
            op("ACTION_BEGIN", &[("action", "Main")]),
            op(
                "SET_FIELD",
                &[("key", "data.value"), ("value", "secret value")],
            ),
            op("EVAL_INT", &[("key", "data.count"), ("expression", "i:1")]),
            op("EMIT_TRACE", &[("event", "Unwanted")]),
        ];
        let quiet = run(instructions.clone(), &["--stdio", "--stdio"], b"");
        assert!(quiet.status.success(), "{quiet:?}");
        assert!(quiet.stdout.is_empty(), "{quiet:?}");
        assert!(quiet.stderr.is_empty(), "{quiet:?}");
        let usual = run(instructions, &[], b"");
        assert!(usual.status.success());
        assert_eq!(usual.stdout, b"data.count=1\ndata.value=secret value\n");
        assert!(String::from_utf8_lossy(&usual.stderr).contains("Unwanted"));
    }

    #[test]
    fn native_stdio_requires_authority_before_missing_text() {
        let output = run(
            vec![op("STDOUT_WRITE_TEXT", &[("source", "data.missing")])],
            &[],
            b"",
        );
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-CAPABILITY"));
    }

    #[test]
    fn native_stdio_rejects_malformed_flags_before_ordinary_effects() {
        for malformed in ["--stdio=true", "--stdio-extra", "--stdioé"] {
            let output = run(
                vec![op("ACTION_BEGIN", &[("action", "MustNotStart")])],
                &["--stdio", malformed],
                b"",
            );
            assert!(!output.status.success(), "{malformed}: {output:?}");
            assert!(output.stdout.is_empty());
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("AIL-RUNTIME-CAPABILITY"), "{stderr}");
            assert!(!stderr.contains("MustNotStart"));
        }
    }

    #[test]
    fn native_stdio_malformed_startup_with_broken_stderr_exits_normally_in_both_emitters() {
        use std::os::fd::OwnedFd;
        use std::os::unix::net::UnixStream;
        let action = AilBytecodeAction {
            name: "MustNotStart".into(),
            instructions: vec![
                op("ACTION_BEGIN", &[("action", "MustNotStart")]),
                op("SET_FIELD", &[("key", "data.value"), ("value", "effect")]),
                op("RETURN_SUCCESS", &[]),
            ],
        };
        let mut failures = Vec::new();
        for (emitter, binary) in [
            ("scalar", emit(&action, &BTreeMap::new()).unwrap()),
            (
                "legacy",
                emit_linux_x86_64_elf_for_action(&action, &BTreeMap::new()).unwrap(),
            ),
        ] {
            for args in [
                vec!["--stdio", "--stdio=1"],
                vec!["--stdio=1", "--stdio"],
                vec!["--stdio=1"],
            ] {
                let (writer, reader) = UnixStream::pair().unwrap();
                drop(reader);
                let output = run_configured(&binary, &args, b"", None, |command| {
                    command.stderr(Stdio::from(OwnedFd::from(writer)));
                });
                if output.status.code() != Some(1) || !output.stdout.is_empty() {
                    failures.push(format!("{emitter} {args:?}: {output:?}"));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    fn error(output: &Output, category: &str) {
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(category),
            "{output:?}"
        );
    }

    #[test]
    fn native_stdio_legacy_wrappers_quiet_state_copy_and_trace_without_changing_argv() {
        let action = AilBytecodeAction {
            name: "Legacy".into(),
            instructions: vec![
                op("SET_FIELD", &[("key", "data.value"), ("value", "hello")]),
                op(
                    "COPY_FIELD",
                    &[("key", "data.copy"), ("source", "input.text")],
                ),
                op("EMIT_TRACE", &[("event", "Unwanted")]),
                op("RETURN_SUCCESS", &[]),
            ],
        };
        let binary = emit_linux_x86_64_elf_for_action(&action, &BTreeMap::new()).unwrap();
        let quiet = run_binary(&binary, &["--stdio", "input.text=echo"], b"", None);
        assert!(quiet.status.success(), "{quiet:?}");
        assert!(
            quiet.stdout.is_empty() && quiet.stderr.is_empty(),
            "{quiet:?}"
        );
        let usual = run_binary(&binary, &["input.text=echo"], b"", None);
        assert!(usual.status.success(), "{usual:?}");
        assert_eq!(usual.stdout, b"data.value=hello\ndata.copy=echo\n");
        let malformed = run_binary(&binary, &["--stdio=1"], b"", None);
        error(&malformed, "AIL-RUNTIME-CAPABILITY");
        assert!(malformed.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&malformed.stderr).contains("Unwanted"));
    }

    #[test]
    fn native_stdio_quiet_mode_keeps_return_validation_and_only_named_failure_diagnostics() {
        for instructions in [
            vec![op("OUTPUT_FIELD", &[("key", "missing.value")])],
            vec![
                op("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "4")]),
                op("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                op("OUTPUT_FIELD", &[("key", "missing.value")]),
                op("NATIVE_OUTPUT_SCOPE_END", &[("return_cost", "1")]),
                op("NATIVE_FUEL_SCOPE_END", &[("return_cost", "0")]),
            ],
        ] {
            let output = run(instructions, &["--stdio"], b"");
            error(&output, "AIL-RUNTIME-OUTPUT");
            assert!(output.stdout.is_empty());
        }
        let action = AilBytecodeAction {
            name: "Fails".into(),
            instructions: vec![
                op("ACTION_BEGIN", &[("action", "Unwanted")]),
                op(
                    "REQUIRE_EXISTS",
                    &[
                        ("key", "missing.value"),
                        ("rule", "Required"),
                        ("failure", "Absent"),
                    ],
                ),
            ],
        };
        let failures = BTreeMap::from([(
            "Absent".into(),
            AilBytecodeFailure {
                name: "Absent".into(),
                traces: vec!["Unwanted".into()],
            },
        )]);
        for binary in [
            emit(&action, &failures).unwrap(),
            emit_linux_x86_64_elf_for_action(&action, &failures).unwrap(),
        ] {
            let output = run_binary(&binary, &["--stdio"], b"", None);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert_eq!(output.stderr, b"failure Absent\n");
        }
    }

    #[test]
    fn native_stdio_read_boundaries_large_pipes_and_size_before_utf8() {
        let binary = binary(vec![
            op("STDIN_READ_TEXT", &[("key", "data.body")]),
            op("STDOUT_WRITE_TEXT", &[("source", "data.body")]),
            op("STDERR_WRITE_TEXT", &[("source", "data.body")]),
        ]);
        for size in [0, 1, 65537, 1_048_576] {
            let mut input = vec![b'x'; size];
            if size > 4 {
                input[8189.min(size - 4)..8193.min(size)].copy_from_slice("🙂".as_bytes());
            }
            let output = run_binary(&binary, &["--stdio"], &input, None);
            assert!(output.status.success(), "size {size}: {output:?}");
            assert_eq!(output.stdout, input);
            assert_eq!(output.stderr, input);
        }
        let mut input = vec![b'x'; 1_048_577];
        input[0] = 0xff;
        let output = run_binary(&binary, &["--stdio"], &input, None);
        error(&output, "AIL-RUNTIME-TEXT-LIMIT");
        assert!(output.stdout.is_empty());
    }

    #[test]
    fn native_stdio_recovery_keeps_consumed_cursor_prior_effects_and_destination_aliases() {
        let output = run(
            vec![
                op("SET_FIELD", &[("key", "data.body"), ("value", "old")]),
                op(
                    "COPY_FIELD",
                    &[("key", "data.alias"), ("source", "data.body")],
                ),
                op("STDOUT_WRITE_TEXT", &[("source", "data.body")]),
                op("TRY_BEGIN", &[("errors", "TEXT"), ("key", "data.error")]),
                op("STDIN_READ_TEXT", &[("key", "data.body")]),
                op("RECOVER", &[]),
                op("STDERR_WRITE_TEXT", &[("source", "data.error")]),
                op("TRY_END", &[]),
                op("STDOUT_WRITE_TEXT", &[("source", "data.body")]),
                op("STDOUT_WRITE_TEXT", &[("source", "data.alias")]),
                op("STDIN_READ_TEXT", &[("key", "data.body")]),
                op("STDOUT_WRITE_TEXT", &[("source", "data.body")]),
            ],
            &["--stdio"],
            &[b'a', 0xff, b'z'],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"oldoldold");
        assert_eq!(output.stderr, b"AIL-RUNTIME-TEXT");
    }

    #[test]
    fn native_stdio_validates_full_source_before_writes_and_does_not_infer_grants() {
        use std::os::unix::ffi::OsStringExt;
        let binary = binary(vec![op("STDOUT_WRITE_TEXT", &[("source", "data.body")])]);
        let output = run_configured(&binary, &["--stdio"], b"", None, |command| {
            command.arg(std::ffi::OsString::from_vec(
                b"data.body=valid-prefix\xff".to_vec(),
            ));
        });
        error(&output, "AIL-RUNTIME-TEXT");
        assert!(output.stdout.is_empty());
        let output = run_binary(&binary, &["data.body=--stdio", "x=--stdio"], b"", None);
        error(&output, "AIL-RUNTIME-CAPABILITY");
        assert!(output.stdout.is_empty());
    }

    #[test]
    fn native_stdio_closed_descriptors_and_empty_writes() {
        for (opcode, field, shell) in [
            ("STDIN_READ_TEXT", "key", "exec 0<&-; exec \"$@\""),
            ("STDOUT_WRITE_TEXT", "source", "exec 1>&-; exec \"$@\""),
            ("STDERR_WRITE_TEXT", "source", "exec 2>&-; exec \"$@\""),
        ] {
            let binary = binary(vec![op(opcode, &[(field, "data.body")])]);
            let output = run_binary(&binary, &["--stdio", "data.body=text"], b"", Some(shell));
            assert_eq!(output.status.code(), Some(1), "{opcode}: {output:?}");
            if opcode != "STDERR_WRITE_TEXT" {
                error(&output, "AIL-RUNTIME-IO");
            }
            if opcode != "STDIN_READ_TEXT" {
                let empty = run_binary(&binary, &["--stdio", "data.body="], b"", Some(shell));
                assert!(empty.status.success(), "{opcode}: {empty:?}");
            }
        }
    }

    #[test]
    fn native_stdio_broken_output_diagnoses_io_instead_of_sigpipe_termination() {
        use std::os::fd::OwnedFd;
        use std::os::unix::net::UnixStream;
        for opcode in ["STDOUT_WRITE_TEXT", "STDERR_WRITE_TEXT"] {
            let (writer, reader) = UnixStream::pair().unwrap();
            drop(reader);
            let binary = binary(vec![op(opcode, &[("source", "data.body")])]);
            let output = run_configured(
                &binary,
                &["--stdio", "data.body=text"],
                b"",
                None,
                |command| {
                    let stream = Stdio::from(OwnedFd::from(writer));
                    if opcode == "STDOUT_WRITE_TEXT" {
                        command.stdout(stream);
                    } else {
                        command.stderr(stream);
                    }
                },
            );
            assert_eq!(output.status.code(), Some(1), "{opcode}: {output:?}");
            if opcode == "STDOUT_WRITE_TEXT" {
                error(&output, "AIL-RUNTIME-IO");
            }
        }
    }

    #[test]
    fn native_stdio_scoped_reads_share_cursor_without_changing_parent_fuel_or_outputs() {
        let output = run(
            vec![
                op("NATIVE_FUEL_ROOT", &[("limit", "20")]),
                op("OUTPUT_FIELD", &[("key", "data.alias")]),
                op("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "3")]),
                op("NATIVE_OUTPUT_SCOPE_BEGIN", &[]),
                op("STDIN_READ_TEXT", &[("key", "data.body")]),
                op("OUTPUT_FIELD", &[("key", "data.body")]),
                op("NATIVE_OUTPUT_SCOPE_END", &[("return_cost", "1")]),
                op("NATIVE_FUEL_SCOPE_END", &[("return_cost", "0")]),
                op(
                    "COPY_FIELD",
                    &[("key", "data.alias"), ("source", "data.body")],
                ),
                op("NATIVE_FUEL_SCOPE_BEGIN", &[("limit", "2")]),
                op("STDIN_READ_TEXT", &[("key", "data.body")]),
                op("NATIVE_FUEL_SCOPE_END", &[("return_cost", "1")]),
                op("STDOUT_WRITE_TEXT", &[("source", "data.alias")]),
                op("STDOUT_WRITE_TEXT", &[("source", "data.body")]),
                op("RETURN_SUCCESS", &[]),
            ],
            &["--stdio"],
            b"one shared cursor",
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"one shared cursor");
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn native_stdio_nonblocking_empty_input_and_full_output_fail_io() {
        use std::os::fd::OwnedFd;
        use std::os::unix::net::UnixStream;
        let (reader, _writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        let output = run_configured(
            &binary(vec![op("STDIN_READ_TEXT", &[("key", "data.body")])]),
            &["--stdio"],
            b"",
            None,
            |command| {
                command.stdin(Stdio::from(OwnedFd::from(reader)));
            },
        );
        error(&output, "AIL-RUNTIME-IO");
        let (writer, _reader) = UnixStream::pair().unwrap();
        writer.set_nonblocking(true).unwrap();
        // File uses write(2), matching the native helper. UnixStream::write
        // uses sendto(2), which some test sandboxes independently restrict.
        let mut writer = std::fs::File::from(OwnedFd::from(writer));
        loop {
            match writer.write(&[b'x'; 8192]) {
                Ok(count) => assert!(count > 0),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) => panic!("unexpected socket fill failure: {error}"),
            }
        }
        let output = run_configured(
            &binary(vec![op("STDOUT_WRITE_TEXT", &[("source", "data.body")])]),
            &["--stdio", "data.body=remaining"],
            b"",
            None,
            |command| {
                command.stdout(Stdio::from(OwnedFd::from(writer)));
            },
        );
        error(&output, "AIL-RUNTIME-IO");
    }

    #[test]
    fn native_stdio_oversize_input_consumes_exactly_limit_plus_one_without_closing_descriptor() {
        use std::io::Seek;
        let path = std::env::temp_dir().join(format!(
            "ail-native-stdio-input-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, vec![b'a'; 1_048_590]).unwrap();
        let mut input = std::fs::File::open(&path).unwrap();
        let output = run_configured(
            &binary(vec![op("STDIN_READ_TEXT", &[("key", "data.body")])]),
            &["--stdio"],
            b"",
            None,
            |command| {
                command.stdin(Stdio::from(input.try_clone().unwrap()));
            },
        );
        error(&output, "AIL-RUNTIME-TEXT-LIMIT");
        assert_eq!(input.stream_position().unwrap(), 1_048_577);
        let mut remaining = Vec::new();
        input.read_to_end(&mut remaining).unwrap();
        assert_eq!(remaining, vec![b'a'; 13]);
        std::fs::remove_file(path).unwrap();
    }

    fn fill_arena(instructions: &mut Vec<AilBytecodeInstruction>, copies: usize) {
        let block = "x".repeat(1_048_576);
        instructions.extend([
            op("SET_FIELD", &[("key", "data.block"), ("value", &block)]),
            op("SET_FIELD", &[("key", "data.empty"), ("value", "")]),
            op("EVAL_INT", &[("key", "data.count"), ("expression", "i:0")]),
            op("LABEL", &[("name", "fill")]),
            op(
                "TEXT_JOIN",
                &[
                    ("key", "data.owned"),
                    ("left", "data.block"),
                    ("right", "data.empty"),
                ],
            ),
            op(
                "EVAL_INT",
                &[("key", "data.count"), ("expression", "f:data.count i:1 +")],
            ),
            op(
                "BRANCH_INT",
                &[
                    ("left", "f:data.count"),
                    ("right", &format!("i:{copies}")),
                    ("comparison", "<"),
                    ("label", "fill"),
                ],
            ),
        ]);
    }

    #[test]
    fn native_stdio_exact_arena_charges_and_uncharged_writes_after_full_budget() {
        let mut instructions = vec![op("STDIN_READ_TEXT", &[("key", "data.body")])];
        fill_arena(&mut instructions, 63);
        instructions.push(op("STDOUT_WRITE_TEXT", &[("source", "data.body")]));
        let input = vec![b'b'; 1_048_576];
        let output = run(instructions.clone(), &["--stdio"], &input);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, input);
        assert!(output.stderr.is_empty());
        instructions.push(op("STDIN_READ_TEXT", &[("key", "data.body")]));
        let output = run(instructions, &["--stdio"], &input);
        error(&output, "AIL-RUNTIME-TEXT-LIMIT");
        assert_eq!(output.stdout, input); // prior output survives fatal empty-read charge

        let mut instructions = vec![
            op("TRY_BEGIN", &[("errors", "TEXT"), ("key", "data.error")]),
            op("STDIN_READ_TEXT", &[("key", "data.body")]),
            op("RECOVER", &[]),
            op("TRY_END", &[]),
        ];
        fill_arena(&mut instructions, 64);
        instructions.push(op("STDOUT_WRITE_TEXT", &[("source", "data.error")]));
        let output = run(instructions.clone(), &["--stdio"], &[0xff]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"AIL-RUNTIME-TEXT"); // failed read charged zero
        let output = run(instructions, &["--stdio"], b"");
        error(&output, "AIL-RUNTIME-TEXT-LIMIT"); // successful empty read charged one
        assert!(output.stdout.is_empty());
    }

    #[test]
    fn native_stdio_repeated_reads_release_scratch_under_address_space_and_fd_limits() {
        let instructions = vec![
            op("NATIVE_FUEL_ROOT", &[("limit", "20000")]),
            op("TRY_BEGIN", &[("errors", "TEXT"), ("key", "data.error")]),
            op("STDIN_READ_TEXT", &[("key", "data.body")]),
            op("RECOVER", &[]),
            op("TRY_END", &[]),
            op("EVAL_INT", &[("key", "data.count"), ("expression", "i:0")]),
            op("LABEL", &[("name", "read")]),
            op("STDIN_READ_TEXT", &[("key", "data.body")]),
            op(
                "EVAL_INT",
                &[("key", "data.count"), ("expression", "f:data.count i:1 +")],
            ),
            op(
                "BRANCH_INT",
                &[
                    ("left", "f:data.count"),
                    ("right", "i:2000"),
                    ("comparison", "<"),
                    ("label", "read"),
                ],
            ),
            op("STDOUT_WRITE_TEXT", &[("source", "data.count")]),
            op("STDERR_WRITE_TEXT", &[("source", "data.error")]),
        ];
        let output = run_binary(
            &binary(instructions),
            &["--stdio"],
            &[0xff],
            Some("ulimit -v 98304; ulimit -n 12; exec \"$@\""),
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"2000");
        assert_eq!(output.stderr, b"AIL-RUNTIME-TEXT");
    }

    // Install a real, non-SA_RESTART SIGALRM handler before the emitted entry
    // point. All normal program code/data references are RIP-relative. The
    // handler emits a marker so the test also proves a signal actually arrived.
    fn with_interrupting_alarm(binary: &[u8]) -> Vec<u8> {
        assert_eq!(u16::from_le_bytes(binary[56..58].try_into().unwrap()), 1);
        let mut code = X64Code::default();
        code.emit(&[0x48, 0x83, 0xec, 32]);
        code.emit_lea_rsi_label("test_alarm_handler");
        code.emit(&[0x48, 0x89, 0x34, 0x24]);
        code.emit(&[0x48, 0xc7, 0x44, 0x24, 8, 0, 0, 0, 4]); // SA_RESTORER only
        code.emit_lea_rsi_label("test_alarm_restorer");
        code.emit(&[0x48, 0x89, 0x74, 0x24, 16]);
        code.emit(&[0x48, 0xc7, 0x44, 0x24, 24, 0, 0, 0, 0]);
        code.emit(&[0xb8, 13, 0, 0, 0, 0xbf, 14, 0, 0, 0]);
        code.emit(&[
            0x48, 0x89, 0xe6, 0x31, 0xd2, 0x41, 0xba, 8, 0, 0, 0, 0x0f, 0x05,
        ]);
        code.emit(&[0x48, 0x83, 0xc4, 32]);
        code.emit(&[0xb8, 37, 0, 0, 0, 0xbf, 1, 0, 0, 0, 0x0f, 0x05]); // alarm(1)
        code.emit_jmp_label("test_alarm_program");
        code.label("test_alarm_handler").unwrap();
        code.emit(&[0x50, 0x57, 0x56, 0x52]);
        code.emit_write_label(2, "test_alarm_marker", 1);
        code.emit(&[0x5a, 0x5e, 0x5f, 0x58, 0xc3]);
        code.label("test_alarm_restorer").unwrap();
        code.emit(&[0xb8, 15, 0, 0, 0, 0x0f, 0x05]); // rt_sigreturn
        code.label("test_alarm_marker").unwrap();
        code.emit(b"I");
        code.label("test_alarm_program").unwrap();
        code.emit(&binary[120..]); // one standard 64-byte ELF + 56-byte program header
        wrap_linux_x86_64_elf(&code.finish().unwrap())
    }

    #[test]
    fn native_stdio_real_interrupted_read_and_short_write_preserve_progress() {
        use std::os::fd::OwnedFd;
        use std::os::unix::net::UnixStream;
        let (reader, writer) = UnixStream::pair().unwrap();
        let producer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1500));
            let mut writer = std::fs::File::from(OwnedFd::from(writer));
            writer.write_all(b"after signal")
        });
        let program = with_interrupting_alarm(&binary(vec![
            op("STDIN_READ_TEXT", &[("key", "data.body")]),
            op("STDOUT_WRITE_TEXT", &[("source", "data.body")]),
        ]));
        let output = run_configured(&program, &["--stdio"], b"", None, |command| {
            command.stdin(Stdio::from(OwnedFd::from(reader)));
        });
        producer.join().unwrap().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"after signal");
        assert_eq!(output.stderr, b"I");

        let (writer, mut reader) = UnixStream::pair().unwrap();
        let consumer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1500));
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).unwrap();
            bytes
        });
        let text = "x".repeat(1_048_576);
        let program = with_interrupting_alarm(&binary(vec![
            op("SET_FIELD", &[("key", "data.body"), ("value", &text)]),
            op("STDOUT_WRITE_TEXT", &[("source", "data.body")]),
        ]));
        let output = run_configured(&program, &["--stdio"], b"", None, |command| {
            command.stdout(Stdio::from(OwnedFd::from(writer)));
        });
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stderr, b"I");
        assert_eq!(consumer.join().unwrap(), text.as_bytes());
    }
}
