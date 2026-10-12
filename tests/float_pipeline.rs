use ail::ail::*;
use std::collections::BTreeMap;

fn core(source: &str) -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}
const SOURCE: &str = r#"The application Floating manages measurements.

A Data has:
- a: Float
- b: Float
- n: Int
- denominator: Float
- raw: Text
- document: Json
- path: Text

An Output has:
- value: Float
- whole: Int
- raw: Text

Function: midpoint.
The function needs:
- a: Float
- b: Float
The function produces:
- result: Float
The function stores:
- result: Float
The function executes:
- the function computes floating local.result as (local.a + local.b) / 2
- if floating local.result < 0:
- the function computes floating local.result as -local.result
- end if
- the function returns local.result

Action: Run.
When Run happens:
- the system assigns 1.5 to data.a
- the system assigns -8.5 to data.b
- the system calls midpoint with a = data.a; b = data.b into output.value
- the system assigns 2 to data.n
- the system converts integer data.n to floating data.denominator
- the system computes floating output.value as output.value / data.denominator
- the system converts floating output.value to integer output.whole
- the system assigns {} to data.document
- the system assigns "/mean" to data.path
- the system sets JSON value at path data.path in data.document to output.value into data.document
- the system formats data.document as JSON text into output.raw
- the system records a trace event named Measured
- the system returns output.value
- the system returns output.whole
"#;

#[test]
fn floating_functions_conversions_and_json_roundtrip_through_core_and_ste_vm() {
    let original = core(SOURCE);
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&candidate).unwrap()).unwrap();
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(result.final_state["output.value"], "1.75e0");
        assert_eq!(result.final_state["output.whole"], "1");
        assert_eq!(result.final_state["output.raw"], r#"{"mean":1.75e0}"#);
        assert!(!result.final_state.keys().any(|k| k.starts_with("local.")));
    }
}

#[test]
fn floating_source_rejects_wrong_typed_fields_and_hidden_globals() {
    for source in [
        SOURCE.replace("- a: Float", "- a: Text"),
        SOURCE.replace("as (local.a + local.b) / 2", "as (data.a + local.b) / 2"),
        SOURCE.replace(
            "integer data.n to floating data.denominator",
            "integer data.a to floating data.denominator",
        ),
    ] {
        let source = core(&source);
        assert!(!check_ail_core(&source).is_empty());
        assert!(compile_ail_core_ste(&source).is_err());
    }
}

#[test]
fn imported_floating_actions_qualify_conversions_conditions_and_function_calls() {
    let mut child = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    child.metadata.name = "float-child".into();
    child.spec_text = SOURCE.replace(
        "- the system converts floating output.value to integer output.whole",
        "- if floating output.value > 0:\n- the system converts floating output.value to integer output.whole\n- end if",
    );
    let mut parent = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    parent.spec_text = "The application Parent manages measurements.\nAction: Run.\nWhen Run happens:\n- the system calls Child.Run\n- the system calls Child.Run\n- the system returns child.output.value\n- the system returns child.output.whole\n- the system records a trace event named Done\n".into();
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
    let saved = parse_ail_core_text(&render_ail_core(&original)).unwrap();
    let program = ail::ste::parse_ail_ste(&compile_ail_core_ste(&saved).unwrap()).unwrap();
    let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(result.final_state["child.output.value"], "1.75e0");
    assert_eq!(result.final_state["child.output.whole"], "1");
    assert!(!result.final_state.keys().any(|key| key.contains("local.")));
    assert_eq!(
        program.actions["Child.midpoint"].instructions[0].operands["parameters"],
        "a:Float;b:Float"
    );
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let binary = std::env::temp_dir().join(format!("ail-float-import-{}", std::process::id()));
        let bytes = compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").unwrap();
        std::fs::write(&binary, bytes).unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
        let output = std::process::Command::new(&binary).output().unwrap();
        std::fs::remove_file(binary).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            b"child.output.value=1.75e0\nchild.output.whole=1\n"
        );
    }
}
