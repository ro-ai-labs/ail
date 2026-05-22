use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::ExitCode;

use eigl::ail::{
    apply_ail_patch, check_ail_core, check_ail_requirements, compile_ail_bytecode_native_elf,
    compile_ail_core_bytecode, compile_ail_core_native_elf, draft_ail_requirements, draft_ail_spec,
    draft_ail_spec_from_requirements, elaborate_ail_core, load_ail_package_dir, parse_ail_bytecode,
    parse_ail_core_text, parse_ail_package_document, parse_ail_package_spec_text,
    parse_ail_patch_text, render_ail_bytecode, render_ail_core, render_ail_flow_view,
    render_ail_runtime_state_lines, render_ail_spec, repair_ail_requirements_from_diagnostics,
    repair_ail_spec_from_diagnostics, run_ail_bytecode_action, run_ail_compiler_pass_on_core,
    run_ail_conformance, verify_ail_bytecode,
};
use eigl::apply_rif_patch;
use eigl::checker::check_document;
use eigl::collections::{collection_path_value_with, collection_record_keys};
use eigl::core_model::{json_string, json_value};
use eigl::eig_ir::{lower_document, run_bytecode_with_operation_outputs};
use eigl::explanations::explain_intent;
use eigl::expression;
use eigl::graph_builder::build_program;
use eigl::interpreter::simulate_with_operation_outputs;
use eigl::llm_round_trip;
use eigl::parse_rif_patch;
use eigl::predicate;
use eigl::render_rif_document;
use eigl::rif_model::{Intent, RifDocument, TriggerDefinition};
use eigl::views::{
    effect_view, failure_view, flow_view, permission_view, security_view, view_model_json,
};
use eigl::{parse_rif_file, parse_rsl_file};

struct CliOptions {
    selected_intent: Option<String>,
    runtime_state: BTreeMap<String, String>,
    request_state: BTreeMap<String, String>,
    state_in: Option<String>,
    state_out: Option<String>,
    data_in: Option<String>,
    data_out: Option<String>,
    operation_outputs: BTreeMap<String, String>,
    listen: Option<String>,
    llm_endpoint: Option<String>,
    artifact_dir: Option<String>,
    patch_path: Option<String>,
    dispatch_method: Option<String>,
    dispatch_path: Option<String>,
    trigger_name: Option<String>,
    ail_action: Option<String>,
    ail_prompt: Option<String>,
    ail_pass_target: Option<String>,
    ail_build_pass: Option<String>,
    ail_build_agent: Option<String>,
    ail_build_target_model: Option<String>,
    ail_requirements_file: Option<String>,
    ail_spec_file: Option<String>,
    ail_core_file: Option<String>,
    ail_compile_target: Option<String>,
    ail_compile_out: Option<String>,
}

struct EndpointExecutionResult {
    run: eigl::eig_ir::BytecodeRunResult,
    success_status: Option<String>,
    response: BTreeMap<String, String>,
    error_status: Option<String>,
    error_response: BTreeMap<String, String>,
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<u8, String> {
    if args.len() < 2 || (matches!(args[0].as_str(), "patch" | "ail-patch") && args.len() < 3) {
        return Err(usage());
    }
    let command = &args[0];
    let path = &args[1];
    let cli_options = parse_cli_options(command, &args[2..])?;
    if command == "ail-vm" {
        return run_ail_vm_command(path, &cli_options);
    }
    if matches!(
        command.as_str(),
        "ail-check"
            | "ail-core"
            | "ail-flow"
            | "ail-lower"
            | "ail-compile"
            | "ail-run"
            | "ail-conformance"
            | "ail-requirements"
            | "ail-spec"
            | "ail-draft"
            | "ail-build"
            | "ail-pass"
            | "ail-patch"
    ) {
        return run_ail_command(command, path, &cli_options);
    }
    let mut document = parse_document_file(path)?;
    if let Some(intent_name) = &cli_options.selected_intent {
        select_intent(&mut document, intent_name)?;
    }

    match command.as_str() {
        "check" => {
            let diagnostics = check_document(&document);
            if diagnostics.is_empty() {
                println!("no diagnostics");
                Ok(0)
            } else {
                for diagnostic in diagnostics {
                    println!("{diagnostic}");
                }
                Ok(1)
            }
        }
        "graph" => {
            println!("{}", build_program(&document).to_json());
            Ok(0)
        }
        "views" => {
            println!("Flow");
            println!("{}", flow_view(&document));
            println!();
            println!("Failures");
            let failures = failure_view(&document);
            println!(
                "{}",
                if failures.is_empty() {
                    "(none)"
                } else {
                    &failures
                }
            );
            println!();
            println!("Permissions");
            println!("{}", permission_view(&document));
            println!();
            println!("Effects");
            println!("{}", effect_view(&document));
            println!();
            println!("Security");
            println!("{}", security_view(&document));
            println!();
            println!("Explanation");
            println!("{}", explain_intent(&document));
            Ok(0)
        }
        "simulate" => {
            let runtime_state = load_execution_state(&cli_options)?;
            validate_runtime_state(&document, &runtime_state)?;
            validate_operation_outputs(&document, &cli_options.operation_outputs)?;
            let result = simulate_with_operation_outputs(
                &document,
                runtime_state,
                cli_options.operation_outputs.clone(),
                BTreeMap::new(),
            );
            println!("{}", result.status);
            if let Some(failure) = &result.failure {
                println!("failure={failure}");
            }
            print_runtime_state(&document, &result.final_state);
            if !result.outputs.is_empty() {
                println!(
                    "outputs={}",
                    result.outputs.keys().cloned().collect::<Vec<_>>().join(",")
                );
            }
            if !result.trace.is_empty() {
                println!("trace={}", result.trace.join(" -> "));
            }
            if result.status == "succeeded" {
                validate_collection_constraints(&document, &result.final_state)?;
                save_runtime_state(&cli_options, &result.final_state)?;
                save_data_store(&document, &cli_options, &result.final_state)?;
            }
            Ok(if result.status == "succeeded" { 0 } else { 1 })
        }
        "lower" => {
            println!("{}", lower_document(&document).to_json());
            Ok(0)
        }
        "run" => {
            let runtime_state = load_execution_state(&cli_options)?;
            validate_runtime_state(&document, &runtime_state)?;
            validate_operation_outputs(&document, &cli_options.operation_outputs)?;
            let bytecode = lower_document(&document);
            let result = run_bytecode_with_operation_outputs(
                &bytecode,
                runtime_state,
                cli_options.operation_outputs.clone(),
                BTreeMap::new(),
            );
            println!("bytecode {}", result.status);
            if let Some(failure) = &result.failure {
                println!("failure={failure}");
            }
            print_runtime_state(&document, &result.final_state);
            if !result.outputs.is_empty() {
                println!(
                    "outputs={}",
                    result.outputs.keys().cloned().collect::<Vec<_>>().join(",")
                );
            }
            if !result.trace.is_empty() {
                println!("trace={}", result.trace.join(" -> "));
            }
            if result.status == "succeeded" {
                validate_collection_constraints(&document, &result.final_state)?;
                save_runtime_state(&cli_options, &result.final_state)?;
                save_data_store(&document, &cli_options, &result.final_state)?;
            }
            Ok(if result.status == "succeeded" { 0 } else { 1 })
        }
        "dispatch" => {
            let Some(method) = cli_options.dispatch_method.as_ref() else {
                return Err("dispatch requires an HTTP method".to_string());
            };
            let Some(request_path) = cli_options.dispatch_path.as_ref() else {
                return Err("dispatch requires a request path".to_string());
            };
            let runtime_state = load_execution_state(&cli_options)?;
            let result = execute_endpoint(
                &document,
                method,
                request_path,
                &runtime_state,
                &cli_options.request_state,
                &cli_options.operation_outputs,
            )?;
            println!("dispatch {}", result.run.status);
            if let Some(failure) = &result.run.failure {
                println!("failure={failure}");
            }
            print_runtime_state(&document, &result.run.final_state);
            if !result.run.outputs.is_empty() {
                println!(
                    "outputs={}",
                    result
                        .run
                        .outputs
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
            for (key, value) in &result.response {
                println!("response.{key}={value}");
            }
            for (key, value) in &result.error_response {
                println!("error.{key}={value}");
            }
            if let Some(status) = &result.success_status
                && result.run.status == "succeeded"
            {
                println!("http_status={status}");
            }
            if let Some(status) = &result.error_status
                && result.run.status != "succeeded"
            {
                println!("http_status={status}");
            }
            if result.run.status == "succeeded" {
                validate_collection_constraints(&document, &result.run.final_state)?;
                save_runtime_state(&cli_options, &result.run.final_state)?;
                save_data_store(&document, &cli_options, &result.run.final_state)?;
            }
            Ok(if result.run.status == "succeeded" {
                0
            } else {
                1
            })
        }
        "emit" | "schedule" | "dequeue" => {
            let Some(trigger_name) = cli_options.trigger_name.as_ref() else {
                return Err(format!("{command} requires a trigger name"));
            };
            let runtime_state = load_execution_state(&cli_options)?;
            let result = execute_trigger(
                &document,
                trigger_name,
                &runtime_state,
                &cli_options.request_state,
                &cli_options.operation_outputs,
            )?;
            println!("{command} {}", result.status);
            if let Some(failure) = &result.failure {
                println!("failure={failure}");
            }
            print_runtime_state(&document, &result.final_state);
            if !result.outputs.is_empty() {
                println!(
                    "outputs={}",
                    result.outputs.keys().cloned().collect::<Vec<_>>().join(",")
                );
            }
            if result.status == "succeeded" {
                validate_collection_constraints(&document, &result.final_state)?;
                save_runtime_state(&cli_options, &result.final_state)?;
                save_data_store(&document, &cli_options, &result.final_state)?;
            }
            Ok(if result.status == "succeeded" { 0 } else { 1 })
        }
        "serve" => {
            let listen = cli_options.listen.as_deref().unwrap_or("127.0.0.1:3000");
            let listener = TcpListener::bind(listen)
                .map_err(|error| format!("failed to bind '{listen}': {error}"))?;
            let actual_listen = listener
                .local_addr()
                .map(|addr| addr.to_string())
                .unwrap_or_else(|_| listen.to_string());
            eprintln!("listening on {actual_listen}");
            for stream in listener.incoming() {
                let mut stream = match stream {
                    Ok(stream) => stream,
                    Err(error) => {
                        eprintln!("accept error: {error}");
                        continue;
                    }
                };
                if let Err(error) = handle_http_connection(&document, &cli_options, &mut stream) {
                    let _ = write_http_response(
                        &mut stream,
                        "500 Internal Server Error",
                        "text/plain; charset=utf-8",
                        &format!("{error}\n"),
                    );
                }
            }
            Ok(0)
        }
        "normalize" => {
            println!("{}", render_rif_document(&document));
            Ok(0)
        }
        "patch" => {
            let Some(patch_path) = cli_options.patch_path.as_ref() else {
                return Err("patch requires a patch file".to_string());
            };
            let patch_text = fs::read_to_string(patch_path)
                .map_err(|error| format!("failed to read {patch_path}: {error}"))?;
            let patch = parse_rif_patch(&patch_text)?;
            let patched = apply_rif_patch(&document, &patch)?;
            println!("{}", render_rif_document(&patched));
            Ok(0)
        }
        "llm-roundtrip" => {
            let endpoint = cli_options
                .llm_endpoint
                .as_deref()
                .unwrap_or("http://inteligentia-pro-1:8080/v1/chat/completions");
            let (generated_rsl, _canonical) = llm_round_trip(&document, endpoint)?;
            println!("{generated_rsl}");
            println!();
            println!("roundtrip=ok");
            Ok(0)
        }
        "view-model" => {
            println!("{}", view_model_json(&document));
            Ok(0)
        }
        _ => Err(format!("unknown command '{command}'")),
    }
}

fn usage() -> String {
    "usage: eigl <check|graph|views|simulate|lower|run|dispatch|emit|schedule|dequeue|serve|normalize|patch|llm-roundtrip|view-model|ail-check|ail-core|ail-flow|ail-lower|ail-compile|ail-run|ail-vm|ail-conformance|ail-requirements|ail-spec|ail-draft|ail-build|ail-pass|ail-patch> <path> [patch|target-package] [--intent name] [--action name] [--prompt text] [--requirements-file path] [--spec-file path] [--core-file path] [--pass path] [--agent path] [--target target] [--target-model name] [--out path] [--artifact-dir path] [--state-in path] [--state-out path] [--data-in path] [--data-out path] [--operation-output name=value] [--listen addr] [--llm-endpoint url] [method path|trigger] [key=value ...]\nail-pass usage: eigl ail-pass <compiler-pass-package-or-bytecode> <target-package> --action <PassName> [--agent <agent-package-or-bytecode>] [--target linux-x86_64-elf --artifact-dir <dir>] OR eigl ail-pass <compiler-pass-package-or-bytecode> --core-file <checked-core> --action <PassName> [--agent <agent-package-or-bytecode>] [--target linux-x86_64-elf --artifact-dir <dir>]"
        .to_string()
}

struct AilBuildArtifactSet<'a> {
    requirements: Option<&'a str>,
    spec_text: Option<&'a str>,
    core_text: &'a str,
    bytecode_text: &'a str,
    bytecode_fingerprint: &'a str,
    target_name: Option<&'a str>,
    target_executable: Option<&'a [u8]>,
    pass_bytecode_text: Option<&'a str>,
    pass_bytecode_fingerprint: Option<&'a str>,
    pass_trace: Option<&'a [String]>,
    agent_bytecode_text: Option<&'a str>,
    agent_trace: Option<&'a [String]>,
    agent_native_executables: &'a [AilNativeArtifact],
}

struct AilNativeArtifact {
    target_name: String,
    file_name: String,
    bytes: Vec<u8>,
}

struct AilPassArtifactSet<'a> {
    pass_bytecode_text: &'a str,
    input_core_text: &'a str,
    output_core_text: &'a str,
    trace: &'a [String],
    pass_native_executables: &'a [AilNativeArtifact],
    agent_bytecode_text: Option<&'a str>,
    agent_trace: Option<&'a [String]>,
    agent_native_executables: &'a [AilNativeArtifact],
}

struct AilBuildAgentStart {
    state: BTreeMap<String, String>,
    trace: Vec<String>,
}

struct AilBuildAgentRun {
    bytecode: eigl::ail::AilBytecodeProgram,
    bytecode_text: String,
    state: BTreeMap<String, String>,
    trace: Vec<String>,
}

struct AilBuildPassAcceptance<'a> {
    requirements_artifact: Option<&'a str>,
    spec_text: Option<&'a str>,
    core_text: &'a str,
    pass_bytecode_text: &'a str,
    pass_bytecode_fingerprint: &'a str,
    pass_trace: &'a [String],
}

fn render_ail_build_manifest(artifacts: &AilBuildArtifactSet<'_>) -> String {
    let mut lines = vec!["AIL-Build-Manifest:".to_string()];
    if artifacts.requirements.is_some() {
        lines.push("artifact requirements.ail-requirements.md".to_string());
    }
    if artifacts.spec_text.is_some() {
        lines.push("artifact accepted.ail-spec.md".to_string());
    }
    lines.push("artifact checked.ail-core.txt".to_string());
    lines.push(format!(
        "bytecode artifact.ailbc.json {}",
        artifacts.bytecode_fingerprint
    ));
    if let (Some(target_name), Some(target_executable)) =
        (artifacts.target_name, artifacts.target_executable)
    {
        lines.push(format!(
            "target {target_name} target.elf {}",
            ail_artifact_fingerprint_bytes(target_executable)
        ));
    }
    if let Some(pass_bytecode_text) = artifacts.pass_bytecode_text {
        let pass_bytecode_fingerprint = artifacts
            .pass_bytecode_fingerprint
            .map(str::to_string)
            .unwrap_or_else(|| ail_artifact_fingerprint(pass_bytecode_text));
        lines.push(format!(
            "compiler-pass pass.ailbc.json {pass_bytecode_fingerprint}"
        ));
    }
    if artifacts.pass_trace.is_some() {
        lines.push("trace pass-trace.txt".to_string());
    }
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        lines.push(format!(
            "agent agent.ailbc.json {}",
            ail_artifact_fingerprint(agent_bytecode_text)
        ));
    }
    if artifacts.agent_trace.is_some() {
        lines.push("trace agent-trace.txt".to_string());
    }
    for native_agent in artifacts.agent_native_executables {
        lines.push(format!(
            "agent-target {} {} {}",
            native_agent.target_name,
            native_agent.file_name,
            ail_artifact_fingerprint_bytes(&native_agent.bytes)
        ));
    }
    format!("{}\n", lines.join("\n"))
}

