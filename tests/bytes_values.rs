//! Public Bytes acceptance; independent arbitrary-byte oracle is in scripts/ail_bytes_differential.py.
use ail::ail::*;
use std::collections::BTreeMap;

const DECLARATIONS: &str = "The application Octets manages bytes.\n\nA Data has:\n- raw: Bytes\n- other: Bytes\n- copy: Bytes\n- text: Text\n- hex: Text\n- index: Int\n- start: Int\n- end: Int\n- value: Int\n- count: Int\n- order: Int\n- failure: Text\n\n";

fn source(body: &str) -> String {
    format!(
        "{DECLARATIONS}Action: Run.\nWhen Run happens:\n{body}\n- the system records a trace event named Done\n"
    )
}

fn core(source: &str) -> AilCore {
    let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    elaborate_ail_core(
        &package,
        &parse_ail_package_spec_text(&package, source).unwrap(),
    )
}

#[test]
fn bytes_are_distinct_values_across_core_rendering_ste_vm_and_native_lowering() {
    let original = core(&source(
        "- the system assigns \"hex:00ff80\" to data.raw\n- the system counts bytes in data.raw into data.count\n- the system gets byte -1 from bytes data.raw into data.value\n- the system slices bytes data.raw from 0 to 2 into data.copy\n- the system joins bytes data.copy and data.raw into data.other\n- the system sets byte 0 in bytes data.other to 127 into data.other\n- the system encodes text data.text as UTF-8 bytes into data.copy\n- the system decodes bytes data.copy as UTF-8 text into data.text\n- the system decodes hexadecimal text data.hex into bytes data.copy\n- the system encodes bytes data.copy as hexadecimal text into data.hex\n- the system compares bytes data.raw with data.other into data.order",
    ));
    assert_eq!(
        original
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind == "BytesOperation")
            .count(),
        10
    );
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        assert!(
            check_ail_core(&candidate).is_empty(),
            "{:?}",
            check_ail_core(&candidate)
        );
        let direct = compile_ail_core_bytecode(&candidate).unwrap();
        let ste = ail::ste::parse_ail_ste(&ail::ste::render_ail_ste(&direct).unwrap()).unwrap();
        for program in [direct, ste] {
            let state = BTreeMap::from([
                ("data.text".into(), "λ\0🙂".into()),
                ("data.hex".into(), "Aa00Ff".into()),
            ]);
            let result = run_ail_bytecode_action(&program, "Run", state).unwrap();
            assert_eq!(result.final_state["data.raw"], "hex:00ff80");
            assert_eq!(result.final_state["data.other"], "hex:7fff00ff80");
            assert_eq!(result.final_state["data.copy"], "hex:aa00ff");
            assert_eq!(result.final_state["data.text"], "λ\0🙂");
            assert_eq!(result.final_state["data.hex"], "aa00ff");
            assert_eq!(result.final_state["data.count"], "3");
            assert_eq!(result.final_state["data.value"], "128");
            assert_eq!(result.final_state["data.order"], "-1");
            assert_eq!(
                &compile_ail_bytecode_native_elf(&program, "Run", "linux-x86_64-elf").unwrap()[..4],
                b"\x7fELF"
            );
        }
    }
}

fn rejected(candidate: &AilCore, reason: &str) {
    assert!(
        compile_ail_core_bytecode(candidate).is_err(),
        "accepted {reason}"
    );
    if let Ok(saved) = parse_ail_core_text(&render_ail_core(candidate)) {
        assert!(
            compile_ail_core_bytecode(&saved).is_err(),
            "saved Core accepted {reason}"
        );
    }
}

