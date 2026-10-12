use ail::ail::*;
use ail::core_model::json_string;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn package() -> AilPackage {
    load_ail_package_dir("examples/support_ticket.ail").unwrap()
}

fn response(spec: &str) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!(
        "http://{}/v1/chat/completions",
        listener.local_addr().unwrap()
    );
    let body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(spec)
    );
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let mut request = Vec::new();
        let header_end = loop {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
            if request.ends_with(b"\r\n\r\n") {
                break request.len();
            }
        };
        let headers = String::from_utf8_lossy(&request);
        let length: usize = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|v| v.trim().parse().unwrap())
            })
            .unwrap();
        request.resize(header_end + length, 0);
        stream.read_exact(&mut request[header_end..]).unwrap();
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        String::from_utf8(request[header_end..].to_vec()).unwrap()
    });
    (endpoint, server)
}

#[test]
fn existing_direct_draft_rejects_checker_clean_destructive_stub() {
    let package = package();
    let stub = "The application SupportTicket manages customer support interactions.\nA Ticket has:\n- id: Text\n- status: State<Open, Closed>\nAction: Close ticket.\nWhen close ticket happens:\n- the system changes the ticket status to Closed\n- the system records a trace event named TicketClosed\n";
    let doc = parse_ail_package_spec_text(&package, stub).unwrap();
    assert!(check_ail_core(&elaborate_ail_core(&package, &doc)).is_empty());
    let (endpoint, server) = response(stub);
    let draft = draft_ail_spec(&package, "Reformat without changing behavior", &endpoint).unwrap();
    server.join().unwrap();
    assert!(!draft.success(), "destructive replacement was accepted");
    assert!(
        draft
            .diagnostics
            .iter()
            .any(|d| d.code.starts_with("AIL-EDIT-"))
    );
}

const HEAD: &str = "The application Audit manages numbers.\nA Data has:\n- value: Int\n";
const DEMO: &str = "Action: Demo.\nWhen Demo happens:\n- the system computes data.value as 7 + 2\n- the system returns data.value\n- the system records a trace event named Done\n";
fn arithmetic_package() -> AilPackage {
    let mut package = load_ail_package_dir("bench/R001-add/program.ail").unwrap();
    package.spec_text = format!("{HEAD}{DEMO}");
    package
}
fn checked(package: &AilPackage, spec: &str) -> AilCore {
    let core = elaborate_ail_core(
        package,
        &parse_ail_package_spec_text(package, spec).unwrap(),
    );
    assert!(
        check_ail_core(&core).is_empty(),
        "{:?}",
        check_ail_core(&core)
    );
    core
}
fn arithmetic_patch(base: &AilCore) -> String {
    let expression = base
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "IntegerExpression" && n.attributes.contains_key("operator"))
        .unwrap();
    format!(
        r#"{{"schema":"ail-core.patch.v0","package":{},"base_hash":{},"ops":[{{"op":"replace_node_attributes","target":{},"attributes":{{"operator":"-"}}}}]}}"#,
        json_string(&base.package.name),
        json_string(&ail_core_hash(base)),
        json_string(&format!("{}:{}", expression.kind, expression.name))
    )
}

#[test]
fn preserve_context_accepts_formatting_and_safe_additions_but_rejects_edits() {
    let package = arithmetic_package();
    let context = AilEditContext::preserve_package(&package).unwrap();
    for spec in [
        format!("\n{}\n", package.spec_text.replace("\n", "\n\n")),
        format!(
            "{}Action: Extra.\nWhen Extra happens:\n- the system records a trace event named ExtraDone\n",
            package.spec_text
        ),
    ] {
        let result = check_ail_draft_spec_with_edit_context(&package, spec, None, &context);
        assert!(result.success(), "{:?}", result.diagnostics);
    }
    for replacement in ["7 - 2", "8 + 2"] {
        let candidate = package.spec_text.replace("7 + 2", replacement);
        assert!(
            !check_ail_draft_spec_with_edit_context(&package, candidate, None, &context).success()
        );
    }
    let core = checked(&package, &package.spec_text);
    let report = context.render_admission(&core);
    assert!(report.contains("policy PreserveBaseline"));
    assert!(report.contains("admitted true"));
    assert_eq!(report, context.render_admission(&core));
}