fn write_ail_build_artifacts(
    artifact_dir: &str,
    artifacts: AilBuildArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-build artifact dir {artifact_dir}: {error}")
    })?;
    if let Some(requirements) = artifacts.requirements {
        fs::write(root.join("requirements.ail-requirements.md"), requirements)
            .map_err(|error| format!("failed to write ail-build requirements artifact: {error}"))?;
    }
    if let Some(spec_text) = artifacts.spec_text {
        fs::write(root.join("accepted.ail-spec.md"), spec_text)
            .map_err(|error| format!("failed to write ail-build spec artifact: {error}"))?;
    }
    fs::write(root.join("checked.ail-core.txt"), artifacts.core_text)
        .map_err(|error| format!("failed to write ail-build core artifact: {error}"))?;
    fs::write(root.join("artifact.ailbc.json"), artifacts.bytecode_text)
        .map_err(|error| format!("failed to write ail-build bytecode artifact: {error}"))?;
    fs::write(
        root.join("artifact.fingerprint.txt"),
        format!("{}\n", artifacts.bytecode_fingerprint),
    )
    .map_err(|error| format!("failed to write ail-build bytecode fingerprint artifact: {error}"))?;
    if let Some(target_executable) = artifacts.target_executable {
        let target_path = root.join("target.elf");
        fs::write(&target_path, target_executable)
            .map_err(|error| format!("failed to write ail-build target artifact: {error}"))?;
        set_native_executable_permissions(&target_path.to_string_lossy())?;
        fs::write(
            root.join("target.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint_bytes(target_executable)),
        )
        .map_err(|error| {
            format!("failed to write ail-build target fingerprint artifact: {error}")
        })?;
    }
    if let Some(pass_bytecode_text) = artifacts.pass_bytecode_text {
        fs::write(root.join("pass.ailbc.json"), pass_bytecode_text).map_err(|error| {
            format!("failed to write ail-build pass bytecode artifact: {error}")
        })?;
    }
    if let Some(pass_bytecode_fingerprint) = artifacts.pass_bytecode_fingerprint {
        fs::write(
            root.join("pass.fingerprint.txt"),
            format!("{pass_bytecode_fingerprint}\n"),
        )
        .map_err(|error| {
            format!("failed to write ail-build pass bytecode fingerprint artifact: {error}")
        })?;
    }
    if let Some(pass_trace) = artifacts.pass_trace {
        fs::write(
            root.join("pass-trace.txt"),
            format!("{}\n", pass_trace.join("\n")),
        )
        .map_err(|error| format!("failed to write ail-build pass trace artifact: {error}"))?;
    }
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        fs::write(root.join("agent.ailbc.json"), agent_bytecode_text).map_err(|error| {
            format!("failed to write ail-build agent bytecode artifact: {error}")
        })?;
        fs::write(
            root.join("agent.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(agent_bytecode_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-build agent bytecode fingerprint artifact: {error}")
        })?;
    }
    if let Some(agent_trace) = artifacts.agent_trace {
        fs::write(
            root.join("agent-trace.txt"),
            format!("{}\n", agent_trace.join("\n")),
        )
        .map_err(|error| format!("failed to write ail-build agent trace artifact: {error}"))?;
    }
    for native_agent in artifacts.agent_native_executables {
        let artifact_path = root.join(&native_agent.file_name);
        fs::write(&artifact_path, &native_agent.bytes).map_err(|error| {
            format!(
                "failed to write ail-build native agent artifact {}: {error}",
                native_agent.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
    }
    let manifest_text = render_ail_build_manifest(&artifacts);
    fs::write(root.join("manifest.ail-build.txt"), &manifest_text)
        .map_err(|error| format!("failed to write ail-build manifest artifact: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| format!("failed to write ail-build manifest fingerprint artifact: {error}"))?;
    Ok(())
}

fn render_ail_lower_manifest(bytecode_fingerprint: &str) -> String {
    format!(
        "AIL-Lower-Manifest:\nartifact checked.ail-core.txt\nbytecode artifact.ailbc.json {bytecode_fingerprint}\n"
    )
}

fn write_ail_lower_artifacts(
    artifact_dir: &str,
    core_text: &str,
    bytecode_text: &str,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-lower artifact dir {artifact_dir}: {error}")
    })?;
    fs::write(root.join("checked.ail-core.txt"), core_text)
        .map_err(|error| format!("failed to write ail-lower core artifact: {error}"))?;
    fs::write(root.join("artifact.ailbc.json"), bytecode_text)
        .map_err(|error| format!("failed to write ail-lower bytecode artifact: {error}"))?;
    let bytecode_fingerprint = ail_artifact_fingerprint(bytecode_text);
    fs::write(
        root.join("artifact.fingerprint.txt"),
        format!("{bytecode_fingerprint}\n"),
    )
    .map_err(|error| format!("failed to write ail-lower bytecode fingerprint artifact: {error}"))?;
    let manifest_text = render_ail_lower_manifest(&bytecode_fingerprint);
    fs::write(root.join("manifest.ail-lower.txt"), &manifest_text)
        .map_err(|error| format!("failed to write ail-lower manifest artifact: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| format!("failed to write ail-lower manifest fingerprint artifact: {error}"))?;
    Ok(())
}

fn write_native_executable(path: &str, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes)
        .map_err(|error| format!("failed to write native executable {path}: {error}"))?;
    set_native_executable_permissions(path)
}

#[cfg(unix)]
fn set_native_executable_permissions(path: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| format!("failed to stat native executable {path}: {error}"))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("failed to mark native executable {path} executable: {error}"))
}

#[cfg(not(unix))]
fn set_native_executable_permissions(_path: &str) -> Result<(), String> {
    Ok(())
}

fn ail_artifact_fingerprint(text: &str) -> String {
    ail_artifact_fingerprint_bytes(text.as_bytes())
}

fn ail_artifact_fingerprint_bytes(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv64:{hash:016x}")
}

fn render_ail_pass_manifest(artifacts: &AilPassArtifactSet<'_>) -> String {
    let pass_fingerprint = ail_artifact_fingerprint(artifacts.pass_bytecode_text);
    let mut lines = vec![
        "AIL-Pass-Manifest:".to_string(),
        format!("compiler-pass pass.ailbc.json {pass_fingerprint}"),
    ];
    for native_pass in artifacts.pass_native_executables {
        lines.push(format!(
            "compiler-pass-target {} {} {}",
            native_pass.target_name,
            native_pass.file_name,
            ail_artifact_fingerprint_bytes(&native_pass.bytes)
        ));
    }
    lines.push("core-input input.ail-core.txt".to_string());
    lines.push("core-output output.ail-core.txt".to_string());
    lines.push("trace trace.txt".to_string());
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        lines.push(format!(
            "agent agent.ailbc.json {}",
            ail_artifact_fingerprint(agent_bytecode_text)
        ));
    }
    if artifacts.agent_trace.is_some() {
        lines.push("trace agent-trace.txt".to_string());
    }
    for native_agent in artifacts.agent_native_executables {
        lines.push(format!(
            "agent-target {} {} {}",
            native_agent.target_name,
            native_agent.file_name,
            ail_artifact_fingerprint_bytes(&native_agent.bytes)
        ));
    }
    format!("{}\n", lines.join("\n"))
}