#[test]
fn bytes_core_rejects_duplicate_hidden_reversed_roles_and_forged_owners() {
    let original = core(&source(
        "- the system slices bytes data.raw from data.start to data.end into data.copy\n- the system sets byte data.index in bytes data.raw to data.value into data.copy\n- the system joins bytes data.raw and data.other into data.copy",
    ));
    for role in [
        "bytes_step",
        "bytes_source",
        "bytes_other",
        "bytes_destination",
        "bytes_index",
        "bytes_start",
        "bytes_end",
        "bytes_value",
    ] {
        for mutation in ["duplicate", "same-id", "metadata", "reverse", "missing"] {
            let mut candidate = original.clone();
            let index = candidate
                .graph
                .edges
                .iter()
                .position(|e| e.kind == role)
                .unwrap();
            match mutation {
                "duplicate" | "same-id" => {
                    let mut extra = candidate.graph.edges[index].clone();
                    if mutation == "duplicate" {
                        extra.id = format!("extra-{role}");
                    }
                    extra.ordinal = candidate.graph.edges.len();
                    candidate.graph.edges.push(extra);
                }
                "metadata" => {
                    candidate.graph.edges[index]
                        .attributes
                        .insert("hidden".into(), "ignored".into());
                }
                "reverse" => {
                    let edge = &mut candidate.graph.edges[index];
                    std::mem::swap(&mut edge.source, &mut edge.target);
                }
                _ => {
                    candidate.graph.edges.remove(index);
                }
            }
            rejected(&candidate, &format!("{role}/{mutation}"));
        }
    }
    for mutation in [
        "extra-attribute",
        "typed-op",
        "orphan-field",
        "local-field",
        "duplicate-field-id",
        "duplicate-owner-id",
    ] {
        let mut candidate = original.clone();
        let field_id = candidate
            .graph
            .edges
            .iter()
            .find(|e| e.kind == "bytes_source")
            .unwrap()
            .target
            .clone();
        match mutation {
            "extra-attribute" | "typed-op" => {
                let op = candidate
                    .graph
                    .nodes
                    .iter_mut()
                    .find(|n| n.kind == "BytesOperation")
                    .unwrap();
                if mutation == "typed-op" {
                    op.type_name = Some("Bytes".into());
                } else {
                    op.attributes
                        .insert("runtime_source".into(), "forged.raw".into());
                }
            }
            "orphan-field" => candidate
                .graph
                .edges
                .retain(|e| e.kind != "has_field" || e.target != field_id),
            "local-field" => {
                candidate
                    .graph
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == field_id)
                    .unwrap()
                    .attributes
                    .insert("function_owner".into(), "foreign".into());
            }
            _ => {
                let id = if mutation == "duplicate-owner-id" {
                    candidate
                        .graph
                        .edges
                        .iter()
                        .find(|e| e.kind == "has_field" && e.target == field_id)
                        .unwrap()
                        .source
                        .clone()
                } else {
                    field_id
                };
                let extra = candidate
                    .graph
                    .nodes
                    .iter()
                    .find(|n| n.id == id)
                    .unwrap()
                    .clone();
                candidate.graph.nodes.push(extra);
            }
        }
        rejected(&candidate, mutation);
    }
}

#[test]
fn byte_sources_precede_integer_errors_but_all_integer_operands_precede_bounds() {
    let program = compile_ail_core_bytecode(&core(&source("- the system sets byte data.index / data.start in bytes data.raw to data.value / data.end into data.copy"))).unwrap();
    for (raw, index, start, value, end, error) in [
        ("hex:FF", "0", "0", "256", "0", "BYTES"),
        ("hex:00", "0", "0", "256", "0", "DIVZERO"),
        ("hex:00", "99", "1", "1", "0", "DIVZERO"),
        ("hex:00", "99", "1", "256", "1", "INDEX"),
        ("hex:00", "0", "1", "256", "1", "BYTES"),
        ("hex:00", "-9223372036854775808", "1", "0", "1", "INDEX"),
    ] {
        let state = BTreeMap::from([
            ("data.raw".into(), raw.into()),
            ("data.index".into(), index.into()),
            ("data.start".into(), start.into()),
            ("data.value".into(), value.into()),
            ("data.end".into(), end.into()),
        ]);
        let failure = run_ail_bytecode_action(&program, "Run", state).unwrap_err();
        assert!(
            failure.contains(&format!("AIL-RUNTIME-{error}:")),
            "{failure}"
        );
    }
}

#[test]
fn recovery_preserves_destination_and_input_aliases_with_present_empty_values() {
    let program = compile_ail_core_bytecode(&core(&source("- the system assigns \"hex:0080ff\" to data.raw\n- the system changes the data raw as the data copy\n- try:\n- the system sets byte 0 in bytes data.raw to 256 into data.raw\n- recover BYTES into data.failure:\n- the system counts bytes in data.copy into data.count\n- end try\n- the system slices bytes data.raw from 2 to 1 into data.raw"))).unwrap();
    let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
    assert_eq!(result.final_state["data.copy"], "hex:0080ff");
    assert_eq!(result.final_state["data.raw"], "hex:");
    assert_eq!(result.final_state["data.count"], "3");
    assert_eq!(result.final_state["data.failure"], "AIL-RUNTIME-BYTES");
}

