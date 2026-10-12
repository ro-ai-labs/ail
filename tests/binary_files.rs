//! Binary file modes share existing grants without weakening Text file semantics.
use ail::ail::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const SOURCE: &str = "The application Binary manages files.\n\nA Data has:\n- raw: Bytes\n- text: Text\n- count: Int\n\nAction: Run.\nWhen Run happens:\n- the system reads bytes from file resource input into data.raw\n- the system writes bytes data.raw to file resource output\n- the system counts bytes in data.raw into data.count\n- the system returns data.count\n- the system records a trace event named Done\n";

fn core(source: &str) -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "ail-binary-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn binary_files_roundtrip_every_octet_under_existing_caller_bindings() {
    let original = core(SOURCE);
    assert!(
        check_ail_core(&original).is_empty(),
        "{:?}",
        check_ail_core(&original)
    );
    let scratch = Scratch::new();
    let input = scratch.0.join("input = arbitrary.bin");
    let output = scratch.0.join("output.bin");
    let bytes: Vec<u8> = (0..=255).collect();
    fs::write(&input, &bytes).unwrap();
    let grants = AilFileResources {
        reads: BTreeMap::from([("input".into(), input)]),
        writes: BTreeMap::from([("output".into(), output.clone())]),
    };
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let direct = compile_ail_core_bytecode(&candidate).unwrap();
        let ste = ail::ste::parse_ail_ste(&ail::ste::render_ail_ste(&direct).unwrap()).unwrap();
        for program in [direct, ste] {
            fs::write(&output, b"old longer sentinel").unwrap();
            let result =
                run_ail_bytecode_action_with_files(&program, "Run", BTreeMap::new(), &grants)
                    .unwrap();
            assert_eq!(result.final_state["data.count"], "256");
            assert_eq!(fs::read(&output).unwrap(), bytes);
            assert_eq!(
                &compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").unwrap()[..4],
                b"\x7fELF"
            );
        }
    }
}

fn writer() -> AilBytecodeProgram {
    compile_ail_core_bytecode(&core(&SOURCE.replace(
        "- the system reads bytes from file resource input into data.raw\n",
        "",
    )))
    .unwrap()
}

#[test]
fn bytes_write_checks_authority_then_transport_before_truncation() {
    let program = writer();
    let scratch = Scratch::new();
    let output = scratch.0.join("sentinel.bin");
    fs::write(&output, b"keep\0\xff").unwrap();
    let grants = AilFileResources {
        reads: BTreeMap::new(),
        writes: BTreeMap::from([("output".into(), output.clone())]),
    };
    let invalid = BTreeMap::from([("data.raw".into(), "hex:FF".into())]);
    let no_grant = run_ail_bytecode_action_with_files(
        &program,
        "Run",
        invalid.clone(),
        &AilFileResources::default(),
    )
    .unwrap_err();
    assert!(no_grant.contains("AIL-RUNTIME-CAPABILITY:"), "{no_grant}");
    for state in [
        BTreeMap::new(),
        invalid,
        BTreeMap::from([("data.raw".into(), "hex:0".into())]),
    ] {
        let error =
            run_ail_bytecode_action_with_files(&program, "Run", state, &grants).unwrap_err();
        assert!(error.contains("AIL-RUNTIME-BYTES:"), "{error}");
        assert_eq!(fs::read(&output).unwrap(), b"keep\0\xff");
    }
    let empty = BTreeMap::from([("data.raw".into(), "hex:".into())]);
    run_ail_bytecode_action_with_files(&program, "Run", empty, &grants).unwrap();
    assert_eq!(fs::read(&output).unwrap(), b"");
}