fn write_ail_pass_artifacts(
    artifact_dir: &str,
    artifacts: AilPassArtifactSet<'_>,
) -> Result<(), String> {
    let root = std::path::Path::new(artifact_dir);
    fs::create_dir_all(root).map_err(|error| {
        format!("failed to create ail-pass artifact dir {artifact_dir}: {error}")
    })?;
    fs::write(root.join("pass.ailbc.json"), artifacts.pass_bytecode_text)
        .map_err(|error| format!("failed to write ail-pass bytecode artifact: {error}"))?;
    fs::write(
        root.join("pass.fingerprint.txt"),
        format!(
            "{}\n",
            ail_artifact_fingerprint(artifacts.pass_bytecode_text)
        ),
    )
    .map_err(|error| format!("failed to write ail-pass bytecode fingerprint artifact: {error}"))?;
    fs::write(root.join("input.ail-core.txt"), artifacts.input_core_text)
        .map_err(|error| format!("failed to write ail-pass input core artifact: {error}"))?;
    fs::write(root.join("output.ail-core.txt"), artifacts.output_core_text)
        .map_err(|error| format!("failed to write ail-pass output core artifact: {error}"))?;
    fs::write(
        root.join("trace.txt"),
        format!("{}\n", artifacts.trace.join("\n")),
    )
    .map_err(|error| format!("failed to write ail-pass trace artifact: {error}"))?;
    for native_pass in artifacts.pass_native_executables {
        let artifact_path = root.join(&native_pass.file_name);
        fs::write(&artifact_path, &native_pass.bytes).map_err(|error| {
            format!(
                "failed to write ail-pass native compiler-pass artifact {}: {error}",
                native_pass.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
    }
    if let Some(agent_bytecode_text) = artifacts.agent_bytecode_text {
        fs::write(root.join("agent.ailbc.json"), agent_bytecode_text).map_err(|error| {
            format!("failed to write ail-pass agent bytecode artifact: {error}")
        })?;
        fs::write(
            root.join("agent.fingerprint.txt"),
            format!("{}\n", ail_artifact_fingerprint(agent_bytecode_text)),
        )
        .map_err(|error| {
            format!("failed to write ail-pass agent bytecode fingerprint artifact: {error}")
        })?;
    }
    if let Some(agent_trace) = artifacts.agent_trace {
        fs::write(
            root.join("agent-trace.txt"),
            format!("{}\n", agent_trace.join("\n")),
        )
        .map_err(|error| format!("failed to write ail-pass agent trace artifact: {error}"))?;
    }
    for native_agent in artifacts.agent_native_executables {
        let artifact_path = root.join(&native_agent.file_name);
        fs::write(&artifact_path, &native_agent.bytes).map_err(|error| {
            format!(
                "failed to write ail-pass native agent artifact {}: {error}",
                native_agent.file_name
            )
        })?;
        set_native_executable_permissions(&artifact_path.to_string_lossy())?;
    }
    let manifest_text = render_ail_pass_manifest(&artifacts);
    fs::write(root.join("manifest.ail-pass.txt"), &manifest_text)
        .map_err(|error| format!("failed to write ail-pass manifest artifact: {error}"))?;
    fs::write(
        root.join("manifest.fingerprint.txt"),
        format!("{}\n", ail_artifact_fingerprint(&manifest_text)),
    )
    .map_err(|error| format!("failed to write ail-pass manifest fingerprint artifact: {error}"))?;
    Ok(())
}

fn run_ail_vm_command(path: &str, cli_options: &CliOptions) -> Result<u8, String> {
    let action = cli_options
        .ail_action
        .as_deref()
        .ok_or_else(|| "ail-vm requires --action <name>".to_string())?;
    let bytecode_text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
    let bytecode = parse_ail_bytecode(&bytecode_text)?;
    let diagnostics = verify_ail_bytecode(&bytecode);
    if !diagnostics.is_empty() {
        println!("ail-vm diagnostics:");
        for diagnostic in diagnostics {
            println!("{diagnostic}");
        }
        return Ok(1);
    }
    let result = run_ail_bytecode_action(&bytecode, action, cli_options.runtime_state.clone())?;
    println!("ail-vm {}", result.status);
    if let Some(failure) = &result.failure {
        println!("failure={failure}");
    }
    for (key, value) in &result.final_state {
        println!("{key}={value}");
    }
    if !result.trace.is_empty() {
        println!("trace={}", result.trace.join(" -> "));
    }
    Ok(if result.status == "succeeded" { 0 } else { 1 })
}

fn run_ail_pass_command(pass_path: &str, cli_options: &CliOptions) -> Result<u8, String> {
    let action = cli_options
        .ail_action
        .as_deref()
        .ok_or_else(|| "ail-pass requires --action <name>".to_string())?;

    let (pass_bytecode, pass_bytecode_text) =
        load_ail_bytecode_or_compile_package(pass_path, "ail-pass compiler pass")?;
    let bytecode_diagnostics = verify_ail_bytecode(&pass_bytecode);
    if !bytecode_diagnostics.is_empty() {
        println!("ail-pass diagnostics:");
        for diagnostic in bytecode_diagnostics {
            println!("{diagnostic}");
        }
        return Ok(1);
    }

    let target_core = load_ail_pass_target_core(cli_options)?;
    let target_diagnostics = check_ail_core(&target_core);
    if !target_diagnostics.is_empty() {
        for diagnostic in target_diagnostics {
            println!("{diagnostic}");
        }
        return Ok(1);
    }

    let result = run_ail_compiler_pass_on_core(&pass_bytecode, action, &target_core)?;
    let result_diagnostics = check_ail_core(&result.core);
    if !result_diagnostics.is_empty() {
        println!("ail-pass diagnostics:");
        for diagnostic in result_diagnostics {
            println!("{diagnostic}");
        }
        return Ok(1);
    }
    let input_core_text = format!("{}\n", render_ail_core(&target_core));
    let output_core_text = format!("{}\n", render_ail_core(&result.core));
    let pass_native_artifacts = if let Some(target) = &cli_options.ail_compile_target {
        compile_ail_pass_native_artifacts(&pass_bytecode, target)?
    } else {
        Vec::new()
    };
    let mut agent_run = if let Some(agent_path) = &cli_options.ail_build_agent {
        Some(run_ail_pass_agent_accept_pass_output(
            agent_path,
            &output_core_text,
            &pass_bytecode_text,
            &ail_artifact_fingerprint(&pass_bytecode_text),
            &result.run.trace,
        )?)
    } else {
        None
    };
    let agent_native_artifacts = if let (Some(target), Some(agent_run)) =
        (&cli_options.ail_compile_target, agent_run.as_ref())
    {
        compile_ail_build_agent_native_artifacts(&agent_run.bytecode, target)?
    } else {
        Vec::new()
    };
    if let (Some(agent_run), Some(_artifact_dir)) =
        (agent_run.as_mut(), cli_options.artifact_dir.as_ref())
    {
        let manifest_text = render_ail_pass_manifest(&AilPassArtifactSet {
            pass_bytecode_text: &pass_bytecode_text,
            input_core_text: &input_core_text,
            output_core_text: &output_core_text,
            trace: &result.run.trace,
            pass_native_executables: pass_native_artifacts.as_slice(),
            agent_bytecode_text: Some(agent_run.bytecode_text.as_str()),
            agent_trace: Some(agent_run.trace.as_slice()),
            agent_native_executables: agent_native_artifacts.as_slice(),
        });
        let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
        run_ail_pass_agent_verify_manifest(agent_run, &manifest_text, &manifest_fingerprint)?;
    }
    if let Some(artifact_dir) = &cli_options.artifact_dir {
        write_ail_pass_artifacts(
            artifact_dir,
            AilPassArtifactSet {
                pass_bytecode_text: &pass_bytecode_text,
                input_core_text: &input_core_text,
                output_core_text: &output_core_text,
                trace: &result.run.trace,
                pass_native_executables: pass_native_artifacts.as_slice(),
                agent_bytecode_text: agent_run.as_ref().map(|run| run.bytecode_text.as_str()),
                agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
                agent_native_executables: agent_native_artifacts.as_slice(),
            },
        )?;
    }
    print!("{output_core_text}");
    Ok(0)
}

fn run_ail_pass_agent_accept_pass_output(
    agent_path: &str,
    output_core_text: &str,
    pass_bytecode_text: &str,
    pass_bytecode_fingerprint: &str,
    pass_trace: &[String],
) -> Result<AilBuildAgentRun, String> {
    let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
    if !agent_bytecode
        .actions
        .contains_key("AcceptCompilerPassOutput")
    {
        return Err("ail-pass --agent requires an AcceptCompilerPassOutput action".to_string());
    }
    let state = BTreeMap::from([
        ("buildrequest.id".to_string(), "ail-pass".to_string()),
        (
            "buildrequest.developer prompt".to_string(),
            "skipped".to_string(),
        ),
        (
            "buildrequest.requirements".to_string(),
            "skipped".to_string(),
        ),
        ("buildrequest.spec".to_string(), "skipped".to_string()),
        (
            "buildrequest.core ir".to_string(),
            output_core_text.to_string(),
        ),
        (
            "buildrequest.compiler pass artifact".to_string(),
            format!(
                "Verified AIL compiler pass bytecode ({} bytes)",
                pass_bytecode_text.len()
            ),
        ),
        (
            "buildrequest.compiler pass fingerprint".to_string(),
            pass_bytecode_fingerprint.to_string(),
        ),
        (
            "buildrequest.compiler pass trace".to_string(),
            pass_trace.join("\n"),
        ),
        ("buildrequest.status".to_string(), "CoreLoaded".to_string()),
    ]);
    let run = run_ail_bytecode_action(&agent_bytecode, "AcceptCompilerPassOutput", state)?;
    if run.status != "succeeded" {
        let mut message = "ail-pass agent AcceptCompilerPassOutput failed".to_string();
        if let Some(failure) = run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !run.trace.is_empty() {
            message.push_str(&format!("\n{}", run.trace.join("\n")));
        }
        return Err(message);
    }
    Ok(AilBuildAgentRun {
        bytecode: agent_bytecode,
        bytecode_text: agent_bytecode_text,
        state: run.final_state,
        trace: run.trace,
    })
}

fn run_ail_pass_agent_verify_manifest(
    agent_run: &mut AilBuildAgentRun,
    manifest_text: &str,
    manifest_fingerprint: &str,
) -> Result<(), String> {
    if !agent_run
        .bytecode
        .actions
        .contains_key("VerifyPassManifest")
    {
        return Err(
            "ail-pass --agent --artifact-dir requires a VerifyPassManifest action".to_string(),
        );
    }
    let mut verify_state = agent_run.state.clone();
    verify_state.insert(
        "buildrequest.artifact manifest".to_string(),
        manifest_text.to_string(),
    );
    verify_state.insert(
        "buildrequest.artifact manifest fingerprint".to_string(),
        manifest_fingerprint.to_string(),
    );
    let verify_run =
        run_ail_bytecode_action(&agent_run.bytecode, "VerifyPassManifest", verify_state)?;
    if verify_run.status != "succeeded" {
        let mut message = "ail-pass agent VerifyPassManifest failed".to_string();
        if let Some(failure) = verify_run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !verify_run.trace.is_empty() {
            message.push_str(&format!("\n{}", verify_run.trace.join("\n")));
        }
        return Err(message);
    }
    agent_run.trace.extend(verify_run.trace);
    agent_run.state = verify_run.final_state;
    Ok(())
}

fn load_ail_pass_target_core(cli_options: &CliOptions) -> Result<eigl::ail::AilCore, String> {
    if cli_options.ail_core_file.is_some() {
        return parse_cli_ail_core(cli_options);
    }
    let target_path = cli_options
        .ail_pass_target
        .as_deref()
        .ok_or_else(|| "ail-pass requires a target package or --core-file <path>".to_string())?;
    let target_package = load_ail_package_dir(target_path)?;
    let target_document = parse_ail_package_document(&target_package)?;
    Ok(elaborate_ail_core(&target_package, &target_document))
}

fn load_ail_bytecode_or_compile_package(
    path: &str,
    context: &str,
) -> Result<(eigl::ail::AilBytecodeProgram, String), String> {
    if std::path::Path::new(path).is_file() {
        let text =
            fs::read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
        let bytecode = parse_ail_bytecode(&text)?;
        let normalized_text = if text.ends_with('\n') {
            text
        } else {
            format!("{text}\n")
        };
        return Ok((bytecode, normalized_text));
    }

    let package = load_ail_package_dir(path)?;
    let document = parse_ail_package_document(&package)?;
    let core = elaborate_ail_core(&package, &document);
    let diagnostics = check_ail_core(&core);
    if !diagnostics.is_empty() {
        for diagnostic in diagnostics {
            println!("{diagnostic}");
        }
        return Err(format!("{context} package has diagnostics"));
    }
    let bytecode = compile_ail_core_bytecode(&core)?;
    let text = format!("{}\n", render_ail_bytecode(&bytecode));
    Ok((bytecode, text))
}

fn select_single_ail_pass_action(
    bytecode: &eigl::ail::AilBytecodeProgram,
) -> Result<String, String> {
    let action_names = bytecode.actions.keys().cloned().collect::<Vec<_>>();
    if let [action_name] = action_names.as_slice() {
        return Ok(action_name.clone());
    }
    Err(format!(
        "ail-build --pass requires exactly one compiler pass action, found {}",
        action_names.len()
    ))
}

fn load_verified_ail_build_agent(
    agent_path: &str,
) -> Result<(eigl::ail::AilBytecodeProgram, String), String> {
    let (agent_bytecode, agent_bytecode_text) =
        load_ail_bytecode_or_compile_package(agent_path, "ail-build agent")?;
    let diagnostics = verify_ail_bytecode(&agent_bytecode);
    if !diagnostics.is_empty() {
        return Err(format!(
            "ail-build agent bytecode has diagnostics:\n{}",
            diagnostics.join("\n")
        ));
    }
    if agent_bytecode.profile != "Application" {
        return Err(format!(
            "ail-build --agent requires an Application-profile agent, found {}",
            agent_bytecode.profile
        ));
    }
    Ok((agent_bytecode, agent_bytecode_text))
}

fn compile_ail_build_agent_native_artifacts(
    agent_bytecode: &eigl::ail::AilBytecodeProgram,
    target: &str,
) -> Result<Vec<AilNativeArtifact>, String> {
    compile_ail_native_artifacts(agent_bytecode, target, "agent")
}

fn compile_ail_pass_native_artifacts(
    pass_bytecode: &eigl::ail::AilBytecodeProgram,
    target: &str,
) -> Result<Vec<AilNativeArtifact>, String> {
    compile_ail_native_artifacts(pass_bytecode, target, "pass")
}

fn compile_ail_native_artifacts(
    bytecode: &eigl::ail::AilBytecodeProgram,
    target: &str,
    file_prefix: &str,
) -> Result<Vec<AilNativeArtifact>, String> {
    let mut artifacts = Vec::new();
    for action_name in bytecode.actions.keys() {
        let bytes = compile_ail_bytecode_native_elf(bytecode, action_name, target)?;
        artifacts.push(AilNativeArtifact {
            target_name: target.to_string(),
            file_name: native_action_file_name(file_prefix, action_name),
            bytes,
        });
    }
    Ok(artifacts)
}

fn native_action_file_name(file_prefix: &str, action_name: &str) -> String {
    let safe_action = action_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("{file_prefix}-{safe_action}.elf")
}

fn run_ail_build_agent_capture(
    agent_path: &str,
    package_name: &str,
    capture_prompt: &str,
) -> Result<AilBuildAgentStart, String> {
    let (agent_bytecode, _) = load_verified_ail_build_agent(agent_path)?;
    if !agent_bytecode.actions.contains_key("CaptureRequirements") {
        return Err(
            "ail-build --agent requires a CaptureRequirements action for prompt builds".to_string(),
        );
    }
    let capture_run = run_ail_bytecode_action(
        &agent_bytecode,
        "CaptureRequirements",
        BTreeMap::from([
            ("buildrequest.id".to_string(), package_name.to_string()),
            (
                "buildrequest.developer prompt".to_string(),
                capture_prompt.to_string(),
            ),
            (
                "buildrequest.status".to_string(),
                "PromptReceived".to_string(),
            ),
        ]),
    )?;
    if capture_run.status != "succeeded" {
        let mut message = "ail-build agent CaptureRequirements failed".to_string();
        if let Some(failure) = capture_run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !capture_run.trace.is_empty() {
            message.push_str(&format!("\n{}", capture_run.trace.join("\n")));
        }
        return Err(message);
    }
    Ok(AilBuildAgentStart {
        state: capture_run.final_state,
        trace: capture_run.trace,
    })
}

fn render_ail_build_agent_requirements_context(agent_start: &AilBuildAgentStart) -> String {
    let mut lines = vec![
        "AIL agent CaptureRequirements bytecode completed before this base LLM request."
            .to_string(),
    ];
    lines.extend(
        agent_start
            .state
            .iter()
            .filter(|(key, _)| key.starts_with("buildrequest."))
            .map(|(key, value)| format!("{key}={value}")),
    );
    lines.join("\n")
}

fn start_ail_build_agent_from_saved_requirements(
    package: &eigl::ail::AilPackage,
    prompt: &str,
    requirements_artifact: &str,
) -> AilBuildAgentStart {
    AilBuildAgentStart {
        state: BTreeMap::from([
            ("buildrequest.id".to_string(), package.metadata.name.clone()),
            (
                "buildrequest.developer prompt".to_string(),
                prompt.to_string(),
            ),
            (
                "buildrequest.requirements".to_string(),
                requirements_artifact.to_string(),
            ),
            (
                "buildrequest.status".to_string(),
                "RequirementsLoaded".to_string(),
            ),
        ]),
        trace: Vec::new(),
    }
}

fn run_ail_build_agent_prepare_spec(
    agent_path: &str,
    mut agent_start: AilBuildAgentStart,
    requirements_artifact: &str,
) -> Result<AilBuildAgentStart, String> {
    let (agent_bytecode, _) = load_verified_ail_build_agent(agent_path)?;
    if !agent_bytecode.actions.contains_key("PrepareSpecDraft") {
        return Err(
            "ail-build --agent requires a PrepareSpecDraft action for prompt builds".to_string(),
        );
    }
    agent_start.state.insert(
        "buildrequest.requirements".to_string(),
        requirements_artifact.to_string(),
    );
    let prepare_run =
        run_ail_bytecode_action(&agent_bytecode, "PrepareSpecDraft", agent_start.state)?;
    if prepare_run.status != "succeeded" {
        let mut message = "ail-build agent PrepareSpecDraft failed".to_string();
        if let Some(failure) = prepare_run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !prepare_run.trace.is_empty() {
            message.push_str(&format!("\n{}", prepare_run.trace.join("\n")));
        }
        return Err(message);
    }
    agent_start.trace.extend(prepare_run.trace);
    agent_start.state = prepare_run.final_state;
    Ok(agent_start)
}

fn render_ail_build_agent_spec_context(agent_start: &AilBuildAgentStart) -> String {
    let mut lines = vec![
        "AIL agent PrepareSpecDraft bytecode completed before this base LLM request.".to_string(),
    ];
    lines.extend(
        agent_start
            .state
            .iter()
            .filter(|(key, _)| key.starts_with("buildrequest."))
            .filter(|(key, _)| key.as_str() != "buildrequest.requirements")
            .map(|(key, value)| format!("{key}={value}")),
    );
    lines.join("\n")
}

fn run_ail_build_agent_accept_spec(
    agent_path: &str,
    mut agent_start: AilBuildAgentStart,
    requirements_artifact: &str,
    spec_text: &str,
) -> Result<AilBuildAgentStart, String> {
    let (agent_bytecode, _) = load_verified_ail_build_agent(agent_path)?;
    if !agent_bytecode.actions.contains_key("AcceptSpecDraft") {
        return Err("ail-build --agent requires an AcceptSpecDraft action".to_string());
    }
    agent_start.state.insert(
        "buildrequest.requirements".to_string(),
        requirements_artifact.to_string(),
    );
    agent_start
        .state
        .insert("buildrequest.spec".to_string(), spec_text.to_string());
    let accept_run =
        run_ail_bytecode_action(&agent_bytecode, "AcceptSpecDraft", agent_start.state)?;
    if accept_run.status != "succeeded" {
        let mut message = "ail-build agent AcceptSpecDraft failed".to_string();
        if let Some(failure) = accept_run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !accept_run.trace.is_empty() {
            message.push_str(&format!("\n{}", accept_run.trace.join("\n")));
        }
        return Err(message);
    }
    agent_start.trace.extend(accept_run.trace);
    agent_start.state = accept_run.final_state;
    Ok(agent_start)
}

fn start_ail_build_agent_from_saved_spec(
    package: &eigl::ail::AilPackage,
    spec_text: &str,
) -> AilBuildAgentStart {
    AilBuildAgentStart {
        state: BTreeMap::from([
            ("buildrequest.id".to_string(), package.metadata.name.clone()),
            (
                "buildrequest.developer prompt".to_string(),
                "skipped".to_string(),
            ),
            (
                "buildrequest.requirements".to_string(),
                "skipped".to_string(),
            ),
            ("buildrequest.spec".to_string(), spec_text.to_string()),
            ("buildrequest.status".to_string(), "SpecLoaded".to_string()),
        ]),
        trace: Vec::new(),
    }
}

fn run_ail_build_agent_accept_pass_output(
    agent_path: &str,
    mut agent_start: AilBuildAgentStart,
    acceptance: AilBuildPassAcceptance<'_>,
) -> Result<AilBuildAgentStart, String> {
    let (agent_bytecode, _) = load_verified_ail_build_agent(agent_path)?;
    if !agent_bytecode
        .actions
        .contains_key("AcceptCompilerPassOutput")
    {
        return Err(
            "ail-build --agent --pass requires an AcceptCompilerPassOutput action".to_string(),
        );
    }
    if let Some(requirements_artifact) = acceptance.requirements_artifact {
        agent_start.state.insert(
            "buildrequest.requirements".to_string(),
            requirements_artifact.to_string(),
        );
    }
    if let Some(spec_text) = acceptance.spec_text {
        agent_start
            .state
            .insert("buildrequest.spec".to_string(), spec_text.to_string());
    }
    agent_start.state.insert(
        "buildrequest.core ir".to_string(),
        acceptance.core_text.to_string(),
    );
    agent_start.state.insert(
        "buildrequest.compiler pass artifact".to_string(),
        format!(
            "Verified AIL compiler pass bytecode ({} bytes)",
            acceptance.pass_bytecode_text.len()
        ),
    );
    agent_start.state.insert(
        "buildrequest.compiler pass fingerprint".to_string(),
        acceptance.pass_bytecode_fingerprint.to_string(),
    );
    agent_start.state.insert(
        "buildrequest.compiler pass trace".to_string(),
        acceptance.pass_trace.join("\n"),
    );
    let pass_run = run_ail_bytecode_action(
        &agent_bytecode,
        "AcceptCompilerPassOutput",
        agent_start.state,
    )?;
    if pass_run.status != "succeeded" {
        let mut message = "ail-build agent AcceptCompilerPassOutput failed".to_string();
        if let Some(failure) = pass_run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !pass_run.trace.is_empty() {
            message.push_str(&format!("\n{}", pass_run.trace.join("\n")));
        }
        return Err(message);
    }
    agent_start.trace.extend(pass_run.trace);
    agent_start.state = pass_run.final_state;
    Ok(agent_start)
}

fn run_ail_build_agent_accept_core(
    agent_path: &str,
    mut agent_start: AilBuildAgentStart,
    requirements_artifact: Option<&str>,
    spec_text: Option<&str>,
    core_text: &str,
) -> Result<AilBuildAgentStart, String> {
    let (agent_bytecode, _) = load_verified_ail_build_agent(agent_path)?;
    if !agent_bytecode.actions.contains_key("AcceptCoreIR") {
        return Err("ail-build --agent requires an AcceptCoreIR action".to_string());
    }
    if let Some(requirements_artifact) = requirements_artifact {
        agent_start.state.insert(
            "buildrequest.requirements".to_string(),
            requirements_artifact.to_string(),
        );
    }
    if let Some(spec_text) = spec_text {
        agent_start
            .state
            .insert("buildrequest.spec".to_string(), spec_text.to_string());
    }
    agent_start
        .state
        .insert("buildrequest.core ir".to_string(), core_text.to_string());
    let accept_run = run_ail_bytecode_action(&agent_bytecode, "AcceptCoreIR", agent_start.state)?;
    if accept_run.status != "succeeded" {
        let mut message = "ail-build agent AcceptCoreIR failed".to_string();
        if let Some(failure) = accept_run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !accept_run.trace.is_empty() {
            message.push_str(&format!("\n{}", accept_run.trace.join("\n")));
        }
        return Err(message);
    }
    agent_start.trace.extend(accept_run.trace);
    agent_start.state = accept_run.final_state;
    Ok(agent_start)
}

fn start_ail_build_agent_from_checked_core(
    core: &eigl::ail::AilCore,
    requirements_artifact: Option<&str>,
    spec_text: Option<&str>,
    capture_prompt: Option<&str>,
) -> AilBuildAgentStart {
    let mut state = BTreeMap::from([
        ("buildrequest.id".to_string(), core.package.name.clone()),
        (
            "buildrequest.developer prompt".to_string(),
            capture_prompt.unwrap_or("skipped").to_string(),
        ),
        (
            "buildrequest.requirements".to_string(),
            requirements_artifact.unwrap_or("skipped").to_string(),
        ),
        (
            "buildrequest.spec".to_string(),
            spec_text.unwrap_or("skipped").to_string(),
        ),
        (
            "buildrequest.status".to_string(),
            if spec_text.is_some() {
                "SpecCaptured".to_string()
            } else {
                "CoreLoaded".to_string()
            },
        ),
    ]);
    state.insert("buildrequest.core ir".to_string(), "Pending".to_string());
    AilBuildAgentStart {
        state,
        trace: Vec::new(),
    }
}

fn run_ail_build_agent(
    agent_path: &str,
    core: &eigl::ail::AilCore,
    requirements_artifact: Option<&str>,
    spec_text: Option<&str>,
    capture_prompt: Option<&str>,
    target_model: Option<&str>,
    agent_start: Option<AilBuildAgentStart>,
) -> Result<AilBuildAgentRun, String> {
    let (agent_bytecode, agent_bytecode_text) = load_verified_ail_build_agent(agent_path)?;
    if !agent_bytecode.actions.contains_key("CompileApplication") {
        return Err("ail-build --agent requires a CompileApplication action".to_string());
    }
    let (mut compile_state, mut trace) = if let Some(agent_start) = agent_start {
        (agent_start.state, agent_start.trace)
    } else {
        (
            BTreeMap::from([
                ("buildrequest.id".to_string(), core.package.name.clone()),
                (
                    "buildrequest.developer prompt".to_string(),
                    capture_prompt.unwrap_or("skipped").to_string(),
                ),
                (
                    "buildrequest.status".to_string(),
                    "PromptReceived".to_string(),
                ),
            ]),
            Vec::new(),
        )
    };
    if trace.is_empty()
        && let Some(capture_prompt) = capture_prompt
    {
        if !agent_bytecode.actions.contains_key("CaptureRequirements") {
            return Err(
                "ail-build --agent requires a CaptureRequirements action for prompt builds"
                    .to_string(),
            );
        }
        let capture_run = run_ail_bytecode_action(
            &agent_bytecode,
            "CaptureRequirements",
            BTreeMap::from([
                ("buildrequest.id".to_string(), core.package.name.clone()),
                (
                    "buildrequest.developer prompt".to_string(),
                    capture_prompt.to_string(),
                ),
                (
                    "buildrequest.status".to_string(),
                    "PromptReceived".to_string(),
                ),
            ]),
        )?;
        if capture_run.status != "succeeded" {
            let mut message = "ail-build agent CaptureRequirements failed".to_string();
            if let Some(failure) = capture_run.failure {
                message.push_str(&format!(": {failure}"));
            }
            if !capture_run.trace.is_empty() {
                message.push_str(&format!("\n{}", capture_run.trace.join("\n")));
            }
            return Err(message);
        }
        trace.extend(capture_run.trace);
        compile_state = capture_run.final_state;
    }
    compile_state.insert(
        "buildrequest.requirements".to_string(),
        requirements_artifact.unwrap_or("skipped").to_string(),
    );
    if let Some(target_model) = target_model {
        if !agent_bytecode
            .actions
            .contains_key("CompareAgentPromptPortability")
        {
            return Err(
                "ail-build --agent --target-model requires a CompareAgentPromptPortability action"
                    .to_string(),
            );
        }
        compile_state.insert(
            "buildrequest.target model".to_string(),
            target_model.to_string(),
        );
        let compare_run = run_ail_bytecode_action(
            &agent_bytecode,
            "CompareAgentPromptPortability",
            compile_state,
        )?;
        if compare_run.status != "succeeded" {
            let mut message = "ail-build agent CompareAgentPromptPortability failed".to_string();
            if let Some(failure) = compare_run.failure {
                message.push_str(&format!(": {failure}"));
            }
            if !compare_run.trace.is_empty() {
                message.push_str(&format!("\n{}", compare_run.trace.join("\n")));
            }
            return Err(message);
        }
        trace.extend(compare_run.trace);
        compile_state = compare_run.final_state;
    }
    let build_status = if compile_state
        .get("buildrequest.status")
        .is_some_and(|status| status == "CoreChecked")
    {
        "CoreChecked"
    } else if spec_text.is_some() {
        "SpecCaptured"
    } else {
        "CoreChecked"
    };
    compile_state.insert(
        "buildrequest.spec".to_string(),
        spec_text.unwrap_or("skipped").to_string(),
    );
    compile_state.insert("buildrequest.core ir".to_string(), "Checked".to_string());
    compile_state.insert(
        "buildrequest.bytecode artifact".to_string(),
        "Pending".to_string(),
    );
    compile_state.insert("buildrequest.status".to_string(), build_status.to_string());
    let run = run_ail_bytecode_action(&agent_bytecode, "CompileApplication", compile_state)?;
    if run.status != "succeeded" {
        let mut message = "ail-build agent CompileApplication failed".to_string();
        if let Some(failure) = run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !run.trace.is_empty() {
            message.push_str(&format!("\n{}", run.trace.join("\n")));
        }
        return Err(message);
    }
    trace.extend(run.trace);
    Ok(AilBuildAgentRun {
        bytecode: agent_bytecode,
        bytecode_text: agent_bytecode_text,
        state: run.final_state,
        trace,
    })
}

fn run_ail_build_agent_verify_bytecode(
    agent_run: &mut AilBuildAgentRun,
    bytecode_text: &str,
    bytecode_fingerprint: &str,
) -> Result<(), String> {
    if !agent_run
        .bytecode
        .actions
        .contains_key("VerifyBytecodeArtifact")
    {
        return Err(
            "ail-build --agent requires a VerifyBytecodeArtifact action after bytecode emission"
                .to_string(),
        );
    }
    let mut verify_state = agent_run.state.clone();
    verify_state.insert(
        "buildrequest.bytecode artifact".to_string(),
        format!("Verified AIL-Bytecode ({} bytes)", bytecode_text.len()),
    );
    verify_state.insert(
        "buildrequest.bytecode fingerprint".to_string(),
        bytecode_fingerprint.to_string(),
    );
    let verify_run =
        run_ail_bytecode_action(&agent_run.bytecode, "VerifyBytecodeArtifact", verify_state)?;
    if verify_run.status != "succeeded" {
        let mut message = "ail-build agent VerifyBytecodeArtifact failed".to_string();
        if let Some(failure) = verify_run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !verify_run.trace.is_empty() {
            message.push_str(&format!("\n{}", verify_run.trace.join("\n")));
        }
        return Err(message);
    }
    agent_run.trace.extend(verify_run.trace);
    agent_run.state = verify_run.final_state;
    Ok(())
}

fn run_ail_build_agent_verify_target_artifact(
    agent_run: &mut AilBuildAgentRun,
    artifact_summary: &str,
    artifact_fingerprint: &str,
) -> Result<(), String> {
    if !agent_run
        .bytecode
        .actions
        .contains_key("VerifyTargetArtifact")
    {
        return Err(
            "ail-build --agent requires a VerifyTargetArtifact action after target artifact emission"
                .to_string(),
        );
    }
    let mut verify_state = agent_run.state.clone();
    verify_state.insert(
        "buildrequest.target artifact".to_string(),
        artifact_summary.to_string(),
    );
    verify_state.insert(
        "buildrequest.target artifact fingerprint".to_string(),
        artifact_fingerprint.to_string(),
    );
    let verify_run =
        run_ail_bytecode_action(&agent_run.bytecode, "VerifyTargetArtifact", verify_state)?;
    if verify_run.status != "succeeded" {
        let mut message = "ail-build agent VerifyTargetArtifact failed".to_string();
        if let Some(failure) = verify_run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !verify_run.trace.is_empty() {
            message.push_str(&format!("\n{}", verify_run.trace.join("\n")));
        }
        return Err(message);
    }
    agent_run.trace.extend(verify_run.trace);
    agent_run.state = verify_run.final_state;
    Ok(())
}

fn run_ail_build_agent_verify_manifest(
    agent_run: &mut AilBuildAgentRun,
    manifest_text: &str,
    manifest_fingerprint: &str,
) -> Result<(), String> {
    if !agent_run
        .bytecode
        .actions
        .contains_key("VerifyBuildManifest")
    {
        return Err(
            "ail-build --agent --artifact-dir requires a VerifyBuildManifest action".to_string(),
        );
    }
    let mut verify_state = agent_run.state.clone();
    verify_state.insert(
        "buildrequest.artifact manifest".to_string(),
        manifest_text.to_string(),
    );
    verify_state.insert(
        "buildrequest.artifact manifest fingerprint".to_string(),
        manifest_fingerprint.to_string(),
    );
    let verify_run =
        run_ail_bytecode_action(&agent_run.bytecode, "VerifyBuildManifest", verify_state)?;
    if verify_run.status != "succeeded" {
        let mut message = "ail-build agent VerifyBuildManifest failed".to_string();
        if let Some(failure) = verify_run.failure {
            message.push_str(&format!(": {failure}"));
        }
        if !verify_run.trace.is_empty() {
            message.push_str(&format!("\n{}", verify_run.trace.join("\n")));
        }
        return Err(message);
    }
    agent_run.trace.extend(verify_run.trace);
    agent_run.state = verify_run.final_state;
    Ok(())
}

fn draft_checked_ail_requirements_for_package(
    package: &eigl::ail::AilPackage,
    prompt: &str,
    endpoint: &str,
    agent_requirements_context: Option<&str>,
) -> Result<(String, Vec<eigl::ail::AilDiagnostic>), String> {
    let grounded_prompt = if let Some(agent_requirements_context) =
        agent_requirements_context.filter(|context| !context.trim().is_empty())
    {
        format!(
            concat!(
                "{}\n\n",
                "Use this AIL agent preflight state as a requirements coverage checklist. ",
                "Do not restate it by itself; produce a full AIL-Requirements artifact with bullets for domain objects, required fields, action inputs or preconditions, failure cases, guarantees, trace events, secrets, permissions, and runtime inputs.\n\n",
                "AGENT REQUIREMENTS CONTEXT:\n",
                "{}"
            ),
            prompt, agent_requirements_context
        )
    } else {
        prompt.to_string()
    };
    let mut requirements = draft_ail_requirements(package, &grounded_prompt, endpoint)?;
    let mut diagnostics = check_ail_requirements(package, &requirements);
    if !diagnostics.is_empty() {
        requirements = repair_ail_requirements_from_diagnostics(
            package,
            &grounded_prompt,
            &requirements,
            &diagnostics,
            endpoint,
        )?;
        diagnostics = check_ail_requirements(package, &requirements);
    }
    Ok((requirements, diagnostics))
}

fn draft_checked_ail_spec_for_requirements(
    package: &eigl::ail::AilPackage,
    prompt: &str,
    requirements: &str,
    endpoint: &str,
    agent_spec_context: Option<&str>,
) -> Result<eigl::ail::AilDraftResult, String> {
    let grounded_prompt = if let Some(agent_spec_context) =
        agent_spec_context.filter(|context| !context.trim().is_empty())
    {
        format!(
            concat!(
                "{}\n\n",
                "Use this AIL agent preflight state as a spec coverage checklist. ",
                "Do not restate it by itself; produce a complete AIL-Spec candidate that preserves the checked requirements, domain model, actions, failures, guarantees, traces, secrets, runtime inputs, and bytecode compilation path.\n\n",
                "AGENT SPEC CONTEXT:\n",
                "{}"
            ),
            prompt, agent_spec_context
        )
    } else {
        prompt.to_string()
    };
    let mut draft =
        draft_ail_spec_from_requirements(package, &grounded_prompt, requirements, endpoint)?;
    if !draft.success() {
        draft = repair_ail_spec_from_diagnostics(
            package,
            &grounded_prompt,
            requirements,
            &draft.spec_text,
            &draft.diagnostics,
            endpoint,
        )?;
    }
    Ok(draft)
}

fn read_checked_ail_requirements_file(
    package: &eigl::ail::AilPackage,
    requirements_file: &str,
) -> Result<(String, Vec<eigl::ail::AilDiagnostic>), String> {
    let requirements = fs::read_to_string(requirements_file)
        .map_err(|error| format!("failed to read {requirements_file}: {error}"))?;
    let requirements = requirements.trim().to_string();
    let diagnostics = check_ail_requirements(package, &requirements);
    Ok((requirements, diagnostics))
}

fn parse_cli_ail_document(
    package: &eigl::ail::AilPackage,
    cli_options: &CliOptions,
) -> Result<eigl::ail::AilDocument, String> {
    if let Some(spec_file) = &cli_options.ail_spec_file {
        let spec_text = fs::read_to_string(spec_file)
            .map_err(|error| format!("failed to read {spec_file}: {error}"))?;
        return parse_ail_package_spec_text(package, &spec_text);
    }
    parse_ail_package_document(package)
}

fn parse_cli_ail_core(cli_options: &CliOptions) -> Result<eigl::ail::AilCore, String> {
    let core_file = cli_options
        .ail_core_file
        .as_deref()
        .ok_or_else(|| "missing --core-file path".to_string())?;
    let core_text = fs::read_to_string(core_file)
        .map_err(|error| format!("failed to read {core_file}: {error}"))?;
    parse_ail_core_text(&core_text)
}

fn run_ail_compile_from_core(
    core: &eigl::ail::AilCore,
    cli_options: &CliOptions,
) -> Result<u8, String> {
    let diagnostics = check_ail_core(core);
    if !diagnostics.is_empty() {
        for diagnostic in diagnostics {
            println!("{diagnostic}");
        }
        return Ok(1);
    }
    let action = cli_options
        .ail_action
        .as_deref()
        .ok_or_else(|| "ail-compile requires --action <name>".to_string())?;
    let target = cli_options
        .ail_compile_target
        .as_deref()
        .ok_or_else(|| "ail-compile requires --target <target>".to_string())?;
    let out = cli_options
        .ail_compile_out
        .as_deref()
        .ok_or_else(|| "ail-compile requires --out <path>".to_string())?;
    let executable = compile_ail_core_native_elf(core, action, target)?;
    write_native_executable(out, &executable)?;
    println!("ail-compile wrote {target} executable {out}");
    Ok(0)
}

fn run_ail_build_from_core(
    mut core: eigl::ail::AilCore,
    cli_options: &CliOptions,
    requirements_artifact: Option<&str>,
    spec_text: Option<&str>,
    capture_prompt: Option<&str>,
    agent_start: Option<AilBuildAgentStart>,
) -> Result<u8, String> {
    let core_diagnostics = check_ail_core(&core);
    if !core_diagnostics.is_empty() {
        println!("ail-build diagnostics:");
        for diagnostic in core_diagnostics {
            println!("{diagnostic}");
        }
        return Ok(1);
    }
    let mut pass_bytecode_artifact = None;
    let mut pass_bytecode_fingerprint_artifact = None;
    let mut pass_trace_artifact = None;
    if let Some(pass_path) = &cli_options.ail_build_pass {
        let (pass_bytecode, pass_bytecode_text) =
            load_ail_bytecode_or_compile_package(pass_path, "ail-build compiler pass")?;
        let pass_diagnostics = verify_ail_bytecode(&pass_bytecode);
        if !pass_diagnostics.is_empty() {
            println!("ail-build diagnostics:");
            for diagnostic in pass_diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
        let pass_action = select_single_ail_pass_action(&pass_bytecode)?;
        let pass_result = run_ail_compiler_pass_on_core(&pass_bytecode, &pass_action, &core)?;
        core = pass_result.core;
        pass_bytecode_fingerprint_artifact = Some(ail_artifact_fingerprint(&pass_bytecode_text));
        pass_bytecode_artifact = Some(pass_bytecode_text);
        pass_trace_artifact = Some(pass_result.run.trace);
        let core_diagnostics = check_ail_core(&core);
        if !core_diagnostics.is_empty() {
            println!("ail-build diagnostics:");
            for diagnostic in core_diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
    }
    let agent_start = if let Some(agent_path) = &cli_options.ail_build_agent {
        let core_text = render_ail_core(&core);
        let mut agent_start = agent_start.unwrap_or_else(|| {
            start_ail_build_agent_from_checked_core(
                &core,
                requirements_artifact,
                spec_text,
                capture_prompt,
            )
        });
        if let (Some(pass_bytecode_text), Some(pass_trace)) = (
            pass_bytecode_artifact.as_deref(),
            pass_trace_artifact.as_deref(),
        ) {
            let pass_bytecode_fingerprint = pass_bytecode_fingerprint_artifact
                .as_deref()
                .ok_or_else(|| "missing compiler pass bytecode fingerprint".to_string())?;
            agent_start = run_ail_build_agent_accept_pass_output(
                agent_path,
                agent_start,
                AilBuildPassAcceptance {
                    requirements_artifact,
                    spec_text,
                    core_text: &core_text,
                    pass_bytecode_text,
                    pass_bytecode_fingerprint,
                    pass_trace,
                },
            )?;
        }
        Some(run_ail_build_agent_accept_core(
            agent_path,
            agent_start,
            requirements_artifact,
            spec_text,
            &core_text,
        )?)
    } else {
        agent_start
    };
    let mut agent_run = if let Some(agent_path) = &cli_options.ail_build_agent {
        Some(run_ail_build_agent(
            agent_path,
            &core,
            requirements_artifact,
            spec_text,
            capture_prompt,
            cli_options.ail_build_target_model.as_deref(),
            agent_start,
        )?)
    } else {
        None
    };
    let bytecode = compile_ail_core_bytecode(&core)?;
    let diagnostics = verify_ail_bytecode(&bytecode);
    if !diagnostics.is_empty() {
        println!("ail-build diagnostics:");
        for diagnostic in diagnostics {
            println!("{diagnostic}");
        }
        return Ok(1);
    }
    let bytecode_text = format!("{}\n", render_ail_bytecode(&bytecode));
    let bytecode_fingerprint = ail_artifact_fingerprint(&bytecode_text);
    let native_build = if let Some(target) = &cli_options.ail_compile_target {
        let action = cli_options
            .ail_action
            .as_deref()
            .ok_or_else(|| "ail-build native output requires --action <name>".to_string())?;
        let out = cli_options
            .ail_compile_out
            .as_deref()
            .ok_or_else(|| "ail-build native output requires --out <path>".to_string())?;
        Some((
            target.to_string(),
            out.to_string(),
            compile_ail_core_native_elf(&core, action, target)?,
        ))
    } else {
        None
    };
    if let Some(agent_run) = agent_run.as_mut() {
        run_ail_build_agent_verify_bytecode(agent_run, &bytecode_text, &bytecode_fingerprint)?;
        if let Some((target, out, executable)) = native_build.as_ref() {
            let target_fingerprint = ail_artifact_fingerprint_bytes(executable);
            run_ail_build_agent_verify_target_artifact(
                agent_run,
                &format!("{target} executable {} bytes at {out}", executable.len()),
                &target_fingerprint,
            )?;
        }
    }
    if let Some(artifact_dir) = &cli_options.artifact_dir {
        let core_text = format!("{}\n", render_ail_core(&core));
        let agent_native_artifacts = if let (Some((target, _, _)), Some(agent_run)) =
            (native_build.as_ref(), agent_run.as_ref())
        {
            compile_ail_build_agent_native_artifacts(&agent_run.bytecode, target)?
        } else {
            Vec::new()
        };
        if let Some(agent_run) = agent_run.as_mut() {
            let manifest_text = render_ail_build_manifest(&AilBuildArtifactSet {
                requirements: requirements_artifact,
                spec_text,
                core_text: &core_text,
                bytecode_text: &bytecode_text,
                bytecode_fingerprint: &bytecode_fingerprint,
                target_name: native_build.as_ref().map(|(target, _, _)| target.as_str()),
                target_executable: native_build
                    .as_ref()
                    .map(|(_, _, executable)| executable.as_slice()),
                pass_bytecode_text: pass_bytecode_artifact.as_deref(),
                pass_bytecode_fingerprint: pass_bytecode_fingerprint_artifact.as_deref(),
                pass_trace: pass_trace_artifact.as_deref(),
                agent_bytecode_text: Some(agent_run.bytecode_text.as_str()),
                agent_trace: Some(agent_run.trace.as_slice()),
                agent_native_executables: agent_native_artifacts.as_slice(),
            });
            let manifest_fingerprint = ail_artifact_fingerprint(&manifest_text);
            run_ail_build_agent_verify_manifest(agent_run, &manifest_text, &manifest_fingerprint)?;
        }
        write_ail_build_artifacts(
            artifact_dir,
            AilBuildArtifactSet {
                requirements: requirements_artifact,
                spec_text,
                core_text: &core_text,
                bytecode_text: &bytecode_text,
                bytecode_fingerprint: &bytecode_fingerprint,
                target_name: native_build.as_ref().map(|(target, _, _)| target.as_str()),
                target_executable: native_build
                    .as_ref()
                    .map(|(_, _, executable)| executable.as_slice()),
                pass_bytecode_text: pass_bytecode_artifact.as_deref(),
                pass_bytecode_fingerprint: pass_bytecode_fingerprint_artifact.as_deref(),
                pass_trace: pass_trace_artifact.as_deref(),
                agent_bytecode_text: agent_run.as_ref().map(|run| run.bytecode_text.as_str()),
                agent_trace: agent_run.as_ref().map(|run| run.trace.as_slice()),
                agent_native_executables: agent_native_artifacts.as_slice(),
            },
        )?;
    }
    if let Some((target, out, executable)) = native_build {
        write_native_executable(&out, &executable)?;
        println!("ail-build wrote {target} executable {out}");
        return Ok(0);
    }
    print!("{bytecode_text}");
    Ok(0)
}

fn run_ail_command(command: &str, path: &str, cli_options: &CliOptions) -> Result<u8, String> {
    if command == "ail-pass" {
        return run_ail_pass_command(path, cli_options);
    }
    if command == "ail-build" && cli_options.ail_core_file.is_some() {
        let core = parse_cli_ail_core(cli_options)?;
        return run_ail_build_from_core(core, cli_options, None, None, None, None);
    }
    if command == "ail-compile" && cli_options.ail_core_file.is_some() {
        let core = parse_cli_ail_core(cli_options)?;
        return run_ail_compile_from_core(&core, cli_options);
    }
    if command == "ail-lower" && cli_options.ail_core_file.is_some() {
        let core = parse_cli_ail_core(cli_options)?;
        let diagnostics = check_ail_core(&core);
        if !diagnostics.is_empty() {
            for diagnostic in diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
        let bytecode = compile_ail_core_bytecode(&core)?;
        let bytecode_diagnostics = verify_ail_bytecode(&bytecode);
        if !bytecode_diagnostics.is_empty() {
            for diagnostic in bytecode_diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
        let bytecode_text = format!("{}\n", render_ail_bytecode(&bytecode));
        if let Some(artifact_dir) = &cli_options.artifact_dir {
            let core_text = format!("{}\n", render_ail_core(&core));
            write_ail_lower_artifacts(artifact_dir, &core_text, &bytecode_text)?;
        }
        print!("{bytecode_text}");
        return Ok(0);
    }
    let package = load_ail_package_dir(path)?;
    if command == "ail-conformance" {
        let result = run_ail_conformance(&package)?;
        println!("ail conformance: package {}", result.package_name);
        if result.accepted_diagnostics.is_empty() {
            println!("valid: {}", result.accepted_fixture);
        } else {
            for diagnostic in &result.accepted_diagnostics {
                println!(
                    "valid: {} {}",
                    result.accepted_fixture,
                    diagnostic.detailed_message()
                );
            }
        }
        for fixture in &result.accepted {
            if fixture.diagnostics.is_empty() {
                println!("accepted: {}", fixture.fixture);
            } else {
                for diagnostic in &fixture.diagnostics {
                    println!(
                        "accepted: {} {}",
                        fixture.fixture,
                        diagnostic.detailed_message()
                    );
                }
            }
        }
        for fixture in &result.rejected {
            if fixture.diagnostics.is_empty() {
                println!("rejected: {} unexpectedly accepted", fixture.fixture);
            } else {
                for diagnostic in &fixture.diagnostics {
                    println!(
                        "rejected: {} {}",
                        fixture.fixture,
                        diagnostic.detailed_message()
                    );
                }
            }
        }
        if result.success() {
            println!("ail conformance: ok");
            return Ok(0);
        }
        println!("ail conformance: failed");
        return Ok(1);
    }
    if command == "ail-draft" {
        let prompt = cli_options
            .ail_prompt
            .as_deref()
            .ok_or_else(|| "ail-draft requires --prompt <text>".to_string())?;
        let endpoint = cli_options
            .llm_endpoint
            .as_deref()
            .unwrap_or(&package.metadata.base_llm_endpoint);
        let result = draft_ail_spec(&package, prompt, endpoint)?;
        println!("ail-draft candidate:");
        println!("{}", result.spec_text);
        if result.success() {
            println!("ail-draft diagnostics: none");
            return Ok(0);
        }
        println!("ail-draft diagnostics:");
        for diagnostic in result.diagnostics {
            println!("{}", diagnostic.detailed_message());
        }
        return Ok(1);
    }
    if command == "ail-requirements" {
        let prompt = cli_options
            .ail_prompt
            .as_deref()
            .ok_or_else(|| "ail-requirements requires --prompt <text>".to_string())?;
        let endpoint = cli_options
            .llm_endpoint
            .as_deref()
            .unwrap_or(&package.metadata.base_llm_endpoint);
        let (requirements, diagnostics) =
            draft_checked_ail_requirements_for_package(&package, prompt, endpoint, None)?;
        if !diagnostics.is_empty() {
            println!("ail-requirements diagnostics:");
            for diagnostic in diagnostics {
                println!("{}", diagnostic.detailed_message());
            }
            return Ok(1);
        }
        println!("{requirements}");
        return Ok(0);
    }
    if command == "ail-spec" {
        let prompt = cli_options
            .ail_prompt
            .as_deref()
            .ok_or_else(|| "ail-spec requires --prompt <text>".to_string())?;
        let requirements_file = cli_options
            .ail_requirements_file
            .as_deref()
            .ok_or_else(|| "ail-spec requires --requirements-file <path>".to_string())?;
        let (requirements, requirements_diagnostics) =
            read_checked_ail_requirements_file(&package, requirements_file)?;
        if !requirements_diagnostics.is_empty() {
            println!("ail-spec requirements diagnostics:");
            for diagnostic in requirements_diagnostics {
                println!("{}", diagnostic.detailed_message());
            }
            return Ok(1);
        }
        let endpoint = cli_options
            .llm_endpoint
            .as_deref()
            .unwrap_or(&package.metadata.base_llm_endpoint);
        let draft = draft_checked_ail_spec_for_requirements(
            &package,
            prompt,
            &requirements,
            endpoint,
            None,
        )?;
        if !draft.success() {
            println!("ail-spec diagnostics:");
            for diagnostic in draft.diagnostics {
                println!("{}", diagnostic.detailed_message());
            }
            return Ok(1);
        }
        println!("{}", draft.spec_text);
        return Ok(0);
    }
    if command == "ail-build" {
        let (requirements_artifact, spec_text, core, capture_prompt, agent_start) =
            if let Some(spec_file) = cli_options.ail_spec_file.as_deref() {
                let spec_text = fs::read_to_string(spec_file)
                    .map_err(|error| format!("failed to read {spec_file}: {error}"))?;
                let spec_text = spec_text.trim().to_string();
                let agent_start = if let Some(agent_path) = cli_options.ail_build_agent.as_deref() {
                    let agent_start = start_ail_build_agent_from_saved_spec(&package, &spec_text);
                    Some(run_ail_build_agent_accept_spec(
                        agent_path,
                        agent_start,
                        "skipped",
                        &spec_text,
                    )?)
                } else {
                    None
                };
                let document = parse_ail_package_spec_text(&package, &spec_text)?;
                let core = elaborate_ail_core(&package, &document);
                (None, spec_text, core, None, agent_start)
            } else {
                let prompt = cli_options
                    .ail_prompt
                    .as_deref()
                    .ok_or_else(|| "ail-build requires --prompt <text>".to_string())?;
                let mut agent_start = if let Some(agent_path) =
                    cli_options.ail_build_agent.as_deref()
                    && cli_options.ail_requirements_file.is_none()
                {
                    Some(run_ail_build_agent_capture(
                        agent_path,
                        &package.metadata.name,
                        prompt,
                    )?)
                } else {
                    None
                };
                let endpoint = cli_options
                    .llm_endpoint
                    .as_deref()
                    .unwrap_or(&package.metadata.base_llm_endpoint);
                let agent_requirements_context = agent_start
                    .as_ref()
                    .map(render_ail_build_agent_requirements_context);
                let (requirements, requirements_diagnostics) =
                    if let Some(requirements_file) = cli_options.ail_requirements_file.as_deref() {
                        read_checked_ail_requirements_file(&package, requirements_file)?
                    } else {
                        draft_checked_ail_requirements_for_package(
                            &package,
                            prompt,
                            endpoint,
                            agent_requirements_context.as_deref(),
                        )?
                    };
                let capture_prompt = cli_options
                    .ail_requirements_file
                    .is_none()
                    .then(|| prompt.to_string());
                if !requirements_diagnostics.is_empty() {
                    println!("ail-build requirements diagnostics:");
                    for diagnostic in requirements_diagnostics {
                        println!("{}", diagnostic.detailed_message());
                    }
                    return Ok(1);
                }
                if agent_start.is_none()
                    && cli_options.ail_build_agent.is_some()
                    && cli_options.ail_requirements_file.is_some()
                {
                    agent_start = Some(start_ail_build_agent_from_saved_requirements(
                        &package,
                        prompt,
                        &requirements,
                    ));
                }
                let agent_spec_context = if let (Some(agent_path), Some(previous_agent_start)) =
                    (cli_options.ail_build_agent.as_deref(), agent_start.take())
                {
                    let prepared_agent_start = run_ail_build_agent_prepare_spec(
                        agent_path,
                        previous_agent_start,
                        &requirements,
                    )?;
                    let context = render_ail_build_agent_spec_context(&prepared_agent_start);
                    agent_start = Some(prepared_agent_start);
                    Some(context)
                } else {
                    None
                };
                let draft = draft_checked_ail_spec_for_requirements(
                    &package,
                    prompt,
                    &requirements,
                    endpoint,
                    agent_spec_context.as_deref(),
                )?;
                if !draft.success() {
                    println!("ail-build diagnostics:");
                    for diagnostic in draft.diagnostics {
                        println!("{}", diagnostic.detailed_message());
                    }
                    return Ok(1);
                }
                if let (Some(agent_path), Some(previous_agent_start)) =
                    (cli_options.ail_build_agent.as_deref(), agent_start.take())
                {
                    agent_start = Some(run_ail_build_agent_accept_spec(
                        agent_path,
                        previous_agent_start,
                        &requirements,
                        &draft.spec_text,
                    )?);
                }
                let document = parse_ail_package_spec_text(&package, &draft.spec_text)?;
                let core = elaborate_ail_core(&package, &document);
                (
                    Some(requirements),
                    draft.spec_text,
                    core,
                    capture_prompt,
                    agent_start,
                )
            };
        return run_ail_build_from_core(
            core,
            cli_options,
            requirements_artifact.as_deref(),
            Some(&spec_text),
            capture_prompt.as_deref(),
            agent_start,
        );
    }
    let document = parse_cli_ail_document(&package, cli_options)?;
    if command == "ail-patch" {
        let Some(patch_path) = cli_options.patch_path.as_ref() else {
            return Err("ail-patch requires a patch file".to_string());
        };
        let patch_text = fs::read_to_string(patch_path)
            .map_err(|error| format!("failed to read {patch_path}: {error}"))?;
        let patch = parse_ail_patch_text(&patch_text)?;
        let patched = apply_ail_patch(&document, &patch)?;
        let core = elaborate_ail_core(&package, &patched);
        let diagnostics = check_ail_core(&core);
        if !diagnostics.is_empty() {
            for diagnostic in diagnostics {
                println!("{diagnostic}");
            }
            return Ok(1);
        }
        println!("{}", render_ail_spec(&patched));
        return Ok(0);
    }
    let core = elaborate_ail_core(&package, &document);
    let diagnostics = check_ail_core(&core);
    match command {
        "ail-check" => {
            println!("ail package: {}", package.metadata.name);
            println!("profile: {}", package.metadata.profile);
            println!("base_llm_endpoint: {}", package.metadata.base_llm_endpoint);
            if diagnostics.is_empty() {
                println!("ail diagnostics: none");
                Ok(0)
            } else {
                println!("ail diagnostics:");
                for diagnostic in diagnostics {
                    println!("{diagnostic}");
                }
                Ok(1)
            }
        }
        "ail-core" => {
            if !diagnostics.is_empty() {
                for diagnostic in diagnostics {
                    println!("{diagnostic}");
                }
                return Ok(1);
            }
            println!("{}", render_ail_core(&core));
            Ok(0)
        }
        "ail-flow" => {
            if !diagnostics.is_empty() {
                for diagnostic in diagnostics {
                    println!("{diagnostic}");
                }
                return Ok(1);
            }
            println!("{}", render_ail_flow_view(&core));
            Ok(0)
        }
        "ail-lower" => {
            if !diagnostics.is_empty() {
                for diagnostic in diagnostics {
                    println!("{diagnostic}");
                }
                return Ok(1);
            }
            let bytecode = compile_ail_core_bytecode(&core)?;
            let bytecode_text = format!("{}\n", render_ail_bytecode(&bytecode));
            if let Some(artifact_dir) = &cli_options.artifact_dir {
                let core_text = format!("{}\n", render_ail_core(&core));
                write_ail_lower_artifacts(artifact_dir, &core_text, &bytecode_text)?;
            }
            print!("{bytecode_text}");
            Ok(0)
        }
        "ail-compile" => run_ail_compile_from_core(&core, cli_options),
        "ail-run" => {
            if !diagnostics.is_empty() {
                for diagnostic in diagnostics {
                    println!("{diagnostic}");
                }
                return Ok(1);
            }
            let action = cli_options
                .ail_action
                .as_deref()
                .ok_or_else(|| "ail-run requires --action <name>".to_string())?;
            let bytecode = compile_ail_core_bytecode(&core)?;
            let result =
                run_ail_bytecode_action(&bytecode, action, cli_options.runtime_state.clone())?;
            println!("ail-run {}", result.status);
            if let Some(failure) = &result.failure {
                println!("failure={failure}");
            }
            for line in render_ail_runtime_state_lines(&document, &result.final_state) {
                println!("{line}");
            }
            if !result.trace.is_empty() {
                println!("trace={}", result.trace.join(" -> "));
            }
            Ok(if result.status == "succeeded" { 0 } else { 1 })
        }
        _ => Err(format!("unknown AIL command '{command}'")),
    }
}

fn print_runtime_state(document: &RifDocument, runtime_state: &BTreeMap<String, String>) {
    for line in runtime_state_lines(document, runtime_state) {
        println!("{line}");
    }
}

fn runtime_state_lines(
    document: &RifDocument,
    runtime_state: &BTreeMap<String, String>,
) -> Vec<String> {
    runtime_state
        .iter()
        .map(|(key, value)| format!("{key}={}", display_runtime_value(document, key, value)))
        .collect()
}

fn display_runtime_value(document: &RifDocument, key: &str, value: &str) -> String {
    let is_secret = runtime_state_type(document, &document.intent, key)
        .ok()
        .flatten()
        .is_some_and(type_contains_secret);
    if is_secret {
        "<secret>".to_string()
    } else {
        value.to_string()
    }
}

fn type_contains_secret(type_name: &str) -> bool {
    let type_name = type_name.trim();
    if generic_inner(type_name, "Secret").is_some() {
        return true;
    }
    let Some((_, inner)) = type_name.split_once('<') else {
        return false;
    };
    let Some(inner) = inner.strip_suffix('>') else {
        return false;
    };
    split_top_level_commas(inner)
        .into_iter()
        .any(type_contains_secret)
}

fn parse_document_file(path: &str) -> Result<RifDocument, String> {
    if path.ends_with(".rsl.md") || path.ends_with(".rsl") {
        parse_rsl_file(path)
    } else {
        parse_rif_file(path)
    }
}

fn parse_cli_options(command: &str, args: &[String]) -> Result<CliOptions, String> {
    let mut selected_intent = None;
    let mut runtime_state = BTreeMap::new();
    let mut request_state = BTreeMap::new();
    let mut state_in = None;
    let mut state_out = None;
    let mut data_in = None;
    let mut data_out = None;
    let mut operation_outputs = BTreeMap::new();
    let mut listen = None;
    let mut llm_endpoint = None;
    let mut artifact_dir = None;
    let mut patch_path = None;
    let mut dispatch_method = None;
    let mut dispatch_path = None;
    let mut trigger_name = None;
    let mut ail_action = None;
    let mut ail_prompt = None;
    let mut ail_pass_target = None;
    let mut ail_build_pass = None;
    let mut ail_build_agent = None;
    let mut ail_build_target_model = None;
    let mut ail_requirements_file = None;
    let mut ail_spec_file = None;
    let mut ail_core_file = None;
    let mut ail_compile_target = None;
    let mut ail_compile_out = None;
    let mut index = 0;
    if command == "patch" || command == "ail-patch" {
        let Some(path) = args.get(index) else {
            return Err(format!("{command} requires a patch file"));
        };
        patch_path = Some(path.clone());
        index += 1;
    }
    if command == "dispatch" {
        let Some(method) = args.get(index) else {
            return Err("dispatch requires an HTTP method".to_string());
        };
        let Some(path) = args.get(index + 1) else {
            return Err("dispatch requires a request path".to_string());
        };
        dispatch_method = Some(method.clone());
        dispatch_path = Some(path.clone());
        index += 2;
    }
    if command == "emit" || command == "schedule" || command == "dequeue" {
        let Some(name) = args.get(index) else {
            return Err(format!("{command} requires a trigger name"));
        };
        trigger_name = Some(name.clone());
        index += 1;
    }
    if command == "ail-pass" && args.get(index).is_none_or(|arg| arg != "--core-file") {
        let Some(target_package) = args.get(index) else {
            return Err("ail-pass requires a target package or --core-file <path>".to_string());
        };
        ail_pass_target = Some(target_package.clone());
        index += 1;
    }
    while index < args.len() {
        let arg = &args[index];
        if arg == "--intent" {
            let Some(intent_name) = args.get(index + 1) else {
                return Err("missing value for --intent".to_string());
            };
            selected_intent = Some(intent_name.clone());
            index += 2;
            continue;
        }
        if arg == "--action" {
            if !matches!(
                command,
                "ail-run" | "ail-vm" | "ail-pass" | "ail-compile" | "ail-build"
            ) {
                return Err(usage());
            }
            let Some(action_name) = args.get(index + 1) else {
                return Err("missing value for --action".to_string());
            };
            ail_action = Some(action_name.clone());
            index += 2;
            continue;
        }
        if arg == "--prompt" {
            if !matches!(
                command,
                "ail-requirements" | "ail-spec" | "ail-draft" | "ail-build"
            ) {
                return Err(usage());
            }
            let Some(prompt) = args.get(index + 1) else {
                return Err("missing value for --prompt".to_string());
            };
            ail_prompt = Some(prompt.clone());
            index += 2;
            continue;
        }
        if arg == "--requirements-file" {
            if !matches!(command, "ail-spec" | "ail-build") {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --requirements-file".to_string());
            };
            ail_requirements_file = Some(path.clone());
            index += 2;
            continue;
        }
        if arg == "--spec-file" {
            if !matches!(
                command,
                "ail-check"
                    | "ail-core"
                    | "ail-flow"
                    | "ail-lower"
                    | "ail-compile"
                    | "ail-run"
                    | "ail-build"
            ) {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --spec-file".to_string());
            };
            ail_spec_file = Some(path.clone());
            index += 2;
            continue;
        }
        if arg == "--core-file" {
            if !matches!(
                command,
                "ail-lower" | "ail-pass" | "ail-compile" | "ail-build"
            ) {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --core-file".to_string());
            };
            ail_core_file = Some(path.clone());
            index += 2;
            continue;
        }
        if arg == "--pass" {
            if command != "ail-build" {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --pass".to_string());
            };
            ail_build_pass = Some(path.clone());
            index += 2;
            continue;
        }
        if arg == "--agent" {
            if !matches!(command, "ail-build" | "ail-pass") {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --agent".to_string());
            };
            ail_build_agent = Some(path.clone());
            index += 2;
            continue;
        }
        if arg == "--target" {
            if !matches!(command, "ail-compile" | "ail-build" | "ail-pass") {
                return Err(usage());
            }
            let Some(target) = args.get(index + 1) else {
                return Err("missing value for --target".to_string());
            };
            ail_compile_target = Some(target.clone());
            index += 2;
            continue;
        }
        if arg == "--target-model" {
            if command != "ail-build" {
                return Err(usage());
            }
            let Some(model) = args.get(index + 1) else {
                return Err("missing value for --target-model".to_string());
            };
            ail_build_target_model = Some(model.clone());
            index += 2;
            continue;
        }
        if arg == "--out" {
            if !matches!(command, "ail-compile" | "ail-build") {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --out".to_string());
            };
            ail_compile_out = Some(path.clone());
            index += 2;
            continue;
        }
        if arg == "--state-in" {
            if !matches!(
                command,
                "run" | "simulate" | "dispatch" | "emit" | "schedule" | "dequeue" | "serve"
            ) {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --state-in".to_string());
            };
            state_in = Some(path.clone());
            index += 2;
            continue;
        }
        if arg == "--state-out" {
            if !matches!(
                command,
                "run" | "simulate" | "dispatch" | "emit" | "schedule" | "dequeue" | "serve"
            ) {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --state-out".to_string());
            };
            state_out = Some(path.clone());
            index += 2;
            continue;
        }
        if arg == "--data-in" {
            if !matches!(
                command,
                "run" | "simulate" | "dispatch" | "emit" | "schedule" | "dequeue" | "serve"
            ) {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --data-in".to_string());
            };
            data_in = Some(path.clone());
            index += 2;
            continue;
        }
        if arg == "--data-out" {
            if !matches!(
                command,
                "run" | "simulate" | "dispatch" | "emit" | "schedule" | "dequeue" | "serve"
            ) {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --data-out".to_string());
            };
            data_out = Some(path.clone());
            index += 2;
            continue;
        }
        if arg == "--operation-output" {
            if !matches!(
                command,
                "run" | "simulate" | "dispatch" | "emit" | "schedule" | "dequeue" | "serve"
            ) {
                return Err(usage());
            }
            let Some(value) = args.get(index + 1) else {
                return Err("missing value for --operation-output".to_string());
            };
            insert_runtime_state_arg(value, &mut operation_outputs)?;
            index += 2;
            continue;
        }
        if arg == "--listen" {
            if command != "serve" {
                return Err(usage());
            }
            let Some(addr) = args.get(index + 1) else {
                return Err("missing value for --listen".to_string());
            };
            listen = Some(addr.clone());
            index += 2;
            continue;
        }

        if arg == "--llm-endpoint" {
            if !matches!(
                command,
                "llm-roundtrip" | "ail-requirements" | "ail-spec" | "ail-draft" | "ail-build"
            ) {
                return Err(usage());
            }
            let Some(url) = args.get(index + 1) else {
                return Err("missing value for --llm-endpoint".to_string());
            };
            llm_endpoint = Some(url.clone());
            index += 2;
            continue;
        }

        if arg == "--artifact-dir" {
            if !matches!(command, "ail-build" | "ail-pass" | "ail-lower") {
                return Err(usage());
            }
            let Some(path) = args.get(index + 1) else {
                return Err("missing value for --artifact-dir".to_string());
            };
            artifact_dir = Some(path.clone());
            index += 2;
            continue;
        }

        if !matches!(
            command,
            "run"
                | "simulate"
                | "dispatch"
                | "emit"
                | "schedule"
                | "dequeue"
                | "serve"
                | "ail-run"
                | "ail-vm"
        ) && command != "dispatch"
            && command != "emit"
            && command != "schedule"
            && command != "dequeue"
        {
            return Err(usage());
        }

        if command == "dispatch"
            || command == "emit"
            || command == "schedule"
            || command == "dequeue"
        {
            insert_runtime_state_arg(arg, &mut request_state)?;
        } else {
            insert_runtime_state_arg(arg, &mut runtime_state)?;
        }
        index += 1;
    }

    if ail_core_file.is_some() && ail_spec_file.is_some() {
        return Err("--core-file cannot be combined with --spec-file".to_string());
    }
    if command == "ail-build" && ail_requirements_file.is_some() && ail_spec_file.is_some() {
        return Err("--requirements-file cannot be combined with --spec-file".to_string());
    }
    if command == "ail-build" && ail_requirements_file.is_some() && ail_core_file.is_some() {
        return Err("--requirements-file cannot be combined with --core-file".to_string());
    }
    if command == "ail-build" && ail_build_target_model.is_some() && ail_build_agent.is_none() {
        return Err("--target-model requires --agent".to_string());
    }
    if command == "ail-build" {
        let native_requested = ail_compile_target.is_some() || ail_compile_out.is_some();
        if native_requested && ail_compile_target.is_none() {
            return Err("ail-build native output requires --target <target>".to_string());
        }
        if native_requested && ail_compile_out.is_none() {
            return Err("ail-build native output requires --out <path>".to_string());
        }
        if native_requested && ail_action.is_none() {
            return Err("ail-build native output requires --action <name>".to_string());
        }
        if !native_requested && ail_action.is_some() {
            return Err("ail-build --action requires --target and --out".to_string());
        }
    }
    if command == "ail-pass" && ail_core_file.is_none() && ail_pass_target.is_none() {
        return Err("ail-pass requires a target package or --core-file <path>".to_string());
    }
    if command == "ail-pass" && ail_compile_target.is_some() && artifact_dir.is_none() {
        return Err("ail-pass --target requires --artifact-dir <dir>".to_string());
    }

    Ok(CliOptions {
        selected_intent,
        runtime_state,
        request_state,
        state_in,
        state_out,
        data_in,
        data_out,
        operation_outputs,
        listen,
        llm_endpoint,
        artifact_dir,
        patch_path,
        dispatch_method,
        dispatch_path,
        trigger_name,
        ail_action,
        ail_prompt,
        ail_pass_target,
        ail_build_pass,
        ail_build_agent,
        ail_build_target_model,
        ail_requirements_file,
        ail_spec_file,
        ail_core_file,
        ail_compile_target,
        ail_compile_out,
    })
}

fn load_execution_state(cli_options: &CliOptions) -> Result<BTreeMap<String, String>, String> {
    let mut runtime_state = load_data_store_map(cli_options)?;
    runtime_state.extend(load_runtime_state(cli_options)?);
    Ok(runtime_state)
}

fn load_runtime_state(cli_options: &CliOptions) -> Result<BTreeMap<String, String>, String> {
    let mut runtime_state = if let Some(path) = &cli_options.state_in {
        load_state_file(path)?
    } else {
        BTreeMap::new()
    };
    runtime_state.extend(cli_options.runtime_state.clone());
    Ok(runtime_state)
}

fn load_data_store_map(cli_options: &CliOptions) -> Result<BTreeMap<String, String>, String> {
    if let Some(path) = &cli_options.data_in {
        load_state_file(path)
    } else {
        Ok(BTreeMap::new())
    }
}

fn save_runtime_state(
    cli_options: &CliOptions,
    runtime_state: &BTreeMap<String, String>,
) -> Result<(), String> {
    let Some(path) = &cli_options.state_out else {
        return Ok(());
    };
    let content = runtime_state
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, content)
        .map_err(|error| format!("failed to write state file '{}': {error}", path))
}

fn save_data_store(
    document: &RifDocument,
    cli_options: &CliOptions,
    runtime_state: &BTreeMap<String, String>,
) -> Result<(), String> {
    let Some(path) = &cli_options.data_out else {
        return Ok(());
    };
    let data_entries = persistent_state_entries(document, runtime_state);
    let content = data_entries
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, content)
        .map_err(|error| format!("failed to write data file '{}': {error}", path))
}

fn load_state_file(path: &str) -> Result<BTreeMap<String, String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read state file '{path}': {error}"))?;
    let mut state = BTreeMap::new();
    for (line_number, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!(
                "invalid state file '{path}' line {}: expected key=value",
                line_number + 1
            ));
        };
        if key.trim().is_empty() {
            return Err(format!(
                "invalid state file '{path}' line {}: key cannot be empty",
                line_number + 1
            ));
        }
        state.insert(key.trim().to_string(), value.trim().to_string());
    }
    Ok(state)
}

fn persistent_state_entries(
    document: &RifDocument,
    runtime_state: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    runtime_state
        .iter()
        .filter(|(key, _)| is_collection_state_key(document, key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn is_collection_state_key(document: &RifDocument, key: &str) -> bool {
    let Some((root, rest)) = key.split_once('.') else {
        return false;
    };
    if !document.application.collections.contains_key(root) {
        return false;
    }
    let Some((record_id, _)) = rest.split_once('.') else {
        return false;
    };
    !record_id.trim().is_empty()
}

fn select_endpoint<'a>(
    document: &'a RifDocument,
    method: &str,
    path: &str,
) -> Result<
    (
        &'a eigl::rif_model::EndpointDefinition,
        BTreeMap<String, String>,
    ),
    String,
> {
    for endpoint in &document.application.endpoints {
        if !endpoint.method.eq_ignore_ascii_case(method) {
            continue;
        }
        if let Some(path_params) = match_endpoint_path(&endpoint.path, path) {
            return Ok((endpoint, path_params));
        }
    }
    Err(format!("unknown endpoint '{method} {path}'"))
}

fn select_trigger<'a>(
    document: &'a RifDocument,
    trigger_name: &str,
) -> Result<&'a TriggerDefinition, String> {
    document
        .application
        .triggers
        .iter()
        .find(|trigger| trigger.name == trigger_name)
        .ok_or_else(|| format!("unknown trigger '{trigger_name}'"))
}

fn apply_endpoint_bindings(
    endpoint: &eigl::rif_model::EndpointDefinition,
    document: &RifDocument,
    intent: &Intent,
    runtime_state: &BTreeMap<String, String>,
    request_state: &BTreeMap<String, String>,
    bound_state: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    for (target, source) in &endpoint.bindings {
        let Some(target_type) = runtime_state_type(document, intent, target)? else {
            return Err(format!(
                "unknown endpoint binding target '{}': not a valid state path",
                target
            ));
        };
        if let Some(field_values) = typed_object_binding_values(
            document,
            target,
            source,
            runtime_state,
            request_state,
            target_type,
        ) {
            bound_state.extend(field_values?);
            continue;
        }
        let value = evaluate_binding_expression(source, runtime_state, request_state);
        if let Some(field_values) = typed_object_field_values(document, target, &value, target_type)
        {
            bound_state.extend(field_values?);
            continue;
        }
        validate_runtime_value(document, target, &value, target_type)?;
        bound_state.insert(target.clone(), value);
    }
    Ok(())
}

fn apply_trigger_bindings(
    trigger: &TriggerDefinition,
    document: &RifDocument,
    intent: &Intent,
    runtime_state: &BTreeMap<String, String>,
    request_state: &BTreeMap<String, String>,
    bound_state: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    for (target, source) in &trigger.bindings {
        let Some(target_type) = runtime_state_type(document, intent, target)? else {
            return Err(format!(
                "unknown trigger binding target '{}': not a valid state path",
                target
            ));
        };
        if let Some(field_values) = typed_object_binding_values(
            document,
            target,
            source,
            runtime_state,
            request_state,
            target_type,
        ) {
            bound_state.extend(field_values?);
            continue;
        }
        let value = evaluate_binding_expression(source, runtime_state, request_state);
        if let Some(field_values) = typed_object_field_values(document, target, &value, target_type)
        {
            bound_state.extend(field_values?);
            continue;
        }
        validate_runtime_value(document, target, &value, target_type)?;
        bound_state.insert(target.clone(), value);
    }
    Ok(())
}

fn evaluate_binding_expression(
    expression: &str,
    runtime_state: &BTreeMap<String, String>,
    request_state: &BTreeMap<String, String>,
) -> String {
    expression::evaluate(expression, |token| {
        binding_expression_value(token, runtime_state, request_state)
    })
}

fn binding_expression_value(
    token: &str,
    runtime_state: &BTreeMap<String, String>,
    request_state: &BTreeMap<String, String>,
) -> Option<String> {
    if let Some(value) = request_state
        .get(token)
        .cloned()
        .or_else(|| runtime_state.get(token).cloned())
    {
        return Some(value);
    }
    if let Some(value) = collection_path_value_with(runtime_state, token, |name| {
        request_state
            .get(name)
            .cloned()
            .or_else(|| runtime_state.get(name).cloned())
    }) {
        return Some(value);
    }
    if let Some(value) = expression::resolve_container_count(token, |name| {
        binding_expression_value(name, runtime_state, request_state)
    }) {
        return Some(value);
    }
    if let Some(value) = expression::resolve_option_value_lookup(token, |name| {
        binding_expression_value(name, runtime_state, request_state)
    }) {
        return Some(value);
    }
    if let Some(value) = expression::resolve_result_variant_lookup(token, |name| {
        binding_expression_value(name, runtime_state, request_state)
    }) {
        return Some(value);
    }
    if let Some(value) = expression::resolve_object_field_lookup(token, |name| {
        binding_expression_value(name, runtime_state, request_state)
    }) {
        return Some(value);
    }
    if let Some(value) = expression::resolve_map_lookup(token, |name| {
        binding_expression_value(name, runtime_state, request_state)
    }) {
        return Some(value);
    }
    if let Some(value) = expression::resolve_list_lookup(token, |name| {
        binding_expression_value(name, runtime_state, request_state)
    }) {
        return Some(value);
    }
    if token.contains('[') {
        let resolved = resolve_binding_path(token, runtime_state, request_state);
        return request_state
            .get(&resolved)
            .cloned()
            .or_else(|| runtime_state.get(&resolved).cloned());
    }
    None
}

fn resolve_binding_path(
    expression: &str,
    runtime_state: &BTreeMap<String, String>,
    request_state: &BTreeMap<String, String>,
) -> String {
    let mut resolved = String::new();
    let mut rest = expression;
    while let Some(open) = rest.find('[') {
        resolved.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(']') else {
            return expression.to_string();
        };
        let inner = &after_open[..close];
        if inner.contains('=') {
            resolved.push('[');
            resolved.push_str(inner);
            resolved.push(']');
            rest = &after_open[close + 1..];
            continue;
        }
        let value = resolve_binding_path(inner.trim(), runtime_state, request_state);
        if !resolved.is_empty() && !resolved.ends_with('.') {
            resolved.push('.');
        }
        resolved.push_str(&value);
        rest = &after_open[close + 1..];
    }
    resolved.push_str(rest);
    request_state
        .get(&resolved)
        .cloned()
        .or_else(|| runtime_state.get(&resolved).cloned())
        .unwrap_or(resolved)
}

fn resolve_binding_state_path(
    expression: &str,
    runtime_state: &BTreeMap<String, String>,
    request_state: &BTreeMap<String, String>,
) -> String {
    if !expression.contains('[') {
        return expression.to_string();
    }

    let mut resolved = String::new();
    let mut rest = expression;
    while let Some(open) = rest.find('[') {
        resolved.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(']') else {
            return expression.to_string();
        };
        let inner = &after_open[..close];
        if inner.contains('=') {
            resolved.push('[');
            resolved.push_str(inner);
            resolved.push(']');
            rest = &after_open[close + 1..];
            continue;
        }
        let value = evaluate_binding_expression(inner.trim(), runtime_state, request_state);
        if !resolved.is_empty() && !resolved.ends_with('.') {
            resolved.push('.');
        }
        resolved.push_str(&value);
        rest = &after_open[close + 1..];
    }
    resolved.push_str(rest);
    resolved
}

fn execute_endpoint(
    document: &RifDocument,
    method: &str,
    request_path: &str,
    runtime_state: &BTreeMap<String, String>,
    request_state: &BTreeMap<String, String>,
    operation_outputs: &BTreeMap<String, String>,
) -> Result<EndpointExecutionResult, String> {
    let (path, path_params) = split_request_path(request_path);
    let (endpoint, matched_path_params) = select_endpoint(document, method, path)?;
    let target_intent = document
        .intents
        .iter()
        .find(|intent| intent.name == endpoint.target)
        .cloned()
        .ok_or_else(|| format!("unknown endpoint target '{}'", endpoint.target))?;
    let mut bound_state = runtime_state.clone();
    let mut combined_request_state = request_state.clone();
    combined_request_state.extend(path_params);
    combined_request_state.extend(matched_path_params);
    for requirement in &endpoint.requires {
        let (condition, failure_name) = endpoint_requirement_condition(requirement);
        if !predicate::evaluate(condition, &bound_state, &combined_request_state) {
            let run = eigl::eig_ir::BytecodeRunResult {
                status: "failed".to_string(),
                final_state: bound_state,
                outputs: BTreeMap::new(),
                trace: vec![format!("CHECK FAILED {condition}")],
                failure: Some(failure_name.to_string()),
            };
            let error_response =
                evaluate_endpoint_error_response(document, endpoint, &run, &combined_request_state);
            let error_status = endpoint_error_status(endpoint, run.failure.as_deref());
            return Ok(EndpointExecutionResult {
                run,
                success_status: None,
                response: BTreeMap::new(),
                error_status,
                error_response,
            });
        }
    }
    if let Some(message) =
        endpoint_request_validation_failure(endpoint, document, &combined_request_state)
    {
        let run = eigl::eig_ir::BytecodeRunResult {
            status: "failed".to_string(),
            final_state: bound_state,
            outputs: BTreeMap::new(),
            trace: vec![message],
            failure: Some("BadRequest".to_string()),
        };
        let error_response =
            evaluate_endpoint_error_response(document, endpoint, &run, &combined_request_state);
        let error_status = endpoint_error_status(endpoint, run.failure.as_deref());
        return Ok(EndpointExecutionResult {
            run,
            success_status: None,
            response: BTreeMap::new(),
            error_status,
            error_response,
        });
    }
    apply_endpoint_bindings(
        endpoint,
        document,
        &target_intent,
        runtime_state,
        &combined_request_state,
        &mut bound_state,
    )?;
    let mut dispatch_document = document.clone();
    dispatch_document.intent = target_intent;
    validate_runtime_state(&dispatch_document, &bound_state)?;
    validate_operation_outputs(&dispatch_document, operation_outputs)?;
    let run = run_bytecode_with_operation_outputs(
        &lower_document(&dispatch_document),
        bound_state,
        operation_outputs.clone(),
        BTreeMap::new(),
    );
    let response = if run.status == "succeeded" {
        evaluate_endpoint_response(document, endpoint, &run, &combined_request_state)
    } else {
        BTreeMap::new()
    };
    let error_response = if run.status == "succeeded" {
        BTreeMap::new()
    } else {
        evaluate_endpoint_error_response(document, endpoint, &run, &combined_request_state)
    };
    let error_status = endpoint_error_status(endpoint, run.failure.as_deref());
    Ok(EndpointExecutionResult {
        run,
        success_status: endpoint.response_status.clone(),
        response,
        error_status,
        error_response,
    })
}

fn endpoint_request_validation_failure(
    endpoint: &eigl::rif_model::EndpointDefinition,
    document: &RifDocument,
    request_state: &BTreeMap<String, String>,
) -> Option<String> {
    for (name, type_name) in &endpoint.request_fields {
        let Some(value) = request_state.get(name) else {
            return Some(format!("missing endpoint request field '{name}'"));
        };
        if let Err(error) = validate_runtime_value(document, name, value, type_name) {
            return Some(error);
        }
    }
    None
}

fn endpoint_requirement_condition(requirement: &str) -> (&str, &str) {
    let Some((condition, failure)) = requirement.rsplit_once(" else ") else {
        return (requirement, "Unauthorized");
    };
    let failure = failure.trim();
    if failure.is_empty() {
        (requirement, "Unauthorized")
    } else {
        (condition.trim(), failure)
    }
}

fn evaluate_endpoint_response(
    document: &RifDocument,
    endpoint: &eigl::rif_model::EndpointDefinition,
    result: &eigl::eig_ir::BytecodeRunResult,
    request_state: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut response_state = result.final_state.clone();
    response_state.extend(result.outputs.clone());
    endpoint
        .responses
        .iter()
        .map(|(name, source)| {
            (
                name.clone(),
                endpoint
                    .response_fields
                    .get(name)
                    .and_then(|type_name| {
                        typed_object_response_value(
                            document,
                            source,
                            &response_state,
                            request_state,
                            type_name,
                        )
                    })
                    .unwrap_or_else(|| {
                        evaluate_binding_expression(source, &response_state, request_state)
                    }),
            )
        })
        .collect()
}

fn evaluate_endpoint_error_response(
    document: &RifDocument,
    endpoint: &eigl::rif_model::EndpointDefinition,
    result: &eigl::eig_ir::BytecodeRunResult,
    request_state: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut response_state = result.final_state.clone();
    response_state.extend(result.outputs.clone());
    response_state.insert(
        "failure".to_string(),
        result
            .failure
            .clone()
            .unwrap_or_else(|| result.status.clone()),
    );
    let (fields, responses) = endpoint
        .error_cases
        .get(result.failure.as_deref().unwrap_or(""))
        .map(|error| (&error.response_fields, &error.responses))
        .unwrap_or((&endpoint.error_fields, &endpoint.error_responses));
    responses
        .iter()
        .map(|(name, source)| {
            (
                name.clone(),
                fields
                    .get(name)
                    .and_then(|type_name| {
                        typed_object_response_value(
                            document,
                            source,
                            &response_state,
                            request_state,
                            type_name,
                        )
                    })
                    .unwrap_or_else(|| {
                        evaluate_binding_expression(source, &response_state, request_state)
                    }),
            )
        })
        .collect()
}

fn typed_object_response_value(
    document: &RifDocument,
    source: &str,
    runtime_state: &BTreeMap<String, String>,
    request_state: &BTreeMap<String, String>,
    type_name: &str,
) -> Option<String> {
    let source = source.trim();
    let thing = document.application.things.get(type_name.trim())?;
    typed_object_state_value(document, source, thing, runtime_state).or_else(|| {
        let value = request_state.get(source)?;
        typed_object_field_values_for_thing(document, source, value, thing).ok()?;
        Some(value.clone())
    })
}

fn typed_object_state_value(
    document: &RifDocument,
    source: &str,
    thing: &eigl::rif_model::ThingDefinition,
    runtime_state: &BTreeMap<String, String>,
) -> Option<String> {
    let mut entries = Vec::new();
    for field in thing.fields.values() {
        let field_source = format!("{source}.{}", field.name);
        let value = if let Some(nested_thing) = document.application.things.get(&field.type_name) {
            typed_object_state_value(document, &field_source, nested_thing, runtime_state)?
        } else {
            runtime_state.get(&field_source)?.clone()
        };
        entries.push(format!(
            "{}:{}",
            json_string(&field.name),
            json_value(&value)
        ));
    }
    Some(format!("{{{}}}", entries.join(",")))
}

fn endpoint_error_status(
    endpoint: &eigl::rif_model::EndpointDefinition,
    failure: Option<&str>,
) -> Option<String> {
    failure
        .and_then(|name| endpoint.error_cases.get(name))
        .and_then(|error| error.status.clone())
        .or_else(|| endpoint.error_status.clone())
}

fn execute_trigger(
    document: &RifDocument,
    trigger_name: &str,
    runtime_state: &BTreeMap<String, String>,
    request_state: &BTreeMap<String, String>,
    operation_outputs: &BTreeMap<String, String>,
) -> Result<eigl::eig_ir::BytecodeRunResult, String> {
    let trigger = select_trigger(document, trigger_name)?;
    let target_intent = document
        .intents
        .iter()
        .find(|intent| intent.name == trigger.target)
        .cloned()
        .ok_or_else(|| format!("unknown trigger target '{}'", trigger.target))?;
    let mut bound_state = runtime_state.clone();
    let mut combined_request_state = request_state.clone();
    combined_request_state.extend(
        request_state
            .iter()
            .map(|(key, value)| (format!("event.{key}"), value.clone())),
    );
    combined_request_state.insert("event.name".to_string(), trigger.name.clone());
    combined_request_state.insert(
        "event.kind".to_string(),
        if trigger.queue.is_some() {
            "queue".to_string()
        } else if trigger.schedule.is_some() {
            "schedule".to_string()
        } else {
            "event".to_string()
        },
    );
    if let Some(schedule) = &trigger.schedule {
        combined_request_state.insert("event.schedule".to_string(), schedule.clone());
    }
    if let Some(queue) = &trigger.queue {
        combined_request_state.insert("event.queue".to_string(), queue.clone());
    }
    if let Some(message) =
        trigger_payload_validation_failure(trigger, document, &combined_request_state)
    {
        return Ok(eigl::eig_ir::BytecodeRunResult {
            status: "failed".to_string(),
            final_state: bound_state,
            outputs: BTreeMap::new(),
            trace: vec![message],
            failure: Some("BadEvent".to_string()),
        });
    }
    for requirement in &trigger.requires {
        if !predicate::evaluate(requirement, &bound_state, &combined_request_state) {
            return Ok(eigl::eig_ir::BytecodeRunResult {
                status: "failed".to_string(),
                final_state: bound_state,
                outputs: BTreeMap::new(),
                trace: vec![format!("CHECK FAILED {requirement}")],
                failure: Some("Unauthorized".to_string()),
            });
        }
    }
    apply_trigger_bindings(
        trigger,
        document,
        &target_intent,
        runtime_state,
        &combined_request_state,
        &mut bound_state,
    )?;
    let mut dispatch_document = document.clone();
    dispatch_document.intent = target_intent;
    validate_runtime_state(&dispatch_document, &bound_state)?;
    validate_operation_outputs(&dispatch_document, operation_outputs)?;
    Ok(run_bytecode_with_operation_outputs(
        &lower_document(&dispatch_document),
        bound_state,
        operation_outputs.clone(),
        BTreeMap::new(),
    ))
}

fn trigger_payload_validation_failure(
    trigger: &TriggerDefinition,
    document: &RifDocument,
    request_state: &BTreeMap<String, String>,
) -> Option<String> {
    for (name, type_name) in &trigger.payload_fields {
        let Some(value) = request_state
            .get(name)
            .or_else(|| request_state.get(&format!("event.{name}")))
        else {
            return Some(format!("missing trigger payload field '{name}'"));
        };
        if let Err(error) = validate_runtime_value(document, name, value, type_name) {
            return Some(error);
        }
    }
    None
}

fn match_endpoint_path(template: &str, actual: &str) -> Option<BTreeMap<String, String>> {
    let template_parts: Vec<_> = template.trim_matches('/').split('/').collect();
    let actual_parts: Vec<_> = actual.trim_matches('/').split('/').collect();
    if template_parts.len() != actual_parts.len() {
        return None;
    }
    let mut params = BTreeMap::new();
    for (template_part, actual_part) in template_parts.into_iter().zip(actual_parts) {
        if let Some(name) = template_part
            .strip_prefix('{')
            .and_then(|part| part.strip_suffix('}'))
        {
            if name.is_empty() {
                return None;
            }
            params.insert(name.to_string(), actual_part.to_string());
        } else if template_part != actual_part {
            return None;
        }
    }
    Some(params)
}

fn split_request_path(path: &str) -> (&str, BTreeMap<String, String>) {
    let Some((base, query)) = path.split_once('?') else {
        return (path, BTreeMap::new());
    };
    (base, parse_query_values(query))
}

fn parse_key_values(text: &str, delimiter: char) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for pair in text.split(delimiter) {
        if let Some((key, value)) = pair.split_once('=')
            && !key.trim().is_empty()
        {
            values.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    values
}

fn parse_query_values(query: &str) -> BTreeMap<String, String> {
    let raw_values = parse_key_values(query, '&');
    let mut values = raw_values.clone();
    for (key, value) in raw_values {
        if key.starts_with("query.") {
            continue;
        }
        values.insert(format!("query.{key}"), value);
    }
    values
}

fn parse_request_payload(body: &str, content_type: Option<&str>) -> BTreeMap<String, String> {
    if (content_type.is_some_and(|content_type| content_type.contains("application/json"))
        || body.trim_start().starts_with('{'))
        && let Some(values) = parse_json_object(body)
    {
        return values;
    }
    parse_key_values(body, '&')
}

fn parse_json_object(text: &str) -> Option<BTreeMap<String, String>> {
    let mut parser = JsonParser::new(text);
    let mut values = BTreeMap::new();
    parser.parse_object_into("", &mut values).ok()?;
    parser.skip_whitespace();
    if parser.is_eof() { Some(values) } else { None }
}

struct JsonParser<'a> {
    text: &'a str,
    index: usize,
}

impl<'a> JsonParser<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, index: 0 }
    }

    fn is_eof(&self) -> bool {
        self.index >= self.text.len()
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.index += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.text[self.index..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.index += ch.len_utf8();
        Some(ch)
    }

    fn parse_object_into(
        &mut self,
        prefix: &str,
        values: &mut BTreeMap<String, String>,
    ) -> Result<String, String> {
        self.skip_whitespace();
        if self.next_char() != Some('{') {
            return Err("expected JSON object".to_string());
        }
        self.skip_whitespace();
        if self.peek_char() == Some('}') {
            self.index += 1;
            if !prefix.is_empty() {
                values.insert(prefix.to_string(), "{}".to_string());
            }
            return Ok("{}".to_string());
        }
        let mut entries = Vec::new();
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.next_char() != Some(':') {
                return Err("expected ':' in JSON object".to_string());
            }
            self.skip_whitespace();
            let field_prefix = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            let value = self.parse_value_into(&field_prefix, values)?;
            entries.push(format!("{}:{}", json_string(&key), value));
            self.skip_whitespace();
            match self.next_char() {
                Some(',') => continue,
                Some('}') => break,
                _ => return Err("expected ',' or '}' in JSON object".to_string()),
            }
        }
        let rendered = format!("{{{}}}", entries.join(","));
        if !prefix.is_empty() {
            values.insert(prefix.to_string(), rendered.clone());
        }
        Ok(rendered)
    }

    fn parse_value_into(
        &mut self,
        prefix: &str,
        values: &mut BTreeMap<String, String>,
    ) -> Result<String, String> {
        self.skip_whitespace();
        match self.peek_char() {
            Some('{') => self.parse_object_into(prefix, values),
            Some('[') => self.parse_array_into(prefix, values),
            Some('"') => {
                let value = self.parse_string()?;
                let rendered = json_string(&value);
                values.insert(prefix.to_string(), value);
                Ok(rendered)
            }
            Some('t') => {
                self.expect_literal("true")?;
                values.insert(prefix.to_string(), "true".to_string());
                Ok("true".to_string())
            }
            Some('f') => {
                self.expect_literal("false")?;
                values.insert(prefix.to_string(), "false".to_string());
                Ok("false".to_string())
            }
            Some('n') => {
                self.expect_literal("null")?;
                values.insert(prefix.to_string(), "null".to_string());
                Ok("null".to_string())
            }
            Some(ch) if ch == '-' || ch.is_ascii_digit() => {
                let number = self.parse_number();
                values.insert(prefix.to_string(), number.clone());
                Ok(number)
            }
            _ => Err("unexpected JSON value".to_string()),
        }
    }

    fn parse_array_into(
        &mut self,
        prefix: &str,
        values: &mut BTreeMap<String, String>,
    ) -> Result<String, String> {
        self.skip_whitespace();
        if self.next_char() != Some('[') {
            return Err("expected JSON array".to_string());
        }
        self.skip_whitespace();
        if self.peek_char() == Some(']') {
            self.index += 1;
            if !prefix.is_empty() {
                values.insert(prefix.to_string(), "[]".to_string());
            }
            return Ok("[]".to_string());
        }

        let mut index = 0usize;
        let mut items = Vec::new();
        loop {
            self.skip_whitespace();
            let item_prefix = if prefix.is_empty() {
                format!("[{index}]")
            } else {
                format!("{prefix}[{index}]")
            };
            items.push(self.parse_value_into(&item_prefix, values)?);
            self.skip_whitespace();
            match self.next_char() {
                Some(',') => {
                    index += 1;
                }
                Some(']') => break,
                _ => return Err("expected ',' or ']' in JSON array".to_string()),
            }
        }
        let rendered = format!("[{}]", items.join(","));
        if !prefix.is_empty() {
            values.insert(prefix.to_string(), rendered.clone());
        }
        Ok(rendered)
    }

    fn parse_string(&mut self) -> Result<String, String> {
        if self.next_char() != Some('"') {
            return Err("expected JSON string".to_string());
        }
        let mut value = String::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '"' => return Ok(value),
                '\\' => {
                    let Some(escaped) = self.next_char() else {
                        return Err("unterminated JSON escape".to_string());
                    };
                    value.push(match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        '/' => '/',
                        'b' => '\u{0008}',
                        'f' => '\u{000C}',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        'u' => self.parse_unicode_escape()?,
                        other => other,
                    });
                }
                other => value.push(other),
            }
        }
        Err("unterminated JSON string".to_string())
    }

    fn parse_unicode_escape(&mut self) -> Result<char, String> {
        let code = self.take_hex_digits(4)?;
        char::from_u32(code).ok_or_else(|| "invalid unicode escape".to_string())
    }

    fn take_hex_digits(&mut self, count: usize) -> Result<u32, String> {
        let mut value = 0u32;
        for _ in 0..count {
            let Some(ch) = self.next_char() else {
                return Err("unterminated unicode escape".to_string());
            };
            value = (value << 4)
                | ch.to_digit(16)
                    .ok_or_else(|| "invalid unicode escape".to_string())?;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> String {
        let start = self.index;
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() || matches!(ch, '-' | '+' | '.' | 'e' | 'E') {
                self.index += ch.len_utf8();
            } else {
                break;
            }
        }
        self.text[start..self.index].to_string()
    }

    fn expect_literal(&mut self, literal: &str) -> Result<(), String> {
        for expected in literal.chars() {
            if self.next_char() != Some(expected) {
                return Err(format!("expected '{literal}'"));
            }
        }
        Ok(())
    }
}