#[test]
fn expected_patch_matches_exact_target_and_rejects_unrelated_edits() {
    let package = arithmetic_package();
    let base = checked(&package, &package.spec_text);
    let patch = arithmetic_patch(&base);
    let context = AilEditContext::expected_patch(&package, &patch).unwrap();
    let target = apply_ail_core_patch_text(&base, &patch).unwrap();
    assert!(context.check_candidate(&target).is_empty());
    assert!(!context.check_candidate(&base).is_empty());
    let mut extra = target.clone();
    extra
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.kind == "Action")
        .unwrap()
        .attributes
        .insert("label".into(), "Unrelated".into());
    assert!(!context.check_candidate(&extra).is_empty());
    let spec = package.spec_text.replace("7 + 2", "7 - 2");
    // Retained writes.provenance is an exact fact: changing the expression
    // requires authorizing that attribute as well, not only its operator.
    assert!(!check_ail_draft_spec_with_edit_context(&package, spec, None, &context).success());
    assert!(
        context
            .render_admission(&target)
            .contains("exact-expected-target-matching true")
    );
    assert!(
        AilEditContext::expected_core_patch(&base, &patch)
            .unwrap()
            .check_candidate(&target)
            .is_empty()
    );
}

#[test]
fn invalid_baselines_and_stale_or_wrong_package_targets_fail_before_network() {
    let package = arithmetic_package();
    let base = checked(&package, &package.spec_text);
    let patch = arithmetic_patch(&base);
    assert!(
        AilEditContext::expected_patch(&package, &patch.replace(&ail_core_hash(&base), "stale"))
            .unwrap_err()
            .contains("AIL-EDIT-")
    );
    assert!(
        AilEditContext::expected_patch(
            &package,
            &patch.replace(&json_string(&base.package.name), "\"wrong-package\"")
        )
        .is_err()
    );
    let mut malformed = package.clone();
    malformed.spec_text = "nonsense".into();
    assert!(
        draft_ail_spec(&malformed, "draft", "http://127.0.0.1:1")
            .unwrap_err()
            .contains("AIL-EDIT-")
    );
    let mut ambiguous = base.clone();
    let mut duplicate = ambiguous
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Action")
        .unwrap()
        .clone();
    duplicate.id.push_str("-duplicate");
    ambiguous.graph.nodes.push(duplicate);
    assert!(AilEditContext::preserve_core(&ambiguous).is_err());
    assert!(AilEditContext::expected_core_patch(&ambiguous, &patch).is_err());
}

#[test]
fn greenfield_is_explicit_empty_source_and_fixes_metadata() {
    let mut package = arithmetic_package();
    assert!(AilEditContext::greenfield(&package).is_err());
    package.spec_text = " \n\t".into();
    assert!(
        AilEditContext::preserve_package(&package)
            .unwrap_err()
            .contains("greenfield")
    );
    let context = AilEditContext::greenfield(&package).unwrap();
    let candidate = checked(&package, &format!("{HEAD}{DEMO}"));
    assert!(context.check_candidate(&candidate).is_empty());
    let mut changed = candidate.clone();
    changed.package.version = "99".into();
    assert!(!context.check_candidate(&changed).is_empty());
    assert!(
        context
            .render_admission(&candidate)
            .contains("policy Greenfield")
    );
}

#[test]
fn captured_snapshot_cannot_be_replaced_between_retries() {
    let package = arithmetic_package();
    let context = AilEditContext::preserve_package(&package).unwrap();
    let mut changed = package.clone();
    changed.spec_text = changed.spec_text.replace("7 + 2", "7 - 2");
    let result =
        check_ail_draft_spec_with_edit_context(&changed, changed.spec_text.clone(), None, &context);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "AIL-EDIT-SNAPSHOT")
    );
    assert!(
        draft_ail_spec_with_edit_context(&changed, "draft", "http://127.0.0.1:1", &context)
            .unwrap_err()
            .contains("AIL-EDIT-SNAPSHOT")
    );
    assert!(
        check_ail_draft_spec_with_edit_context(&package, package.spec_text.clone(), None, &context)
            .success()
    );
}