#[test]
fn invalid_unused_host_bindings_cannot_be_hidden_by_a_valid_bytes_write() {
    let program = writer();
    let scratch = Scratch::new();
    let output = scratch.0.join("sentinel.bin");
    for invalid_reads in [false, true] {
        for (name, path) in [
            ("bad/name", "valid"),
            ("unused", ""),
            ("unused", "bad\0path"),
        ] {
            fs::write(&output, b"preserved").unwrap();
            let mut grants = AilFileResources {
                reads: BTreeMap::new(),
                writes: BTreeMap::from([("output".into(), output.clone())]),
            };
            let bindings = if invalid_reads {
                &mut grants.reads
            } else {
                &mut grants.writes
            };
            bindings.insert(name.into(), PathBuf::from(path));
            let error = run_ail_bytecode_action_with_files(
                &program,
                "Run",
                BTreeMap::from([("data.raw".into(), "hex:00ff".into())]),
                &grants,
            )
            .unwrap_err();
            assert!(error.contains("AIL-RUNTIME-CAPABILITY:"), "{error}");
            assert_eq!(fs::read(&output).unwrap(), b"preserved");
        }
    }
}

#[test]
fn bytes_read_accepts_exact_raw_limit_and_old_text_read_stays_strict() {
    let scratch = Scratch::new();
    let input = scratch.0.join("input.bin");
    let grants = AilFileResources {
        reads: BTreeMap::from([("input".into(), input.clone())]),
        writes: BTreeMap::new(),
    };
    let read_only = SOURCE.replace(
        "- the system writes bytes data.raw to file resource output\n",
        "",
    );
    let bytes_program = compile_ail_core_bytecode(&core(&read_only)).unwrap();
    fs::write(&input, vec![0xff; 1_048_576]).unwrap();
    let result =
        run_ail_bytecode_action_with_files(&bytes_program, "Run", BTreeMap::new(), &grants)
            .unwrap();
    assert_eq!(result.final_state["data.count"], "1048576");
    assert_eq!(result.final_state["data.raw"].len(), 2_097_156);
    fs::write(&input, vec![0; 1_048_577]).unwrap();
    let error = run_ail_bytecode_action_with_files(&bytes_program, "Run", BTreeMap::new(), &grants)
        .unwrap_err();
    assert!(error.contains("AIL-RUNTIME-BYTES-LIMIT:"), "{error}");
    let text_source = read_only
        .replace(
            "reads bytes from file resource input into data.raw",
            "reads text from file resource input into data.text",
        )
        .replace("counts bytes in data.raw", "counts characters in data.text");
    let text_program = compile_ail_core_bytecode(&core(&text_source)).unwrap();
    fs::write(&input, [0xff]).unwrap();
    let error = run_ail_bytecode_action_with_files(&text_program, "Run", BTreeMap::new(), &grants)
        .unwrap_err();
    assert!(error.contains("AIL-RUNTIME-TEXT:"), "{error}");
}

#[test]
fn binary_file_core_rejects_hidden_authority_and_duplicate_roles() {
    let original = core(SOURCE);
    for role in ["file_step", "file_source", "file_destination"] {
        let mut changed = original.clone();
        let extra = changed
            .graph
            .edges
            .iter()
            .find(|e| e.kind == role)
            .unwrap()
            .clone();
        changed.graph.edges.push(extra);
        assert!(
            compile_ail_core_bytecode(&changed).is_err(),
            "duplicate {role}"
        );
        if let Ok(saved) = parse_ail_core_text(&render_ail_core(&changed)) {
            assert!(
                compile_ail_core_bytecode(&saved).is_err(),
                "saved duplicate {role}"
            );
        }
    }
    for attribute in ["path", "grant", "root", "source", "key"] {
        let mut changed = original.clone();
        changed
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.kind == "FileOperation")
            .unwrap()
            .attributes
            .insert(attribute.into(), "/hidden/path".into());
        assert!(
            compile_ail_core_bytecode(&changed).is_err(),
            "hidden {attribute}"
        );
    }
    let wrong = SOURCE.replace("- raw: Bytes", "- raw: Text");
    assert!(compile_ail_core_bytecode(&core(&wrong)).is_err());
}