fn handle_http_connection(
    document: &RifDocument,
    cli_options: &CliOptions,
    stream: &mut std::net::TcpStream,
) -> Result<(), String> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 1024];
    loop {
        let read = stream
            .read(&mut temp)
            .map_err(|error| format!("failed to read request: {error}"))?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..read]);
        if let Some(header_end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&buffer[..header_end + 4]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.eq_ignore_ascii_case("content-length") {
                        value.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            if buffer.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    let request = String::from_utf8_lossy(&buffer);
    let mut parts = request.splitn(2, "\r\n\r\n");
    let head = parts.next().unwrap_or("");
    let body = parts.next().unwrap_or("");
    let mut lines = head.lines();
    let Some(request_line) = lines.next() else {
        return Err("malformed HTTP request".to_string());
    };
    let mut request_parts = request_line.split_whitespace();
    let Some(method) = request_parts.next() else {
        return Err("malformed HTTP request line".to_string());
    };
    let Some(path) = request_parts.next() else {
        return Err("malformed HTTP request line".to_string());
    };
    let headers = parse_request_headers(lines);
    let content_type = headers.get("content_type").cloned();
    let mut request_state = parse_request_payload(body, content_type.as_deref());
    if let Some(token) = headers
        .get("authorization")
        .and_then(|value| parse_bearer_token(value))
    {
        request_state.insert("auth.bearer_token".to_string(), token);
    }
    if let Some(cookie_header) = headers.get("cookie") {
        request_state.extend(parse_cookie_header(cookie_header));
    }
    request_state.extend(
        headers
            .into_iter()
            .map(|(name, value)| (format!("headers.{name}"), value)),
    );
    request_state.extend(parse_query_values(
        path.split_once('?').map(|(_, query)| query).unwrap_or(""),
    ));
    let runtime_state = load_execution_state(cli_options)?;
    let result = execute_endpoint(
        document,
        method,
        path,
        &runtime_state,
        &request_state,
        &cli_options.operation_outputs,
    )?;
    let fallback_failure_status = if result.run.failure.as_deref() == Some("Unauthorized") {
        "401 Unauthorized"
    } else {
        "400 Bad Request"
    };
    let status = if result.run.status == "succeeded" {
        result.success_status.as_deref().unwrap_or("200 OK")
    } else {
        result
            .error_status
            .as_deref()
            .unwrap_or(fallback_failure_status)
    };
    let (content_type, body) = if result.run.status == "succeeded" && !result.response.is_empty() {
        (
            "application/json; charset=utf-8",
            render_json_object(&result.response),
        )
    } else if result.run.status != "succeeded" && !result.error_response.is_empty() {
        (
            "application/json; charset=utf-8",
            render_json_object(&result.error_response),
        )
    } else {
        (
            "text/plain; charset=utf-8",
            format!(
                "dispatch {}\nfailure={}\n{}\n",
                result.run.status,
                result.run.failure.as_deref().unwrap_or(""),
                runtime_state_lines(document, &result.run.final_state).join("\n")
            ),
        )
    };
    write_http_response(stream, status, content_type, &body)?;
    if result.run.status == "succeeded" {
        save_runtime_state(cli_options, &result.run.final_state)?;
        save_data_store(document, cli_options, &result.run.final_state)?;
    }
    Ok(())
}

fn write_http_response(
    stream: &mut std::net::TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> Result<(), String> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|error| format!("failed to write response: {error}"))
}