fn complete_arithmetic_patch(base: &AilCore) -> String {
    let patch = arithmetic_patch(base);
    let write = base
        .graph
        .edges
        .iter()
        .find(|e| e.kind == "writes")
        .unwrap();
    let source = base
        .graph
        .nodes
        .iter()
        .find(|n| n.id == write.source)
        .unwrap();
    let target = base
        .graph
        .nodes
        .iter()
        .find(|n| n.id == write.target)
        .unwrap();
    let extra_op = format!(
        r#",{{"op":"replace_edge_attributes","kind":"writes","source":{},"target":{},"attributes":{{"provenance":{}}}}}"#,
        json_string(&format!("{}:{}", source.kind, source.name)),
        json_string(&format!("{}:{}", target.kind, target.name)),
        json_string(&write.attributes["provenance"].replace("7 + 2", "7 - 2"))
    );
    patch.strip_suffix("]}").unwrap().to_string() + &extra_op + "]}"
}

#[test]
fn fully_authorized_arithmetic_change_admits_spec_and_executes() {
    let package = arithmetic_package();
    let base = checked(&package, &package.spec_text);
    let patch = complete_arithmetic_patch(&base);
    let context = AilEditContext::expected_patch(&package, &patch).unwrap();
    let spec = package.spec_text.replace("7 + 2", "7 - 2");
    let result = check_ail_draft_spec_with_edit_context(&package, spec.clone(), None, &context);
    assert!(result.success(), "{:?}", result.diagnostics);
    let target = checked(&package, &spec);
    let bytecode = compile_ail_core_bytecode(&target).unwrap();
    let run =
        run_ail_bytecode_action(&bytecode, "Demo", std::collections::BTreeMap::new()).unwrap();
    assert_eq!(
        run.final_state.get("data.value").map(String::as_str),
        Some("5")
    );
    let extra = format!(
        "{spec}Action: Extra.\nWhen Extra happens:\n- the system records a trace event named ExtraDone\n"
    );
    assert!(!check_ail_draft_spec_with_edit_context(&package, extra, None, &context).success());
}

#[test]
fn explicitly_authorized_trace_replacement_admits_exact_spec() {
    let package = arithmetic_package();
    let base = checked(&package, &package.spec_text);
    let old = base
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Trace" && n.name == "Done")
        .unwrap();
    let mut operations = vec![];
    for edge in base
        .graph
        .edges
        .iter()
        .filter(|e| e.source == old.id || e.target == old.id)
    {
        let source = base
            .graph
            .nodes
            .iter()
            .find(|n| n.id == edge.source)
            .unwrap();
        let target = base
            .graph
            .nodes
            .iter()
            .find(|n| n.id == edge.target)
            .unwrap();
        operations.push(format!(
            r#"{{"op":"remove_edge","kind":{},"source":{},"target":{}}}"#,
            json_string(&edge.kind),
            json_string(&format!("{}:{}", source.kind, source.name)),
            json_string(&format!("{}:{}", target.kind, target.name))
        ));
    }
    operations.extend([
        r#"{"op":"remove_node","target":"Trace:Done"}"#.into(),
        r#"{"op":"add_node","kind":"Trace","name":"Reviewed","provenance":["trace:Reviewed"]}"#
            .into(),
        r#"{"op":"add_edge","kind":"records_trace","source":"Action:Demo","target":"Trace:Reviewed"}"#
            .into(),
    ]);
    let patch = format!(
        r#"{{"schema":"ail-core.patch.v0","base_hash":{},"ops":[{}]}}"#,
        json_string(&ail_core_hash(&base)),
        operations.join(",")
    );
    let context = AilEditContext::expected_patch(&package, &patch).unwrap();
    let result = check_ail_draft_spec_with_edit_context(
        &package,
        package.spec_text.replace("named Done", "named Reviewed"),
        None,
        &context,
    );
    assert!(result.success(), "{:?}", result.diagnostics);
    assert!(!context.check_candidate(&base).is_empty());
}