#[test]
fn bytes_literals_cannot_coerce_other_types_and_large_literals_survive_roundtrip() {
    for literal in [
        "\"hex:FF\"",
        "\"hex:0\"",
        "\"hex:gg\"",
        "\"hex:00 \"",
        "\"00\"",
        "0",
        "null",
        "[0]",
    ] {
        let text = source(&format!("- the system assigns {literal} to data.raw"));
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        if let Ok(document) = parse_ail_package_spec_text(&package, &text) {
            assert!(
                compile_ail_core_bytecode(&elaborate_ail_core(&package, &document)).is_err(),
                "accepted {literal}"
            );
        }
    }
    for body in [
        "- the system counts bytes in data.text into data.count",
        "- the system counts characters in data.raw into data.count",
        "- the system parses JSON text data.text into data.raw",
        "- the system computes data.count as data.raw + 1",
    ] {
        let package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
        if let Ok(document) = parse_ail_package_spec_text(&package, &source(body)) {
            assert!(
                compile_ail_core_bytecode(&elaborate_ail_core(&package, &document)).is_err(),
                "accepted {body}"
            );
        }
    }
    let transport = format!("hex:{}", "80".repeat(524_289));
    let original = core(&source(&format!(
        "- the system assigns \"{transport}\" to data.raw\n- the system counts bytes in data.raw into data.count"
    )));
    for candidate in [
        original.clone(),
        parse_ail_core_text(&render_ail_core(&original)).unwrap(),
        core(&render_ail_spec_from_core(&original)),
    ] {
        let direct = compile_ail_core_bytecode(&candidate).unwrap();
        let program = ail::ste::parse_ail_ste(&ail::ste::render_ail_ste(&direct).unwrap()).unwrap();
        let result = run_ail_bytecode_action(&program, "Run", BTreeMap::new()).unwrap();
        assert_eq!(result.final_state["data.raw"], transport);
        assert_eq!(result.final_state["data.count"], "524289");
    }
}

#[test]
fn bytes_semantic_accesses_and_participating_provenance_cannot_be_bypassed() {
    let original = core(&source(
        "- the system gets byte data.index from bytes data.raw into data.value\n- the system slices bytes data.raw from 0 to data.end into data.copy",
    ));
    for role in ["bytes_source", "bytes_destination"] {
        let mut candidate = original.clone();
        let id = candidate
            .graph
            .edges
            .iter()
            .find(|e| e.kind == role)
            .unwrap()
            .target
            .clone();
        candidate
            .graph
            .nodes
            .iter_mut()
            .find(|n| n.id == id)
            .unwrap()
            .attributes
            .insert("secret".into(), "true".into());
        candidate
            .graph
            .edges
            .retain(|e| !matches!(e.kind.as_str(), "reads" | "writes"));
        rejected(&candidate, &format!("secret semantic {role}"));
    }
    let mut participating = std::collections::BTreeSet::new();
    for edge in &original.graph.edges {
        if edge.kind.starts_with("bytes_") {
            participating.insert(edge.source.clone());
            participating.insert(edge.target.clone());
        }
    }
    for edge in &original.graph.edges {
        if edge.kind == "has_field" && participating.contains(&edge.target) {
            participating.insert(edge.source.clone());
        }
    }
    for edge in &original.graph.edges {
        if edge.kind == "has_provenance" && participating.contains(&edge.source) {
            participating.insert(edge.target.clone());
        }
    }
    for kind in [
        "Action",
        "Thing",
        "BytesOperation",
        "Field",
        "IntegerExpression",
        "Provenance",
    ] {
        let targets: Vec<_> = original
            .graph
            .nodes
            .iter()
            .filter(|n| n.kind == kind && participating.contains(&n.id))
            .cloned()
            .collect();
        assert!(!targets.is_empty(), "missing fixture kind {kind}");
        for node in targets {
            let mut candidate = original.clone();
            candidate.graph.nodes.push(node);
            rejected(&candidate, &format!("ambiguous participating {kind}"));
        }
    }
}