fn render_json_object(values: &BTreeMap<String, String>) -> String {
    let fields = values
        .iter()
        .map(|(key, value)| format!("{}:{}", json_string(key), json_value(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{fields}}}")
}

fn parse_request_headers(lines: std::str::Lines<'_>) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.insert(normalize_header_name(name), value.trim().to_string());
    }
    headers
}

fn normalize_header_name(name: &str) -> String {
    name.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch == '-' { '_' } else { ch })
        .collect()
}

fn parse_bearer_token(value: &str) -> Option<String> {
    let value = value.trim();
    let (scheme, token) = value.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("bearer") && !token.trim().is_empty() {
        Some(token.trim().to_string())
    } else {
        None
    }
}

fn parse_cookie_header(value: &str) -> BTreeMap<String, String> {
    let mut cookies = BTreeMap::new();
    for pair in value.split(';') {
        let Some((name, raw_value)) = pair.trim().split_once('=') else {
            continue;
        };
        let name = normalize_header_name(name);
        let value = raw_value.trim().to_string();
        if !name.is_empty() {
            cookies.insert(format!("cookies.{name}"), value.clone());
            if matches!(name.as_str(), "session" | "session_id") {
                cookies.insert("auth.session_id".to_string(), value);
            }
        }
    }
    cookies
}

