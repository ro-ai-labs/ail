use std::process::Command;

#[test]
fn malformed_or_unsupported_process_grants_fail_before_execution() {
    for (command, flag) in [
        ("ail-run", "--run-process"),
        ("ail-run", "--run-process-extra=tool=/bin/true"),
        ("ail-run", "--run-process=tool=relative/path"),
        ("ail-run", "--run-process=bad-name=/bin/true"),
        ("ail-check", "--run-process=tool=/bin/true"),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_ail"))
            .args([command, "bench/R001-add/program.ail", flag])
            .output()
            .unwrap();
        assert!(!result.status.success(), "{result:?}");
        assert!(result.stdout.is_empty(), "{result:?}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("AIL-RUNTIME-CAPABILITY"),
            "{flag}: {result:?}"
        );
    }
}

#[test]
fn unused_process_grant_does_not_open_or_execute_its_path() {
    let result = Command::new(env!("CARGO_BIN_EXE_ail"))
        .args([
            "ail-run",
            "bench/R001-add/program.ail",
            "--action",
            "AddNumbers",
            "sum.a=2",
            "sum.b=3",
            "--stdio",
            "--run-process=unused=/definitely/missing/ail-process-tool",
        ])
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    assert!(
        result.stdout.is_empty() && result.stderr.is_empty(),
        "{result:?}"
    );
}