#[test]
fn resolved_imports_are_captured_for_preserve_and_greenfield() {
    let mut package = arithmetic_package();
    let import = AilImportSpec {
        path: "captured-only".into(),
        version: None,
        alias: "Library".into(),
        resolved_package: None,
        registry_identity: None,
    };
    package.metadata.imports.push(import.clone());
    package.imports.push(AilLoadedImport {
        spec: import,
        package: Box::new(arithmetic_package()),
    });
    let preserve = AilEditContext::preserve_package(&package).unwrap();
    let mut changed = package.clone();
    changed.imports[0].package.spec_text = changed.imports[0]
        .package
        .spec_text
        .replace("7 + 2", "7 - 2");
    assert!(
        check_ail_draft_spec_with_edit_context(
            &changed,
            changed.spec_text.clone(),
            None,
            &preserve
        )
        .diagnostics
        .iter()
        .any(|d| d.code == "AIL-EDIT-SNAPSHOT")
    );
    let mut green_package = package.clone();
    green_package.spec_text.clear();
    let green = AilEditContext::greenfield(&green_package).unwrap();
    let candidate = checked(&green_package, &package.spec_text);
    assert!(
        green.check_candidate(&candidate).is_empty(),
        "{:?}",
        green.check_candidate(&candidate)
    );
    let mut modified = candidate.clone();
    modified
        .graph
        .nodes
        .iter_mut()
        .find(|n| {
            n.kind == "IntegerExpression"
                && n.name.contains("Library")
                && n.attributes.contains_key("operator")
        })
        .unwrap()
        .attributes
        .insert("operator".into(), "-".into());
    assert!(!green.check_candidate(&modified).is_empty());
    let mut deleted = candidate.clone();
    let imported_action = deleted
        .graph
        .nodes
        .iter()
        .find(|n| n.kind == "Action" && n.name.contains("Library"))
        .unwrap()
        .id
        .clone();
    deleted.graph.nodes.retain(|n| n.id != imported_action);
    deleted
        .graph
        .edges
        .retain(|e| e.source != imported_action && e.target != imported_action);
    assert!(!green.check_candidate(&deleted).is_empty());
}

#[test]
fn draft_variants_and_repair_use_exact_target_and_immutable_retry_context() {
    let package = arithmetic_package();
    let base = checked(&package, &package.spec_text);
    let context =
        AilEditContext::expected_patch(&package, &complete_arithmetic_patch(&base)).unwrap();
    let target_spec = package.spec_text.replace("7 + 2", "7 - 2");
    let requirements = "AIL-Requirements:\n- Demo records Done.\n";
    for variant in 0..5 {
        let (endpoint, server) = response(&target_spec);
        let result = match variant {
            0 => {
                draft_ail_spec_with_edit_context(&package, "change arithmetic", &endpoint, &context)
                    .unwrap()
            }
            1 => draft_ail_spec_from_requirements_with_edit_context(
                &package,
                "change arithmetic",
                requirements,
                &endpoint,
                &context,
            )
            .unwrap(),
            2 => {
                draft_ail_spec_from_requirements_recorded_with_edit_context(
                    &package,
                    "change arithmetic",
                    requirements,
                    &endpoint,
                    &context,
                )
                .unwrap()
                .0
            }
            3 => {
                draft_ail_spec_from_requirements_recorded_with_max_tokens_with_edit_context(
                    &package,
                    "change arithmetic",
                    requirements,
                    &endpoint,
                    1024,
                    &context,
                )
                .unwrap()
                .0
            }
            _ => {
                let rejected = check_ail_draft_spec_with_edit_context(
                    &package,
                    package.spec_text.clone(),
                    None,
                    &context,
                );
                assert!(!rejected.success());
                repair_ail_spec_from_diagnostics_with_edit_context(
                    &package,
                    "change arithmetic",
                    requirements,
                    &rejected.spec_text,
                    &rejected.diagnostics,
                    &endpoint,
                    &context,
                )
                .unwrap()
            }
        };
        let prompt = server.join().unwrap();
        assert!(
            result.success(),
            "variant {variant}: {:?}",
            result.diagnostics
        );
        assert!(prompt.contains("ExpectedPatchTarget"));
        assert!(prompt.contains("writes.provenance"));
        assert!(prompt.contains("operator=-"));
    }
    // A failed model candidate is never promoted to a baseline for repair.
    assert!(!context.check_candidate(&base).is_empty());
}