fn insert_runtime_state_arg(
    arg: &str,
    runtime_state: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let Some((key, value)) = arg.split_once('=') else {
        return Err(format!("invalid run argument '{arg}': expected key=value"));
    };
    if key.is_empty() {
        return Err(format!("invalid run argument '{arg}': key cannot be empty"));
    }
    runtime_state.insert(key.to_string(), value.to_string());
    Ok(())
}

fn select_intent(document: &mut RifDocument, intent_name: &str) -> Result<(), String> {
    let Some(intent) = document
        .intents
        .iter()
        .find(|intent| intent.name == intent_name)
        .cloned()
    else {
        let available = document
            .intents
            .iter()
            .map(|intent| intent.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "unknown intent '{intent_name}'{}",
            if available.is_empty() {
                String::new()
            } else {
                format!("; available intents: {available}")
            }
        ));
    };
    document.intent = intent;
    Ok(())
}

fn validate_runtime_state(
    document: &RifDocument,
    runtime_state: &BTreeMap<String, String>,
) -> Result<(), String> {
    for (key, value) in runtime_state {
        let Some(type_name) = runtime_state_type(document, &document.intent, key)? else {
            continue;
        };
        validate_runtime_value(document, key, value, type_name)?;
    }
    validate_collection_constraints(document, runtime_state)?;
    Ok(())
}

