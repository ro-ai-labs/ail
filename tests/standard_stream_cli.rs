use std::process::Command;

#[test]
fn oversized_cli_input_does_not_prefetch_past_the_sentinel() {
    use std::io::{Seek, Write};
    let path = std::env::temp_dir().join(format!("ail-stdio-cursor-{}", std::process::id()));
    let mut input = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    std::fs::remove_file(path).unwrap();
    input.write_all(&vec![b'x'; 1_048_576 + 16_384]).unwrap();
    input.rewind().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ail"))
        .args([
            "ail-run",
            "examples/json-lines-filter",
            "--action",
            "Filter",
            "--stdio",
        ])
        .stdin(input.try_clone().unwrap())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-TEXT-LIMIT"));
    assert_eq!(input.stream_position().unwrap(), 1_048_577);
}

#[test]
fn failed_fatal_diagnostic_is_best_effort_without_a_secondary_panic() {
    let failed_writer = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ail"))
        .args([
            "ail-run",
            "bench/R001-add/program.ail",
            "--action",
            "AddNumbers",
            "--stdio",
        ])
        .stderr(failed_writer)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{:?}", output);
    assert!(output.stdout.is_empty());
}

#[test]
fn quiet_run_suppresses_automatic_reporting_without_new_opcodes() {
    let normal = Command::new(env!("CARGO_BIN_EXE_ail"))
        .args([
            "ail-run",
            "bench/R001-add/program.ail",
            "--action",
            "AddNumbers",
            "sum.a=2",
            "sum.b=3",
        ])
        .output()
        .unwrap();
    assert!(normal.status.success(), "{:?}", normal);
    assert!(!normal.stdout.is_empty());
    let quiet = Command::new(env!("CARGO_BIN_EXE_ail"))
        .args([
            "ail-run",
            "bench/R001-add/program.ail",
            "--action",
            "AddNumbers",
            "sum.a=2",
            "sum.b=3",
            "--stdio",
            "--stdio",
        ])
        .output()
        .unwrap();
    assert!(quiet.status.success(), "{:?}", quiet);
    assert!(
        quiet.stdout.is_empty() && quiet.stderr.is_empty(),
        "{:?}",
        quiet
    );
}

#[test]
fn malformed_or_unsupported_stdio_flag_is_a_capability_error() {
    for (command, flag) in [
        ("ail-run", "--stdio=true"),
        ("ail-run", "--stdio-extra"),
        ("ail-check", "--stdio"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_ail"))
            .args([command, "bench/R001-add/program.ail", flag])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("AIL-RUNTIME-CAPABILITY"),
            "{:?}",
            output
        );
    }
}