#[test]
fn requirements_variants_receive_the_same_expected_authority() {
    let package = arithmetic_package();
    let base = checked(&package, &package.spec_text);
    let context =
        AilEditContext::expected_patch(&package, &complete_arithmetic_patch(&base)).unwrap();
    let requirements =
        "AIL-Requirements:\n- Demo computes arithmetic.\n- Demo records Done.\n- No failures.\n";
    for variant in 0..5 {
        let (endpoint, server) = response(requirements);
        match variant {
            0 => {
                draft_ail_requirements_with_edit_context(
                    &package,
                    "requirements",
                    &endpoint,
                    &context,
                )
                .unwrap();
            }
            1 => {
                draft_ail_requirements_response_with_edit_context(
                    &package,
                    "requirements",
                    &endpoint,
                    &context,
                )
                .unwrap();
            }
            2 => {
                draft_ail_requirements_response_recorded_with_edit_context(
                    &package,
                    "requirements",
                    &endpoint,
                    &context,
                )
                .unwrap();
            }
            3 => {
                draft_ail_requirements_response_recorded_with_max_tokens_with_edit_context(
                    &package,
                    "requirements",
                    &endpoint,
                    1024,
                    &context,
                )
                .unwrap();
            }
            _ => {
                repair_ail_requirements_from_diagnostics_with_edit_context(
                    &package,
                    "requirements",
                    requirements,
                    &[],
                    &endpoint,
                    &context,
                )
                .unwrap();
            }
        }
        let prompt = server.join().unwrap();
        assert!(prompt.contains("ExpectedPatchTarget"));
        assert!(prompt.contains("operator=-"));
    }
}

#[test]
fn invalid_expected_target_and_invalid_candidate_fail_closed() {
    let package = arithmetic_package();
    let base = checked(&package, &package.spec_text);
    let patch = arithmetic_patch(&base).replace("\"operator\":\"-\"", "\"operator\":\"?\"");
    assert!(
        AilEditContext::expected_patch(&package, &patch)
            .unwrap_err()
            .contains("AIL-EDIT-TARGET")
    );
    let preserve = AilEditContext::preserve_package(&package).unwrap();
    let invalid = package.spec_text.replace("named Done", "named");
    let result = check_ail_draft_spec_with_edit_context(&package, invalid, None, &preserve);
    assert!(!result.success());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| !d.code.starts_with("AIL-EDIT-"))
    );
    assert!(preserve.check_candidate(&base).is_empty());
    let mut invalid_core = base.clone();
    invalid_core.graph.edges[0].ordinal = invalid_core.graph.edges[1].ordinal;
    assert!(AilEditContext::preserve_core(&invalid_core).is_err());
    assert!(!preserve.check_candidate(&invalid_core).is_empty());
}

#[test]
fn standalone_requirements_authoring_works_before_local_source_exists() {
    let mut package = arithmetic_package();
    package.spec_text.clear();
    let requirements =
        "AIL-Requirements:\n- Manage numbers.\n- Demo computes arithmetic.\n- Record Done.";
    for variant in 0..5 {
        let (endpoint, server) = response(requirements);
        let result = match variant {
            0 => {
                let text =
                    draft_ail_requirements(&package, "capture initial requirements", &endpoint)
                        .unwrap();
                ail::llm::LlmArtifactResponse::Artifact(text)
            }
            1 => {
                draft_ail_requirements_response(&package, "capture initial requirements", &endpoint)
                    .unwrap()
            }
            2 => {
                draft_ail_requirements_response_recorded(
                    &package,
                    "capture initial requirements",
                    &endpoint,
                )
                .unwrap()
                .outcome
            }
            3 => {
                draft_ail_requirements_response_recorded_with_max_tokens(
                    &package,
                    "capture initial requirements",
                    &endpoint,
                    1024,
                )
                .unwrap()
                .outcome
            }
            _ => ail::llm::LlmArtifactResponse::Artifact(
                repair_ail_requirements_from_diagnostics(
                    &package,
                    "capture initial requirements",
                    requirements,
                    &[],
                    &endpoint,
                )
                .unwrap(),
            ),
        };
        // Check before joining: rejecting an empty executable baseline here
        // must fail without waiting for the unused server.
        let ail::llm::LlmArtifactResponse::Artifact(text) = result else {
            panic!("standalone requirements returned questions");
        };
        assert_eq!(text, requirements);
        server.join().unwrap();
    }
}