fn validate_operation_outputs(
    document: &RifDocument,
    operation_outputs: &BTreeMap<String, String>,
) -> Result<(), String> {
    let reachable_intents = reachable_intent_names(document);
    for (name, value) in operation_outputs {
        let output_types = reachable_operation_output_types(document, &reachable_intents, name);
        if output_types.is_empty() {
            return Err(format!("unknown operation output '{name}'"));
        };
        for type_name in output_types {
            validate_runtime_value(document, name, value, &type_name)?;
        }
    }
    Ok(())
}

fn reachable_intent_names(document: &RifDocument) -> BTreeSet<String> {
    let mut reachable = BTreeSet::new();
    let mut stack = vec![document.intent.name.clone()];
    while let Some(intent_name) = stack.pop() {
        if !reachable.insert(intent_name.clone()) {
            continue;
        }
        let Some(intent) = document
            .intents
            .iter()
            .find(|candidate| candidate.name == intent_name)
            .or_else(|| (document.intent.name == intent_name).then_some(&document.intent))
        else {
            continue;
        };
        for target in invoked_intent_names(intent) {
            if !reachable.contains(&target) {
                stack.push(target);
            }
        }
    }
    reachable
}

fn invoked_intent_names(intent: &Intent) -> Vec<String> {
    let mut targets = Vec::new();
    for step in &intent.steps {
        if let Some(invoke) = &step.invoke {
            targets.push(invoke.target.clone());
        }
        targets.extend(
            step.parallel_invokes
                .iter()
                .map(|invoke| invoke.target.clone()),
        );
        if let Some(invoke) = &step.otherwise_invoke {
            targets.push(invoke.target.clone());
        }
        targets.extend(
            step.otherwise_parallel_invokes
                .iter()
                .map(|invoke| invoke.target.clone()),
        );
    }
    targets
}

