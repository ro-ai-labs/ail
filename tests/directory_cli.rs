use std::process::Command;

#[test]
fn malformed_or_unsupported_directory_grants_fail_before_execution() {
    for (command, flags) in [
        ("ail-run", vec!["--read-directory"]),
        ("ail-run", vec!["--write-directory-extra=out=/tmp"]),
        ("ail-run", vec!["--read-directory=work=relative/path"]),
        ("ail-run", vec!["--write-directory=bad-name=/tmp"]),
        (
            "ail-run",
            vec![
                "--read-directory=work=relative",
                "--read-directory=work=/tmp",
            ],
        ),
        ("ail-check", vec!["--read-directory=work=/tmp"]),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_ail"))
            .args([command, "bench/R001-add/program.ail"])
            .args(&flags)
            .output()
            .unwrap();
        assert!(!result.status.success(), "{result:?}");
        assert!(result.stdout.is_empty(), "{result:?}");
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("AIL-RUNTIME-CAPABILITY"),
            "{flags:?}: {result:?}"
        );
    }
}

#[test]
fn unused_directory_grants_are_not_opened_and_quiet_mode_stays_quiet() {
    let result = Command::new(env!("CARGO_BIN_EXE_ail"))
        .args([
            "ail-run",
            "bench/R001-add/program.ail",
            "--action",
            "AddNumbers",
            "sum.a=2",
            "sum.b=3",
            "--stdio",
            "--read-directory=unused=/definitely/missing/ail-directory-input",
            "--write-directory=unused=/definitely/missing/ail-directory-output",
        ])
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    assert!(
        result.stdout.is_empty() && result.stderr.is_empty(),
        "{result:?}"
    );
}