fn permission_import_package() -> AilPackage {
    let mut imported = arithmetic_package();
    imported.spec_text = "The application Library manages permissions.\nA User has:\n- role: State<Allowed, Denied>\nAction: Protected.\nWhen Protected happens:\n- the system requires the user.role to be Allowed\n- the system records a trace event named ProtectedDone\n".into();
    let mut package = imported.clone();
    package.spec_text.clear();
    let import = AilImportSpec {
        path: "captured".into(),
        version: None,
        alias: "Library".into(),
        resolved_package: None,
        registry_identity: None,
    };
    package.metadata.imports.push(import.clone());
    package.imports.push(AilLoadedImport {
        spec: import,
        package: Box::new(imported),
    });
    package
}

#[test]
fn greenfield_rejects_local_addition_that_changes_imported_failure_selection() {
    let package = permission_import_package();
    let context = AilEditContext::greenfield(&package).unwrap();
    let local = "The application Consumer manages work.\n";
    let original = checked(&package, local);
    let added = checked(
        &package,
        &format!(
            "{local}Failure PermissionDenied happens when access is denied:\n- the caller sees \"Local denied\"\n- the trace records LocalDenied\n"
        ),
    );
    let denied = std::collections::BTreeMap::from([("library.user.role".into(), "Denied".into())]);
    let original_run = run_ail_bytecode_action(
        &compile_ail_core_bytecode(&original).unwrap(),
        "Library.Protected",
        denied.clone(),
    )
    .unwrap();
    let added_run = run_ail_bytecode_action(
        &compile_ail_core_bytecode(&added).unwrap(),
        "Library.Protected",
        denied,
    )
    .unwrap();
    assert_eq!(original_run.failure.as_deref(), Some("RequirementFailed"));
    assert_eq!(added_run.failure.as_deref(), Some("PermissionDenied"));
    assert!(added_run.trace.contains(&"trace LocalDenied".to_string()));
    assert!(context.check_candidate(&original).is_empty());
    let diagnostics = context.check_candidate(&added);
    assert!(
        !diagnostics.is_empty(),
        "local addition changed imported runtime behavior but was admitted"
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == "AIL-EDIT-REJECTED" && d.message.contains("Library.Protected"))
    );
    let report = context.render_admission(&added);
    assert!(report.contains("admitted false"));
    assert!(report.contains("lowering-validation existing-records-changed"));
}

#[test]
fn greenfield_allows_harmless_local_declarations_and_calls_into_imports() {
    let package = permission_import_package();
    let context = AilEditContext::greenfield(&package).unwrap();
    let candidate = checked(
        &package,
        "The application Consumer manages work.\nA Local has:\n- label: Text\nFailure LocalMissing happens when local work is missing:\n- the caller sees \"Missing local work\"\n- the trace records LocalMissingTrace\nAction: Use library.\nWhen Use library happens:\n- the system calls Library.Protected\n- the system records a trace event named LocalDone\n",
    );
    let diagnostics = context.check_candidate(&candidate);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let bytecode = compile_ail_core_bytecode(&candidate).unwrap();
    let allowed =
        std::collections::BTreeMap::from([("library.user.role".into(), "Allowed".into())]);
    let run = run_ail_bytecode_action(&bytecode, "UseLibrary", allowed).unwrap();
    assert_eq!(run.status, "succeeded");
    assert!(
        run.trace
            .contains(&"trace Library.ProtectedDone".to_string())
    );
    assert!(run.trace.contains(&"trace LocalDone".to_string()));
    assert!(
        context
            .render_admission(&candidate)
            .contains("lowering-validation existing-records-preserved")
    );
}

#[test]
fn greenfield_nonlowerable_import_retains_narrow_structural_evidence() {
    let mut package = permission_import_package();
    package.metadata.profile = "AIL-Meta".into();
    let context = AilEditContext::greenfield(&package).unwrap();
    let candidate = checked(
        &package,
        "The application Consumer manages work.\nA Local has:\n- label: Text\n",
    );
    assert!(compile_ail_core_bytecode(&candidate).is_err());
    assert!(context.check_candidate(&candidate).is_empty());
    let report = context.render_admission(&candidate);
    assert!(report.contains("admitted true"));
    assert!(
        report.contains(
            "lowering-validation baseline-unavailable (structural/effective-payload-only)"
        )
    );
}
