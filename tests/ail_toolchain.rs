use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use ail::ail::{
    DEFAULT_BASE_LLM_ENDPOINT, ail_core_hash, apply_ail_core_patch_text, apply_ail_patch,
    check_ail_core, check_ail_core_diagnostics, compile_ail_bytecode,
    compile_ail_bytecode_native_elf, compile_ail_core_bytecode, compile_ail_core_native_elf,
    elaborate_ail_core, load_ail_package_dir, parse_ail_bytecode, parse_ail_core_text,
    parse_ail_package_document, parse_ail_package_spec_text, parse_ail_patch_text,
    parse_ail_spec_text, render_ail_bytecode, render_ail_core, render_ail_flow_view,
    render_ail_package_dependency_report, render_ail_spec, render_ail_spec_from_core,
    run_ail_action, run_ail_bytecode_action, run_ail_compiler_pass_on_core, run_ail_conformance,
    verify_ail_bytecode,
};
use ail::core_model::json_string;

const REQUIREMENTS_PROMPT_ASSET: &str = include_str!("../docs/ail/prompts/requirements.system.md");
const SPEC_DRAFT_PROMPT_ASSET: &str = include_str!("../docs/ail/prompts/spec-draft.system.md");
const INTERVIEW_PROMPT_ASSET: &str = include_str!("../docs/ail/prompts/interview.system.md");

fn fixture(name: &str) -> String {
    format!("{}/examples/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn fnv64_fingerprint(text: &str) -> String {
    fnv64_fingerprint_bytes(text.as_bytes())
}

fn fnv64_fingerprint_bytes(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv64:{hash:016x}")
}

fn e2e_corpus_entry_text(index: usize, overrides: &[(&str, &str)]) -> String {
    let prompt_files = [
        "docs/ail/prompts/interview.system.md",
        "docs/ail/prompts/requirements.system.md",
        "docs/ail/prompts/spec-draft.system.md",
        "docs/ail/prompts/core-draft.system.md",
        "docs/ail/prompts/diagnostic-repair.system.md",
        "docs/ail/prompts/core-to-spec.system.md",
        "docs/ail/prompts/core-to-summary.system.md",
        "docs/ail/prompts/flow-patch.system.md",
        "docs/ail/prompts/trace-debug.system.md",
        "docs/ail/prompts/interop.system.md",
    ];
    let prompt_file = prompt_files[index % prompt_files.len()];
    let profile = match index {
        0..=39 => "Application",
        40..=54 => "AgentTool",
        55..=64 => "Compiler",
        _ => "System",
    };
    let surface_tags = match index {
        0..=9 => "standard-library",
        10..=19 => "package-import",
        20..=24 => "ui",
        25..=29 => "c-host-interop",
        30..=34 => "backend-portability",
        _ => "core",
    };
    let target = match index {
        85..=89 => "wasm32-unknown-sandbox-wasm",
        90..=94 => "aarch64-apple-darwin-libsystem-macho",
        95..=99 => "vm",
        _ => "linux-x86_64-elf",
    };
    let executor_family = if index == 99 {
        "codex-skill-agent"
    } else {
        "llm-http"
    };
    let mut fields = BTreeMap::from([
        ("semantic-task", format!("support-ticket-{index}")),
        ("profile", profile.to_string()),
        ("surface-tags", surface_tags.to_string()),
        ("package", "examples/support_ticket.ail".to_string()),
        ("prompt-file", prompt_file.to_string()),
        ("prompt-version", "ail-prompts.v0.2".to_string()),
        ("prompt-fingerprint", format!("fnv64:prompt-{index}")),
        ("executor-family", executor_family.to_string()),
        ("executor-label", "local-executor".to_string()),
        ("capture-origin", "deterministic-seed".to_string()),
        ("request-file", format!("requests/example-{index}.json")),
        ("response-file", format!("responses/example-{index}.json")),
        ("artifact-kind", "ail-spec".to_string()),
        ("checker-result", "accepted".to_string()),
        ("target", target.to_string()),
        ("vm-action", "CloseTicket".to_string()),
        (
            "runtime-state",
            "ticket.id=T-1;ticket.status=Open".to_string(),
        ),
    ]);
    if executor_family == "llm-http" {
        let endpoint_label = if index == 1 {
            "local-endpoint-alt"
        } else {
            "local-endpoint"
        };
        fields.insert("endpoint-label", endpoint_label.to_string());
    }
    for (key, value) in overrides {
        fields.insert(key, (*value).to_string());
    }
    let mut text = format!("## End-To-End Example: example-{index}\n");
    for (key, value) in fields {
        text.push_str(&format!("{key}: {value}\n"));
    }
    text.push('\n');
    text
}

fn e2e_corpus_text_with_override(index_to_override: usize, overrides: &[(&str, &str)]) -> String {
    let mut corpus_text = String::new();
    for index in 0..100 {
        if index == index_to_override {
            corpus_text.push_str(&e2e_corpus_entry_text(index, overrides));
        } else {
            corpus_text.push_str(&e2e_corpus_entry_text(index, &[]));
        }
    }
    corpus_text
}

fn write_e2e_transcript_files(root: &std::path::Path, count: usize) {
    fs::create_dir_all(root.join("requests")).unwrap();
    fs::create_dir_all(root.join("responses")).unwrap();
    let spec_text = fs::read_to_string(format!(
        "{}/examples/support_ticket.ail/spec.ail-spec.md",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    for index in 0..count {
        fs::write(
            root.join("requests").join(format!("example-{index}.json")),
            format!(r#"{{"prompt":"example-{index}"}}"#),
        )
        .unwrap();
        fs::write(
            root.join("responses").join(format!("example-{index}.json")),
            &spec_text,
        )
        .unwrap();
    }
}

fn detailed_ail_diagnostic(core: &ail::ail::AilCore, code: &str, message: &str) -> String {
    check_ail_core_diagnostics(core)
        .into_iter()
        .find(|diagnostic| diagnostic.code == code && diagnostic.message == message)
        .unwrap_or_else(|| panic!("missing diagnostic {code} {message}"))
        .detailed_message()
}

fn serve_one_chat_response(
    listener: TcpListener,
    response_body: String,
) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "test LLM endpoint did not receive a request"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("test LLM endpoint accept failed: {error}"),
            }
        };
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            let read = reader.read_line(&mut line).unwrap();
            if read == 0 || line == "\r\n" {
                break;
            }
            if let Some(value) = line
                .strip_prefix("Content-Length:")
                .or_else(|| line.strip_prefix("content-length:"))
            {
                content_length = value.trim().parse().unwrap();
            }
        }
        let mut request_body = vec![0; content_length];
        reader.read_exact(&mut request_body).unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8(request_body).unwrap()
    })
}

fn serve_one_llm_response_with_request_line(
    listener: TcpListener,
    response_body: String,
) -> thread::JoinHandle<(String, String)> {
    thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "test LLM endpoint did not receive a request"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("test LLM endpoint accept failed: {error}"),
            }
        };
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            let read = reader.read_line(&mut line).unwrap();
            if read == 0 || line == "\r\n" {
                break;
            }
            if let Some(value) = line
                .strip_prefix("Content-Length:")
                .or_else(|| line.strip_prefix("content-length:"))
            {
                content_length = value.trim().parse().unwrap();
            }
        }
        let mut request_body = vec![0; content_length];
        reader.read_exact(&mut request_body).unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream.write_all(response.as_bytes()).unwrap();
        (
            request_line.trim_end().to_string(),
            String::from_utf8(request_body).unwrap(),
        )
    })
}

fn serve_chat_responses(
    listener: TcpListener,
    response_bodies: Vec<String>,
) -> thread::JoinHandle<Vec<String>> {
    thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let mut request_bodies = Vec::new();
        for response_body in response_bodies {
            let deadline = Instant::now() + Duration::from_secs(2);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "test LLM endpoint did not receive a request"
                        );
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("test LLM endpoint accept failed: {error}"),
                }
            };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut content_length = 0usize;
            loop {
                let mut line = String::new();
                let read = reader.read_line(&mut line).unwrap();
                if read == 0 || line == "\r\n" {
                    break;
                }
                if let Some(value) = line
                    .strip_prefix("Content-Length:")
                    .or_else(|| line.strip_prefix("content-length:"))
                {
                    content_length = value.trim().parse().unwrap();
                }
            }
            let mut request_body = vec![0; content_length];
            reader.read_exact(&mut request_body).unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream.write_all(response.as_bytes()).unwrap();
            request_bodies.push(String::from_utf8(request_body).unwrap());
        }
        request_bodies
    })
}

fn observe_optional_chat_request(
    listener: TcpListener,
    response_body: String,
) -> thread::JoinHandle<Option<String>> {
    thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_millis(500);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return None;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("test LLM endpoint accept failed: {error}"),
            }
        };
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            let read = reader.read_line(&mut line).unwrap();
            if read == 0 || line == "\r\n" {
                break;
            }
            if let Some(value) = line
                .strip_prefix("Content-Length:")
                .or_else(|| line.strip_prefix("content-length:"))
            {
                content_length = value.trim().parse().unwrap();
            }
        }
        let mut request_body = vec![0; content_length];
        reader.read_exact(&mut request_body).unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream.write_all(response.as_bytes()).unwrap();
        Some(String::from_utf8(request_body).unwrap())
    })
}

#[test]
fn ail_package_loader_reads_metadata_and_default_llm_endpoint() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();

    assert_eq!(package.metadata.name, "support-ticket");
    assert_eq!(package.metadata.version, "0.1.0");
    assert_eq!(package.metadata.profile, "Application");
    assert_eq!(package.metadata.entry, "spec.ail-spec.md");
    assert_eq!(package.metadata.conformance, "first-slice");
    assert_eq!(
        package.metadata.base_llm_endpoint,
        DEFAULT_BASE_LLM_ENDPOINT
    );
    assert!(package.spec_text.contains("Action: Close ticket."));
}

#[test]
fn ail_agent_tool_profile_parses_renders_and_checks_refund_tool() {
    let package = load_ail_package_dir(fixture("refund_tool.ail")).unwrap();
    assert_eq!(package.metadata.profile, "AgentTool");

    let document = parse_ail_package_document(&package).unwrap();
    assert!(document.application.name.is_empty());
    let tool = document.tools.get("RefundCustomerPayment").unwrap();
    assert_eq!(tool.label, "Refund customer payment");
    assert!(tool.requirements.contains(&"the order exists".to_string()));
    assert_eq!(tool.inputs["payment token"].type_name, "Secret<Text>");
    assert!(tool.inputs["payment token"].is_secret);
    assert_eq!(tool.outputs["refund id"].type_name, "Text");
    assert!(tool.calls.contains(&"PaymentProvider.refund".to_string()));
    assert!(
        tool.permissions
            .contains(&"requester may create refunds".to_string())
    );
    assert!(
        tool.approvals
            .contains(&"manager approval when the refund amount is over USD 500".to_string())
    );
    assert!(
        tool.secret_protections
            .contains(&"the payment token".to_string())
    );
    assert!(
        tool.traces
            .contains(&"RefundCustomerPaymentRequested".to_string())
    );

    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains("node Tool RefundCustomerPayment"));
    assert!(
        rendered_core.contains("node Input RefundCustomerPayment.payment token : Secret<Text>")
    );
    assert!(rendered_core.contains("node Output RefundCustomerPayment.refund id : Text"));
    assert!(
        rendered_core
            .contains("edge calls Tool:RefundCustomerPayment -> Effect:PaymentProvider.refund")
    );
    assert!(rendered_core.contains("node Permission requester may create refunds"));
    assert!(rendered_core.contains(
        "edge requires Tool:RefundCustomerPayment -> Permission:requester may create refunds"
    ));
    assert!(rendered_core.contains(
        "edge requires_approval Tool:RefundCustomerPayment -> Approval:manager approval when the refund amount is over USD 500"
    ));
    assert!(rendered_core.contains(
        "edge protects_secret Tool:RefundCustomerPayment -> Secret:RefundCustomerPayment.payment token"
    ));
    assert!(rendered_core.contains(
        "edge guarantees Tool:RefundCustomerPayment -> Guarantee:payment token is redacted from all agent-visible output"
    ));
    assert!(rendered_core.contains(
        "edge records_trace Tool:RefundCustomerPayment -> Trace:RefundCustomerPaymentRequested"
    ));

    let flow = render_ail_flow_view(&core);
    assert!(flow.contains(r#""tools":["#), "{flow}");
    assert!(flow.contains(r#""name":"RefundCustomerPayment""#), "{flow}");
    assert!(flow.contains(r#""inputs":["#), "{flow}");
    assert!(
        flow.contains(r#""approvals":["manager approval when the refund amount is over USD 500"]"#),
        "{flow}"
    );
    assert!(
        flow.contains(r#""permissions":["requester may create refunds"]"#),
        "{flow}"
    );
    assert!(
        flow.contains(r#""traces":["RefundCustomerPaymentRequested"]"#),
        "{flow}"
    );
    assert!(
        flow.contains(
            r#"{"kind":"requires","source":"Tool:RefundCustomerPayment","target":"Permission:requester may create refunds","targetName":"requester may create refunds","attributes":{}}"#
        ),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("The tool requires permission:"));
    assert!(rendered_spec.contains("- requester may create refunds"));
    assert!(rendered_spec.contains("The tool requires approval:"));
    assert!(rendered_spec.contains("- manager approval when the refund amount is over USD 500"));
    assert!(rendered_spec.contains("The tool records:"));
    assert!(rendered_spec.contains("- RefundCustomerPaymentRequested"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_agent_tool_profile_lowers_to_verified_bytecode() {
    let package = load_ail_package_dir(fixture("refund_tool.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    let bytecode = compile_ail_bytecode(&package, &document).unwrap();
    let rendered = render_ail_bytecode(&bytecode);

    assert_eq!(bytecode.profile, "AgentTool");
    assert!(bytecode.actions.contains_key("RefundCustomerPayment"));
    assert!(rendered.contains(r#""opcode":"TOOL_BEGIN""#), "{rendered}");
    assert!(rendered.contains(r#""opcode":"TOOL_INPUT""#), "{rendered}");
    assert!(rendered.contains(r#""opcode":"TOOL_OUTPUT""#), "{rendered}");
    assert!(
        rendered.contains(r#""opcode":"TOOL_REQUIREMENT""#),
        "{rendered}"
    );
    assert!(rendered.contains(r#""opcode":"TOOL_CALL""#), "{rendered}");
    assert!(
        rendered.contains(r#""opcode":"TOOL_PERMISSION""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"TOOL_APPROVAL""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"TOOL_SECRET_PROTECTION""#),
        "{rendered}"
    );
    assert!(
        rendered.contains("RefundCustomerPaymentRequested"),
        "{rendered}"
    );

    let parsed = parse_ail_bytecode(&rendered).unwrap();
    assert_eq!(verify_ail_bytecode(&parsed), Vec::<String>::new());

    let run = run_ail_bytecode_action(
        &parsed,
        "RefundCustomerPayment",
        BTreeMap::from([
            ("order id".to_string(), "O-1".to_string()),
            ("payment token".to_string(), "tok_123".to_string()),
            ("refund amount".to_string(), "USD:25.00".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(run.status, "succeeded");
    assert!(
        run.trace
            .contains(&"tool Refund customer payment started".to_string())
    );
    assert!(
        run.trace
            .contains(&"trace RefundCustomerPaymentRequested".to_string())
    );
}

#[test]
fn ail_compiler_profile_parses_renders_and_checks_compiler_pass() {
    let package = load_ail_package_dir(fixture("compiler_pass.ail")).unwrap();
    assert_eq!(package.metadata.profile, "Compiler");

    let document = parse_ail_package_document(&package).unwrap();
    assert!(document.application.name.is_empty());
    let pass = document
        .compiler_passes
        .get("InferReadPermissions")
        .unwrap();
    assert_eq!(pass.label, "Infer read permissions");
    assert!(
        pass.purpose
            .contains("adds missing read permission requirements")
    );
    assert_eq!(pass.inputs["input graph"].type_name, "AIL-Core graph");
    assert_eq!(pass.outputs["diagnostics"].type_name, "List<Diagnostic>");
    assert!(
        pass.reads
            .contains(&"every edge whose kind is reads".to_string())
    );
    assert!(
        pass.writes
            .contains(&"a candidate read Permission".to_string())
    );
    assert!(
        pass.guarantees
            .contains(&"every added permission has provenance from this pass".to_string())
    );
    assert!(pass.traces.contains(&"ReadPermissionAdded".to_string()));
    assert!(
        pass.failures
            .contains(&"SecretReadNeedsHumanConfirmation".to_string())
    );

    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains("node Action InferReadPermissions [kind=CompilerPass"));
    assert!(rendered_core.contains("node Value InferReadPermissions.input graph : AIL-Core graph"));
    assert!(
        rendered_core.contains("node Value InferReadPermissions.diagnostics : List<Diagnostic>")
    );
    assert!(rendered_core.contains(
        "edge reads Action:InferReadPermissions -> Value:InferReadPermissions.input graph"
    ));
    assert!(
        rendered_core.contains(
            "edge writes Action:InferReadPermissions -> Effect:a candidate read Permission"
        )
    );
    assert!(rendered_core.contains("edge contains Action:InferReadPermissions -> Step:finds the actor, tool, view, or pass that performs the read"));
    assert!(rendered_core.contains(
        "edge may_fail_with Action:InferReadPermissions -> Failure:SecretReadNeedsHumanConfirmation"
    ));

    let flow = render_ail_flow_view(&core);
    assert!(flow.contains(r#""actions":[]"#), "{flow}");
    assert!(flow.contains(r#""compilerPasses":["#), "{flow}");
    assert!(flow.contains(r#""name":"InferReadPermissions""#), "{flow}");
    assert!(
        flow.contains(
            r#"{"kind":"reads","source":"Action:InferReadPermissions","target":"Value:InferReadPermissions.input graph","targetName":"InferReadPermissions.input graph","attributes":{}}"#
        ),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_compiler_profile_lowers_to_verified_bytecode() {
    let package = load_ail_package_dir(fixture("compiler_pass.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    let bytecode = compile_ail_bytecode(&package, &document).unwrap();
    let rendered = render_ail_bytecode(&bytecode);

    assert_eq!(bytecode.profile, "Compiler");
    assert!(bytecode.actions.contains_key("InferReadPermissions"));
    assert!(rendered.contains(r#""opcode":"PASS_BEGIN""#), "{rendered}");
    assert!(rendered.contains(r#""opcode":"PASS_INPUT""#), "{rendered}");
    assert!(rendered.contains(r#""opcode":"PASS_OUTPUT""#), "{rendered}");
    assert!(rendered.contains(r#""opcode":"PASS_READ""#), "{rendered}");
    assert!(rendered.contains(r#""opcode":"PASS_STEP""#), "{rendered}");
    assert!(rendered.contains(r#""opcode":"PASS_WRITE""#), "{rendered}");
    assert!(
        rendered.contains(r#""opcode":"CORE_INFER_READ_PERMISSIONS""#),
        "{rendered}"
    );
    assert!(rendered.contains("ReadPermissionAdded"), "{rendered}");

    let parsed = parse_ail_bytecode(&rendered).unwrap();
    assert_eq!(verify_ail_bytecode(&parsed), Vec::<String>::new());

    let run = run_ail_bytecode_action(
        &parsed,
        "InferReadPermissions",
        BTreeMap::from([
            ("input graph".to_string(), "checked graph".to_string()),
            ("package policy".to_string(), "infer reads".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(run.status, "succeeded");
    assert!(
        run.trace
            .contains(&"compiler pass Infer read permissions started".to_string())
    );
    assert!(run.trace.contains(&"trace ReadPermissionAdded".to_string()));
}

#[test]
fn ail_compiler_pass_bytecode_transforms_checked_core_ir() {
    let pass_package = load_ail_package_dir(fixture("compiler_pass.ail")).unwrap();
    let pass_document = parse_ail_package_document(&pass_package).unwrap();
    let pass_bytecode = compile_ail_bytecode(&pass_package, &pass_document).unwrap();

    let app_package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let app_document = parse_ail_package_document(&app_package).unwrap();
    let app_core = elaborate_ail_core(&app_package, &app_document);
    assert_eq!(check_ail_core(&app_core), Vec::<String>::new());

    let result =
        run_ail_compiler_pass_on_core(&pass_bytecode, "InferReadPermissions", &app_core).unwrap();
    let rendered = render_ail_core(&result.core);

    assert_eq!(result.run.status, "succeeded");
    assert!(
        result
            .run
            .trace
            .contains(&"trace ReadPermissionAdded".to_string())
    );
    assert!(
        result
            .run
            .trace
            .contains(&"core transform infer read permissions".to_string())
    );
    assert!(
        rendered.contains("node Permission read Ticket.status"),
        "{rendered}"
    );
    assert!(
        rendered
            .contains("edge requires Action:MarksOverdueTickets -> Permission:read Ticket.status"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "node Provenance compiler_pass:InferReadPermissions.permission:read Ticket.status"
        ),
        "{rendered}"
    );
    assert_eq!(check_ail_core(&result.core), Vec::<String>::new());
}

#[test]
fn ail_compiler_pass_transform_requires_explicit_bytecode_opcode() {
    let pass_package = load_ail_package_dir(fixture("compiler_pass.ail")).unwrap();
    let pass_document = parse_ail_package_document(&pass_package).unwrap();
    let mut pass_bytecode = compile_ail_bytecode(&pass_package, &pass_document).unwrap();
    let action = pass_bytecode
        .actions
        .get_mut("InferReadPermissions")
        .unwrap();
    action
        .instructions
        .retain(|instruction| instruction.opcode != "CORE_INFER_READ_PERMISSIONS");

    let app_package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let app_document = parse_ail_package_document(&app_package).unwrap();
    let app_core = elaborate_ail_core(&app_package, &app_document);
    assert_eq!(check_ail_core(&app_core), Vec::<String>::new());

    let result =
        run_ail_compiler_pass_on_core(&pass_bytecode, "InferReadPermissions", &app_core).unwrap();
    let rendered = render_ail_core(&result.core);

    assert!(
        !rendered.contains("node Permission read Ticket.status"),
        "{rendered}"
    );
    assert!(
        !result
            .run
            .trace
            .contains(&"core transform infer read permissions".to_string()),
        "{:?}",
        result.run.trace
    );
}

#[test]
fn ail_system_profile_parses_renders_and_checks_resource_capabilities() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    assert_eq!(package.metadata.profile, "System");

    let document = parse_ail_package_document(&package).unwrap();
    assert!(document.application.name.is_empty());

    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains("node SystemComponent NetworkPacketReceiver"));
    assert!(rendered_core.contains("node Resource NetworkPacketReceiver.rx buffer : Buffer"));
    assert!(rendered_core.contains("node Resource NetworkPacketReceiver.packet metadata : Buffer"));
    assert!(rendered_core.contains("node Resource NetworkPacketReceiver.network device : Device"));
    assert!(rendered_core.contains("node Capability access network device"));
    assert!(rendered_core.contains("node Capability read packet metadata"));
    assert!(rendered_core.contains(
        "edge uses_resource SystemComponent:NetworkPacketReceiver -> Resource:NetworkPacketReceiver.network device"
    ));
    assert!(rendered_core.contains(
        "edge owns_resource SystemComponent:NetworkPacketReceiver -> Resource:NetworkPacketReceiver.rx buffer"
    ));
    assert!(rendered_core.contains(
        "edge borrows_resource SystemComponent:NetworkPacketReceiver -> Resource:NetworkPacketReceiver.packet metadata"
    ));
    assert!(rendered_core.contains("node Region NetworkPacketReceiver.packet processing region"));
    assert!(rendered_core.contains(
        "edge uses_region SystemComponent:NetworkPacketReceiver -> Region:NetworkPacketReceiver.packet processing region"
    ));
    assert!(rendered_core.contains(
        "edge in_region Resource:NetworkPacketReceiver.rx buffer -> Region:NetworkPacketReceiver.packet processing region"
    ));
    assert!(rendered_core.contains(
        "edge in_region Resource:NetworkPacketReceiver.packet metadata -> Region:NetworkPacketReceiver.packet processing region"
    ));
    assert!(rendered_core.contains(
        "edge requires SystemComponent:NetworkPacketReceiver -> Capability:access network device"
    ));
    assert!(rendered_core.contains(
        "edge requires SystemComponent:NetworkPacketReceiver -> Capability:read packet metadata"
    ));
    assert!(rendered_core.contains(
        "edge authorizes_resource Capability:access network device -> Resource:NetworkPacketReceiver.network device"
    ));
    assert!(rendered_core.contains(
        "edge authorizes_resource Capability:read packet metadata -> Resource:NetworkPacketReceiver.packet metadata"
    ));
    assert!(rendered_core.contains(
        "edge performs SystemComponent:NetworkPacketReceiver -> Effect:read network device"
    ));
    assert!(rendered_core.contains(
        "edge performs SystemComponent:NetworkPacketReceiver -> Effect:read packet metadata"
    ));
    assert!(rendered_core.contains(
        "edge performs SystemComponent:NetworkPacketReceiver -> Effect:release rx buffer"
    ));
    assert!(rendered_core.contains(
        "edge targets_resource Effect:read network device -> Resource:NetworkPacketReceiver.network device"
    ));
    assert!(rendered_core.contains(
        "edge targets_resource Effect:read packet metadata -> Resource:NetworkPacketReceiver.packet metadata"
    ));
    assert!(rendered_core.contains(
        "edge targets_resource Effect:write rx buffer -> Resource:NetworkPacketReceiver.rx buffer"
    ));
    assert!(rendered_core.contains(
        "edge targets_resource Effect:release rx buffer -> Resource:NetworkPacketReceiver.rx buffer"
    ));
    assert!(rendered_core.contains(
        "edge records_trace SystemComponent:NetworkPacketReceiver -> Trace:PacketReceived"
    ));

    let flow = render_ail_flow_view(&core);
    assert!(flow.contains(r#""systemComponents":["#), "{flow}");
    assert!(flow.contains(r#""name":"NetworkPacketReceiver""#), "{flow}");
    assert!(
        flow.contains(r#""capabilities":["access network device","read packet metadata"]"#),
        "{flow}"
    );
    assert!(
        flow.contains(
            r#""effects":["read network device","read packet metadata","release rx buffer","write rx buffer"]"#
        ),
        "{flow}"
    );
    assert!(
        flow.contains(r#""owns":["NetworkPacketReceiver.rx buffer"]"#),
        "{flow}"
    );
    assert!(
        flow.contains(
            r#"{"kind":"performs","source":"SystemComponent:NetworkPacketReceiver","target":"Effect:read network device","targetName":"read network device","attributes":{"provenance":"system_component:NetworkPacketReceiver.effect:read network device"}}"#
        ),
        "{flow}"
    );
    assert!(
        flow.contains(r#""borrows":["NetworkPacketReceiver.packet metadata"]"#),
        "{flow}"
    );
    assert!(
        flow.contains(r#""regions":["NetworkPacketReceiver.packet processing region"]"#),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("System component: Network packet receiver."));
    assert!(rendered_spec.contains("The component uses:"));
    assert!(rendered_spec.contains("- network device: Device"));
    assert!(rendered_spec.contains("The component owns:"));
    assert!(rendered_spec.contains("- rx buffer"));
    assert!(rendered_spec.contains("The component borrows:"));
    assert!(rendered_spec.contains("- packet metadata"));
    assert!(rendered_spec.contains("The component places:"));
    assert!(rendered_spec.contains("- rx buffer in packet processing region"));
    assert!(rendered_spec.contains("- packet metadata in packet processing region"));
    assert!(rendered_spec.contains("The component requires capability:"));
    assert!(rendered_spec.contains("- access network device"));
    assert!(rendered_spec.contains("- read packet metadata"));
    assert!(rendered_spec.contains("The component performs:"));
    assert!(rendered_spec.contains("- read network device"));
    assert!(rendered_spec.contains("- read packet metadata"));
    assert!(rendered_spec.contains("- release rx buffer"));
    assert!(rendered_spec.contains("The component records:"));
    assert!(rendered_spec.contains("- PacketReceived"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_system_profile_lowers_to_verified_bytecode() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    let bytecode = compile_ail_bytecode(&package, &document).unwrap();
    let rendered = render_ail_bytecode(&bytecode);

    assert_eq!(bytecode.profile, "System");
    assert!(bytecode.actions.contains_key("NetworkPacketReceiver"));
    assert!(
        rendered.contains(r#""opcode":"SYSTEM_BEGIN""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"SYSTEM_RESOURCE""#),
        "{rendered}"
    );
    assert!(rendered.contains(r#""opcode":"SYSTEM_OWNS""#), "{rendered}");
    assert!(
        rendered.contains(r#""opcode":"SYSTEM_BORROWS""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"SYSTEM_REGION""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"SYSTEM_CAPABILITY""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"SYSTEM_EFFECT""#),
        "{rendered}"
    );
    assert!(rendered.contains("PacketReceived"), "{rendered}");

    let parsed = parse_ail_bytecode(&rendered).unwrap();
    assert_eq!(verify_ail_bytecode(&parsed), Vec::<String>::new());

    let run = run_ail_bytecode_action(&parsed, "NetworkPacketReceiver", BTreeMap::new()).unwrap();

    assert_eq!(run.status, "succeeded");
    assert!(
        run.trace
            .contains(&"system component Network packet receiver started".to_string())
    );
    assert!(run.trace.contains(&"trace PacketReceived".to_string()));
}

#[test]
fn ail_spec_parses_function_surface_into_core_and_round_trips() {
    let package = load_ail_package_dir(fixture("recursive_factorial.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);

    assert!(rendered_core.contains("node Function factorial"));
    assert!(rendered_core.contains("node Input factorial.n : Int"));
    assert!(rendered_core.contains("node Output factorial.result : Int"));
    assert!(rendered_core.contains("node Branch factorial.n is 0 [condition=n is 0]"));
    assert!(
        rendered_core.contains("node Call factorial.factorial with n minus 1 [target=factorial]")
    );
    assert!(rendered_core.contains("node Return factorial.1 [value=1]"));
    assert!(rendered_core.contains(
        "node Return factorial.n multiplied by the recursive result [value=n multiplied by the recursive result]"
    ));
    assert!(
        rendered_core
            .contains("edge calls Function:factorial -> Call:factorial.factorial with n minus 1")
    );
    assert!(
        rendered_core.contains("edge records_trace Function:factorial -> Trace:FactorialCalled")
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("Function: factorial."));
    assert!(rendered_spec.contains("The function needs:"));
    assert!(rendered_spec.contains("- n: Int"));
    assert!(rendered_spec.contains("When factorial runs:"));
    assert!(rendered_spec.contains("- otherwise the function calls factorial with n minus 1"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_spec_lowers_function_surface_into_runnable_bytecode() {
    let package = load_ail_package_dir(fixture("recursive_factorial.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let bytecode = compile_ail_bytecode(&package, &document).unwrap();

    assert!(bytecode.actions.contains_key("factorial"));
    let rendered = render_ail_bytecode(&bytecode);
    assert!(
        rendered.contains(r#""opcode":"FUNCTION_BEGIN""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"FUNCTION_INPUT""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"FUNCTION_OUTPUT""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"FUNCTION_BRANCH""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"FUNCTION_CALL""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"FUNCTION_RETURN""#),
        "{rendered}"
    );
    assert!(rendered.contains("FactorialCalled"), "{rendered}");

    let parsed = parse_ail_bytecode(&rendered).unwrap();
    assert_eq!(verify_ail_bytecode(&parsed), Vec::<String>::new());

    let run = run_ail_bytecode_action(
        &parsed,
        "factorial",
        BTreeMap::from([("n".to_string(), "3".to_string())]),
    )
    .unwrap();

    assert_eq!(run.status, "succeeded");
    assert!(
        run.trace
            .contains(&"function factorial started".to_string())
    );
    assert!(run.trace.contains(&"function input n:Int".to_string()));
    assert!(
        run.trace
            .contains(&"function output result:Int".to_string())
    );
    assert!(run.trace.contains(&"function branch n is 0".to_string()));
    assert!(run.trace.contains(&"function call factorial".to_string()));
    assert!(run.trace.contains(&"function return 1".to_string()));
    assert!(
        run.trace
            .contains(&"function return n multiplied by the recursive result".to_string())
    );
    assert!(run.trace.contains(&"trace FactorialCalled".to_string()));
}

#[test]
fn ail_c_interop_import_parses_into_external_binding_core() {
    let mut package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    package.metadata.profile = "C interop".to_string();
    let spec = r#"C library: zlib.

The library imports function compress2.

compress2 needs:

- dest: Pointer<UInt8> borrowed mutable
- dest_len: Pointer<UInt64> borrowed mutable
- source: Pointer<UInt8> borrowed
- source_len: UInt64
- level: Int

compress2 produces:

- status: CInt

compress2 maps errno or status codes:

- Z_OK maps to success
- Z_MEM_ERROR maps to Failure.OutOfMemory
- Z_BUF_ERROR maps to Failure.OutputBufferTooSmall

compress2 requires capability:

- call zlib compress2

compress2 records trace event named ForeignCallCompress2
"#;

    let document = parse_ail_spec_text(spec).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);

    assert!(
        rendered_core.contains(
            "node ExternalBinding zlib.compress2 [binding_kind=CFunction,library=zlib,symbol=compress2]"
        ),
        "{rendered_core}"
    );
    assert!(rendered_core.contains("node Layout zlib.compress2.signature : cdecl"));
    assert!(rendered_core.contains("node Input zlib.compress2.dest : Pointer<UInt8>"));
    assert!(rendered_core.contains("node Input zlib.compress2.dest_len : Pointer<UInt64>"));
    assert!(rendered_core.contains("node Output zlib.compress2.status : CInt"));
    assert!(rendered_core.contains("node StatusMap zlib.compress2.Z_OK : success [code=Z_OK]"));
    assert!(rendered_core.contains("node Capability call zlib compress2"));
    assert!(rendered_core.contains("node Failure OutOfMemory"));
    assert!(rendered_core.contains(
        "edge requires ExternalBinding:zlib.compress2 -> Capability:call zlib compress2"
    ));
    assert!(rendered_core.contains(
        "edge maps_status ExternalBinding:zlib.compress2 -> StatusMap:zlib.compress2.Z_OK [code=Z_OK]"
    ));
    assert!(
        rendered_core
            .contains("edge may_fail_with ExternalBinding:zlib.compress2 -> Failure:OutOfMemory")
    );
    assert!(rendered_core.contains(
        "edge records_trace ExternalBinding:zlib.compress2 -> Trace:ForeignCallCompress2"
    ));

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("C library: zlib."));
    assert!(rendered_spec.contains("The library imports function compress2."));
    assert!(rendered_spec.contains("- dest: Pointer<UInt8> borrowed mutable"));
    assert!(rendered_spec.contains("- Z_OK maps to success"));
    assert!(rendered_spec.contains("- call zlib compress2"));
    assert!(rendered_spec.contains("compress2 records trace event named ForeignCallCompress2"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn cli_ail_ffi_checks_struct_layout_fixture() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("c_interop.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("accepted: struct-layout-minimal.ail-spec.md"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ffi_checks_callback_lifetime_fixture() {
    let package = load_ail_package_dir(fixture("c_interop.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered = render_ail_core(&core);
    assert!(
        rendered.contains(
            "node ExternalBinding libc.qsort [binding_kind=CFunction,library=libc,symbol=qsort]"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains("node Input libc.qsort.comparator : Callback<Pointer<Void>,Pointer<Void>,CInt> [ownership=borrowed callback noescape]"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "edge records_trace ExternalBinding:libc.qsort -> Trace:ForeignCallbackCompared"
        ),
        "{rendered}"
    );
}

#[test]
fn cli_ail_ffi_rejects_borrowed_pointer_escape() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("c_interop.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rejected: borrowed-pointer-escape.ail-spec.md AIL-FFI-OWNERSHIP-001"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ffi_rejects_missing_status_map() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("c_interop.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rejected: missing-status-map.ail-spec.md AIL-FFI-ERRNO-001"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ffi_accepts_owned_pointer_release_fixture() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("c_interop.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("accepted: owned-pointer-release-minimal.ail-spec.md"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ffi_rejects_owned_pointer_without_release() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("c_interop.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout
            .contains("rejected: owned-pointer-without-release.ail-spec.md AIL-FFI-OWNERSHIP-002"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ffi_rejects_nullable_to_non_null_mismatch() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("c_interop.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rejected: nullable-to-non-null.ail-spec.md AIL-FFI-NULL-001"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ffi_rejects_mutable_pointer_aliasing() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("c_interop.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rejected: mutable-pointer-aliasing.ail-spec.md AIL-FFI-ALIAS-001"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ffi_rejects_secret_leakage() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("c_interop.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rejected: secret-leakage.ail-spec.md AIL-FFI-SECRET-001"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ffi_records_foreign_call_trace_contract() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-c-interop-wasm-contract-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let output = Command::new(binary)
        .args([
            "ail-compile",
            &fixture("c_interop.ail"),
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--all-actions",
            "--agent",
            &fixture("ail_toolchain_agent.ail"),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let contract_report =
        fs::read_to_string(artifact_dir.join("wasm-contract-report.txt")).unwrap();
    assert!(
        contract_report.contains("host-import-trace zlib.compress2 ForeignCallCompress2"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-import-trace libc.qsort ForeignCallbackCompared"),
        "{contract_report}"
    );
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn ail_standard_library_option_type_parses_into_core() {
    let package = load_ail_package_dir(fixture("option_map.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);

    assert!(rendered_core.contains("node Type Option<T>"));
    assert!(rendered_core.contains("node Variant Option<T>.Some [label=Some]"));
    assert!(rendered_core.contains("node Variant Option<T>.None [label=None]"));
    assert!(rendered_core.contains("node Field Option<T>.Some.value : T"));
    assert!(rendered_core.contains("edge contains Type:Option<T> -> Variant:Option<T>.Some"));
    assert!(
        rendered_core
            .contains("edge has_field Variant:Option<T>.Some -> Field:Option<T>.Some.value")
    );
    assert!(rendered_core.contains("node Function Option.map"));
    assert!(
        rendered_core
            .contains("edge records_trace Function:Option.map -> Trace:OptionMapEvaluated")
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("Type: Option<T>."));
    assert!(rendered_spec.contains("Option has variants:"));
    assert!(rendered_spec.contains("- Some(value: T)"));
    assert!(rendered_spec.contains("- None"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_standard_library_option_map_executes_collection_transform_bytecode() {
    let package = load_ail_package_dir(fixture("option_map.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let bytecode = compile_ail_bytecode(&package, &document).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    let rendered = render_ail_bytecode(&bytecode);
    assert!(rendered.contains(r#""opcode":"OPTION_MAP""#), "{rendered}");

    let some_run = run_ail_bytecode_action(
        &bytecode,
        "Option.map",
        BTreeMap::from([
            ("option.variant".to_string(), "Some".to_string()),
            ("option.value".to_string(), "41".to_string()),
            ("mapper.result".to_string(), "42".to_string()),
        ]),
    )
    .unwrap();
    assert_eq!(some_run.status, "succeeded");
    assert_eq!(
        some_run
            .final_state
            .get("result.variant")
            .map(String::as_str),
        Some("Some")
    );
    assert_eq!(
        some_run.final_state.get("result.value").map(String::as_str),
        Some("42")
    );
    assert!(
        some_run
            .trace
            .contains(&"option map Some(value) with mapper -> Some(mapped value)".to_string()),
        "{:?}",
        some_run.trace
    );
    assert!(
        some_run
            .trace
            .contains(&"trace OptionMapEvaluated".to_string()),
        "{:?}",
        some_run.trace
    );

    let none_run = run_ail_bytecode_action(
        &bytecode,
        "Option.map",
        BTreeMap::from([("option.variant".to_string(), "None".to_string())]),
    )
    .unwrap();
    assert_eq!(none_run.status, "succeeded");
    assert_eq!(
        none_run
            .final_state
            .get("result.variant")
            .map(String::as_str),
        Some("None")
    );
    assert_eq!(none_run.final_state.get("result.value"), None);
    assert!(
        none_run
            .trace
            .contains(&"option map None -> None".to_string()),
        "{:?}",
        none_run.trace
    );
}

#[test]
fn cli_ail_stdlib_packages_have_checked_package_artifacts() {
    let required_packages = [
        (
            "ail_std_core.ail",
            "ail.std.core",
            [
                "node Function Identity.copy",
                "edge records_trace Function:Identity.copy -> Trace:IdentityCopied",
            ]
            .as_slice(),
        ),
        (
            "ail_std_collections.ail",
            "ail.std.collections",
            [
                "node Type Option<T>",
                "node Type Result<T,E>",
                "node Type List<T>",
                "node Type Map<K,V>",
                "node Type Set<T>",
                "node Function Option.map",
            ]
            .as_slice(),
        ),
        (
            "ail_std_effects.ail",
            "ail.std.effects",
            [
                "node Action ReadResource [label=Read resource",
                "node Action WriteResource [label=Write resource",
                "node Action SendNetworkMessage [label=Send network message",
                "edge records_trace Action:ReadResource -> Trace:ResourceRead",
                "edge records_trace Action:WriteResource -> Trace:ResourceWritten",
                "edge records_trace Action:SendNetworkMessage -> Trace:NetworkMessageSent",
            ]
            .as_slice(),
        ),
        (
            "ail_std_security.ail",
            "ail.std.security",
            [
                "node Field SecretEnvelope.payload : Secret<Text>",
                "node Action RevealSecret [label=Reveal secret",
                "edge protects_secret Action:RevealSecret -> Field:SecretEnvelope.payload",
                "edge records_trace Action:RevealSecret -> Trace:SecretRevealed",
            ]
            .as_slice(),
        ),
        (
            "ail_std_runtime.ail",
            "ail.std.runtime",
            [
                "node Failure RuntimeUnavailable",
                "node Action RunTask [label=Run task",
                "edge may_fail_with Action:RunTask -> Failure:RuntimeUnavailable",
                "edge records_trace Action:RunTask -> Trace:TaskRun",
            ]
            .as_slice(),
        ),
    ];

    for (fixture_name, package_name, expected_core_fragments) in required_packages {
        let package = load_ail_package_dir(fixture(fixture_name)).unwrap();
        assert_eq!(package.metadata.name, package_name);
        assert_eq!(package.metadata.conformance, "v0.2");
        assert!(
            package
                .metadata
                .target_support
                .contains_key("ail-core.schema.v0"),
            "{package_name} missing ail-core.schema.v0 target support"
        );
        assert!(
            package
                .metadata
                .features
                .iter()
                .any(|feature| feature == "stdlib"),
            "{package_name} missing stdlib feature"
        );

        let document = parse_ail_package_document(&package).unwrap();
        let core = elaborate_ail_core(&package, &document);
        assert_eq!(
            check_ail_core(&core),
            Vec::<String>::new(),
            "{package_name} did not check cleanly"
        );
        let rendered_core = render_ail_core(&core);
        assert!(
            rendered_core.contains(&format!("package: {package_name}")),
            "{rendered_core}"
        );
        assert!(
            rendered_core.contains("conformance: v0.2"),
            "{rendered_core}"
        );
        assert!(
            rendered_core.contains("target-support: ail-core.schema.v0=supported"),
            "{rendered_core}"
        );
        for expected in expected_core_fragments {
            assert!(
                rendered_core.contains(expected),
                "{package_name} missing checked core fragment {expected}\n{rendered_core}"
            );
        }

        let rendered_spec = render_ail_spec(&document);
        let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
        let reparsed_core = elaborate_ail_core(&package, &reparsed);
        assert_eq!(
            check_ail_core(&reparsed_core),
            Vec::<String>::new(),
            "{package_name} rendered AIL-Spec did not re-check cleanly"
        );

        let conformance = run_ail_conformance(&package).unwrap();
        assert!(
            conformance.success(),
            "{package_name} conformance failed: {conformance:?}"
        );
        assert!(
            !conformance.accepted.is_empty(),
            "{package_name} missing accepted conformance fixture"
        );
    }
}

#[test]
fn cli_ail_stdlib_import_records_dependency_report() {
    let package = load_ail_package_dir(fixture("ail_std_runtime.ail")).unwrap();
    let report = render_ail_package_dependency_report(&package).unwrap();
    assert!(
        report.contains("root-package ail.std.runtime 0.2.0"),
        "{report}"
    );
    assert!(
        report.contains("resolved-import Effects path=../ail_std_effects.ail requirement=compatible ^0.2 name=ail.std.effects version=0.2.0"),
        "{report}"
    );
    assert!(
        report.contains("resolved-import Core path=../ail_std_core.ail requirement=compatible ^0.2 name=ail.std.core version=0.2.0"),
        "{report}"
    );
    assert!(report.contains("source-path="), "{report}");
    assert!(report.contains("package-hash=ail-package:"), "{report}");
}

#[test]
fn cli_ail_std_rejects_invalid_generic_variant_payload() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("ail_std_collections.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rejected: invalid-generic-variant-payload.ail-spec.md AIL-TYPE-001"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_std_rejects_missing_capability_grant() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("ail_std_runtime.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rejected: missing-capability-grant.ail AIL-PACKAGE-001"),
        "{stdout}"
    );
}

#[test]
fn ail_ui_route_surface_parses_into_core() {
    let mut package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    package.metadata.profile = "UI".to_string();
    let spec = r#"Route: Ticket detail.

The route path is:

- /tickets/:ticket_id

The route reads:

- Ticket.id
- Ticket.status
- Ticket.public_updates

The route requires permission:

- requester may read ticket

The route records trace:

- RouteTicketDetailViewed
"#;

    let document = parse_ail_spec_text(spec).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);

    assert!(
        rendered_core
            .contains("node Route TicketDetail [label=Ticket detail,path=/tickets/:ticket_id]")
    );
    assert!(rendered_core.contains("node Value TicketDetail.Ticket.id"));
    assert!(rendered_core.contains("node Permission requester may read ticket"));
    assert!(
        rendered_core.contains("edge reads Route:TicketDetail -> Value:TicketDetail.Ticket.id")
    );
    assert!(
        rendered_core
            .contains("edge requires Route:TicketDetail -> Permission:requester may read ticket")
    );
    assert!(
        rendered_core
            .contains("edge records_trace Route:TicketDetail -> Trace:RouteTicketDetailViewed")
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("Route: Ticket detail."));
    assert!(rendered_spec.contains("- /tickets/:ticket_id"));
    assert!(rendered_spec.contains("- Ticket.public_updates"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn cli_ail_ui_form_calls_checked_action() {
    let package = load_ail_package_dir(fixture("ui_workflow.ail")).unwrap();
    let spec =
        fs::read_to_string(format!("{}/spec.ail-spec.md", fixture("ui_workflow.ail"))).unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains("node Form CreateTicketForm [label=Create ticket]"));
    assert!(rendered_core.contains("node Field CreateTicketForm.title : Text"));
    assert!(rendered_core.contains("node Rule title is not empty"));
    assert!(rendered_core.contains("node Accessibility title error is announced"));
    assert!(rendered_core.contains("edge calls Form:CreateTicketForm -> Action:CreateTicket"));
    assert!(
        rendered_core
            .contains("edge has_field Form:CreateTicketForm -> Field:CreateTicketForm.title")
    );
    assert!(
        rendered_core.contains("edge validates Form:CreateTicketForm -> Rule:title is not empty")
    );
    assert!(
        rendered_core
            .contains("edge records_trace Form:CreateTicketForm -> Trace:FormValidationFailed")
    );
}

#[test]
fn cli_ail_ui_dashboard_requires_matching_permission() {
    let package = load_ail_package_dir(fixture("ui_workflow.ail")).unwrap();
    let spec =
        fs::read_to_string(format!("{}/spec.ail-spec.md", fixture("ui_workflow.ail"))).unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(
        rendered_core
            .contains("node Dashboard SupportManagerDashboard [label=Support manager dashboard]")
    );
    assert!(rendered_core.contains("edge reads Dashboard:SupportManagerDashboard -> Value:SupportManagerDashboard.Ticket.status"));
    assert!(rendered_core.contains("edge requires Dashboard:SupportManagerDashboard -> Permission:Support manager may view overdue tickets"));
    assert!(
        rendered_core.contains(
            "edge records_trace Dashboard:SupportManagerDashboard -> Trace:DashboardViewed"
        )
    );
}

#[test]
fn cli_ail_ui_workflow_blocks_out_of_order_provider_call() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("ui_workflow.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "rejected: workflow-out-of-order-provider-call.ail-spec.md AIL-UI-WORKFLOW-001"
        ),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ui_rejects_unreachable_form_action() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("ui_workflow.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rejected: form-missing-action.ail-spec.md AIL-UI-FORM-001"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ui_rejects_dashboard_without_permission() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("ui_workflow.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rejected: dashboard-missing-permission.ail-spec.md AIL-UI-PERMISSION-001"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ui_rejects_inaccessible_error_text() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("ui_workflow.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("rejected: inaccessible-error-text.ail-spec.md AIL-UI-A11Y-001"),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ui_rejects_destructive_action_without_confirmation() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let output = Command::new(binary)
        .args(["ail-conformance", &fixture("ui_workflow.ail")])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "rejected: destructive-action-without-confirmation.ail-spec.md AIL-UI-CONFIRM-001"
        ),
        "{stdout}"
    );
}

#[test]
fn cli_ail_ui_accessibility_trace_records_field_error_announcement() {
    let package = load_ail_package_dir(fixture("ui_workflow.ail")).unwrap();
    let spec =
        fs::read_to_string(format!("{}/spec.ail-spec.md", fixture("ui_workflow.ail"))).unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains(
        "edge has_accessibility Form:CreateTicketForm -> Accessibility:title error is announced"
    ));
    assert!(
        rendered_core
            .contains("edge records_trace Form:CreateTicketForm -> Trace:FormValidationFailed")
    );
}

#[test]
fn cli_ail_flow_projects_ui_profile_blocks() {
    let package = load_ail_package_dir(fixture("ui_workflow.ail")).unwrap();
    let spec =
        fs::read_to_string(format!("{}/spec.ail-spec.md", fixture("ui_workflow.ail"))).unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let flow = render_ail_flow_view(&core);
    assert!(flow.contains(r#""routes":[{"#), "{flow}");
    assert!(flow.contains(r#""forms":[{"#), "{flow}");
    assert!(flow.contains(r#""dashboards":[{"#), "{flow}");
    assert!(flow.contains(r#""workflows":[{"#), "{flow}");
    assert!(flow.contains(r#""accessibility":[{"#), "{flow}");
}

#[test]
fn ail_system_profile_accepts_mutable_borrowed_resources() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/accepted/mutable-borrow-minimal.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains(
        "edge mutably_borrows_resource SystemComponent:DMAWriter -> Resource:DMAWriter.dma ring"
    ));
    assert!(
        rendered_core
            .contains("edge targets_resource Effect:write dma ring -> Resource:DMAWriter.dma ring")
    );

    let flow = render_ail_flow_view(&core);
    assert!(
        flow.contains(r#""mutablyBorrows":["DMAWriter.dma ring"]"#),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("The component mutably borrows:"));
    assert!(rendered_spec.contains("- dma ring"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_system_profile_accepts_moved_owned_resources() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/accepted/move-resource-minimal.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(
        rendered_core
            .contains("edge performs SystemComponent:PacketHandoff -> Effect:move rx buffer")
    );
    assert!(rendered_core.contains(
        "edge targets_resource Effect:move rx buffer -> Resource:PacketHandoff.rx buffer"
    ));

    let flow = render_ail_flow_view(&core);
    assert!(flow.contains(r#""effects":["move rx buffer"]"#), "{flow}");

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("- move rx buffer"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_system_profile_accepts_resource_layout_declarations() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/accepted/layout-minimal.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains("node Layout PacketLayout.packet header : repr(C), align 8"));
    assert!(rendered_core.contains(
        "edge uses_layout SystemComponent:PacketLayout -> Layout:PacketLayout.packet header"
    ));
    assert!(
        rendered_core
            .contains("edge layouts_resource Layout:PacketLayout.packet header -> Resource:PacketLayout.packet header")
    );

    let flow = render_ail_flow_view(&core);
    assert!(
        flow.contains(r#""layouts":["PacketLayout.packet header: repr(C), align 8"]"#),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("The component lays out:"));
    assert!(rendered_spec.contains("- packet header: repr(C), align 8"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_system_profile_accepts_resource_allocation_declarations() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/accepted/allocation-minimal.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains("node Allocation PacketAllocator.packet buffer : stack"));
    assert!(rendered_core.contains(
        "edge uses_allocation SystemComponent:PacketAllocator -> Allocation:PacketAllocator.packet buffer"
    ));
    assert!(
        rendered_core
            .contains("edge allocates_resource Allocation:PacketAllocator.packet buffer -> Resource:PacketAllocator.packet buffer")
    );

    let flow = render_ail_flow_view(&core);
    assert!(
        flow.contains(r#""allocations":["PacketAllocator.packet buffer: stack"]"#),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("The component allocates:"));
    assert!(rendered_spec.contains("- packet buffer: stack"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_system_profile_accepts_interrupt_context_declarations() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/accepted/interrupt-context-minimal.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains("node ExecutionContext TimerInterruptHandler.interrupt"));
    assert!(rendered_core.contains(
        "edge runs_in_context SystemComponent:TimerInterruptHandler -> ExecutionContext:TimerInterruptHandler.interrupt"
    ));

    let flow = render_ail_flow_view(&core);
    assert!(
        flow.contains(r#""contexts":["TimerInterruptHandler.interrupt"]"#),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("The component runs in context:"));
    assert!(rendered_spec.contains("- interrupt"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_system_profile_accepts_interrupt_priority_declarations() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/accepted/interrupt-priority-minimal.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(
        rendered_core.contains("node InterruptPriority TimerInterruptHandler.interrupt : high")
    );
    assert!(rendered_core.contains(
        "edge uses_interrupt_priority SystemComponent:TimerInterruptHandler -> InterruptPriority:TimerInterruptHandler.interrupt"
    ));
    assert!(
        rendered_core
            .contains("edge prioritizes_context InterruptPriority:TimerInterruptHandler.interrupt -> ExecutionContext:TimerInterruptHandler.interrupt")
    );

    let flow = render_ail_flow_view(&core);
    assert!(
        flow.contains(r#""priorities":["TimerInterruptHandler.interrupt: high"]"#),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("The component sets interrupt priority:"));
    assert!(rendered_spec.contains("- interrupt: high"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_system_profile_accepts_interrupt_mask_declarations() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/accepted/interrupt-mask-minimal.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains(
        "node InterruptMask TimerInterruptHandler.interrupt : mask lower priority interrupts"
    ));
    assert!(rendered_core.contains(
        "edge uses_interrupt_mask SystemComponent:TimerInterruptHandler -> InterruptMask:TimerInterruptHandler.interrupt"
    ));
    assert!(
        rendered_core.contains(
            "edge masks_context InterruptMask:TimerInterruptHandler.interrupt -> ExecutionContext:TimerInterruptHandler.interrupt"
        )
    );

    let flow = render_ail_flow_view(&core);
    assert!(
        flow.contains(
            r#""interruptMasks":["TimerInterruptHandler.interrupt: mask lower priority interrupts"]"#
        ),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("The component masks interrupt:"));
    assert!(rendered_spec.contains("- interrupt: mask lower priority interrupts"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_system_profile_accepts_scheduler_task_declarations() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/accepted/scheduler-task-minimal.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains("node SchedulerTask PacketScheduler.packet poller : process"));
    assert!(rendered_core.contains(
        "edge schedules_task SystemComponent:PacketScheduler -> SchedulerTask:PacketScheduler.packet poller"
    ));
    assert!(
        rendered_core
            .contains("edge task_runs_in_context SchedulerTask:PacketScheduler.packet poller -> ExecutionContext:PacketScheduler.process")
    );

    let flow = render_ail_flow_view(&core);
    assert!(
        flow.contains(r#""tasks":["PacketScheduler.packet poller: process"]"#),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("The component schedules task:"));
    assert!(rendered_spec.contains("- packet poller: process"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_system_profile_accepts_scheduler_task_priority_declarations() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/accepted/scheduler-task-priority-minimal.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(
        rendered_core
            .contains("node SchedulerTaskPriority PacketScheduler.packet poller : realtime")
    );
    assert!(rendered_core.contains(
        "edge uses_task_priority SystemComponent:PacketScheduler -> SchedulerTaskPriority:PacketScheduler.packet poller"
    ));
    assert!(
        rendered_core.contains(
            "edge prioritizes_task SchedulerTaskPriority:PacketScheduler.packet poller -> SchedulerTask:PacketScheduler.packet poller"
        )
    );

    let flow = render_ail_flow_view(&core);
    assert!(
        flow.contains(r#""taskPriorities":["PacketScheduler.packet poller: realtime"]"#),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("The component sets task priority:"));
    assert!(rendered_spec.contains("- packet poller: realtime"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_system_profile_accepts_scheduler_task_timing_declarations() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/accepted/scheduler-task-timing-minimal.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains(
        "node SchedulerTaskTiming PacketScheduler.packet poller : deadline 10 ms, budget 2 ms"
    ));
    assert!(rendered_core.contains(
        "edge uses_task_timing SystemComponent:PacketScheduler -> SchedulerTaskTiming:PacketScheduler.packet poller"
    ));
    assert!(
        rendered_core.contains(
            "edge times_task SchedulerTaskTiming:PacketScheduler.packet poller -> SchedulerTask:PacketScheduler.packet poller"
        )
    );

    let flow = render_ail_flow_view(&core);
    assert!(
        flow.contains(
            r#""taskTimings":["PacketScheduler.packet poller: deadline 10 ms, budget 2 ms"]"#
        ),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("The component sets task timing:"));
    assert!(rendered_spec.contains("- packet poller: deadline 10 ms, budget 2 ms"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_system_profile_accepts_lock_guard_declarations() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/accepted/lock-guard-minimal.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(
        rendered_core.contains("node LockGuard PacketScheduler.scheduler state : scheduler lock")
    );
    assert!(rendered_core.contains(
        "edge uses_lock_guard SystemComponent:PacketScheduler -> LockGuard:PacketScheduler.scheduler state"
    ));
    assert!(rendered_core.contains(
        "edge guards_resource LockGuard:PacketScheduler.scheduler state -> Resource:PacketScheduler.scheduler state"
    ));
    assert!(rendered_core.contains(
        "edge uses_lock_resource LockGuard:PacketScheduler.scheduler state -> Resource:PacketScheduler.scheduler lock"
    ));

    let flow = render_ail_flow_view(&core);
    assert!(
        flow.contains(r#""lockGuards":["PacketScheduler.scheduler state: scheduler lock"]"#),
        "{flow}"
    );

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("The component guards:"));
    assert!(rendered_spec.contains("- scheduler state with scheduler lock"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    assert_eq!(
        render_ail_core(&elaborate_ail_core(&package, &reparsed)),
        rendered_core
    );
}

#[test]
fn ail_package_imports_namespace_declarations_and_round_trip() {
    let package = load_ail_package_dir(fixture("support_composed.ail")).unwrap();

    assert_eq!(package.metadata.name, "support-composed");
    assert_eq!(package.metadata.imports.len(), 1);
    assert_eq!(package.metadata.imports[0].path, "../support_shared.ail");
    assert_eq!(package.metadata.imports[0].alias, "Shared");
    assert_eq!(package.imports.len(), 1);
    assert_eq!(package.imports[0].package.metadata.name, "support-shared");

    let document = parse_ail_package_document(&package).unwrap();
    assert!(document.things.contains_key("Shared.User"));
    assert!(document.things.contains_key("Ticket"));
    assert!(!document.things.contains_key("User"));
    assert_eq!(
        document.things["Ticket"].fields["customer"].type_name,
        "Shared.User"
    );
    assert!(document.failures.contains_key("Shared.PermissionDenied"));

    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(rendered_core.contains("node Thing Shared.User"));
    assert!(rendered_core.contains("node Field Shared.User.email : Text"));
    assert!(rendered_core.contains("node Failure Shared.PermissionDenied"));

    let rendered_spec = render_ail_spec(&document);
    assert!(rendered_spec.contains("A Shared.User has:"));
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();
    let reparsed_core = render_ail_core(&elaborate_ail_core(&package, &reparsed));
    assert_eq!(reparsed_core, rendered_core);
}

#[test]
fn ail_spec_parser_extracts_support_ticket_semantics() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();

    assert_eq!(document.application.name, "Support Tickets");
    assert_eq!(
        document.application.purpose,
        "customer support tickets, assignments, updates, internal notes, and overdue-ticket review"
    );
    assert_eq!(
        document.things["Ticket"].fields["status"].type_name,
        "State<New, Open, Assigned, Closed, Overdue>"
    );
    assert_eq!(
        document.things["Ticket"].fields["internal notes"].type_name,
        "Secret<List<Text>>"
    );
    assert!(document.things["Ticket"].fields["internal notes"].is_secret);
    assert!(document.actions.contains_key("CreateTicket"));
    assert!(document.actions.contains_key("AssignTicket"));
    assert!(document.actions.contains_key("CloseTicket"));
    assert!(
        document.actions["CloseTicket"]
            .guarantees
            .contains(&"closed tickets do not appear in the open ticket queue".to_string())
    );
    assert!(
        document.actions["CloseTicket"]
            .secret_protections
            .contains(&"internal notes to the customer".to_string())
    );
    assert!(document.failures.contains_key("NotFound"));
    assert!(document.failures.contains_key("PermissionDenied"));
    assert_eq!(
        document.failures["PermissionDenied"].condition,
        "a user tries to see internal notes without support staff permission"
    );
}

#[test]
fn ail_spec_parser_normalizes_common_llm_type_aliases() {
    let document = parse_ail_spec_text(
        r#"
The application Alias Tickets manages tickets drafted by a model.

A Ticket has:

- id: String
- internal notes: Secret List<String>
"#,
    )
    .unwrap();

    assert_eq!(document.things["Ticket"].fields["id"].type_name, "Text");
    assert_eq!(
        document.things["Ticket"].fields["internal notes"].type_name,
        "Secret<List<Text>>"
    );
    assert!(document.things["Ticket"].fields["internal notes"].is_secret);
}

#[test]
fn ail_core_elaboration_serializes_support_ticket_graph() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let diagnostics = check_ail_core(&core);

    assert_eq!(diagnostics, Vec::<String>::new());
    assert!(
        core.graph
            .find_node("Application", "Support Tickets")
            .is_some()
    );
    assert!(core.graph.find_node("Thing", "Ticket").is_some());
    assert!(core.graph.find_node("User", "Customer").is_some());
    assert!(core.graph.find_node("Action", "CloseTicket").is_some());
    assert!(
        core.graph
            .find_node("Failure", "PermissionDenied")
            .is_some()
    );
    assert!(core.graph.find_node("Trace", "TicketClosed").is_some());
    assert!(
        core.graph
            .find_node("Provenance", "action:CloseTicket")
            .is_some()
    );

    let rendered = render_ail_core(&core);
    assert!(rendered.contains("package: support-ticket"));
    assert!(rendered.contains("node User Customer"));
    assert!(rendered.contains("node Action CloseTicket"));
    assert!(rendered.contains("node Field Ticket.internal notes : Secret<List<Text>>"));
    assert!(rendered.contains("node Failure PermissionDenied"));
    assert!(
        rendered
            .contains("edge has_provenance Action:CloseTicket -> Provenance:action:CloseTicket")
    );
}

#[test]
fn ail_core_text_preserves_package_entry_features_and_imports() {
    let package = load_ail_package_dir(fixture("support_composed.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    let rendered = render_ail_core(&core);
    assert!(rendered.contains("entry: spec.ail-spec.md"));
    assert!(rendered.contains("features: imports, things, actions, failures, guarantees, traces"));
    assert!(rendered.contains("imports: ../support_shared.ail as Shared"));

    let reparsed = parse_ail_core_text(&rendered).unwrap();
    assert_eq!(reparsed.package.entry, package.metadata.entry);
    assert_eq!(reparsed.package.features, package.metadata.features);
    assert_eq!(reparsed.package.imports, package.metadata.imports);
}

#[test]
fn ail_package_loader_accepts_versioned_imports_and_rejects_mismatches() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-versioned-imports-{}-{unique_suffix}",
        std::process::id()
    ));
    let shared = root.join("shared");
    let app = root.join("app");
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&app).unwrap();
    fs::write(
        shared.join("ail-package.md"),
        r#"name: shared-lib
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
"#,
    )
    .unwrap();
    fs::write(shared.join("spec.ail-spec.md"), "Application: Shared.\n").unwrap();
    fs::write(app.join("spec.ail-spec.md"), "Application: App.\n").unwrap();
    fs::write(
        app.join("ail-package.md"),
        r#"name: app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports
imports: ../shared@0.1.0 as Shared
conformance: first-slice
"#,
    )
    .unwrap();

    let package = load_ail_package_dir(&app).unwrap();
    assert_eq!(package.metadata.imports[0].path, "../shared");
    assert_eq!(package.metadata.imports[0].alias, "Shared");
    assert_eq!(package.imports[0].package.metadata.version, "0.1.0");

    fs::write(
        app.join("ail-package.md"),
        r#"name: app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports
imports: ../shared@0.2.0 as Shared
conformance: first-slice
"#,
    )
    .unwrap();
    let error = load_ail_package_dir(&app).unwrap_err();
    assert!(
        error.contains("requires version 0.2.0") && error.contains("shared-lib"),
        "{error}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_ail_package_resolves_compatible_local_import_ranges() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-compatible-import-range-{}-{unique_suffix}",
        std::process::id()
    ));
    let shared = root.join("shared");
    let app = root.join("app");
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&app).unwrap();
    fs::write(
        shared.join("ail-package.md"),
        r#"name: shared-lib
version: 0.1.3
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
"#,
    )
    .unwrap();
    fs::write(shared.join("spec.ail-spec.md"), "Application: Shared.\n").unwrap();
    fs::write(app.join("spec.ail-spec.md"), "Application: App.\n").unwrap();
    fs::write(
        app.join("ail-package.md"),
        r#"name: app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports
imports: ../shared compatible ^0.1 as Shared
conformance: first-slice
"#,
    )
    .unwrap();

    let package = load_ail_package_dir(&app).unwrap();
    assert_eq!(package.metadata.imports[0].path, "../shared");
    assert_eq!(
        package.metadata.imports[0].version.as_deref(),
        Some("compatible ^0.1")
    );
    assert_eq!(package.imports[0].package.metadata.version, "0.1.3");

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_ail_package_rejects_unbounded_major_import_ranges() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-unbounded-import-range-{}-{unique_suffix}",
        std::process::id()
    ));
    let shared = root.join("shared");
    let app = root.join("app");
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&app).unwrap();
    fs::write(
        shared.join("ail-package.md"),
        r#"name: shared-lib
version: 1.2.3
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
"#,
    )
    .unwrap();
    fs::write(shared.join("spec.ail-spec.md"), "Application: Shared.\n").unwrap();
    fs::write(app.join("spec.ail-spec.md"), "Application: App.\n").unwrap();
    fs::write(
        app.join("ail-package.md"),
        r#"name: app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports
imports: ../shared compatible * as Shared
conformance: first-slice
"#,
    )
    .unwrap();

    let error = load_ail_package_dir(&app).unwrap_err();
    assert!(
        error.contains("unbounded major") && error.contains("../shared"),
        "{error}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_ail_package_resolves_registry_import_identity_from_index() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-registry-import-{}-{unique_suffix}",
        std::process::id()
    ));
    let registry = root.join("registry");
    let shared = root.join("packages/shared-lib-0.1.3");
    let app = root.join("app");
    fs::create_dir_all(&registry).unwrap();
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&app).unwrap();
    fs::write(
        registry.join("ail-registry.md"),
        format!(
            r#"package: shared-lib
version: 0.1.3
identity: registry.local/shared-lib@0.1.3
path: {}
"#,
            shared.display()
        ),
    )
    .unwrap();
    fs::write(
        shared.join("ail-package.md"),
        r#"name: shared-lib
version: 0.1.3
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
"#,
    )
    .unwrap();
    fs::write(shared.join("spec.ail-spec.md"), "Application: Shared.\n").unwrap();
    fs::write(app.join("spec.ail-spec.md"), "Application: App.\n").unwrap();
    fs::write(
        app.join("ail-package.md"),
        r#"name: app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports
registry: ../registry/ail-registry.md
imports: shared-lib compatible ^0.1 as Shared
conformance: first-slice
"#,
    )
    .unwrap();

    let package = load_ail_package_dir(&app).unwrap();
    assert_eq!(package.metadata.imports[0].path, "shared-lib");
    assert_eq!(
        package.metadata.imports[0].version.as_deref(),
        Some("compatible ^0.1")
    );
    assert_eq!(
        package.metadata.imports[0].resolved_package.as_deref(),
        Some("shared-lib")
    );
    assert_eq!(
        package.imports[0].spec.registry_identity.as_deref(),
        Some("registry.local/shared-lib@0.1.3")
    );

    let report = render_ail_package_dependency_report(&package).unwrap();
    assert!(
        report.contains("registry-identity=registry.local/shared-lib@0.1.3"),
        "{report}"
    );
    assert!(
        report.contains("resolved-import Shared path=shared-lib requirement=compatible ^0.1 name=shared-lib version=0.1.3"),
        "{report}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_ail_package_rejects_unknown_registry_import() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-registry-missing-import-{}-{unique_suffix}",
        std::process::id()
    ));
    let registry = root.join("registry");
    let app = root.join("app");
    fs::create_dir_all(&registry).unwrap();
    fs::create_dir_all(&app).unwrap();
    fs::write(
        registry.join("ail-registry.md"),
        r#"package: other-lib
version: 0.1.0
identity: registry.local/other-lib@0.1.0
path: ../other
"#,
    )
    .unwrap();
    fs::write(app.join("spec.ail-spec.md"), "Application: App.\n").unwrap();
    fs::write(
        app.join("ail-package.md"),
        r#"name: app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports
registry: ../registry/ail-registry.md
imports: shared-lib@0.1.0 as Shared
conformance: first-slice
"#,
    )
    .unwrap();

    let error = load_ail_package_dir(&app).unwrap_err();
    assert!(
        error.contains("registry") && error.contains("shared-lib"),
        "{error}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ail_package_dependency_report_records_resolved_imports_and_grants() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-package-dependency-report-{}-{unique_suffix}",
        std::process::id()
    ));
    let shared = root.join("shared");
    let app = root.join("app");
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&app).unwrap();
    fs::write(
        shared.join("ail-package.md"),
        r#"name: shared-lib
version: 0.1.3
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
capability-grants:
  - package: shared-lib
    capability: read shared ticket data
    effects: [read, network]
    approvals: [operator approval]
"#,
    )
    .unwrap();
    fs::write(shared.join("spec.ail-spec.md"), "Application: Shared.\n").unwrap();
    fs::write(app.join("spec.ail-spec.md"), "Application: App.\n").unwrap();
    fs::write(
        app.join("ail-package.md"),
        r#"name: app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports
imports: ../shared compatible ^0.1 as Shared
conformance: first-slice
"#,
    )
    .unwrap();

    let package = load_ail_package_dir(&app).unwrap();
    let report = render_ail_package_dependency_report(&package).unwrap();
    assert!(
        report.contains("AIL-Package-Dependency-Report:"),
        "{report}"
    );
    assert!(report.contains("root-package app 0.1.0"), "{report}");
    assert!(
        report.contains("resolved-import Shared path=../shared requirement=compatible ^0.1 name=shared-lib version=0.1.3"),
        "{report}"
    );
    assert!(report.contains("source-path="), "{report}");
    assert!(report.contains("package-hash=ail-package:"), "{report}");
    assert!(
        report.contains("capability-grant package=shared-lib capability=read shared ticket data effects=read|network approvals=operator approval"),
        "{report}"
    );
    assert!(
        report.contains("imported-effect-classes Shared network|read"),
        "{report}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_ail_package_rejects_imported_effect_without_capability_grant() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-imported-effect-without-grant-{}-{unique_suffix}",
        std::process::id()
    ));
    let shared = root.join("shared");
    let app = root.join("app");
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&app).unwrap();
    fs::write(
        shared.join("ail-package.md"),
        r#"name: shared-net
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions
conformance: first-slice
"#,
    )
    .unwrap();
    fs::write(
        shared.join("spec.ail-spec.md"),
        r#"The application Shared Net manages shared network operations.

Action: Send packet.

When a service sends a packet:

- the system changes network
- the system records a trace event named PacketSent
"#,
    )
    .unwrap();
    fs::write(
        app.join("ail-package.md"),
        r#"name: app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports
imports: ../shared as Shared
conformance: first-slice
"#,
    )
    .unwrap();
    fs::write(
        app.join("spec.ail-spec.md"),
        "The application App manages imported behavior.\n",
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            app.to_str().unwrap(),
            "--spec-file",
            app.join("spec.ail-spec.md").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("AIL-PACKAGE-001")
            && stdout.contains("Shared.SendPacket")
            && stdout.contains("network")
            && stdout.contains("capability grant"),
        "{stdout}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_ail_package_accepts_imported_effect_grant_by_resolved_package_name() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-imported-effect-with-resolved-grant-{}-{unique_suffix}",
        std::process::id()
    ));
    let shared = root.join("shared");
    let app = root.join("app");
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&app).unwrap();
    fs::write(
        shared.join("ail-package.md"),
        r#"name: shared-net
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions
conformance: first-slice
"#,
    )
    .unwrap();
    fs::write(
        shared.join("spec.ail-spec.md"),
        r#"The application Shared Net manages shared network operations.

Action: Send packet.

When a service sends a packet:

- the system changes network
- the system records a trace event named PacketSent
"#,
    )
    .unwrap();
    fs::write(
        app.join("ail-package.md"),
        r#"name: app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports
imports: ../shared as Shared
conformance: first-slice
capability-grants:
  - package: shared-net
    capability: send shared network packets
    effects: [network]
    approvals: [network owner approval]
"#,
    )
    .unwrap();
    fs::write(
        app.join("spec.ail-spec.md"),
        "The application App manages imported behavior.\n",
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            app.to_str().unwrap(),
            "--spec-file",
            app.join("spec.ail-spec.md").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert!(bytecode.actions.contains_key("Shared.SendPacket"));
    assert_eq!(
        bytecode.capability_grants[0].package,
        "shared-net".to_string()
    );
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ail_package_loader_rejects_duplicate_import_aliases() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-duplicate-import-aliases-{}-{unique_suffix}",
        std::process::id()
    ));
    let shared_a = root.join("shared-a");
    let shared_b = root.join("shared-b");
    let app = root.join("app");
    fs::create_dir_all(&shared_a).unwrap();
    fs::create_dir_all(&shared_b).unwrap();
    fs::create_dir_all(&app).unwrap();
    for (package_root, package_name) in [(&shared_a, "shared-a"), (&shared_b, "shared-b")] {
        fs::write(
            package_root.join("ail-package.md"),
            format!(
                "name: {package_name}\nversion: 0.1.0\nprofile: Application\nentry: spec.ail-spec.md\nfeatures: things\nconformance: first-slice\n"
            ),
        )
        .unwrap();
        fs::write(
            package_root.join("spec.ail-spec.md"),
            "Application: Shared.\n",
        )
        .unwrap();
    }
    fs::write(app.join("spec.ail-spec.md"), "Application: App.\n").unwrap();
    fs::write(
        app.join("ail-package.md"),
        r#"name: app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports
imports: ../shared-a@0.1.0 as Shared, ../shared-b@0.1.0 as Shared
conformance: first-slice
"#,
    )
    .unwrap();

    let error = load_ail_package_dir(&app).unwrap_err();
    assert!(error.contains("duplicate import alias Shared"), "{error}");

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ail_core_text_preserves_manifest_prompt_pack() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-prompt-pack-manifest-{}-{unique_suffix}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: prompt-pack-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
prompt-pack: ail.prompts@0.1
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        "The application Prompt Pack App manages prompt pack preservation.\n",
    )
    .unwrap();

    let package = load_ail_package_dir(&root).unwrap();
    assert_eq!(
        package.metadata.prompt_pack.as_deref(),
        Some("ail.prompts@0.1")
    );
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let rendered = render_ail_core(&core);
    assert!(
        rendered.contains("prompt-pack: ail.prompts@0.1"),
        "{rendered}"
    );

    let reparsed = parse_ail_core_text(&rendered).unwrap();
    assert_eq!(
        reparsed.package.prompt_pack.as_deref(),
        Some("ail.prompts@0.1")
    );
    let rerendered = render_ail_core(&reparsed);
    assert!(
        rerendered.contains("prompt-pack: ail.prompts@0.1"),
        "{rerendered}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ail_core_text_preserves_manifest_target_support() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-target-support-manifest-{}-{unique_suffix}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: target-support-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
target-support:
  x86_64-unknown-linux-syscall-elf: supported
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        "The application Target Support App manages target metadata.\n",
    )
    .unwrap();

    let package = load_ail_package_dir(&root).unwrap();
    assert_eq!(
        package
            .metadata
            .target_support
            .get("x86_64-unknown-linux-syscall-elf")
            .map(String::as_str),
        Some("supported")
    );
    assert_eq!(
        package
            .metadata
            .target_support
            .get("wasm32-unknown-sandbox-wasm")
            .map(String::as_str),
        Some("supported-with-host-imports")
    );
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let rendered = render_ail_core(&core);
    assert!(
        rendered.contains(
            "target-support: wasm32-unknown-sandbox-wasm=supported-with-host-imports, x86_64-unknown-linux-syscall-elf=supported"
        ),
        "{rendered}"
    );

    let reparsed = parse_ail_core_text(&rendered).unwrap();
    assert_eq!(
        reparsed.package.target_support,
        package.metadata.target_support
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ail_core_reports_unknown_target_support_status_metadata() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-unknown-target-support-status-{}-{unique_suffix}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: unknown-target-support-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
target-support:
  x86_64-unknown-linux-syscall-elf: experimental-preview
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        "The application Unknown Target Support App manages target validation.\n",
    )
    .unwrap();

    let package = load_ail_package_dir(&root).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let diagnostics = check_ail_core_diagnostics(&core);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "AIL-BACKEND-002"
                && diagnostic
                    .message
                    .contains("x86_64-unknown-linux-syscall-elf")
                && diagnostic.message.contains("experimental-preview")
        }),
        "{diagnostics:?}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ail_core_text_preserves_manifest_schema_version_and_safety_level() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-schema-safety-manifest-{}-{unique_suffix}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: schema-safety-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
schema-version: ail-core.schema.v0
safety-level: standard
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        "The application Schema Safety App manages schema and safety metadata.\n",
    )
    .unwrap();

    let package = load_ail_package_dir(&root).unwrap();
    assert_eq!(
        package.metadata.schema_version.as_deref(),
        Some("ail-core.schema.v0")
    );
    assert_eq!(package.metadata.safety_level.as_deref(), Some("standard"));
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let rendered = render_ail_core(&core);
    assert!(
        rendered.contains("schema-version: ail-core.schema.v0"),
        "{rendered}"
    );
    assert!(rendered.contains("safety-level: standard"), "{rendered}");

    let reparsed = parse_ail_core_text(&rendered).unwrap();
    assert_eq!(
        reparsed.package.schema_version,
        package.metadata.schema_version
    );
    assert_eq!(reparsed.package.safety_level, package.metadata.safety_level);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ail_core_reports_unknown_schema_version_metadata() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-unknown-schema-version-{}-{unique_suffix}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: unknown-schema-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
schema-version: ail-core.schema.v99
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        "The application Unknown Schema App manages schema validation.\n",
    )
    .unwrap();

    let package = load_ail_package_dir(&root).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let diagnostics = check_ail_core_diagnostics(&core);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "AIL-SCHEMA-003"
                && diagnostic.message.contains("ail-core.schema.v99")
        }),
        "{diagnostics:?}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ail_core_reports_unknown_safety_level_metadata() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-unknown-safety-level-{}-{unique_suffix}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: unknown-safety-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
safety-level: casual
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        "The application Unknown Safety App manages safety validation.\n",
    )
    .unwrap();

    let package = load_ail_package_dir(&root).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let diagnostics = check_ail_core_diagnostics(&core);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "AIL-SAFETY-001" && diagnostic.message.contains("casual")
        }),
        "{diagnostics:?}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ail_core_text_preserves_manifest_capability_grants() {
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-capability-grants-manifest-{}-{unique_suffix}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: capability-grant-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports
conformance: first-slice
capability-grants:
  - package: payments.stripe
    capability: call external payment provider
    effects: [network, money]
    approvals: [manager approval over USD 500]
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        "The application Capability Grant App manages capability metadata.\n",
    )
    .unwrap();

    let package = load_ail_package_dir(&root).unwrap();
    assert_eq!(package.metadata.capability_grants.len(), 1);
    let grant = &package.metadata.capability_grants[0];
    assert_eq!(grant.package, "payments.stripe");
    assert_eq!(grant.capability, "call external payment provider");
    assert_eq!(
        grant.effects,
        vec!["network".to_string(), "money".to_string()]
    );
    assert_eq!(
        grant.approvals,
        vec!["manager approval over USD 500".to_string()]
    );

    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let rendered = render_ail_core(&core);
    assert!(
        rendered.contains(
            "capability-grants: package=payments.stripe;capability=call external payment provider;effects=network|money;approvals=manager approval over USD 500"
        ),
        "{rendered}"
    );

    let reparsed = parse_ail_core_text(&rendered).unwrap();
    assert_eq!(
        reparsed.package.capability_grants,
        package.metadata.capability_grants
    );
    let bytecode = compile_ail_core_bytecode(&reparsed).unwrap();
    let rendered_bytecode = render_ail_bytecode(&bytecode);
    assert!(
        rendered_bytecode.contains(
            r#""capability_grants":[{"package":"payments.stripe","capability":"call external payment provider","effects":["network","money"],"approvals":["manager approval over USD 500"]}]"#
        ),
        "{rendered_bytecode}"
    );
    let parsed_bytecode = parse_ail_bytecode(&rendered_bytecode).unwrap();
    assert_eq!(render_ail_bytecode(&parsed_bytecode), rendered_bytecode);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ail_core_elaboration_preserves_provenance_for_behavior_bullets() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);

    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    assert!(core.graph.has_edge(
        "has_provenance",
        "the ticket to exist",
        "action:CloseTicket.requirement:the ticket to exist"
    ));
    assert!(core.graph.has_edge(
        "has_provenance",
        "a public update",
        "action:CloseTicket.write:a public update"
    ));
    assert!(core.graph.has_edge(
        "has_provenance",
        "closed tickets do not appear in the open ticket queue",
        "action:CloseTicket.guarantee:closed tickets do not appear in the open ticket queue"
    ));
    assert!(core.graph.has_edge(
        "has_provenance",
        "TicketClosed",
        "action:CloseTicket.trace:TicketClosed"
    ));
    assert!(core.graph.has_edge(
        "has_provenance",
        "the caller sees \"Ticket not found\"",
        "failure:NotFound.handling:the caller sees \"Ticket not found\""
    ));
    assert!(core.graph.has_edge(
        "has_provenance",
        "TicketNotFound",
        "failure:NotFound.trace:TicketNotFound"
    ));
}

#[test]
fn ail_core_reports_missing_provenance_for_semantic_nodes() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let mut core = elaborate_ail_core(&package, &document);
    let rule_id = core
        .graph
        .find_node("Rule", "the ticket to exist")
        .unwrap()
        .id
        .clone();

    core.graph
        .edges
        .retain(|edge| !(edge.kind == "has_provenance" && edge.source == rule_id));

    assert!(
        check_ail_core(&core)
            .contains(&"AIL011 rule 'the ticket to exist' is missing provenance".to_string())
    );
    let detailed = detailed_ail_diagnostic(
        &core,
        "AIL011",
        "rule 'the ticket to exist' is missing provenance",
    );
    assert!(detailed.contains("graph=node:Rule:"), "{detailed}");
    assert!(
        detailed.contains("repair=Attach provenance to rule 'the ticket to exist'."),
        "{detailed}"
    );
}

#[test]
fn ail_core_checker_rejects_unknown_schema_items() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let mut core = elaborate_ail_core(&package, &document);
    let close_ticket = core
        .graph
        .find_node("Action", "CloseTicket")
        .unwrap()
        .clone();
    let provenance =
        core.graph
            .add_node("Provenance", "test:unknown-widget", None, BTreeMap::new());
    let widget = core
        .graph
        .add_node("Widget", "Forgotten review state", None, BTreeMap::new());
    core.graph
        .add_edge("has_provenance", &widget, &provenance, BTreeMap::new());
    core.graph
        .add_edge("forgets_semantics", &close_ticket, &widget, BTreeMap::new());

    let diagnostics = check_ail_core(&core);

    assert!(
        diagnostics.contains(&"AIL-SCHEMA-001 unknown AIL-Core node kind 'Widget'".to_string()),
        "{diagnostics:?}"
    );
    assert!(
        diagnostics
            .contains(&"AIL-SCHEMA-002 unknown AIL-Core edge kind 'forgets_semantics'".to_string()),
        "{diagnostics:?}"
    );
    let detailed_node = detailed_ail_diagnostic(
        &core,
        "AIL-SCHEMA-001",
        "unknown AIL-Core node kind 'Widget'",
    );
    assert!(
        detailed_node.contains("graph=node:Widget:"),
        "{detailed_node}"
    );
    assert!(
        detailed_node.contains("repair=Use a node kind declared in ail-core.schema.v0"),
        "{detailed_node}"
    );
}

#[test]
fn ail_core_reports_unattached_guarantees() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let mut core = elaborate_ail_core(&package, &document);
    let guarantee_id = core
        .graph
        .find_node(
            "Guarantee",
            "closed tickets do not appear in the open ticket queue",
        )
        .unwrap()
        .id
        .clone();

    core.graph
        .edges
        .retain(|edge| !(edge.kind == "guarantees" && edge.target == guarantee_id));

    assert!(check_ail_core(&core).contains(
        &"AIL012 guarantee 'closed tickets do not appear in the open ticket queue' is not attached to an action or tool"
            .to_string()
    ));
    let detailed = detailed_ail_diagnostic(
        &core,
        "AIL012",
        "guarantee 'closed tickets do not appear in the open ticket queue' is not attached to an action or tool",
    );
    assert!(
        detailed.contains("source=action:CloseTicket.guarantee:closed tickets do not appear in the open ticket queue"),
        "{detailed}"
    );
    assert!(detailed.contains("graph=node:Guarantee:"), "{detailed}");
    assert!(
        detailed.contains(
            "repair=Attach guarantee 'closed tickets do not appear in the open ticket queue' to an action or tool."
        ),
        "{detailed}"
    );
}

#[test]
fn ail_core_reports_unattached_traces() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let mut core = elaborate_ail_core(&package, &document);
    let trace = core
        .graph
        .add_node("Trace", "OrphanTrace", None, BTreeMap::new());
    let provenance = core
        .graph
        .add_node("Provenance", "trace:OrphanTrace", None, BTreeMap::new());
    core.graph
        .add_edge("has_provenance", &trace, &provenance, BTreeMap::new());

    assert!(check_ail_core(&core).contains(
        &"AIL013 trace 'OrphanTrace' is not recorded by an action or failure".to_string()
    ));
    let detailed = detailed_ail_diagnostic(
        &core,
        "AIL013",
        "trace 'OrphanTrace' is not recorded by an action or failure",
    );
    assert!(detailed.contains("source=trace:OrphanTrace"), "{detailed}");
    assert!(detailed.contains("graph=node:Trace:"), "{detailed}");
    assert!(
        detailed.contains("repair=Record trace 'OrphanTrace' from an action or failure."),
        "{detailed}"
    );
}

#[test]
fn ail_core_reports_unattached_rules() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let mut core = elaborate_ail_core(&package, &document);
    let rule = core
        .graph
        .add_node("Rule", "orphan rule", None, BTreeMap::new());
    let provenance = core
        .graph
        .add_node("Provenance", "rule:orphan", None, BTreeMap::new());
    core.graph
        .add_edge("has_provenance", &rule, &provenance, BTreeMap::new());

    assert!(
        check_ail_core(&core)
            .contains(&"AIL014 rule 'orphan rule' is not required by an action".to_string())
    );
    let detailed = detailed_ail_diagnostic(
        &core,
        "AIL014",
        "rule 'orphan rule' is not required by an action",
    );
    assert!(detailed.contains("source=rule:orphan"), "{detailed}");
    assert!(detailed.contains("graph=node:Rule:"), "{detailed}");
    assert!(
        detailed.contains("repair=Attach rule 'orphan rule' to an action requirement."),
        "{detailed}"
    );
}

#[test]
fn ail_core_reports_unattached_effects() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let mut core = elaborate_ail_core(&package, &document);
    let effect = core
        .graph
        .add_node("Effect", "orphan effect", None, BTreeMap::new());
    let provenance = core
        .graph
        .add_node("Provenance", "effect:orphan", None, BTreeMap::new());
    core.graph
        .add_edge("has_provenance", &effect, &provenance, BTreeMap::new());

    assert!(check_ail_core(&core).contains(
        &"AIL015 effect 'orphan effect' is not attached to an action or failure".to_string()
    ));
    let detailed = detailed_ail_diagnostic(
        &core,
        "AIL015",
        "effect 'orphan effect' is not attached to an action or failure",
    );
    assert!(detailed.contains("source=effect:orphan"), "{detailed}");
    assert!(detailed.contains("graph=node:Effect:"), "{detailed}");
    assert!(
        detailed.contains("repair=Attach effect 'orphan effect' to an action or failure."),
        "{detailed}"
    );
}

#[test]
fn ail_core_reports_unattached_secrets() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let mut core = elaborate_ail_core(&package, &document);
    let secret = core
        .graph
        .add_node("Secret", "orphan secret", None, BTreeMap::new());
    let provenance = core
        .graph
        .add_node("Provenance", "secret:orphan", None, BTreeMap::new());
    core.graph
        .add_edge("has_provenance", &secret, &provenance, BTreeMap::new());

    assert!(check_ail_core(&core).contains(
        &"AIL016 secret 'orphan secret' is not attached to a field or action".to_string()
    ));
    let detailed = detailed_ail_diagnostic(
        &core,
        "AIL016",
        "secret 'orphan secret' is not attached to a field or action",
    );
    assert!(detailed.contains("source=secret:orphan"), "{detailed}");
    assert!(detailed.contains("graph=node:Secret:"), "{detailed}");
    assert!(
        detailed
            .contains("repair=Attach secret 'orphan secret' to a field or action protection edge."),
        "{detailed}"
    );
}

#[test]
fn ail_flow_projection_renders_no_code_view_from_core() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let flow = render_ail_flow_view(&core);
    let expected_core_hash = ail_core_hash(&core);

    assert!(flow.contains(r#""kind":"AIL-Flow""#));
    assert!(flow.contains(r#""application":"Support Tickets""#));
    assert!(
        flow.contains(&format!(r#""coreHash":"{expected_core_hash}""#)),
        "{flow}"
    );
    assert!(flow.contains(r#""name":"Ticket","coreLabel":"Thing:Ticket""#));
    assert!(flow.contains(
        r#""name":"internal notes","coreLabel":"Field:Ticket.internal notes","type":"Secret<List<Text>>","secret":true"#
    ));
    assert!(flow.contains(r#""name":"CloseTicket","coreLabel":"Action:CloseTicket""#));
    assert!(flow.contains(r#""writes":["Ticket.status","a public update"]"#));
    assert!(flow.contains(r#""traces":["TicketClosed"]"#));
    assert!(flow.contains(r#""edgeRefs":["#), "{flow}");
    assert!(
        flow.contains(
            r#"{"kind":"records_trace","source":"Action:CloseTicket","target":"Trace:TicketClosed","targetName":"TicketClosed","attributes":{}}"#
        ),
        "{flow}"
    );
    assert!(
        flow.contains(
            r#"{"kind":"writes","source":"Action:CloseTicket","target":"Field:Ticket.status","targetName":"Ticket.status","attributes":{"provenance":"action:CloseTicket.write:the ticket status to Closed"}}"#
        ),
        "{flow}"
    );
    assert!(flow.contains(
        r#""views":["a customer-visible ticket history that includes public updates and never includes internal notes","an Overdue tickets view for support managers","an open ticket queue for support agents"]"#
    ));
}

#[test]
fn ail_core_rendering_and_hash_are_stable_across_edge_order() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let mut reordered = core.clone();
    reordered.graph.edges.reverse();

    assert_eq!(render_ail_core(&reordered), render_ail_core(&core));
    assert_eq!(ail_core_hash(&reordered), ail_core_hash(&core));
}

#[test]
fn cli_ail_flow_edit_renames_action_card_and_round_trips_to_spec() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let core_output = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    let core = parse_ail_core_text(&core_text).unwrap();
    let core_hash = ail_core_hash(&core);
    let flow = render_ail_flow_view(&core);
    assert!(
        flow.contains(r#""coreLabel":"Action:CloseTicket""#),
        "{flow}"
    );

    let core_path = std::env::temp_dir().join(format!(
        "ail-flow-edit-action-card-{}.ail-core.txt",
        std::process::id()
    ));
    let edit_path = std::env::temp_dir().join(format!(
        "ail-flow-edit-action-card-{}.ail-flow.edit.json",
        std::process::id()
    ));
    let patched_core_path = std::env::temp_dir().join(format!(
        "ail-flow-edit-action-card-patched-{}.ail-core.txt",
        std::process::id()
    ));
    fs::write(&core_path, core_text).unwrap();
    fs::write(
        &edit_path,
        format!(
            r#"{{
  "schema": "ail-flow.edit.v0",
  "package": "support-ticket",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "edits": [
    {{
      "op": "ActionCard.rename",
      "target": "Action:CloseTicket",
      "label": "Resolve ticket",
      "provenance": ["flow:ActionCard:CloseTicket.label"]
    }}
  ]
}}"#
        ),
    )
    .unwrap();

    let patched_core = Command::new(binary)
        .args([
            "ail-flow-edit",
            "--core-file",
            core_path.to_str().unwrap(),
            edit_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        patched_core.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&patched_core.stdout),
        String::from_utf8_lossy(&patched_core.stderr)
    );
    let patched_core_text = String::from_utf8(patched_core.stdout).unwrap();
    let patched_core_artifact = parse_ail_core_text(&patched_core_text).unwrap();
    assert_eq!(check_ail_core(&patched_core_artifact), Vec::<String>::new());
    fs::write(&patched_core_path, patched_core_text).unwrap();

    let spec = Command::new(binary)
        .args([
            "ail-spec",
            "--core-file",
            patched_core_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        spec.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&spec.stdout),
        String::from_utf8_lossy(&spec.stderr)
    );
    let spec_stdout = String::from_utf8_lossy(&spec.stdout);
    assert!(
        spec_stdout.contains("Action: Resolve ticket."),
        "{spec_stdout}"
    );
    assert!(
        spec_stdout.contains("- the system records a trace event named TicketClosed"),
        "{spec_stdout}"
    );

    fs::remove_file(core_path).unwrap();
    fs::remove_file(edit_path).unwrap();
    fs::remove_file(patched_core_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_flow_edit_adds_action_requirement_and_native_enforces_it() {
    use std::os::unix::fs::PermissionsExt;

    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let core_output = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    let core = parse_ail_core_text(&core_text).unwrap();
    let core_hash = ail_core_hash(&core);

    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let core_path = std::env::temp_dir().join(format!(
        "ail-flow-edit-add-requirement-{}-{unique_suffix}.ail-core.txt",
        std::process::id()
    ));
    let edit_path = std::env::temp_dir().join(format!(
        "ail-flow-edit-add-requirement-{}-{unique_suffix}.ail-flow.edit.json",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-flow-edit-add-requirement-native-{}-{unique_suffix}",
        std::process::id()
    ));
    fs::write(&core_path, core_text).unwrap();
    fs::write(
        &edit_path,
        format!(
            r#"{{
  "schema": "ail-flow.edit.v0",
  "package": "support-ticket",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "edits": [
    {{
      "op": "ActionCard.addRequirement",
      "target": "Action:CloseTicket",
      "requirement": "the ticket status to be Open",
      "provenance": ["flow:ActionCard:CloseTicket.requirement:open-status"]
    }}
  ]
}}"#
        ),
    )
    .unwrap();

    let patched_core = Command::new(binary)
        .args([
            "ail-flow-edit",
            "--core-file",
            core_path.to_str().unwrap(),
            edit_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        patched_core.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&patched_core.stdout),
        String::from_utf8_lossy(&patched_core.stderr)
    );
    let patched_core_text = String::from_utf8(patched_core.stdout).unwrap();
    let patched_core_artifact = parse_ail_core_text(&patched_core_text).unwrap();
    assert_eq!(check_ail_core(&patched_core_artifact), Vec::<String>::new());

    let patched_spec = render_ail_spec_from_core(&patched_core_artifact);
    assert!(
        patched_spec.contains("- the system requires the ticket status to be Open"),
        "{patched_spec}"
    );

    let bytecode = compile_ail_core_bytecode(&patched_core_artifact).unwrap();
    let close_ticket = bytecode.actions.get("CloseTicket").unwrap();
    assert!(
        close_ticket.instructions.iter().any(|instruction| {
            instruction.opcode == "REQUIRE_FIELD_IN"
                && instruction
                    .operands
                    .get("key")
                    .is_some_and(|key| key == "ticket.status")
                && instruction
                    .operands
                    .get("rule")
                    .is_some_and(|rule| rule == "the ticket status to be Open")
        }),
        "{close_ticket:?}"
    );

    let executable =
        compile_ail_core_native_elf(&patched_core_artifact, "CloseTicket", "linux-x86_64-elf")
            .unwrap();
    fs::write(&executable_path, executable).unwrap();
    let mut permissions = fs::metadata(&executable_path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable_path, permissions).unwrap();

    let success = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(success.status.success(), "Open ticket should pass");
    assert_eq!(
        String::from_utf8_lossy(&success.stdout),
        "ticket.status=Closed\n"
    );
    assert!(
        String::from_utf8_lossy(&success.stderr)
            .contains("rule passed: the ticket status to be Open"),
        "stderr:\n{}",
        String::from_utf8_lossy(&success.stderr)
    );

    let failed = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Assigned"])
        .output()
        .unwrap();
    assert!(!failed.status.success(), "Assigned ticket should fail");
    assert_eq!(String::from_utf8_lossy(&failed.stdout), "");
    assert!(
        String::from_utf8_lossy(&failed.stderr).contains("failure RequirementFailed"),
        "stderr:\n{}",
        String::from_utf8_lossy(&failed.stderr)
    );

    fs::remove_file(core_path).unwrap();
    fs::remove_file(edit_path).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
fn cli_ail_flow_edit_adds_data_table_field_and_round_trips_to_spec() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let core_output = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    let core = parse_ail_core_text(&core_text).unwrap();
    let core_hash = ail_core_hash(&core);

    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let core_path = std::env::temp_dir().join(format!(
        "ail-flow-edit-add-field-{}-{unique_suffix}.ail-core.txt",
        std::process::id()
    ));
    let edit_path = std::env::temp_dir().join(format!(
        "ail-flow-edit-add-field-{}-{unique_suffix}.ail-flow.edit.json",
        std::process::id()
    ));
    fs::write(&core_path, core_text).unwrap();
    fs::write(
        &edit_path,
        format!(
            r#"{{
  "schema": "ail-flow.edit.v0",
  "package": "support-ticket",
  "base_hash": "{core_hash}",
  "source_view": "DataTable:Ticket",
  "edits": [
    {{
      "op": "DataTable.addField",
      "target": "Thing:Ticket",
      "name": "priority",
      "type": "State<Low, Normal, High>",
      "secret": "false",
      "provenance": ["flow:DataTable:Ticket.field:priority"]
    }}
  ]
}}"#
        ),
    )
    .unwrap();

    let patched_core = Command::new(binary)
        .args([
            "ail-flow-edit",
            "--core-file",
            core_path.to_str().unwrap(),
            edit_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        patched_core.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&patched_core.stdout),
        String::from_utf8_lossy(&patched_core.stderr)
    );
    let patched_core_text = String::from_utf8(patched_core.stdout).unwrap();
    let patched_core_artifact = parse_ail_core_text(&patched_core_text).unwrap();
    assert_eq!(check_ail_core(&patched_core_artifact), Vec::<String>::new());
    assert!(
        patched_core_artifact
            .graph
            .find_node("Field", "Ticket.priority")
            .is_some()
    );
    assert!(
        patched_core_text.contains("edge has_field Thing:Ticket -> Field:Ticket.priority"),
        "{patched_core_text}"
    );

    let patched_spec = render_ail_spec_from_core(&patched_core_artifact);
    assert!(
        patched_spec.contains("- priority: State<Low, Normal, High>"),
        "{patched_spec}"
    );
    let patched_flow = render_ail_flow_view(&patched_core_artifact);
    assert!(
        patched_flow.contains(
            r#""name":"priority","coreLabel":"Field:Ticket.priority","type":"State<Low, Normal, High>","secret":false"#
        ),
        "{patched_flow}"
    );

    fs::remove_file(core_path).unwrap();
    fs::remove_file(edit_path).unwrap();
}

#[test]
fn ail_core_patch_rejects_package_mismatch() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "package": "wrong-package",
  "base_hash": "{core_hash}",
  "ops": []
}}"#
    );
    let error = apply_ail_core_patch_text(&core, &patch).unwrap_err();

    assert!(
        error.contains(
            "AIL-Core patch package mismatch: expected support-ticket, got wrong-package"
        ),
        "{error}"
    );
}

#[test]
fn ail_core_patch_rejects_non_string_package_guard() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "package": [],
  "base_hash": "{core_hash}",
  "ops": []
}}"#
    );
    let Err(error) = apply_ail_core_patch_text(&core, &patch) else {
        panic!("expected non-string package guard to be rejected");
    };

    assert!(
        error.contains("AIL-Core patch field 'package' must be a string"),
        "{error}"
    );
}

#[test]
fn ail_core_patch_rejects_checker_invalid_result() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "add_node",
      "kind": "Widget",
      "name": "Forgotten review state"
    }}
  ]
}}"#
    );
    let Err(error) = apply_ail_core_patch_text(&core, &patch) else {
        panic!("expected checker-invalid patch result to be rejected");
    };

    assert!(
        error.contains("AIL-Core patch result failed checker"),
        "{error}"
    );
    assert!(
        error.contains("unknown AIL-Core node kind 'Widget'"),
        "{error}"
    );
}

#[test]
fn ail_core_patch_removes_edge_by_core_labels() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "add_node",
      "kind": "Provenance",
      "name": "flow:ActionCard:CloseTicket.transient-note"
    }},
    {{
      "op": "add_edge",
      "kind": "has_provenance",
      "source": "Action:CloseTicket",
      "target": "Provenance:flow:ActionCard:CloseTicket.transient-note"
    }},
    {{
      "op": "remove_edge",
      "kind": "has_provenance",
      "source": "Action:CloseTicket",
      "target": "Provenance:flow:ActionCard:CloseTicket.transient-note"
    }}
  ]
}}"#
    );
    let patched = apply_ail_core_patch_text(&core, &patch).unwrap();

    assert_eq!(check_ail_core(&patched), Vec::<String>::new());
    assert!(!render_ail_core(&patched).contains(
        "edge has_provenance Action:CloseTicket -> Provenance:flow:ActionCard:CloseTicket.transient-note"
    ));
}

#[test]
fn ail_core_patch_remove_edge_rejects_missing_edge() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "remove_edge",
      "kind": "requires",
      "source": "Action:CloseTicket",
      "target": "Trace:TicketClosed"
    }}
  ]
}}"#
    );
    let error = apply_ail_core_patch_text(&core, &patch).unwrap_err();

    assert!(
        error.contains(
            "AIL-Core patch remove_edge did not find edge requires Action:CloseTicket -> Trace:TicketClosed"
        ),
        "{error}"
    );
}

#[test]
fn ail_core_patch_add_node_rejects_existing_node() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "add_node",
      "kind": "Rule",
      "name": "the ticket to exist"
    }}
  ]
}}"#
    );
    let Err(error) = apply_ail_core_patch_text(&core, &patch) else {
        panic!("expected duplicate add_node to be rejected");
    };

    assert!(
        error.contains(
            "AIL-Core patch add_node refuses to add existing node Rule:the ticket to exist"
        ),
        "{error}"
    );
}

#[test]
fn ail_core_patch_add_edge_rejects_existing_edge() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "add_edge",
      "kind": "records_trace",
      "source": "Action:CloseTicket",
      "target": "Trace:TicketClosed"
    }}
  ]
}}"#
    );
    let Err(error) = apply_ail_core_patch_text(&core, &patch) else {
        panic!("expected duplicate add_edge to be rejected");
    };

    assert!(
        error.contains(
            "AIL-Core patch add_edge refuses to add existing edge records_trace Action:CloseTicket -> Trace:TicketClosed"
        ),
        "{error}"
    );
}

#[test]
fn ail_core_patch_removes_detached_node_by_core_label() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "add_node",
      "kind": "Rule",
      "name": "temporary review note"
    }},
    {{
      "op": "remove_node",
      "target": "Rule:temporary review note"
    }}
  ]
}}"#
    );
    let patched = apply_ail_core_patch_text(&core, &patch).unwrap();
    let rendered = render_ail_core(&patched);

    assert_eq!(check_ail_core(&patched), Vec::<String>::new());
    assert!(
        !rendered.contains("node Rule temporary review note"),
        "{rendered}"
    );
}

#[test]
fn ail_core_patch_remove_node_rejects_incident_edges() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "remove_node",
      "target": "Rule:the ticket to exist"
    }}
  ]
}}"#
    );
    let error = apply_ail_core_patch_text(&core, &patch).unwrap_err();

    assert!(
        error.contains(
            "AIL-Core patch remove_node refuses to remove Rule:the ticket to exist because it has incident edges"
        ),
        "{error}"
    );
}

#[test]
fn ail_core_patch_declares_node_provenance_by_core_label() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "declare_provenance",
      "target": "Action:CloseTicket",
      "provenance": ["flow:ActionCard:CloseTicket.reviewed"]
    }}
  ]
}}"#
    );
    let patched = apply_ail_core_patch_text(&core, &patch).unwrap();
    let rendered = render_ail_core(&patched);

    assert_eq!(check_ail_core(&patched), Vec::<String>::new());
    assert!(
        rendered.contains("node Provenance flow:ActionCard:CloseTicket.reviewed"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "edge has_provenance Action:CloseTicket -> Provenance:flow:ActionCard:CloseTicket.reviewed"
        ),
        "{rendered}"
    );
}

#[test]
fn ail_core_patch_declare_provenance_requires_entries() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "declare_provenance",
      "target": "Action:CloseTicket"
    }}
  ]
}}"#
    );
    let error = apply_ail_core_patch_text(&core, &patch).unwrap_err();

    assert!(
        error.contains("AIL-Core patch declare_provenance must provide provenance"),
        "{error}"
    );
}

#[test]
fn ail_core_patch_replaces_edge_attributes_by_core_labels() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "add_node",
      "kind": "Provenance",
      "name": "flow:ActionCard:CloseTicket.edge-note"
    }},
    {{
      "op": "add_edge",
      "kind": "has_provenance",
      "source": "Action:CloseTicket",
      "target": "Provenance:flow:ActionCard:CloseTicket.edge-note",
      "attributes": {{
        "provenance": "flow:ActionCard:CloseTicket.edge-note.initial"
      }}
    }},
    {{
      "op": "replace_edge_attributes",
      "kind": "has_provenance",
      "source": "Action:CloseTicket",
      "target": "Provenance:flow:ActionCard:CloseTicket.edge-note",
      "attributes": {{
        "provenance": "flow:ActionCard:CloseTicket.edge-note.reviewed",
        "reviewed": "true"
      }}
    }}
  ]
}}"#
    );
    let patched = apply_ail_core_patch_text(&core, &patch).unwrap();
    let rendered = render_ail_core(&patched);

    assert_eq!(check_ail_core(&patched), Vec::<String>::new());
    assert!(
        rendered.contains(
            "edge has_provenance Action:CloseTicket -> Provenance:flow:ActionCard:CloseTicket.edge-note [provenance=flow:ActionCard:CloseTicket.edge-note.reviewed,reviewed=true]"
        ),
        "{rendered}"
    );
}

#[test]
fn ail_core_patch_replace_edge_attributes_requires_attributes() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_hash = ail_core_hash(&core);
    let patch = format!(
        r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "replace_edge_attributes",
      "kind": "records_trace",
      "source": "Action:CloseTicket",
      "target": "Trace:TicketClosed"
    }}
  ]
}}"#
    );
    let error = apply_ail_core_patch_text(&core, &patch).unwrap_err();

    assert!(
        error.contains("AIL-Core patch replace_edge_attributes must provide attributes"),
        "{error}"
    );
}

#[test]
fn ail_patch_adds_field_view_and_action_then_round_trips() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let patch_text = fs::read_to_string(format!(
        "{}/examples/patches/escalate-ticket.ail-patch.md",
        fixture("support_ticket.ail")
    ))
    .unwrap();
    let patch = parse_ail_patch_text(&patch_text).unwrap();
    let patched = apply_ail_patch(&document, &patch).unwrap();

    assert_eq!(
        patched.things["Ticket"].fields["escalation reason"].type_name,
        "Text"
    );
    assert!(
        patched
            .application
            .views
            .contains(&"an escalation queue for support managers".to_string())
    );
    assert!(patched.actions.contains_key("EscalateTicket"));
    assert_eq!(
        patched.actions["EscalateTicket"].trigger,
        "a support agent escalates a ticket"
    );
    assert!(
        patched.actions["EscalateTicket"]
            .writes
            .contains(&"the ticket escalation reason".to_string())
    );
    assert!(
        patched.actions["EscalateTicket"]
            .traces
            .contains(&"TicketEscalated".to_string())
    );

    let diagnostics = check_ail_core(&elaborate_ail_core(&package, &patched));
    assert_eq!(diagnostics, Vec::<String>::new());

    let rendered = render_ail_spec(&patched);
    assert!(rendered.contains("- escalation reason: Text"));
    assert!(rendered.contains("Action: Escalate ticket."));
    let reparsed = parse_ail_spec_text(&rendered).unwrap();
    let patched_core = render_ail_core(&elaborate_ail_core(&package, &patched));
    let reparsed_core = render_ail_core(&elaborate_ail_core(&package, &reparsed));
    assert_eq!(reparsed_core, patched_core);
}

#[test]
fn ail_spec_render_reparse_preserves_core_equivalence() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let rendered_spec = render_ail_spec(&document);
    let reparsed = parse_ail_spec_text(&rendered_spec).unwrap();

    assert!(rendered_spec.contains("The application Support Tickets manages"));
    assert!(rendered_spec.contains("Action: Close ticket."));
    assert!(rendered_spec.contains("- the system records a trace event named TicketClosed"));

    let original_core = render_ail_core(&elaborate_ail_core(&package, &document));
    let reparsed_core = render_ail_core(&elaborate_ail_core(&package, &reparsed));
    assert_eq!(reparsed_core, original_core);
}

#[test]
fn ail_spec_renders_from_checked_core_and_round_trips_equivalent() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let original_core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&original_core), Vec::<String>::new());
    let checked_core_text = render_ail_core(&original_core);
    let checked_core = parse_ail_core_text(&checked_core_text).unwrap();

    let rendered_spec = render_ail_spec_from_core(&checked_core);

    assert!(rendered_spec.contains("The application Support Tickets manages"));
    assert!(rendered_spec.contains("Action: Close ticket."));
    assert!(rendered_spec.contains("- the system changes the ticket status to Closed"));
    assert!(rendered_spec.contains("Failure PermissionDenied happens when"));

    let reparsed = parse_ail_package_spec_text(&package, &rendered_spec).unwrap();
    let reparsed_core = elaborate_ail_core(&package, &reparsed);
    assert_eq!(check_ail_core(&reparsed_core), Vec::<String>::new());
    assert_eq!(render_ail_core(&reparsed_core), checked_core_text);
}

#[test]
fn ail_spec_renders_from_checked_core_for_non_application_profiles() {
    for fixture_name in ["refund_tool.ail", "compiler_pass.ail", "network_driver.ail"] {
        let package = load_ail_package_dir(fixture(fixture_name)).unwrap();
        let document = parse_ail_package_document(&package).unwrap();
        let original_core = elaborate_ail_core(&package, &document);
        assert_eq!(check_ail_core(&original_core), Vec::<String>::new());
        let checked_core_text = render_ail_core(&original_core);
        let checked_core = parse_ail_core_text(&checked_core_text).unwrap();

        let rendered_spec = render_ail_spec_from_core(&checked_core);
        let reparsed = parse_ail_package_spec_text(&package, &rendered_spec).unwrap();
        let reparsed_core = elaborate_ail_core(&package, &reparsed);

        assert_eq!(
            check_ail_core(&reparsed_core),
            Vec::<String>::new(),
            "{fixture_name}\n{rendered_spec}"
        );
        assert_eq!(
            render_ail_core(&reparsed_core),
            checked_core_text,
            "{fixture_name}\n{rendered_spec}"
        );
    }
}

#[test]
fn ail_runtime_executes_close_ticket_success_and_not_found_failure() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let success = run_ail_action(
        &document,
        "CloseTicket",
        BTreeMap::from([
            ("ticket.id".to_string(), "T-1".to_string()),
            ("ticket.status".to_string(), "Open".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(success.status, "succeeded");
    assert_eq!(success.failure, None);
    assert_eq!(success.final_state["ticket.status"], "Closed");
    assert!(
        success
            .trace
            .contains(&"action CloseTicket started".to_string())
    );
    assert!(
        success
            .trace
            .contains(&"rule passed: the ticket to exist".to_string())
    );
    assert!(
        success
            .trace
            .contains(&"write ticket.status=Closed".to_string())
    );
    assert!(success.trace.contains(&"trace TicketClosed".to_string()));

    let missing = run_ail_action(
        &document,
        "CloseTicket",
        BTreeMap::from([("ticket.status".to_string(), "Open".to_string())]),
    )
    .unwrap();

    assert_eq!(missing.status, "failed");
    assert_eq!(missing.failure.as_deref(), Some("NotFound"));
    assert_eq!(
        missing.final_state.get("ticket.status").map(String::as_str),
        Some("Open")
    );
    assert!(
        missing
            .trace
            .contains(&"action CloseTicket started".to_string())
    );
    assert!(missing.trace.contains(&"failure NotFound".to_string()));
    assert!(missing.trace.contains(&"trace TicketNotFound".to_string()));
}

#[test]
fn ail_runtime_enforces_create_ticket_input_requirements() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let success = run_ail_action(
        &document,
        "CreateTicket",
        BTreeMap::from([
            ("customer.id".to_string(), "C-1".to_string()),
            ("ticket.title".to_string(), "Printer".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(success.status, "succeeded");
    assert_eq!(success.failure, None);
    assert_eq!(
        success.final_state.get("ticket.status").map(String::as_str),
        Some("New")
    );
    assert_eq!(
        success
            .final_state
            .get("ticket.customer.id")
            .map(String::as_str),
        Some("C-1")
    );
    assert!(
        success
            .trace
            .contains(&"rule passed: the customer id and title".to_string())
    );

    let missing = run_ail_action(
        &document,
        "CreateTicket",
        BTreeMap::from([("customer.id".to_string(), "C-1".to_string())]),
    )
    .unwrap();

    assert_eq!(missing.status, "failed");
    assert_eq!(missing.failure.as_deref(), Some("RequirementFailed"));
    assert_eq!(
        missing.final_state.get("customer.id").map(String::as_str),
        Some("C-1")
    );
    assert!(
        !missing
            .trace
            .contains(&"rule passed: the customer id and title".to_string())
    );
}

#[test]
fn ail_runtime_enforces_overdue_time_requirement() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_spec_text(&package.spec_text).unwrap();
    let success = run_ail_action(
        &document,
        "MarksOverdueTickets",
        BTreeMap::from([
            (
                "current.time".to_string(),
                "2026-05-23T10:00:00Z".to_string(),
            ),
            (
                "ticket.due_at".to_string(),
                "2026-05-23T09:00:00Z".to_string(),
            ),
            ("ticket.status".to_string(), "Open".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(success.status, "succeeded");
    assert_eq!(
        success.final_state.get("ticket.status").map(String::as_str),
        Some("Overdue")
    );
    assert!(
        success
            .trace
            .contains(&"rule passed: the current time to be later than due_at".to_string())
    );

    let not_due = run_ail_action(
        &document,
        "MarksOverdueTickets",
        BTreeMap::from([
            (
                "current.time".to_string(),
                "2026-05-23T08:00:00Z".to_string(),
            ),
            (
                "ticket.due_at".to_string(),
                "2026-05-23T09:00:00Z".to_string(),
            ),
            ("ticket.status".to_string(), "Open".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(not_due.status, "failed");
    assert_eq!(not_due.failure.as_deref(), Some("RequirementFailed"));
    assert_eq!(
        not_due.final_state.get("ticket.status").map(String::as_str),
        Some("Open")
    );
}

#[test]
fn ail_compiler_lowers_checked_application_to_bytecode() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    let bytecode = compile_ail_bytecode(&package, &document).unwrap();
    let rendered = render_ail_bytecode(&bytecode);

    assert!(rendered.contains(r#""kind":"AIL-Bytecode""#), "{rendered}");
    assert!(
        rendered.contains(r#""package":"support-ticket""#),
        "{rendered}"
    );
    assert!(rendered.contains(r#""action":"CloseTicket""#), "{rendered}");
    assert!(
        rendered.contains(r#""opcode":"ACTION_BEGIN""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"REQUIRE_EXISTS""#),
        "{rendered}"
    );
    assert!(rendered.contains(r#""key":"ticket.id""#), "{rendered}");
    assert!(
        rendered.contains(r#""opcode":"REQUIRE_FIELD_NOT_EQUALS""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""key":"ticket.assignee.role""#),
        "{rendered}"
    );
    assert!(rendered.contains(r#""key":"customer.id""#), "{rendered}");
    assert!(rendered.contains(r#""key":"ticket.title""#), "{rendered}");
    assert!(
        rendered.contains(r#""rule":"the customer id and title""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""rule":"the assignee role to be SupportAgent or SupportManager""#),
        "{rendered}"
    );
    assert!(rendered.contains(r#""opcode":"SET_FIELD""#), "{rendered}");
    assert!(rendered.contains(r#""value":"Closed""#), "{rendered}");
    assert!(rendered.contains(r#""value":"New""#), "{rendered}");
    assert!(rendered.contains(r#""opcode":"COPY_FIELD""#), "{rendered}");
    assert!(rendered.contains(r#""source":"customer.id""#), "{rendered}");
    assert!(
        rendered.contains(r#""key":"ticket.customer.id""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""opcode":"REQUIRE_FIELD_AFTER""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""source":"current.time""#),
        "{rendered}"
    );
    assert!(rendered.contains(r#""key":"ticket.due_at""#), "{rendered}");
    assert!(rendered.contains(r#""opcode":"EMIT_TRACE""#), "{rendered}");
    assert!(rendered.contains(r#""failure":"NotFound""#), "{rendered}");
    assert!(
        rendered.contains(r#""traces":["TicketNotFound"]"#),
        "{rendered}"
    );
}

#[test]
fn ail_compiler_lowers_checked_core_ir_to_bytecode() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    let bytecode = compile_ail_core_bytecode(&core).unwrap();
    let rendered = render_ail_bytecode(&bytecode);

    assert!(rendered.contains(r#""kind":"AIL-Bytecode""#), "{rendered}");
    assert!(
        rendered.contains(r#""package":"support-ticket""#),
        "{rendered}"
    );
    assert!(rendered.contains(r#""action":"CloseTicket""#), "{rendered}");
    assert!(
        rendered.contains(r#""opcode":"REQUIRE_EXISTS""#),
        "{rendered}"
    );
    assert!(rendered.contains(r#""key":"ticket.id""#), "{rendered}");
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    let run = run_ail_bytecode_action(
        &bytecode,
        "CloseTicket",
        BTreeMap::from([
            ("ticket.id".to_string(), "T-1".to_string()),
            ("ticket.status".to_string(), "Open".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(run.status, "succeeded");
    assert_eq!(run.final_state["ticket.status"], "Closed");
    assert!(run.trace.contains(&"trace TicketClosed".to_string()));
}

#[test]
fn ail_core_compilers_reject_unchecked_core_ir() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let mut core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    let close_ticket_id = core
        .graph
        .find_node("Action", "CloseTicket")
        .unwrap()
        .id
        .clone();
    core.graph
        .edges
        .retain(|edge| !(edge.kind == "records_trace" && edge.source == close_ticket_id));
    assert!(
        check_ail_core(&core)
            .contains(&"AIL-TRACE-001 action CloseTicket is missing trace coverage".to_string())
    );

    let bytecode_error = compile_ail_core_bytecode(&core).unwrap_err();
    assert!(
        bytecode_error.contains("cannot compile unchecked AIL-Core"),
        "{bytecode_error}"
    );
    assert!(
        bytecode_error.contains("AIL-TRACE-001 action CloseTicket is missing trace coverage"),
        "{bytecode_error}"
    );

    let native_error =
        compile_ail_core_native_elf(&core, "CloseTicket", "linux-x86_64-elf").unwrap_err();
    assert!(
        native_error.contains("cannot compile unchecked AIL-Core"),
        "{native_error}"
    );
    assert!(
        native_error.contains("AIL-TRACE-001 action CloseTicket is missing trace coverage"),
        "{native_error}"
    );
}

#[test]
fn ail_core_native_compile_rejects_manifest_unsupported_target() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let mut core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    core.package.target_support = BTreeMap::from([(
        "x86_64-unknown-linux-syscall-elf".to_string(),
        "planned-contract".to_string(),
    )]);

    let error = compile_ail_core_native_elf(&core, "CloseTicket", "linux-x86_64-elf").unwrap_err();
    assert!(error.contains("AIL-BACKEND-001"), "{error}");
    assert!(
        error.contains("x86_64-unknown-linux-syscall-elf"),
        "{error}"
    );
    assert!(error.contains("planned-contract"), "{error}");
}

#[test]
fn ail_core_native_compile_reports_unknown_target_support_status_before_backend_support() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let mut core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    core.package.target_support = BTreeMap::from([(
        "x86_64-unknown-linux-syscall-elf".to_string(),
        "experimental-preview".to_string(),
    )]);

    let error = compile_ail_core_native_elf(&core, "CloseTicket", "linux-x86_64-elf").unwrap_err();
    assert!(
        error.contains("cannot compile unchecked AIL-Core"),
        "{error}"
    );
    assert!(error.contains("AIL-BACKEND-002"), "{error}");
    assert!(
        error.contains("x86_64-unknown-linux-syscall-elf"),
        "{error}"
    );
    assert!(error.contains("experimental-preview"), "{error}");
}

#[test]
fn ail_bytecode_native_compile_rejects_manifest_unsupported_target() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let mut core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    core.package.target_support = BTreeMap::from([(
        "x86_64-unknown-linux-syscall-elf".to_string(),
        "planned-contract".to_string(),
    )]);
    let bytecode = compile_ail_core_bytecode(&core).unwrap();
    let rendered = render_ail_bytecode(&bytecode);
    assert!(
        rendered.contains(
            r#""target_support":{"x86_64-unknown-linux-syscall-elf":"planned-contract"}"#
        ),
        "{rendered}"
    );

    let parsed = parse_ail_bytecode(&rendered).unwrap();
    let error =
        compile_ail_bytecode_native_elf(&parsed, "CloseTicket", "linux-x86_64-elf").unwrap_err();
    assert!(error.contains("AIL-BACKEND-001"), "{error}");
    assert!(
        error.contains("x86_64-unknown-linux-syscall-elf"),
        "{error}"
    );
    assert!(error.contains("planned-contract"), "{error}");
}

#[test]
fn ail_bytecode_vm_executes_close_ticket_success_and_failure() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let bytecode = compile_ail_bytecode(&package, &document).unwrap();

    let success = run_ail_bytecode_action(
        &bytecode,
        "CloseTicket",
        BTreeMap::from([
            ("ticket.id".to_string(), "T-1".to_string()),
            ("ticket.status".to_string(), "Open".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(success.status, "succeeded");
    assert_eq!(success.failure, None);
    assert_eq!(success.final_state["ticket.status"], "Closed");
    assert!(
        success
            .trace
            .contains(&"action CloseTicket started".to_string())
    );
    assert!(
        success
            .trace
            .contains(&"rule passed: the ticket to exist".to_string())
    );
    assert!(
        success
            .trace
            .contains(&"write ticket.status=Closed".to_string())
    );
    assert!(success.trace.contains(&"trace TicketClosed".to_string()));

    let missing = run_ail_bytecode_action(
        &bytecode,
        "CloseTicket",
        BTreeMap::from([("ticket.status".to_string(), "Open".to_string())]),
    )
    .unwrap();

    assert_eq!(missing.status, "failed");
    assert_eq!(missing.failure.as_deref(), Some("NotFound"));
    assert!(missing.trace.contains(&"failure NotFound".to_string()));
    assert!(missing.trace.contains(&"trace TicketNotFound".to_string()));
}

#[test]
fn ail_bytecode_vm_executes_create_ticket_state_writes() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let bytecode = compile_ail_bytecode(&package, &document).unwrap();

    let success = run_ail_bytecode_action(
        &bytecode,
        "CreateTicket",
        BTreeMap::from([
            ("customer.id".to_string(), "C-1".to_string()),
            ("ticket.title".to_string(), "Printer".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(success.status, "succeeded");
    assert_eq!(
        success.final_state.get("ticket.status").map(String::as_str),
        Some("New")
    );
    assert_eq!(
        success
            .final_state
            .get("ticket.customer.id")
            .map(String::as_str),
        Some("C-1")
    );
    assert!(
        success
            .trace
            .contains(&"write ticket.customer.id".to_string())
    );
}

#[test]
fn ail_bytecode_vm_enforces_overdue_time_requirement() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let bytecode = compile_ail_bytecode(&package, &document).unwrap();

    let success = run_ail_bytecode_action(
        &bytecode,
        "MarksOverdueTickets",
        BTreeMap::from([
            (
                "current.time".to_string(),
                "2026-05-23T10:00:00Z".to_string(),
            ),
            (
                "ticket.due_at".to_string(),
                "2026-05-23T09:00:00Z".to_string(),
            ),
            ("ticket.status".to_string(), "Open".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(success.status, "succeeded");
    assert_eq!(
        success.final_state.get("ticket.status").map(String::as_str),
        Some("Overdue")
    );

    let not_due = run_ail_bytecode_action(
        &bytecode,
        "MarksOverdueTickets",
        BTreeMap::from([
            (
                "current.time".to_string(),
                "2026-05-23T08:00:00Z".to_string(),
            ),
            (
                "ticket.due_at".to_string(),
                "2026-05-23T09:00:00Z".to_string(),
            ),
            ("ticket.status".to_string(), "Open".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(not_due.status, "failed");
    assert_eq!(not_due.failure.as_deref(), Some("RequirementFailed"));
    assert_eq!(
        not_due.final_state.get("ticket.status").map(String::as_str),
        Some("Open")
    );
}

#[test]
fn ail_bytecode_artifact_parses_and_executes_without_source_package() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let bytecode = compile_ail_bytecode(&package, &document).unwrap();
    let rendered = render_ail_bytecode(&bytecode);
    let parsed = parse_ail_bytecode(&rendered).unwrap();

    assert_eq!(render_ail_bytecode(&parsed), rendered);

    let success = run_ail_bytecode_action(
        &parsed,
        "CloseTicket",
        BTreeMap::from([
            ("ticket.id".to_string(), "T-1".to_string()),
            ("ticket.status".to_string(), "Open".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(success.status, "succeeded");
    assert_eq!(success.final_state["ticket.status"], "Closed");
    assert!(success.trace.contains(&"trace TicketClosed".to_string()));
}

#[test]
fn ail_bytecode_verifier_rejects_invalid_opcodes_and_operands() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let bytecode = compile_ail_bytecode(&package, &document).unwrap();
    let rendered = render_ail_bytecode(&bytecode);
    let parsed = parse_ail_bytecode(&rendered).unwrap();

    assert_eq!(verify_ail_bytecode(&parsed), Vec::<String>::new());

    let mut invalid_opcode = parsed.clone();
    invalid_opcode
        .actions
        .get_mut("CloseTicket")
        .unwrap()
        .instructions
        .iter_mut()
        .find(|instruction| instruction.opcode == "SET_FIELD")
        .unwrap()
        .opcode = "SET_FIELD_BOGUS".to_string();
    let diagnostics: Vec<String> = verify_ail_bytecode(&invalid_opcode);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.contains("AILBC001")
                && diagnostic.contains("CloseTicket")
                && diagnostic.contains("SET_FIELD_BOGUS")
        }),
        "{diagnostics:?}"
    );

    let mut missing_operand = parsed.clone();
    let set_field = missing_operand
        .actions
        .get_mut("CloseTicket")
        .unwrap()
        .instructions
        .iter_mut()
        .find(|instruction| instruction.opcode == "SET_FIELD")
        .unwrap();
    set_field.operands.remove("key");
    let diagnostics: Vec<String> = verify_ail_bytecode(&missing_operand);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.contains("AILBC002")
                && diagnostic.contains("SET_FIELD")
                && diagnostic.contains("key")
        }),
        "{diagnostics:?}"
    );
}

#[test]
fn ail_bytecode_verifier_rejects_non_integer_add_int_delta() {
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "bad-counter",
  "version": "0.1.0",
  "profile": "Application",
  "failures": [],
  "actions": [
    {
      "action": "BadCounter",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"BadCounter"}},
        {"opcode":"ADD_INT_FIELD","operands":{"key":"counter","delta":"one","text":"bad increment"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    let bytecode = parse_ail_bytecode(bytecode_text).unwrap();
    let diagnostics = verify_ail_bytecode(&bytecode);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.contains("AILBC006")
                && diagnostic.contains("BadCounter")
                && diagnostic.contains("ADD_INT_FIELD")
                && diagnostic.contains("delta")
                && diagnostic.contains("one")
        }),
        "{diagnostics:?}"
    );
}

#[test]
fn ail_bytecode_verifier_rejects_unknown_target_support_status() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let mut bytecode = compile_ail_bytecode(&package, &document).unwrap();
    bytecode.target_support = BTreeMap::from([(
        "x86_64-unknown-linux-syscall-elf".to_string(),
        "experimental-preview".to_string(),
    )]);

    let diagnostics = verify_ail_bytecode(&bytecode);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.contains("AIL-BACKEND-002")
                && diagnostic.contains("x86_64-unknown-linux-syscall-elf")
                && diagnostic.contains("experimental-preview")
        }),
        "{diagnostics:?}"
    );
}

#[test]
fn ail_bytecode_vm_executes_branch_and_jump_control_flow() {
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "branching-example",
  "version": "0.1.0",
  "profile": "Application",
  "failures": [],
  "actions": [
    {
      "action": "ResolveTicket",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"ResolveTicket"}},
        {"opcode":"BRANCH_FIELD_EQUALS","operands":{"key":"ticket.priority","value":"High","label":"high_priority"}},
        {"opcode":"SET_FIELD","operands":{"key":"ticket.queue","value":"standard","text":"standard queue"}},
        {"opcode":"JUMP","operands":{"label":"done"}},
        {"opcode":"LABEL","operands":{"name":"high_priority"}},
        {"opcode":"SET_FIELD","operands":{"key":"ticket.queue","value":"urgent","text":"urgent queue"}},
        {"opcode":"LABEL","operands":{"name":"done"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    let bytecode = parse_ail_bytecode(bytecode_text).unwrap();

    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    let high_priority = run_ail_bytecode_action(
        &bytecode,
        "ResolveTicket",
        BTreeMap::from([("ticket.priority".to_string(), "High".to_string())]),
    )
    .unwrap();
    assert_eq!(high_priority.status, "succeeded");
    assert_eq!(
        high_priority
            .final_state
            .get("ticket.queue")
            .map(String::as_str),
        Some("urgent")
    );
    assert!(
        high_priority
            .trace
            .contains(&"branch high_priority taken".to_string()),
        "{:?}",
        high_priority.trace
    );

    let standard = run_ail_bytecode_action(
        &bytecode,
        "ResolveTicket",
        BTreeMap::from([("ticket.priority".to_string(), "Low".to_string())]),
    )
    .unwrap();
    assert_eq!(standard.status, "succeeded");
    assert_eq!(
        standard.final_state.get("ticket.queue").map(String::as_str),
        Some("standard")
    );
    assert!(
        standard
            .trace
            .contains(&"branch high_priority skipped".to_string()),
        "{:?}",
        standard.trace
    );
    assert!(standard.trace.contains(&"jump done".to_string()));

    let mut missing_label = bytecode.clone();
    missing_label
        .actions
        .get_mut("ResolveTicket")
        .unwrap()
        .instructions
        .iter_mut()
        .find(|instruction| instruction.opcode == "JUMP")
        .unwrap()
        .operands
        .insert("label".to_string(), "missing".to_string());
    let diagnostics = verify_ail_bytecode(&missing_label);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.contains("AILBC003")
                && diagnostic.contains("ResolveTicket")
                && diagnostic.contains("missing")
        }),
        "{diagnostics:?}"
    );
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn ail_native_elf_executes_bytecode_branch_and_jump_control_flow() {
    use std::os::unix::fs::PermissionsExt;

    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "branching-example",
  "version": "0.1.0",
  "profile": "Application",
  "failures": [],
  "actions": [
    {
      "action": "ResolveTicket",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"ResolveTicket"}},
        {"opcode":"BRANCH_FIELD_EQUALS","operands":{"key":"ticket.priority","value":"High","label":"high_priority"}},
        {"opcode":"SET_FIELD","operands":{"key":"ticket.queue","value":"standard","text":"standard queue"}},
        {"opcode":"JUMP","operands":{"label":"done"}},
        {"opcode":"LABEL","operands":{"name":"high_priority"}},
        {"opcode":"SET_FIELD","operands":{"key":"ticket.queue","value":"urgent","text":"urgent queue"}},
        {"opcode":"LABEL","operands":{"name":"done"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    let bytecode = parse_ail_bytecode(bytecode_text).unwrap();
    let executable =
        compile_ail_bytecode_native_elf(&bytecode, "ResolveTicket", "linux-x86_64-elf").unwrap();
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let executable_dir = std::env::temp_dir().join(format!(
        "ail-branch-bytecode-native-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&executable_dir);
    fs::create_dir(&executable_dir).unwrap();
    let executable_path = executable_dir.join("ResolveTicket");
    {
        let mut file = fs::File::create(&executable_path).unwrap();
        file.write_all(&executable).unwrap();
        file.sync_all().unwrap();
    }
    let mut permissions = fs::metadata(&executable_path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable_path, permissions).unwrap();

    let high = Command::new(&executable_path)
        .arg("ticket.priority=High")
        .output()
        .unwrap();
    assert!(high.status.success(), "high priority branch failed");
    assert_eq!(
        String::from_utf8_lossy(&high.stdout),
        "ticket.queue=urgent\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&high.stderr),
        concat!(
            "action ResolveTicket started\n",
            "branch high_priority taken\n",
            "write ticket.queue=urgent\n"
        )
    );

    let standard = Command::new(&executable_path)
        .arg("ticket.priority=Low")
        .output()
        .unwrap();
    assert!(standard.status.success(), "standard branch failed");
    assert_eq!(
        String::from_utf8_lossy(&standard.stdout),
        "ticket.queue=standard\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&standard.stderr),
        concat!(
            "action ResolveTicket started\n",
            "branch high_priority skipped\n",
            "write ticket.queue=standard\n",
            "jump done\n"
        )
    );

    fs::remove_dir_all(executable_dir).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn ail_native_elf_rejects_backward_bytecode_jump() {
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "looping-example",
  "version": "0.1.0",
  "profile": "Application",
  "failures": [],
  "actions": [
    {
      "action": "LoopForever",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"LoopForever"}},
        {"opcode":"LABEL","operands":{"name":"loop"}},
        {"opcode":"JUMP","operands":{"label":"loop"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    let bytecode = parse_ail_bytecode(bytecode_text).unwrap();
    let error =
        compile_ail_bytecode_native_elf(&bytecode, "LoopForever", "linux-x86_64-elf").unwrap_err();

    assert!(
        error.contains("backward JUMP") && error.contains("LoopForever"),
        "{error}"
    );
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn ail_native_elf_rejects_backward_bytecode_branch() {
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "looping-branch-example",
  "version": "0.1.0",
  "profile": "Application",
  "failures": [],
  "actions": [
    {
      "action": "LoopOnBranch",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"LoopOnBranch"}},
        {"opcode":"LABEL","operands":{"name":"loop"}},
        {"opcode":"BRANCH_FIELD_EQUALS","operands":{"key":"counter","value":"1","label":"loop"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    let bytecode = parse_ail_bytecode(bytecode_text).unwrap();
    let error =
        compile_ail_bytecode_native_elf(&bytecode, "LoopOnBranch", "linux-x86_64-elf").unwrap_err();

    assert!(
        error.contains("backward BRANCH_FIELD_EQUALS") && error.contains("LoopOnBranch"),
        "{error}"
    );
}

#[test]
fn ail_bytecode_vm_executes_action_call_control_flow() {
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "call-example",
  "version": "0.1.0",
  "profile": "Application",
  "failures": [],
  "actions": [
    {
      "action": "ResolveTicket",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"ResolveTicket"}},
        {"opcode":"CALL_ACTION","operands":{"target":"CloseTicket"}},
        {"opcode":"SET_FIELD","operands":{"key":"ticket.resolution","value":"Resolved","text":"resolution note"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    },
    {
      "action": "CloseTicket",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"CloseTicket"}},
        {"opcode":"SET_FIELD","operands":{"key":"ticket.status","value":"Closed","text":"close ticket"}},
        {"opcode":"EMIT_TRACE","operands":{"event":"TicketClosed"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    let bytecode = parse_ail_bytecode(bytecode_text).unwrap();

    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    let run = run_ail_bytecode_action(
        &bytecode,
        "ResolveTicket",
        BTreeMap::from([("ticket.status".to_string(), "Open".to_string())]),
    )
    .unwrap();
    assert_eq!(run.status, "succeeded");
    assert_eq!(
        run.final_state.get("ticket.status").map(String::as_str),
        Some("Closed")
    );
    assert_eq!(
        run.final_state.get("ticket.resolution").map(String::as_str),
        Some("Resolved")
    );
    assert!(run.trace.contains(&"call action CloseTicket".to_string()));
    assert!(
        run.trace
            .contains(&"action CloseTicket started".to_string())
    );
    assert!(run.trace.contains(&"trace TicketClosed".to_string()));

    let mut missing_target = bytecode.clone();
    missing_target
        .actions
        .get_mut("ResolveTicket")
        .unwrap()
        .instructions
        .iter_mut()
        .find(|instruction| instruction.opcode == "CALL_ACTION")
        .unwrap()
        .operands
        .insert("target".to_string(), "MissingAction".to_string());
    let diagnostics = verify_ail_bytecode(&missing_target);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.contains("AILBC005")
                && diagnostic.contains("ResolveTicket")
                && diagnostic.contains("MissingAction")
        }),
        "{diagnostics:?}"
    );
}

#[test]
fn ail_spec_lowers_action_call_bullets_to_call_action_bytecode() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let spec = r#"
The application Call Example manages ticket actions.

A Ticket has:

- status: State<Open, Closed>
- resolution: Text

Action: Close ticket.

When close ticket happens:

- the system changes the ticket status to Closed
- the system records a trace event named TicketClosed

Action: Resolve ticket.

When resolve ticket happens:

- the system calls CloseTicket
- the system changes the ticket resolution to Resolved
- the system records a trace event named TicketResolved
"#;
    let document = parse_ail_spec_text(spec).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(
        rendered_core.contains("edge calls Action:ResolveTicket -> Action:CloseTicket"),
        "{rendered_core}"
    );

    let interpreted = run_ail_action(
        &document,
        "ResolveTicket",
        BTreeMap::from([("ticket.status".to_string(), "Open".to_string())]),
    )
    .unwrap();
    assert_eq!(interpreted.status, "succeeded");
    assert_eq!(
        interpreted
            .final_state
            .get("ticket.status")
            .map(String::as_str),
        Some("Closed")
    );
    assert_eq!(
        interpreted
            .final_state
            .get("ticket.resolution")
            .map(String::as_str),
        Some("Resolved")
    );
    assert!(
        interpreted
            .trace
            .contains(&"call action CloseTicket".to_string())
    );

    let bytecode = compile_ail_core_bytecode(&core).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    let resolve_ticket = bytecode.actions.get("ResolveTicket").unwrap();
    assert!(resolve_ticket.instructions.iter().any(|instruction| {
        instruction.opcode == "CALL_ACTION"
            && instruction
                .operands
                .get("target")
                .is_some_and(|target| target == "CloseTicket")
    }));

    let run = run_ail_bytecode_action(
        &bytecode,
        "ResolveTicket",
        BTreeMap::from([("ticket.status".to_string(), "Open".to_string())]),
    )
    .unwrap();
    assert_eq!(run.status, "succeeded");
    assert_eq!(
        run.final_state.get("ticket.status").map(String::as_str),
        Some("Closed")
    );
    assert_eq!(
        run.final_state.get("ticket.resolution").map(String::as_str),
        Some("Resolved")
    );
    assert!(run.trace.contains(&"call action CloseTicket".to_string()));
    assert!(run.trace.contains(&"trace TicketClosed".to_string()));
    assert!(run.trace.contains(&"trace TicketResolved".to_string()));
}

#[test]
fn ail_spec_lowers_stateful_counter_increment_to_integer_bytecode() {
    let package = load_ail_package_dir(fixture("stateful_counter.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    let bytecode = compile_ail_core_bytecode(&core).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    let rendered = render_ail_bytecode(&bytecode);
    assert!(
        rendered.contains(r#""opcode":"ADD_INT_FIELD""#),
        "{rendered}"
    );
    assert!(rendered.contains(r#""key":"counter.value""#), "{rendered}");
    assert!(rendered.contains(r#""delta":"1""#), "{rendered}");

    let run = run_ail_bytecode_action(
        &bytecode,
        "IncrementCounter",
        BTreeMap::from([("counter.value".to_string(), "41".to_string())]),
    )
    .unwrap();

    assert_eq!(run.status, "succeeded");
    assert_eq!(
        run.final_state.get("counter.value").map(String::as_str),
        Some("42")
    );
    assert!(
        run.trace
            .contains(&"add counter.value by 1 -> 42".to_string()),
        "{:?}",
        run.trace
    );
    assert!(
        run.trace.contains(&"trace CounterIncremented".to_string()),
        "{:?}",
        run.trace
    );
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn ail_native_elf_executes_bytecode_action_call_control_flow() {
    use std::os::unix::fs::PermissionsExt;

    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "call-example",
  "version": "0.1.0",
  "profile": "Application",
  "failures": [],
  "actions": [
    {
      "action": "ResolveTicket",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"ResolveTicket"}},
        {"opcode":"CALL_ACTION","operands":{"target":"CloseTicket"}},
        {"opcode":"SET_FIELD","operands":{"key":"ticket.resolution","value":"Resolved","text":"resolution note"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    },
    {
      "action": "CloseTicket",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"CloseTicket"}},
        {"opcode":"SET_FIELD","operands":{"key":"ticket.status","value":"Closed","text":"close ticket"}},
        {"opcode":"EMIT_TRACE","operands":{"event":"TicketClosed"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    let bytecode = parse_ail_bytecode(bytecode_text).unwrap();
    let executable =
        compile_ail_bytecode_native_elf(&bytecode, "ResolveTicket", "linux-x86_64-elf").unwrap();
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let executable_path = std::env::temp_dir().join(format!(
        "ail-call-bytecode-native-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);
    fs::write(&executable_path, executable).unwrap();
    let mut permissions = fs::metadata(&executable_path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable_path, permissions).unwrap();

    let run = Command::new(&executable_path)
        .arg("ticket.status=Open")
        .output()
        .unwrap();
    assert!(run.status.success(), "native CALL_ACTION failed");
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        concat!("ticket.status=Closed\n", "ticket.resolution=Resolved\n")
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        concat!(
            "action ResolveTicket started\n",
            "call action CloseTicket\n",
            "action CloseTicket started\n",
            "write ticket.status=Closed\n",
            "trace TicketClosed\n",
            "write ticket.resolution=Resolved\n"
        )
    );

    fs::remove_file(executable_path).unwrap();
}

#[test]
fn ail_bytecode_vm_executes_integer_loop_state_mutation() {
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "loop-example",
  "version": "0.1.0",
  "profile": "Application",
  "failures": [],
  "actions": [
    {
      "action": "Countdown",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"Countdown"}},
        {"opcode":"LABEL","operands":{"name":"loop"}},
        {"opcode":"BRANCH_FIELD_EQUALS","operands":{"key":"counter","value":"0","label":"done"}},
        {"opcode":"ADD_INT_FIELD","operands":{"key":"counter","delta":"-1","text":"decrement counter"}},
        {"opcode":"ADD_INT_FIELD","operands":{"key":"iterations","delta":"1","text":"count iteration"}},
        {"opcode":"JUMP","operands":{"label":"loop"}},
        {"opcode":"LABEL","operands":{"name":"done"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    let bytecode = parse_ail_bytecode(bytecode_text).unwrap();

    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    let run = run_ail_bytecode_action(
        &bytecode,
        "Countdown",
        BTreeMap::from([
            ("counter".to_string(), "3".to_string()),
            ("iterations".to_string(), "0".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(run.status, "succeeded");
    assert_eq!(
        run.final_state.get("counter").map(String::as_str),
        Some("0")
    );
    assert_eq!(
        run.final_state.get("iterations").map(String::as_str),
        Some("3")
    );
    assert!(
        run.trace.contains(&"add counter by -1 -> 2".to_string()),
        "{:?}",
        run.trace
    );
    assert!(
        run.trace.contains(&"add iterations by 1 -> 3".to_string()),
        "{:?}",
        run.trace
    );
}

#[test]
fn ail_spec_lowers_repeated_task_to_repeat_action_bytecode() {
    let package = load_ail_package_dir(fixture("repeated_task.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let rendered_core = render_ail_core(&core);
    assert!(
        rendered_core.contains(
            "edge repeats Action:RunMaintenanceCycle -> Action:IncrementCounter [count=3]"
        ),
        "{rendered_core}"
    );

    let bytecode = compile_ail_core_bytecode(&core).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    let rendered = render_ail_bytecode(&bytecode);
    assert!(
        rendered.contains(r#""opcode":"REPEAT_ACTION""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""target":"IncrementCounter""#),
        "{rendered}"
    );
    assert!(rendered.contains(r#""count":"3""#), "{rendered}");

    let run = run_ail_bytecode_action(
        &bytecode,
        "RunMaintenanceCycle",
        BTreeMap::from([("counter.value".to_string(), "0".to_string())]),
    )
    .unwrap();

    assert_eq!(run.status, "succeeded");
    assert_eq!(
        run.final_state.get("counter.value").map(String::as_str),
        Some("3")
    );
    assert!(
        run.trace
            .contains(&"repeat action IncrementCounter 3 times".to_string()),
        "{:?}",
        run.trace
    );
    assert!(
        run.trace
            .contains(&"repeat IncrementCounter iteration 3".to_string()),
        "{:?}",
        run.trace
    );
    assert!(
        run.trace
            .contains(&"trace MaintenanceCycleCompleted".to_string()),
        "{:?}",
        run.trace
    );
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn ail_native_elf_executes_repeated_action_bytecode() {
    use std::os::unix::fs::PermissionsExt;

    let package = load_ail_package_dir(fixture("repeated_task.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let bytecode = compile_ail_bytecode(&package, &document).unwrap();
    let executable =
        compile_ail_bytecode_native_elf(&bytecode, "RunMaintenanceCycle", "linux-x86_64-elf")
            .unwrap();
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let executable_path = std::env::temp_dir().join(format!(
        "ail-repeat-bytecode-native-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);
    fs::write(&executable_path, executable).unwrap();
    let mut permissions = fs::metadata(&executable_path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable_path, permissions).unwrap();

    let run = Command::new(&executable_path)
        .arg("counter.value=0")
        .output()
        .unwrap();
    assert!(run.status.success(), "native repeated action failed");
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "counter.value=1\ncounter.value=2\ncounter.value=3\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        concat!(
            "action RunMaintenanceCycle started\n",
            "repeat action IncrementCounter 3 times\n",
            "repeat IncrementCounter iteration 1\n",
            "action IncrementCounter started\n",
            "add counter.value by 1 -> 1\n",
            "trace CounterIncremented\n",
            "repeat IncrementCounter iteration 2\n",
            "action IncrementCounter started\n",
            "add counter.value by 1 -> 2\n",
            "trace CounterIncremented\n",
            "repeat IncrementCounter iteration 3\n",
            "action IncrementCounter started\n",
            "add counter.value by 1 -> 3\n",
            "trace CounterIncremented\n",
            "trace MaintenanceCycleCompleted\n"
        )
    );

    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn ail_native_elf_executes_bytecode_integer_state_mutation() {
    use std::os::unix::fs::PermissionsExt;

    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "counter-example",
  "version": "0.1.0",
  "profile": "Application",
  "failures": [],
  "actions": [
    {
      "action": "IncrementCounter",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"IncrementCounter"}},
        {"opcode":"ADD_INT_FIELD","operands":{"key":"counter","delta":"2","text":"increment counter"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    let bytecode = parse_ail_bytecode(bytecode_text).unwrap();
    let executable =
        compile_ail_bytecode_native_elf(&bytecode, "IncrementCounter", "linux-x86_64-elf").unwrap();
    let executable_path = std::env::temp_dir().join(format!(
        "ail-add-int-bytecode-native-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);
    fs::write(&executable_path, executable).unwrap();
    let mut permissions = fs::metadata(&executable_path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable_path, permissions).unwrap();

    let run = Command::new(&executable_path)
        .arg("counter=3")
        .output()
        .unwrap();
    assert!(run.status.success(), "native ADD_INT_FIELD failed");
    assert_eq!(String::from_utf8_lossy(&run.stdout), "counter=5\n");
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        concat!(
            "action IncrementCounter started\n",
            "add counter by 2 -> 5\n"
        )
    );

    let invalid = Command::new(&executable_path)
        .arg("counter=abc")
        .output()
        .unwrap();
    assert!(
        !invalid.status.success(),
        "native ADD_INT_FIELD accepted text"
    );
    assert_eq!(String::from_utf8_lossy(&invalid.stdout), "");

    fs::remove_file(executable_path).unwrap();
}

#[test]
fn ail_toolchain_agent_package_lowers_to_verified_bytecode() {
    let package = load_ail_package_dir(fixture("ail_toolchain_agent.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    let bytecode = compile_ail_bytecode(&package, &document).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    let rendered = render_ail_bytecode(&bytecode);

    assert!(
        rendered.contains(r#""package":"ail-toolchain-agent""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""action":"CompileApplication""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""action":"CompareAgentPromptPortability""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""action":"CompileNativeTarget""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""action":"VerifyBytecodeArtifact""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""action":"VerifyLowerManifest""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""action":"VerifyTargetArtifact""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""action":"VerifyBuildManifest""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""action":"VerifyCompileManifest""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""action":"VerifyPassManifest""#),
        "{rendered}"
    );
    assert!(
        rendered.contains(r#""value":"BytecodeReady""#),
        "{rendered}"
    );

    let run = run_ail_bytecode_action(
        &bytecode,
        "CompileApplication",
        BTreeMap::from([
            ("buildrequest.id".to_string(), "BR-1".to_string()),
            (
                "buildrequest.status".to_string(),
                "SpecCaptured".to_string(),
            ),
            (
                "buildrequest.requirements".to_string(),
                "support ticket requirements".to_string(),
            ),
            (
                "buildrequest.spec".to_string(),
                "checked support ticket spec".to_string(),
            ),
        ]),
    )
    .unwrap();

    assert_eq!(run.status, "succeeded");
    assert_eq!(run.final_state["buildrequest.status"], "BytecodeReady");
    assert!(
        run.trace
            .contains(&"trace ApplicationBytecodeCompiled".to_string())
    );

    let verify_run = run_ail_bytecode_action(
        &bytecode,
        "VerifyBytecodeArtifact",
        BTreeMap::from([
            ("buildrequest.id".to_string(), "BR-1".to_string()),
            (
                "buildrequest.status".to_string(),
                "BytecodeReady".to_string(),
            ),
            (
                "buildrequest.bytecode artifact".to_string(),
                "Verified AIL-Bytecode".to_string(),
            ),
        ]),
    )
    .unwrap();

    assert_eq!(verify_run.status, "succeeded");
    assert_eq!(
        verify_run.final_state["buildrequest.bytecode verification report"],
        "Verified"
    );
    assert!(
        verify_run
            .trace
            .contains(&"trace BytecodeArtifactVerified".to_string())
    );

    let native_compile_run = run_ail_bytecode_action(
        &bytecode,
        "CompileNativeTarget",
        BTreeMap::from([
            ("buildrequest.id".to_string(), "BR-1".to_string()),
            (
                "buildrequest.status".to_string(),
                "BytecodeReady".to_string(),
            ),
            (
                "buildrequest.bytecode artifact".to_string(),
                "Verified AIL-Bytecode".to_string(),
            ),
            (
                "buildrequest.bytecode fingerprint".to_string(),
                "fnv64:bytecode".to_string(),
            ),
            (
                "buildrequest.target platform".to_string(),
                "linux-x86_64-elf".to_string(),
            ),
            (
                "buildrequest.target artifact".to_string(),
                "linux-x86_64-elf executable 512 bytes".to_string(),
            ),
            (
                "buildrequest.target artifact fingerprint".to_string(),
                "fnv64:target".to_string(),
            ),
        ]),
    )
    .unwrap();

    assert_eq!(native_compile_run.status, "succeeded");
    assert_eq!(
        native_compile_run.final_state["buildrequest.target artifact compilation report"],
        "Emitted"
    );
    assert!(
        native_compile_run
            .trace
            .contains(&"read buildrequest.bytecode artifact".to_string())
    );
    assert!(
        native_compile_run
            .trace
            .contains(&"read buildrequest.bytecode fingerprint".to_string())
    );
    assert!(
        native_compile_run
            .trace
            .contains(&"read buildrequest.target platform".to_string())
    );
    assert!(
        native_compile_run
            .trace
            .contains(&"trace NativeTargetCompiled".to_string())
    );

    let target_verify_run = run_ail_bytecode_action(
        &bytecode,
        "VerifyTargetArtifact",
        BTreeMap::from([
            ("buildrequest.id".to_string(), "BR-1".to_string()),
            (
                "buildrequest.status".to_string(),
                "BytecodeReady".to_string(),
            ),
            (
                "buildrequest.target artifact".to_string(),
                "linux-x86_64-elf executable 512 bytes".to_string(),
            ),
            (
                "buildrequest.target artifact fingerprint".to_string(),
                "fnv64:target".to_string(),
            ),
        ]),
    )
    .unwrap();

    assert_eq!(target_verify_run.status, "succeeded");
    assert_eq!(
        target_verify_run.final_state["buildrequest.target artifact verification report"],
        "Verified"
    );
    assert!(
        target_verify_run
            .trace
            .contains(&"trace TargetArtifactVerified".to_string())
    );

    let manifest_run = run_ail_bytecode_action(
        &bytecode,
        "VerifyBuildManifest",
        BTreeMap::from([
            ("buildrequest.id".to_string(), "BR-1".to_string()),
            (
                "buildrequest.status".to_string(),
                "BytecodeReady".to_string(),
            ),
            (
                "buildrequest.artifact manifest".to_string(),
                "AIL-Build-Manifest:\nbytecode artifact.ailbc.json fnv64:1234".to_string(),
            ),
            (
                "buildrequest.artifact manifest fingerprint".to_string(),
                "fnv64:manifest".to_string(),
            ),
            (
                "buildrequest.machine bytecode contract".to_string(),
                "machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable".to_string(),
            ),
        ]),
    )
    .unwrap();

    assert_eq!(manifest_run.status, "succeeded");
    assert_eq!(
        manifest_run.final_state["buildrequest.artifact manifest verification report"],
        "Verified"
    );
    assert!(
        manifest_run
            .trace
            .contains(&"trace BuildManifestVerified".to_string())
    );
    assert!(
        manifest_run
            .trace
            .contains(&"read buildrequest.machine bytecode contract".to_string())
    );

    let compile_manifest_run = run_ail_bytecode_action(
        &bytecode,
        "VerifyCompileManifest",
        BTreeMap::from([
            ("buildrequest.id".to_string(), "BR-1".to_string()),
            (
                "buildrequest.status".to_string(),
                "BytecodeReady".to_string(),
            ),
            (
                "buildrequest.bytecode fingerprint".to_string(),
                "fnv64:bytecode".to_string(),
            ),
            (
                "buildrequest.target artifact".to_string(),
                "linux-x86_64-elf executable 512 bytes".to_string(),
            ),
            (
                "buildrequest.target artifact fingerprint".to_string(),
                "fnv64:target".to_string(),
            ),
            (
                "buildrequest.artifact manifest".to_string(),
                "AIL-Compile-Manifest:\nbytecode artifact.ailbc.json fnv64:bytecode".to_string(),
            ),
            (
                "buildrequest.artifact manifest fingerprint".to_string(),
                "fnv64:manifest".to_string(),
            ),
            (
                "buildrequest.machine bytecode contract".to_string(),
                "machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable".to_string(),
            ),
        ]),
    )
    .unwrap();

    assert_eq!(compile_manifest_run.status, "succeeded");
    assert_eq!(
        compile_manifest_run.final_state["buildrequest.artifact manifest verification report"],
        "Verified"
    );
    assert!(
        compile_manifest_run
            .trace
            .contains(&"trace CompileManifestVerified".to_string())
    );
    assert!(
        compile_manifest_run
            .trace
            .contains(&"read buildrequest.machine bytecode contract".to_string())
    );

    let pass_manifest_run = run_ail_bytecode_action(
        &bytecode,
        "VerifyPassManifest",
        BTreeMap::from([
            ("buildrequest.id".to_string(), "BR-1".to_string()),
            ("buildrequest.status".to_string(), "PassApplied".to_string()),
            (
                "buildrequest.artifact manifest".to_string(),
                "AIL-Pass-Manifest:\ncompiler-pass pass.ailbc.json fnv64:1234".to_string(),
            ),
            (
                "buildrequest.artifact manifest fingerprint".to_string(),
                "fnv64:manifest".to_string(),
            ),
            (
                "buildrequest.compiler pass fingerprint".to_string(),
                "fnv64:pass".to_string(),
            ),
            (
                "buildrequest.machine bytecode contract".to_string(),
                "machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable".to_string(),
            ),
        ]),
    )
    .unwrap();

    assert_eq!(pass_manifest_run.status, "succeeded");
    assert_eq!(
        pass_manifest_run.final_state["buildrequest.artifact manifest verification report"],
        "Verified"
    );
    assert!(
        pass_manifest_run
            .trace
            .contains(&"trace PassManifestVerified".to_string())
    );
    assert!(
        pass_manifest_run
            .trace
            .contains(&"read buildrequest.machine bytecode contract".to_string())
    );
}

#[test]
fn cli_ail_vm_executes_saved_bytecode_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let lowered = Command::new(binary)
        .args(["ail-lower", &package])
        .output()
        .unwrap();
    assert!(
        lowered.status.success(),
        "{}",
        String::from_utf8_lossy(&lowered.stderr)
    );

    let suffix = format!("{}-ail-bytecode-artifact", std::process::id());
    let bytecode_path = std::env::temp_dir().join(format!("{suffix}.ailbc.json"));
    fs::write(&bytecode_path, lowered.stdout).unwrap();

    let success = Command::new(binary)
        .args([
            "ail-vm",
            bytecode_path.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "ticket.id=T-1",
            "ticket.status=Open",
            "ticket.internal notes=sensitive note",
        ])
        .output()
        .unwrap();
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let success_stdout = String::from_utf8_lossy(&success.stdout);
    assert!(success_stdout.contains("ail-vm succeeded"));
    assert!(success_stdout.contains("ticket.status=Closed"));
    assert!(success_stdout.contains("trace=action CloseTicket started"));
    assert!(success_stdout.contains("trace TicketClosed"));

    let missing = Command::new(binary)
        .args([
            "ail-vm",
            bytecode_path.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "ticket.status=Open",
        ])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    let missing_stdout = String::from_utf8_lossy(&missing.stdout);
    assert!(missing_stdout.contains("ail-vm failed"));
    assert!(missing_stdout.contains("failure=NotFound"));
    assert!(missing_stdout.contains("trace TicketNotFound"));
}

#[test]
fn cli_ail_vm_rejects_invalid_bytecode_before_execution() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let lowered = Command::new(binary)
        .args(["ail-lower", &package])
        .output()
        .unwrap();
    assert!(
        lowered.status.success(),
        "{}",
        String::from_utf8_lossy(&lowered.stderr)
    );

    let invalid_bytecode = String::from_utf8(lowered.stdout).unwrap().replacen(
        r#""opcode":"SET_FIELD""#,
        r#""opcode":"SET_FIELD_BOGUS""#,
        1,
    );
    let suffix = format!("{}-invalid-ail-bytecode-artifact", std::process::id());
    let bytecode_path = std::env::temp_dir().join(format!("{suffix}.ailbc.json"));
    fs::write(&bytecode_path, invalid_bytecode).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-vm",
            bytecode_path.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "ticket.id=T-1",
            "ticket.status=Open",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail-vm diagnostics:"), "{stdout}");
    assert!(stdout.contains("AILBC001"), "{stdout}");
    assert!(stdout.contains("SET_FIELD_BOGUS"), "{stdout}");
    assert!(!stdout.contains("ail-vm succeeded"), "{stdout}");
    assert!(!stdout.contains("ticket.status=Closed"), "{stdout}");
}

#[test]
fn cli_ail_pass_runs_compiler_pass_over_checked_package_core() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let pass_package = fixture("compiler_pass.ail");
    let target_package = fixture("support_ticket.ail");

    let output = Command::new(binary)
        .args([
            "ail-pass",
            &pass_package,
            &target_package,
            "--action",
            "InferReadPermissions",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("package: support-ticket"), "{stdout}");
    assert!(
        stdout.contains("node Permission read Ticket.status"),
        "{stdout}"
    );
    assert!(
        stdout
            .contains("edge requires Action:MarksOverdueTickets -> Permission:read Ticket.status"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "node Provenance compiler_pass:InferReadPermissions.permission:read Ticket.status"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("trace="), "{stdout}");
}

#[test]
fn cli_ail_pass_writes_auditable_intermediate_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let pass_package = fixture("compiler_pass.ail");
    let target_package = fixture("support_ticket.ail");
    let artifact_dir =
        std::env::temp_dir().join(format!("ail-ail-pass-artifacts-{}", std::process::id()));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-pass",
            &pass_package,
            &target_package,
            "--action",
            "InferReadPermissions",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let pass_bytecode = fs::read_to_string(artifact_dir.join("pass.ailbc.json")).unwrap();
    let pass_source_manifest =
        fs::read_to_string(artifact_dir.join("compiler-pass.source.ail-package.md")).unwrap();
    assert_eq!(
        pass_source_manifest,
        fs::read_to_string(format!("{pass_package}/ail-package.md")).unwrap()
    );
    let pass_source_spec =
        fs::read_to_string(artifact_dir.join("compiler-pass.source.ail-spec.md")).unwrap();
    assert_eq!(
        pass_source_spec,
        fs::read_to_string(format!("{pass_package}/spec.ail-spec.md")).unwrap()
    );
    let pass_source_bundle =
        format!("ail-package.md:\n{pass_source_manifest}\nspec.ail-spec.md:\n{pass_source_spec}");
    let pass_source_fingerprint =
        fs::read_to_string(artifact_dir.join("compiler-pass.source.fingerprint.txt")).unwrap();
    assert_eq!(
        pass_source_fingerprint.trim(),
        fnv64_fingerprint(&pass_source_bundle)
    );
    let target_source_manifest =
        fs::read_to_string(artifact_dir.join("target.source.ail-package.md")).unwrap();
    assert_eq!(
        target_source_manifest,
        fs::read_to_string(format!("{target_package}/ail-package.md")).unwrap()
    );
    let target_source_spec =
        fs::read_to_string(artifact_dir.join("target.source.ail-spec.md")).unwrap();
    assert_eq!(
        target_source_spec,
        fs::read_to_string(format!("{target_package}/spec.ail-spec.md")).unwrap()
    );
    let target_source_bundle = format!(
        "ail-package.md:\n{target_source_manifest}\nspec.ail-spec.md:\n{target_source_spec}"
    );
    let target_source_fingerprint =
        fs::read_to_string(artifact_dir.join("target.source.fingerprint.txt")).unwrap();
    assert_eq!(
        target_source_fingerprint.trim(),
        fnv64_fingerprint(&target_source_bundle)
    );
    let pass_fingerprint = fs::read_to_string(artifact_dir.join("pass.fingerprint.txt")).unwrap();
    let input_core = fs::read_to_string(artifact_dir.join("input.ail-core.txt")).unwrap();
    let output_core = fs::read_to_string(artifact_dir.join("output.ail-core.txt")).unwrap();
    let trace = fs::read_to_string(artifact_dir.join("trace.txt")).unwrap();
    let expected_pass_fingerprint = fnv64_fingerprint(&pass_bytecode);

    assert_eq!(output_core, stdout);
    assert_eq!(pass_fingerprint.trim(), expected_pass_fingerprint);
    assert!(pass_bytecode.contains(r#""package":"ail-meta-permissions""#));
    assert!(pass_bytecode.contains(r#""opcode":"CORE_INFER_READ_PERMISSIONS""#));
    assert!(!input_core.contains("node Permission read Ticket.status"));
    assert!(output_core.contains("node Permission read Ticket.status"));
    assert!(trace.contains("compiler pass Infer read permissions started"));
    assert!(trace.contains("core transform infer read permissions"));
    assert!(
        trace.contains("compiler pass InferReadPermissions added Permission read Ticket.status")
    );

    let parsed_bytecode = parse_ail_bytecode(&pass_bytecode).unwrap();
    assert_eq!(verify_ail_bytecode(&parsed_bytecode), Vec::<String>::new());

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-pass.txt")).unwrap();
    assert!(manifest.contains("AIL-Pass-Manifest:"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "compiler-pass-source compiler-pass.source.ail-package.md compiler-pass.source.ail-spec.md {}",
            fnv64_fingerprint(&pass_source_bundle)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "target-source target.source.ail-package.md target.source.ail-spec.md {}",
            fnv64_fingerprint(&target_source_bundle)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "compiler-pass pass.ailbc.json {expected_pass_fingerprint}"
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains("core-input input.ail-core.txt"),
        "{manifest}"
    );
    assert!(
        manifest.contains("core-output output.ail-core.txt"),
        "{manifest}"
    );
    assert!(manifest.contains("trace trace.txt"), "{manifest}");
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(&artifact_dir).unwrap();
}

#[test]
fn cli_ail_pass_agent_accepts_pass_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let pass_package = fixture("compiler_pass.ail");
    let target_package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-pass-agent-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-pass",
            &pass_package,
            &target_package,
            "--action",
            "InferReadPermissions",
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("node Permission read Ticket.status"),
        "{stdout}"
    );

    let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    assert!(agent_bytecode.contains(r#""package":"ail-toolchain-agent""#));
    assert!(agent_bytecode.contains(r#""action":"AcceptCompilerPassOutput""#));
    let agent_fingerprint = fs::read_to_string(artifact_dir.join("agent.fingerprint.txt")).unwrap();
    assert_eq!(agent_fingerprint.trim(), fnv64_fingerprint(&agent_bytecode));
    let parsed_agent = parse_ail_bytecode(&agent_bytecode).unwrap();
    assert_eq!(verify_ail_bytecode(&parsed_agent), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action AcceptCompilerPassOutput started"));
    assert!(agent_trace.contains("read buildrequest.core ir"));
    assert!(agent_trace.contains("read buildrequest.compiler pass artifact"));
    assert!(agent_trace.contains("read buildrequest.compiler pass fingerprint"));
    assert!(agent_trace.contains("read buildrequest.compiler pass trace"));
    assert!(agent_trace.contains("write buildrequest.compiler pass review report=Accepted"));
    assert!(agent_trace.contains("write buildrequest.status=PassApplied"));
    assert!(agent_trace.contains("trace CompilerPassOutputAccepted"));
    let accept_index = agent_trace
        .find("action AcceptCompilerPassOutput started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let manifest_index = agent_trace
        .find("action VerifyPassManifest started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(accept_index < manifest_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.artifact manifest"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest fingerprint"));
    assert!(agent_trace.contains("read buildrequest.compiler pass source package"));
    assert!(agent_trace.contains("read buildrequest.compiler pass source package fingerprint"));
    assert!(agent_trace.contains("read buildrequest.source package"));
    assert!(agent_trace.contains("read buildrequest.source package fingerprint"));
    assert!(
        agent_trace.contains("write buildrequest.artifact manifest verification report=Verified")
    );
    assert!(agent_trace.contains("trace PassManifestVerified"));

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-pass.txt")).unwrap();
    let pass_source_manifest =
        fs::read_to_string(artifact_dir.join("compiler-pass.source.ail-package.md")).unwrap();
    let pass_source_spec =
        fs::read_to_string(artifact_dir.join("compiler-pass.source.ail-spec.md")).unwrap();
    let pass_source_bundle =
        format!("ail-package.md:\n{pass_source_manifest}\nspec.ail-spec.md:\n{pass_source_spec}");
    let target_source_manifest =
        fs::read_to_string(artifact_dir.join("target.source.ail-package.md")).unwrap();
    let target_source_spec =
        fs::read_to_string(artifact_dir.join("target.source.ail-spec.md")).unwrap();
    let target_source_bundle = format!(
        "ail-package.md:\n{target_source_manifest}\nspec.ail-spec.md:\n{target_source_spec}"
    );
    assert!(
        manifest.contains(&format!(
            "compiler-pass-source compiler-pass.source.ail-package.md compiler-pass.source.ail-spec.md {}",
            fnv64_fingerprint(&pass_source_bundle)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "target-source target.source.ail-package.md target.source.ail-spec.md {}",
            fnv64_fingerprint(&target_source_bundle)
        )),
        "{manifest}"
    );
    assert!(manifest.contains("agent agent.ailbc.json"), "{manifest}");
    assert!(manifest.contains("trace agent-trace.txt"), "{manifest}");
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));
    assert!(agent_bytecode.contains(r#""action":"VerifyPassManifest""#));

    fs::remove_dir_all(&artifact_dir).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_pass_writes_native_tool_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let pass_package = fixture("compiler_pass.ail");
    let target_package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-pass-native-tool-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-pass",
            &pass_package,
            &target_package,
            "--action",
            "InferReadPermissions",
            "--agent",
            &agent_package,
            "--target",
            "linux-x86_64-elf",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let pass_native = fs::read(artifact_dir.join("pass-InferReadPermissions.elf")).unwrap();
    assert_eq!(&pass_native[0..4], b"\x7fELF");
    let expected_pass_native_fingerprint = fnv64_fingerprint_bytes(&pass_native);
    let pass_run = Command::new(artifact_dir.join("pass-InferReadPermissions.elf"))
        .arg("input graph=checked")
        .arg("package policy=default")
        .output()
        .unwrap();
    assert!(pass_run.status.success(), "native pass executable failed");
    assert!(
        String::from_utf8_lossy(&pass_run.stderr).contains("trace ReadPermissionAdded"),
        "{}",
        String::from_utf8_lossy(&pass_run.stderr)
    );

    let agent_native = fs::read(artifact_dir.join("agent-AcceptCompilerPassOutput.elf")).unwrap();
    assert_eq!(&agent_native[0..4], b"\x7fELF");
    let expected_agent_native_fingerprint = fnv64_fingerprint_bytes(&agent_native);
    let agent_run = Command::new(artifact_dir.join("agent-AcceptCompilerPassOutput.elf"))
        .args([
            "buildrequest.id=ail-pass",
            "buildrequest.developer prompt=skipped",
            "buildrequest.requirements=skipped",
            "buildrequest.spec=skipped",
            "buildrequest.core ir=ok",
            "buildrequest.compiler pass artifact=ok",
            "buildrequest.compiler pass fingerprint=fnv64:test",
            "buildrequest.compiler pass trace=ok",
            "buildrequest.status=CoreLoaded",
        ])
        .output()
        .unwrap();
    assert!(
        agent_run.status.success(),
        "native agent pass-acceptance executable failed"
    );
    assert!(
        String::from_utf8_lossy(&agent_run.stderr).contains("trace CompilerPassOutputAccepted"),
        "{}",
        String::from_utf8_lossy(&agent_run.stderr)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-pass.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "compiler-pass-target linux-x86_64-elf pass-InferReadPermissions.elf {expected_pass_native_fingerprint}"
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "agent-target linux-x86_64-elf agent-AcceptCompilerPassOutput.elf {expected_agent_native_fingerprint}"
        )),
        "{manifest}"
    );
    let native_bytecode_report =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.txt")).unwrap();
    assert!(
        native_bytecode_report.contains("AIL-Pass-Native-Bytecode:"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("target linux-x86_64-elf"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains(&format!(
            "machine-bytecode compiler-pass-target linux-x86_64-elf pass-InferReadPermissions.elf elf64-little-x86_64-executable {expected_pass_native_fingerprint} bytes {}",
            pass_native.len()
        )),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains(&format!(
            "machine-bytecode agent-target linux-x86_64-elf agent-AcceptCompilerPassOutput.elf elf64-little-x86_64-executable {expected_agent_native_fingerprint} bytes {}",
            agent_native.len()
        )),
        "{native_bytecode_report}"
    );
    let native_bytecode_report_fingerprint =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.fingerprint.txt")).unwrap();
    assert_eq!(
        native_bytecode_report_fingerprint.trim(),
        fnv64_fingerprint(&native_bytecode_report)
    );
    assert!(
        manifest.contains(&format!(
            "native-bytecode native-bytecode-report.txt {}",
            fnv64_fingerprint(&native_bytecode_report)
        )),
        "{manifest}"
    );
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Pass-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("target linux-x86_64-elf"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("host-language-runtime none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("dynamic-linker none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("shared-libraries none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("library-dependencies none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("linker-invocation none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains(
            "machine-bytecode-dependency pass-InferReadPermissions.elf standalone-linux-syscall-elf"
        ),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains(
            "machine-bytecode-dependency agent-AcceptCompilerPassOutput.elf standalone-linux-syscall-elf"
        ),
        "{dependency_report}"
    );
    let dependency_report_fingerprint =
        fs::read_to_string(artifact_dir.join("dependency-report.fingerprint.txt")).unwrap();
    assert_eq!(
        dependency_report_fingerprint.trim(),
        fnv64_fingerprint(&dependency_report)
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action VerifyPassManifest started"));
    assert!(agent_trace.contains("read buildrequest.machine bytecode contract"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.dependency report"));
    assert!(agent_trace.contains("read buildrequest.dependency report fingerprint"));

    fs::remove_dir_all(&artifact_dir).unwrap();
}

#[test]
fn cli_ail_pass_accepts_saved_compiler_pass_bytecode_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let pass_package = fixture("compiler_pass.ail");
    let target_package = fixture("support_ticket.ail");

    let lowered = Command::new(binary)
        .args(["ail-lower", &pass_package])
        .output()
        .unwrap();
    assert!(
        lowered.status.success(),
        "{}",
        String::from_utf8_lossy(&lowered.stderr)
    );

    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-compiler-pass-{}.ailbc.json",
        std::process::id()
    ));
    fs::write(&bytecode_path, lowered.stdout).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-pass",
            bytecode_path.to_str().unwrap(),
            &target_package,
            "--action",
            "InferReadPermissions",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("package: support-ticket"), "{stdout}");
    assert!(
        stdout.contains("node Permission read Ticket.status"),
        "{stdout}"
    );
    assert!(
        stdout
            .contains("edge requires Action:MarksOverdueTickets -> Permission:read Ticket.status"),
        "{stdout}"
    );

    fs::remove_file(bytecode_path).unwrap();
}

#[test]
fn cli_ail_pass_accepts_saved_core_file_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let pass_package = fixture("compiler_pass.ail");
    let target_package = fixture("support_ticket.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-pass-core-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let lowered = Command::new(binary)
        .args(["ail-lower", &pass_package])
        .output()
        .unwrap();
    assert!(
        lowered.status.success(),
        "{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-compiler-pass-core-target-{}.ailbc.json",
        std::process::id()
    ));
    fs::write(&bytecode_path, lowered.stdout).unwrap();

    let target_core = Command::new(binary)
        .args(["ail-core", &target_package])
        .output()
        .unwrap();
    assert!(
        target_core.status.success(),
        "{}",
        String::from_utf8_lossy(&target_core.stderr)
    );
    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-pass-input-{}.ail-core.txt",
        std::process::id()
    ));
    fs::write(&core_path, target_core.stdout).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-pass",
            bytecode_path.to_str().unwrap(),
            "--core-file",
            core_path.to_str().unwrap(),
            "--action",
            "InferReadPermissions",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("package: support-ticket"), "{stdout}");
    assert!(
        stdout.contains("node Permission read Ticket.status"),
        "{stdout}"
    );
    assert!(
        stdout
            .contains("edge requires Action:MarksOverdueTickets -> Permission:read Ticket.status"),
        "{stdout}"
    );

    let input_core = fs::read_to_string(artifact_dir.join("input.ail-core.txt")).unwrap();
    let output_core = fs::read_to_string(artifact_dir.join("output.ail-core.txt")).unwrap();
    let trace = fs::read_to_string(artifact_dir.join("trace.txt")).unwrap();
    assert_eq!(
        input_core,
        fs::read_to_string(&core_path)
            .unwrap()
            .trim_end_matches('\n')
            .to_string()
            + "\n"
    );
    assert_eq!(output_core, stdout);
    assert!(trace.contains("core transform infer read permissions"));

    fs::remove_file(bytecode_path).unwrap();
    fs::remove_file(core_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn ail_runtime_executes_generic_field_writes_and_requirements() {
    let package = load_ail_package_dir(fixture("runtime_generic.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let success = run_ail_action(
        &document,
        "PrioritizeTicket",
        BTreeMap::from([
            ("ticket.id".to_string(), "T-1".to_string()),
            ("ticket.priority".to_string(), "Low".to_string()),
            ("supportticket.priority".to_string(), "Low".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(success.status, "succeeded");
    assert_eq!(success.final_state["ticket.priority"], "High");
    assert_eq!(success.final_state["supportticket.priority"], "Low");
    assert!(
        success
            .trace
            .contains(&"rule passed: the ticket to exist".to_string())
    );
    assert!(
        success
            .trace
            .contains(&"rule passed: the ticket priority not to be High".to_string())
    );
    assert!(
        success
            .trace
            .contains(&"write ticket.priority=High".to_string())
    );
    assert!(
        success
            .trace
            .contains(&"trace TicketPrioritized".to_string())
    );

    let already_high = run_ail_action(
        &document,
        "PrioritizeTicket",
        BTreeMap::from([
            ("ticket.id".to_string(), "T-1".to_string()),
            ("ticket.priority".to_string(), "High".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(already_high.status, "failed");
    assert_eq!(already_high.failure.as_deref(), Some("RequirementFailed"));
    assert_eq!(
        already_high
            .final_state
            .get("ticket.priority")
            .map(String::as_str),
        Some("High")
    );
}

#[test]
fn ail_runtime_enforces_positive_field_requirements_and_read_traces() {
    let package = load_ail_package_dir(fixture("secret_access.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let success = run_ail_action(
        &document,
        "ViewInternalNotes",
        BTreeMap::from([
            ("ticket.id".to_string(), "T-1".to_string()),
            ("ticket.internal notes".to_string(), "[private]".to_string()),
            ("requester.role".to_string(), "SupportAgent".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(success.status, "succeeded");
    assert_eq!(success.failure, None);
    assert!(
        success
            .trace
            .contains(&"rule passed: the ticket to exist".to_string())
    );
    assert!(success.trace.contains(
        &"rule passed: the requester role to be SupportAgent or SupportManager".to_string()
    ));
    assert!(
        success
            .trace
            .contains(&"read ticket.internal notes".to_string())
    );
    assert!(
        success
            .trace
            .contains(&"trace InternalNotesViewed".to_string())
    );

    let denied = run_ail_action(
        &document,
        "ViewInternalNotes",
        BTreeMap::from([
            ("ticket.id".to_string(), "T-1".to_string()),
            ("ticket.internal notes".to_string(), "[private]".to_string()),
            ("requester.role".to_string(), "Customer".to_string()),
        ]),
    )
    .unwrap();

    assert_eq!(denied.status, "failed");
    assert_eq!(denied.failure.as_deref(), Some("PermissionDenied"));
    assert!(
        denied
            .trace
            .contains(&"failure PermissionDenied".to_string())
    );
    assert!(
        denied
            .trace
            .contains(&"trace InternalNotesDenied".to_string())
    );
}

#[test]
fn cli_ail_check_and_core_use_package_loader() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");

    let check = Command::new(binary)
        .args(["ail-check", &package])
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let check_stdout = String::from_utf8_lossy(&check.stdout);
    assert!(check_stdout.contains("ail diagnostics: none"));
    assert!(check_stdout.contains(DEFAULT_BASE_LLM_ENDPOINT));

    let core = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core.status.success(),
        "{}",
        String::from_utf8_lossy(&core.stderr)
    );
    let core_stdout = String::from_utf8_lossy(&core.stdout);
    assert!(core_stdout.contains("package: support-ticket"));
    assert!(core_stdout.contains("node Action CloseTicket"));
    assert!(core_stdout.contains("node Trace TicketClosed"));

    let flow = Command::new(binary)
        .args(["ail-flow", &package])
        .output()
        .unwrap();
    assert!(
        flow.status.success(),
        "{}",
        String::from_utf8_lossy(&flow.stderr)
    );
    let flow_stdout = String::from_utf8_lossy(&flow.stdout);
    assert!(flow_stdout.contains(r#""kind":"AIL-Flow""#));
    assert!(flow_stdout.contains(r#""application":"Support Tickets""#));
    let expected_flow_hash = ail_core_hash(&parse_ail_core_text(&core_stdout).unwrap());
    assert!(
        flow_stdout.contains(&format!(r#""coreHash":"{expected_flow_hash}""#)),
        "{flow_stdout}"
    );
    assert!(flow_stdout.contains(r#""name":"CloseTicket","coreLabel":"Action:CloseTicket""#));
    assert!(
        flow_stdout.contains(
            r#"{"kind":"records_trace","source":"Action:CloseTicket","target":"Trace:TicketClosed","targetName":"TicketClosed","attributes":{}}"#
        ),
        "{flow_stdout}"
    );

    let lowered = Command::new(binary)
        .args(["ail-lower", &package])
        .output()
        .unwrap();
    assert!(
        lowered.status.success(),
        "{}",
        String::from_utf8_lossy(&lowered.stderr)
    );
    let lowered_stdout = String::from_utf8_lossy(&lowered.stdout);
    assert!(lowered_stdout.contains(r#""kind":"AIL-Bytecode""#));
    assert!(lowered_stdout.contains(r#""action":"CloseTicket""#));
    assert!(lowered_stdout.contains(r#""opcode":"SET_FIELD""#));

    let patch_path = format!("{package}/examples/patches/escalate-ticket.ail-patch.md");
    let patched = Command::new(binary)
        .args(["ail-patch", &package, &patch_path])
        .output()
        .unwrap();
    assert!(
        patched.status.success(),
        "{}",
        String::from_utf8_lossy(&patched.stderr)
    );
    let patched_stdout = String::from_utf8_lossy(&patched.stdout);
    assert!(patched_stdout.contains("- escalation reason: Text"));
    assert!(patched_stdout.contains("Action: Escalate ticket."));
    assert!(patched_stdout.contains("TicketEscalated"));

    let core_patch_path = std::env::temp_dir().join(format!(
        "ail-flow-require-open-status-{}.ail-core.patch.json",
        std::process::id()
    ));
    let core_path = std::env::temp_dir().join(format!(
        "ail-flow-require-open-status-{}.ail-core.txt",
        std::process::id()
    ));
    let core_output = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "{}",
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    let core_hash = ail_core_hash(&parse_ail_core_text(&core_text).unwrap());
    fs::write(&core_path, &core_text).unwrap();

    let stale_patch_path = std::env::temp_dir().join(format!(
        "ail-flow-stale-base-{}.ail-core.patch.json",
        std::process::id()
    ));
    fs::write(
        &stale_patch_path,
        r#"{
  "schema": "ail-core.patch.v0",
  "base_hash": "ail-core:fnv64:0000000000000000",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {
      "op": "add_node",
      "kind": "Rule",
      "name": "the ticket status to be Open",
      "provenance": ["flow:ActionCard:CloseTicket.requirement:open-status"]
    }
  ]
}"#,
    )
    .unwrap();
    let stale_patch = Command::new(binary)
        .args([
            "ail-patch",
            "--core-file",
            core_path.to_str().unwrap(),
            stale_patch_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !stale_patch.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&stale_patch.stdout),
        String::from_utf8_lossy(&stale_patch.stderr)
    );
    assert!(
        String::from_utf8_lossy(&stale_patch.stderr).contains("AIL-Core patch base_hash mismatch"),
        "stderr:\n{}",
        String::from_utf8_lossy(&stale_patch.stderr)
    );
    fs::write(
        &core_patch_path,
        format!(
            r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "add_node",
      "kind": "Rule",
      "name": "the ticket status to be Open",
      "provenance": ["flow:ActionCard:CloseTicket.requirement:open-status"]
    }},
    {{
      "op": "add_edge",
      "kind": "requires",
      "source": "Action:CloseTicket",
      "target": "Rule:the ticket status to be Open",
      "provenance": ["flow:ActionCard:CloseTicket.requirement:open-status"]
    }}
  ]
}}"#
        ),
    )
    .unwrap();

    let patched_core = Command::new(binary)
        .args([
            "ail-patch",
            "--core-file",
            core_path.to_str().unwrap(),
            core_patch_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        patched_core.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&patched_core.stdout),
        String::from_utf8_lossy(&patched_core.stderr)
    );
    let patched_core_text = String::from_utf8(patched_core.stdout).unwrap();
    let patched_core_artifact = parse_ail_core_text(&patched_core_text).unwrap();
    assert_eq!(check_ail_core(&patched_core_artifact), Vec::<String>::new());
    let patched_spec = render_ail_spec_from_core(&patched_core_artifact);
    assert!(
        patched_spec.contains("- the system requires the ticket status to be Open"),
        "{patched_spec}"
    );

    let replace_patch_path = std::env::temp_dir().join(format!(
        "ail-flow-rename-close-ticket-{}.ail-core.patch.json",
        std::process::id()
    ));
    fs::write(
        &replace_patch_path,
        format!(
            r#"{{
  "schema": "ail-core.patch.v0",
  "base_hash": "{core_hash}",
  "source_view": "ActionCard:CloseTicket",
  "ops": [
    {{
      "op": "replace_node_attributes",
      "target": "Action:CloseTicket",
      "attributes": {{
        "label": "Resolve ticket"
      }},
      "provenance": ["flow:ActionCard:CloseTicket.label"]
    }}
  ]
}}"#
        ),
    )
    .unwrap();
    let relabeled_core = Command::new(binary)
        .args([
            "ail-patch",
            "--core-file",
            core_path.to_str().unwrap(),
            replace_patch_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        relabeled_core.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&relabeled_core.stdout),
        String::from_utf8_lossy(&relabeled_core.stderr)
    );
    let relabeled_core_text = String::from_utf8(relabeled_core.stdout).unwrap();
    let relabeled_core_artifact = parse_ail_core_text(&relabeled_core_text).unwrap();
    assert_eq!(
        check_ail_core(&relabeled_core_artifact),
        Vec::<String>::new()
    );
    let relabeled_spec = render_ail_spec_from_core(&relabeled_core_artifact);
    assert!(
        relabeled_spec.contains("Action: Resolve ticket."),
        "{relabeled_spec}"
    );
    assert!(
        relabeled_spec.contains("- the system records a trace event named TicketClosed"),
        "{relabeled_spec}"
    );

    fs::remove_file(stale_patch_path).unwrap();
    fs::remove_file(replace_patch_path).unwrap();
    fs::remove_file(core_patch_path).unwrap();
    fs::remove_file(core_path).unwrap();

    let composed = fixture("support_composed.ail");
    let composed_core = Command::new(binary)
        .args(["ail-core", &composed])
        .output()
        .unwrap();
    assert!(
        composed_core.status.success(),
        "{}",
        String::from_utf8_lossy(&composed_core.stderr)
    );
    let composed_core_stdout = String::from_utf8_lossy(&composed_core.stdout);
    assert!(composed_core_stdout.contains("node Thing Shared.User"));
    assert!(composed_core_stdout.contains("node Field Shared.User.email : Text"));

    let runtime_package = fixture("runtime_generic.ail");
    let runtime = Command::new(binary)
        .args([
            "ail-run",
            &runtime_package,
            "--action",
            "PrioritizeTicket",
            "ticket.id=T-1",
            "ticket.priority=Low",
        ])
        .output()
        .unwrap();
    assert!(
        runtime.status.success(),
        "{}",
        String::from_utf8_lossy(&runtime.stderr)
    );
    let runtime_stdout = String::from_utf8_lossy(&runtime.stdout);
    assert!(runtime_stdout.contains("ail-run succeeded"));
    assert!(runtime_stdout.contains("ticket.priority=High"));
    assert!(runtime_stdout.contains("trace=action PrioritizeTicket started"));

    let secret_access_package = fixture("secret_access.ail");
    let secret_access = Command::new(binary)
        .args([
            "ail-run",
            &secret_access_package,
            "--action",
            "ViewInternalNotes",
            "ticket.id=T-1",
            "ticket.internal notes=[private]",
            "requester.role=Customer",
        ])
        .output()
        .unwrap();
    assert_eq!(secret_access.status.code(), Some(1));
    let secret_access_stdout = String::from_utf8_lossy(&secret_access.stdout);
    assert!(secret_access_stdout.contains("ail-run failed"));
    assert!(secret_access_stdout.contains("failure=PermissionDenied"));
    assert!(secret_access_stdout.contains("ticket.internal notes=<secret>"));
    assert!(!secret_access_stdout.contains("[private]"));
    assert!(secret_access_stdout.contains("trace=action ViewInternalNotes started"));
    assert!(secret_access_stdout.contains("failure PermissionDenied"));
}

#[test]
fn cli_ail_core_and_lower_accept_saved_spec_file_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let spec_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-generated-{}.ail-spec.md",
        std::process::id()
    ));
    fs::write(
        &spec_path,
        fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap(),
    )
    .unwrap();

    let core_output = Command::new(binary)
        .args([
            "ail-core",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_stdout = String::from_utf8_lossy(&core_output.stdout);
    assert!(core_stdout.contains("package: support-ticket"));
    assert!(core_stdout.contains("node Action CloseTicket"));

    let lower_output = Command::new(binary)
        .args([
            "ail-lower",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        lower_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&lower_output.stdout),
        String::from_utf8_lossy(&lower_output.stderr)
    );
    let bytecode = parse_ail_bytecode(&String::from_utf8_lossy(&lower_output.stdout)).unwrap();
    assert_eq!(bytecode.profile, "Application");
    assert!(bytecode.actions.contains_key("CloseTicket"));
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    fs::remove_file(spec_path).unwrap();
}

#[test]
fn cli_ail_lower_accepts_saved_core_file_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");

    let source_lower = Command::new(binary)
        .args(["ail-lower", &package])
        .output()
        .unwrap();
    assert!(
        source_lower.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&source_lower.stdout),
        String::from_utf8_lossy(&source_lower.stderr)
    );
    let source_bytecode = parse_ail_bytecode(&String::from_utf8_lossy(&source_lower.stdout))
        .expect("source lowering should produce valid bytecode");

    let core_output = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    let parsed_core = parse_ail_core_text(&core_text).unwrap();
    assert_eq!(check_ail_core(&parsed_core), Vec::<String>::new());
    let parsed_bytecode = compile_ail_core_bytecode(&parsed_core).unwrap();
    assert_eq!(parsed_bytecode, source_bytecode);

    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-checked-{}.ail-core.txt",
        std::process::id()
    ));
    fs::write(&core_path, core_text).unwrap();
    let missing_source_package =
        std::env::temp_dir().join(format!("ail-missing-source-package-{}", std::process::id()));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-lower-core-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let lower_output = Command::new(binary)
        .args([
            "ail-lower",
            missing_source_package.to_str().unwrap(),
            "--core-file",
            core_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        lower_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&lower_output.stdout),
        String::from_utf8_lossy(&lower_output.stderr)
    );
    let bytecode = parse_ail_bytecode(&String::from_utf8_lossy(&lower_output.stdout)).unwrap();
    assert_eq!(bytecode, source_bytecode);
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    let close_ticket = bytecode.actions.get("CloseTicket").unwrap();
    assert!(close_ticket.instructions.iter().any(|instruction| {
        instruction.opcode == "SET_FIELD"
            && instruction
                .operands
                .get("text")
                .is_some_and(|text| text == "the ticket status to Closed")
    }));

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert_eq!(
        bytecode_artifact,
        String::from_utf8_lossy(&lower_output.stdout)
    );
    let bytecode_fingerprint =
        fs::read_to_string(artifact_dir.join("artifact.fingerprint.txt")).unwrap();
    assert_eq!(
        bytecode_fingerprint.trim(),
        fnv64_fingerprint(&bytecode_artifact)
    );
    let core_artifact = fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert_eq!(core_artifact, fs::read_to_string(&core_path).unwrap());
    let core_fingerprint =
        fs::read_to_string(artifact_dir.join("checked.ail-core.fingerprint.txt")).unwrap();
    assert_eq!(core_fingerprint.trim(), fnv64_fingerprint(&core_artifact));
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-lower.txt")).unwrap();
    assert!(manifest.contains("AIL-Lower-Manifest:"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "core checked.ail-core.txt {}",
            fnv64_fingerprint(&core_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bytecode artifact.ailbc.json {}",
            fnv64_fingerprint(&bytecode_artifact)
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_file(core_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_lower_records_dependency_report_for_imported_package_graph() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_composed.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-lower-imported-package-dependencies-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-lower",
            &package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Package-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains(
            "resolved-import Shared path=../support_shared.ail requirement=none name=support-shared version=0.1.0"
        ),
        "{dependency_report}"
    );
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-lower.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_spec_accepts_saved_core_file_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");

    let core_output = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    let source_core = parse_ail_core_text(&core_text).unwrap();

    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-spec-from-core-{}.ail-core.txt",
        std::process::id()
    ));
    fs::write(&core_path, &core_text).unwrap();
    let missing_source_package =
        std::env::temp_dir().join(format!("ail-missing-source-package-{}", std::process::id()));

    let spec_output = Command::new(binary)
        .args([
            "ail-spec",
            missing_source_package.to_str().unwrap(),
            "--core-file",
            core_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        spec_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&spec_output.stdout),
        String::from_utf8_lossy(&spec_output.stderr)
    );
    let spec_stdout = String::from_utf8(spec_output.stdout).unwrap();
    assert!(spec_stdout.contains("The application Support Tickets manages"));
    assert!(spec_stdout.contains("Action: Close ticket."));
    assert!(spec_stdout.contains("Failure PermissionDenied happens when"));

    let reparsed = parse_ail_spec_text(&spec_stdout).unwrap();
    let reparsed_core = elaborate_ail_core(
        &ail::ail::AilPackage {
            metadata: source_core.package.clone(),
            root: std::path::PathBuf::new(),
            spec_path: std::path::PathBuf::new(),
            spec_text: String::new(),
            imports: Vec::new(),
        },
        &reparsed,
    );
    assert_eq!(render_ail_core(&reparsed_core), core_text.trim_end());

    fs::remove_file(core_path).unwrap();
}

#[test]
fn cli_ail_spec_core_file_writes_roundtrip_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");

    let core_output = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    let source_core = parse_ail_core_text(&core_text).unwrap();
    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-spec-artifacts-core-{}.ail-core.txt",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-spec-core-roundtrip-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::write(&core_path, &core_text).unwrap();

    let spec_output = Command::new(binary)
        .args([
            "ail-spec",
            "--core-file",
            core_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        spec_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&spec_output.stdout),
        String::from_utf8_lossy(&spec_output.stderr)
    );
    let spec_stdout = String::from_utf8(spec_output.stdout).unwrap();
    assert!(spec_stdout.contains("The application Support Tickets manages"));

    let core_artifact = fs::read_to_string(artifact_dir.join("source.ail-core.txt")).unwrap();
    assert_eq!(core_artifact, core_text);
    let core_fingerprint =
        fs::read_to_string(artifact_dir.join("source.ail-core.fingerprint.txt")).unwrap();
    assert_eq!(core_fingerprint.trim(), fnv64_fingerprint(&core_artifact));
    let spec_artifact = fs::read_to_string(artifact_dir.join("rendered.ail-spec.md")).unwrap();
    assert_eq!(spec_artifact, spec_stdout);
    let spec_fingerprint =
        fs::read_to_string(artifact_dir.join("rendered.ail-spec.fingerprint.txt")).unwrap();
    assert_eq!(spec_fingerprint.trim(), fnv64_fingerprint(&spec_artifact));
    let roundtrip_core_artifact =
        fs::read_to_string(artifact_dir.join("roundtrip.ail-core.txt")).unwrap();
    let reparsed_spec = parse_ail_spec_text(&spec_artifact).unwrap();
    let roundtrip_core = elaborate_ail_core(
        &ail::ail::AilPackage {
            metadata: source_core.package.clone(),
            root: std::path::PathBuf::new(),
            spec_path: std::path::PathBuf::new(),
            spec_text: String::new(),
            imports: Vec::new(),
        },
        &reparsed_spec,
    );
    assert_eq!(
        roundtrip_core_artifact,
        format!("{}\n", render_ail_core(&roundtrip_core))
    );
    assert_eq!(ail_core_hash(&roundtrip_core), ail_core_hash(&source_core));
    let roundtrip_core_fingerprint =
        fs::read_to_string(artifact_dir.join("roundtrip.ail-core.fingerprint.txt")).unwrap();
    assert_eq!(
        roundtrip_core_fingerprint.trim(),
        fnv64_fingerprint(&roundtrip_core_artifact)
    );
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-spec.txt")).unwrap();
    assert!(manifest.contains("AIL-Spec-Manifest:"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "source-core source.ail-core.txt {}",
            fnv64_fingerprint(&core_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "rendered-spec rendered.ail-spec.md {}",
            fnv64_fingerprint(&spec_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "roundtrip-core roundtrip.ail-core.txt {}",
            fnv64_fingerprint(&roundtrip_core_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "roundtrip-hash {} {}",
            ail_core_hash(&source_core),
            ail_core_hash(&roundtrip_core)
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_file(core_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_spec_core_file_does_not_require_dummy_package_path() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");

    let core_output = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-spec-pathless-core-{}.ail-core.txt",
        std::process::id()
    ));
    fs::write(&core_path, core_output.stdout).unwrap();

    let spec_output = Command::new(binary)
        .args(["ail-spec", "--core-file", core_path.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        spec_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&spec_output.stdout),
        String::from_utf8_lossy(&spec_output.stderr)
    );
    let spec_stdout = String::from_utf8(spec_output.stdout).unwrap();
    assert!(spec_stdout.contains("The application Support Tickets manages"));
    assert!(spec_stdout.contains("Action: Close ticket."));

    fs::remove_file(core_path).unwrap();
}

#[test]
fn cli_ail_spec_rejects_unchecked_core_file_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");

    let core_output = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    let unchecked_core_text = core_text.replace(
        "node Field Ticket.status : State<New, Open, Assigned, Closed, Overdue> [secret=false]",
        "node Field Ticket.status : MysteryStatus [secret=false]",
    );
    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-spec-unchecked-core-{}.ail-core.txt",
        std::process::id()
    ));
    fs::write(&core_path, unchecked_core_text).unwrap();
    let missing_source_package =
        std::env::temp_dir().join(format!("ail-missing-source-package-{}", std::process::id()));

    let spec_output = Command::new(binary)
        .args([
            "ail-spec",
            missing_source_package.to_str().unwrap(),
            "--core-file",
            core_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        !spec_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&spec_output.stdout),
        String::from_utf8_lossy(&spec_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&spec_output.stdout);
    assert!(stdout.contains("ail-spec core diagnostics:"), "{stdout}");
    assert!(
        stdout.contains("AIL-TYPE-001 field Ticket.status has unknown type 'MysteryStatus'"),
        "{stdout}"
    );
    assert!(!stdout.contains("The application Support Tickets manages"));

    fs::remove_file(core_path).unwrap();
}

#[test]
fn cli_ail_spec_rejects_unknown_core_schema_items() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");

    let core_output = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    let mut unchecked_core_text = core_text.replace(
        "\nedges:\n",
        "\nnode Widget Forgotten review state\n\nedges:\n",
    );
    unchecked_core_text
        .push_str("edge forgets_semantics Action:CloseTicket -> Widget:Forgotten review state\n");
    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-spec-unknown-schema-{}.ail-core.txt",
        std::process::id()
    ));
    fs::write(&core_path, unchecked_core_text).unwrap();
    let missing_source_package =
        std::env::temp_dir().join(format!("ail-missing-source-package-{}", std::process::id()));

    let spec_output = Command::new(binary)
        .args([
            "ail-spec",
            missing_source_package.to_str().unwrap(),
            "--core-file",
            core_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        !spec_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&spec_output.stdout),
        String::from_utf8_lossy(&spec_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&spec_output.stdout);
    assert!(stdout.contains("ail-spec core diagnostics:"), "{stdout}");
    assert!(
        stdout.contains("AIL-SCHEMA-001 unknown AIL-Core node kind 'Widget'"),
        "{stdout}"
    );
    assert!(
        stdout.contains("AIL-SCHEMA-002 unknown AIL-Core edge kind 'forgets_semantics'"),
        "{stdout}"
    );
    assert!(!stdout.contains("The application Support Tickets manages"));

    fs::remove_file(core_path).unwrap();
}

#[test]
fn cli_ail_lower_agent_verifies_manifest_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-lower-agent-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-lower",
            &package,
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    assert!(agent_bytecode.contains(r#""package":"ail-toolchain-agent""#));
    assert!(agent_bytecode.contains(r#""action":"VerifyLowerManifest""#));
    let agent_fingerprint = fs::read_to_string(artifact_dir.join("agent.fingerprint.txt")).unwrap();
    assert_eq!(agent_fingerprint.trim(), fnv64_fingerprint(&agent_bytecode));
    let parsed_agent = parse_ail_bytecode(&agent_bytecode).unwrap();
    assert_eq!(verify_ail_bytecode(&parsed_agent), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action VerifyLowerManifest started"));
    assert!(agent_trace.contains("read buildrequest.core ir"));
    assert!(agent_trace.contains("read buildrequest.core ir fingerprint"));
    assert!(agent_trace.contains("read buildrequest.source package"));
    assert!(agent_trace.contains("read buildrequest.source package fingerprint"));
    assert!(agent_trace.contains("read buildrequest.bytecode artifact"));
    assert!(agent_trace.contains("read buildrequest.bytecode fingerprint"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest fingerprint"));
    assert!(
        agent_trace.contains("write buildrequest.artifact manifest verification report=Verified")
    );
    assert!(agent_trace.contains("trace LowerManifestVerified"));

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert_eq!(bytecode_artifact, String::from_utf8_lossy(&output.stdout));
    let source_manifest = fs::read_to_string(artifact_dir.join("source.ail-package.md")).unwrap();
    assert_eq!(
        source_manifest,
        fs::read_to_string(format!("{package}/ail-package.md")).unwrap()
    );
    let source_spec = fs::read_to_string(artifact_dir.join("source.ail-spec.md")).unwrap();
    assert_eq!(
        source_spec,
        fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap()
    );
    let source_bundle =
        format!("ail-package.md:\n{source_manifest}\nspec.ail-spec.md:\n{source_spec}");
    let source_fingerprint =
        fs::read_to_string(artifact_dir.join("source.fingerprint.txt")).unwrap();
    assert_eq!(source_fingerprint.trim(), fnv64_fingerprint(&source_bundle));
    let core_artifact = fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap();
    let core_fingerprint =
        fs::read_to_string(artifact_dir.join("checked.ail-core.fingerprint.txt")).unwrap();
    assert_eq!(core_fingerprint.trim(), fnv64_fingerprint(&core_artifact));
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-lower.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "source-package source.ail-package.md source.ail-spec.md {}",
            fnv64_fingerprint(&source_bundle)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "core checked.ail-core.txt {}",
            fnv64_fingerprint(&core_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bytecode artifact.ailbc.json {}",
            fnv64_fingerprint(&bytecode_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "agent agent.ailbc.json {}",
            fnv64_fingerprint(&agent_bytecode)
        )),
        "{manifest}"
    );
    assert!(manifest.contains("trace agent-trace.txt"), "{manifest}");
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_lower_writes_native_agent_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-lower-native-agent-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-lower",
            &package,
            "--agent",
            &agent_package,
            "--target",
            "linux-x86_64-elf",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let agent_native = fs::read(artifact_dir.join("agent-VerifyLowerManifest.elf")).unwrap();
    assert_eq!(&agent_native[0..4], b"\x7fELF");
    let expected_agent_native_fingerprint = fnv64_fingerprint_bytes(&agent_native);
    let native_run = Command::new(artifact_dir.join("agent-VerifyLowerManifest.elf"))
        .args([
            "buildrequest.id=support-ticket-lower",
            "buildrequest.status=BytecodeReady",
            "buildrequest.core ir=ok",
            "buildrequest.core ir fingerprint=fnv64:core",
            "buildrequest.bytecode artifact=ok",
            "buildrequest.bytecode fingerprint=fnv64:bytecode",
            "buildrequest.artifact manifest=ok",
            "buildrequest.artifact manifest fingerprint=fnv64:manifest",
            "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
        ])
        .output()
        .unwrap();
    assert!(
        native_run.status.success(),
        "native lower manifest verifier failed"
    );
    assert!(
        String::from_utf8_lossy(&native_run.stderr).contains("trace LowerManifestVerified"),
        "{}",
        String::from_utf8_lossy(&native_run.stderr)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-lower.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "agent-target linux-x86_64-elf agent-VerifyLowerManifest.elf {expected_agent_native_fingerprint}"
        )),
        "{manifest}"
    );
    let native_bytecode_report =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.txt")).unwrap();
    assert!(
        native_bytecode_report.contains("AIL-Lower-Native-Bytecode:"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("target linux-x86_64-elf"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains(&format!(
            "machine-bytecode agent-target linux-x86_64-elf agent-VerifyLowerManifest.elf elf64-little-x86_64-executable {expected_agent_native_fingerprint} bytes {}",
            agent_native.len()
        )),
        "{native_bytecode_report}"
    );
    let native_bytecode_report_fingerprint =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.fingerprint.txt")).unwrap();
    assert_eq!(
        native_bytecode_report_fingerprint.trim(),
        fnv64_fingerprint(&native_bytecode_report)
    );
    assert!(
        manifest.contains(&format!(
            "native-bytecode native-bytecode-report.txt {}",
            fnv64_fingerprint(&native_bytecode_report)
        )),
        "{manifest}"
    );
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Lower-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("target linux-x86_64-elf"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("host-language-runtime none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("dynamic-linker none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("shared-libraries none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("library-dependencies none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("linker-invocation none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains(
            "machine-bytecode-dependency agent-VerifyLowerManifest.elf standalone-linux-syscall-elf"
        ),
        "{dependency_report}"
    );
    let dependency_report_fingerprint =
        fs::read_to_string(artifact_dir.join("dependency-report.fingerprint.txt")).unwrap();
    assert_eq!(
        dependency_report_fingerprint.trim(),
        fnv64_fingerprint(&dependency_report)
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action VerifyLowerManifest started"));
    assert!(agent_trace.contains("read buildrequest.machine bytecode contract"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.dependency report"));
    assert!(agent_trace.contains("read buildrequest.dependency report fingerprint"));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_emits_runnable_linux_x86_64_elf_executable() {
    use std::os::unix::fs::PermissionsExt;

    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let executable_path =
        std::env::temp_dir().join(format!("ail-close-ticket-native-{}", std::process::id()));
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let executable = fs::read(&executable_path).unwrap();
    assert_eq!(&executable[0..4], b"\x7fELF");
    assert_eq!(executable[4], 2, "ELFCLASS64");
    assert_eq!(executable[5], 1, "little-endian ELF");
    assert_eq!(&executable[18..20], &[0x3e, 0x00], "EM_X86_64");
    assert_ne!(
        fs::metadata(&executable_path).unwrap().permissions().mode() & 0o111,
        0,
        "native output should be executable"
    );

    let run = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "native executable failed: {}",
        run.status
    );

    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_accepts_saved_bytecode_artifact_for_native_elf() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-ail-compile-saved-bytecode-{}.ailbc.json",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-ail-compile-saved-bytecode-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_file(&executable_path);

    let lowered = Command::new(binary)
        .args(["ail-lower", &package])
        .output()
        .unwrap();
    assert!(
        lowered.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&lowered.stdout),
        String::from_utf8_lossy(&lowered.stderr)
    );
    fs::write(&bytecode_path, lowered.stdout).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let executable = fs::read(&executable_path).unwrap();
    assert_eq!(&executable[0..4], b"\x7fELF");
    let run = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "native executable failed: {}",
        run.status
    );
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("ticket.status=Closed"),
        "{}",
        String::from_utf8_lossy(&run.stdout)
    );
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("trace TicketClosed"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );

    fs::remove_file(bytecode_path).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_writes_saved_bytecode_native_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-ail-compile-bytecode-artifacts-{}.ailbc.json",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-ail-compile-bytecode-artifacts-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-compile-bytecode-artifacts-dir-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_file(&executable_path);
    let _ = fs::remove_dir_all(&artifact_dir);

    let lowered = Command::new(binary)
        .args(["ail-lower", &package])
        .output()
        .unwrap();
    assert!(
        lowered.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&lowered.stdout),
        String::from_utf8_lossy(&lowered.stderr)
    );
    let lowered_bytecode = String::from_utf8(lowered.stdout).unwrap();
    fs::write(&bytecode_path, &lowered_bytecode).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert_eq!(bytecode_artifact, lowered_bytecode);
    let bytecode_fingerprint =
        fs::read_to_string(artifact_dir.join("artifact.fingerprint.txt")).unwrap();
    assert_eq!(
        bytecode_fingerprint.trim(),
        fnv64_fingerprint(&bytecode_artifact)
    );
    let target_artifact = fs::read(artifact_dir.join("target.elf")).unwrap();
    assert_eq!(&target_artifact[0..4], b"\x7fELF");
    let target_fingerprint =
        fs::read_to_string(artifact_dir.join("target.fingerprint.txt")).unwrap();
    assert_eq!(
        target_fingerprint.trim(),
        fnv64_fingerprint_bytes(&target_artifact)
    );
    assert_eq!(target_artifact, fs::read(&executable_path).unwrap());
    let native_bytecode_report =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.txt")).unwrap();
    assert!(
        native_bytecode_report.contains("AIL-Compile-Native-Bytecode:"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("target linux-x86_64-elf"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("bytecode-level machine"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("bytecode-container linux-elf-executable"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("bytecode-format elf64-little-x86_64-executable"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("action CloseTicket"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains(&format!(
            "machine-bytecode target linux-x86_64-elf target.elf elf64-little-x86_64-executable {} bytes {}",
            fnv64_fingerprint_bytes(&target_artifact),
            target_artifact.len()
        )),
        "{native_bytecode_report}"
    );
    let native_bytecode_report_fingerprint =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.fingerprint.txt")).unwrap();
    assert_eq!(
        native_bytecode_report_fingerprint.trim(),
        fnv64_fingerprint(&native_bytecode_report)
    );
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Compile-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("target linux-x86_64-elf"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("host-language-runtime none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("dynamic-linker none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("shared-libraries none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("library-dependencies none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("linker-invocation none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report
            .contains("machine-bytecode-dependency target.elf standalone-linux-syscall-elf"),
        "{dependency_report}"
    );
    let dependency_report_fingerprint =
        fs::read_to_string(artifact_dir.join("dependency-report.fingerprint.txt")).unwrap();
    assert_eq!(
        dependency_report_fingerprint.trim(),
        fnv64_fingerprint(&dependency_report)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(manifest.contains("AIL-Compile-Manifest:"), "{manifest}");
    assert!(manifest.contains("action CloseTicket"), "{manifest}");
    assert!(
        manifest.contains("machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable"),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bytecode artifact.ailbc.json {}",
            fnv64_fingerprint(&bytecode_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "native-bytecode native-bytecode-report.txt {}",
            fnv64_fingerprint(&native_bytecode_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "target linux-x86_64-elf target.elf {}",
            fnv64_fingerprint_bytes(&target_artifact)
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    let native_run = Command::new(artifact_dir.join("target.elf"))
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(native_run.status.success(), "artifact target.elf failed");
    assert!(
        String::from_utf8_lossy(&native_run.stdout).contains("ticket.status=Closed"),
        "{}",
        String::from_utf8_lossy(&native_run.stdout)
    );

    fs::remove_file(bytecode_path).unwrap();
    fs::remove_file(executable_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_writes_saved_bytecode_wasm_contract_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-wasm-contract-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-bytecode-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: wasm-contract-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces
conformance: first-slice
target-support:
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Wasm Contract App manages portable compilation.

Action: Emit trace.

When emit trace happens:

- the system records a trace event named WasmContractCompiled
"#,
    )
    .unwrap();

    let lowered = Command::new(binary)
        .args(["ail-lower", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        lowered.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&lowered.stdout),
        String::from_utf8_lossy(&lowered.stderr)
    );
    let lowered_bytecode = String::from_utf8(lowered.stdout).unwrap();
    fs::write(&bytecode_path, &lowered_bytecode).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--action",
            "EmitTrace",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("ail-compile wrote wasm32-unknown-sandbox-wasm contract"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert_eq!(bytecode_artifact, lowered_bytecode);
    let bytecode_fingerprint =
        fs::read_to_string(artifact_dir.join("artifact.fingerprint.txt")).unwrap();
    assert_eq!(
        bytecode_fingerprint.trim(),
        fnv64_fingerprint(&bytecode_artifact)
    );
    assert!(
        !artifact_dir.join("target.elf").exists(),
        "wasm contract artifacts must not pretend to be native ELF output"
    );

    let contract_report =
        fs::read_to_string(artifact_dir.join("wasm-contract-report.txt")).unwrap();
    assert!(
        contract_report.contains("AIL-Wasm-Contract-Report:"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("target wasm32-unknown-sandbox-wasm"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("status supported-with-host-imports"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("bytecode-level portable-vm-contract"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-boundary declared-imports-only"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-import-metadata present-in-saved-bytecode"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-imports none"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("action EmitTrace"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("trace-preservation required"),
        "{contract_report}"
    );
    let contract_report_fingerprint =
        fs::read_to_string(artifact_dir.join("wasm-contract-report.fingerprint.txt")).unwrap();
    assert_eq!(
        contract_report_fingerprint.trim(),
        fnv64_fingerprint(&contract_report)
    );

    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Compile-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("target wasm32-unknown-sandbox-wasm"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("host-language-runtime none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("dynamic-linker none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("shared-libraries none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("library-dependencies none"),
        "{dependency_report}"
    );
    let dependency_report_fingerprint =
        fs::read_to_string(artifact_dir.join("dependency-report.fingerprint.txt")).unwrap();
    assert_eq!(
        dependency_report_fingerprint.trim(),
        fnv64_fingerprint(&dependency_report)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(manifest.contains("AIL-Compile-Manifest:"), "{manifest}");
    assert!(manifest.contains("action EmitTrace"), "{manifest}");
    assert!(
        manifest.contains("machine-bytecode-contract wasm32-unknown-sandbox-wasm bytecode-level portable-vm-contract bytecode-container wasm-sandbox-contract bytecode-format wasm32-contract-report"),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bytecode artifact.ailbc.json {}",
            fnv64_fingerprint(&bytecode_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "wasm-contract wasm-contract-report.txt {}",
            fnv64_fingerprint(&contract_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(root).unwrap();
    fs::remove_file(bytecode_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_package_writes_wasm_contract_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-wasm-contract-source-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-source-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: wasm-source-contract-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces
conformance: first-slice
target-support:
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Wasm Source Contract App manages portable compilation.

Action: Emit trace.

When emit trace happens:

- the system records a trace event named WasmSourceContractCompiled
"#,
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            root.to_str().unwrap(),
            "--action",
            "EmitTrace",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("ail-compile wrote wasm32-unknown-sandbox-wasm contract"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );

    assert!(
        fs::read_to_string(artifact_dir.join("source.ail-package.md"))
            .unwrap()
            .contains("name: wasm-source-contract-app")
    );
    assert!(
        fs::read_to_string(artifact_dir.join("source.ail-spec.md"))
            .unwrap()
            .contains("Action: Emit trace.")
    );
    let checked_core = fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert!(
        checked_core
            .contains("target-support: wasm32-unknown-sandbox-wasm=supported-with-host-imports"),
        "{checked_core}"
    );
    let checked_core_fingerprint =
        fs::read_to_string(artifact_dir.join("checked.ail-core.fingerprint.txt")).unwrap();
    assert_eq!(
        checked_core_fingerprint.trim(),
        fnv64_fingerprint(&checked_core)
    );
    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert!(bytecode_artifact.contains(r#""action":"EmitTrace""#));
    let contract_report =
        fs::read_to_string(artifact_dir.join("wasm-contract-report.txt")).unwrap();
    assert!(
        contract_report.contains("action EmitTrace")
            && contract_report.contains("trace-preservation required"),
        "{contract_report}"
    );
    assert!(
        !artifact_dir.join("target.elf").exists(),
        "source Wasm contract artifacts must not emit native ELF output"
    );
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(manifest.contains("source-package source.ail-package.md source.ail-spec.md"));
    assert!(manifest.contains("core checked.ail-core.txt"));
    assert!(
        manifest.contains("machine-bytecode-contract wasm32-unknown-sandbox-wasm bytecode-level portable-vm-contract bytecode-container wasm-sandbox-contract bytecode-format wasm32-contract-report"),
        "{manifest}"
    );
    assert!(manifest.contains("wasm-contract wasm-contract-report.txt"));
    assert!(!manifest.contains("target wasm32-unknown-sandbox-wasm target.elf"));

    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_records_dependency_report_for_imported_package_graph() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-compile-imported-package-root-{}-{unique_suffix}",
        std::process::id()
    ));
    let shared = root.join("shared");
    let app = root.join("app");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-compile-imported-package-dependencies-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&app).unwrap();
    fs::write(
        shared.join("ail-package.md"),
        r#"name: shared-types
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: things
conformance: first-slice
"#,
    )
    .unwrap();
    fs::write(
        shared.join("spec.ail-spec.md"),
        r#"The application Shared Types manages shared declarations.

A User has:

- id: Text
"#,
    )
    .unwrap();
    fs::write(
        app.join("ail-package.md"),
        r#"name: imported-compile-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: imports, actions, traces
imports: ../shared as Shared
conformance: first-slice
target-support:
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        app.join("spec.ail-spec.md"),
        r#"The application Imported Compile App manages imported compile evidence.

Action: Close ticket.

When a support agent closes a ticket:

- the system records a trace event named TicketClosed
"#,
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            app.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Compile-Dependency-Report:")
            && dependency_report.contains("AIL-Package-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains(
            "resolved-import Shared path=../shared requirement=none name=shared-types version=0.1.0"
        ),
        "{dependency_report}"
    );
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );

    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_package_wasm_contract_snapshots_spec_file_override() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-wasm-contract-spec-file-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let override_spec_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-spec-file-{}-{unique_suffix}.ail-spec.md",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-spec-file-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&override_spec_path);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: wasm-spec-file-contract-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces
conformance: first-slice
target-support:
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Wasm Spec File Contract App manages stale entry specs.

Action: Original trace.

When original trace happens:

- the system records a trace event named OriginalSpecTrace
"#,
    )
    .unwrap();
    fs::write(
        &override_spec_path,
        r#"The application Wasm Spec File Contract App manages override specs.

Action: Override trace.

When override trace happens:

- the system records a trace event named OverrideSpecTrace
"#,
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            root.to_str().unwrap(),
            "--spec-file",
            override_spec_path.to_str().unwrap(),
            "--action",
            "OverrideTrace",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let source_spec = fs::read_to_string(artifact_dir.join("source.ail-spec.md")).unwrap();
    assert!(
        source_spec.contains("Action: Override trace."),
        "{source_spec}"
    );
    assert!(
        !source_spec.contains("Action: Original trace."),
        "{source_spec}"
    );
    let checked_core = fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert!(
        checked_core.contains("node Action OverrideTrace"),
        "{checked_core}"
    );
    assert!(
        !checked_core.contains("node Action OriginalTrace"),
        "{checked_core}"
    );
    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert!(bytecode_artifact.contains(r#""action":"OverrideTrace""#));
    let contract_report =
        fs::read_to_string(artifact_dir.join("wasm-contract-report.txt")).unwrap();
    assert!(
        contract_report.contains("action OverrideTrace"),
        "{contract_report}"
    );

    fs::remove_dir_all(root).unwrap();
    fs::remove_file(override_spec_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_core_file_writes_wasm_contract_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-wasm-contract-core-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let core_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-core-{}-{unique_suffix}.ail-core.txt",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-core-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&core_path);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: wasm-core-contract-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces
conformance: first-slice
target-support:
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Wasm Core Contract App manages checked core artifacts.

Action: Emit trace.

When emit trace happens:

- the system records a trace event named WasmCoreContractCompiled
"#,
    )
    .unwrap();

    let core_output = Command::new(binary)
        .args(["ail-core", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    fs::write(&core_path, &core_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            "--core-file",
            core_path.to_str().unwrap(),
            "--action",
            "EmitTrace",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(!artifact_dir.join("source.ail-package.md").exists());
    assert!(!artifact_dir.join("source.ail-spec.md").exists());
    let checked_core = fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert_eq!(checked_core, core_text);
    let checked_core_fingerprint =
        fs::read_to_string(artifact_dir.join("checked.ail-core.fingerprint.txt")).unwrap();
    assert_eq!(
        checked_core_fingerprint.trim(),
        fnv64_fingerprint(&checked_core)
    );
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(!manifest.contains("source-package"));
    assert!(
        manifest.contains(&format!(
            "core checked.ail-core.txt {}",
            fnv64_fingerprint(&checked_core)
        )),
        "{manifest}"
    );
    assert!(manifest.contains("wasm-contract wasm-contract-report.txt"));
    assert!(
        !artifact_dir.join("target.elf").exists(),
        "checked-core Wasm contract artifacts must not emit native ELF output"
    );

    fs::remove_dir_all(root).unwrap();
    fs::remove_file(core_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_wasm_contract_enumerates_external_bindings() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-wasm-contract-import-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-import-bytecode-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-import-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: wasm-import-contract-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces, c-interop
conformance: first-slice
target-support:
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Wasm Import Contract App manages portable compression.

C library: zlib.

The library imports function compress2.

compress2 needs:

- dest: Pointer<UInt8> borrowed mutable
- dest_len: Pointer<UInt64> borrowed mutable
- source: Pointer<UInt8> borrowed
- source_len: UInt64
- level: Int

compress2 produces:

- status: CInt

compress2 maps errno or status codes:

- Z_OK maps to success
- Z_MEM_ERROR maps to Failure.OutOfMemory
- Z_BUF_ERROR maps to Failure.OutputBufferTooSmall

compress2 requires capability:

- call zlib compress2

compress2 records trace event named ForeignCallCompress2

Action: Compress payload.

When compress payload happens:

- the system records a trace event named PayloadCompressed
"#,
    )
    .unwrap();

    let lowered = Command::new(binary)
        .args(["ail-lower", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        lowered.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&lowered.stdout),
        String::from_utf8_lossy(&lowered.stderr)
    );
    let lowered_bytecode = String::from_utf8(lowered.stdout).unwrap();
    assert!(
        lowered_bytecode.contains(r#""external_bindings":[{"#),
        "{lowered_bytecode}"
    );
    assert!(lowered_bytecode.contains(r#""name":"zlib.compress2""#));
    assert!(lowered_bytecode.contains(r#""symbol":"compress2""#));
    assert!(lowered_bytecode.contains(r#""capabilities":["call zlib compress2"]"#));
    fs::write(&bytecode_path, &lowered_bytecode).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--action",
            "CompressPayload",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let contract_report =
        fs::read_to_string(artifact_dir.join("wasm-contract-report.txt")).unwrap();
    assert!(
        contract_report.contains("host-boundary declared-imports-only"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-import-metadata present-in-saved-bytecode"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-import zlib.compress2 library zlib symbol compress2 binding-kind CFunction calling-convention cdecl"),
        "{contract_report}"
    );
    assert!(
        contract_report
            .contains("host-import-input zlib.compress2 dest Pointer<UInt8> borrowed mutable"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-import-output zlib.compress2 status CInt"),
        "{contract_report}"
    );
    assert!(
        contract_report
            .contains("host-import-status zlib.compress2 Z_MEM_ERROR Failure.OutOfMemory"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-import-status zlib.compress2 Z_OK success"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-import-capability zlib.compress2 call zlib compress2"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-import-trace zlib.compress2 ForeignCallCompress2"),
        "{contract_report}"
    );
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("library-dependencies zlib"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("host-import-dependency zlib.compress2 library zlib symbol compress2 binding-kind CFunction calling-convention cdecl"),
        "{dependency_report}"
    );

    fs::remove_dir_all(root).unwrap();
    fs::remove_file(bytecode_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_writes_darwin_macho_contract_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-darwin-contract-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-darwin-contract-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: darwin-contract-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces, c-interop
conformance: first-slice
target-support:
  aarch64-apple-darwin-libsystem-macho: planned-contract
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Darwin Contract App manages libSystem boundaries.

C library: libSystem.

The library imports function getpid.

getpid produces:

- status: CInt

getpid maps errno or status codes:

- OK maps to success
- EINVAL maps to Failure.InvalidProcessIdRead

getpid requires capability:

- call libSystem getpid

getpid records trace event named ForeignGetPid

Action: Read process id.

When read process id happens:

- the system records a trace event named ProcessIdRead
"#,
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            root.to_str().unwrap(),
            "--action",
            "ReadProcessId",
            "--target",
            "aarch64-apple-darwin-libsystem-macho",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("ail-compile wrote aarch64-apple-darwin-libsystem-macho contract"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );

    assert!(
        fs::read_to_string(artifact_dir.join("source.ail-package.md"))
            .unwrap()
            .contains("name: darwin-contract-app")
    );
    let checked_core = fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert!(
        checked_core
            .contains("target-support: aarch64-apple-darwin-libsystem-macho=planned-contract"),
        "{checked_core}"
    );
    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert!(bytecode_artifact.contains(r#""action":"ReadProcessId""#));
    assert!(bytecode_artifact.contains(r#""name":"libSystem.getpid""#));
    let bytecode_fingerprint =
        fs::read_to_string(artifact_dir.join("artifact.fingerprint.txt")).unwrap();
    assert_eq!(
        bytecode_fingerprint.trim(),
        fnv64_fingerprint(&bytecode_artifact)
    );
    assert!(
        !artifact_dir.join("target.elf").exists(),
        "Darwin contract artifacts must not pretend to be native ELF output"
    );

    let contract_report =
        fs::read_to_string(artifact_dir.join("darwin-macho-contract-report.txt")).unwrap();
    assert!(
        contract_report.contains("AIL-Darwin-MachO-Contract-Report:"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("target aarch64-apple-darwin-libsystem-macho"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("status planned-contract"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("bytecode-level portable-vm-contract"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("bytecode-container darwin-macho-contract"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("bytecode-format macho64-arm64-contract-report"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-boundary libSystem-and-entitlements"),
        "{contract_report}"
    );
    assert!(
        contract_report
            .contains("external-symbol libSystem.getpid library libSystem symbol getpid"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("capability libSystem.getpid call libSystem getpid"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("trace-preservation required"),
        "{contract_report}"
    );
    let contract_report_fingerprint =
        fs::read_to_string(artifact_dir.join("darwin-macho-contract-report.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        contract_report_fingerprint.trim(),
        fnv64_fingerprint(&contract_report)
    );

    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("target aarch64-apple-darwin-libsystem-macho"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("dynamic-linker libSystem"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("shared-libraries libSystem"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("machine-bytecode-dependency darwin-macho-contract-report.txt contract-only-darwin-macho"),
        "{dependency_report}"
    );
    let dependency_report_fingerprint =
        fs::read_to_string(artifact_dir.join("dependency-report.fingerprint.txt")).unwrap();
    assert_eq!(
        dependency_report_fingerprint.trim(),
        fnv64_fingerprint(&dependency_report)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(manifest.contains("AIL-Compile-Manifest:"), "{manifest}");
    assert!(manifest.contains("action ReadProcessId"), "{manifest}");
    assert!(
        manifest.contains("machine-bytecode-contract aarch64-apple-darwin-libsystem-macho bytecode-level portable-vm-contract bytecode-container darwin-macho-contract bytecode-format macho64-arm64-contract-report"),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "darwin-macho-contract darwin-macho-contract-report.txt {}",
            fnv64_fingerprint(&contract_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains("dependencies dependency-report.txt"),
        "{manifest}"
    );
    assert!(
        !manifest.contains("target aarch64-apple-darwin-libsystem-macho target.elf"),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_darwin_contract_rejects_linux_only_syscall_effect() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-darwin-linux-effect-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-darwin-linux-effect-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: darwin-linux-effect-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces, system-components, capabilities, effects
conformance: first-slice
target-support:
  aarch64-apple-darwin-libsystem-macho: planned-contract
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Darwin Linux Effect App manages target-specific effects.

System component: Linux syscall bridge.

The component requires capability:

- call linux syscall exit

The component performs:

- linux syscall exit

Action: Linux exit.

When linux exit happens:

- the system records a trace event named LinuxExit
"#,
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            root.to_str().unwrap(),
            "--action",
            "LinuxExit",
            "--target",
            "aarch64-apple-darwin-libsystem-macho",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined_output = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined_output.contains("AIL-BACKEND-001 target aarch64-apple-darwin-libsystem-macho does not support Linux-only syscall effect 'linux syscall exit'"),
        "{combined_output}"
    );

    fs::remove_dir_all(root).unwrap();
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_compile_wasm_contract_marks_reachable_call_trace_required() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-call-bytecode-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-call-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_dir_all(&artifact_dir);
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "wasm-call-example",
  "version": "0.1.0",
  "profile": "Application",
  "target_support": {"wasm32-unknown-sandbox-wasm":"supported-with-host-imports"},
  "failures": [],
  "actions": [
    {
      "action": "ResolveTicket",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"ResolveTicket"}},
        {"opcode":"CALL_ACTION","operands":{"target":"CloseTicket"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    },
    {
      "action": "CloseTicket",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"CloseTicket"}},
        {"opcode":"EMIT_TRACE","operands":{"event":"TicketClosed"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    fs::write(&bytecode_path, bytecode_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--action",
            "ResolveTicket",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let contract_report =
        fs::read_to_string(artifact_dir.join("wasm-contract-report.txt")).unwrap();
    assert!(
        contract_report.contains("action ResolveTicket"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("trace-preservation required"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-boundary saved-bytecode-contract"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-import-metadata not-present-in-saved-bytecode"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("host-imports not-enumerated-in-saved-bytecode"),
        "{contract_report}"
    );
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("library-dependencies not-enumerated-in-saved-bytecode"),
        "{dependency_report}"
    );

    fs::remove_file(bytecode_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_wasm_contract_rejects_stale_executable_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-stale-bytecode-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-stale-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::write(artifact_dir.join("target.elf"), b"stale-native-output").unwrap();
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "wasm-stale-example",
  "version": "0.1.0",
  "profile": "Application",
  "target_support": {"wasm32-unknown-sandbox-wasm":"supported-with-host-imports"},
  "failures": [],
  "actions": [
    {
      "action": "EmitTrace",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"EmitTrace"}},
        {"opcode":"EMIT_TRACE","operands":{"event":"WasmContractCompiled"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    fs::write(&bytecode_path, bytecode_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--action",
            "EmitTrace",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("contains stale executable artifact target.elf"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    fs::remove_file(bytecode_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_wasm_contract_bundle_rejects_stale_native_bundle_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-stale-bundle-bytecode-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-stale-bundle-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::write(
        artifact_dir.join("target-EmitTrace.elf"),
        b"stale-native-bundle-output",
    )
    .unwrap();
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "wasm-stale-bundle-example",
  "version": "0.1.0",
  "profile": "Application",
  "target_support": {"wasm32-unknown-sandbox-wasm":"supported-with-host-imports"},
  "failures": [],
  "actions": [
    {
      "action": "EmitTrace",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"EmitTrace"}},
        {"opcode":"EMIT_TRACE","operands":{"event":"WasmContractCompiled"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    fs::write(&bytecode_path, bytecode_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--all-actions",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("contains stale executable artifact target-EmitTrace.elf"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    fs::remove_file(bytecode_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_wasm_contract_bundle_rejects_stale_native_agent_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-stale-agent-bytecode-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-stale-agent-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&artifact_dir).unwrap();
    fs::write(
        artifact_dir.join("agent-VerifyCompileManifest.elf"),
        b"stale-native-agent-output",
    )
    .unwrap();
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "wasm-stale-agent-example",
  "version": "0.1.0",
  "profile": "Application",
  "target_support": {"wasm32-unknown-sandbox-wasm":"supported-with-host-imports"},
  "failures": [],
  "actions": [
    {
      "action": "EmitTrace",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"EmitTrace"}},
        {"opcode":"EMIT_TRACE","operands":{"event":"WasmContractCompiled"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    fs::write(&bytecode_path, bytecode_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--all-actions",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("contains stale executable artifact agent-VerifyCompileManifest.elf"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    fs::remove_file(bytecode_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_writes_saved_bytecode_wasm_contract_bundle() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-bundle-bytecode-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-bundle-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_dir_all(&artifact_dir);
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "wasm-bundle-example",
  "version": "0.1.0",
  "profile": "Application",
  "target_support": {"wasm32-unknown-sandbox-wasm":"supported-with-host-imports"},
  "failures": [],
  "actions": [
    {
      "action": "EmitAuditTrace",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"EmitAuditTrace"}},
        {"opcode":"EMIT_TRACE","operands":{"event":"AuditTraceEmitted"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    },
    {
      "action": "ReturnOnly",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"ReturnOnly"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    fs::write(&bytecode_path, bytecode_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--all-actions",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("ail-compile wrote wasm32-unknown-sandbox-wasm contract bundle"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert_eq!(bytecode_artifact, bytecode_text);
    assert!(!artifact_dir.join("target.elf").exists());
    assert!(!artifact_dir.join("target.wasm").exists());
    assert!(!artifact_dir.join("native-bytecode-report.txt").exists());

    let contract_report =
        fs::read_to_string(artifact_dir.join("wasm-contract-report.txt")).unwrap();
    assert!(
        contract_report.contains("AIL-Wasm-Contract-Report:"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("bundle all-actions"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("action EmitAuditTrace trace-preservation required"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("action ReturnOnly trace-preservation not-required-by-action"),
        "{contract_report}"
    );
    assert!(
        contract_report.contains("executable-wasm-module none"),
        "{contract_report}"
    );
    let contract_report_fingerprint =
        fs::read_to_string(artifact_dir.join("wasm-contract-report.fingerprint.txt")).unwrap();
    assert_eq!(
        contract_report_fingerprint.trim(),
        fnv64_fingerprint(&contract_report)
    );

    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Compile-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("bundle all-actions"),
        "{dependency_report}"
    );
    assert!(
        dependency_report
            .contains("machine-bytecode-dependency wasm-contract-report.txt portable-vm-contract"),
        "{dependency_report}"
    );
    let dependency_report_fingerprint =
        fs::read_to_string(artifact_dir.join("dependency-report.fingerprint.txt")).unwrap();
    assert_eq!(
        dependency_report_fingerprint.trim(),
        fnv64_fingerprint(&dependency_report)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(manifest.contains("AIL-Compile-Manifest:"), "{manifest}");
    assert!(manifest.contains("bundle all-actions"), "{manifest}");
    assert!(
        manifest.contains("machine-bytecode-contract wasm32-unknown-sandbox-wasm bytecode-level portable-vm-contract bytecode-container wasm-sandbox-contract bytecode-format wasm32-contract-report"),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "wasm-contract wasm-contract-report.txt {}",
            fnv64_fingerprint(&contract_report)
        )),
        "{manifest}"
    );
    assert!(!manifest.contains("native-bytecode native-bytecode-report.txt"));
    assert!(!manifest.contains("target wasm32-unknown-sandbox-wasm"));
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_file(bytecode_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_package_and_core_file_write_wasm_contract_bundles() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-wasm-contract-bundle-source-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let core_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-bundle-core-{}-{unique_suffix}.ail-core.txt",
        std::process::id()
    ));
    let source_artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-bundle-source-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let core_artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-bundle-core-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&core_path);
    let _ = fs::remove_dir_all(&source_artifact_dir);
    let _ = fs::remove_dir_all(&core_artifact_dir);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: wasm-source-bundle-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces
conformance: first-slice
target-support:
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Wasm Source Bundle App manages portable action bundles.

Action: Emit first trace.

When emit first trace happens:

- the system records a trace event named FirstWasmBundleTrace

Action: Emit second trace.

When emit second trace happens:

- the system records a trace event named SecondWasmBundleTrace
"#,
    )
    .unwrap();

    let source_output = Command::new(binary)
        .args([
            "ail-compile",
            root.to_str().unwrap(),
            "--all-actions",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            source_artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        source_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&source_output.stdout),
        String::from_utf8_lossy(&source_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&source_output.stdout)
            .contains("ail-compile wrote wasm32-unknown-sandbox-wasm contract bundle"),
        "{}",
        String::from_utf8_lossy(&source_output.stdout)
    );
    assert!(
        fs::read_to_string(source_artifact_dir.join("source.ail-package.md"))
            .unwrap()
            .contains("name: wasm-source-bundle-app")
    );
    assert!(
        fs::read_to_string(source_artifact_dir.join("source.ail-spec.md"))
            .unwrap()
            .contains("Action: Emit second trace.")
    );
    let source_checked_core =
        fs::read_to_string(source_artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert!(
        source_checked_core
            .contains("target-support: wasm32-unknown-sandbox-wasm=supported-with-host-imports"),
        "{source_checked_core}"
    );
    let source_contract_report =
        fs::read_to_string(source_artifact_dir.join("wasm-contract-report.txt")).unwrap();
    assert!(
        source_contract_report.contains("bundle all-actions"),
        "{source_contract_report}"
    );
    assert!(
        source_contract_report.contains("action EmitFirstTrace trace-preservation required"),
        "{source_contract_report}"
    );
    assert!(
        source_contract_report.contains("action EmitSecondTrace trace-preservation required"),
        "{source_contract_report}"
    );
    let source_manifest =
        fs::read_to_string(source_artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(
        source_manifest.contains("source-package source.ail-package.md source.ail-spec.md"),
        "{source_manifest}"
    );
    assert!(
        source_manifest.contains("core checked.ail-core.txt"),
        "{source_manifest}"
    );
    assert!(source_manifest.contains("bundle all-actions"));
    assert!(!source_manifest.contains("native-bytecode native-bytecode-report.txt"));
    assert!(!source_artifact_dir.join("target.elf").exists());
    assert!(!source_artifact_dir.join("target.wasm").exists());

    let core_output = Command::new(binary)
        .args(["ail-core", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    fs::write(&core_path, &core_text).unwrap();

    let core_compile_output = Command::new(binary)
        .args([
            "ail-compile",
            "--core-file",
            core_path.to_str().unwrap(),
            "--all-actions",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            core_artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        core_compile_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_compile_output.stdout),
        String::from_utf8_lossy(&core_compile_output.stderr)
    );
    assert!(!core_artifact_dir.join("source.ail-package.md").exists());
    assert!(!core_artifact_dir.join("source.ail-spec.md").exists());
    let core_checked_core =
        fs::read_to_string(core_artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert_eq!(core_checked_core, core_text);
    let core_contract_report =
        fs::read_to_string(core_artifact_dir.join("wasm-contract-report.txt")).unwrap();
    assert!(
        core_contract_report.contains("bundle all-actions")
            && core_contract_report.contains("action EmitFirstTrace trace-preservation required")
            && core_contract_report.contains("action EmitSecondTrace trace-preservation required"),
        "{core_contract_report}"
    );
    let core_manifest =
        fs::read_to_string(core_artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(!core_manifest.contains("source-package"));
    assert!(
        core_manifest.contains(&format!(
            "core checked.ail-core.txt {}",
            fnv64_fingerprint(&core_checked_core)
        )),
        "{core_manifest}"
    );
    assert!(core_manifest.contains("bundle all-actions"));
    assert!(!core_manifest.contains("native-bytecode native-bytecode-report.txt"));
    assert!(!core_artifact_dir.join("target.elf").exists());
    assert!(!core_artifact_dir.join("target.wasm").exists());

    fs::remove_dir_all(root).unwrap();
    fs::remove_file(core_path).unwrap();
    fs::remove_dir_all(source_artifact_dir).unwrap();
    fs::remove_dir_all(core_artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_wasm_contract_agent_verifies_manifest_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-wasm-contract-agent-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-agent-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: wasm-agent-contract-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces
conformance: first-slice
target-support:
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Wasm Agent Contract App manages portable agent verification.

Action: Emit trace.

When emit trace happens:

- the system records a trace event named WasmAgentContractCompiled
"#,
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            root.to_str().unwrap(),
            "--action",
            "EmitTrace",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    assert!(agent_bytecode.contains(r#""package":"ail-toolchain-agent""#));
    assert!(agent_bytecode.contains(r#""action":"VerifyCompileManifest""#));
    let agent_fingerprint = fs::read_to_string(artifact_dir.join("agent.fingerprint.txt")).unwrap();
    assert_eq!(agent_fingerprint.trim(), fnv64_fingerprint(&agent_bytecode));
    let parsed_agent = parse_ail_bytecode(&agent_bytecode).unwrap();
    assert_eq!(verify_ail_bytecode(&parsed_agent), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action VerifyCompileManifest started"));
    assert!(agent_trace.contains("read buildrequest.bytecode fingerprint"));
    assert!(agent_trace.contains("read buildrequest.target artifact"));
    assert!(agent_trace.contains("read buildrequest.target artifact fingerprint"));
    assert!(agent_trace.contains("read buildrequest.machine bytecode contract"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.dependency report"));
    assert!(agent_trace.contains("read buildrequest.dependency report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest fingerprint"));
    assert!(
        agent_trace.contains("write buildrequest.artifact manifest verification report=Verified")
    );
    assert!(agent_trace.contains("trace CompileManifestVerified"));
    let wasm_machine_contract_rule = "rule passed: the BuildRequest machine bytecode contract to be machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable or machine-bytecode-contract wasm32-unknown-sandbox-wasm bytecode-level portable-vm-contract bytecode-container wasm-sandbox-contract bytecode-format wasm32-contract-report or none";
    assert!(agent_trace.contains(wasm_machine_contract_rule));

    assert!(
        !artifact_dir
            .join("agent-VerifyCompileManifest.elf")
            .exists()
    );
    assert!(!artifact_dir.join("target.elf").exists());
    assert!(!artifact_dir.join("target.wasm").exists());
    assert!(!artifact_dir.join("native-bytecode-report.txt").exists());

    let contract_report =
        fs::read_to_string(artifact_dir.join("wasm-contract-report.txt")).unwrap();
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report
            .contains("machine-bytecode-dependency wasm-contract-report.txt portable-vm-contract")
    );
    assert!(!dependency_report.contains("agent-VerifyCompileManifest.elf"));
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(manifest.contains("action EmitTrace"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "wasm-contract wasm-contract-report.txt {}",
            fnv64_fingerprint(&contract_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "agent agent.ailbc.json {}",
            fnv64_fingerprint(&agent_bytecode)
        )),
        "{manifest}"
    );
    assert!(manifest.contains("trace agent-trace.txt"), "{manifest}");
    assert!(!manifest.contains("agent-target"));
    assert!(!manifest.contains("native-bytecode native-bytecode-report.txt"));
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_wasm_contract_agent_receives_contract_report_as_target_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-wasm-contract-agent-target-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let baseline_artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-agent-target-baseline-{}-{unique_suffix}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-agent-target-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let agent_bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-target-agent-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&baseline_artifact_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    let _ = fs::remove_file(&agent_bytecode_path);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: wasm-agent-target-artifact-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces
conformance: first-slice
target-support:
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Wasm Agent Target Artifact App manages target artifact state.

Action: Emit trace.

When emit trace happens:

- the system records a trace event named WasmAgentTargetArtifactCompiled
"#,
    )
    .unwrap();

    let baseline_output = Command::new(binary)
        .args([
            "ail-compile",
            root.to_str().unwrap(),
            "--action",
            "EmitTrace",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            baseline_artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        baseline_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&baseline_output.stdout),
        String::from_utf8_lossy(&baseline_output.stderr)
    );
    let expected_contract_report =
        fs::read_to_string(baseline_artifact_dir.join("wasm-contract-report.txt")).unwrap();
    let expected_contract_fingerprint = fnv64_fingerprint(&expected_contract_report);
    let expected_contract_line = "machine-bytecode-contract wasm32-unknown-sandbox-wasm bytecode-level portable-vm-contract bytecode-container wasm-sandbox-contract bytecode-format wasm32-contract-report";
    let agent_bytecode = format!(
        r#"{{
  "kind": "AIL-Bytecode",
  "package": "wasm-contract-target-agent",
  "version": "0.1.0",
  "profile": "Application",
  "target_support": {{}},
  "failures": [],
  "actions": [
    {{
      "action": "VerifyCompileManifest",
      "instructions": [
        {{"opcode":"ACTION_BEGIN","operands":{{"action":"VerifyCompileManifest"}}}},
        {{"opcode":"REQUIRE_EXISTS","operands":{{"key":"buildrequest.id","rule":"the BuildRequest to exist","failure":"NotFound"}}}},
        {{"opcode":"REQUIRE_FIELD_IN","operands":{{"key":"buildrequest.status","values":"BytecodeReady","rule":"the BuildRequest status to be BytecodeReady","failure":"RequirementFailed"}}}},
        {{"opcode":"REQUIRE_FIELD_IN","operands":{{"key":"buildrequest.target artifact","values":{},"rule":"the BuildRequest target artifact to be the Wasm contract report","failure":"RequirementFailed"}}}},
        {{"opcode":"REQUIRE_FIELD_IN","operands":{{"key":"buildrequest.target artifact fingerprint","values":{},"rule":"the BuildRequest target artifact fingerprint to match the Wasm contract report","failure":"RequirementFailed"}}}},
        {{"opcode":"REQUIRE_FIELD_IN","operands":{{"key":"buildrequest.machine bytecode contract","values":{},"rule":"the BuildRequest machine bytecode contract to be the Wasm contract report contract","failure":"RequirementFailed"}}}},
        {{"opcode":"READ_FIELD","operands":{{"key":"buildrequest.target artifact","text":"the BuildRequest target artifact"}}}},
        {{"opcode":"READ_FIELD","operands":{{"key":"buildrequest.target artifact fingerprint","text":"the BuildRequest target artifact fingerprint"}}}},
        {{"opcode":"SET_FIELD","operands":{{"key":"buildrequest.artifact manifest verification report","text":"the BuildRequest artifact manifest verification report to Verified","value":"Verified"}}}},
        {{"opcode":"EMIT_TRACE","operands":{{"event":"CompileManifestVerified"}}}},
        {{"opcode":"RETURN_SUCCESS","operands":{{}}}}
      ]
    }}
  ]
}}"#,
        json_string(&expected_contract_report),
        json_string(&expected_contract_fingerprint),
        json_string(expected_contract_line),
    );
    fs::write(&agent_bytecode_path, &agent_bytecode).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            root.to_str().unwrap(),
            "--action",
            "EmitTrace",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--agent",
            agent_bytecode_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(
        agent_trace.contains(
            "rule passed: the BuildRequest target artifact to be the Wasm contract report"
        ),
        "{agent_trace}"
    );
    assert!(
        agent_trace.contains("trace CompileManifestVerified"),
        "{agent_trace}"
    );

    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(baseline_artifact_dir).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
    fs::remove_file(agent_bytecode_path).unwrap();
}

#[test]
fn cli_ail_compile_saved_bytecode_and_core_file_wasm_contract_agents() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-wasm-contract-agent-boundaries-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-agent-boundaries-bytecode-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let core_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-agent-boundaries-core-{}-{unique_suffix}.ail-core.txt",
        std::process::id()
    ));
    let bytecode_artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-agent-boundaries-bytecode-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let core_artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-agent-boundaries-core-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_file(&core_path);
    let _ = fs::remove_dir_all(&bytecode_artifact_dir);
    let _ = fs::remove_dir_all(&core_artifact_dir);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: wasm-agent-boundaries-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces
conformance: first-slice
target-support:
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Wasm Agent Boundaries App manages portable artifact boundaries.

Action: Emit trace.

When emit trace happens:

- the system records a trace event named WasmAgentBoundaryCompiled
"#,
    )
    .unwrap();

    let lowered = Command::new(binary)
        .args(["ail-lower", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        lowered.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&lowered.stdout),
        String::from_utf8_lossy(&lowered.stderr)
    );
    fs::write(&bytecode_path, lowered.stdout).unwrap();
    let core_output = Command::new(binary)
        .args(["ail-core", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    fs::write(&core_path, &core_text).unwrap();

    for (input_args, artifact_dir) in [
        (
            vec![
                "ail-compile".to_string(),
                bytecode_path.to_string_lossy().into_owned(),
            ],
            bytecode_artifact_dir.clone(),
        ),
        (
            vec![
                "ail-compile".to_string(),
                "--core-file".to_string(),
                core_path.to_string_lossy().into_owned(),
            ],
            core_artifact_dir.clone(),
        ),
    ] {
        let mut args = input_args;
        args.extend([
            "--action".to_string(),
            "EmitTrace".to_string(),
            "--target".to_string(),
            "wasm32-unknown-sandbox-wasm".to_string(),
            "--agent".to_string(),
            agent_package.clone(),
            "--artifact-dir".to_string(),
            artifact_dir.to_string_lossy().into_owned(),
        ]);
        let output = Command::new(binary).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
        assert!(agent_trace.contains("action VerifyCompileManifest started"));
        assert!(agent_trace.contains("trace CompileManifestVerified"));
        let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
        let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
        assert!(
            manifest.contains(&format!(
                "agent agent.ailbc.json {}",
                fnv64_fingerprint(&agent_bytecode)
            )),
            "{manifest}"
        );
        assert!(manifest.contains("trace agent-trace.txt"), "{manifest}");
        assert!(!manifest.contains("agent-target"));
        assert!(
            !artifact_dir
                .join("agent-VerifyCompileManifest.elf")
                .exists()
        );
        assert!(!artifact_dir.join("native-bytecode-report.txt").exists());
    }
    let core_checked_core =
        fs::read_to_string(core_artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert_eq!(core_checked_core, core_text);
    assert!(!bytecode_artifact_dir.join("checked.ail-core.txt").exists());

    fs::remove_dir_all(root).unwrap();
    fs::remove_file(bytecode_path).unwrap();
    fs::remove_file(core_path).unwrap();
    fs::remove_dir_all(bytecode_artifact_dir).unwrap();
    fs::remove_dir_all(core_artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_wasm_contract_all_actions_agent_verifies_bundle_manifest() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-all-actions-agent-bytecode-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-all-actions-agent-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_dir_all(&artifact_dir);
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "wasm-all-actions-agent-example",
  "version": "0.1.0",
  "profile": "Application",
  "target_support": {"wasm32-unknown-sandbox-wasm":"supported-with-host-imports"},
  "failures": [],
  "actions": [
    {
      "action": "EmitTrace",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"EmitTrace"}},
        {"opcode":"EMIT_TRACE","operands":{"event":"WasmAllActionsAgentVerified"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    fs::write(&bytecode_path, bytecode_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--all-actions",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("ail-compile wrote wasm32-unknown-sandbox-wasm contract bundle"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let contract_report =
        fs::read_to_string(artifact_dir.join("wasm-contract-report.txt")).unwrap();
    assert!(contract_report.contains("bundle all-actions"));
    assert!(
        contract_report.contains("action EmitTrace trace-preservation required"),
        "{contract_report}"
    );
    let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    assert!(agent_bytecode.contains(r#""action":"VerifyCompileBundleManifest""#));
    let agent_fingerprint = fs::read_to_string(artifact_dir.join("agent.fingerprint.txt")).unwrap();
    assert_eq!(agent_fingerprint.trim(), fnv64_fingerprint(&agent_bytecode));
    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action VerifyCompileBundleManifest started"));
    assert!(agent_trace.contains("read buildrequest.target artifact"));
    assert!(agent_trace.contains("read buildrequest.target artifact fingerprint"));
    assert!(agent_trace.contains("read buildrequest.machine bytecode contract"));
    assert!(agent_trace.contains("trace CompileBundleManifestVerified"));
    let wasm_machine_contract_rule = "rule passed: the BuildRequest machine bytecode contract to be machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable or machine-bytecode-contract wasm32-unknown-sandbox-wasm bytecode-level portable-vm-contract bytecode-container wasm-sandbox-contract bytecode-format wasm32-contract-report or none";
    assert!(agent_trace.contains(wasm_machine_contract_rule));
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(manifest.contains("bundle all-actions"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "agent agent.ailbc.json {}",
            fnv64_fingerprint(&agent_bytecode)
        )),
        "{manifest}"
    );
    assert!(manifest.contains("trace agent-trace.txt"), "{manifest}");
    assert!(!manifest.contains("agent-target"));
    assert!(!manifest.contains("native-bytecode native-bytecode-report.txt"));
    assert!(
        !artifact_dir
            .join("agent-VerifyCompileBundleManifest.elf")
            .exists()
    );
    assert!(!artifact_dir.join("target-EmitTrace.elf").exists());
    assert!(!artifact_dir.join("native-bytecode-report.txt").exists());

    fs::remove_file(bytecode_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_wasm_contract_all_actions_agent_accepts_package_and_core_routes() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "ail-wasm-contract-all-actions-agent-routes-package-{}-{unique_suffix}",
        std::process::id()
    ));
    let core_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-all-actions-agent-routes-core-{}-{unique_suffix}.ail-core.txt",
        std::process::id()
    ));
    let package_artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-all-actions-agent-routes-package-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let core_artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-all-actions-agent-routes-core-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_file(&core_path);
    let _ = fs::remove_dir_all(&package_artifact_dir);
    let _ = fs::remove_dir_all(&core_artifact_dir);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("ail-package.md"),
        r#"name: wasm-all-actions-agent-routes-app
version: 0.1.0
profile: Application
entry: spec.ail-spec.md
features: actions, traces
conformance: first-slice
target-support:
  wasm32-unknown-sandbox-wasm: supported-with-host-imports
"#,
    )
    .unwrap();
    fs::write(
        root.join("spec.ail-spec.md"),
        r#"The application Wasm All Actions Agent Routes App manages portable bundles.

Action: Emit first trace.

When emit first trace happens:

- the system records a trace event named WasmAllActionsAgentRouteFirst

Action: Emit second trace.

When emit second trace happens:

- the system records a trace event named WasmAllActionsAgentRouteSecond
"#,
    )
    .unwrap();

    let core_output = Command::new(binary)
        .args(["ail-core", root.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_text = String::from_utf8(core_output.stdout).unwrap();
    fs::write(&core_path, &core_text).unwrap();

    for (input_args, artifact_dir) in [
        (
            vec![
                "ail-compile".to_string(),
                root.to_string_lossy().into_owned(),
            ],
            package_artifact_dir.clone(),
        ),
        (
            vec![
                "ail-compile".to_string(),
                "--core-file".to_string(),
                core_path.to_string_lossy().into_owned(),
            ],
            core_artifact_dir.clone(),
        ),
    ] {
        let mut args = input_args;
        args.extend([
            "--all-actions".to_string(),
            "--target".to_string(),
            "wasm32-unknown-sandbox-wasm".to_string(),
            "--agent".to_string(),
            agent_package.clone(),
            "--artifact-dir".to_string(),
            artifact_dir.to_string_lossy().into_owned(),
        ]);
        let output = Command::new(binary).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains("ail-compile wrote wasm32-unknown-sandbox-wasm contract bundle"),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );

        let contract_report =
            fs::read_to_string(artifact_dir.join("wasm-contract-report.txt")).unwrap();
        assert!(contract_report.contains("bundle all-actions"));
        assert!(
            contract_report.contains("action EmitFirstTrace trace-preservation required"),
            "{contract_report}"
        );
        assert!(
            contract_report.contains("action EmitSecondTrace trace-preservation required"),
            "{contract_report}"
        );
        let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
        assert!(agent_bytecode.contains(r#""action":"VerifyCompileBundleManifest""#));
        let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
        assert!(agent_trace.contains("action VerifyCompileBundleManifest started"));
        assert!(agent_trace.contains("read buildrequest.target artifact"));
        assert!(agent_trace.contains("read buildrequest.target artifact fingerprint"));
        assert!(agent_trace.contains("trace CompileBundleManifestVerified"));
        let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
        assert!(manifest.contains("bundle all-actions"), "{manifest}");
        assert!(
            manifest.contains(&format!(
                "agent agent.ailbc.json {}",
                fnv64_fingerprint(&agent_bytecode)
            )),
            "{manifest}"
        );
        assert!(manifest.contains("trace agent-trace.txt"), "{manifest}");
        assert!(!manifest.contains("agent-target"));
        assert!(!manifest.contains("native-bytecode native-bytecode-report.txt"));
        assert!(
            !artifact_dir
                .join("agent-VerifyCompileBundleManifest.elf")
                .exists()
        );
        assert!(!artifact_dir.join("target-EmitFirstTrace.elf").exists());
        assert!(!artifact_dir.join("native-bytecode-report.txt").exists());
    }

    let source_manifest =
        fs::read_to_string(package_artifact_dir.join("source.ail-package.md")).unwrap();
    assert_eq!(
        source_manifest,
        fs::read_to_string(root.join("ail-package.md")).unwrap()
    );
    let source_spec = fs::read_to_string(package_artifact_dir.join("source.ail-spec.md")).unwrap();
    assert_eq!(
        source_spec,
        fs::read_to_string(root.join("spec.ail-spec.md")).unwrap()
    );
    let source_bundle =
        format!("ail-package.md:\n{source_manifest}\nspec.ail-spec.md:\n{source_spec}");
    let package_manifest =
        fs::read_to_string(package_artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(
        package_manifest.contains(&format!(
            "source-package source.ail-package.md source.ail-spec.md {}",
            fnv64_fingerprint(&source_bundle)
        )),
        "{package_manifest}"
    );
    let package_trace = fs::read_to_string(package_artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(package_trace.contains("read buildrequest.source package"));
    assert!(package_trace.contains("read buildrequest.source package fingerprint"));

    let core_checked_core =
        fs::read_to_string(core_artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert_eq!(core_checked_core, core_text);
    let core_manifest =
        fs::read_to_string(core_artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(!core_manifest.contains("source-package"));
    assert!(!core_artifact_dir.join("source.ail-package.md").exists());

    fs::remove_dir_all(root).unwrap();
    fs::remove_file(core_path).unwrap();
    fs::remove_dir_all(package_artifact_dir).unwrap();
    fs::remove_dir_all(core_artifact_dir).unwrap();
}

#[test]
fn cli_ail_compile_wasm_contract_all_actions_agent_receives_contract_report_as_target_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-all-actions-agent-target-bytecode-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let baseline_artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-all-actions-agent-target-baseline-{}-{unique_suffix}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-wasm-contract-all-actions-agent-target-artifacts-{}-{unique_suffix}",
        std::process::id()
    ));
    let agent_bytecode_path = std::env::temp_dir().join(format!(
        "ail-wasm-contract-all-actions-target-agent-{}-{unique_suffix}.ailbc.json",
        std::process::id()
    ));
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_dir_all(&baseline_artifact_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    let _ = fs::remove_file(&agent_bytecode_path);
    let bytecode_text = r#"{
  "kind": "AIL-Bytecode",
  "package": "wasm-all-actions-target-agent-example",
  "version": "0.1.0",
  "profile": "Application",
  "target_support": {"wasm32-unknown-sandbox-wasm":"supported-with-host-imports"},
  "failures": [],
  "actions": [
    {
      "action": "EmitTrace",
      "instructions": [
        {"opcode":"ACTION_BEGIN","operands":{"action":"EmitTrace"}},
        {"opcode":"EMIT_TRACE","operands":{"event":"WasmAllActionsTargetAgentVerified"}},
        {"opcode":"RETURN_SUCCESS","operands":{}}
      ]
    }
  ]
}"#;
    fs::write(&bytecode_path, bytecode_text).unwrap();

    let baseline_output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--all-actions",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--artifact-dir",
            baseline_artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        baseline_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&baseline_output.stdout),
        String::from_utf8_lossy(&baseline_output.stderr)
    );
    let expected_contract_report =
        fs::read_to_string(baseline_artifact_dir.join("wasm-contract-report.txt")).unwrap();
    let expected_contract_fingerprint = fnv64_fingerprint(&expected_contract_report);
    let expected_contract_line = "machine-bytecode-contract wasm32-unknown-sandbox-wasm bytecode-level portable-vm-contract bytecode-container wasm-sandbox-contract bytecode-format wasm32-contract-report";
    let agent_bytecode = format!(
        r#"{{
  "kind": "AIL-Bytecode",
  "package": "wasm-contract-bundle-target-agent",
  "version": "0.1.0",
  "profile": "Application",
  "target_support": {{}},
  "failures": [],
  "actions": [
    {{
      "action": "VerifyCompileBundleManifest",
      "instructions": [
        {{"opcode":"ACTION_BEGIN","operands":{{"action":"VerifyCompileBundleManifest"}}}},
        {{"opcode":"REQUIRE_EXISTS","operands":{{"key":"buildrequest.id","rule":"the BuildRequest to exist","failure":"NotFound"}}}},
        {{"opcode":"REQUIRE_FIELD_IN","operands":{{"key":"buildrequest.status","values":"BytecodeReady","rule":"the BuildRequest status to be BytecodeReady","failure":"RequirementFailed"}}}},
        {{"opcode":"REQUIRE_FIELD_IN","operands":{{"key":"buildrequest.target artifact","values":{},"rule":"the BuildRequest target artifact to be the Wasm contract bundle report","failure":"RequirementFailed"}}}},
        {{"opcode":"REQUIRE_FIELD_IN","operands":{{"key":"buildrequest.target artifact fingerprint","values":{},"rule":"the BuildRequest target artifact fingerprint to match the Wasm contract bundle report","failure":"RequirementFailed"}}}},
        {{"opcode":"REQUIRE_FIELD_IN","operands":{{"key":"buildrequest.machine bytecode contract","values":{},"rule":"the BuildRequest machine bytecode contract to be the Wasm contract report contract","failure":"RequirementFailed"}}}},
        {{"opcode":"READ_FIELD","operands":{{"key":"buildrequest.target artifact","text":"the BuildRequest target artifact"}}}},
        {{"opcode":"READ_FIELD","operands":{{"key":"buildrequest.target artifact fingerprint","text":"the BuildRequest target artifact fingerprint"}}}},
        {{"opcode":"SET_FIELD","operands":{{"key":"buildrequest.artifact manifest verification report","text":"the BuildRequest artifact manifest verification report to Verified","value":"Verified"}}}},
        {{"opcode":"EMIT_TRACE","operands":{{"event":"CompileBundleManifestVerified"}}}},
        {{"opcode":"RETURN_SUCCESS","operands":{{}}}}
      ]
    }}
  ]
}}"#,
        json_string(&expected_contract_report),
        json_string(&expected_contract_fingerprint),
        json_string(expected_contract_line),
    );
    fs::write(&agent_bytecode_path, &agent_bytecode).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--all-actions",
            "--target",
            "wasm32-unknown-sandbox-wasm",
            "--agent",
            agent_bytecode_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(
        agent_trace.contains(
            "rule passed: the BuildRequest target artifact to be the Wasm contract bundle report"
        ),
        "{agent_trace}"
    );
    assert!(
        agent_trace
            .contains("rule passed: the BuildRequest target artifact fingerprint to match the Wasm contract bundle report"),
        "{agent_trace}"
    );
    assert!(
        agent_trace.contains("trace CompileBundleManifestVerified"),
        "{agent_trace}"
    );

    fs::remove_file(bytecode_path).unwrap();
    fs::remove_dir_all(baseline_artifact_dir).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
    fs::remove_file(agent_bytecode_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_agent_verifies_manifest_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let bytecode_path = std::env::temp_dir().join(format!(
        "ail-ail-compile-agent-manifest-{}.ailbc.json",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-ail-compile-agent-manifest-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-compile-agent-manifest-dir-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&bytecode_path);
    let _ = fs::remove_file(&executable_path);
    let _ = fs::remove_dir_all(&artifact_dir);

    let lowered = Command::new(binary)
        .args(["ail-lower", &package])
        .output()
        .unwrap();
    assert!(
        lowered.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&lowered.stdout),
        String::from_utf8_lossy(&lowered.stderr)
    );
    fs::write(&bytecode_path, lowered.stdout).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            bytecode_path.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    assert!(agent_bytecode.contains(r#""package":"ail-toolchain-agent""#));
    assert!(agent_bytecode.contains(r#""action":"VerifyCompileManifest""#));
    let agent_fingerprint = fs::read_to_string(artifact_dir.join("agent.fingerprint.txt")).unwrap();
    assert_eq!(agent_fingerprint.trim(), fnv64_fingerprint(&agent_bytecode));
    let parsed_agent = parse_ail_bytecode(&agent_bytecode).unwrap();
    assert_eq!(verify_ail_bytecode(&parsed_agent), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action VerifyCompileManifest started"));
    assert!(agent_trace.contains("read buildrequest.bytecode fingerprint"));
    assert!(agent_trace.contains("read buildrequest.target artifact"));
    assert!(agent_trace.contains("read buildrequest.target artifact fingerprint"));
    let machine_bytecode_contract_rule = "rule passed: the BuildRequest machine bytecode contract to be machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable or machine-bytecode-contract wasm32-unknown-sandbox-wasm bytecode-level portable-vm-contract bytecode-container wasm-sandbox-contract bytecode-format wasm32-contract-report or none";
    assert!(agent_trace.contains(machine_bytecode_contract_rule));
    assert!(agent_trace.contains("read buildrequest.machine bytecode contract"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.dependency report"));
    assert!(agent_trace.contains("read buildrequest.dependency report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest fingerprint"));
    assert!(
        agent_trace.contains("write buildrequest.artifact manifest verification report=Verified")
    );
    assert!(agent_trace.contains("trace CompileManifestVerified"));

    let agent_native = fs::read(artifact_dir.join("agent-VerifyCompileManifest.elf")).unwrap();
    assert_eq!(&agent_native[0..4], b"\x7fELF");
    let expected_agent_native_fingerprint = fnv64_fingerprint_bytes(&agent_native);
    let native_agent_run = Command::new(artifact_dir.join("agent-VerifyCompileManifest.elf"))
        .args([
            "buildrequest.id=support-ticket-compile",
            "buildrequest.status=BytecodeReady",
            "buildrequest.bytecode fingerprint=fnv64:bytecode",
            "buildrequest.target artifact=ok",
            "buildrequest.target artifact fingerprint=fnv64:target",
            "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
            "buildrequest.native bytecode report=ok",
            "buildrequest.native bytecode report fingerprint=fnv64:native-bytecode",
            "buildrequest.dependency report=ok",
            "buildrequest.dependency report fingerprint=fnv64:dependencies",
            "buildrequest.artifact manifest=ok",
            "buildrequest.artifact manifest fingerprint=fnv64:manifest",
        ])
        .output()
        .unwrap();
    assert!(
        native_agent_run.status.success(),
        "native compile manifest verifier failed"
    );
    assert!(
        String::from_utf8_lossy(&native_agent_run.stderr).contains("trace CompileManifestVerified"),
        "{}",
        String::from_utf8_lossy(&native_agent_run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&native_agent_run.stderr).contains(machine_bytecode_contract_rule),
        "{}",
        String::from_utf8_lossy(&native_agent_run.stderr)
    );
    let bad_contract_run = Command::new(artifact_dir.join("agent-VerifyCompileManifest.elf"))
        .args([
            "buildrequest.id=support-ticket-compile",
            "buildrequest.status=BytecodeReady",
            "buildrequest.bytecode fingerprint=fnv64:bytecode",
            "buildrequest.target artifact=ok",
            "buildrequest.target artifact fingerprint=fnv64:target",
            "buildrequest.machine bytecode contract=wrong-contract",
            "buildrequest.native bytecode report=ok",
            "buildrequest.native bytecode report fingerprint=fnv64:native-bytecode",
            "buildrequest.dependency report=ok",
            "buildrequest.dependency report fingerprint=fnv64:dependencies",
            "buildrequest.artifact manifest=ok",
            "buildrequest.artifact manifest fingerprint=fnv64:manifest",
        ])
        .output()
        .unwrap();
    assert!(
        !bad_contract_run.status.success(),
        "native compile manifest verifier accepted a bad machine bytecode contract\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bad_contract_run.stdout),
        String::from_utf8_lossy(&bad_contract_run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&bad_contract_run.stderr).contains("failure RequirementFailed"),
        "{}",
        String::from_utf8_lossy(&bad_contract_run.stderr)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains(
            "machine-bytecode-dependency agent-VerifyCompileManifest.elf standalone-linux-syscall-elf"
        ),
        "{dependency_report}"
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "agent agent.ailbc.json {}",
            fnv64_fingerprint(&agent_bytecode)
        )),
        "{manifest}"
    );
    assert!(manifest.contains("trace agent-trace.txt"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "agent-target linux-x86_64-elf agent-VerifyCompileManifest.elf {expected_agent_native_fingerprint}"
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_file(bytecode_path).unwrap();
    fs::remove_file(executable_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_package_agent_records_source_package_fingerprints() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let executable_path = std::env::temp_dir().join(format!(
        "ail-ail-compile-package-source-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-compile-package-source-dir-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let source_manifest = fs::read_to_string(artifact_dir.join("source.ail-package.md")).unwrap();
    assert_eq!(
        source_manifest,
        fs::read_to_string(format!("{package}/ail-package.md")).unwrap()
    );
    let source_spec = fs::read_to_string(artifact_dir.join("source.ail-spec.md")).unwrap();
    assert_eq!(
        source_spec,
        fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap()
    );
    let source_bundle =
        format!("ail-package.md:\n{source_manifest}\nspec.ail-spec.md:\n{source_spec}");
    let source_fingerprint =
        fs::read_to_string(artifact_dir.join("source.fingerprint.txt")).unwrap();
    assert_eq!(source_fingerprint.trim(), fnv64_fingerprint(&source_bundle));

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action VerifyCompileManifest started"));
    assert!(agent_trace.contains("read buildrequest.source package"));
    assert!(agent_trace.contains("read buildrequest.source package fingerprint"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report"));
    assert!(agent_trace.contains("trace CompileManifestVerified"));

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    let target_artifact = fs::read(artifact_dir.join("target.elf")).unwrap();
    assert_eq!(&target_artifact[0..4], b"\x7fELF");
    assert_eq!(target_artifact, fs::read(&executable_path).unwrap());
    let native_bytecode_report =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.txt")).unwrap();
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "source-package source.ail-package.md source.ail-spec.md {}",
            fnv64_fingerprint(&source_bundle)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bytecode artifact.ailbc.json {}",
            fnv64_fingerprint(&bytecode_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "target linux-x86_64-elf target.elf {}",
            fnv64_fingerprint_bytes(&target_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "native-bytecode native-bytecode-report.txt {}",
            fnv64_fingerprint(&native_bytecode_report)
        )),
        "{manifest}"
    );

    fs::remove_file(executable_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_writes_all_action_native_bundle() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-compile-all-actions-dir-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--all-actions",
            "--target",
            "linux-x86_64-elf",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("ail-compile wrote linux-x86_64-elf native bundle"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert!(bytecode_artifact.contains(r#""package":"ail-toolchain-agent""#));
    assert!(bytecode_artifact.contains(r#""action":"CompileApplication""#));
    assert!(bytecode_artifact.contains(r#""action":"VerifyBuildManifest""#));
    let source_manifest = fs::read_to_string(artifact_dir.join("source.ail-package.md")).unwrap();
    assert_eq!(
        source_manifest,
        fs::read_to_string(format!("{package}/ail-package.md")).unwrap()
    );
    let source_spec = fs::read_to_string(artifact_dir.join("source.ail-spec.md")).unwrap();
    assert_eq!(
        source_spec,
        fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap()
    );
    let source_bundle =
        format!("ail-package.md:\n{source_manifest}\nspec.ail-spec.md:\n{source_spec}");
    let source_fingerprint =
        fs::read_to_string(artifact_dir.join("source.fingerprint.txt")).unwrap();
    assert_eq!(source_fingerprint.trim(), fnv64_fingerprint(&source_bundle));
    let bytecode_fingerprint =
        fs::read_to_string(artifact_dir.join("artifact.fingerprint.txt")).unwrap();
    assert_eq!(
        bytecode_fingerprint.trim(),
        fnv64_fingerprint(&bytecode_artifact)
    );

    let compile_application = fs::read(artifact_dir.join("target-CompileApplication.elf")).unwrap();
    assert_eq!(&compile_application[0..4], b"\x7fELF");
    let expected_compile_application_fingerprint = fnv64_fingerprint_bytes(&compile_application);
    let verify_manifest = fs::read(artifact_dir.join("target-VerifyBuildManifest.elf")).unwrap();
    assert_eq!(&verify_manifest[0..4], b"\x7fELF");
    let expected_manifest_fingerprint = fnv64_fingerprint_bytes(&verify_manifest);
    let native_bytecode_report =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.txt")).unwrap();
    assert!(
        native_bytecode_report.contains("AIL-Compile-Bundle-Native-Bytecode:"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("target linux-x86_64-elf"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("bundle all-actions"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains(&format!(
            "machine-bytecode target linux-x86_64-elf target-CompileApplication.elf elf64-little-x86_64-executable {} bytes {}",
            expected_compile_application_fingerprint,
            compile_application.len()
        )),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains(&format!(
            "machine-bytecode target linux-x86_64-elf target-VerifyBuildManifest.elf elf64-little-x86_64-executable {} bytes {}",
            expected_manifest_fingerprint,
            verify_manifest.len()
        )),
        "{native_bytecode_report}"
    );
    let native_bytecode_report_fingerprint =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.fingerprint.txt")).unwrap();
    assert_eq!(
        native_bytecode_report_fingerprint.trim(),
        fnv64_fingerprint(&native_bytecode_report)
    );
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Compile-Bundle-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("target linux-x86_64-elf"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("bundle all-actions"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("host-language-runtime none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("dynamic-linker none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("shared-libraries none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("library-dependencies none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("linker-invocation none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains(
            "machine-bytecode-dependency target-CompileApplication.elf standalone-linux-syscall-elf"
        ),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains(
            "machine-bytecode-dependency target-VerifyBuildManifest.elf standalone-linux-syscall-elf"
        ),
        "{dependency_report}"
    );
    let dependency_report_fingerprint =
        fs::read_to_string(artifact_dir.join("dependency-report.fingerprint.txt")).unwrap();
    assert_eq!(
        dependency_report_fingerprint.trim(),
        fnv64_fingerprint(&dependency_report)
    );

    let native_run = Command::new(artifact_dir.join("target-VerifyBuildManifest.elf"))
        .args([
            "buildrequest.id=BR-1",
            "buildrequest.status=BytecodeReady",
            "buildrequest.core ir fingerprint=fnv64:core",
            "buildrequest.bytecode fingerprint=fnv64:bytecode",
            "buildrequest.target artifact fingerprint=fnv64:target",
            "buildrequest.compiler pass target artifact fingerprint=fnv64:pass-target",
            "buildrequest.prompt portability report fingerprint=fnv64:prompt-portability",
            "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
            "buildrequest.dependency report=ok",
            "buildrequest.dependency report fingerprint=fnv64:dependencies",
            "buildrequest.artifact manifest=ok",
            "buildrequest.artifact manifest fingerprint=fnv64:manifest",
        ])
        .output()
        .unwrap();
    assert!(
        native_run.status.success(),
        "native bundle manifest verifier failed"
    );
    assert!(
        String::from_utf8_lossy(&native_run.stderr).contains("trace BuildManifestVerified"),
        "{}",
        String::from_utf8_lossy(&native_run.stderr)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    assert!(manifest.contains("AIL-Compile-Manifest:"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "source-package source.ail-package.md source.ail-spec.md {}",
            fnv64_fingerprint(&source_bundle)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bytecode artifact.ailbc.json {}",
            fnv64_fingerprint(&bytecode_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "target linux-x86_64-elf target-VerifyBuildManifest.elf {expected_manifest_fingerprint}"
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "native-bytecode native-bytecode-report.txt {}",
            fnv64_fingerprint(&native_bytecode_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_agent_verifies_all_action_native_bundle() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-compile-all-actions-agent-dir-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--all-actions",
            "--target",
            "linux-x86_64-elf",
            "--agent",
            &package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert!(bytecode_artifact.contains(r#""action":"VerifyCompileBundleManifest""#));
    let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    assert_eq!(agent_bytecode, bytecode_artifact);
    let agent_fingerprint = fs::read_to_string(artifact_dir.join("agent.fingerprint.txt")).unwrap();
    assert_eq!(agent_fingerprint.trim(), fnv64_fingerprint(&agent_bytecode));

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action VerifyCompileBundleManifest started"));
    assert!(agent_trace.contains("read buildrequest.bytecode fingerprint"));
    assert!(agent_trace.contains("read buildrequest.source package"));
    assert!(agent_trace.contains("read buildrequest.source package fingerprint"));
    assert!(agent_trace.contains("read buildrequest.target artifact"));
    assert!(agent_trace.contains("read buildrequest.target artifact fingerprint"));
    assert!(agent_trace.contains("read buildrequest.machine bytecode contract"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.dependency report"));
    assert!(agent_trace.contains("read buildrequest.dependency report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest fingerprint"));
    assert!(
        agent_trace.contains("write buildrequest.artifact manifest verification report=Verified")
    );
    assert!(agent_trace.contains("trace CompileBundleManifestVerified"));

    let target_verifier =
        fs::read(artifact_dir.join("target-VerifyCompileBundleManifest.elf")).unwrap();
    assert_eq!(&target_verifier[0..4], b"\x7fELF");
    let agent_verifier =
        fs::read(artifact_dir.join("agent-VerifyCompileBundleManifest.elf")).unwrap();
    assert_eq!(&agent_verifier[0..4], b"\x7fELF");
    let expected_agent_verifier_fingerprint = fnv64_fingerprint_bytes(&agent_verifier);
    let native_agent_run = Command::new(artifact_dir.join("agent-VerifyCompileBundleManifest.elf"))
        .args([
            "buildrequest.id=ail-toolchain-agent-compile-bundle",
            "buildrequest.status=BytecodeReady",
            "buildrequest.bytecode fingerprint=fnv64:bytecode",
            "buildrequest.target artifact=bundle",
            "buildrequest.target artifact fingerprint=fnv64:target-bundle",
            "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
            "buildrequest.native bytecode report=ok",
            "buildrequest.native bytecode report fingerprint=fnv64:native-bytecode",
            "buildrequest.dependency report=ok",
            "buildrequest.dependency report fingerprint=fnv64:dependencies",
            "buildrequest.artifact manifest=ok",
            "buildrequest.artifact manifest fingerprint=fnv64:manifest",
        ])
        .output()
        .unwrap();
    assert!(
        native_agent_run.status.success(),
        "native compile bundle verifier failed"
    );
    assert!(
        String::from_utf8_lossy(&native_agent_run.stderr)
            .contains("trace CompileBundleManifestVerified"),
        "{}",
        String::from_utf8_lossy(&native_agent_run.stderr)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-compile.txt")).unwrap();
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains(
            "machine-bytecode-dependency agent-VerifyCompileBundleManifest.elf standalone-linux-syscall-elf"
        ),
        "{dependency_report}"
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    assert!(manifest.contains("bundle all-actions"), "{manifest}");
    let source_manifest = fs::read_to_string(artifact_dir.join("source.ail-package.md")).unwrap();
    let source_spec = fs::read_to_string(artifact_dir.join("source.ail-spec.md")).unwrap();
    let source_bundle =
        format!("ail-package.md:\n{source_manifest}\nspec.ail-spec.md:\n{source_spec}");
    assert!(
        manifest.contains(&format!(
            "source-package source.ail-package.md source.ail-spec.md {}",
            fnv64_fingerprint(&source_bundle)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "agent agent.ailbc.json {}",
            fnv64_fingerprint(&agent_bytecode)
        )),
        "{manifest}"
    );
    assert!(manifest.contains("trace agent-trace.txt"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "agent-target linux-x86_64-elf agent-VerifyCompileBundleManifest.elf {expected_agent_verifier_fingerprint}"
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_bootstrap_writes_native_toolchain_bundle() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let toolchain_package = fixture("ail_toolchain_agent.ail");
    let compiler_pass = fixture("compiler_pass.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-bootstrap-native-bundle-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-bootstrap",
            &toolchain_package,
            "--pass",
            &compiler_pass,
            "--target",
            "linux-x86_64-elf",
            "--agent",
            &toolchain_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("ail-bootstrap wrote linux-x86_64-elf bootstrap bundle"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );

    let toolchain_bytecode =
        fs::read_to_string(artifact_dir.join("toolchain-agent.ailbc.json")).unwrap();
    assert!(toolchain_bytecode.contains(r#""package":"ail-toolchain-agent""#));
    assert!(toolchain_bytecode.contains(r#""action":"CompileNativeTarget""#));
    assert!(toolchain_bytecode.contains(r#""action":"VerifyBootstrapManifest""#));
    let toolchain_fingerprint =
        fs::read_to_string(artifact_dir.join("toolchain-agent.fingerprint.txt")).unwrap();
    assert_eq!(
        toolchain_fingerprint.trim(),
        fnv64_fingerprint(&toolchain_bytecode)
    );
    let toolchain_source_manifest =
        fs::read_to_string(artifact_dir.join("toolchain-agent.source.ail-package.md")).unwrap();
    assert!(
        toolchain_source_manifest.contains("name: ail-toolchain-agent"),
        "{toolchain_source_manifest}"
    );
    let toolchain_source_spec =
        fs::read_to_string(artifact_dir.join("toolchain-agent.source.ail-spec.md")).unwrap();
    assert!(
        toolchain_source_spec.contains("Action: Verify bootstrap manifest."),
        "{toolchain_source_spec}"
    );
    let toolchain_source_fingerprint =
        fs::read_to_string(artifact_dir.join("toolchain-agent.source.fingerprint.txt")).unwrap();
    let toolchain_source_bundle = format!(
        "ail-package.md:\n{toolchain_source_manifest}\nspec.ail-spec.md:\n{toolchain_source_spec}"
    );
    assert_eq!(
        toolchain_source_fingerprint.trim(),
        fnv64_fingerprint(&toolchain_source_bundle)
    );
    let toolchain_core =
        fs::read_to_string(artifact_dir.join("toolchain-agent.checked.ail-core.txt")).unwrap();
    assert!(
        toolchain_core.contains("package: ail-toolchain-agent"),
        "{toolchain_core}"
    );
    assert!(
        toolchain_core.contains("node Action VerifyBootstrapManifest"),
        "{toolchain_core}"
    );
    let toolchain_core_fingerprint =
        fs::read_to_string(artifact_dir.join("toolchain-agent.core.fingerprint.txt")).unwrap();
    assert_eq!(
        toolchain_core_fingerprint.trim(),
        fnv64_fingerprint(&toolchain_core)
    );

    let pass_bytecode = fs::read_to_string(artifact_dir.join("compiler-pass.ailbc.json")).unwrap();
    assert!(pass_bytecode.contains(r#""package":"ail-meta-permissions""#));
    assert!(pass_bytecode.contains(r#""action":"InferReadPermissions""#));
    let pass_fingerprint =
        fs::read_to_string(artifact_dir.join("compiler-pass.fingerprint.txt")).unwrap();
    assert_eq!(pass_fingerprint.trim(), fnv64_fingerprint(&pass_bytecode));
    let pass_source_manifest =
        fs::read_to_string(artifact_dir.join("compiler-pass.source.ail-package.md")).unwrap();
    assert!(
        pass_source_manifest.contains("name: ail-meta-permissions"),
        "{pass_source_manifest}"
    );
    let pass_source_spec =
        fs::read_to_string(artifact_dir.join("compiler-pass.source.ail-spec.md")).unwrap();
    assert!(
        pass_source_spec.contains("Compiler pass: Infer read permissions."),
        "{pass_source_spec}"
    );
    let pass_source_fingerprint =
        fs::read_to_string(artifact_dir.join("compiler-pass.source.fingerprint.txt")).unwrap();
    let pass_source_bundle =
        format!("ail-package.md:\n{pass_source_manifest}\nspec.ail-spec.md:\n{pass_source_spec}");
    assert_eq!(
        pass_source_fingerprint.trim(),
        fnv64_fingerprint(&pass_source_bundle)
    );
    let pass_core =
        fs::read_to_string(artifact_dir.join("compiler-pass.checked.ail-core.txt")).unwrap();
    assert!(
        pass_core.contains("package: ail-meta-permissions"),
        "{pass_core}"
    );
    assert!(
        pass_core.contains("node Action InferReadPermissions [kind=CompilerPass"),
        "{pass_core}"
    );
    let pass_core_fingerprint =
        fs::read_to_string(artifact_dir.join("compiler-pass.core.fingerprint.txt")).unwrap();
    assert_eq!(pass_core_fingerprint.trim(), fnv64_fingerprint(&pass_core));

    let toolchain_pass_output =
        fs::read_to_string(artifact_dir.join("toolchain-agent.pass-output.ail-core.txt")).unwrap();
    assert!(
        toolchain_pass_output.contains("package: ail-toolchain-agent"),
        "{toolchain_pass_output}"
    );
    assert!(
        toolchain_pass_output
            .contains("node Provenance compiler_pass:InferReadPermissions.permission:"),
        "{toolchain_pass_output}"
    );
    let toolchain_pass_output_fingerprint =
        fs::read_to_string(artifact_dir.join("toolchain-agent.pass-output.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        toolchain_pass_output_fingerprint.trim(),
        fnv64_fingerprint(&toolchain_pass_output)
    );
    let toolchain_pass_trace =
        fs::read_to_string(artifact_dir.join("toolchain-agent.pass-trace.txt")).unwrap();
    assert!(
        toolchain_pass_trace.contains("core transform infer read permissions"),
        "{toolchain_pass_trace}"
    );
    let toolchain_pass_trace_fingerprint =
        fs::read_to_string(artifact_dir.join("toolchain-agent.pass-trace.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        toolchain_pass_trace_fingerprint.trim(),
        fnv64_fingerprint(&toolchain_pass_trace)
    );
    let parsed_pass_output = parse_ail_core_text(&toolchain_pass_output).unwrap();
    let expected_toolchain_bytecode =
        render_ail_bytecode(&compile_ail_core_bytecode(&parsed_pass_output).unwrap());
    assert_eq!(
        toolchain_bytecode,
        format!("{expected_toolchain_bytecode}\n")
    );
    let fixed_point_report =
        fs::read_to_string(artifact_dir.join("bootstrap-fixed-point-report.txt")).unwrap();
    assert!(
        fixed_point_report.contains("fixed-point: ok"),
        "{fixed_point_report}"
    );
    assert!(
        fixed_point_report.contains(&format!(
            "first-pass-output {}",
            fnv64_fingerprint(&toolchain_pass_output)
        )),
        "{fixed_point_report}"
    );
    assert!(
        fixed_point_report.contains(&format!(
            "second-pass-output {}",
            fnv64_fingerprint(&toolchain_pass_output)
        )),
        "{fixed_point_report}"
    );
    assert!(
        fixed_point_report.contains("second-pass-changed false"),
        "{fixed_point_report}"
    );
    let fixed_point_fingerprint =
        fs::read_to_string(artifact_dir.join("bootstrap-fixed-point-report.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        fixed_point_fingerprint.trim(),
        fnv64_fingerprint(&fixed_point_report)
    );

    let toolchain_conformance =
        fs::read_to_string(artifact_dir.join("toolchain-agent-conformance-report.txt")).unwrap();
    assert!(
        toolchain_conformance.contains("ail conformance: package ail-toolchain-agent"),
        "{toolchain_conformance}"
    );
    assert!(
        toolchain_conformance.contains("ail conformance: ok"),
        "{toolchain_conformance}"
    );
    let toolchain_conformance_fingerprint =
        fs::read_to_string(artifact_dir.join("toolchain-agent-conformance-report.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        toolchain_conformance_fingerprint.trim(),
        fnv64_fingerprint(&toolchain_conformance)
    );
    let pass_conformance =
        fs::read_to_string(artifact_dir.join("compiler-pass-conformance-report.txt")).unwrap();
    assert!(
        pass_conformance.contains("ail conformance: package ail-meta-permissions"),
        "{pass_conformance}"
    );
    assert!(pass_conformance.contains("ail conformance: ok"));
    let pass_conformance_fingerprint =
        fs::read_to_string(artifact_dir.join("compiler-pass-conformance-report.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        pass_conformance_fingerprint.trim(),
        fnv64_fingerprint(&pass_conformance)
    );

    let toolchain_verifier =
        fs::read(artifact_dir.join("toolchain-agent-VerifyBootstrapManifest.elf")).unwrap();
    assert_eq!(&toolchain_verifier[0..4], b"\x7fELF");
    let expected_toolchain_verifier_fingerprint = fnv64_fingerprint_bytes(&toolchain_verifier);
    let compiler_pass_native =
        fs::read(artifact_dir.join("compiler-pass-InferReadPermissions.elf")).unwrap();
    assert_eq!(&compiler_pass_native[0..4], b"\x7fELF");
    let expected_compiler_pass_fingerprint = fnv64_fingerprint_bytes(&compiler_pass_native);
    let agent_verifier = fs::read(artifact_dir.join("agent-VerifyBootstrapManifest.elf")).unwrap();
    assert_eq!(&agent_verifier[0..4], b"\x7fELF");
    let expected_agent_verifier_fingerprint = fnv64_fingerprint_bytes(&agent_verifier);
    let native_bytecode_report =
        fs::read_to_string(artifact_dir.join("bootstrap-native-bytecode-report.txt")).unwrap();
    assert!(
        native_bytecode_report.contains("AIL-Bootstrap-Native-Bytecode:"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("target linux-x86_64-elf"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("bytecode-level machine"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("bytecode-container linux-elf-executable"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("bytecode-format elf64-little-x86_64-executable"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains(&format!(
            "machine-bytecode toolchain-agent-target linux-x86_64-elf toolchain-agent-VerifyBootstrapManifest.elf elf64-little-x86_64-executable {} bytes {}",
            expected_toolchain_verifier_fingerprint,
            toolchain_verifier.len()
        )),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains(&format!(
            "machine-bytecode compiler-pass-target linux-x86_64-elf compiler-pass-InferReadPermissions.elf elf64-little-x86_64-executable {} bytes {}",
            expected_compiler_pass_fingerprint,
            compiler_pass_native.len()
        )),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains(&format!(
            "machine-bytecode agent-target linux-x86_64-elf agent-VerifyBootstrapManifest.elf elf64-little-x86_64-executable {} bytes {}",
            expected_agent_verifier_fingerprint,
            agent_verifier.len()
        )),
        "{native_bytecode_report}"
    );
    let native_bytecode_report_fingerprint =
        fs::read_to_string(artifact_dir.join("bootstrap-native-bytecode-report.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        native_bytecode_report_fingerprint.trim(),
        fnv64_fingerprint(&native_bytecode_report)
    );
    let host_boundary_report =
        fs::read_to_string(artifact_dir.join("bootstrap-host-boundary-report.txt")).unwrap();
    assert!(
        host_boundary_report.contains("AIL-Bootstrap-Host-Boundary:"),
        "{host_boundary_report}"
    );
    assert!(
        host_boundary_report.contains("no-host-backend-source true"),
        "{host_boundary_report}"
    );
    assert!(
        host_boundary_report.contains("generated-host-language-source none"),
        "{host_boundary_report}"
    );
    assert!(
        host_boundary_report.contains("forbidden-host-source-suffixes .rs .c .cc .cpp .h .hpp .py .js .ts .go .java .ll .bc .wasm"),
        "{host_boundary_report}"
    );
    assert!(
        host_boundary_report.contains("ail-source toolchain-agent.source.ail-spec.md"),
        "{host_boundary_report}"
    );
    assert!(
        host_boundary_report.contains("ail-bytecode toolchain-agent.ailbc.json"),
        "{host_boundary_report}"
    );
    assert!(
        host_boundary_report.contains("machine-bytecode toolchain-agent-VerifyBootstrapManifest.elf elf64-little-x86_64-executable"),
        "{host_boundary_report}"
    );
    let host_boundary_report_fingerprint =
        fs::read_to_string(artifact_dir.join("bootstrap-host-boundary-report.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        host_boundary_report_fingerprint.trim(),
        fnv64_fingerprint(&host_boundary_report)
    );
    let dependency_report =
        fs::read_to_string(artifact_dir.join("bootstrap-dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Bootstrap-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("host-language-runtime none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("dynamic-linker none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("shared-libraries none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("library-dependencies none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("linker-invocation none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("machine-bytecode-dependency toolchain-agent-VerifyBootstrapManifest.elf standalone-linux-syscall-elf"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains(
            "machine-bytecode-dependency compiler-pass-InferReadPermissions.elf standalone-linux-syscall-elf"
        ),
        "{dependency_report}"
    );
    let dependency_report_fingerprint =
        fs::read_to_string(artifact_dir.join("bootstrap-dependency-report.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        dependency_report_fingerprint.trim(),
        fnv64_fingerprint(&dependency_report)
    );
    let handoff_report =
        fs::read_to_string(artifact_dir.join("bootstrap-handoff-report.txt")).unwrap();
    assert!(
        handoff_report.contains("AIL-Bootstrap-Handoff-Report:"),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains("target linux-x86_64-elf"),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains("runtime-abi linux-syscall-argv-key-value"),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains("handoff-native-role toolchain-agent all-actions ok count 18"),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains("handoff-native-role compiler-pass all-actions ok count 1"),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains("handoff-native-role agent all-actions ok count 18"),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains(
            "handoff-native-action toolchain-agent-AcceptFlowReview.elf ok trace FlowReviewAccepted"
        ),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains(
            "handoff-native-action toolchain-agent-CompileApplication.elf ok trace ApplicationBytecodeCompiled"
        ),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains(
            "handoff-native-action toolchain-agent-CompileNativeTarget.elf ok trace NativeTargetCompiled"
        ),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains(
            "handoff-native-action compiler-pass-InferReadPermissions.elf ok trace ReadPermissionAdded"
        ),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains(
            "handoff-native-action toolchain-agent-VerifyConformanceManifest.elf ok trace ConformanceManifestVerified"
        ),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains(
            "handoff-native-action toolchain-agent-CompareAgentPromptPortability.elf ok trace AgentPromptPortabilityCompared"
        ),
        "{handoff_report}"
    );
    assert!(
        handoff_report.contains(
            "handoff-native-action agent-VerifyBootstrapManifest.elf ok trace BootstrapManifestVerified"
        ),
        "{handoff_report}"
    );
    let handoff_report_fingerprint =
        fs::read_to_string(artifact_dir.join("bootstrap-handoff-report.fingerprint.txt")).unwrap();
    assert_eq!(
        handoff_report_fingerprint.trim(),
        fnv64_fingerprint(&handoff_report)
    );

    let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    assert_eq!(agent_bytecode, toolchain_bytecode);
    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action VerifyBootstrapManifest started"));
    assert!(agent_trace.contains("read buildrequest.source package"));
    assert!(agent_trace.contains("read buildrequest.source package fingerprint"));
    assert!(agent_trace.contains("read buildrequest.core ir"));
    assert!(agent_trace.contains("read buildrequest.core ir fingerprint"));
    assert!(agent_trace.contains("read buildrequest.bytecode fingerprint"));
    assert!(agent_trace.contains("read buildrequest.compiler pass fingerprint"));
    assert!(agent_trace.contains("read buildrequest.compiler pass trace"));
    assert!(agent_trace.contains("read buildrequest.fixed point report"));
    assert!(agent_trace.contains("read buildrequest.fixed point report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.conformance report"));
    assert!(agent_trace.contains("read buildrequest.conformance report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.machine bytecode contract"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.host boundary report"));
    assert!(agent_trace.contains("read buildrequest.host boundary report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.dependency report"));
    assert!(agent_trace.contains("read buildrequest.dependency report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.handoff report"));
    assert!(agent_trace.contains("read buildrequest.handoff report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.target artifact fingerprint"));
    assert!(agent_trace.contains("read buildrequest.compiler pass target artifact fingerprint"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest fingerprint"));
    assert!(
        agent_trace.contains("write buildrequest.artifact manifest verification report=Verified")
    );
    assert!(agent_trace.contains("trace BootstrapManifestVerified"));

    let native_agent_run = Command::new(artifact_dir.join("agent-VerifyBootstrapManifest.elf"))
        .args([
            "buildrequest.id=ail-toolchain-agent-bootstrap",
            "buildrequest.status=BytecodeReady",
            "buildrequest.source package=ok",
            "buildrequest.source package fingerprint=fnv64:source",
            "buildrequest.core ir=ok",
            "buildrequest.core ir fingerprint=fnv64:core",
            "buildrequest.bytecode fingerprint=fnv64:toolchain",
            "buildrequest.compiler pass fingerprint=fnv64:pass",
            "buildrequest.compiler pass trace=ok",
            "buildrequest.fixed point report=ok",
            "buildrequest.fixed point report fingerprint=fnv64:fixed-point",
            "buildrequest.conformance report=ok",
            "buildrequest.conformance report fingerprint=fnv64:conformance",
            "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
            "buildrequest.native bytecode report=ok",
            "buildrequest.native bytecode report fingerprint=fnv64:native-bytecode",
            "buildrequest.host boundary report=ok",
            "buildrequest.host boundary report fingerprint=fnv64:host-boundary",
            "buildrequest.dependency report=ok",
            "buildrequest.dependency report fingerprint=fnv64:dependencies",
            "buildrequest.handoff report=ok",
            "buildrequest.handoff report fingerprint=fnv64:handoff",
            "buildrequest.target artifact fingerprint=fnv64:toolchain-native",
            "buildrequest.compiler pass target artifact fingerprint=fnv64:pass-native",
            "buildrequest.artifact manifest=ok",
            "buildrequest.artifact manifest fingerprint=fnv64:manifest",
        ])
        .output()
        .unwrap();
    assert!(
        native_agent_run.status.success(),
        "native bootstrap manifest verifier failed"
    );
    assert!(
        String::from_utf8_lossy(&native_agent_run.stderr)
            .contains("trace BootstrapManifestVerified"),
        "{}",
        String::from_utf8_lossy(&native_agent_run.stderr)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-bootstrap.txt")).unwrap();
    assert!(manifest.contains("AIL-Bootstrap-Manifest:"), "{manifest}");
    assert!(manifest.contains("target linux-x86_64-elf"), "{manifest}");
    assert!(
        manifest.contains("no-host-backend-source true"),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "toolchain-agent toolchain-agent.ailbc.json {}",
            fnv64_fingerprint(&toolchain_bytecode)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "toolchain-agent-source toolchain-agent.source.ail-package.md toolchain-agent.source.ail-spec.md {}",
            fnv64_fingerprint(&toolchain_source_bundle)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "toolchain-agent-core toolchain-agent.checked.ail-core.txt {}",
            fnv64_fingerprint(&toolchain_core)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "compiler-pass compiler-pass.ailbc.json {}",
            fnv64_fingerprint(&pass_bytecode)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "compiler-pass-source compiler-pass.source.ail-package.md compiler-pass.source.ail-spec.md {}",
            fnv64_fingerprint(&pass_source_bundle)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "compiler-pass-core compiler-pass.checked.ail-core.txt {}",
            fnv64_fingerprint(&pass_core)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "toolchain-agent-pass-output toolchain-agent.pass-output.ail-core.txt {}",
            fnv64_fingerprint(&toolchain_pass_output)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "toolchain-agent-pass-trace toolchain-agent.pass-trace.txt {}",
            fnv64_fingerprint(&toolchain_pass_trace)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bootstrap-fixed-point bootstrap-fixed-point-report.txt {}",
            fnv64_fingerprint(&fixed_point_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bootstrap-native-bytecode bootstrap-native-bytecode-report.txt {}",
            fnv64_fingerprint(&native_bytecode_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains("machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable"),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bootstrap-host-boundary bootstrap-host-boundary-report.txt {}",
            fnv64_fingerprint(&host_boundary_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bootstrap-dependencies bootstrap-dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bootstrap-handoff bootstrap-handoff-report.txt {}",
            fnv64_fingerprint(&handoff_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "toolchain-agent-conformance toolchain-agent-conformance-report.txt {}",
            fnv64_fingerprint(&toolchain_conformance)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "compiler-pass-conformance compiler-pass-conformance-report.txt {}",
            fnv64_fingerprint(&pass_conformance)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "toolchain-agent-target linux-x86_64-elf toolchain-agent-VerifyBootstrapManifest.elf {expected_toolchain_verifier_fingerprint}"
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "compiler-pass-target linux-x86_64-elf compiler-pass-InferReadPermissions.elf {expected_compiler_pass_fingerprint}"
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "agent agent.ailbc.json {}",
            fnv64_fingerprint(&agent_bytecode)
        )),
        "{manifest}"
    );
    assert!(manifest.contains("trace agent-trace.txt"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "agent-target linux-x86_64-elf agent-VerifyBootstrapManifest.elf {expected_agent_verifier_fingerprint}"
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_accepts_saved_spec_file_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let spec_path = std::env::temp_dir().join(format!(
        "ail-ail-compile-saved-spec-{}.ail-spec.md",
        std::process::id()
    ));
    let executable_path =
        std::env::temp_dir().join(format!("ail-ail-compile-saved-spec-{}", std::process::id()));
    let _ = fs::remove_file(&executable_path);
    fs::write(
        &spec_path,
        fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap(),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let run = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(run.status.success());
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "ticket.status=Closed\n"
    );

    fs::remove_file(spec_path).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_accepts_saved_core_file_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let document = parse_ail_package_document(&package).unwrap();
    let core = elaborate_ail_core(&package, &document);
    let core_path = std::env::temp_dir().join(format!(
        "ail-ail-compile-saved-core-{}.ail-core.txt",
        std::process::id()
    ));
    let executable_path =
        std::env::temp_dir().join(format!("ail-ail-compile-saved-core-{}", std::process::id()));
    let _ = fs::remove_file(&executable_path);
    fs::write(&core_path, render_ail_core(&core)).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-compile",
            fixture("support_ticket.ail").as_str(),
            "--core-file",
            core_path.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let run = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(run.status.success());
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "ticket.status=Closed\n"
    );

    fs::remove_file(core_path).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_native_system_component_emits_resource_trace() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("network_driver.ail");
    let executable_path =
        std::env::temp_dir().join(format!("ail-network-driver-native-{}", std::process::id()));
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "NetworkPacketReceiver",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let run = Command::new(&executable_path).output().unwrap();
    assert!(
        run.status.success(),
        "system component native executable failed"
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("system component Network packet receiver started"),
        "{stderr}"
    );
    assert!(
        stderr.contains("system resource rx buffer:Buffer"),
        "{stderr}"
    );
    assert!(stderr.contains("system owns rx buffer"), "{stderr}");
    assert!(
        stderr.contains("system places rx buffer in packet processing region"),
        "{stderr}"
    );
    assert!(
        stderr.contains("system effect read network device"),
        "{stderr}"
    );
    assert!(stderr.contains("trace PacketReceived"), "{stderr}");

    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_native_rejects_unlowered_observed_requirements() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let spec_path = std::env::temp_dir().join(format!(
        "ail-manual-approval-native-unlowered-{}.ail-spec.md",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-manual-approval-native-unlowered-{}",
        std::process::id()
    ));
    let spec_text = fs::read_to_string(format!("{package}/spec.ail-spec.md"))
        .unwrap()
        .replace(
            "the system requires the ticket to exist",
            "the system requires manual override approval",
        );
    fs::write(&spec_path, spec_text).unwrap();
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let status = output.status;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = fs::remove_file(&spec_path);
    let _ = fs::remove_file(&executable_path);
    assert!(
        !status.success(),
        "native compile should reject unlowered observed requirements\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains(
            "unsupported native linux-x86_64-elf observed rule 'manual override approval' in action 'CloseTicket'"
        ),
        "{stderr}"
    );
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_native_executable_enforces_overdue_time_requirement() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let executable_path = std::env::temp_dir().join(format!(
        "ail-overdue-ticket-native-time-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "MarksOverdueTickets",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let success = Command::new(&executable_path)
        .args([
            "current.time=2026-05-23T10:00:00Z",
            "ticket.due_at=2026-05-23T09:00:00Z",
            "ticket.status=Open",
        ])
        .output()
        .unwrap();
    assert!(success.status.success(), "overdue ticket should pass");
    assert_eq!(
        String::from_utf8_lossy(&success.stdout),
        "ticket.status=Overdue\n"
    );
    assert!(
        String::from_utf8_lossy(&success.stderr)
            .contains("rule passed: the current time to be later than due_at"),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );

    let not_due = Command::new(&executable_path)
        .args([
            "current.time=2026-05-23T08:00:00Z",
            "ticket.due_at=2026-05-23T09:00:00Z",
            "ticket.status=Open",
        ])
        .output()
        .unwrap();
    assert!(!not_due.status.success(), "not-overdue ticket should fail");
    assert_eq!(String::from_utf8_lossy(&not_due.stdout), "");
    assert!(
        String::from_utf8_lossy(&not_due.stderr).contains("failure RequirementFailed"),
        "{}",
        String::from_utf8_lossy(&not_due.stderr)
    );

    let missing_clock = Command::new(&executable_path)
        .args(["ticket.due_at=2026-05-23T09:00:00Z", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(
        !missing_clock.status.success(),
        "missing current.time should fail"
    );
    assert_eq!(String::from_utf8_lossy(&missing_clock.stdout), "");

    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_native_executable_enforces_create_ticket_inputs() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let executable_path = std::env::temp_dir().join(format!(
        "ail-create-ticket-native-inputs-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "CreateTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let success = Command::new(&executable_path)
        .args(["customer.id=C-1", "ticket.title=Printer"])
        .output()
        .unwrap();
    assert!(
        success.status.success(),
        "CreateTicket should accept customer id and title"
    );
    assert_eq!(
        String::from_utf8_lossy(&success.stdout),
        "ticket.status=New\nticket.customer.id=C-1\n"
    );
    assert!(
        String::from_utf8_lossy(&success.stderr).contains("rule passed: the customer id and title"),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );

    let missing_title = Command::new(&executable_path)
        .arg("customer.id=C-1")
        .output()
        .unwrap();
    assert!(
        !missing_title.status.success(),
        "missing ticket.title should fail"
    );
    assert_eq!(String::from_utf8_lossy(&missing_title.stdout), "");

    let missing_customer = Command::new(&executable_path)
        .arg("ticket.title=Printer")
        .output()
        .unwrap();
    assert!(
        !missing_customer.status.success(),
        "missing customer.id should fail"
    );
    assert_eq!(String::from_utf8_lossy(&missing_customer.stdout), "");

    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_native_executable_enforces_close_ticket_requirements() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let executable_path = std::env::temp_dir().join(format!(
        "ail-close-ticket-native-abi-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let success = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(
        success.status.success(),
        "open ticket should satisfy requirements"
    );

    let missing_ticket = Command::new(&executable_path)
        .arg("ticket.status=Open")
        .output()
        .unwrap();
    assert!(
        !missing_ticket.status.success(),
        "missing ticket.id should fail requirements"
    );

    let closed_ticket = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Closed"])
        .output()
        .unwrap();
    assert!(
        !closed_ticket.status.success(),
        "Closed ticket status should fail requirements"
    );

    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_native_executable_emits_close_ticket_state_write() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let executable_path = std::env::temp_dir().join(format!(
        "ail-close-ticket-native-write-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let success = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(
        success.status.success(),
        "native executable failed: {}",
        success.status
    );
    assert_eq!(
        String::from_utf8_lossy(&success.stdout),
        "ticket.status=Closed\n"
    );

    let failed = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Closed"])
        .output()
        .unwrap();
    assert!(!failed.status.success(), "closed ticket should fail");
    assert_eq!(String::from_utf8_lossy(&failed.stdout), "");

    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_native_executable_emits_trace_to_stderr() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let executable_path = std::env::temp_dir().join(format!(
        "ail-close-ticket-native-trace-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let success = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(
        success.status.success(),
        "native executable failed: {}",
        success.status
    );
    assert_eq!(
        String::from_utf8_lossy(&success.stdout),
        "ticket.status=Closed\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&success.stderr),
        concat!(
            "action CloseTicket started\n",
            "rule passed: the ticket to exist\n",
            "rule passed: the ticket status not to be Closed\n",
            "write ticket.status=Closed\n",
            "effect a public update\n",
            "guarantee checked: closed tickets do not appear in the open ticket queue\n",
            "trace TicketClosed\n"
        )
    );

    let failed = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Closed"])
        .output()
        .unwrap();
    assert!(!failed.status.success(), "closed ticket should fail");
    assert_eq!(String::from_utf8_lossy(&failed.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&failed.stderr),
        concat!(
            "action CloseTicket started\n",
            "rule passed: the ticket to exist\n",
            "failure RequirementFailed\n"
        )
    );

    let missing_ticket = Command::new(&executable_path)
        .arg("ticket.status=Open")
        .output()
        .unwrap();
    assert!(
        !missing_ticket.status.success(),
        "missing ticket should fail"
    );
    assert_eq!(String::from_utf8_lossy(&missing_ticket.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&missing_ticket.stderr),
        concat!(
            "action CloseTicket started\n",
            "failure NotFound\n",
            "trace TicketNotFound\n"
        )
    );

    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_build_native_executable_enforces_llm_style_is_field_requirement() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let spec_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-is-requirement-{}.ail-spec.md",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-support-ticket-is-requirement-artifacts-{}",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-is-requirement-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let _ = fs::remove_file(&executable_path);
    let spec_text = fs::read_to_string(format!("{package}/spec.ail-spec.md"))
        .unwrap()
        .replace(
            "the system requires the ticket status not to be Closed",
            "the system requires the ticket status is Open",
        );
    fs::write(&spec_path, spec_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert!(
        parse_ail_bytecode(&bytecode_artifact)
            .unwrap()
            .actions
            .get("CloseTicket")
            .unwrap()
            .instructions
            .iter()
            .any(|instruction| {
                instruction.opcode == "REQUIRE_FIELD_IN"
                    && instruction
                        .operands
                        .get("rule")
                        .is_some_and(|rule| rule == "the ticket status is Open")
            }),
        "{bytecode_artifact}"
    );

    let success = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(success.status.success(), "Open ticket should pass");
    assert_eq!(
        String::from_utf8_lossy(&success.stdout),
        "ticket.status=Closed\n"
    );

    let failed = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Closed"])
        .output()
        .unwrap();
    assert!(!failed.status.success(), "Closed ticket should fail");
    assert_eq!(String::from_utf8_lossy(&failed.stdout), "");

    fs::remove_file(spec_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_build_native_executable_enforces_llm_style_is_not_field_requirement() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let spec_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-is-not-requirement-{}.ail-spec.md",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-support-ticket-is-not-requirement-artifacts-{}",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-is-not-requirement-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let _ = fs::remove_file(&executable_path);
    let spec_text = fs::read_to_string(format!("{package}/spec.ail-spec.md"))
        .unwrap()
        .replace(
            "- the system requires the ticket to exist\n- the system requires the ticket status not to be Closed",
            "- the system requires the ticket exists and its status is not Closed",
        );
    fs::write(&spec_path, spec_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert!(
        parse_ail_bytecode(&bytecode_artifact)
            .unwrap()
            .actions
            .get("CloseTicket")
            .unwrap()
            .instructions
            .iter()
            .any(|instruction| {
                instruction.opcode == "REQUIRE_FIELD_NOT_EQUALS"
                    && instruction.operands.get("rule").is_some_and(|rule| {
                        rule == "the ticket exists and its status is not Closed"
                    })
                    && instruction
                        .operands
                        .get("value")
                        .is_some_and(|value| value == "Closed")
            }),
        "{bytecode_artifact}"
    );
    assert!(
        parse_ail_bytecode(&bytecode_artifact)
            .unwrap()
            .actions
            .get("CloseTicket")
            .unwrap()
            .instructions
            .iter()
            .any(|instruction| {
                instruction.opcode == "REQUIRE_EXISTS"
                    && instruction
                        .operands
                        .get("key")
                        .is_some_and(|key| key == "ticket.id")
                    && instruction
                        .operands
                        .get("failure")
                        .is_some_and(|failure| failure == "NotFound")
            }),
        "{bytecode_artifact}"
    );

    let success = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(success.status.success(), "Open ticket should pass");
    assert_eq!(
        String::from_utf8_lossy(&success.stdout),
        "ticket.status=Closed\n"
    );

    let failed = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Closed"])
        .output()
        .unwrap();
    assert!(!failed.status.success(), "Closed ticket should fail");
    assert_eq!(String::from_utf8_lossy(&failed.stdout), "");

    let missing_ticket = Command::new(&executable_path)
        .arg("ticket.status=Open")
        .output()
        .unwrap();
    assert!(
        !missing_ticket.status.success(),
        "missing ticket should fail"
    );
    assert_eq!(
        String::from_utf8_lossy(&missing_ticket.stderr),
        concat!(
            "action CloseTicket started\n",
            "failure NotFound\n",
            "trace TicketNotFound\n"
        )
    );

    fs::remove_file(spec_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_build_native_executable_enforces_llm_style_has_role_requirement() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let spec_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-has-role-requirement-{}.ail-spec.md",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-support-ticket-has-role-requirement-artifacts-{}",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-has-role-requirement-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let _ = fs::remove_file(&executable_path);
    let spec_text = fs::read_to_string(format!("{package}/spec.ail-spec.md"))
        .unwrap()
        .replace(
            "- the system requires the ticket status not to be Closed",
            concat!(
                "- the system requires the ticket status not to be Closed\n",
                "- the system requires the actor has role SupportAgent"
            ),
        );
    fs::write(&spec_path, spec_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert!(
        parse_ail_bytecode(&bytecode_artifact)
            .unwrap()
            .actions
            .get("CloseTicket")
            .unwrap()
            .instructions
            .iter()
            .any(|instruction| {
                instruction.opcode == "REQUIRE_FIELD_IN"
                    && instruction
                        .operands
                        .get("key")
                        .is_some_and(|key| key == "actor.role")
                    && instruction
                        .operands
                        .get("values")
                        .is_some_and(|values| values == "SupportAgent")
            }),
        "{bytecode_artifact}"
    );

    let success = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=Open",
            "actor.role=SupportAgent",
        ])
        .output()
        .unwrap();
    assert!(success.status.success(), "SupportAgent actor should pass");
    assert_eq!(
        String::from_utf8_lossy(&success.stdout),
        "ticket.status=Closed\n"
    );

    let failed = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open", "actor.role=Customer"])
        .output()
        .unwrap();
    assert!(!failed.status.success(), "Customer actor should fail");
    assert_eq!(String::from_utf8_lossy(&failed.stdout), "");

    fs::remove_file(spec_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_build_native_executable_enforces_llm_style_trailing_role_requirement() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let spec_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-trailing-role-requirement-{}.ail-spec.md",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-support-ticket-trailing-role-requirement-artifacts-{}",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-trailing-role-requirement-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let _ = fs::remove_file(&executable_path);
    let spec_text = fs::read_to_string(format!("{package}/spec.ail-spec.md"))
        .unwrap()
        .replace(
            "- the system requires the ticket status not to be Closed",
            concat!(
                "- the system requires the ticket status not to be Closed\n",
                "- the system requires the caller has Admin or SupportAgent role"
            ),
        );
    fs::write(&spec_path, spec_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert!(
        parse_ail_bytecode(&bytecode_artifact)
            .unwrap()
            .actions
            .get("CloseTicket")
            .unwrap()
            .instructions
            .iter()
            .any(|instruction| {
                instruction.opcode == "REQUIRE_FIELD_IN"
                    && instruction
                        .operands
                        .get("key")
                        .is_some_and(|key| key == "caller.role")
                    && instruction
                        .operands
                        .get("values")
                        .is_some_and(|values| values == "Admin\u{1f}SupportAgent")
            }),
        "{bytecode_artifact}"
    );

    let admin = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open", "caller.role=Admin"])
        .output()
        .unwrap();
    assert!(admin.status.success(), "Admin caller should pass");
    assert_eq!(
        String::from_utf8_lossy(&admin.stdout),
        "ticket.status=Closed\n"
    );

    let support_agent = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=Open",
            "caller.role=SupportAgent",
        ])
        .output()
        .unwrap();
    assert!(
        support_agent.status.success(),
        "SupportAgent caller should pass"
    );
    assert_eq!(
        String::from_utf8_lossy(&support_agent.stdout),
        "ticket.status=Closed\n"
    );

    let failed = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=Open",
            "caller.role=Customer",
        ])
        .output()
        .unwrap();
    assert!(!failed.status.success(), "Customer caller should fail");
    assert_eq!(String::from_utf8_lossy(&failed.stdout), "");

    fs::remove_file(spec_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_build_native_executable_enforces_llm_style_has_permission_requirement() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let spec_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-has-permission-requirement-{}.ail-spec.md",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-support-ticket-has-permission-requirement-artifacts-{}",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-has-permission-requirement-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let _ = fs::remove_file(&executable_path);
    let spec_text = fs::read_to_string(format!("{package}/spec.ail-spec.md"))
        .unwrap()
        .replace(
            "- the system requires the ticket status not to be Closed",
            concat!(
                "- the system requires the ticket status not to be Closed\n",
                "- the system requires the requesting user has permission to modify ticket status"
            ),
        );
    fs::write(&spec_path, spec_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert!(
        parse_ail_bytecode(&bytecode_artifact)
            .unwrap()
            .actions
            .get("CloseTicket")
            .unwrap()
            .instructions
            .iter()
            .any(|instruction| {
                instruction.opcode == "REQUIRE_FIELD_IN"
                    && instruction
                        .operands
                        .get("key")
                        .is_some_and(|key| key == "requesting user.permission")
                    && instruction
                        .operands
                        .get("values")
                        .is_some_and(|values| values == "modify ticket status")
            }),
        "{bytecode_artifact}"
    );

    let success = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=Open",
            "requesting user.permission=modify ticket status",
        ])
        .output()
        .unwrap();
    assert!(success.status.success(), "permission should pass");
    assert_eq!(
        String::from_utf8_lossy(&success.stdout),
        "ticket.status=Closed\n"
    );

    let failed = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=Open",
            "requesting user.permission=view ticket",
        ])
        .output()
        .unwrap();
    assert!(!failed.status.success(), "wrong permission should fail");
    assert_eq!(String::from_utf8_lossy(&failed.stdout), "");

    fs::remove_file(spec_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_native_executable_enforces_field_in_requirements() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let executable_path = std::env::temp_dir().join(format!(
        "ail-assign-ticket-native-field-in-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "AssignTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let support_agent = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=Open",
            "ticket.assignee.role=SupportAgent",
        ])
        .output()
        .unwrap();
    assert!(support_agent.status.success(), "SupportAgent should pass");
    assert_eq!(
        String::from_utf8_lossy(&support_agent.stdout),
        "ticket.status=Assigned\n"
    );

    let support_manager = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=New",
            "ticket.assignee.role=SupportManager",
        ])
        .output()
        .unwrap();
    assert!(
        support_manager.status.success(),
        "SupportManager should pass"
    );
    assert_eq!(
        String::from_utf8_lossy(&support_manager.stdout),
        "ticket.status=Assigned\n"
    );

    let missing_role = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.status=Open"])
        .output()
        .unwrap();
    assert!(
        !missing_role.status.success(),
        "missing ticket.assignee.role should fail"
    );
    assert_eq!(String::from_utf8_lossy(&missing_role.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&missing_role.stderr),
        concat!(
            "action AssignTicket started\n",
            "rule passed: the ticket to exist\n",
            "rule passed: the ticket status to be New or Open\n",
            "failure PermissionDenied\n",
            "trace InternalNotesDenied\n"
        )
    );

    let customer = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=Open",
            "ticket.assignee.role=Customer",
        ])
        .output()
        .unwrap();
    assert!(
        !customer.status.success(),
        "Customer assignee role should fail"
    );
    assert_eq!(String::from_utf8_lossy(&customer.stdout), "");

    let missing_status = Command::new(&executable_path)
        .args(["ticket.id=T-1", "ticket.assignee.role=SupportAgent"])
        .output()
        .unwrap();
    assert!(
        !missing_status.status.success(),
        "missing ticket.status should fail"
    );
    assert_eq!(String::from_utf8_lossy(&missing_status.stdout), "");

    let closed_ticket = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=Closed",
            "ticket.assignee.role=SupportAgent",
        ])
        .output()
        .unwrap();
    assert!(
        !closed_ticket.status.success(),
        "Closed ticket status should fail allow-list requirement"
    );
    assert_eq!(String::from_utf8_lossy(&closed_ticket.stdout), "");

    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_native_executable_emits_nested_object_field_write() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let executable_path = std::env::temp_dir().join(format!(
        "ail-assign-ticket-native-object-write-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "AssignTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let assigned = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=Open",
            "ticket.assignee.id=A-1",
            "ticket.assignee.role=SupportAgent",
        ])
        .output()
        .unwrap();
    assert!(
        assigned.status.success(),
        "assignee object write should pass"
    );
    assert_eq!(
        String::from_utf8_lossy(&assigned.stdout),
        "ticket.assignee.id=A-1\nticket.status=Assigned\n"
    );
    assert!(
        String::from_utf8_lossy(&assigned.stderr).contains("write ticket.assignee"),
        "{}",
        String::from_utf8_lossy(&assigned.stderr)
    );

    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_native_agent_tool_emits_audit_trace() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("refund_tool.ail");
    let executable_path =
        std::env::temp_dir().join(format!("ail-refund-tool-native-{}", std::process::id()));
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "RefundCustomerPayment",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let run = Command::new(&executable_path)
        .args([
            "order id=O-1",
            "refund amount=USD:25.00",
            "reason=duplicate",
            "payment token=tok_secret",
        ])
        .output()
        .unwrap();
    assert!(run.status.success(), "refund tool native executable failed");
    assert_eq!(String::from_utf8_lossy(&run.stdout), "");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("tool Refund customer payment started"),
        "{stderr}"
    );
    assert!(
        stderr.contains("tool call PaymentProvider.refund"),
        "{stderr}"
    );
    assert!(
        stderr.contains("tool secret protection payment token"),
        "{stderr}"
    );
    assert!(
        stderr.contains("trace RefundCustomerPaymentRequested"),
        "{stderr}"
    );
    assert!(!stderr.contains("tok_secret"), "{stderr}");

    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_compile_native_compiler_pass_emits_transform_trace() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("compiler_pass.ail");
    let executable_path =
        std::env::temp_dir().join(format!("ail-compiler-pass-native-{}", std::process::id()));
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-compile",
            &package,
            "--action",
            "InferReadPermissions",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let run = Command::new(&executable_path)
        .arg("input graph=checked")
        .arg("package policy=default")
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "compiler pass native executable failed"
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("compiler pass Infer read permissions started"),
        "{stderr}"
    );
    assert!(
        stderr.contains("pass read every edge whose kind is reads"),
        "{stderr}"
    );
    assert!(
        stderr.contains("core transform infer read permissions"),
        "{stderr}"
    );
    assert!(stderr.contains("trace ReadPermissionAdded"), "{stderr}");

    fs::remove_file(executable_path).unwrap();
}

#[test]
fn cli_ail_run_executes_close_ticket_with_trace() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");

    let success = Command::new(binary)
        .args([
            "ail-run",
            &package,
            "--action",
            "CloseTicket",
            "ticket.id=T-1",
            "ticket.status=Open",
        ])
        .output()
        .unwrap();
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let success_stdout = String::from_utf8_lossy(&success.stdout);
    assert!(success_stdout.contains("ail-run succeeded"));
    assert!(success_stdout.contains("ticket.status=Closed"));
    assert!(
        success_stdout
            .contains("trace=action CloseTicket started -> rule passed: the ticket to exist")
    );

    let missing = Command::new(binary)
        .args([
            "ail-run",
            &package,
            "--action",
            "CloseTicket",
            "ticket.status=Open",
        ])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    let missing_stdout = String::from_utf8_lossy(&missing.stdout);
    assert!(missing_stdout.contains("ail-run failed"));
    assert!(missing_stdout.contains("failure=NotFound"));
    assert!(missing_stdout.contains("trace=action CloseTicket started -> failure NotFound"));
}

#[test]
fn cli_ail_run_accepts_saved_core_file_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let core_output = Command::new(binary)
        .args(["ail-core", &package])
        .output()
        .unwrap();
    assert!(
        core_output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&core_output.stdout),
        String::from_utf8_lossy(&core_output.stderr)
    );
    let core_path = std::env::temp_dir().join(format!(
        "ail-run-saved-core-{}.ail-core.txt",
        std::process::id()
    ));
    fs::write(&core_path, core_output.stdout).unwrap();

    let success = Command::new(binary)
        .args([
            "ail-run",
            "--core-file",
            core_path.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "ticket.id=T-1",
            "ticket.status=Open",
            "ticket.internal notes=sensitive note",
        ])
        .output()
        .unwrap();
    assert!(
        success.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&success.stdout),
        String::from_utf8_lossy(&success.stderr)
    );
    let success_stdout = String::from_utf8_lossy(&success.stdout);
    assert!(success_stdout.contains("ail-run succeeded"));
    assert!(success_stdout.contains("ticket.status=Closed"));
    assert!(success_stdout.contains("ticket.internal notes=<secret>"));
    assert!(!success_stdout.contains("sensitive note"));
    assert!(
        success_stdout
            .contains("trace=action CloseTicket started -> rule passed: the ticket to exist")
    );

    let missing = Command::new(binary)
        .args([
            "ail-run",
            "--core-file",
            core_path.to_str().unwrap(),
            "--action",
            "CloseTicket",
            "ticket.status=Open",
        ])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    let missing_stdout = String::from_utf8_lossy(&missing.stdout);
    assert!(missing_stdout.contains("ail-run failed"));
    assert!(missing_stdout.contains("failure=NotFound"));
    assert!(missing_stdout.contains("trace=action CloseTicket started -> failure NotFound"));

    fs::remove_file(core_path).unwrap();
}

#[test]
fn cli_ail_run_redacts_secret_runtime_state() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");

    let success = Command::new(binary)
        .args([
            "ail-run",
            &package,
            "--action",
            "CloseTicket",
            "ticket.id=T-1",
            "ticket.status=Open",
            "ticket.internal notes=sensitive note",
        ])
        .output()
        .unwrap();
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let success_stdout = String::from_utf8_lossy(&success.stdout);
    assert!(success_stdout.contains("ticket.internal notes=<secret>"));
    assert!(!success_stdout.contains("sensitive note"));

    let tool_package = fixture("refund_tool.ail");
    let tool_success = Command::new(binary)
        .args([
            "ail-run",
            &tool_package,
            "--action",
            "RefundCustomerPayment",
            "order id=O-1",
            "payment token=tok_123",
            "refund amount=USD:25.00",
            "reason=duplicate",
        ])
        .output()
        .unwrap();
    assert!(
        tool_success.status.success(),
        "{}",
        String::from_utf8_lossy(&tool_success.stderr)
    );
    let tool_stdout = String::from_utf8_lossy(&tool_success.stdout);
    assert!(tool_stdout.contains("payment token=<secret>"));
    assert!(!tool_stdout.contains("tok_123"));
}

#[test]
fn ail_core_reports_stable_invalid_fixture_diagnostics() {
    let package = load_ail_package_dir(fixture("support_ticket.ail")).unwrap();
    let rejected_dir = format!("{}/examples/rejected", fixture("support_ticket.ail"));

    let missing_reference =
        fs::read_to_string(format!("{rejected_dir}/missing-reference.ail-spec.md")).unwrap();
    let missing_reference_doc = parse_ail_spec_text(&missing_reference).unwrap();
    let missing_reference_core = elaborate_ail_core(&package, &missing_reference_doc);
    assert!(check_ail_core(&missing_reference_core).contains(
        &"AIL001 unknown requirement reference 'account' in action CloseTicket".to_string()
    ));

    let secret_leak =
        fs::read_to_string(format!("{rejected_dir}/secret-leak.ail-spec.md")).unwrap();
    let secret_leak_doc = parse_ail_spec_text(&secret_leak).unwrap();
    let secret_leak_core = elaborate_ail_core(&package, &secret_leak_doc);
    assert!(
        check_ail_core(&secret_leak_core)
            .contains(&"AIL002 secret field Ticket.internal notes is written without an explicit protection rule".to_string())
    );

    let missing_failure = fs::read_to_string(format!(
        "{rejected_dir}/missing-failure-handler.ail-spec.md"
    ))
    .unwrap();
    let missing_failure_doc = parse_ail_spec_text(&missing_failure).unwrap();
    let missing_failure_core = elaborate_ail_core(&package, &missing_failure_doc);
    let missing_failure_diagnostics = check_ail_core(&missing_failure_core);
    assert!(
        missing_failure_diagnostics
            .contains(&"AIL003 action CloseTicket names failure 'payment provider rejects the update' without a declared Failure section".to_string())
    );
    assert!(
        !missing_failure_diagnostics.contains(
            &"AIL-FAILURE-001 failure payment provider rejects the update is missing declared handling"
                .to_string()
        )
    );

    let failure_without_handling = fs::read_to_string(format!(
        "{rejected_dir}/failure-without-handling.ail-spec.md"
    ))
    .unwrap();
    let failure_without_handling_doc = parse_ail_spec_text(&failure_without_handling).unwrap();
    let failure_without_handling_core = elaborate_ail_core(&package, &failure_without_handling_doc);
    let failure_without_handling_diagnostics = check_ail_core(&failure_without_handling_core);
    assert!(
        failure_without_handling_diagnostics
            .contains(&"AIL-FAILURE-001 failure NotFound is missing declared handling".to_string())
    );
    assert!(
        !failure_without_handling_diagnostics.contains(
            &"AIL003 action CloseTicket names failure 'NotFound' without a declared Failure section"
                .to_string()
        )
    );

    let failure_without_trace =
        fs::read_to_string(format!("{rejected_dir}/failure-without-trace.ail-spec.md")).unwrap();
    let failure_without_trace_doc = parse_ail_spec_text(&failure_without_trace).unwrap();
    let failure_without_trace_core = elaborate_ail_core(&package, &failure_without_trace_doc);
    assert!(
        check_ail_core(&failure_without_trace_core)
            .contains(&"AIL-TRACE-002 failure NotFound is missing trace coverage".to_string())
    );

    let action_without_trace =
        fs::read_to_string(format!("{rejected_dir}/action-without-trace.ail-spec.md")).unwrap();
    let action_without_trace_doc = parse_ail_spec_text(&action_without_trace).unwrap();
    let action_without_trace_core = elaborate_ail_core(&package, &action_without_trace_doc);
    assert!(
        check_ail_core(&action_without_trace_core)
            .contains(&"AIL-TRACE-001 action CloseTicket is missing trace coverage".to_string())
    );

    let unknown_field =
        fs::read_to_string(format!("{rejected_dir}/unknown-field.ail-spec.md")).unwrap();
    let unknown_field_doc = parse_ail_spec_text(&unknown_field).unwrap();
    let unknown_field_core = elaborate_ail_core(&package, &unknown_field_doc);
    let unknown_field_diagnostics = check_ail_core(&unknown_field_core);
    assert!(
        unknown_field_diagnostics.contains(
            &"AIL004 action ArchiveTicket reads unknown field reference 'ticket owner email'"
                .to_string()
        )
    );
    assert!(unknown_field_diagnostics.contains(
        &"AIL004 action ArchiveTicket writes unknown field reference 'ticket archive code to Archived'"
            .to_string()
    ));

    let secret_read = fs::read_to_string(format!(
        "{rejected_dir}/secret-read-without-protection.ail-spec.md"
    ))
    .unwrap();
    let secret_read_doc = parse_ail_spec_text(&secret_read).unwrap();
    let secret_read_core = elaborate_ail_core(&package, &secret_read_doc);
    assert!(
        check_ail_core(&secret_read_core)
            .contains(&"AIL005 secret field Ticket.internal notes is read without an explicit protection rule".to_string())
    );

    let unknown_type =
        fs::read_to_string(format!("{rejected_dir}/unknown-field-type.ail-spec.md")).unwrap();
    let unknown_type_doc = parse_ail_spec_text(&unknown_type).unwrap();
    let unknown_type_core = elaborate_ail_core(&package, &unknown_type_doc);
    assert!(
        check_ail_core(&unknown_type_core).contains(
            &"AIL-TYPE-001 field Ticket.metadata has unknown type 'MysteryBox'".to_string()
        )
    );

    let unknown_requirement_field = fs::read_to_string(format!(
        "{rejected_dir}/unknown-requirement-field.ail-spec.md"
    ))
    .unwrap();
    let unknown_requirement_field_doc = parse_ail_spec_text(&unknown_requirement_field).unwrap();
    let unknown_requirement_field_core =
        elaborate_ail_core(&package, &unknown_requirement_field_doc);
    assert!(
        check_ail_core(&unknown_requirement_field_core).contains(
            &"AIL007 action CloseTicket requirement references unknown field 'ticket priority'"
                .to_string()
        )
    );
}

#[test]
fn ail_core_reports_unknown_profile_value_types() {
    let tool_package = load_ail_package_dir(fixture("refund_tool.ail")).unwrap();
    let tool_spec = fs::read_to_string(format!("{}/spec.ail-spec.md", fixture("refund_tool.ail")))
        .unwrap()
        .replace(
            "payment token: Secret<Text>",
            "payment token: Secret<MysteryCredential>",
        )
        .replace("refund id: Text", "refund id: MysteryReceipt");
    let tool_doc = parse_ail_spec_text(&tool_spec).unwrap();
    let tool_core = elaborate_ail_core(&tool_package, &tool_doc);
    let tool_diagnostics = check_ail_core(&tool_core);
    assert!(tool_diagnostics.contains(
        &"AIL-TYPE-001 input RefundCustomerPayment.payment token has unknown type 'Secret<MysteryCredential>'"
            .to_string()
    ));
    assert!(
        tool_diagnostics.contains(
            &"AIL-TYPE-001 output RefundCustomerPayment.refund id has unknown type 'MysteryReceipt'"
                .to_string()
        )
    );
    let detailed_tool = detailed_ail_diagnostic(
        &tool_core,
        "AIL-TYPE-001",
        "input RefundCustomerPayment.payment token has unknown type 'Secret<MysteryCredential>'",
    );
    assert!(
        detailed_tool.contains("source=tool:RefundCustomerPayment.input:payment token"),
        "{detailed_tool}"
    );
    assert!(
        detailed_tool.contains(
            "repair=Use a supported AIL type for input RefundCustomerPayment.payment token or declare a Thing named 'MysteryCredential'."
        ),
        "{detailed_tool}"
    );

    let compiler_package = load_ail_package_dir(fixture("compiler_pass.ail")).unwrap();
    let compiler_spec =
        fs::read_to_string(format!("{}/spec.ail-spec.md", fixture("compiler_pass.ail")))
            .unwrap()
            .replace("input graph: AIL-Core graph", "input graph: MysteryGraph")
            .replace(
                "diagnostics: List<Diagnostic>",
                "diagnostics: List<MysteryDiagnostic>",
            );
    let compiler_doc = parse_ail_spec_text(&compiler_spec).unwrap();
    let compiler_core = elaborate_ail_core(&compiler_package, &compiler_doc);
    let compiler_diagnostics = check_ail_core(&compiler_core);
    assert!(
        compiler_diagnostics.contains(
            &"AIL-TYPE-001 value InferReadPermissions.input graph has unknown type 'MysteryGraph'"
                .to_string()
        )
    );
    assert!(compiler_diagnostics.contains(
        &"AIL-TYPE-001 value InferReadPermissions.diagnostics has unknown type 'List<MysteryDiagnostic>'"
            .to_string()
    ));
    let detailed_value = detailed_ail_diagnostic(
        &compiler_core,
        "AIL-TYPE-001",
        "value InferReadPermissions.diagnostics has unknown type 'List<MysteryDiagnostic>'",
    );
    assert!(
        detailed_value.contains("source=compiler_pass:InferReadPermissions.output:diagnostics"),
        "{detailed_value}"
    );
    assert!(
        detailed_value.contains(
            "repair=Use a supported AIL type for value InferReadPermissions.diagnostics or declare a Thing named 'MysteryDiagnostic'."
        ),
        "{detailed_value}"
    );
}

#[test]
fn ail_core_reports_agent_tool_missing_trace_coverage() {
    let package = load_ail_package_dir(fixture("refund_tool.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/tool-without-trace.ail-spec.md",
        fixture("refund_tool.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL-TRACE-001"
                && diagnostic.message
                    == "tool RefundCustomerPayment is missing audit trace coverage"
        })
        .unwrap_or_else(|| panic!("missing AIL-TRACE-001 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("tool:RefundCustomerPayment")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("node:Tool:refundcustomerpayment:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some("Add a 'The tool records:' section to tool RefundCustomerPayment.")
    );
}

#[test]
fn ail_core_reports_agent_tool_approval_mentions_without_approval_rules() {
    let package = load_ail_package_dir(fixture("refund_tool.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/approval-without-rule.ail-spec.md",
        fixture("refund_tool.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL018"
                && diagnostic.message
                    == "tool RefundCustomerPayment mentions approval but has no explicit approval rule"
        })
        .unwrap_or_else(|| panic!("missing AIL018 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("tool:RefundCustomerPayment")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("node:Tool:refundcustomerpayment:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some("Add a 'The tool requires approval:' section to tool RefundCustomerPayment.")
    );
}

#[test]
fn ail_core_reports_agent_tool_permission_mentions_without_permission_rules() {
    let package = load_ail_package_dir(fixture("refund_tool.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/permission-without-rule.ail-spec.md",
        fixture("refund_tool.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL019"
                && diagnostic.message
                    == "tool RefundCustomerPayment mentions permission but has no explicit permission rule"
        })
        .unwrap_or_else(|| panic!("missing AIL019 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("tool:RefundCustomerPayment")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("node:Tool:refundcustomerpayment:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some("Add a 'The tool requires permission:' section to tool RefundCustomerPayment.")
    );
}

#[test]
fn ail_core_reports_agent_tool_secret_outputs_without_reveal_permission() {
    let package = load_ail_package_dir(fixture("refund_tool.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/secret-output.ail-spec.md",
        fixture("refund_tool.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL020"
                && diagnostic.message
                    == "output RefundCustomerPayment.payment token discloses secret type 'Secret<Text>' without reveal permission"
        })
        .unwrap_or_else(|| panic!("missing AIL020 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("tool:RefundCustomerPayment.output:payment token")
    );
    assert!(
        diagnostic.affected_graph_item.as_deref().is_some_and(
            |item| item.starts_with("node:Output:refundcustomerpayment-payment-token:")
        ),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Change output RefundCustomerPayment.payment token to a non-secret redacted type or add an explicit reveal permission."
        )
    );
}

#[test]
fn ail_core_reports_system_effects_without_capabilities() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/effect-without-capability.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL021"
                && diagnostic.message
                    == "system component NetworkPacketReceiver performs effect 'read network device' without a declared capability"
        })
        .unwrap_or_else(|| panic!("missing AIL021 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:NetworkPacketReceiver.effect:read network device")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("edge:edge:performs:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Add a 'The component requires capability:' section to system component NetworkPacketReceiver."
        )
    );
}

#[test]
fn ail_core_reports_system_effects_that_miss_declared_resources() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/unknown-effect-resource.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL022"
                && diagnostic.message
                    == "system component NetworkPacketReceiver effect 'read dma ring' targets unknown resource 'dma ring'"
        })
        .unwrap_or_else(|| panic!("missing AIL022 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:NetworkPacketReceiver.effect:read dma ring")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("node:Effect:read-dma-ring:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Declare resource 'dma ring' on system component NetworkPacketReceiver or update the effect to target a declared resource."
        )
    );
}

#[test]
fn ail_core_reports_system_device_effects_without_matching_capability() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/device-effect-without-matching-capability.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL023"
                && diagnostic.message
                    == "system component NetworkPacketReceiver effect 'read network device' targets device resource 'network device' without a matching capability"
        })
        .unwrap_or_else(|| panic!("missing AIL023 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:NetworkPacketReceiver.effect:read network device")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("edge:edge:targets-resource:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Add a capability such as 'access network device' to system component NetworkPacketReceiver."
        )
    );
}

#[test]
fn ail_core_reports_system_mutable_resource_effects_without_ownership() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/mutable-effect-without-ownership.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL024"
                && diagnostic.message
                    == "system component NetworkPacketReceiver mutates resource 'rx buffer' without ownership"
        })
        .unwrap_or_else(|| panic!("missing AIL024 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:NetworkPacketReceiver.effect:write rx buffer")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("edge:edge:targets-resource:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Add 'rx buffer' to 'The component owns:' for system component NetworkPacketReceiver."
        )
    );
}

#[test]
fn ail_core_reports_system_moves_without_ownership() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/move-without-ownership.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL024"
                && diagnostic.message
                    == "system component PacketHandoff moves resource 'rx buffer' without ownership"
        })
        .unwrap_or_else(|| panic!("missing move ownership diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:PacketHandoff.effect:move rx buffer")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("edge:edge:targets-resource:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Add 'rx buffer' to 'The component owns:' for system component PacketHandoff before moving it."
        )
    );
}

#[test]
fn ail_core_reports_system_layouts_for_unknown_resources() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/layout-unknown-resource.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL031"
                && diagnostic.message
                    == "system component PacketLayout declares layout for unknown resource 'dma ring'"
        })
        .unwrap_or_else(|| panic!("missing AIL031 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:PacketLayout.layout:dma ring")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("node:Layout:packetlayout-dma-ring:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Declare resource 'dma ring' in 'The component uses:' or update the layout bullet for system component PacketLayout."
        )
    );
}

#[test]
fn ail_core_reports_system_allocations_for_unknown_resources() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/allocation-unknown-resource.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL032"
                && diagnostic.message
                    == "system component PacketAllocator declares allocation for unknown resource 'dma ring'"
        })
        .unwrap_or_else(|| panic!("missing AIL032 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:PacketAllocator.allocation:dma ring")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("node:Allocation:packetallocator-dma-ring:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Declare resource 'dma ring' in 'The component uses:' or update the allocation bullet for system component PacketAllocator."
        )
    );
}

#[test]
fn ail_core_reports_interrupt_context_blocking_effects() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/interrupt-context-blocking-effect.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL033"
                && diagnostic.message
                    == "system component TimerInterruptHandler performs blocking effect 'wait for scheduler' in interrupt context"
        })
        .unwrap_or_else(|| panic!("missing AIL033 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:TimerInterruptHandler.effect:wait for scheduler")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("edge:edge:performs:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Move blocking effect 'wait for scheduler' out of interrupt context or remove the 'interrupt' context declaration for system component TimerInterruptHandler."
        )
    );
}

#[test]
fn ail_core_reports_interrupt_priorities_for_unknown_contexts() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/interrupt-priority-unknown-context.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL034"
                && diagnostic.message
                    == "system component TimerPriority configures priority for unknown context 'interrupt'"
        })
        .unwrap_or_else(|| panic!("missing AIL034 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:TimerPriority.priority:interrupt")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("node:InterruptPriority:timerpriority-interrupt:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Add 'interrupt' to 'The component runs in context:' or update the priority bullet for system component TimerPriority."
        )
    );
}

#[test]
fn ail_core_reports_interrupt_masks_for_unknown_contexts() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/interrupt-mask-unknown-context.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL040"
                && diagnostic.message
                    == "system component TimerMask configures interrupt mask for unknown context 'interrupt'"
        })
        .unwrap_or_else(|| panic!("missing AIL040 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:TimerMask.interrupt_mask:interrupt")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("node:InterruptMask:timermask-interrupt:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Add 'interrupt' to 'The component runs in context:' or update the interrupt mask bullet for system component TimerMask."
        )
    );
}

#[test]
fn ail_core_reports_scheduler_tasks_for_unknown_contexts() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/scheduler-task-unknown-context.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL035"
                && diagnostic.message
                    == "system component PacketScheduler schedules task 'packet poller' for unknown context 'process'"
        })
        .unwrap_or_else(|| panic!("missing AIL035 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:PacketScheduler.task:packet poller")
    );
    assert!(
        diagnostic.affected_graph_item.as_deref().is_some_and(
            |item| item.starts_with("node:SchedulerTask:packetscheduler-packet-poller:")
        ),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Add 'process' to 'The component runs in context:' or update the task bullet for system component PacketScheduler."
        )
    );
}

#[test]
fn ail_core_reports_scheduler_task_priorities_for_unknown_tasks() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/scheduler-task-priority-unknown-task.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL036"
                && diagnostic.message
                    == "system component PacketScheduler configures priority for unknown task 'packet poller'"
        })
        .unwrap_or_else(|| panic!("missing AIL036 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:PacketScheduler.task_priority:packet poller")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item
                .starts_with("node:SchedulerTaskPriority:packetscheduler-packet-poller:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Add 'packet poller' to 'The component schedules task:' or update the task priority bullet for system component PacketScheduler."
        )
    );
}

#[test]
fn ail_core_reports_scheduler_task_timings_for_unknown_tasks() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/scheduler-task-timing-unknown-task.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL037"
                && diagnostic.message
                    == "system component PacketScheduler configures timing for unknown task 'packet poller'"
        })
        .unwrap_or_else(|| panic!("missing AIL037 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:PacketScheduler.task_timing:packet poller")
    );
    assert!(
        diagnostic.affected_graph_item.as_deref().is_some_and(
            |item| item.starts_with("node:SchedulerTaskTiming:packetscheduler-packet-poller:")
        ),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Add 'packet poller' to 'The component schedules task:' or update the task timing bullet for system component PacketScheduler."
        )
    );
}

#[test]
fn ail_core_reports_lock_guards_for_unknown_locks() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/lock-guard-unknown-lock.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL039"
                && diagnostic.message
                    == "system component PacketScheduler guards resource 'scheduler state' with unknown lock resource 'scheduler lock'"
        })
        .unwrap_or_else(|| panic!("missing AIL039 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:PacketScheduler.lock_guard:scheduler state")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("node:LockGuard:packetscheduler-scheduler-state:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Declare lock resource 'scheduler lock' in 'The component uses:' or update the lock guard bullet for system component PacketScheduler."
        )
    );
}

#[test]
fn ail_core_reports_lock_guards_for_unknown_resources() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/lock-guard-unknown-resource.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL038"
                && diagnostic.message
                    == "system component PacketScheduler declares lock guard for unknown resource 'scheduler state'"
        })
        .unwrap_or_else(|| panic!("missing AIL038 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:PacketScheduler.lock_guard:scheduler state")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("node:LockGuard:packetscheduler-scheduler-state:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Declare resource 'scheduler state' in 'The component uses:' or update the lock guard bullet for system component PacketScheduler."
        )
    );
}

#[test]
fn ail_core_reports_system_read_effects_without_ownership_or_borrowing() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/read-effect-without-borrow.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL025"
                && diagnostic.message
                    == "system component NetworkPacketReceiver reads resource 'packet metadata' without ownership or borrowing"
        })
        .unwrap_or_else(|| panic!("missing AIL025 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:NetworkPacketReceiver.effect:read packet metadata")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("edge:edge:targets-resource:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Add 'packet metadata' to 'The component borrows:' or 'The component owns:' for system component NetworkPacketReceiver."
        )
    );
}

#[test]
fn ail_core_reports_system_resource_effects_without_region() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/resource-without-region.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL026"
                && diagnostic.message
                    == "system component NetworkPacketReceiver uses resource 'packet metadata' without a region"
        })
        .unwrap_or_else(|| panic!("missing AIL026 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:NetworkPacketReceiver.effect:read packet metadata")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("edge:edge:targets-resource:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Add 'packet metadata in <region>' to 'The component places:' for system component NetworkPacketReceiver."
        )
    );
}

#[test]
fn ail_core_reports_system_mutable_effects_against_borrowed_resources() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/mutate-borrowed-resource.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL027"
                && diagnostic.message
                    == "system component NetworkPacketReceiver mutates borrowed resource 'rx buffer'"
        })
        .unwrap_or_else(|| panic!("missing AIL027 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:NetworkPacketReceiver.effect:write rx buffer")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("edge:edge:targets-resource:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Remove 'rx buffer' from 'The component borrows:' or stop mutating it in system component NetworkPacketReceiver."
        )
    );
}

#[test]
fn ail_core_reports_system_use_after_release() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/use-after-release.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL028"
                && diagnostic.message
                    == "system component NetworkPacketReceiver uses resource 'rx buffer' after release"
        })
        .unwrap_or_else(|| panic!("missing AIL028 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:NetworkPacketReceiver.effect:read rx buffer")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("edge:edge:targets-resource:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Move 'read rx buffer' before 'release rx buffer' or remove the post-release use in system component NetworkPacketReceiver."
        )
    );
}

#[test]
fn ail_core_reports_system_shared_and_mutable_borrow_conflicts() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/shared-and-mutable-borrow.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL029"
                && diagnostic.message
                    == "system component NetworkPacketReceiver declares resource 'dma ring' as both shared and mutable borrow"
        })
        .unwrap_or_else(|| panic!("missing AIL029 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:NetworkPacketReceiver")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("edge:edge:mutably-borrows-resource:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Remove 'dma ring' from either 'The component borrows:' or 'The component mutably borrows:' for system component NetworkPacketReceiver."
        )
    );
}

#[test]
fn ail_core_reports_system_use_after_move() {
    let package = load_ail_package_dir(fixture("network_driver.ail")).unwrap();
    let spec = fs::read_to_string(format!(
        "{}/examples/rejected/use-after-move.ail-spec.md",
        fixture("network_driver.ail")
    ))
    .unwrap();
    let document = parse_ail_spec_text(&spec).unwrap();
    let diagnostics = check_ail_core_diagnostics(&elaborate_ail_core(&package, &document));
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "AIL030"
                && diagnostic.message
                    == "system component PacketHandoff uses resource 'rx buffer' after move"
        })
        .unwrap_or_else(|| panic!("missing AIL030 diagnostic: {diagnostics:?}"));

    assert_eq!(
        diagnostic.source_provenance.as_deref(),
        Some("system_component:PacketHandoff.effect:read rx buffer")
    );
    assert!(
        diagnostic
            .affected_graph_item
            .as_deref()
            .is_some_and(|item| item.starts_with("edge:edge:targets-resource:")),
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.repair_suggestion.as_deref(),
        Some(
            "Move 'read rx buffer' before 'move rx buffer' or remove the post-move use in system component PacketHandoff."
        )
    );
}

#[test]
fn cli_ail_conformance_checks_valid_and_rejected_fixtures() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");

    let output = Command::new(binary)
        .args(["ail-conformance", &package])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("valid: spec.ail-spec.md"));
    assert!(stdout.contains("accepted: close-ticket-minimal.ail-spec.md"));
    assert!(stdout.contains("rejected: missing-reference.ail-spec.md AIL001"));
    assert!(
        stdout.contains("source=action:CloseTicket.requirement:the account to exist"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Declare a Thing named 'account' or update the requirement to reference an existing thing."
        ),
        "{stdout}"
    );
    assert!(stdout.contains("rejected: secret-leak.ail-spec.md AIL002"));
    assert!(
        stdout.contains("source=action:PublishNotes.write:customer visible internal notes"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add a 'the system does not reveal Ticket.internal notes' protection bullet to action PublishNotes."
        ),
        "{stdout}"
    );
    assert!(stdout.contains("rejected: secret-read-without-protection.ail-spec.md AIL005"));
    assert!(
        stdout.contains("source=action:InspectNotes.read:ticket internal notes"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add a 'the system does not reveal Ticket.internal notes' protection bullet to action InspectNotes."
        ),
        "{stdout}"
    );
    assert!(stdout.contains("rejected: missing-failure-handler.ail-spec.md AIL003"));
    assert!(
        stdout.contains("source=action:CloseTicket.failure:payment provider rejects the update"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add a 'Failure payment provider rejects the update happens when ...' section with handling and trace bullets."
        ),
        "{stdout}"
    );
    assert!(stdout.contains("rejected: failure-without-handling.ail-spec.md AIL-FAILURE-001"));
    assert!(stdout.contains("source=failure:NotFound"), "{stdout}");
    assert!(
        stdout.contains("repair=Add at least one handling bullet to Failure NotFound."),
        "{stdout}"
    );
    assert!(stdout.contains("rejected: failure-without-trace.ail-spec.md AIL-TRACE-002"));
    assert!(
        stdout.contains("repair=Add a 'the trace records ...' bullet to Failure NotFound."),
        "{stdout}"
    );
    assert!(stdout.contains("rejected: action-without-trace.ail-spec.md AIL-TRACE-001"));
    assert!(stdout.contains("rejected: unknown-field.ail-spec.md AIL004"));
    assert!(
        stdout.contains("source=action:ArchiveTicket.read:ticket owner email"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Declare field 'ticket owner email' on the referenced thing or update the read bullet to an existing field."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("source=action:ArchiveTicket.write:ticket archive code to Archived"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Declare field 'ticket archive code to Archived' on the referenced thing or update the write bullet to an existing field."
        ),
        "{stdout}"
    );
    assert!(stdout.contains("rejected: unknown-field-type.ail-spec.md AIL-TYPE-001"));
    assert!(stdout.contains("source=field:Ticket.metadata"), "{stdout}");
    assert!(
        stdout.contains(
            "repair=Use a supported AIL type for field Ticket.metadata or declare a Thing named 'MysteryBox'."
        ),
        "{stdout}"
    );
    assert!(stdout.contains("rejected: unknown-requirement-field.ail-spec.md AIL007"));
    assert!(
        stdout.contains("source=action:CloseTicket.requirement:the ticket priority not to be High"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Declare field 'ticket priority' on the referenced thing or update the requirement to an existing field."
        ),
        "{stdout}"
    );
    assert!(stdout.contains("ail conformance: ok"));
}

#[test]
fn cli_ail_conformance_writes_auditable_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-conformance-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-conformance",
            &package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    let report = fs::read_to_string(artifact_dir.join("conformance-report.txt")).unwrap();
    assert_eq!(report, stdout);
    let expected_report_fingerprint = fnv64_fingerprint(&report);
    let report_fingerprint =
        fs::read_to_string(artifact_dir.join("conformance-report.fingerprint.txt")).unwrap();
    assert_eq!(report_fingerprint.trim(), expected_report_fingerprint);

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-conformance.txt")).unwrap();
    assert!(manifest.contains("AIL-Conformance-Manifest:"), "{manifest}");
    assert!(manifest.contains("package support-ticket"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "report conformance-report.txt {expected_report_fingerprint}"
        )),
        "{manifest}"
    );
    assert!(manifest.contains("valid spec.ail-spec.md"), "{manifest}");
    assert!(
        manifest.contains("accepted close-ticket-minimal.ail-spec.md"),
        "{manifest}"
    );
    assert!(
        manifest.contains("rejected missing-reference.ail-spec.md"),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_conformance_records_dependency_report_for_imported_package_graph() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_composed.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-conformance-imported-package-dependencies-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-conformance",
            &package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Package-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains(
            "resolved-import Shared path=../support_shared.ail requirement=none name=support-shared version=0.1.0"
        ),
        "{dependency_report}"
    );
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-conformance.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_conformance_agent_verifies_manifest_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-conformance-agent-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-conformance",
            &package,
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    assert!(agent_bytecode.contains(r#""package":"ail-toolchain-agent""#));
    assert!(agent_bytecode.contains(r#""action":"VerifyConformanceManifest""#));
    let agent_fingerprint = fs::read_to_string(artifact_dir.join("agent.fingerprint.txt")).unwrap();
    assert_eq!(agent_fingerprint.trim(), fnv64_fingerprint(&agent_bytecode));
    let parsed_agent = parse_ail_bytecode(&agent_bytecode).unwrap();
    assert_eq!(verify_ail_bytecode(&parsed_agent), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action VerifyConformanceManifest started"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest fingerprint"));
    assert!(agent_trace.contains("read buildrequest.conformance report"));
    assert!(agent_trace.contains("read buildrequest.conformance report fingerprint"));
    assert!(
        agent_trace.contains("write buildrequest.artifact manifest verification report=Verified")
    );
    assert!(agent_trace.contains("trace ConformanceManifestVerified"));

    let report = fs::read_to_string(artifact_dir.join("conformance-report.txt")).unwrap();
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-conformance.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "report conformance-report.txt {}",
            fnv64_fingerprint(&report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "agent agent.ailbc.json {}",
            fnv64_fingerprint(&agent_bytecode)
        )),
        "{manifest}"
    );
    assert!(manifest.contains("trace agent-trace.txt"), "{manifest}");
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_conformance_writes_native_agent_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-conformance-native-agent-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-conformance",
            &package,
            "--agent",
            &agent_package,
            "--target",
            "linux-x86_64-elf",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let agent_native = fs::read(artifact_dir.join("agent-VerifyConformanceManifest.elf")).unwrap();
    assert_eq!(&agent_native[0..4], b"\x7fELF");
    let expected_agent_native_fingerprint = fnv64_fingerprint_bytes(&agent_native);
    let native_run = Command::new(artifact_dir.join("agent-VerifyConformanceManifest.elf"))
        .args([
            "buildrequest.id=support-ticket-conformance",
            "buildrequest.status=BytecodeReady",
            "buildrequest.conformance report=ok",
            "buildrequest.conformance report fingerprint=fnv64:report",
            "buildrequest.artifact manifest=ok",
            "buildrequest.artifact manifest fingerprint=fnv64:manifest",
            "buildrequest.machine bytecode contract=machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable",
        ])
        .output()
        .unwrap();
    assert!(
        native_run.status.success(),
        "native conformance agent verifier failed"
    );
    assert!(
        String::from_utf8_lossy(&native_run.stderr).contains("trace ConformanceManifestVerified"),
        "{}",
        String::from_utf8_lossy(&native_run.stderr)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-conformance.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "agent-target linux-x86_64-elf agent-VerifyConformanceManifest.elf {expected_agent_native_fingerprint}"
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains("machine-bytecode-contract linux-x86_64-elf bytecode-level machine bytecode-container linux-elf-executable bytecode-format elf64-little-x86_64-executable"),
        "{manifest}"
    );
    let native_bytecode_report =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.txt")).unwrap();
    assert!(
        native_bytecode_report.contains("AIL-Conformance-Native-Bytecode:"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("target linux-x86_64-elf"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("bytecode-level machine"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("bytecode-container linux-elf-executable"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("bytecode-format elf64-little-x86_64-executable"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains(&format!(
            "machine-bytecode agent-target linux-x86_64-elf agent-VerifyConformanceManifest.elf elf64-little-x86_64-executable {expected_agent_native_fingerprint} bytes {}",
            agent_native.len()
        )),
        "{native_bytecode_report}"
    );
    let native_bytecode_report_fingerprint =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.fingerprint.txt")).unwrap();
    assert_eq!(
        native_bytecode_report_fingerprint.trim(),
        fnv64_fingerprint(&native_bytecode_report)
    );
    assert!(
        manifest.contains(&format!(
            "native-bytecode native-bytecode-report.txt {}",
            fnv64_fingerprint(&native_bytecode_report)
        )),
        "{manifest}"
    );
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Conformance-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("target linux-x86_64-elf"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("host-language-runtime none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("dynamic-linker none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("shared-libraries none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("library-dependencies none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("linker-invocation none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains(
            "machine-bytecode-dependency agent-VerifyConformanceManifest.elf standalone-linux-syscall-elf"
        ),
        "{dependency_report}"
    );
    let dependency_report_fingerprint =
        fs::read_to_string(artifact_dir.join("dependency-report.fingerprint.txt")).unwrap();
    assert_eq!(
        dependency_report_fingerprint.trim(),
        fnv64_fingerprint(&dependency_report)
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action VerifyConformanceManifest started"));
    assert!(agent_trace.contains("read buildrequest.machine bytecode contract"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report"));
    assert!(agent_trace.contains("read buildrequest.native bytecode report fingerprint"));
    assert!(agent_trace.contains("read buildrequest.dependency report"));
    assert!(agent_trace.contains("read buildrequest.dependency report fingerprint"));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_conformance_checks_agent_tool_fixtures() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("refund_tool.ail");

    let output = Command::new(binary)
        .args(["ail-conformance", &package])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail conformance: package refund-tool"));
    assert!(stdout.contains("valid: spec.ail-spec.md"));
    assert!(stdout.contains("accepted: refund-minimal.ail-spec.md"));
    assert!(stdout.contains("rejected: approval-without-rule.ail-spec.md AIL018"));
    assert!(stdout.contains("rejected: permission-without-rule.ail-spec.md AIL019"));
    assert!(stdout.contains("rejected: secret-output.ail-spec.md AIL020"));
    assert!(stdout.contains("rejected: unknown-input-type.ail-spec.md AIL-TYPE-001"));
    assert!(stdout.contains("rejected: tool-without-trace.ail-spec.md AIL-TRACE-001"));
    assert!(
        stdout.contains("source=tool:RefundCustomerPayment.input:payment token"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Use a supported AIL type for input RefundCustomerPayment.payment token or declare a Thing named 'MysteryCredential'."
        ),
        "{stdout}"
    );
    assert!(stdout.contains("ail conformance: ok"));
}

#[test]
fn cli_ail_conformance_checks_compiler_profile_fixtures() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("compiler_pass.ail");

    let output = Command::new(binary)
        .args(["ail-conformance", &package])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail conformance: package ail-meta-permissions"));
    assert!(stdout.contains("valid: spec.ail-spec.md"));
    assert!(stdout.contains("accepted: infer-read-permissions-minimal.ail-spec.md"));
    assert!(stdout.contains("rejected: unknown-value-type.ail-spec.md AIL-TYPE-001"));
    assert!(
        stdout.contains("source=compiler_pass:InferReadPermissions.output:diagnostics"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Use a supported AIL type for value InferReadPermissions.diagnostics or declare a Thing named 'MysteryDiagnostic'."
        ),
        "{stdout}"
    );
    assert!(stdout.contains("ail conformance: ok"));
}

#[test]
fn cli_ail_conformance_checks_system_profile_fixtures() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("network_driver.ail");

    let output = Command::new(binary)
        .args(["ail-conformance", &package])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail conformance: package network-driver"));
    assert!(stdout.contains("valid: spec.ail-spec.md"));
    assert!(stdout.contains("accepted: allocation-minimal.ail-spec.md"));
    assert!(stdout.contains("accepted: interrupt-context-minimal.ail-spec.md"));
    assert!(stdout.contains("accepted: interrupt-mask-minimal.ail-spec.md"));
    assert!(stdout.contains("accepted: interrupt-priority-minimal.ail-spec.md"));
    assert!(stdout.contains("accepted: layout-minimal.ail-spec.md"));
    assert!(stdout.contains("accepted: lock-guard-minimal.ail-spec.md"));
    assert!(stdout.contains("accepted: move-resource-minimal.ail-spec.md"));
    assert!(stdout.contains("accepted: mutable-borrow-minimal.ail-spec.md"));
    assert!(stdout.contains("accepted: packet-receive-minimal.ail-spec.md"));
    assert!(stdout.contains("accepted: scheduler-task-priority-minimal.ail-spec.md"));
    assert!(stdout.contains("accepted: scheduler-task-minimal.ail-spec.md"));
    assert!(stdout.contains("accepted: scheduler-task-timing-minimal.ail-spec.md"));
    assert!(
        stdout.contains("rejected: device-effect-without-matching-capability.ail-spec.md AIL023")
    );
    assert!(stdout.contains("rejected: allocation-unknown-resource.ail-spec.md AIL032"));
    assert!(stdout.contains("rejected: effect-without-capability.ail-spec.md AIL021"));
    assert!(stdout.contains("rejected: interrupt-context-blocking-effect.ail-spec.md AIL033"));
    assert!(stdout.contains("rejected: interrupt-mask-unknown-context.ail-spec.md AIL040"));
    assert!(stdout.contains("rejected: interrupt-priority-unknown-context.ail-spec.md AIL034"));
    assert!(stdout.contains("rejected: layout-unknown-resource.ail-spec.md AIL031"));
    assert!(stdout.contains("rejected: lock-guard-unknown-lock.ail-spec.md AIL039"));
    assert!(stdout.contains("rejected: lock-guard-unknown-resource.ail-spec.md AIL038"));
    assert!(stdout.contains("rejected: move-without-ownership.ail-spec.md AIL024"));
    assert!(stdout.contains("rejected: mutable-effect-without-ownership.ail-spec.md AIL024"));
    assert!(stdout.contains("rejected: mutate-borrowed-resource.ail-spec.md AIL027"));
    assert!(stdout.contains("rejected: read-effect-without-borrow.ail-spec.md AIL025"));
    assert!(stdout.contains("rejected: resource-without-region.ail-spec.md AIL026"));
    assert!(stdout.contains("rejected: scheduler-task-priority-unknown-task.ail-spec.md AIL036"));
    assert!(stdout.contains("rejected: scheduler-task-unknown-context.ail-spec.md AIL035"));
    assert!(stdout.contains("rejected: scheduler-task-timing-unknown-task.ail-spec.md AIL037"));
    assert!(stdout.contains("rejected: shared-and-mutable-borrow.ail-spec.md AIL029"));
    assert!(stdout.contains("rejected: use-after-move.ail-spec.md AIL030"));
    assert!(stdout.contains("rejected: use-after-release.ail-spec.md AIL028"));
    assert!(stdout.contains("rejected: unknown-effect-resource.ail-spec.md AIL022"));
    assert!(
        stdout.contains("source=system_component:NetworkPacketReceiver.effect:read network device"),
        "{stdout}"
    );
    assert!(
        stdout.contains("source=system_component:NetworkPacketReceiver.effect:read network device"),
        "{stdout}"
    );
    assert!(
        stdout.contains("source=system_component:NetworkPacketReceiver.effect:read dma ring"),
        "{stdout}"
    );
    assert!(
        stdout.contains("source=system_component:NetworkPacketReceiver.effect:write rx buffer"),
        "{stdout}"
    );
    assert!(
        stdout
            .contains("source=system_component:NetworkPacketReceiver.effect:read packet metadata"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add a 'The component requires capability:' section to system component NetworkPacketReceiver."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Declare resource 'dma ring' in 'The component uses:' or update the layout bullet for system component PacketLayout."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Declare resource 'dma ring' in 'The component uses:' or update the allocation bullet for system component PacketAllocator."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Move blocking effect 'wait for scheduler' out of interrupt context or remove the 'interrupt' context declaration for system component TimerInterruptHandler."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add 'interrupt' to 'The component runs in context:' or update the priority bullet for system component TimerPriority."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add 'interrupt' to 'The component runs in context:' or update the interrupt mask bullet for system component TimerMask."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add 'process' to 'The component runs in context:' or update the task bullet for system component PacketScheduler."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add 'packet poller' to 'The component schedules task:' or update the task priority bullet for system component PacketScheduler."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add 'packet poller' to 'The component schedules task:' or update the task timing bullet for system component PacketScheduler."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Declare lock resource 'scheduler lock' in 'The component uses:' or update the lock guard bullet for system component PacketScheduler."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Declare resource 'scheduler state' in 'The component uses:' or update the lock guard bullet for system component PacketScheduler."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Declare resource 'dma ring' on system component NetworkPacketReceiver or update the effect to target a declared resource."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add a capability such as 'access network device' to system component NetworkPacketReceiver."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add 'rx buffer' to 'The component owns:' for system component NetworkPacketReceiver."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add 'rx buffer' to 'The component owns:' for system component PacketHandoff before moving it."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Remove 'rx buffer' from 'The component borrows:' or stop mutating it in system component NetworkPacketReceiver."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Move 'read rx buffer' before 'release rx buffer' or remove the post-release use in system component NetworkPacketReceiver."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Move 'read rx buffer' before 'move rx buffer' or remove the post-move use in system component PacketHandoff."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Remove 'dma ring' from either 'The component borrows:' or 'The component mutably borrows:' for system component NetworkPacketReceiver."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add 'packet metadata' to 'The component borrows:' or 'The component owns:' for system component NetworkPacketReceiver."
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "repair=Add 'packet metadata in <region>' to 'The component places:' for system component NetworkPacketReceiver."
        ),
        "{stdout}"
    );
    assert!(stdout.contains("ail conformance: ok"));
}

#[test]
fn cli_ail_draft_uses_llm_endpoint_and_checks_candidate_spec() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-draft",
            &package,
            "--prompt",
            "Draft an AIL support ticket app",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(request_body.contains(r#""messages":"#));
    assert!(request_body.contains(r#""chat_template_kwargs":{"enable_thinking":false}"#));
    assert!(request_body.contains("Draft an AIL support ticket app"));
    assert!(request_body.contains("package support-ticket"));
    assert!(request_body.contains("AIL-Spec"));
    assert!(request_body.contains("# Prompt: spec-draft.system"));
    assert!(request_body.contains("version: 0.1.0"));
    assert!(request_body.contains("target artifact: AIL-Spec Canonical"));
    assert!(request_body.contains("prompt_file: spec-draft.system.md"));
    assert!(request_body.contains("prompt_version: 0.1.0"));
    assert!(request_body.contains(&format!(
        "prompt_fingerprint: {}",
        fnv64_fingerprint(SPEC_DRAFT_PROMPT_ASSET)
    )));
    assert!(request_body.contains("The application <Name> manages <purpose>."));
    assert!(request_body.contains("A <Thing> has:"));
    assert!(request_body.contains("Action: <human label>."));
    assert!(request_body.contains("Failure <Name> happens when <condition>:"));
    assert!(request_body.contains("Secret<List<Text>>"));
    assert!(request_body.contains("artifact_kind"));
    assert!(request_body.contains("AIL-Spec Canonical"));
    assert!(request_body.contains("checker_handoff"));
    assert!(request_body.contains("expected_profile"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail-draft candidate:"));
    assert!(stdout.contains("Action: Close ticket."));
    assert!(stdout.contains("ail-draft diagnostics: none"));
}

#[test]
fn cli_ail_requirements_root_llm_endpoint_uses_completion_api() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- Store ticket object fields id, status, title, customer, assignee, due_at, public updates, and internal notes data.\n",
        "- Actions create ticket, assign ticket, close ticket, and mark overdue update behavior.\n",
        "- Each action requires runtime input such as ticket id, caller role, status precondition, and title.\n",
        "- Failure cases include ticket not found and permission denied errors.\n",
        "- Trace records TicketCreated, TicketAssigned, TicketClosed, TicketOverdue, TicketNotFound, and InternalNotesDenied audit events.\n",
        "- Guarantees always preserve secret internal notes and closed tickets do not appear in the open queue.\n"
    );
    let response_body = format!(r#"{{"content":{}}}"#, json_string(requirements));
    let server = serve_one_llm_response_with_request_line(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-requirements",
            &package,
            "--prompt",
            "Capture support ticket requirements through the AI IDE",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/", addr.port()),
        ])
        .output()
        .unwrap();

    let (request_line, request_body) = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_line, "POST /completion HTTP/1.1");
    assert!(request_body.contains(r#""prompt":"#), "{request_body}");
    assert!(!request_body.contains(r#""messages":"#), "{request_body}");
    assert!(request_body.contains("Capture support ticket requirements through the AI IDE"));
    assert!(request_body.contains("# Prompt: requirements.system"));
    assert!(request_body.contains("version: 0.1.0"));
    assert!(request_body.contains("target artifact: AIL-Requirements"));
    assert!(request_body.contains("prompt_file: requirements.system.md"));
    assert!(request_body.contains("prompt_version: 0.1.0"));
    assert!(request_body.contains(&format!(
        "prompt_fingerprint: {}",
        fnv64_fingerprint(REQUIREMENTS_PROMPT_ASSET)
    )));
    assert!(request_body.contains("artifact_kind"));
    assert!(request_body.contains("AIL-Requirements"));
    assert!(request_body.contains("checker_handoff"));
    assert!(request_body.contains("expected_profile"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("AIL-Requirements:"));
    assert!(stdout.contains("TicketClosed"));
}

#[test]
fn cli_ail_interview_surfaces_prompt_envelope_questions_as_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let artifact_dir =
        std::env::temp_dir().join(format!("ail-interview-artifacts-{}", std::process::id()));
    let _ = fs::remove_dir_all(&artifact_dir);
    let envelope = concat!(
        "{",
        "\"artifact_kind\":\"AIL-Interview\",",
        "\"artifact_text\":\"\",",
        "\"questions\":[\"Which user roles may close tickets?\",\"Which trace events must be emitted?\"],",
        "\"assumptions\":[],",
        "\"provenance\":[\"prompt:roles\",\"prompt:trace\"],",
        "\"checker_handoff\":{\"must_check\":true,\"expected_profile\":\"Application\",\"expected_features\":[]}",
        "}"
    );
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(envelope)
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-interview",
            &package,
            "--prompt",
            "Build a support ticket app",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let request_body = server.join().unwrap();
    assert!(request_body.contains("# Prompt: interview.system"));
    assert!(request_body.contains("target artifact: blocking questions or AIL-Requirements seed"));
    assert!(request_body.contains("prompt_file: interview.system.md"));
    assert!(request_body.contains("prompt_version: 0.1.0"));
    assert!(request_body.contains(&format!(
        "prompt_fingerprint: {}",
        fnv64_fingerprint(INTERVIEW_PROMPT_ASSET)
    )));
    assert!(request_body.contains("artifact_kind"));
    assert!(request_body.contains("AIL-Interview"));
    assert!(request_body.contains("Build a support ticket app"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("AIL-Interview:\n"), "{stdout}");
    assert!(
        stdout.contains("- Which user roles may close tickets?"),
        "{stdout}"
    );
    assert!(
        stdout.contains("- Which trace events must be emitted?"),
        "{stdout}"
    );
    assert!(!stdout.contains("AIL-Requirements:"), "{stdout}");

    let interview_artifact =
        fs::read_to_string(artifact_dir.join("interview.ail-interview.md")).unwrap();
    assert_eq!(interview_artifact, stdout);
    let interview_fingerprint =
        fs::read_to_string(artifact_dir.join("interview.fingerprint.txt")).unwrap();
    assert_eq!(
        interview_fingerprint.trim(),
        fnv64_fingerprint(&interview_artifact)
    );
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-interview.txt")).unwrap();
    assert!(manifest.contains("AIL-Interview-Manifest:"), "{manifest}");
    assert!(
        manifest.contains("package support-ticket 0.1.0"),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "interview interview.ail-interview.md {}",
            fnv64_fingerprint(&interview_artifact)
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_draft_accepts_prompt_envelope_artifact_text() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let envelope = format!(
        concat!(
            "{{",
            "\"artifact_kind\":\"AIL-Spec Canonical\",",
            "\"artifact_text\":{},",
            "\"questions\":[],",
            "\"assumptions\":[],",
            "\"provenance\":[\"mock:0\"],",
            "\"checker_handoff\":{{\"must_check\":true,\"expected_profile\":\"Application\",\"expected_features\":[]}}",
            "}}"
        ),
        json_string(&response_spec)
    );
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!("```json\n{envelope}\n```"))
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-draft",
            &package,
            "--prompt",
            "Draft an AIL support ticket app with an envelope response",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(request_body.contains("Draft an AIL support ticket app with an envelope response"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail-draft candidate:"));
    assert!(stdout.contains("Action: Close ticket."));
    assert!(stdout.contains("ail-draft diagnostics: none"));
    assert!(!stdout.contains("artifact_text"), "{stdout}");
}

#[test]
fn cli_ail_draft_prints_structured_candidate_diagnostics() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(format!(
        "{package}/examples/rejected/missing-reference.ail-spec.md"
    ))
    .unwrap();
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-draft",
            &package,
            "--prompt",
            "Draft an AIL support ticket app with a bad requirement",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(request_body.contains(r#""chat_template_kwargs":{"enable_thinking":false}"#));
    assert!(request_body.contains("bad requirement"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail-draft candidate:"));
    assert!(stdout.contains("ail-draft diagnostics:"));
    assert!(
        stdout.contains("AIL001 unknown requirement reference 'account' in action CloseTicket"),
        "{stdout}"
    );
    assert!(
        stdout.contains("source=action:CloseTicket.requirement:the account to exist"),
        "{stdout}"
    );
    assert!(stdout.contains("graph=node:Rule:"), "{stdout}");
    assert!(
        stdout.contains(
            "repair=Declare a Thing named 'account' or update the requirement to reference an existing thing."
        ),
        "{stdout}"
    );
}

#[test]
fn cli_ail_draft_can_emit_machine_readable_diagnostics() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(format!(
        "{package}/examples/rejected/missing-reference.ail-spec.md"
    ))
    .unwrap();
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-draft",
            &package,
            "--prompt",
            "Draft an AIL support ticket app with a bad requirement",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
            "--diagnostics-json",
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(request_body.contains("bad requirement"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("{\n"), "{stdout}");
    assert!(stdout.contains(r#""candidate_artifact":"#), "{stdout}");
    assert!(stdout.contains("Action: Close ticket."), "{stdout}");
    assert!(stdout.contains(r#""diagnostics":"#), "{stdout}");
    assert!(stdout.contains(r#""code":"AIL001""#), "{stdout}");
    assert!(
        stdout.contains(
            r#""message":"unknown requirement reference 'account' in action CloseTicket""#
        ),
        "{stdout}"
    );
    assert!(stdout.contains(r#""severity":"error""#), "{stdout}");
    assert!(
        stdout.contains(
            r#""source_provenance":"action:CloseTicket.requirement:the account to exist""#
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(r#""affected_graph_item":"node:Rule:"#),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            r#""repair_suggestion":"Declare a Thing named 'account' or update the requirement to reference an existing thing.""#
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("ail-draft diagnostics:"), "{stdout}");
}

#[test]
fn cli_ail_build_uses_llm_candidate_and_outputs_verified_bytecode() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- A ticket has fields id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Closing a ticket changes ticket status to Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let request_bodies = server.join().unwrap();
    assert_eq!(request_bodies.len(), 2);
    assert!(request_bodies[0].contains(r#""chat_template_kwargs":{"enable_thinking":false}"#));
    assert!(request_bodies[0].contains("Draft AIL requirements"));
    assert!(request_bodies[0].contains("Build an AIL support ticket bytecode artifact"));
    assert!(request_bodies[0].contains("application domain objects"));
    assert!(!request_bodies[0].contains("compiler passes"));
    assert!(!request_bodies[0].contains("system components"));
    assert!(request_bodies[1].contains("Draft an AIL-Spec candidate"));
    assert!(request_bodies[1].contains("DRAFT REQUIREMENTS:"));
    assert!(request_bodies[1].contains("Closing a ticket changes ticket status to Closed"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    assert!(bytecode.actions.contains_key("CloseTicket"));

    let run = run_ail_bytecode_action(
        &bytecode,
        "CloseTicket",
        BTreeMap::from([
            ("ticket.id".to_string(), "T-1".to_string()),
            ("ticket.status".to_string(), "Open".to_string()),
        ]),
    )
    .unwrap();
    assert_eq!(run.status, "succeeded");
    assert_eq!(run.final_state["ticket.status"], "Closed");
}

#[test]
fn cli_ail_build_retries_malformed_requirements_prompt_envelope() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let invalid_envelope = concat!(
        r#"{"artifact_kind":"AIL-Requirements","#,
        r#""artifact_text":"AIL-Requirements:\n- The application manages support tickets.\n","#,
        r#""questions":["Which trace events must be emitted?"],"#,
        r#""checker_handoff":{"must_check":true,"expected_profile":"Application"}}"#
    );
    let invalid_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(invalid_envelope)
    );
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![invalid_body, requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 3);
    assert!(
        request_bodies[1].contains(
            "AIL-PROMPT-001 prompt envelope cannot contain both artifact_text and questions"
        ),
        "{}",
        request_bodies[1]
    );
    let bytecode = parse_ail_bytecode(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    assert!(bytecode.actions.contains_key("CloseTicket"));
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_build_native_prompt_names_requested_action_in_llm_prompts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let executable_path = std::env::temp_dir().join(format!(
        "ail-build-native-prompt-action-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!("```ail\n{response_spec}\n```"))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--target",
            "linux-x86_64-elf",
            "--action",
            "CloseTicket",
            "--out",
            executable_path.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    assert!(
        request_bodies[0].contains("must define action CloseTicket"),
        "{}",
        request_bodies[0]
    );
    assert!(
        request_bodies[0].contains("PACKAGE SOURCE AIL-SPEC CONTEXT:"),
        "{}",
        request_bodies[0]
    );
    assert!(
        request_bodies[0].contains("Action: Close ticket."),
        "{}",
        request_bodies[0]
    );
    assert!(
        request_bodies[1].contains("must define action CloseTicket"),
        "{}",
        request_bodies[1]
    );
    assert!(
        request_bodies[1].contains("PACKAGE SOURCE AIL-SPEC CONTEXT:"),
        "{}",
        request_bodies[1]
    );
    assert!(executable_path.exists());

    fs::remove_file(executable_path).unwrap();
}

#[test]
fn cli_ail_build_includes_saved_interview_answers_in_requirements_prompt() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let interview_path = std::env::temp_dir().join(format!(
        "ail-build-interview-answers-{}.ail-interview.md",
        std::process::id()
    ));
    fs::write(
        &interview_path,
        concat!(
            "AIL-Interview:\n",
            "- Q: Which user roles may close tickets?\n",
            "  A: SupportAgent and SupportManager.\n",
            "- Q: Which trace events must be emitted?\n",
            "  A: TicketClosed and TicketNotFound.\n"
        ),
    )
    .unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- A ticket has fields id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- SupportAgent and SupportManager may close tickets.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact after interview",
            "--interview-file",
            interview_path.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let request_bodies = server.join().unwrap();
    assert_eq!(request_bodies.len(), 2);
    assert!(
        request_bodies[0].contains("SAVED INTERVIEW ANSWERS:"),
        "{}",
        request_bodies[0]
    );
    assert!(
        request_bodies[0].contains("SupportAgent and SupportManager"),
        "{}",
        request_bodies[0]
    );
    assert!(
        request_bodies[0].contains("TicketClosed and TicketNotFound"),
        "{}",
        request_bodies[0]
    );
    assert!(
        !request_bodies[1].contains("SAVED INTERVIEW ANSWERS:"),
        "{}",
        request_bodies[1]
    );
    assert!(request_bodies[1].contains("DRAFT REQUIREMENTS:"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    fs::remove_file(interview_path).unwrap();
}

#[test]
fn cli_ail_build_repairs_rejected_candidate_before_lowering() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires declared ticket fields only.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let rejected_spec = fs::read_to_string(format!(
        "{package}/examples/rejected/missing-reference.ail-spec.md"
    ))
    .unwrap();
    let rejected_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{rejected_spec}\n```"
        ))
    );
    let repaired_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let repaired_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{repaired_spec}\n```"
        ))
    );
    let server = serve_chat_responses(
        listener,
        vec![requirements_body, rejected_body, repaired_body],
    );

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let request_bodies = server.join().unwrap();
    assert_eq!(request_bodies.len(), 3);
    assert!(request_bodies[1].contains("DRAFT REQUIREMENTS:"));
    assert!(request_bodies[2].contains("Repair an AIL-Spec candidate"));
    assert!(request_bodies[2].contains("AIL001 unknown requirement reference"));
    assert!(
        request_bodies[2].contains(
            "repair=Declare a Thing named 'account' or update the requirement to reference an existing thing."
        ),
        "{}",
        request_bodies[2]
    );
    assert!(request_bodies[2].contains("PREVIOUS AIL-SPEC CANDIDATE:"));
    assert!(request_bodies[2].contains("DRAFT REQUIREMENTS:"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(bytecode.profile, "Application");
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    assert!(bytecode.actions.contains_key("CloseTicket"));
}

#[test]
fn cli_ail_build_repairs_spec_that_drops_permission_requirement() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Permission check must restrict the CloseTicket action to support agents.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let dropped_permission_spec =
        fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let dropped_permission_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{dropped_permission_spec}\n```"
        ))
    );
    let repaired_spec = dropped_permission_spec.replace(
        "- the system requires the ticket status not to be Closed",
        concat!(
            "- the system requires the ticket status not to be Closed\n",
            "- the system requires the assignee role to be SupportAgent or SupportManager"
        ),
    );
    let repaired_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{repaired_spec}\n```"
        ))
    );
    let server = serve_chat_responses(
        listener,
        vec![requirements_body, dropped_permission_body, repaired_body],
    );

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let request_bodies = server.join().unwrap();
    assert_eq!(request_bodies.len(), 3);
    assert!(request_bodies[2].contains("Repair an AIL-Spec candidate"));
    assert!(request_bodies[2].contains("AILR011"));
    assert!(request_bodies[2].contains("permission requirement for action CloseTicket"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    let close_ticket = bytecode.actions.get("CloseTicket").unwrap();
    assert!(close_ticket.instructions.iter().any(|instruction| {
        instruction.opcode == "REQUIRE_FIELD_IN"
            && instruction
                .operands
                .get("key")
                .is_some_and(|key| key == "ticket.assignee.role")
    }));
}

#[test]
fn cli_ail_build_repairs_spec_that_drops_failure_requirement() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure ProviderRejected happens when payment provider rejects the ticket closure and records ProviderRejected.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let dropped_failure_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let dropped_failure_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{dropped_failure_spec}\n```"
        ))
    );
    let repaired_spec = format!(
        "{}\n\n{}",
        dropped_failure_spec.trim_end(),
        concat!(
            "Failure ProviderRejected happens when payment provider rejects the ticket closure:\n\n",
            "- the caller sees \"Provider rejected\"\n",
            "- the trace records ProviderRejected\n"
        )
    );
    let repaired_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{repaired_spec}\n```"
        ))
    );
    let server = serve_chat_responses(
        listener,
        vec![requirements_body, dropped_failure_body, repaired_body],
    );

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let request_bodies = server.join().unwrap();
    assert_eq!(request_bodies.len(), 3);
    assert!(request_bodies[2].contains("Repair an AIL-Spec candidate"));
    assert!(request_bodies[2].contains("AILR012"));
    assert!(request_bodies[2].contains("Failure ProviderRejected"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert!(bytecode.failures.contains_key("ProviderRejected"));
}

#[test]
fn cli_ail_build_repairs_spec_that_drops_trace_requirement() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The CloseTicket action records trace event TicketClosureAudited.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let dropped_trace_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let dropped_trace_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{dropped_trace_spec}\n```"
        ))
    );
    let repaired_spec = dropped_trace_spec.replace(
        "- the system records a trace event named TicketClosed",
        "- the system records a trace event named TicketClosureAudited",
    );
    let repaired_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{repaired_spec}\n```"
        ))
    );
    let server = serve_chat_responses(
        listener,
        vec![requirements_body, dropped_trace_body, repaired_body],
    );

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let request_bodies = server.join().unwrap();
    assert_eq!(request_bodies.len(), 3);
    assert!(request_bodies[2].contains("Repair an AIL-Spec candidate"));
    assert!(request_bodies[2].contains("AILR013"));
    assert!(request_bodies[2].contains("TicketClosureAudited"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    let close_ticket = bytecode.actions.get("CloseTicket").unwrap();
    assert!(close_ticket.instructions.iter().any(|instruction| {
        instruction.opcode == "EMIT_TRACE"
            && instruction
                .operands
                .get("event")
                .is_some_and(|event| event == "TicketClosureAudited")
    }));
}

#[test]
fn cli_ail_build_repairs_incomplete_requirements_before_spec_drafting() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let incomplete_requirements = "AIL-Requirements:\n- Build support tickets.\n";
    let incomplete_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(incomplete_requirements)
    );
    let repaired_requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets with Ticket fields id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let repaired_requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(repaired_requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(
        listener,
        vec![incomplete_body, repaired_requirements_body, spec_body],
    );

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 3);
    assert!(request_bodies[0].contains("Draft AIL requirements"));
    assert!(request_bodies[1].contains("Repair AIL requirements"));
    assert!(request_bodies[1].contains("AILR003 requirements are missing failure coverage"));
    assert!(!request_bodies[1].contains("Draft an AIL-Spec candidate"));
    assert!(request_bodies[2].contains("Draft an AIL-Spec candidate"));
    assert!(request_bodies[2].contains("Failure NotFound happens when ticket id is missing"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    assert!(bytecode.actions.contains_key("CloseTicket"));
}

#[test]
fn cli_ail_requirements_repairs_incomplete_capture_before_printing() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let incomplete_requirements = "AIL-Requirements:\n- Build support tickets.\n";
    let incomplete_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(incomplete_requirements)
    );
    let repaired_requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets with Ticket fields id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let repaired_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(repaired_requirements)
    );
    let server = serve_chat_responses(listener, vec![incomplete_body, repaired_body]);

    let output = Command::new(binary)
        .args([
            "ail-requirements",
            &package,
            "--prompt",
            "Capture requirements for a support ticket app",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    assert!(request_bodies[0].contains("Draft AIL requirements"));
    assert!(request_bodies[0].contains("Capture requirements for a support ticket app"));
    assert!(request_bodies[1].contains("Repair AIL requirements"));
    assert!(request_bodies[1].contains("AILR003 requirements are missing failure coverage"));
    assert!(!request_bodies[1].contains("Draft an AIL-Spec candidate"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("AIL-Requirements:\n"), "{stdout}");
    assert!(stdout.contains("Ticket fields id, title, status, and secret internal notes"));
    assert!(stdout.contains("Failure NotFound happens when ticket id is missing"));
    assert!(stdout.contains("trace event TicketClosed"));
    assert!(!stdout.contains("Action: Close ticket."), "{stdout}");
}

#[test]
fn cli_ail_requirements_accepts_prompt_envelope_artifact_text() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets with Ticket fields id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n",
        "- Runtime inputs include ticket.id and ticket.status.\n"
    );
    let envelope = format!(
        concat!(
            "{{",
            "\"artifact_kind\":\"AIL-Requirements\",",
            "\"artifact_text\":{},",
            "\"questions\":[],",
            "\"assumptions\":[],",
            "\"provenance\":[\"mock:0\"],",
            "\"checker_handoff\":{{\"must_check\":true,\"expected_profile\":\"Application\",\"expected_features\":[]}}",
            "}}"
        ),
        json_string(requirements)
    );
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!("```json\n{envelope}\n```"))
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-requirements",
            &package,
            "--prompt",
            "Capture requirements for a support ticket app with an envelope response",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        request_body
            .contains("Capture requirements for a support ticket app with an envelope response")
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("AIL-Requirements:\n"), "{stdout}");
    assert!(stdout.contains("Ticket fields id, title, status, and secret internal notes"));
    assert!(!stdout.contains("artifact_text"), "{stdout}");
}

#[test]
fn cli_ail_requirements_includes_saved_interview_answers_in_capture_prompt() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let interview_path = std::env::temp_dir().join(format!(
        "ail-requirements-interview-answers-{}.ail-interview.md",
        std::process::id()
    ));
    fs::write(
        &interview_path,
        concat!(
            "AIL-Interview:\n",
            "- Q: Which user roles may close tickets?\n",
            "  A: SupportAgent and SupportManager.\n",
            "- Q: Which trace events must be emitted?\n",
            "  A: TicketClosed and TicketNotFound.\n"
        ),
    )
    .unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets with Ticket fields id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- SupportAgent and SupportManager may close tickets.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-requirements",
            &package,
            "--prompt",
            "Capture requirements after interview answers",
            "--interview-file",
            interview_path.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let request_body = server.join().unwrap();
    assert!(
        request_body.contains("SAVED INTERVIEW ANSWERS:"),
        "{request_body}"
    );
    assert!(
        request_body.contains("SupportAgent and SupportManager"),
        "{request_body}"
    );
    assert!(
        request_body.contains("TicketClosed and TicketNotFound"),
        "{request_body}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("AIL-Requirements:\n"), "{stdout}");
    assert!(stdout.contains("SupportAgent and SupportManager"));

    fs::remove_file(interview_path).unwrap();
}

#[test]
fn cli_ail_requirements_rejects_prompt_envelope_wrong_artifact_kind() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets with Ticket fields id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n",
        "- Runtime inputs include ticket.id and ticket.status.\n"
    );
    let envelope = format!(
        concat!(
            "{{",
            "\"artifact_kind\":\"AIL-Spec Canonical\",",
            "\"artifact_text\":{},",
            "\"questions\":[],",
            "\"assumptions\":[],",
            "\"provenance\":[\"mock:0\"],",
            "\"checker_handoff\":{{\"must_check\":true,\"expected_profile\":\"Application\",\"expected_features\":[]}}",
            "}}"
        ),
        json_string(requirements)
    );
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&envelope)
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-requirements",
            &package,
            "--prompt",
            "Capture requirements with a wrong artifact kind",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(request_body.contains("wrong artifact kind"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        stderr.contains("AIL-PROMPT-001 prompt envelope artifact_kind must be AIL-Requirements"),
        "{stderr}"
    );
}

#[test]
fn cli_ail_requirements_surfaces_prompt_envelope_questions() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let envelope = concat!(
        "{",
        "\"artifact_kind\":\"AIL-Requirements\",",
        "\"artifact_text\":\"\",",
        "\"questions\":[\"Which user roles may close tickets?\",\"Which trace events must be emitted?\"],",
        "\"assumptions\":[],",
        "\"provenance\":[\"mock:0\"],",
        "\"checker_handoff\":{\"must_check\":true,\"expected_profile\":\"Application\",\"expected_features\":[]}",
        "}"
    );
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(envelope)
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-requirements",
            &package,
            "--prompt",
            "Capture requirements for an underspecified support ticket app",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(request_body.contains("Capture requirements for an underspecified support ticket app"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        stderr.contains("model returned blocking questions"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Which user roles may close tickets?"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Which trace events must be emitted?"),
        "{stderr}"
    );
    assert!(!stderr.contains("failed to connect"), "{stderr}");
}

#[test]
fn cli_ail_draft_rejects_malformed_prompt_envelope() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let envelope = concat!(
        "{",
        "\"artifact_kind\":\"AIL-Spec Canonical\",",
        "\"assumptions\":[],",
        "\"provenance\":[\"mock:0\"],",
        "\"checker_handoff\":{\"must_check\":true,\"expected_profile\":\"Application\",\"expected_features\":[]}",
        "}"
    );
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(envelope)
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-draft",
            &package,
            "--prompt",
            "Draft an AIL support ticket app with a malformed prompt envelope",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(request_body.contains("malformed prompt envelope"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        stderr.contains("AIL-PROMPT-001 prompt envelope must contain artifact_text or questions"),
        "{stderr}"
    );
    assert!(!stderr.contains("AIL000 parse error"), "{stderr}");
}

#[test]
fn cli_ail_draft_rejects_prompt_envelope_without_checker_handoff_must_check() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let envelope = format!(
        concat!(
            "{{",
            "\"artifact_kind\":\"AIL-Spec Canonical\",",
            "\"artifact_text\":{},",
            "\"questions\":[],",
            "\"assumptions\":[],",
            "\"provenance\":[\"mock:0\"],",
            "\"checker_handoff\":{{\"must_check\":false,\"expected_profile\":\"Application\",\"expected_features\":[]}}",
            "}}"
        ),
        json_string(&response_spec)
    );
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&envelope)
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-draft",
            &package,
            "--prompt",
            "Draft an AIL support ticket app without checker handoff",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(request_body.contains("without checker handoff"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        stderr.contains("AIL-PROMPT-001 prompt envelope checker_handoff.must_check must be true"),
        "{stderr}"
    );
}

#[test]
fn cli_ail_draft_rejects_prompt_envelope_profile_mismatch() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let envelope = format!(
        concat!(
            "{{",
            "\"artifact_kind\":\"AIL-Spec Canonical\",",
            "\"artifact_text\":{},",
            "\"questions\":[],",
            "\"assumptions\":[],",
            "\"provenance\":[\"mock:0\"],",
            "\"checker_handoff\":{{\"must_check\":true,\"expected_profile\":\"System\",\"expected_features\":[]}}",
            "}}"
        ),
        json_string(&response_spec)
    );
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&envelope)
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-draft",
            &package,
            "--prompt",
            "Draft an AIL support ticket app with a profile mismatch",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(request_body.contains("profile mismatch"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.is_empty(), "{stdout}");
    assert!(
        stderr.contains(
            "AIL-PROMPT-001 prompt envelope checker_handoff.expected_profile must be Application"
        ),
        "{stderr}"
    );
}

#[test]
fn cli_ail_spec_drafts_and_repairs_from_checked_requirements_file() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let requirements_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-requirements-{}.ail-requirements.md",
        std::process::id()
    ));
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets with Ticket fields id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    fs::write(&requirements_path, requirements).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let rejected_spec = fs::read_to_string(format!(
        "{package}/examples/rejected/missing-reference.ail-spec.md"
    ))
    .unwrap();
    let rejected_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{rejected_spec}\n```"
        ))
    );
    let repaired_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let repaired_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{repaired_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![rejected_body, repaired_body]);

    let output = Command::new(binary)
        .args([
            "ail-spec",
            &package,
            "--prompt",
            "Draft a support ticket app from captured requirements",
            "--requirements-file",
            requirements_path.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    assert!(request_bodies[0].contains("Draft an AIL-Spec candidate"));
    assert!(request_bodies[0].contains("DRAFT REQUIREMENTS:"));
    assert!(
        request_bodies[0].contains("Ticket fields id, title, status, and secret internal notes")
    );
    assert!(request_bodies[1].contains("Repair an AIL-Spec candidate"));
    assert!(request_bodies[1].contains("AIL001 unknown requirement reference"));
    assert!(request_bodies[1].contains("DRAFT REQUIREMENTS:"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Action: Close ticket."), "{stdout}");
    assert!(!stdout.contains("ail-spec diagnostics:"), "{stdout}");
    assert!(!stdout.contains(r#""kind":"AIL-Bytecode""#), "{stdout}");
    let package = load_ail_package_dir(&package).unwrap();
    let document = parse_ail_package_spec_text(&package, &stdout).unwrap();
    let core = elaborate_ail_core(&package, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());

    fs::remove_file(requirements_path).unwrap();
}

#[test]
fn cli_ail_build_accepts_saved_requirements_file_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let requirements_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-build-requirements-{}.ail-requirements.md",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-requirements-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets with Ticket fields id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    fs::write(&requirements_path, requirements).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact from saved requirements",
            "--requirements-file",
            requirements_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let request_bodies = server.join().unwrap();
    assert_eq!(request_bodies.len(), 1);
    assert!(request_bodies[0].contains("Draft an AIL-Spec candidate"));
    assert!(request_bodies[0].contains("DRAFT REQUIREMENTS:"));
    assert!(
        request_bodies[0].contains("Ticket fields id, title, status, and secret internal notes")
    );
    assert!(!request_bodies[0].contains("Draft AIL requirements"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    assert!(bytecode.actions.contains_key("CloseTicket"));
    let requirements_artifact =
        fs::read_to_string(artifact_dir.join("requirements.ail-requirements.md")).unwrap();
    assert_eq!(requirements_artifact, requirements.trim());
    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert_eq!(bytecode_artifact, stdout);

    fs::remove_file(requirements_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_agent_prepares_saved_requirements_before_spec_drafting() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let requirements_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-agent-requirements-file-{}.ail-requirements.md",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-requirements-file-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets with Ticket fields id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    fs::write(&requirements_path, requirements).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact from saved requirements",
            "--requirements-file",
            requirements_path.to_str().unwrap(),
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 1);
    assert!(request_bodies[0].contains("AGENT SPEC CONTEXT:"));
    assert!(
        request_bodies[0].contains("buildrequest.spec coverage checklist=Prepared"),
        "{}",
        request_bodies[0]
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let prepare_index = agent_trace
        .find("action PrepareSpecDraft started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let accept_spec_index = agent_trace
        .find("action AcceptSpecDraft started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let accept_core_index = agent_trace
        .find("action AcceptCoreIR started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(prepare_index < accept_spec_index, "{agent_trace}");
    assert!(accept_spec_index < accept_core_index, "{agent_trace}");
    assert!(accept_core_index < compile_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.requirements"));
    assert!(agent_trace.contains("write buildrequest.spec coverage checklist=Prepared"));
    assert!(agent_trace.contains("trace SpecDraftPrepared"));
    assert!(agent_trace.contains("write buildrequest.spec review report=Accepted"));
    assert!(agent_trace.contains("trace SpecDraftAccepted"));

    fs::remove_file(requirements_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_accepts_saved_spec_file_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let spec_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-build-spec-{}.ail-spec.md",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-spec-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let spec_text = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    fs::write(&spec_path, &spec_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    assert!(bytecode.actions.contains_key("CloseTicket"));

    assert!(
        !artifact_dir
            .join("requirements.ail-requirements.md")
            .exists()
    );
    let spec_artifact = fs::read_to_string(artifact_dir.join("accepted.ail-spec.md")).unwrap();
    assert_eq!(spec_artifact, spec_text.trim());
    let core_artifact = fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert!(core_artifact.contains("package: support-ticket"));
    assert!(core_artifact.contains("node Action CloseTicket"));
    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert_eq!(bytecode_artifact, stdout);

    fs::remove_file(spec_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_records_dependency_report_for_imported_package_graph() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_composed.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-build-imported-package-dependencies-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            &format!("{package}/spec.ail-spec.md"),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Package-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("root-package support-composed 0.1.0"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains(
            "resolved-import Shared path=../support_shared.ail requirement=none name=support-shared version=0.1.0"
        ),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("package-hash=ail-package:"),
        "{dependency_report}"
    );
    let dependency_report_fingerprint =
        fs::read_to_string(artifact_dir.join("dependency-report.fingerprint.txt")).unwrap();
    assert_eq!(
        dependency_report_fingerprint.trim(),
        fnv64_fingerprint(&dependency_report)
    );
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-build.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_build_saved_spec_can_emit_native_linux_x86_64_elf() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let spec_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-build-native-spec-{}.ail-spec.md",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-build-native-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&executable_path);
    let spec_text = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    fs::write(&spec_path, &spec_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
            "--action",
            "AssignTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail-build wrote linux-x86_64-elf executable"));
    assert!(!stdout.contains("\"kind\":\"AIL-Bytecode\""), "{stdout}");

    let native_run = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=Open",
            "ticket.assignee.role=SupportAgent",
        ])
        .output()
        .unwrap();
    assert!(
        native_run.status.success(),
        "native executable failed: {}",
        native_run.status
    );
    assert_eq!(
        String::from_utf8_lossy(&native_run.stdout),
        "ticket.status=Assigned\n"
    );

    fs::remove_file(spec_path).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_build_native_target_is_in_artifact_manifest() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-support-ticket-build-native-manifest-{}",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-build-native-manifest-out-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            &format!("{package}/spec.ail-spec.md"),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--action",
            "AssignTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let target_artifact = fs::read(artifact_dir.join("target.elf")).unwrap();
    let output_artifact = fs::read(&executable_path).unwrap();
    assert_eq!(target_artifact, output_artifact);
    let expected_target_fingerprint = fnv64_fingerprint_bytes(&target_artifact);
    let target_fingerprint =
        fs::read_to_string(artifact_dir.join("target.fingerprint.txt")).unwrap();
    assert_eq!(target_fingerprint.trim(), expected_target_fingerprint);
    assert_eq!(&target_artifact[0..4], b"\x7fELF");

    let native_bytecode_report =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.txt")).unwrap();
    assert!(
        native_bytecode_report.contains("AIL-Build-Native-Bytecode:"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains("target linux-x86_64-elf"),
        "{native_bytecode_report}"
    );
    assert!(
        native_bytecode_report.contains(&format!(
            "machine-bytecode target linux-x86_64-elf target.elf elf64-little-x86_64-executable {expected_target_fingerprint} bytes {}",
            target_artifact.len()
        )),
        "{native_bytecode_report}"
    );
    let native_bytecode_report_fingerprint =
        fs::read_to_string(artifact_dir.join("native-bytecode-report.fingerprint.txt")).unwrap();
    assert_eq!(
        native_bytecode_report_fingerprint.trim(),
        fnv64_fingerprint(&native_bytecode_report)
    );
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains("AIL-Build-Dependency-Report:"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("target linux-x86_64-elf"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("host-language-runtime none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("dynamic-linker none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("shared-libraries none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("library-dependencies none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report.contains("linker-invocation none"),
        "{dependency_report}"
    );
    assert!(
        dependency_report
            .contains("machine-bytecode-dependency target.elf standalone-linux-syscall-elf"),
        "{dependency_report}"
    );
    let dependency_report_fingerprint =
        fs::read_to_string(artifact_dir.join("dependency-report.fingerprint.txt")).unwrap();
    assert_eq!(
        dependency_report_fingerprint.trim(),
        fnv64_fingerprint(&dependency_report)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-build.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "target linux-x86_64-elf target.elf {expected_target_fingerprint}"
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "native-bytecode native-bytecode-report.txt {}",
            fnv64_fingerprint(&native_bytecode_report)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(artifact_dir).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_build_with_pass_writes_native_pass_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let pass_package = fixture("compiler_pass.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-support-ticket-build-native-pass-manifest-{}",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-build-native-pass-out-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            &format!("{package}/spec.ail-spec.md"),
            "--pass",
            &pass_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--action",
            "AssignTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let pass_native = fs::read(artifact_dir.join("pass-InferReadPermissions.elf")).unwrap();
    assert_eq!(&pass_native[0..4], b"\x7fELF");
    let expected_pass_native_fingerprint = fnv64_fingerprint_bytes(&pass_native);
    let pass_run = Command::new(artifact_dir.join("pass-InferReadPermissions.elf"))
        .arg("input graph=checked")
        .arg("package policy=default")
        .output()
        .unwrap();
    assert!(pass_run.status.success(), "native pass executable failed");
    assert!(
        String::from_utf8_lossy(&pass_run.stderr).contains("trace ReadPermissionAdded"),
        "{}",
        String::from_utf8_lossy(&pass_run.stderr)
    );

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-build.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "compiler-pass-target linux-x86_64-elf pass-InferReadPermissions.elf {expected_pass_native_fingerprint}"
        )),
        "{manifest}"
    );
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains(
            "machine-bytecode-dependency pass-InferReadPermissions.elf standalone-linux-syscall-elf"
        ),
        "{dependency_report}"
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );

    fs::remove_dir_all(artifact_dir).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_build_agent_reads_native_pass_fingerprint() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let pass_package = fixture("compiler_pass.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-native-pass-fingerprint-{}",
        std::process::id()
    ));
    let executable_path = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-native-pass-out-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            &format!("{package}/spec.ail-spec.md"),
            "--pass",
            &pass_package,
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--action",
            "AssignTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let pass_native = fs::read(artifact_dir.join("pass-InferReadPermissions.elf")).unwrap();
    let expected_pass_native_fingerprint = fnv64_fingerprint_bytes(&pass_native);
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-build.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "compiler-pass-target linux-x86_64-elf pass-InferReadPermissions.elf {expected_pass_native_fingerprint}"
        )),
        "{manifest}"
    );

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(
        agent_trace.contains("read buildrequest.compiler pass target artifact fingerprint"),
        "{agent_trace}"
    );
    assert!(agent_trace.contains("trace BuildManifestVerified"));

    fs::remove_dir_all(artifact_dir).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn cli_ail_build_agent_verifies_native_target_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-native-artifacts-{}",
        std::process::id()
    ));
    let executable_path =
        std::env::temp_dir().join(format!("ail-ail-build-agent-native-{}", std::process::id()));
    let _ = fs::remove_dir_all(&artifact_dir);
    let _ = fs::remove_file(&executable_path);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            &format!("{package}/spec.ail-spec.md"),
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--action",
            "AssignTicket",
            "--target",
            "linux-x86_64-elf",
            "--out",
            executable_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail-build wrote linux-x86_64-elf executable"));

    let native_run = Command::new(&executable_path)
        .args([
            "ticket.id=T-1",
            "ticket.status=Open",
            "ticket.assignee.role=SupportAgent",
        ])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&native_run.stdout),
        "ticket.status=Assigned\n"
    );

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let target_verify_index = agent_trace
        .find("action VerifyTargetArtifact started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let manifest_verify_index = agent_trace
        .find("action VerifyBuildManifest started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(compile_index < target_verify_index, "{agent_trace}");
    assert!(target_verify_index < manifest_verify_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.target artifact"));
    assert!(agent_trace.contains("read buildrequest.target artifact fingerprint"));
    assert!(
        agent_trace.contains("write buildrequest.target artifact verification report=Verified")
    );
    assert!(agent_trace.contains("trace TargetArtifactVerified"));
    assert!(
        agent_trace[manifest_verify_index..]
            .contains("read buildrequest.target artifact fingerprint"),
        "{agent_trace}"
    );
    assert!(
        agent_trace[manifest_verify_index..]
            .contains("read buildrequest.machine bytecode contract"),
        "{agent_trace}"
    );
    assert!(
        agent_trace[manifest_verify_index..].contains("read buildrequest.native bytecode report"),
        "{agent_trace}"
    );
    assert!(
        agent_trace[manifest_verify_index..]
            .contains("read buildrequest.native bytecode report fingerprint"),
        "{agent_trace}"
    );
    assert!(
        agent_trace[manifest_verify_index..].contains("read buildrequest.dependency report"),
        "{agent_trace}"
    );
    assert!(
        agent_trace[manifest_verify_index..]
            .contains("read buildrequest.dependency report fingerprint"),
        "{agent_trace}"
    );
    let bytecode_verify_index = agent_trace
        .find("action VerifyBytecodeArtifact started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let native_compile_index = agent_trace
        .find("action CompileNativeTarget started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(compile_index < bytecode_verify_index, "{agent_trace}");
    assert!(
        bytecode_verify_index < native_compile_index,
        "{agent_trace}"
    );
    assert!(native_compile_index < target_verify_index, "{agent_trace}");
    assert!(
        agent_trace[native_compile_index..].contains("read buildrequest.bytecode artifact"),
        "{agent_trace}"
    );
    assert!(
        agent_trace[native_compile_index..].contains("read buildrequest.bytecode fingerprint"),
        "{agent_trace}"
    );
    assert!(
        agent_trace[native_compile_index..].contains("read buildrequest.target platform"),
        "{agent_trace}"
    );
    assert!(
        agent_trace[native_compile_index..]
            .contains("write buildrequest.target artifact compilation report=Emitted"),
        "{agent_trace}"
    );
    assert!(
        agent_trace[native_compile_index..].contains("trace NativeTargetCompiled"),
        "{agent_trace}"
    );

    let agent_native = fs::read(artifact_dir.join("agent-CompileApplication.elf")).unwrap();
    assert_eq!(&agent_native[0..4], b"\x7fELF");
    let expected_agent_native_fingerprint = fnv64_fingerprint_bytes(&agent_native);
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-build.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "agent-target linux-x86_64-elf agent-CompileApplication.elf {expected_agent_native_fingerprint}"
        )),
        "{manifest}"
    );
    let dependency_report = fs::read_to_string(artifact_dir.join("dependency-report.txt")).unwrap();
    assert!(
        dependency_report.contains(
            "machine-bytecode-dependency agent-CompileApplication.elf standalone-linux-syscall-elf"
        ),
        "{dependency_report}"
    );
    assert!(
        manifest.contains(&format!(
            "dependencies dependency-report.txt {}",
            fnv64_fingerprint(&dependency_report)
        )),
        "{manifest}"
    );
    let native_agent_run = Command::new(artifact_dir.join("agent-CompileApplication.elf"))
        .args([
            "buildrequest.id=BR-1",
            "buildrequest.status=SpecCaptured",
            "buildrequest.requirements=ok",
            "buildrequest.spec=ok",
        ])
        .output()
        .unwrap();
    assert!(
        native_agent_run.status.success(),
        "native agent CompileApplication failed"
    );
    assert!(
        String::from_utf8_lossy(&native_agent_run.stderr)
            .contains("trace ApplicationBytecodeCompiled"),
        "{}",
        String::from_utf8_lossy(&native_agent_run.stderr)
    );

    fs::remove_dir_all(artifact_dir).unwrap();
    fs::remove_file(executable_path).unwrap();
}

#[test]
fn cli_ail_build_agent_accepts_saved_spec_before_core_lowering() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let spec_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-agent-spec-file-{}.ail-spec.md",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-spec-file-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let spec_text = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    fs::write(&spec_path, &spec_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            spec_path.to_str().unwrap(),
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let accept_spec_index = agent_trace
        .find("action AcceptSpecDraft started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let accept_core_index = agent_trace
        .find("action AcceptCoreIR started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(accept_spec_index < accept_core_index, "{agent_trace}");
    assert!(accept_core_index < compile_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.spec"));
    assert!(agent_trace.contains("write buildrequest.spec review report=Accepted"));
    assert!(agent_trace.contains("write buildrequest.status=SpecCaptured"));
    assert!(agent_trace.contains("trace SpecDraftAccepted"));

    fs::remove_file(spec_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_saved_spec_checks_before_agent_acceptance() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-invalid-spec-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let invalid_spec = format!("{package}/examples/rejected/missing-reference.ail-spec.md");

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--spec-file",
            &invalid_spec,
            "--agent",
            &package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("ail-build diagnostics:"),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("AIL001 unknown requirement reference 'account' in action CloseTicket"),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("AcceptSpecDraft"),
        "saved spec must be checked before the build agent accepts it:\n{stderr}"
    );
    assert!(!artifact_dir.exists());
}

#[test]
fn cli_ail_build_accepts_saved_core_file_artifact() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-build-core-{}.ail-core.txt",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-core-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let package_model = load_ail_package_dir(&package).unwrap();
    let document = parse_ail_package_document(&package_model).unwrap();
    let core = elaborate_ail_core(&package_model, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let core_text = render_ail_core(&core);
    fs::write(&core_path, &core_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--core-file",
            core_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    assert!(bytecode.actions.contains_key("CloseTicket"));

    assert!(
        !artifact_dir
            .join("requirements.ail-requirements.md")
            .exists()
    );
    assert!(!artifact_dir.join("accepted.ail-spec.md").exists());
    let core_artifact = fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert_eq!(core_artifact, format!("{core_text}\n"));
    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert_eq!(bytecode_artifact, stdout);

    fs::remove_file(core_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_runs_toolchain_agent_bytecode() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-agent-build-core-{}.ail-core.txt",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let package_model = load_ail_package_dir(&package).unwrap();
    let document = parse_ail_package_document(&package_model).unwrap();
    let core = elaborate_ail_core(&package_model, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    fs::write(&core_path, render_ail_core(&core)).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--core-file",
            core_path.to_str().unwrap(),
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    assert!(bytecode.actions.contains_key("CloseTicket"));

    let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    assert!(agent_bytecode.contains(r#""package":"ail-toolchain-agent""#));
    assert!(agent_bytecode.contains(r#""action":"CompileApplication""#));
    let agent_fingerprint = fs::read_to_string(artifact_dir.join("agent.fingerprint.txt")).unwrap();
    assert_eq!(agent_fingerprint.trim(), fnv64_fingerprint(&agent_bytecode));
    let parsed_agent = parse_ail_bytecode(&agent_bytecode).unwrap();
    assert_eq!(verify_ail_bytecode(&parsed_agent), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("action CompileApplication started"));
    assert!(agent_trace.contains("rule passed: the BuildRequest to exist"));
    assert!(agent_trace.contains(
        "rule passed: the BuildRequest status to be SpecCaptured or CoreChecked or FlowReviewed"
    ));
    assert!(agent_trace.contains("write buildrequest.bytecode artifact=Emitted"));
    assert!(agent_trace.contains("trace ApplicationBytecodeCompiled"));
    assert!(
        !artifact_dir
            .join("requirements.ail-requirements.md")
            .exists()
    );
    assert!(!artifact_dir.join("accepted.ail-spec.md").exists());

    fs::remove_file(core_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_agent_accepts_saved_core_before_compile() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-agent-accept-core-file-{}.ail-core.txt",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-core-file-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let package_model = load_ail_package_dir(&package).unwrap();
    let document = parse_ail_package_document(&package_model).unwrap();
    let core = elaborate_ail_core(&package_model, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    fs::write(&core_path, render_ail_core(&core)).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--core-file",
            core_path.to_str().unwrap(),
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let accept_core_index = agent_trace
        .find("action AcceptCoreIR started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(accept_core_index < compile_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.core ir"));
    assert!(agent_trace.contains("write buildrequest.core review report=Accepted"));
    assert!(agent_trace.contains("write buildrequest.status=CoreChecked"));
    assert!(agent_trace.contains("trace CoreIrAccepted"));

    fs::remove_file(core_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_agent_accepts_flow_review_before_compile() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-agent-accept-flow-file-{}.ail-core.txt",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-flow-review-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let package_model = load_ail_package_dir(&package).unwrap();
    let document = parse_ail_package_document(&package_model).unwrap();
    let core = elaborate_ail_core(&package_model, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    fs::write(&core_path, render_ail_core(&core)).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--core-file",
            core_path.to_str().unwrap(),
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let accept_core_index = agent_trace
        .find("action AcceptCoreIR started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let accept_flow_index = agent_trace
        .find("action AcceptFlowReview started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(accept_core_index < accept_flow_index, "{agent_trace}");
    assert!(accept_flow_index < compile_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.flow review"));
    assert!(agent_trace.contains("read buildrequest.flow review fingerprint"));
    assert!(agent_trace.contains("write buildrequest.flow review report=Accepted"));
    assert!(agent_trace.contains("write buildrequest.status=FlowReviewed"));
    assert!(agent_trace.contains("trace FlowReviewAccepted"));

    let flow_artifact = fs::read_to_string(artifact_dir.join("review.ail-flow.json")).unwrap();
    let flow_fingerprint =
        fs::read_to_string(artifact_dir.join("review.ail-flow.fingerprint.txt")).unwrap();
    assert_eq!(flow_fingerprint.trim(), fnv64_fingerprint(&flow_artifact));
    let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    assert!(agent_bytecode.contains(r#""action":"AcceptFlowReview""#));

    fs::remove_file(core_path).unwrap();
    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_agent_records_requirements_capture_before_compile() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-capture-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    assert!(bytecode.actions.contains_key("CloseTicket"));

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let capture_index = agent_trace
        .find("action CaptureRequirements started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(capture_index < compile_index, "{agent_trace}");
    assert!(agent_trace.contains("write buildrequest.status=RequirementsCaptured"));
    assert!(agent_trace.contains("trace RequirementsCaptured"));
    assert!(agent_trace.contains("write buildrequest.status=BytecodeReady"));
    assert!(agent_trace.contains("trace ApplicationBytecodeCompiled"));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_agent_threads_capture_checklist_into_requirements_prompt() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-checklist-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    assert!(request_bodies[0].contains("Draft AIL requirements"));
    assert!(
        request_bodies[0].contains("The first line must be exactly AIL-Requirements:"),
        "{}",
        request_bodies[0]
    );
    assert!(
        request_bodies[0].contains("every requirement bullet must start with \\\"- \\\""),
        "{}",
        request_bodies[0]
    );
    assert!(
        request_bodies[0].contains("AGENT REQUIREMENTS CONTEXT:"),
        "{}",
        request_bodies[0]
    );
    assert!(
        request_bodies[0].contains("buildrequest.requirements coverage checklist=Prepared"),
        "{}",
        request_bodies[0]
    );

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    assert!(agent_trace.contains("write buildrequest.requirements coverage checklist=Prepared"));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_agent_threads_spec_checklist_into_spec_prompt() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-spec-checklist-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    assert!(request_bodies[1].contains("Draft an AIL-Spec candidate"));
    assert!(request_bodies[1].contains("DRAFT REQUIREMENTS:"));
    assert!(
        request_bodies[1].contains("AGENT SPEC CONTEXT:"),
        "{}",
        request_bodies[1]
    );
    assert!(
        request_bodies[1].contains("buildrequest.spec coverage checklist=Prepared"),
        "{}",
        request_bodies[1]
    );

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let prepare_index = agent_trace
        .find("action PrepareSpecDraft started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(prepare_index < compile_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.requirements"));
    assert!(agent_trace.contains("write buildrequest.spec coverage checklist=Prepared"));
    assert!(agent_trace.contains("trace SpecDraftPrepared"));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_agent_accepts_spec_draft_before_compile() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-accept-spec-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let accept_index = agent_trace
        .find("action AcceptSpecDraft started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(accept_index < compile_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.spec"));
    assert!(agent_trace.contains("write buildrequest.spec review report=Accepted"));
    assert!(agent_trace.contains("write buildrequest.status=SpecCaptured"));
    assert!(agent_trace.contains("trace SpecDraftAccepted"));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_agent_accepts_checked_core_before_compile() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-accept-core-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let accept_spec_index = agent_trace
        .find("action AcceptSpecDraft started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let accept_core_index = agent_trace
        .find("action AcceptCoreIR started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(accept_spec_index < accept_core_index, "{agent_trace}");
    assert!(accept_core_index < compile_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.core ir"));
    assert!(agent_trace.contains("write buildrequest.core review report=Accepted"));
    assert!(agent_trace.contains("write buildrequest.status=CoreChecked"));
    assert!(agent_trace.contains("trace CoreIrAccepted"));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_agent_compares_prompt_portability_before_compile() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-portability-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--agent",
            &agent_package,
            "--base-model",
            "unsloth/Qwen3.6-35B-A3B-GGUF:UD-Q4_K_XL",
            "--target-model",
            "future/local-ail-toolchain-model",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let capture_index = agent_trace
        .find("action CaptureRequirements started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let compare_index = agent_trace
        .find("action CompareAgentPromptPortability started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(capture_index < compare_index, "{agent_trace}");
    assert!(compare_index < compile_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.base model"));
    assert!(agent_trace.contains("read buildrequest.target model"));
    assert!(agent_trace.contains("read buildrequest.requirements"));
    assert!(agent_trace.contains("write buildrequest.prompt portability report=Compared"));
    assert!(agent_trace.contains("trace AgentPromptPortabilityCompared"));
    assert!(agent_trace.contains("trace ApplicationBytecodeCompiled"));
    assert!(agent_trace.contains("read buildrequest.prompt portability report fingerprint"));

    let portability_report =
        fs::read_to_string(artifact_dir.join("prompt-portability.txt")).unwrap();
    assert!(
        portability_report.contains("AIL-Prompt-Portability-Report:"),
        "{portability_report}"
    );
    assert!(
        portability_report.contains("base-model unsloth/Qwen3.6-35B-A3B-GGUF:UD-Q4_K_XL"),
        "{portability_report}"
    );
    assert!(
        portability_report.contains("target-model future/local-ail-toolchain-model"),
        "{portability_report}"
    );
    assert!(
        portability_report.contains("agent-action CompareAgentPromptPortability"),
        "{portability_report}"
    );
    assert!(
        portability_report.contains("status Compared"),
        "{portability_report}"
    );
    let portability_fingerprint =
        fs::read_to_string(artifact_dir.join("prompt-portability.fingerprint.txt")).unwrap();
    assert_eq!(
        portability_fingerprint.trim(),
        fnv64_fingerprint(&portability_report)
    );
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-build.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "prompt-portability prompt-portability.txt {}",
            fnv64_fingerprint(&portability_report)
        )),
        "{manifest}"
    );

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_prompt_corpus_accepts_checked_outputs() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-prompt-corpus-accepted-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-prompt-corpus",
            "docs/ail/corpus/prompts",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for task in [
        "interview",
        "requirements",
        "spec-draft",
        "repair",
        "core-to-spec",
        "flow-patch",
        "diagnostic-repair",
        "trace-debug",
    ] {
        assert!(
            stdout.contains(&format!("accepted-task {task}")),
            "{stdout}"
        );
    }
    assert!(
        stdout.contains(
            "semantic-task support-ticket-private-notes model-labels base-local,target-local"
        ),
        "{stdout}"
    );

    let checked_core = fs::read_to_string(
        artifact_dir.join("accepted/support-ticket-spec-draft-base.ail-core.txt"),
    )
    .unwrap();
    assert!(
        checked_core.contains("package: support-ticket") && checked_core.contains("version: 0.1.0"),
        "{checked_core}"
    );
    let checked_core_fingerprint = fs::read_to_string(
        artifact_dir.join("accepted/support-ticket-spec-draft-base.ail-core.fingerprint.txt"),
    )
    .unwrap();
    assert_eq!(
        checked_core_fingerprint.trim(),
        fnv64_fingerprint(&checked_core)
    );

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_prompt_corpus_rejects_semantic_drift_outputs() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-prompt-corpus-rejected-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-prompt-corpus",
            "docs/ail/corpus/prompts",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = fs::read_to_string(artifact_dir.join("prompt-corpus-portability.txt")).unwrap();
    for taxonomy in [
        "prompt-envelope",
        "profile-mismatch",
        "hallucinated-capability",
        "missing-trace",
        "semantic-drift",
    ] {
        assert!(
            report.contains(&format!("failure-taxonomy {taxonomy}")),
            "{report}"
        );
    }
    assert!(
        report.contains("rejected-entry semantic-drift-rejected checker-result rejected diagnostic semantic-drift"),
        "{report}"
    );

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_prompt_corpus_writes_portability_report() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-prompt-corpus-report-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-prompt-corpus",
            "docs/ail/corpus/prompts",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = fs::read_to_string(artifact_dir.join("prompt-corpus-portability.txt")).unwrap();
    assert!(
        report.contains("AIL-Prompt-Corpus-Portability-Report:"),
        "{report}"
    );
    assert!(report.contains("base-model base-local"), "{report}");
    assert!(report.contains("target-model target-local"), "{report}");
    assert!(
        report.contains("prompt-fingerprint docs/ail/prompts/spec-draft.system.md"),
        "{report}"
    );
    assert!(
        report.contains("artifact-fingerprint support-ticket-spec-draft-base"),
        "{report}"
    );
    assert!(
        report.contains("checker-result support-ticket-spec-draft-base accepted"),
        "{report}"
    );
    assert!(
        report.contains("checker-result missing-trace-rejected rejected AIL-TRACE-001"),
        "{report}"
    );

    let report_fingerprint =
        fs::read_to_string(artifact_dir.join("prompt-corpus-portability.fingerprint.txt")).unwrap();
    assert_eq!(report_fingerprint.trim(), fnv64_fingerprint(&report));
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-prompt-corpus.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "portability-report prompt-corpus-portability.txt {}",
            fnv64_fingerprint(&report)
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_e2e_corpus_replays_checked_seed_corpus() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-seed-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            "docs/ail/corpus/e2e",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = fs::read_to_string(artifact_dir.join("e2e-corpus-report.txt")).unwrap();
    assert!(report.contains("entry-count 100"), "{report}");
    assert!(
        report.contains("checker-result-count accepted 99"),
        "{report}"
    );
    assert!(
        report.contains("checker-result-count rejected 1"),
        "{report}"
    );
    assert!(
        report.contains("failure-taxonomy-count semantic-drift 1"),
        "{report}"
    );
    assert!(
        report.contains("capture-origin-count deterministic-seed 80"),
        "{report}"
    );
    assert!(
        report.contains("capture-origin-count live-llm 3"),
        "{report}"
    );
    assert!(
        report.contains("capture-origin-count live-codex 17"),
        "{report}"
    );
    assert!(
        report.contains("entry example-2")
            && report.contains("semantic-task stdlib-collections-live-spec-input-2")
            && report.contains("capture-origin live-llm"),
        "{report}"
    );
    assert!(
        report.contains("entry example-32")
            && report.contains("semantic-task support-ticket-live-spec-input-32")
            && report.contains("capture-origin live-llm"),
        "{report}"
    );
    assert!(
        report.contains("entry example-52")
            && report.contains("semantic-task refund-tool-live-spec-input-52")
            && report.contains("capture-origin live-llm"),
        "{report}"
    );
    assert!(
        report.contains("entry example-92")
            && report.contains("semantic-task support-ticket-live-codex-spec-92")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-95")
            && report.contains("semantic-task stateful-counter-live-codex-core-to-spec-95")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-65")
            && report.contains("semantic-task ui-workflow-live-codex-core-to-spec-65")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-29")
            && report.contains("semantic-task c-interop-live-codex-interop-29")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-74")
            && report.contains("semantic-task network-driver-live-codex-diagnostic-repair-74")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-55")
            && report.contains("semantic-task compiler-pass-live-codex-core-to-spec-55")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-75")
            && report.contains("semantic-task secret-access-live-codex-core-to-spec-75")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-80")
            && report.contains("semantic-task repeated-task-live-codex-interview-80")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-0")
            && report.contains("semantic-task stdlib-collections-live-codex-interview-0")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-10")
            && report.contains("semantic-task support-composed-live-codex-interview-10")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-40")
            && report.contains("semantic-task refund-tool-live-codex-interview-40")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-36")
            && report.contains("semantic-task runtime-generic-live-codex-core-to-summary-36")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-86")
            && report.contains("semantic-task c-interop-live-codex-core-to-summary-86")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-96")
            && report.contains("semantic-task stateful-counter-live-codex-core-to-summary-96")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-25")
            && report.contains("semantic-task c-interop-live-codex-core-to-spec-25")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-66")
            && report.contains("semantic-task network-driver-live-codex-core-to-summary-66")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(
        report.contains("entry example-90")
            && report.contains("semantic-task support-ticket-live-codex-interview-90")
            && report.contains("capture-origin live-codex"),
        "{report}"
    );
    assert!(report.contains("profile-count UI 1"), "{report}");
    assert!(
        report.contains("target-count wasm32-unknown-sandbox-wasm 11"),
        "{report}"
    );
    assert!(
        report.contains("response-fingerprint-duplicate-entry-count 0"),
        "{report}"
    );
    assert!(
        report.contains("extracted-artifact-fingerprint-duplicate-entry-count 0"),
        "{report}"
    );
    assert!(
        report.contains("target-report-fingerprint-duplicate-entry-count 0"),
        "{report}"
    );
    assert!(
        report
            .contains("entry-artifact example-99 diagnostics examples/example-99/diagnostics.txt"),
        "{report}"
    );
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-e2e-corpus.txt")).unwrap();
    assert!(manifest.contains("AIL-E2E-Corpus-Manifest:"), "{manifest}");
    assert!(
        manifest.contains("entry example-99 checker-result rejected target vm"),
        "{manifest}"
    );

    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_release_evidence_rejects_deterministic_seed_corpus() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-release-seed-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            "docs/ail/corpus/e2e",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--release-evidence",
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "ail-e2e-corpus --release-evidence requires zero deterministic-seed entries; found 80"
        ),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_release_evidence_requires_live_codex_for_codex_executor() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-release-codex-origin-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-release-codex-origin-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    let mut corpus_text = String::new();
    for index in 0..100 {
        corpus_text.push_str(&e2e_corpus_entry_text(
            index,
            &[("capture-origin", "live-llm")],
        ));
    }
    fs::write(corpus_dir.join("examples.md"), corpus_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--release-evidence",
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "ail-e2e-corpus --release-evidence codex-skill-agent entry example-99 must use capture-origin live-codex"
        ),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_replays_imported_package_specs() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-imported-package-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-imported-package-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    write_e2e_transcript_files(&corpus_dir, 100);
    fs::write(
        corpus_dir.join("responses").join("example-10.json"),
        fs::read_to_string(format!(
            "{}/examples/support_composed.ail/spec.ail-spec.md",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        corpus_dir.join("examples.md"),
        e2e_corpus_text_with_override(
            10,
            &[
                ("semantic-task", "support-composed-import-10"),
                ("package", "examples/support_composed.ail"),
                ("response-file", "responses/example-10.json"),
                ("target", "vm"),
                ("vm-action", "CloseTicket"),
                ("runtime-state", "ticket.id=T-1;ticket.status=Open"),
            ],
        ),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = fs::read_to_string(artifact_dir.join("e2e-corpus-report.txt")).unwrap();
    assert!(
        report.contains("entry example-10")
            && report.contains("semantic-task support-composed-import-10"),
        "{report}"
    );
    assert!(
        report.contains("entry-artifact example-10 vm-trace examples/example-10/vm-trace.txt"),
        "{report}"
    );
    let checked_core =
        fs::read_to_string(artifact_dir.join("examples/example-10/checked.ail-core.txt")).unwrap();
    assert!(
        checked_core.contains("imports: ../support_shared.ail as Shared resolved support-shared"),
        "{checked_core}"
    );
    assert!(checked_core.contains("Thing Shared.User"), "{checked_core}");
    let vm_trace =
        fs::read_to_string(artifact_dir.join("examples/example-10/vm-trace.txt")).unwrap();
    assert!(vm_trace.contains("trace TicketClosed"), "{vm_trace}");

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_replays_ui_profile_specs() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir =
        std::env::temp_dir().join(format!("ail-e2e-corpus-ui-profile-{}", std::process::id()));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-ui-profile-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    write_e2e_transcript_files(&corpus_dir, 100);
    fs::write(
        corpus_dir.join("responses").join("example-65.json"),
        fs::read_to_string(format!(
            "{}/examples/ui_workflow.ail/spec.ail-spec.md",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        corpus_dir.join("examples.md"),
        e2e_corpus_text_with_override(
            65,
            &[
                ("semantic-task", "ui-workflow-profile-65"),
                ("profile", "UI"),
                ("surface-tags", "ui"),
                ("package", "examples/ui_workflow.ail"),
                ("response-file", "responses/example-65.json"),
                ("target", "vm"),
                ("vm-action", ""),
                ("runtime-state", ""),
            ],
        ),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let checked_core =
        fs::read_to_string(artifact_dir.join("examples/example-65/checked.ail-core.txt")).unwrap();
    assert!(checked_core.contains("profile: UI"), "{checked_core}");
    assert!(
        checked_core.contains("node Route TicketDetail"),
        "{checked_core}"
    );
    assert!(
        checked_core.contains("node Form CreateTicketForm"),
        "{checked_core}"
    );
    assert!(
        checked_core.contains("node Dashboard SupportManagerDashboard"),
        "{checked_core}"
    );
    assert!(
        checked_core.contains("node Workflow RefundApproval"),
        "{checked_core}"
    );
    let bytecode =
        fs::read_to_string(artifact_dir.join("examples/example-65/artifact.ailbc.json")).unwrap();
    assert!(bytecode.contains(r#""profile":"UI""#), "{bytecode}");
    assert!(
        bytecode.contains(r#""action":"CreateTicket""#),
        "{bytecode}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_100_distinct_semantic_examples() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-semantic-threshold-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-semantic-threshold-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    let mut corpus_text = String::new();
    for index in 0..100 {
        corpus_text.push_str(&e2e_corpus_entry_text(
            index,
            &[("semantic-task", "support-ticket-duplicate")],
        ));
    }
    fs::write(corpus_dir.join("examples.md"), corpus_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "ail-e2e-corpus requires at least 100 distinct semantic-task entries; found 1"
        ),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_replay_metadata() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir =
        std::env::temp_dir().join(format!("ail-e2e-corpus-metadata-{}", std::process::id()));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-metadata-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    let mut corpus_text = String::new();
    for index in 0..100 {
        corpus_text.push_str(&format!("## End-To-End Example: example-{index}\n\n"));
    }
    fs::write(corpus_dir.join("examples.md"), corpus_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("e2e corpus entry example-0 is missing semantic-task"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_prompt_version_metadata() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-prompt-version-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-prompt-version-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    fs::write(
        corpus_dir.join("examples.md"),
        e2e_corpus_text_with_override(0, &[("prompt-version", "")]),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("e2e corpus entry example-0 is missing prompt-version"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_llm_endpoint_label() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-llm-endpoint-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-llm-endpoint-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    fs::write(
        corpus_dir.join("examples.md"),
        e2e_corpus_text_with_override(0, &[("endpoint-label", "")]),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("e2e corpus llm-http entry example-0 is missing endpoint-label"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_capture_origin() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-capture-origin-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-capture-origin-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    write_e2e_transcript_files(&corpus_dir, 100);
    fs::write(
        corpus_dir.join("examples.md"),
        e2e_corpus_text_with_override(0, &[("capture-origin", "")]),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("e2e corpus entry example-0 is missing capture-origin"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_rejects_unknown_capture_origin() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-unknown-capture-origin-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-unknown-capture-origin-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    write_e2e_transcript_files(&corpus_dir, 100);
    fs::write(
        corpus_dir.join("examples.md"),
        e2e_corpus_text_with_override(0, &[("capture-origin", "simulated")]),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("e2e corpus entry example-0 has unknown capture-origin simulated"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_rejects_offline_executor_endpoint_label() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-offline-endpoint-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-offline-endpoint-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    fs::write(
        corpus_dir.join("examples.md"),
        e2e_corpus_text_with_override(99, &[("endpoint-label", "offline-endpoint")]),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("e2e corpus offline executor entry example-99 must not set endpoint-label"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_llm_and_codex_executor_families() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-executor-coverage-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-executor-coverage-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    let mut corpus_text = String::new();
    for index in 0..100 {
        corpus_text.push_str(&format!(
            "## End-To-End Example: example-{index}\n\
             semantic-task: support-ticket-{index}\n\
             profile: Application\n\
             package: examples/support_ticket.ail\n\
             prompt-file: docs/ail/prompts/spec-draft.system.md\n\
             prompt-version: ail-prompts.v0.2\n\
             prompt-fingerprint: fnv64:spec-draft\n\
             executor-family: llm-http\n\
             executor-label: local-model\n\
             capture-origin: deterministic-seed\n\
             endpoint-label: inteligentia-pro-1\n\
             request-file: requests/example-{index}.json\n\
             response-file: responses/example-{index}.json\n\
             artifact-kind: ail-spec\n\
             checker-result: accepted\n\
             target: linux-x86_64-elf\n\n"
        ));
    }
    fs::write(corpus_dir.join("examples.md"), corpus_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ail-e2e-corpus requires executor-family codex-skill-agent"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_rejected_example_diagnostics() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-rejected-diagnostics-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-rejected-diagnostics-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    let mut corpus_text = String::new();
    for index in 0..99 {
        corpus_text.push_str(&format!(
            "## End-To-End Example: accepted-{index}\n\
             semantic-task: support-ticket-{index}\n\
             profile: Application\n\
             package: examples/support_ticket.ail\n\
             prompt-file: docs/ail/prompts/spec-draft.system.md\n\
             prompt-version: ail-prompts.v0.2\n\
             prompt-fingerprint: fnv64:spec-draft\n\
             executor-family: llm-http\n\
             executor-label: local-model\n\
             capture-origin: deterministic-seed\n\
             endpoint-label: inteligentia-pro-1\n\
             request-file: requests/accepted-{index}.json\n\
             response-file: responses/accepted-{index}.json\n\
             artifact-kind: ail-spec\n\
             checker-result: accepted\n\
             target: linux-x86_64-elf\n\n"
        ));
    }
    corpus_text.push_str(
        "## End-To-End Example: rejected-0\n\
         semantic-task: support-ticket-rejected\n\
         profile: Application\n\
         package: examples/support_ticket.ail\n\
         prompt-file: docs/ail/prompts/spec-draft.system.md\n\
         prompt-version: ail-prompts.v0.2\n\
         prompt-fingerprint: fnv64:spec-draft\n\
         executor-family: codex-skill-agent\n\
         executor-label: codex-ail-spec-writer\n\
         capture-origin: deterministic-seed\n\
         request-file: requests/rejected-0.json\n\
         response-file: responses/rejected-0.json\n\
         artifact-kind: ail-spec\n\
         checker-result: rejected\n\
         target: vm\n\n",
    );
    fs::write(corpus_dir.join("examples.md"), corpus_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("e2e corpus rejected entry rejected-0 is missing expected-diagnostic"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_full_prompt_pack_coverage() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-prompt-coverage-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-prompt-coverage-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    let mut corpus_text = String::new();
    for index in 0..100 {
        let executor_family = if index == 99 {
            "codex-skill-agent"
        } else {
            "llm-http"
        };
        let endpoint_label = if executor_family == "llm-http" {
            "endpoint-label: local-endpoint\n"
        } else {
            ""
        };
        corpus_text.push_str(&format!(
            "## End-To-End Example: example-{index}\n\
             semantic-task: support-ticket-{index}\n\
             profile: Application\n\
             package: examples/support_ticket.ail\n\
             prompt-file: docs/ail/prompts/spec-draft.system.md\n\
             prompt-version: ail-prompts.v0.2\n\
             prompt-fingerprint: fnv64:spec-draft\n\
             executor-family: {executor_family}\n\
             executor-label: local-executor\n\
             capture-origin: deterministic-seed\n\
             {endpoint_label}\
             request-file: requests/example-{index}.json\n\
             response-file: responses/example-{index}.json\n\
             artifact-kind: ail-spec\n\
             checker-result: accepted\n\
             target: linux-x86_64-elf\n\n"
        ));
    }
    fs::write(corpus_dir.join("examples.md"), corpus_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ail-e2e-corpus requires prompt-file docs/ail/prompts/interview.system.md"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_profile_thresholds() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-profile-coverage-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-profile-coverage-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    let mut corpus_text = String::new();
    for index in 0..100 {
        corpus_text.push_str(&e2e_corpus_entry_text(index, &[("profile", "Application")]));
    }
    fs::write(corpus_dir.join("examples.md"), corpus_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ail-e2e-corpus requires at least 15 profile AgentTool examples; found 0"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_surface_thresholds() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-surface-coverage-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-surface-coverage-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    let mut corpus_text = String::new();
    for index in 0..100 {
        corpus_text.push_str(&e2e_corpus_entry_text(index, &[("surface-tags", "core")]));
    }
    fs::write(corpus_dir.join("examples.md"), corpus_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "ail-e2e-corpus requires at least 10 standard-library or package-import examples; found 0"
        ),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_target_thresholds() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-target-coverage-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-target-coverage-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    let mut corpus_text = String::new();
    for index in 0..100 {
        corpus_text.push_str(&e2e_corpus_entry_text(
            index,
            &[("target", "linux-x86_64-elf")],
        ));
    }
    fs::write(corpus_dir.join("examples.md"), corpus_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "ail-e2e-corpus requires at least 5 target wasm32-unknown-sandbox-wasm examples; found 0"
        ),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_llm_endpoint_diversity() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-endpoint-diversity-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-endpoint-diversity-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    let mut corpus_text = String::new();
    for index in 0..100 {
        if index == 99 {
            corpus_text.push_str(&e2e_corpus_entry_text(index, &[]));
        } else {
            corpus_text.push_str(&e2e_corpus_entry_text(
                index,
                &[("endpoint-label", "local-endpoint")],
            ));
        }
    }
    fs::write(corpus_dir.join("examples.md"), corpus_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "ail-e2e-corpus requires one semantic-task family with at least two llm-http executor/endpoint labels"
        ),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_rejects_unknown_target() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-unknown-target-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-unknown-target-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    fs::write(
        corpus_dir.join("examples.md"),
        e2e_corpus_text_with_override(0, &[("target", "unknown-target")]),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("e2e corpus entry example-0 has unknown target unknown-target"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_rejects_unknown_checker_result() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-unknown-checker-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-unknown-checker-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    fs::write(
        corpus_dir.join("examples.md"),
        e2e_corpus_text_with_override(0, &[("checker-result", "maybe")]),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("e2e corpus entry example-0 has unknown checker-result maybe"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_requires_stored_request_and_response_files() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-missing-transcript-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-missing-transcript-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    fs::write(
        corpus_dir.join("examples.md"),
        e2e_corpus_text_with_override(0, &[]),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr
            .contains("e2e corpus entry example-0 request-file requests/example-0.json is missing"),
        "{stderr}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_replays_rejected_prompt_failures() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-rejected-replay-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-rejected-replay-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    write_e2e_transcript_files(&corpus_dir, 100);
    fs::write(
        corpus_dir.join("requests").join("rejected-0.json"),
        r#"{"prompt":"rejected"}"#,
    )
    .unwrap();
    fs::write(
        corpus_dir.join("responses").join("rejected-0.json"),
        fs::read_to_string(format!(
            "{}/examples/support_ticket.ail/examples/rejected/missing-reference.ail-spec.md",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    let mut corpus_text = String::new();
    for index in 0..100 {
        if index == 99 {
            corpus_text.push_str(&e2e_corpus_entry_text(
                index,
                &[
                    ("semantic-task", "support-ticket-rejected"),
                    ("request-file", "requests/rejected-0.json"),
                    ("response-file", "responses/rejected-0.json"),
                    ("checker-result", "rejected"),
                    ("expected-diagnostic", "AIL001"),
                    ("failure-taxonomy", "semantic-drift"),
                ],
            ));
        } else {
            corpus_text.push_str(&e2e_corpus_entry_text(index, &[]));
        }
    }
    fs::write(corpus_dir.join("examples.md"), corpus_text).unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let diagnostics =
        fs::read_to_string(artifact_dir.join("examples/example-99/diagnostics.txt")).unwrap();
    assert!(
        diagnostics.contains("checker-result rejected")
            && diagnostics.contains("expected-diagnostic AIL001")
            && diagnostics
                .contains("AIL001 unknown requirement reference 'account' in action CloseTicket"),
        "{diagnostics}"
    );
    let diagnostics_fingerprint =
        fs::read_to_string(artifact_dir.join("examples/example-99/diagnostics.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        diagnostics_fingerprint.trim(),
        fnv64_fingerprint(&diagnostics)
    );
    let report = fs::read_to_string(artifact_dir.join("e2e-corpus-report.txt")).unwrap();
    assert!(
        report.contains("checker-result-count accepted 99"),
        "{report}"
    );
    assert!(
        report.contains("checker-result-count rejected 1"),
        "{report}"
    );
    assert!(
        report.contains("failure-taxonomy-count semantic-drift 1"),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "entry-artifact example-99 diagnostics examples/example-99/diagnostics.txt {}",
            diagnostics_fingerprint.trim()
        )),
        "{report}"
    );

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_e2e_corpus_writes_report_for_metadata_complete_corpus() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let corpus_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-complete-metadata-{}",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-e2e-corpus-complete-metadata-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&corpus_dir);
    let _ = fs::remove_dir_all(&artifact_dir);
    fs::create_dir_all(&corpus_dir).unwrap();
    write_e2e_transcript_files(&corpus_dir, 100);
    fs::write(
        corpus_dir.join("examples.md"),
        e2e_corpus_text_with_override(0, &[]),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-e2e-corpus",
            corpus_dir.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = fs::read_to_string(artifact_dir.join("e2e-corpus-report.txt")).unwrap();
    assert!(report.contains("AIL-End-To-End-Corpus-Report:"), "{report}");
    assert!(report.contains("entry-count 100"), "{report}");
    assert!(report.contains("profile-count Application 40"), "{report}");
    assert!(report.contains("profile-count AgentTool 15"), "{report}");
    assert!(report.contains("profile-count Compiler 10"), "{report}");
    assert!(report.contains("profile-count System 35"), "{report}");
    assert!(
        report.contains("prompt-count docs/ail/prompts/spec-draft.system.md 10"),
        "{report}"
    );
    assert!(
        report.contains("executor-family-count llm-http 99"),
        "{report}"
    );
    assert!(
        report.contains("executor-family-count codex-skill-agent 1"),
        "{report}"
    );
    assert!(
        report.contains("target-count linux-x86_64-elf 85"),
        "{report}"
    );
    assert!(
        report.contains("target-count wasm32-unknown-sandbox-wasm 5"),
        "{report}"
    );
    assert!(
        report.contains("target-count aarch64-apple-darwin-libsystem-macho 5"),
        "{report}"
    );
    assert!(report.contains("target-count vm 5"), "{report}");
    assert!(
        report.contains("checker-result-count accepted 100"),
        "{report}"
    );
    assert!(
        report.contains("executor-family codex-skill-agent"),
        "{report}"
    );
    let request_transcript =
        fs::read_to_string(corpus_dir.join("requests/example-0.json")).unwrap();
    let request_fingerprint =
        fs::read_to_string(artifact_dir.join("examples/example-0/request.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        request_fingerprint.trim(),
        fnv64_fingerprint(&request_transcript)
    );
    let response_transcript =
        fs::read_to_string(corpus_dir.join("responses/example-0.json")).unwrap();
    let response_fingerprint =
        fs::read_to_string(artifact_dir.join("examples/example-0/response.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        response_fingerprint.trim(),
        fnv64_fingerprint(&response_transcript)
    );
    let extracted_artifact_fingerprint =
        fs::read_to_string(artifact_dir.join("examples/example-0/artifact.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        extracted_artifact_fingerprint.trim(),
        fnv64_fingerprint(response_transcript.trim())
    );
    let checked_core =
        fs::read_to_string(artifact_dir.join("examples/example-0/checked.ail-core.txt")).unwrap();
    assert!(
        checked_core.contains("package: support-ticket")
            && checked_core.contains("node Action CloseTicket"),
        "{checked_core}"
    );
    let checked_core_fingerprint = fs::read_to_string(
        artifact_dir.join("examples/example-0/checked.ail-core.fingerprint.txt"),
    )
    .unwrap();
    assert_eq!(
        checked_core_fingerprint.trim(),
        fnv64_fingerprint(&checked_core)
    );
    let bytecode_artifact =
        fs::read_to_string(artifact_dir.join("examples/example-0/artifact.ailbc.json")).unwrap();
    assert!(
        bytecode_artifact.contains("\"kind\":\"AIL-Bytecode\"")
            && bytecode_artifact.contains("\"action\":\"CloseTicket\""),
        "{bytecode_artifact}"
    );
    let bytecode = parse_ail_bytecode(&bytecode_artifact).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    let bytecode_fingerprint =
        fs::read_to_string(artifact_dir.join("examples/example-0/artifact.ailbc.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        bytecode_fingerprint.trim(),
        fnv64_fingerprint(&bytecode_artifact)
    );
    let vm_trace =
        fs::read_to_string(artifact_dir.join("examples/example-0/vm-trace.txt")).unwrap();
    assert!(
        vm_trace.contains("action CloseTicket started") && vm_trace.contains("trace TicketClosed"),
        "{vm_trace}"
    );
    let vm_trace_fingerprint =
        fs::read_to_string(artifact_dir.join("examples/example-0/vm-trace.fingerprint.txt"))
            .unwrap();
    assert_eq!(vm_trace_fingerprint.trim(), fnv64_fingerprint(&vm_trace));
    let native_artifact =
        fs::read(artifact_dir.join("examples/example-0/target-CloseTicket.elf")).unwrap();
    assert_eq!(&native_artifact[..4], &[0x7f, b'E', b'L', b'F']);
    let native_fingerprint = fs::read_to_string(
        artifact_dir.join("examples/example-0/target-CloseTicket.elf.fingerprint.txt"),
    )
    .unwrap();
    assert_eq!(
        native_fingerprint.trim(),
        fnv64_fingerprint_bytes(&native_artifact)
    );
    let target_report =
        fs::read_to_string(artifact_dir.join("examples/example-0/target-report.txt")).unwrap();
    assert!(
        target_report.contains("AIL-E2E-Target-Report:")
            && target_report.contains("target linux-x86_64-elf")
            && target_report.contains(&format!(
                "machine-bytecode target linux-x86_64-elf target-CloseTicket.elf elf64-little-x86_64-executable {} bytes {}",
                native_fingerprint.trim(),
                native_artifact.len()
            )),
        "{target_report}"
    );
    let target_report_fingerprint =
        fs::read_to_string(artifact_dir.join("examples/example-0/target-report.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        target_report_fingerprint.trim(),
        fnv64_fingerprint(&target_report)
    );
    let wasm_target_report =
        fs::read_to_string(artifact_dir.join("examples/example-85/target-report.txt")).unwrap();
    assert!(
        wasm_target_report.contains("AIL-Wasm-Contract-Report:")
            && wasm_target_report.contains("target wasm32-unknown-sandbox-wasm")
            && wasm_target_report.contains("action CloseTicket")
            && wasm_target_report.contains("executable-wasm-module none"),
        "{wasm_target_report}"
    );
    let wasm_target_report_fingerprint =
        fs::read_to_string(artifact_dir.join("examples/example-85/target-report.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        wasm_target_report_fingerprint.trim(),
        fnv64_fingerprint(&wasm_target_report)
    );
    let darwin_target_report =
        fs::read_to_string(artifact_dir.join("examples/example-90/target-report.txt")).unwrap();
    assert!(
        darwin_target_report.contains("AIL-Darwin-MachO-Contract-Report:")
            && darwin_target_report.contains("target aarch64-apple-darwin-libsystem-macho")
            && darwin_target_report.contains("action CloseTicket")
            && darwin_target_report.contains("executable-macho-module none"),
        "{darwin_target_report}"
    );
    let darwin_target_report_fingerprint =
        fs::read_to_string(artifact_dir.join("examples/example-90/target-report.fingerprint.txt"))
            .unwrap();
    assert_eq!(
        darwin_target_report_fingerprint.trim(),
        fnv64_fingerprint(&darwin_target_report)
    );
    assert!(
        report.contains(&format!(
            "entry-artifact example-0 request examples/example-0/request.fingerprint.txt {}",
            request_fingerprint.trim()
        )),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "entry-artifact example-0 response examples/example-0/response.fingerprint.txt {}",
            response_fingerprint.trim()
        )),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "entry-artifact example-0 extracted-artifact examples/example-0/artifact.fingerprint.txt {}",
            extracted_artifact_fingerprint.trim()
        )),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "entry-artifact example-0 checked-core examples/example-0/checked.ail-core.txt {}",
            checked_core_fingerprint.trim()
        )),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "entry-artifact example-0 bytecode examples/example-0/artifact.ailbc.json {}",
            bytecode_fingerprint.trim()
        )),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "entry-artifact example-0 vm-trace examples/example-0/vm-trace.txt {}",
            vm_trace_fingerprint.trim()
        )),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "entry-artifact example-0 native linux-x86_64-elf examples/example-0/target-CloseTicket.elf {}",
            native_fingerprint.trim()
        )),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "entry-artifact example-0 target-report examples/example-0/target-report.txt {}",
            target_report_fingerprint.trim()
        )),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "entry-artifact example-85 target-report examples/example-85/target-report.txt {}",
            wasm_target_report_fingerprint.trim()
        )),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "entry-artifact example-90 target-report examples/example-90/target-report.txt {}",
            darwin_target_report_fingerprint.trim()
        )),
        "{report}"
    );
    let report_fingerprint =
        fs::read_to_string(artifact_dir.join("e2e-corpus-report.fingerprint.txt")).unwrap();
    assert_eq!(report_fingerprint.trim(), fnv64_fingerprint(&report));
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-e2e-corpus.txt")).unwrap();
    assert!(manifest.contains("AIL-E2E-Corpus-Manifest:"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "report e2e-corpus-report.txt {}",
            report_fingerprint.trim()
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains("entry example-0 checker-result accepted target linux-x86_64-elf"),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "entry-artifact example-0 bytecode examples/example-0/artifact.ailbc.json {}",
            bytecode_fingerprint.trim()
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "entry-artifact example-85 target-report examples/example-85/target-report.txt {}",
            wasm_target_report_fingerprint.trim()
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    let _ = fs::remove_dir_all(corpus_dir);
    let _ = fs::remove_dir_all(artifact_dir);
}

#[test]
fn cli_ail_build_agent_verifies_bytecode_artifact_after_compile() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-verify-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    let expected_fingerprint = fnv64_fingerprint(&stdout);

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let verify_index = agent_trace
        .find("action VerifyBytecodeArtifact started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let manifest_index = agent_trace
        .find("action VerifyBuildManifest started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(compile_index < verify_index, "{agent_trace}");
    assert!(verify_index < manifest_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.bytecode artifact"));
    assert!(agent_trace.contains("read buildrequest.bytecode fingerprint"));
    assert!(agent_trace.contains("write buildrequest.bytecode verification report=Verified"));
    assert!(agent_trace.contains("trace BytecodeArtifactVerified"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest"));
    assert!(agent_trace.contains("read buildrequest.artifact manifest fingerprint"));
    assert!(agent_trace.contains("read buildrequest.source package"));
    assert!(agent_trace.contains("read buildrequest.source package fingerprint"));
    assert!(agent_trace.contains("read buildrequest.requirements fingerprint"));
    assert!(agent_trace.contains("read buildrequest.spec fingerprint"));
    assert!(agent_trace.contains("read buildrequest.flow review fingerprint"));
    assert!(
        agent_trace.contains("write buildrequest.artifact manifest verification report=Verified")
    );
    assert!(agent_trace.contains("trace BuildManifestVerified"));

    let fingerprint = fs::read_to_string(artifact_dir.join("artifact.fingerprint.txt")).unwrap();
    assert_eq!(fingerprint.trim(), expected_fingerprint);
    let core_artifact = fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap();
    let core_fingerprint =
        fs::read_to_string(artifact_dir.join("checked.ail-core.fingerprint.txt")).unwrap();
    assert_eq!(core_fingerprint.trim(), fnv64_fingerprint(&core_artifact));
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-build.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "core checked.ail-core.txt {}",
            fnv64_fingerprint(&core_artifact)
        )),
        "{manifest}"
    );
    let manifest_fingerprint =
        fs::read_to_string(artifact_dir.join("manifest.fingerprint.txt")).unwrap();
    assert_eq!(manifest_fingerprint.trim(), fnv64_fingerprint(&manifest));

    let agent_bytecode = fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    assert!(agent_bytecode.contains(r#""action":"VerifyBytecodeArtifact""#));
    assert!(agent_bytecode.contains(r#""action":"VerifyBuildManifest""#));

    fs::remove_dir_all(artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_agent_capture_failure_happens_before_llm_request() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-preflight-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = "AIL-Requirements:\n- The application manages support tickets.\n";
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let server = observe_optional_chat_request(listener, requirements_body);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--agent",
            &package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let observed_request = server.join().unwrap();
    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(observed_request, None);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr
            .contains("ail-build --agent requires a CaptureRequirements action for prompt builds"),
        "{stderr}"
    );
    assert!(!artifact_dir.exists());
}

#[test]
fn cli_ail_build_agent_compile_failure_happens_before_bytecode_lowering() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let core_path = std::env::temp_dir().join(format!(
        "ail-support-ticket-agent-prelower-core-{}.ail-core.txt",
        std::process::id()
    ));
    let agent_bytecode_path = std::env::temp_dir().join(format!(
        "ail-toolchain-agent-missing-compile-{}.ailbc.json",
        std::process::id()
    ));
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-prelower-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let package_model = load_ail_package_dir(&package).unwrap();
    let document = parse_ail_package_document(&package_model).unwrap();
    let core = elaborate_ail_core(&package_model, &document);
    assert_eq!(check_ail_core(&core), Vec::<String>::new());
    let unsupported_profile_core =
        render_ail_core(&core).replace("profile: Application", "profile: Experimental");
    fs::write(&core_path, unsupported_profile_core).unwrap();
    let agent_package_model = load_ail_package_dir(&agent_package).unwrap();
    let agent_document = parse_ail_package_document(&agent_package_model).unwrap();
    let agent_core = elaborate_ail_core(&agent_package_model, &agent_document);
    assert_eq!(check_ail_core(&agent_core), Vec::<String>::new());
    let mut agent_bytecode = compile_ail_core_bytecode(&agent_core).unwrap();
    assert!(agent_bytecode.actions.contains_key("AcceptCoreIR"));
    assert!(
        agent_bytecode
            .actions
            .remove("CompileApplication")
            .is_some()
    );
    assert_eq!(verify_ail_bytecode(&agent_bytecode), Vec::<String>::new());
    fs::write(
        &agent_bytecode_path,
        format!("{}\n", render_ail_bytecode(&agent_bytecode)),
    )
    .unwrap();

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--core-file",
            core_path.to_str().unwrap(),
            "--agent",
            agent_bytecode_path.to_str().unwrap(),
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ail-build --agent requires a CompileApplication action"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("ail-lower currently supports"),
        "agent should fail before target bytecode lowering:\n{stderr}"
    );
    assert!(!artifact_dir.exists());

    fs::remove_file(core_path).unwrap();
    fs::remove_file(agent_bytecode_path).unwrap();
}

#[test]
fn cli_ail_build_writes_requirements_spec_core_and_bytecode_artifacts() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let artifact_dir =
        std::env::temp_dir().join(format!("ail-ail-build-artifacts-{}", std::process::id()));
    let _ = fs::remove_dir_all(&artifact_dir);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout_bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&stdout_bytecode), Vec::<String>::new());

    let source_manifest = fs::read_to_string(artifact_dir.join("source.ail-package.md")).unwrap();
    assert_eq!(
        source_manifest,
        fs::read_to_string(format!("{package}/ail-package.md")).unwrap()
    );
    let source_spec = fs::read_to_string(artifact_dir.join("source.ail-spec.md")).unwrap();
    assert_eq!(
        source_spec,
        fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap()
    );
    let source_bundle =
        format!("ail-package.md:\n{source_manifest}\nspec.ail-spec.md:\n{source_spec}");
    let source_fingerprint =
        fs::read_to_string(artifact_dir.join("source.fingerprint.txt")).unwrap();
    assert_eq!(source_fingerprint.trim(), fnv64_fingerprint(&source_bundle));
    let requirements_artifact =
        fs::read_to_string(artifact_dir.join("requirements.ail-requirements.md")).unwrap();
    assert_eq!(requirements_artifact, requirements.trim());
    let requirements_fingerprint =
        fs::read_to_string(artifact_dir.join("requirements.fingerprint.txt")).unwrap();
    assert_eq!(
        requirements_fingerprint.trim(),
        fnv64_fingerprint(&requirements_artifact)
    );
    let spec_artifact = fs::read_to_string(artifact_dir.join("accepted.ail-spec.md")).unwrap();
    assert!(spec_artifact.contains("Action: Close ticket."));
    let spec_fingerprint =
        fs::read_to_string(artifact_dir.join("accepted.ail-spec.fingerprint.txt")).unwrap();
    assert_eq!(spec_fingerprint.trim(), fnv64_fingerprint(&spec_artifact));
    let core_artifact = fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert!(core_artifact.contains("package: support-ticket"));
    assert!(core_artifact.contains("node Action CloseTicket"));
    assert!(core_artifact.contains("edge writes Action:CloseTicket -> Field:Ticket.status"));
    let checked_core = parse_ail_core_text(&core_artifact).unwrap();
    let flow_artifact = fs::read_to_string(artifact_dir.join("review.ail-flow.json")).unwrap();
    let expected_flow_artifact = format!("{}\n", render_ail_flow_view(&checked_core));
    assert_eq!(flow_artifact, expected_flow_artifact);
    assert!(flow_artifact.contains(r#""kind":"AIL-Flow""#));
    assert!(flow_artifact.contains(r#""coreLabel":"Action:CloseTicket""#));
    assert!(flow_artifact.contains(&format!(r#""coreHash":"{}""#, ail_core_hash(&checked_core))));
    let flow_fingerprint =
        fs::read_to_string(artifact_dir.join("review.ail-flow.fingerprint.txt")).unwrap();
    assert_eq!(flow_fingerprint.trim(), fnv64_fingerprint(&flow_artifact));
    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert_eq!(bytecode_artifact, stdout);
    let artifact_bytecode = parse_ail_bytecode(&bytecode_artifact).unwrap();
    assert_eq!(artifact_bytecode, stdout_bytecode);
    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-build.txt")).unwrap();
    assert!(
        manifest.contains(&format!(
            "source-package source.ail-package.md source.ail-spec.md {}",
            fnv64_fingerprint(&source_bundle)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "requirements requirements.ail-requirements.md {}",
            fnv64_fingerprint(&requirements_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "spec accepted.ail-spec.md {}",
            fnv64_fingerprint(&spec_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "flow-review review.ail-flow.json {}",
            fnv64_fingerprint(&flow_artifact)
        )),
        "{manifest}"
    );
}

#[test]
fn cli_ail_build_runs_compiler_pass_before_bytecode_lowering() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let pass_package = fixture("compiler_pass.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-pass-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--pass",
            &pass_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout_bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&stdout_bytecode), Vec::<String>::new());
    assert!(stdout_bytecode.actions.contains_key("CloseTicket"));

    let core_artifact = fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap();
    assert!(
        core_artifact.contains("node Permission read Ticket.status"),
        "{core_artifact}"
    );
    assert!(
        core_artifact
            .contains("edge requires Action:MarksOverdueTickets -> Permission:read Ticket.status"),
        "{core_artifact}"
    );
    assert!(
        core_artifact.contains(
            "node Provenance compiler_pass:InferReadPermissions.permission:read Ticket.status"
        ),
        "{core_artifact}"
    );
    let bytecode_artifact = fs::read_to_string(artifact_dir.join("artifact.ailbc.json")).unwrap();
    assert_eq!(bytecode_artifact, stdout);
    let pass_bytecode_artifact = fs::read_to_string(artifact_dir.join("pass.ailbc.json")).unwrap();
    assert!(pass_bytecode_artifact.contains(r#""package":"ail-meta-permissions""#));
    assert!(pass_bytecode_artifact.contains(r#""opcode":"CORE_INFER_READ_PERMISSIONS""#));
    let parsed_pass_bytecode = parse_ail_bytecode(&pass_bytecode_artifact).unwrap();
    assert_eq!(
        verify_ail_bytecode(&parsed_pass_bytecode),
        Vec::<String>::new()
    );
    let pass_trace = fs::read_to_string(artifact_dir.join("pass-trace.txt")).unwrap();
    assert!(pass_trace.contains("compiler pass Infer read permissions started"));
    assert!(pass_trace.contains("core transform infer read permissions"));
    assert!(
        pass_trace
            .contains("compiler pass InferReadPermissions added Permission read Ticket.status")
    );

    fs::remove_dir_all(&artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_agent_accepts_compiler_pass_output_before_core() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("support_ticket.ail");
    let pass_package = fixture("compiler_pass.ail");
    let agent_package = fixture("ail_toolchain_agent.ail");
    let artifact_dir = std::env::temp_dir().join(format!(
        "ail-ail-build-agent-pass-artifacts-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&artifact_dir);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The application manages support tickets.\n",
        "- Ticket fields include id, title, status, and secret internal notes.\n",
        "- The CloseTicket action requires ticket id input and ticket status not to be Closed.\n",
        "- Failure NotFound happens when ticket id is missing and records TicketNotFound.\n",
        "- The action guarantees closed tickets do not appear in the open queue.\n",
        "- The action records trace event TicketClosed.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL support ticket bytecode artifact",
            "--pass",
            &pass_package,
            "--agent",
            &agent_package,
            "--artifact-dir",
            artifact_dir.to_str().unwrap(),
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    let expected_artifact_fingerprint = fnv64_fingerprint(&stdout);

    let pass_trace = fs::read_to_string(artifact_dir.join("pass-trace.txt")).unwrap();
    assert!(pass_trace.contains("core transform infer read permissions"));
    let pass_bytecode_artifact = fs::read_to_string(artifact_dir.join("pass.ailbc.json")).unwrap();
    let expected_pass_fingerprint = fnv64_fingerprint(&pass_bytecode_artifact);
    let pass_fingerprint = fs::read_to_string(artifact_dir.join("pass.fingerprint.txt")).unwrap();
    assert_eq!(pass_fingerprint.trim(), expected_pass_fingerprint);
    let agent_bytecode_artifact =
        fs::read_to_string(artifact_dir.join("agent.ailbc.json")).unwrap();
    let expected_agent_fingerprint = fnv64_fingerprint(&agent_bytecode_artifact);
    let agent_fingerprint = fs::read_to_string(artifact_dir.join("agent.fingerprint.txt")).unwrap();
    assert_eq!(agent_fingerprint.trim(), expected_agent_fingerprint);

    let agent_trace = fs::read_to_string(artifact_dir.join("agent-trace.txt")).unwrap();
    let accept_spec_index = agent_trace
        .find("action AcceptSpecDraft started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let accept_pass_index = agent_trace
        .find("action AcceptCompilerPassOutput started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let accept_core_index = agent_trace
        .find("action AcceptCoreIR started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    let compile_index = agent_trace
        .find("action CompileApplication started")
        .unwrap_or_else(|| panic!("{agent_trace}"));
    assert!(accept_spec_index < accept_pass_index, "{agent_trace}");
    assert!(accept_pass_index < accept_core_index, "{agent_trace}");
    assert!(accept_core_index < compile_index, "{agent_trace}");
    assert!(agent_trace.contains("read buildrequest.compiler pass artifact"));
    assert!(agent_trace.contains("read buildrequest.compiler pass fingerprint"));
    assert!(agent_trace.contains("read buildrequest.compiler pass trace"));
    assert!(agent_trace.contains("write buildrequest.compiler pass review report=Accepted"));
    assert!(agent_trace.contains("write buildrequest.status=PassApplied"));
    assert!(agent_trace.contains("trace CompilerPassOutputAccepted"));

    let manifest = fs::read_to_string(artifact_dir.join("manifest.ail-build.txt")).unwrap();
    assert!(manifest.contains("AIL-Build-Manifest:"), "{manifest}");
    let requirements_artifact =
        fs::read_to_string(artifact_dir.join("requirements.ail-requirements.md")).unwrap();
    assert!(
        manifest.contains(&format!(
            "requirements requirements.ail-requirements.md {}",
            fnv64_fingerprint(&requirements_artifact)
        )),
        "{manifest}"
    );
    let spec_artifact = fs::read_to_string(artifact_dir.join("accepted.ail-spec.md")).unwrap();
    assert!(
        manifest.contains(&format!(
            "spec accepted.ail-spec.md {}",
            fnv64_fingerprint(&spec_artifact)
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "core checked.ail-core.txt {}",
            fnv64_fingerprint(
                &fs::read_to_string(artifact_dir.join("checked.ail-core.txt")).unwrap()
            )
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "bytecode artifact.ailbc.json {expected_artifact_fingerprint}"
        )),
        "{manifest}"
    );
    assert!(
        manifest.contains(&format!(
            "compiler-pass pass.ailbc.json {expected_pass_fingerprint}"
        )),
        "{manifest}"
    );
    assert!(manifest.contains("trace pass-trace.txt"), "{manifest}");
    assert!(
        manifest.contains(&format!(
            "agent agent.ailbc.json {expected_agent_fingerprint}"
        )),
        "{manifest}"
    );
    assert!(manifest.contains("trace agent-trace.txt"), "{manifest}");

    fs::remove_dir_all(&artifact_dir).unwrap();
}

#[test]
fn cli_ail_build_for_agent_tool_profile_prompts_tool_requirements_and_outputs_bytecode() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("refund_tool.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requirements = concat!(
        "AIL-Requirements:\n",
        "- The tool refunds captured payments.\n",
        "- The tool needs input order id, refund amount, reason, and secret payment token.\n",
        "- The tool produces output refund id.\n",
        "- The tool calls PaymentProvider.refund.\n",
        "- The tool requires permission to create refunds and approval for high-value refunds.\n",
        "- Failure ProviderRejected happens when PaymentProvider rejects the refund.\n",
        "- The tool guarantees payment token redaction.\n",
        "- The tool records trace RefundCustomerPaymentRequested.\n"
    );
    let requirements_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(requirements)
    );
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let spec_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_chat_responses(listener, vec![requirements_body, spec_body]);

    let output = Command::new(binary)
        .args([
            "ail-build",
            &package,
            "--prompt",
            "Build an AIL refund tool bytecode artifact",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_bodies = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(request_bodies.len(), 2);
    assert!(request_bodies[0].contains("Use the AgentTool profile"));
    assert!(request_bodies[0].contains("tool capability"));
    assert!(request_bodies[0].contains("tool inputs and outputs"));
    assert!(!request_bodies[0].contains("compiler passes"));
    assert!(!request_bodies[0].contains("system components"));
    assert!(request_bodies[0].contains("permissions"));
    assert!(request_bodies[1].contains("Use this exact AgentTool surface shape"));
    assert!(request_bodies[1].contains("DRAFT REQUIREMENTS:"));
    assert!(request_bodies[1].contains("PaymentProvider.refund"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytecode = parse_ail_bytecode(&stdout).unwrap();
    assert_eq!(bytecode.profile, "AgentTool");
    assert_eq!(verify_ail_bytecode(&bytecode), Vec::<String>::new());
    assert!(bytecode.actions.contains_key("RefundCustomerPayment"));
}

#[test]
fn cli_ail_draft_for_agent_tool_profile_prompts_tool_surface() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("refund_tool.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-draft",
            &package,
            "--prompt",
            "Draft an AIL refund tool",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(request_body.contains("Use the AgentTool profile"));
    assert!(request_body.contains("Tool: <human label>."));
    assert!(request_body.contains("The AI Agent may request"));
    assert!(request_body.contains("The tool needs:"));
    assert!(request_body.contains("The tool produces:"));
    assert!(request_body.contains("The tool can:"));
    assert!(request_body.contains("The tool requires permission:"));
    assert!(request_body.contains("The tool requires approval:"));
    assert!(request_body.contains("The tool records:"));
    assert!(request_body.contains("The tool guarantees:"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail-draft candidate:"));
    assert!(stdout.contains("Tool: Refund customer payment."));
    assert!(stdout.contains("ail-draft diagnostics: none"));
}

#[test]
fn cli_ail_draft_for_compiler_profile_prompts_compiler_pass_surface() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("compiler_pass.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-draft",
            &package,
            "--prompt",
            "Draft an AIL compiler pass for read permissions",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(request_body.contains("Use the Compiler profile"));
    assert!(request_body.contains("Compiler pass: <human label>."));
    assert!(request_body.contains("The pass needs:"));
    assert!(request_body.contains("The pass produces:"));
    assert!(request_body.contains("When the compiler runs <human label>:"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail-draft candidate:"));
    assert!(stdout.contains("Compiler pass: Infer read permissions."));
    assert!(stdout.contains("ail-draft diagnostics: none"));
}

#[test]
fn cli_ail_draft_for_system_profile_prompts_system_surface() {
    let binary = env!("CARGO_BIN_EXE_ail");
    let package = fixture("network_driver.ail");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let response_spec = fs::read_to_string(format!("{package}/spec.ail-spec.md")).unwrap();
    let response_body = format!(
        r#"{{"choices":[{{"message":{{"content":{}}}}}]}}"#,
        json_string(&format!(
            "<think>ignore this</think>\n```ail\n{response_spec}\n```"
        ))
    );
    let server = serve_one_chat_response(listener, response_body);

    let output = Command::new(binary)
        .args([
            "ail-draft",
            &package,
            "--prompt",
            "Draft an AIL system component for a network driver",
            "--llm-endpoint",
            &format!("http://127.0.0.1:{}/v1/chat/completions", addr.port()),
        ])
        .output()
        .unwrap();

    let request_body = server.join().unwrap();
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(request_body.contains("Use the System profile"));
    assert!(request_body.contains("System component: <human label>."));
    assert!(request_body.contains("The component uses:"));
    assert!(request_body.contains("The component owns:"));
    assert!(request_body.contains("The component borrows:"));
    assert!(request_body.contains("The component mutably borrows:"));
    assert!(request_body.contains("The component places:"));
    assert!(request_body.contains("The component lays out:"));
    assert!(request_body.contains("The component allocates:"));
    assert!(request_body.contains("The component guards:"));
    assert!(request_body.contains("The component runs in context:"));
    assert!(request_body.contains("The component sets interrupt priority:"));
    assert!(request_body.contains("The component masks interrupt:"));
    assert!(request_body.contains("The component schedules task:"));
    assert!(request_body.contains("The component sets task priority:"));
    assert!(request_body.contains("The component sets task timing:"));
    assert!(request_body.contains("The component requires capability:"));
    assert!(request_body.contains("The component performs:"));
    assert!(request_body.contains("The component records:"));
    assert!(request_body.contains("The component guarantees:"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ail-draft candidate:"));
    assert!(stdout.contains("System component: Network packet receiver."));
    assert!(stdout.contains("ail-draft diagnostics: none"));
}