fn reachable_operation_output_types(
    document: &RifDocument,
    reachable_intents: &BTreeSet<String>,
    output_name: &str,
) -> BTreeSet<String> {
    document
        .intents
        .iter()
        .filter(|intent| reachable_intents.contains(&intent.name))
        .chain(
            (!document
                .intents
                .iter()
                .any(|intent| intent.name == document.intent.name))
            .then_some(&document.intent),
        )
        .flat_map(|intent| {
            intent
                .steps
                .iter()
                .filter_map(|step| step.outputs.get(output_name))
                .map(|output| output.type_name.clone())
        })
        .collect()
}

fn validate_collection_constraints(
    document: &RifDocument,
    runtime_state: &BTreeMap<String, String>,
) -> Result<(), String> {
    for collection in document.application.collections.values() {
        for unique_field in &collection.unique_fields {
            let mut seen: BTreeMap<String, String> = BTreeMap::new();
            for record_id in collection_record_keys(runtime_state, &collection.name) {
                let path = format!("{}.{}.{}", collection.name, record_id, unique_field);
                let Some(value) = runtime_state.get(&path).cloned() else {
                    return Err(format!(
                        "collection '{}' record '{}' missing unique field '{}'",
                        collection.name, record_id, unique_field
                    ));
                };
                if let Some(previous_id) = seen.insert(value.clone(), record_id.clone()) {
                    return Err(format!(
                        "collection '{}' unique field '{}' has duplicate value '{}' for records '{}' and '{}'",
                        collection.name, unique_field, value, previous_id, record_id
                    ));
                }
            }
        }
    }
    Ok(())
}

fn runtime_state_type<'a>(
    document: &'a RifDocument,
    intent: &'a Intent,
    key: &str,
) -> Result<Option<&'a str>, String> {
    let (root, field_path) = key.split_once('.').unwrap_or((key, ""));
    if root == "auth" {
        return Ok(Some("Text"));
    }
    if let Some(collection) = document.application.collections.get(root) {
        let Some((record_id, record_field_path)) = field_path.split_once('.') else {
            return Err(format!(
                "unknown runtime state '{key}': collection '{}' values must use record.field paths",
                collection.name
            ));
        };
        if record_id.trim().is_empty() {
            return Err(format!(
                "unknown runtime state '{key}': collection '{}' record id cannot be empty",
                collection.name
            ));
        }
        if record_field_path.trim().is_empty() {
            return Err(format!(
                "unknown runtime state '{key}': collection '{}' values must address a record field",
                collection.name
            ));
        }
        let mut current_type = collection.type_name.as_str();
        let mut traversed_declared_type = false;
        for field_name in record_field_path.split('.') {
            if field_name == "count"
                && (generic_inner(current_type, "List").is_some()
                    || generic_args(current_type, "Map").is_some())
            {
                current_type = "Int";
                traversed_declared_type = true;
                continue;
            }
            let Some(thing) = document.application.things.get(current_type) else {
                if traversed_declared_type {
                    return Err(format!(
                        "unknown runtime state '{key}': type '{current_type}' has no field '{field_name}'"
                    ));
                }
                return Ok(None);
            };
            traversed_declared_type = true;
            let Some(field) = thing.fields.get(field_name) else {
                return Err(format!(
                    "unknown runtime state '{key}': type '{current_type}' has no field '{field_name}'"
                ));
            };
            current_type = field.type_name.as_str();
        }
        return Ok(Some(current_type));
    }

    let Some(root_type) = intent
        .subjects
        .get(root)
        .or_else(|| intent.inputs.get(root))
        .map(|thing| thing.type_name.as_str())
    else {
        return Err(format!(
            "unknown runtime state '{key}': '{root}' is not a subject or input"
        ));
    };

    if field_path.is_empty() {
        return Ok(Some(root_type));
    }

    let mut current_type = root_type;
    let mut traversed_declared_type = false;
    for field_name in field_path.split('.') {
        if field_name == "count"
            && (generic_inner(current_type, "List").is_some()
                || generic_args(current_type, "Map").is_some())
        {
            current_type = "Int";
            traversed_declared_type = true;
            continue;
        }
        let Some(thing) = document.application.things.get(current_type) else {
            if traversed_declared_type {
                return Err(format!(
                    "unknown runtime state '{key}': type '{current_type}' has no field '{field_name}'"
                ));
            }
            return Ok(None);
        };
        traversed_declared_type = true;
        let Some(field) = thing.fields.get(field_name) else {
            return Err(format!(
                "unknown runtime state '{key}': type '{current_type}' has no field '{field_name}'"
            ));
        };
        current_type = field.type_name.as_str();
    }
    Ok(Some(current_type))
}

fn typed_object_field_values(
    document: &RifDocument,
    key: &str,
    value: &str,
    type_name: &str,
) -> Option<Result<BTreeMap<String, String>, String>> {
    let thing = document.application.things.get(type_name.trim())?;
    Some(typed_object_field_values_for_thing(
        document, key, value, thing,
    ))
}

fn typed_object_binding_values(
    document: &RifDocument,
    target: &str,
    source: &str,
    runtime_state: &BTreeMap<String, String>,
    request_state: &BTreeMap<String, String>,
    type_name: &str,
) -> Option<Result<BTreeMap<String, String>, String>> {
    let thing = document.application.things.get(type_name.trim())?;
    if binding_expression_value(source, runtime_state, request_state)
        .is_some_and(|value| parse_json_object(&value).is_some())
    {
        return None;
    }
    Some(typed_object_binding_values_for_thing(
        document,
        target,
        source,
        runtime_state,
        request_state,
        thing,
    ))
}

fn typed_object_binding_values_for_thing(
    document: &RifDocument,
    target: &str,
    source: &str,
    runtime_state: &BTreeMap<String, String>,
    request_state: &BTreeMap<String, String>,
    thing: &eigl::rif_model::ThingDefinition,
) -> Result<BTreeMap<String, String>, String> {
    let mut field_values = BTreeMap::new();
    for field in thing.fields.values() {
        let target_field = format!("{target}.{}", field.name);
        let source_field = format!("{source}.{}", field.name);
        if let Some(nested_thing) = document.application.things.get(&field.type_name) {
            field_values.extend(typed_object_binding_values_for_thing(
                document,
                &target_field,
                &source_field,
                runtime_state,
                request_state,
                nested_thing,
            )?);
        } else {
            let Some(field_value) =
                binding_expression_value(&source_field, runtime_state, request_state)
            else {
                return Err(format!(
                    "invalid runtime value for '{}': missing field '{}'",
                    target, field.name
                ));
            };
            let target_path =
                resolve_binding_state_path(&target_field, runtime_state, request_state);
            validate_runtime_value(document, &target_path, &field_value, &field.type_name)?;
            field_values.insert(target_path, field_value);
        }
    }
    Ok(field_values)
}

fn typed_object_field_values_for_thing(
    document: &RifDocument,
    key: &str,
    value: &str,
    thing: &eigl::rif_model::ThingDefinition,
) -> Result<BTreeMap<String, String>, String> {
    let values = parse_json_object(value).ok_or_else(|| {
        format!(
            "invalid runtime value for '{}': expected {}",
            key, thing.name
        )
    })?;
    let field_names: BTreeSet<_> = thing.fields.keys().map(String::as_str).collect();
    for value_key in values.keys() {
        let Some(root) = json_object_root_field(value_key) else {
            continue;
        };
        if !field_names.contains(root) {
            return Err(format!(
                "invalid runtime value for '{}': type '{}' has no field '{}'",
                key, thing.name, root
            ));
        }
    }

    let mut field_values = BTreeMap::new();
    for field in thing.fields.values() {
        let Some(field_value) = values.get(&field.name) else {
            return Err(format!(
                "invalid runtime value for '{}': missing field '{}'",
                key, field.name
            ));
        };
        let target = format!("{key}.{}", field.name);
        if let Some(nested_values) =
            typed_object_field_values(document, &target, field_value, &field.type_name)
        {
            field_values.extend(nested_values?);
        } else {
            validate_runtime_value(document, &target, field_value, &field.type_name)?;
            field_values.insert(target, field_value.clone());
        }
    }
    Ok(field_values)
}

fn json_object_root_field(key: &str) -> Option<&str> {
    let key = key.trim();
    if key.is_empty() || key.starts_with('[') {
        return None;
    }
    Some(
        key.find(['.', '['])
            .map_or(key, |boundary| &key[..boundary]),
    )
}

fn validate_runtime_value(
    document: &RifDocument,
    key: &str,
    value: &str,
    type_name: &str,
) -> Result<(), String> {
    let type_name = type_name.trim();
    if type_name == "Int" {
        if value.parse::<i64>().is_ok() {
            return Ok(());
        }
        return Err(format!("invalid runtime value for '{key}': expected Int"));
    }
    if type_name == "Decimal" {
        if value.parse::<f64>().is_ok_and(f64::is_finite) {
            return Ok(());
        }
        return Err(format!(
            "invalid runtime value for '{key}': expected Decimal"
        ));
    }
    if type_name == "Money" {
        if is_money_literal(value) {
            return Ok(());
        }
        return Err(format!("invalid runtime value for '{key}': expected Money"));
    }
    if type_name == "Time" {
        if is_time_literal(value) {
            return Ok(());
        }
        return Err(format!("invalid runtime value for '{key}': expected Time"));
    }
    if type_name == "Duration" {
        if is_duration_literal(value) {
            return Ok(());
        }
        return Err(format!(
            "invalid runtime value for '{key}': expected Duration"
        ));
    }
    if type_name == "Bool" {
        if matches!(value, "true" | "false") {
            return Ok(());
        }
        return Err(format!("invalid runtime value for '{key}': expected Bool"));
    }
    if let Some(element_type) = generic_inner(type_name, "List") {
        if let Some(items) = list_literal_items(value)
            && items
                .iter()
                .all(|item| validate_runtime_value(document, key, item, element_type).is_ok())
        {
            return Ok(());
        }
        return Err(format!(
            "invalid runtime value for '{key}': expected List<{}>",
            element_type
        ));
    }
    if let Some(map_types) = generic_args(type_name, "Map")
        && map_types.len() == 2
    {
        let key_type = map_types[0];
        let value_type = map_types[1];
        if let Some(entries) = map_literal_entries(value)
            && entries.iter().all(|(entry_key, entry_value)| {
                validate_runtime_value(document, key, entry_key, key_type).is_ok()
                    && validate_runtime_value(document, key, entry_value, value_type).is_ok()
            })
        {
            return Ok(());
        }
        return Err(format!(
            "invalid runtime value for '{key}': expected Map<{}, {}>",
            key_type, value_type
        ));
    }
    if let Some(inner_type) = generic_inner(type_name, "Option") {
        if value == "None" {
            return Ok(());
        }
        if let Some(inner_value) = option_some_value(value)
            && validate_runtime_value(document, key, inner_value, inner_type).is_ok()
        {
            return Ok(());
        }
        return Err(format!(
            "invalid runtime value for '{key}': expected Option<{}>",
            inner_type
        ));
    }
    if let Some(inner_type) = generic_inner(type_name, "Secret") {
        if validate_runtime_value(document, key, value, inner_type).is_ok() {
            return Ok(());
        }
        return Err(format!(
            "invalid runtime value for '{key}': expected Secret<{}>",
            inner_type
        ));
    }
    if let Some(result_types) = generic_args(type_name, "Result")
        && result_types.len() == 2
    {
        let success_type = result_types[0];
        let failure_type = result_types[1];
        if let Some(success_value) = constructor_value(value, "Success")
            && validate_runtime_value(document, key, success_value, success_type).is_ok()
        {
            return Ok(());
        }
        if let Some(failure_value) = constructor_value(value, "Failure")
            && validate_runtime_value(document, key, failure_value, failure_type).is_ok()
        {
            return Ok(());
        }
        return Err(format!(
            "invalid runtime value for '{key}': expected Result<{}, {}>",
            success_type, failure_type
        ));
    }
    if let Some(field_values) = typed_object_field_values(document, key, value, type_name) {
        field_values?;
        return Ok(());
    }

    if let Some(states) = state_type_values(type_name)
        && !states.iter().any(|state| state == value)
    {
        return Err(format!(
            "invalid runtime value for '{key}': expected one of {}",
            states.join(", ")
        ));
    }
    if let Some(enum_definition) = document.application.enums.get(type_name)
        && !enum_definition
            .values
            .iter()
            .any(|enum_value| enum_value == value)
    {
        return Err(format!(
            "invalid runtime value for '{key}': expected one of {}",
            enum_definition.values.join(", ")
        ));
    }
    Ok(())
}

fn state_type_values(type_name: &str) -> Option<Vec<String>> {
    let inner = type_name.strip_prefix("State<")?.strip_suffix('>')?;
    Some(
        inner
            .split(',')
            .map(str::trim)
            .filter(|state| !state.is_empty())
            .map(ToString::to_string)
            .collect(),
    )
}

fn is_money_literal(value: &str) -> bool {
    let Some((currency, amount)) = value.split_once(':') else {
        return false;
    };
    currency.len() == 3
        && currency.chars().all(|ch| ch.is_ascii_uppercase())
        && amount.parse::<f64>().is_ok_and(f64::is_finite)
}

fn is_time_literal(value: &str) -> bool {
    if value.len() != 20 {
        return false;
    }
    let bytes = value.as_bytes();
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    if !bytes
        .iter()
        .enumerate()
        .filter(|(index, _)| !matches!(index, 4 | 7 | 10 | 13 | 16 | 19))
        .all(|(_, byte)| byte.is_ascii_digit())
    {
        return false;
    }

    let month = parse_time_part(value, 5, 7);
    let day = parse_time_part(value, 8, 10);
    let hour = parse_time_part(value, 11, 13);
    let minute = parse_time_part(value, 14, 16);
    let second = parse_time_part(value, 17, 19);
    matches!(month, Some(1..=12))
        && matches!(day, Some(1..=31))
        && matches!(hour, Some(0..=23))
        && matches!(minute, Some(0..=59))
        && matches!(second, Some(0..=59))
}

fn parse_time_part(value: &str, start: usize, end: usize) -> Option<u32> {
    value.get(start..end)?.parse().ok()
}

fn is_duration_literal(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('P') else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }

    let mut chars = rest.chars().peekable();
    let mut in_time = false;
    let mut seen_time_marker = false;
    let mut seen_component = false;
    while chars.peek().is_some() {
        if chars.peek() == Some(&'T') {
            if seen_time_marker {
                return false;
            }
            seen_time_marker = true;
            in_time = true;
            chars.next();
            if chars.peek().is_none() {
                return false;
            }
            continue;
        }

        let mut has_digits = false;
        while chars.peek().is_some_and(char::is_ascii_digit) {
            has_digits = true;
            chars.next();
        }
        if !has_digits {
            return false;
        }

        let Some(unit) = chars.next() else {
            return false;
        };
        let valid_unit = if in_time {
            matches!(unit, 'H' | 'M' | 'S')
        } else {
            matches!(unit, 'Y' | 'M' | 'W' | 'D')
        };
        if !valid_unit {
            return false;
        }
        seen_component = true;
    }
    seen_component
}

fn list_literal_items(value: &str) -> Option<Vec<&str>> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        split_top_level_commas(inner)
            .into_iter()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn map_literal_entries(value: &str) -> Option<Vec<(&str, &str)>> {
    let inner = value.strip_prefix('{')?.strip_suffix('}')?;
    split_top_level_commas(inner)
        .into_iter()
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| split_top_level_once(entry, ':'))
        .collect()
}

fn option_some_value(value: &str) -> Option<&str> {
    value
        .strip_prefix("Some(")?
        .strip_suffix(')')
        .map(str::trim)
}

fn constructor_value<'a>(value: &'a str, constructor: &str) -> Option<&'a str> {
    let prefix = format!("{constructor}(");
    value
        .strip_prefix(&prefix)?
        .strip_suffix(')')
        .map(str::trim)
}

fn generic_inner<'a>(type_name: &'a str, wrapper: &str) -> Option<&'a str> {
    let trimmed = type_name.trim();
    let inner = trimmed.strip_prefix(wrapper)?.trim();
    inner.strip_prefix('<')?.strip_suffix('>').map(str::trim)
}

fn generic_args<'a>(type_name: &'a str, wrapper: &str) -> Option<Vec<&'a str>> {
    Some(
        split_top_level_commas(generic_inner(type_name, wrapper)?)
            .into_iter()
            .map(str::trim)
            .collect(),
    )
}

fn split_top_level_commas(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut angle_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut in_string = false;
    let mut previous_was_escape = false;

    for (index, ch) in text.char_indices() {
        if in_string {
            if ch == '"' && !previous_was_escape {
                in_string = false;
            }
            previous_was_escape = ch == '\\' && !previous_was_escape;
            if ch != '\\' {
                previous_was_escape = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if angle_depth == 0
                && bracket_depth == 0
                && brace_depth == 0
                && paren_depth == 0 =>
            {
                parts.push(&text[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

fn split_top_level_once(text: &str, separator: char) -> Option<(&str, &str)> {
    let mut angle_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut in_string = false;
    let mut previous_was_escape = false;

    for (index, ch) in text.char_indices() {
        if in_string {
            if ch == '"' && !previous_was_escape {
                in_string = false;
            }
            previous_was_escape = ch == '\\' && !previous_was_escape;
            if ch != '\\' {
                previous_was_escape = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ch if ch == separator
                && angle_depth == 0
                && bracket_depth == 0
                && brace_depth == 0
                && paren_depth == 0 =>
            {
                let right_start = index + ch.len_utf8();
                return Some((text[..index].trim(), text[right_start..].trim()));
            }
            _ => {}
        }
    }
    None
}
